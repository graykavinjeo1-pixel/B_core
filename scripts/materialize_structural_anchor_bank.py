"""Materialize a provenance-bound structural anchor bank for offline rewrites.

Surfaces are retained only because a later calibration needs an approved source
S to rewrite.  This script never creates runtime templates or language rules.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path

from analyze_paired_teacher_state_sensitivity import features

SOURCES = (
    (
        "existing_neutral",
        Path(r"D:\B_Core_validation\expression_primitive_learning_v1\expression_primitive_candidates_ko_v1.json"),
        Path(r"D:\B_Core_validation\expression_primitive_learning_v1\expression_primitive_gated_ko_v1.json"),
    ),
    (
        "runtime_composition",
        Path(r"D:\B_Core_validation\expression_rewrite_calibration_v1\recovered_runtime_anchor_candidates_ko_v1.json"),
        Path(r"D:\B_Core_validation\expression_rewrite_calibration_v1\recovered_runtime_anchor_gated_ko_v1.json"),
    ),
    (
        "independent_state",
        Path(r"D:\B_Core_validation\natural_state_language_grounding_v1\natural_state_language_grounding_candidates_ko_v2.json"),
        Path(r"D:\B_Core_validation\natural_state_language_grounding_v1\natural_state_language_grounding_gated_ko_v2.json"),
    ),
    (
        "paired_state",
        Path(r"D:\B_Core_validation\multi_factor_language_state_v1\paired_teacher_state_sensitivity_candidates_ko_v1.json"),
        Path(r"D:\B_Core_validation\multi_factor_language_state_v1\paired_teacher_state_sensitivity_gated_ko_v1.json"),
    ),
    (
        "speech_act_coverage",
        Path(r"D:\B_Core_validation\multi_factor_language_state_v1\speech_act_state_relevance_candidates_ko_v1.json"),
        Path(r"D:\B_Core_validation\multi_factor_language_state_v1\speech_act_state_relevance_gated_ko_v1.json"),
    ),
    (
        "targeted_structural_gap",
        Path(r"D:\B_Core_validation\expression_rewrite_calibration_v1\structural_gap_anchor_candidates_ko_v1.json"),
        Path(r"D:\B_Core_validation\expression_rewrite_calibration_v1\structural_gap_anchor_gated_ko_v1.json"),
    ),
)


def digest(value: object) -> str:
    return hashlib.sha256(
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def structural_metadata(row: dict[str, object]) -> dict[str, object]:
    surface = str(row["surface"])
    feature = features(surface)
    response = row.get("approved_response_ir", {})
    claims = response.get("claims", []) if isinstance(response, dict) else []
    relation = response.get("discourse_relation", "STATEMENT") if isinstance(response, dict) else "UNKNOWN"
    planner_relation = row.get("planner_relation")
    discourse_marker = feature["discourse_marker"]
    if "따라서" in surface:
        discourse_marker = "따라서"
    elif "이 조건이 충족되면" in surface:
        discourse_marker = "이 조건이 충족되면"
    return {
        # Claim count is the authoritative minimum clause demand, unlike
        # markdown punctuation that also sees headings, lists and tables.
        "semantic_clause_count": len(claims),
        "surface_sentence_count": feature["clause_count"],
        "explicit_subject": not feature["subject_omitted"],
        "discourse_relation": relation,
        "discourse_marker": discourse_marker,
        "ending": feature["ending_family"],
        "connective_present": any(token in surface for token in ("그리고", "하지만", "그래서", "다만", "따라서")),
        "modifier_rich": len(surface) >= 20,
        "planner_relation": planner_relation,
    }


def category_families(anchors: list[dict[str, object]]) -> dict[str, set[str]]:
    categories: dict[str, set[str]] = defaultdict(set)
    for anchor in anchors:
        family = str(anchor["meaning_family_id"])
        shape = anchor["structure"]
        clause_count = int(shape["semantic_clause_count"])
        categories["one_clause" if clause_count == 1 else "two_clause" if clause_count == 2 else "three_plus_clause"].add(family)
        categories["explicit_subject" if shape["explicit_subject"] else "omitted_subject"].add(family)
        categories["discourse_present" if shape["discourse_marker"] != "NONE" else "discourse_absent"].add(family)
        categories[f"ending_{str(shape['ending']).lower()}"].add(family)
    return categories


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    anchors: list[dict[str, object]] = []
    excluded: Counter[str] = Counter()
    for source_name, candidate_path, gate_path in SOURCES:
        if not candidate_path.exists() or not gate_path.exists():
            excluded[f"{source_name}:MISSING_ARTIFACT"] += 1
            continue
        candidates = json.loads(candidate_path.read_text(encoding="utf-8")).get("rows", [])
        gated = json.loads(gate_path.read_text(encoding="utf-8"))
        approved = {
            row["prompt_sha256"]
            for row in gated
            if row.get("approval") == "APPROVED_STATE_CONTRAST_GOLD"
        }
        for row in candidates:
            if row.get("prompt_sha256") not in approved:
                excluded[f"{source_name}:INVERSE_REJECTED"] += 1
                continue
            if row.get("factor") == "EXPRESSION_PRIMITIVE" and row.get("value") != "NEUTRAL":
                excluded[f"{source_name}:EXPRESSION_OPERATION_SOURCE"] += 1
                continue
            if not row.get("surface") or not row.get("approved_response_ir"):
                excluded[f"{source_name}:MISSING_PROVENANCE_OR_SURFACE"] += 1
                continue
            response = row["approved_response_ir"]
            anchors.append(
                {
                    "meaning_family_id": row["meaning_family_id"],
                    "speech_act": row.get("speech_act"),
                    "surface": row["surface"],
                    "surface_sha256": hashlib.sha256(row["surface"].encode()).hexdigest(),
                    "canonical_hash": digest(response),
                    "inverse_exact": True,
                    "unsupported_fact_count": 0,
                    "source_artifact": source_name,
                    "source_candidate_path": str(candidate_path),
                    "source_gate_path": str(gate_path),
                    "source_prompt_sha256": row["prompt_sha256"],
                    "surface_storage_purpose": "OFFLINE_PROVENANCE_BOUND_REWRITE_SOURCE_ONLY",
                    "runtime_template_installation": "FORBIDDEN",
                    "structure": structural_metadata(row),
                }
            )

    categories = category_families(anchors)
    families = {str(anchor["meaning_family_id"]) for anchor in anchors}
    speech_acts = {str(anchor.get("speech_act")) for anchor in anchors}
    required = (
        "one_clause",
        "two_clause",
        "three_plus_clause",
        "explicit_subject",
        "omitted_subject",
        "discourse_present",
        "discourse_absent",
        "ending_formal",
        "ending_polite",
    )
    ready = (
        len(families) >= 20
        and {"INFORM", "QUERY", "REQUEST", "PROMISE", "REASSURE"}.issubset(speech_acts)
        and all(len(categories[name]) >= 2 for name in required)
    )
    matrix = {
        name: {"family_count": len(values), "families": sorted(values)}
        for name, values in sorted(categories.items())
    }
    artifact = {
        "schema": "BCORE.STRUCTURAL_ANCHOR_BANK.V2",
        "prior_anchor_bank": r"D:\B_Core_validation\expression_rewrite_calibration_v1\cross_family_trusted_anchor_bank_ko_v1.json",
        "runtime_installation": "FORBIDDEN",
        "expression_primitive_promotion": "FORBIDDEN",
        "anchor_count": len(anchors),
        "unrelated_meaning_family_count": len(families),
        "source_counts": dict(Counter(anchor["source_artifact"] for anchor in anchors)),
        "speech_act_counts": dict(Counter(str(anchor.get("speech_act")) for anchor in anchors)),
        "coverage_matrix": matrix,
        "missing_required_categories": [name for name in required if len(categories[name]) < 2],
        "readiness": "STRUCTURAL_ANCHOR_BANK_READY" if ready else "INSUFFICIENT_STRUCTURAL_DIVERSITY",
        "excluded_counts": dict(excluded),
        "anchors": anchors,
        "artifact_sha256": "",
    }
    artifact["artifact_sha256"] = digest(artifact)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(artifact, ensure_ascii=False, indent=2), encoding="utf-8")
    print(
        json.dumps(
            {
                "anchors": artifact["anchor_count"],
                "families": artifact["unrelated_meaning_family_count"],
                "missing": artifact["missing_required_categories"],
                "readiness": artifact["readiness"],
            },
            ensure_ascii=False,
        )
    )


if __name__ == "__main__":
    main()
