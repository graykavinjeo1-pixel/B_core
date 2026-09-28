"""Build family-balanced positive calibration evidence without changing the verifier.

The output is an offline research set.  Program-generated candidates are never
used as self-certified positive labels; they remain HUMAN_REVIEW_REQUIRED.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path

from naturalness_verifier import digest, morphology_verdict


STRONG_PROVENANCE = {
    "existing_neutral",
    "runtime_composition",
    "speech_act_coverage",
    "targeted_structural_gap",
}


def surface_hash(surface: str) -> str:
    return hashlib.sha256(surface.encode()).hexdigest()


def claim_summary(ir: dict) -> list[dict]:
    result = []
    for claim in ir.get("claims", []):
        value = claim.get("value", {})
        result.append({
            "subject": claim.get("subject", {}).get("canonical_lexical_label"),
            "relation": claim.get("relation"),
            "value_kind": value.get("kind"),
            "value": value.get("value"),
            "polarity": claim.get("polarity"),
            "modality": claim.get("modality"),
        })
    return result


def calibration_row(surface: str, ir: dict, family_id: str, split: str, origin: str,
                    provenance: list[dict], program: dict | None = None) -> dict:
    return {
        "calibration_id": digest({"surface": surface_hash(surface), "family": family_id, "origin": origin}),
        "meaning_family_id": family_id,
        "split": split,
        "speech_act": ir.get("speech_act"),
        "surface": surface,
        "surface_sha256": surface_hash(surface),
        "approved_response_ir": ir,
        "canonical_summary": {
            "speech_act": ir.get("speech_act"), "operation": ir.get("operation"),
            "discourse_relation": ir.get("discourse_relation"), "claims": claim_summary(ir),
        },
        "origin": origin,
        "provenance": provenance,
        "program": program,
        "naturalness_label": "HUMAN_REVIEW_REQUIRED",
    }


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--fresh", type=Path, required=True)
    ap.add_argument("--transfers", type=Path, required=True)
    ap.add_argument("--transfers-gated", type=Path, required=True)
    ap.add_argument("--classes", type=Path, required=True)
    ap.add_argument("--existing-human-packet", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    ap.add_argument("--human-packet-output", type=Path, required=True)
    args = ap.parse_args()

    fresh = json.loads(args.fresh.read_text(encoding="utf-8"))["rows"]
    transfers = json.loads(args.transfers.read_text(encoding="utf-8"))["rows"]
    gated = {row["prompt_sha256"]: row for row in json.loads(args.transfers_gated.read_text(encoding="utf-8"))}
    classes = json.loads(args.classes.read_text(encoding="utf-8"))["classes"]
    old_packet = json.loads(args.existing_human_packet.read_text(encoding="utf-8"))

    rows: list[dict] = []
    for source in fresh:
        if morphology_verdict(source["trusted_source_surface"])["verdict"] != "PASS":
            continue
        ir = source["approved_response_ir"]
        rows.append(calibration_row(
            source["trusted_source_surface"], ir, ir["semantic_sha256"], source["split"],
            "TRUSTED_BCORE_ORDINARY_REALIZATION",
            [{"source_path": source["source_path"], "inverse_verified": source["inverse_verified"]}],
        ))
    for source in transfers:
        if source["value"] != "APPLY_TRANSFER" or not gated.get(source["prompt_sha256"], {}).get("canonical_inverse"):
            continue
        rows.append(calibration_row(
            source["surface"], source["approved_response_ir"], source["meaning_family_id"], source["split"],
            "FROZEN_EDIT_PROGRAM_TRANSFER",
            [{"prompt_sha256": source["prompt_sha256"], "inverse_verified": True}],
            {"program_cluster_id": source["conditions"]["program_cluster_id"], "direction": source["conditions"]["direction"]},
        ))
    existing_families = {row["meaning_family_id"] for row in rows}
    for item in classes:
        if item["meaning_family_id"] in existing_families:
            continue
        eligible = []
        for surface in item["surfaces"]:
            sources = {record.get("source") for record in surface.get("provenances", [])}
            if sources & STRONG_PROVENANCE:
                eligible.append(surface)
        if not eligible:
            continue
        eligible = [value for value in eligible if morphology_verdict(value["surface"])["verdict"] == "PASS"]
        if not eligible:
            continue
        chosen = sorted(eligible, key=lambda value: value["surface_sha256"])[0]
        rows.append(calibration_row(
            chosen["surface"], item["approved_response_ir"], item["meaning_family_id"], item["split"],
            "INVERSE_APPROVED_STRONG_PROVENANCE_EVIDENCE", chosen.get("provenances", []),
        ))

    # Preserve family distinctions but deduplicate byte-identical surfaces in
    # the human review queue to avoid wasting reviews.
    deduped: list[dict] = []
    seen = set()
    for row in rows:
        key = (row["meaning_family_id"], row["surface_sha256"], row["origin"])
        if key not in seen:
            seen.add(key)
            deduped.append(row)
    rows = deduped
    counts = Counter(row["origin"] for row in rows)
    artifact = {
        "schema": "BCORE.NATURALNESS_ACCEPTANCE_CALIBRATION_SET.V1",
        "runtime_installation": "FORBIDDEN",
        "production_promotion": "FORBIDDEN",
        "self_certified_program_positive_count": 0,
        "candidate_count": len(rows),
        "unrelated_meaning_family_count": len({row["meaning_family_id"] for row in rows}),
        "origin_counts": dict(counts),
        "speech_act_counts": dict(Counter(row["speech_act"] for row in rows)),
        "split_counts": dict(Counter(row["split"] for row in rows)),
        "rows": rows,
        "artifact_sha256": "",
    }
    artifact["artifact_sha256"] = digest(artifact)

    by_hash = {}
    benchmark_path = args.existing_human_packet.parent / "benchmark_candidates_ko_v1.json"
    if benchmark_path.exists():
        for row in json.loads(benchmark_path.read_text(encoding="utf-8"))["rows"]:
            by_hash[surface_hash(row["surface"])] = row
    for row in rows:
        by_hash.setdefault(row["surface_sha256"], row)
    review_items = []
    for old in old_packet["items"]:
        source = by_hash.get(old["surface_sha256"])
        if source is None:
            raise RuntimeError(f"HUMAN_PACKET_SOURCE_NOT_FOUND:{old['review_id']}")
        ir = source.get("approved_response_ir", {})
        review_items.append({
            "review_id": old["review_id"],
            "surface_sha256": old["surface_sha256"],
            "surface": old["surface"],
            "canonical_summary": {
                "speech_act": ir.get("speech_act"), "operation": ir.get("operation"),
                "discourse_relation": ir.get("discourse_relation"), "claims": claim_summary(ir),
            },
            "natural_korean": None,
            "meaning_preserved": None,
            "contextually_usable": None,
            "acceptability": None,
            "notes": None,
        })
    human_packet = {
        "schema": "BCORE.HUMAN_NATURALNESS_CALIBRATION_PACKET.V2",
        "blinded_from_expected_class_program_origin_and_automatic_verdict": True,
        "allowed_acceptability": ["NATURAL", "AWKWARD_BUT_GRAMMATICAL", "UNNATURAL"],
        "reviewer": {"kind": "HUMAN", "reviewer_id": None, "attestation": None, "reviewed_at": None},
        "items": review_items,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(artifact, ensure_ascii=False, indent=2), encoding="utf-8")
    args.human_packet_output.write_text(json.dumps(human_packet, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "candidates": len(rows), "families": artifact["unrelated_meaning_family_count"],
        "origins": artifact["origin_counts"], "human_packet_items": len(review_items),
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
