from __future__ import annotations

import argparse
import json
from pathlib import Path


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", required=True)
    args = parser.parse_args()
    root = Path(args.report).resolve(strict=True)
    raw = load(root / "final_raw_results.json")
    quality = load(root / "quality_gate_receipt.json")
    verifier = load(root / "independent_verifier_ablation.json")
    defect = load(root / "defect_contract_ablation.json")
    spec = load(root / "repair_spec_ablation.json")
    graft = load(root / "coding_graft_self_repair_ablation.json")
    levels = {}
    levels["A"] = raw["l1_tasks"] >= 1 and raw["l1_verified_repairs"] == raw["l1_tasks"] and raw["l1_false_repairs"] == 0
    levels["B"] = raw["l2_tasks"] >= 1 and raw["l2_defect_contract_to_repair_spec_success"] == raw["l2_tasks"] and raw["l2_verified_repairs"] == raw["l2_tasks"]
    levels["C"] = raw["l3_tasks"] >= 1 and raw["l3_observation_to_defect_contract_success"] >= raw["l3_tasks"] and raw["l3_defect_contract_to_repair_spec_success"] >= raw["l3_tasks"]
    levels["D"] = raw["final_tasks"] >= 1 and raw["final_verified_repairs"] == raw["final_tasks"] and raw["complete_self_repair_chains"] >= 1
    levels["E"] = verifier["independent_verifier_causal_value_pass"] is True and verifier["verifier_false_accept_events"] == 0 and raw["core_self_approval_events"] == 0
    levels["F"] = raw["verified_install_events"] >= raw["final_tasks"] and raw["install_without_valid_verification_receipt"] == 0 and raw["unverified_install_events"] == 0 and raw["rollback_available"] is True
    levels["G"] = raw["post_install_regression_pass"] is True and raw["post_install_self_repair_continuity_pass"] is True
    post_final_zero = all(value == 0 for key, value in raw.items() if key.startswith("post_final_"))
    levels["H"] = defect["pass"] is True and spec["pass"] is True and graft["pass"] is True and verifier["independent_verifier_causal_value_pass"] is True and raw["full_source_scan_events"] == 0 and raw["gold_patch_reads"] == 0 and post_final_zero and quality["status"] == "PASS"
    status = "PASS" if all(levels.values()) else "FAIL"
    boundaries = {
        "A": "REPAIR_SPEC_TO_PATCH_LIMIT",
        "B": "DEFECT_CONTRACT_TO_REPAIR_SPEC_LIMIT",
        "C": "OBSERVATION_TO_DEFECT_CONTRACT_LIMIT",
        "D": "PATCH_IMPLEMENTATION_LIMIT",
        "E": "INDEPENDENT_VERIFICATION_LIMIT",
        "F": "INSTALLATION_TRUST_BOUNDARY_LIMIT",
        "G": "SELF_REPAIR_CONTINUITY_LIMIT",
        "H": "SPARSE_SOURCE_ROUTING_LIMIT",
    }
    boundary = "NONE" if status == "PASS" else next(boundaries[level] for level in "ABCDEFGH" if not levels[level])
    output = {
        "schema_version": "B_CORE_RSI_CONTRACT_SECONDARY_ACCEPTANCE_1",
        "status": status,
        "disposition": "EXPLICIT_INDEPENDENTLY_VERIFIED_SELF_REPAIR_PIPELINE_ESTABLISHED" if status == "PASS" else boundary,
        "levels": levels,
        "next_dominant_growth_limit": boundary,
        "reads_primary_acceptance": False,
    }
    (root / "secondary_acceptance.json").write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(output, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
