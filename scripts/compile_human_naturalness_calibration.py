"""Compile actual human ratings into a calibration certificate.

This script rejects incomplete reviews and machine/LLM reviewer declarations.
The certificate calibrates the verifier; it is not runtime sentence authority.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from naturalness_verifier import digest


def wilson_lower(successes: int, total: int, z: float = 1.96) -> float:
    if total == 0:
        return 0.0
    p = successes / total
    denom = 1 + z * z / total
    center = p + z * z / (2 * total)
    spread = z * ((p * (1 - p) / total + z * z / (4 * total * total)) ** 0.5)
    return max(0.0, (center - spread) / denom)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--completed-packet", type=Path, required=True)
    ap.add_argument("--benchmark", type=Path, required=True)
    ap.add_argument("--automatic-report", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()
    packet = json.loads(args.completed_packet.read_text(encoding="utf-8"))
    reviewer = packet.get("reviewer", {})
    if reviewer.get("kind") != "HUMAN" or not reviewer.get("reviewer_id") or not reviewer.get("attestation"):
        raise RuntimeError("ACTUAL_HUMAN_REVIEWER_ATTESTATION_REQUIRED")
    allowed = {"NATURAL", "AWKWARD_BUT_GRAMMATICAL", "UNNATURAL"}
    for row in packet.get("items", []):
        if row.get("acceptability") not in allowed:
            raise RuntimeError(f"INCOMPLETE_ACCEPTABILITY:{row.get('review_id')}")
        if not isinstance(row.get("natural_korean"), bool) or not isinstance(row.get("meaning_preserved"), bool) or not isinstance(row.get("contextually_usable"), bool):
            raise RuntimeError(f"INCOMPLETE_BOOLEAN_RATING:{row.get('review_id')}")
    benchmark = json.loads(args.benchmark.read_text(encoding="utf-8"))["rows"]
    expected = {hashlib.sha256(row["surface"].encode()).hexdigest(): row["value"] for row in benchmark}
    automatic_report = json.loads(args.automatic_report.read_text(encoding="utf-8"))
    automatic_verdict = {row["surface_sha256"]: row["verdict"] for row in automatic_report["benchmark_results"]}
    positives = []
    negatives = []
    for row in packet["items"]:
        target = positives if expected.get(row["surface_sha256"]) == "TRUSTED_POSITIVE" else negatives
        target.append(row)
    def accepted(row: dict) -> bool:
        return row["natural_korean"] and row["meaning_preserved"] and row["contextually_usable"] and row["acceptability"] == "NATURAL"
    positive_accepts = sum(accepted(row) for row in positives)
    human_only_negative_accepts = sum(accepted(row) for row in negatives)
    end_to_end_false_accepts = sum(
        accepted(row) and automatic_verdict.get(row["surface_sha256"]) != "FAIL"
        for row in negatives
    )
    precision_lower = wilson_lower(positive_accepts, len(positives))
    false_accept_rate = end_to_end_false_accepts / len(negatives) if negatives else 1.0
    # Predeclared conservative calibration thresholds. They are not tuned from
    # the submitted labels.
    positive_rate = positive_accepts / len(positives) if positives else 0.0
    calibrated = (
        len(packet["items"]) >= 24
        and positive_rate >= 0.90
        and precision_lower >= 0.95
        and false_accept_rate <= 0.01
    )
    certificate = {
        "schema": "BCORE.HUMAN_NATURALNESS_CALIBRATION.V2",
        "status": "CALIBRATED" if calibrated else "CALIBRATION_FAILED",
        "runtime_authority": False,
        "reviewer_id_sha256": hashlib.sha256(reviewer["reviewer_id"].encode()).hexdigest(),
        "reviewer_attestation": reviewer["attestation"],
        "reviewed_at": reviewer.get("reviewed_at"),
        "benchmark_size": len(packet["items"]),
        "positive_count": len(positives), "positive_accepted": positive_accepts,
        "positive_acceptance_rate": positive_rate if positives else None,
        "precision_lower_bound": precision_lower,
        "hard_negative_count": len(negatives),
        "human_only_negative_accept": human_only_negative_accepts,
        "automatic_gate_rejected_negative": sum(automatic_verdict.get(row["surface_sha256"]) == "FAIL" for row in negatives),
        "hard_negative_false_pass": end_to_end_false_accepts,
        "false_accept_upper_bound": false_accept_rate,
        "thresholds": {"minimum_items": 24, "minimum_positive_acceptance": 0.90, "minimum_positive_wilson_lower_bound": 0.95, "maximum_hard_negative_false_pass_rate": 0.01},
        "completed_packet_sha256": hashlib.sha256(args.completed_packet.read_bytes()).hexdigest(),
        "artifact_sha256": "",
    }
    certificate["artifact_sha256"] = digest(certificate)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(certificate, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "status": certificate["status"], "positive": f"{positive_accepts}/{len(positives)}",
        "human_only_negative_accept": f"{human_only_negative_accepts}/{len(negatives)}",
        "end_to_end_false_pass": f"{end_to_end_false_accepts}/{len(negatives)}",
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
