"""Compile teacher candidates into paired approved state-contrast gold.

Only rows whose canonical inverse gate passed are retained.  The artifact keeps
the pairwise transition; it is not a runtime surface cache.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from collections import defaultdict
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--gated", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    gated = json.loads(args.gated.read_text(encoding="utf-8"))
    groups: dict[tuple[str, str], list[dict]] = defaultdict(list)
    for row in gated:
        if row["approval"] == "APPROVED_STATE_CONTRAST_GOLD":
            paired_id = row.get("paired_observation_id")
            group_id = paired_id if paired_id else row["meaning_family_id"]
            groups[(group_id, row["factor"])].append(row)
    pairs = []
    incomplete = []
    for (group_id, factor), rows in sorted(groups.items()):
        values = sorted(rows, key=lambda row: row.get("paired_side") or row["value"])
        if len(values) < 2:
            incomplete.append({"group_id": group_id, "factor": factor, "approved_values": [row["value"] for row in values]})
            continue
        for left, right in zip(values[::2], values[1::2]):
            pairs.append({
                "paired_observation_id": left.get("paired_observation_id"),
                "acquisition_protocol": left.get("acquisition_protocol"),
                "contrast_kind": left.get("contrast_kind"),
                "interaction_holdout": left.get("interaction_holdout", False),
                "meaning_family_id": left["meaning_family_id"],
                "split": left["split"],
                "factor": factor,
                "speech_act": left.get("speech_act"),
                "left_side": left.get("paired_side"),
                "right_side": right.get("paired_side"),
                "left_value": left["value"],
                "right_value": right["value"],
                "left_conditions": left.get("conditions", {}),
                "right_conditions": right.get("conditions", {}),
                "left_surface": left["surface"],
                "right_surface": right["surface"],
                "left_inverse": left["canonical_inverse"],
                "right_inverse": right["canonical_inverse"],
                "claim_same": left["claim_same"] and right["claim_same"],
                "polarity_same": left["polarity_same"] and right["polarity_same"],
                "modality_same": left["modality_same"] and right["modality_same"],
                "event_phase_same": left["event_phase_same"] and right["event_phase_same"],
                "unsupported_fact_count": left["unsupported_fact_count"] + right["unsupported_fact_count"],
                "teacher_model": left["teacher_model"],
                "surface_pair_changed": left["surface"] != right["surface"],
            })
    result = {
        "schema": "BCORE.APPROVED_STATE_CONTRAST_GOLD.V1",
        "meaning_authority": "APPROVED_CANONICAL_RESPONSE_IR",
        "approval_authority": "RUST_CANONICAL_INVERSE_GATE",
        "surface_authority": "TEACHER_EVIDENCE_ONLY_NO_RUNTIME_TEMPLATE_OR_CACHE",
        "candidate_count": len(gated),
        "approved_candidate_count": sum(row["approval"] == "APPROVED_STATE_CONTRAST_GOLD" for row in gated),
        "rejected_candidate_count": sum(row["approval"] != "APPROVED_STATE_CONTRAST_GOLD" for row in gated),
        "paired_contrast_count": len(pairs),
        "incomplete_pairs": incomplete,
        "pairs": pairs,
        "artifact_sha256": "",
    }
    result["artifact_sha256"] = hashlib.sha256(json.dumps(result, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"output": str(args.output), "candidates": result["candidate_count"], "approved": result["approved_candidate_count"], "pairs": result["paired_contrast_count"], "artifact_sha256": result["artifact_sha256"]}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
