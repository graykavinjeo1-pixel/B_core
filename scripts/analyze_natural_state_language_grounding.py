"""Measure independent state/surface evidence before any controller learning.

This consumes only inverse-approved observations.  It reports repeated state
effects against same-state, wording, omitted-state, and pseudo-label controls;
it does not create a language-state mapping or runtime inventory.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path

from analyze_paired_teacher_state_sensitivity import features, structural_delta


def invariant(row: dict) -> bool:
    return (
        row["canonical_inverse"] and row["claim_same"] and row["polarity_same"]
        and row["modality_same"] and row["event_phase_same"] and row["unsupported_fact_count"] == 0
    )


def signature(left: str, right: str) -> str:
    delta = structural_delta(features(left), features(right))
    return "IDENTICAL" if not delta else "|".join(f"{key}:{value}" for key, value in sorted(delta.items()))


def comparison(rows: list[dict], left_mode: str, right_mode: str, require_values_differ: bool) -> list[dict]:
    by_key: dict[tuple[str, str, str], list[dict]] = defaultdict(list)
    for row in rows:
        by_key[(row["meaning_family_id"], row["factor"], row["split"])].append(row)
    result = []
    for (family, factor, split), items in by_key.items():
        left = [item for item in items if item.get("acquisition_mode") == left_mode]
        right = [item for item in items if item.get("acquisition_mode") == right_mode]
        if left_mode == right_mode:
            # One observation per value: compare the two configured state values.
            values = sorted({item["value"] for item in left})
            if len(values) != 2:
                continue
            a = next(item for item in left if item["value"] == values[0])
            b = next(item for item in left if item["value"] == values[1])
        else:
            pairs = [(a, b) for a in left for b in right if a["value"] == b["value"]]
            if not pairs:
                continue
            for a, b in pairs:
                result.append({
                    "meaning_family_id": family, "factor": factor, "split": split, "left_value": a["value"],
                    "right_value": b["value"], "signature": signature(a["surface"], b["surface"]),
                    "surface_changed": a["surface"] != b["surface"],
                })
            continue
        if require_values_differ and a["value"] == b["value"]:
            continue
        result.append({
            "meaning_family_id": family, "factor": factor, "split": split, "left_value": a["value"],
            "right_value": b["value"], "signature": signature(a["surface"], b["surface"]),
            "surface_changed": a["surface"] != b["surface"],
        })
    return result


def same_state(rows: list[dict]) -> list[dict]:
    indexed = {(row["meaning_family_id"], row["factor"], row["value"], row["split"], row.get("acquisition_mode")): row for row in rows}
    result = []
    for key, left in indexed.items():
        family, factor, value, split, mode = key
        if mode != "DIRECT_A":
            continue
        right = indexed.get((family, factor, value, split, "DIRECT_B"))
        if right is None:
            continue
        result.append({
            "meaning_family_id": family, "factor": factor, "value": value, "split": split,
            "signature": signature(left["surface"], right["surface"]), "surface_changed": left["surface"] != right["surface"],
        })
    return result


def summary(items: list[dict]) -> dict:
    signatures = Counter(item["signature"] for item in items)
    changed = [item for item in items if item["surface_changed"]]
    return {
        "comparison_count": len(items),
        "surface_change_count": len(changed),
        "surface_change_rate": len(changed) / len(items) if items else None,
        "signature_counts": dict(signatures),
        "split_counts": {split: sum(item["split"] == split for item in items) for split in ("TRAIN", "VALIDATION", "BLIND")},
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidates", type=Path, required=True)
    parser.add_argument("--gated", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    candidates = json.loads(args.candidates.read_text(encoding="utf-8"))
    gated = json.loads(args.gated.read_text(encoding="utf-8"))
    approved = [row for row in gated if row["approval"] == "APPROVED_STATE_CONTRAST_GOLD"]
    rejected = [row for row in gated if row["approval"] != "APPROVED_STATE_CONTRAST_GOLD"]
    if not all(invariant(row) for row in approved):
        raise ValueError("approved row violated semantic invariant")

    direct_difference = comparison(approved, "DIRECT_A", "DIRECT_A", True)
    descriptive_difference = comparison(approved, "DESCRIPTIVE", "DESCRIPTIVE", True)
    omitted_difference = comparison(approved, "OMITTED", "OMITTED", True)
    pseudo_difference = comparison(approved, "PSEUDO", "PSEUDO", True)
    wording_difference = same_state(approved)
    direct_descriptive = comparison(approved, "DIRECT_A", "DESCRIPTIVE", False)

    by_factor: dict[str, dict] = {}
    factors = sorted({row["factor"] for row in gated})
    for factor in factors:
        direct = [item for item in direct_difference if item["factor"] == factor]
        descriptive = [item for item in descriptive_difference if item["factor"] == factor]
        omitted = [item for item in omitted_difference if item["factor"] == factor]
        pseudo = [item for item in pseudo_difference if item["factor"] == factor]
        wording = [item for item in wording_difference if item["factor"] == factor]
        explanation = [item for item in direct_descriptive if item["factor"] == factor]
        train_signatures = {item["signature"] for item in direct if item["split"] == "TRAIN" and item["signature"] != "IDENTICAL"}
        blind = [item for item in direct if item["split"] == "BLIND"]
        recurrent_blind = sum(item["signature"] in train_signatures for item in blind)
        raw_direct = [row for row in approved if row["factor"] == factor and row.get("acquisition_mode") in {"DIRECT_A", "DIRECT_B"}]
        status = "NO_APPROVED_STATE_EVIDENCE"
        if direct:
            status = "NATURAL_SIGNAL_UNCONFIRMED"
        if direct and descriptive and any(item["surface_changed"] for item in direct) and any(item["surface_changed"] for item in descriptive):
            status = "NATURAL_SIGNAL_CANDIDATE_REQUIRES_SEALED_REPLICATION"
        by_factor[factor] = {
            "approved_surface_count": len(raw_direct),
            "direct_state_difference": summary(direct),
            "same_state_prompt_permutation": summary(wording),
            "descriptive_state_difference": summary(descriptive),
            "direct_vs_descriptive_same_condition": summary(explanation),
            "omitted_state_negative_control": summary(omitted),
            "pseudo_label_negative_control": summary(pseudo),
            "blind_direct_recurrent_signature_count": recurrent_blind,
            "blind_direct_comparison_count": len(blind),
            "status": status,
        }

    report = {
        "schema": "BCORE.NATURAL_STATE_LANGUAGE_GROUNDING_REPORT.V1",
        "acquisition_protocol": candidates["acquisition_protocol"], "teacher_model": candidates["teacher_model"],
        "bf16_loaded": candidates["bf16_loaded"], "runtime_installation": "FORBIDDEN",
        "paired_condition_prompt": candidates["paired_condition_prompt"], "teacher_forced_difference": candidates["teacher_forced_difference"],
        "fresh_context_per_observation": candidates["fresh_context_per_observation"],
        "condition_order_randomized": candidates["condition_order_randomized"],
        "condition_order_seed": candidates["condition_order_seed"],
        "requested_observation_count": candidates["requested_observation_count"], "candidate_surface_count": len(candidates["rows"]),
        "inverse_approved_surface_count": len(approved), "inverse_rejected_surface_count": len(rejected),
        "semantic_invariance_approved_all": all(invariant(row) for row in approved),
        "factor_summary": by_factor,
        "controller_learning": "FORBIDDEN_UNTIL_SEALED_NATURAL_REPLICATION", "new_latent": "FORBIDDEN",
        "raw_teacher_surface_runtime_installation": "FORBIDDEN", "artifact_sha256": "",
    }
    report["artifact_sha256"] = hashlib.sha256(json.dumps(report, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"output": str(args.output), "approved": len(approved), "rejected": len(rejected),
                      "factors": {factor: value["status"] for factor, value in by_factor.items()},
                      "artifact_sha256": report["artifact_sha256"]}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
