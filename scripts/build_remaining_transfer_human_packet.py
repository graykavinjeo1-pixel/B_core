"""Build a blinded packet for frozen transfers not covered by prior human review."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from build_naturalness_acceptance_calibration import claim_summary
from naturalness_verifier import digest


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--transfers", type=Path, required=True)
    ap.add_argument("--prior-review", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()
    transfers = json.loads(args.transfers.read_text(encoding="utf-8"))["rows"]
    prior = json.loads(args.prior_review.read_text(encoding="utf-8"))
    reviewed = {row["surface_sha256"] for row in prior["items"]}
    unique = {}
    coverage = {}
    for row in transfers:
        if row["value"] != "APPLY_TRANSFER":
            continue
        surface_hash = hashlib.sha256(row["surface"].encode()).hexdigest()
        if surface_hash in reviewed:
            continue
        unique.setdefault(surface_hash, row)
        coverage.setdefault(surface_hash, []).append({
            "meaning_family_id": row["meaning_family_id"], "split": row["split"],
            "program_cluster_id": row["conditions"]["program_cluster_id"],
            "direction": row["conditions"]["direction"],
        })
    items = []
    for surface_hash, row in sorted(unique.items(), key=lambda value: (value[1]["speech_act"], value[0])):
        ir = row["approved_response_ir"]
        items.append({
            "review_id": digest({"surface_sha256": surface_hash, "packet": "REMAINING_TRANSFER_V1"})[:16],
            "surface_sha256": surface_hash, "surface": row["surface"],
            "canonical_summary": {
                "speech_act": ir.get("speech_act"), "operation": ir.get("operation"),
                "discourse_relation": ir.get("discourse_relation"), "claims": claim_summary(ir),
            },
            "natural_korean": None, "meaning_preserved": None,
            "contextually_usable": None, "acceptability": None, "notes": None,
        })
    packet = {
        "schema": "BCORE.HUMAN_NATURALNESS_CALIBRATION_PACKET.V2",
        "blinded_from_expected_class_program_origin_and_automatic_verdict": True,
        "allowed_acceptability": ["NATURAL", "AWKWARD_BUT_GRAMMATICAL", "UNNATURAL"],
        "reviewer": {"kind": "HUMAN", "reviewer_id": None, "attestation": None, "reviewed_at": None},
        "items": items,
        "hidden_coverage": coverage,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(packet, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "items": len(items),
        "covered_transfer_rows": sum(len(value) for value in coverage.values()),
        "covered_families": len({record["meaning_family_id"] for values in coverage.values() for record in values}),
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
