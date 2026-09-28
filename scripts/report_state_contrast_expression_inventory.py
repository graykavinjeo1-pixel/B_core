"""Build the sealed shadow report for state contrast expression inventory learning.

The report contains hashes and aggregate measurements only.  Teacher surface
sentences are deliberately excluded from the durable runtime artifact.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gold", type=Path, required=True)
    ap.add_argument("--analysis", type=Path, required=True)
    ap.add_argument("--inventory", type=Path, required=True)
    ap.add_argument("--controller", type=Path, required=True)
    ap.add_argument("--arms", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()

    gold = load(args.gold)
    analysis = load(args.analysis)
    inventory = load(args.inventory)
    controller = load(args.controller)
    arms = load(args.arms)

    relationship = analysis["factor_summary"]["relationship_social_distance"]
    result = {
        "schema": "BCORE.STATE_CONTRAST_EXPRESSION_INVENTORY_LEARNING.V1",
        "source_gold_artifact_sha256": gold["artifact_sha256"],
        "source_analysis_artifact_sha256": analysis["artifact_sha256"],
        "expression_inventory_artifact_sha256": inventory["artifact_sha256"],
        "controller_artifact_sha256": controller["artifact_sha256"],
        "approved_pair_count": len(gold["pairs"]),
        "relationship_pair_summary": relationship,
        "inventory_rule_count": len(inventory["rules"]),
        "inventory_rules": [
            {
                "transformation_id": rule["transformation_id"],
                "operation": rule["operation"],
                "source_transition_count": rule["source_transition_count"],
                "surface_cache": rule["surface_cache"],
            }
            for rule in inventory["rules"]
        ],
        "teacher_gold_runtime_arms": {
            "artifact_sha256": arms["artifact_sha256"],
            "sealed_blind_pairs": arms["sealed_blind_pairs"],
            "factor_summary": arms["factor_summary"],
        },
        "semantic_authority": "B_CORE_CANONICAL_INVERSE",
        "runtime_surface_authority": "STRUCTURAL_TRANSFORMATIONS_ONLY_NO_TEACHER_SURFACE_CACHE",
        "dialect_status": "DIALECT_HOLD",
        "controller_status": "RESEARCH_SHADOW_ONLY",
        "promotion": "HOLD_ON_NOT_GREATER_THAN_OFF",
        "artifact_sha256": "",
    }
    result["artifact_sha256"] = hashlib.sha256(
        json.dumps(result, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"output": str(args.output), "artifact_sha256": result["artifact_sha256"]}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
