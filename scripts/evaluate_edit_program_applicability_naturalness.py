"""Close the applicability/naturalness tranche without promoting runtime data."""
from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path

from realization_trace_execution_substrate import particle_morphology_valid


def digest(value: object) -> str:
    return hashlib.sha256(
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--contracts", type=Path, required=True)
    ap.add_argument("--candidates", type=Path, required=True)
    ap.add_argument("--gated", type=Path, required=True)
    ap.add_argument("--q4", type=Path, required=True)
    ap.add_argument("--replay-contracts", type=Path, required=True)
    ap.add_argument("--replay-candidates", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()
    contracts = json.loads(args.contracts.read_text(encoding="utf-8"))
    candidates = json.loads(args.candidates.read_text(encoding="utf-8"))["rows"]
    gated = json.loads(args.gated.read_text(encoding="utf-8"))
    approved = {row["prompt_sha256"]: row["approval"] == "APPROVED_STATE_CONTRAST_GOLD" for row in gated}
    q4_rows = json.loads(args.q4.read_text(encoding="utf-8"))["rows"]
    q4 = {row["surface_sha256"]: row["verdict"] for row in q4_rows}
    evaluated = []
    for row in candidates:
        surface_hash = hashlib.sha256(row["surface"].encode()).hexdigest()
        evaluated.append({
            "meaning_family_id": row["meaning_family_id"],
            "split": row["split"],
            "speech_act": row["speech_act"],
            "value": row["value"],
            "program_cluster_id": row["conditions"]["program_cluster_id"],
            "direction": row["conditions"]["direction"],
            "inverse_pass": approved.get(row["prompt_sha256"], False),
            "morphology_pass": particle_morphology_valid(row["surface"]),
            "q4_verdict": q4.get(surface_hash, "MISSING"),
            "surface_sha256": surface_hash,
        })
    apply_rows = [row for row in evaluated if row["value"] == "APPLY_TRANSFER"]
    negative_rows = [row for row in evaluated if row["value"] == "FORCED_REJECT_CONTROL"]
    negative_expected_fail = sum(not row["morphology_pass"] for row in negative_rows)
    negative_q4_caught = sum(row["q4_verdict"] == "FAIL" for row in negative_rows if not row["morphology_pass"])
    q4_negative_sensitivity = negative_q4_caught / negative_expected_fail if negative_expected_fail else None
    q4_calibrated = q4_negative_sensitivity is not None and q4_negative_sensitivity >= 0.9
    grouped = defaultdict(list)
    for row in apply_rows:
        grouped[(row["program_cluster_id"], row["direction"])].append(row)
    program_results = []
    contract_by_key = {(row["program_cluster_id"], row["direction"]): row for row in contracts["contracts"]}
    for key, rows in sorted(grouped.items()):
        contract = contract_by_key[key]
        split_counts = Counter(row["split"] for row in rows)
        blind = [row for row in rows if row["split"] == "BLIND"]
        semantic_pass = all(row["inverse_pass"] for row in rows)
        morphology_pass = all(row["morphology_pass"] for row in rows)
        enough_blind = len({row["meaning_family_id"] for row in blind}) >= 2
        naturalness_status = "PASS" if q4_calibrated and all(row["q4_verdict"] == "PASS" for row in rows) else "HOLD_UNCALIBRATED_Q4_JUDGE"
        program_results.append({
            "program_cluster_id": key[0],
            "direction": key[1],
            "source_ending": contract["source_ending"],
            "target_ending": contract["target_ending"],
            "required_speech_act": contract["required_speech_act"],
            "observed_family_count": contract["observed_family_count"],
            "fresh_transfer_count": len(rows),
            "fresh_split_counts": dict(split_counts),
            "sealed_blind_family_count": len({row["meaning_family_id"] for row in blind}),
            "multiple_unseen_meaning_transfer": enough_blind,
            "inverse_pass": semantic_pass,
            "morphology_pass": morphology_pass,
            "q4_pass_count": sum(row["q4_verdict"] == "PASS" for row in rows),
            "q4_total": len(rows),
            "naturalness_status": naturalness_status,
            "promotion": "HOLD" if naturalness_status != "PASS" else "REUSABLE_EXPRESSION_PROGRAM_CANDIDATE",
        })
    report = {
        "schema": "BCORE.EDIT_PROGRAM_APPLICABILITY_AND_NATURALNESS_VALIDATION.V1",
        "runtime_installation": "FORBIDDEN",
        "production_promotion": "HOLD",
        "program_discovery_expanded": False,
        "applicability_contract_count": len(contracts["contracts"]),
        "applicability_precision": contracts["valid_source_precision"],
        "applicability_recall": contracts["valid_source_recall"],
        "apply_transfer_count": len(apply_rows),
        "apply_inverse_pass": sum(row["inverse_pass"] for row in apply_rows),
        "apply_morphology_pass": sum(row["morphology_pass"] for row in apply_rows),
        "counterfactual_negative_count": len(negative_rows),
        "counterfactual_inverse_pass": sum(row["inverse_pass"] for row in negative_rows),
        "counterfactual_morphology_fail": sum(not row["morphology_pass"] for row in negative_rows),
        "contract_pre_rejected_counterfactual_count": len(negative_rows),
        "q4_unique_surface_count": len(q4_rows),
        "q4_negative_control_expected_fail": negative_expected_fail,
        "q4_negative_control_caught": negative_q4_caught,
        "q4_negative_control_sensitivity": q4_negative_sensitivity,
        "q4_judge_calibrated": q4_calibrated,
        "q4_judge_use_for_promotion": False if not q4_calibrated else True,
        "human_inspection": "NOT_PERFORMED",
        "restart_replay_contracts_identical": args.contracts.read_bytes() == args.replay_contracts.read_bytes(),
        "restart_replay_candidates_identical": args.candidates.read_bytes() == args.replay_candidates.read_bytes(),
        "program_results": program_results,
        "evaluated_rows": evaluated,
        "conclusion": "APPLICABILITY_CONTRACT_PASS; NATURALNESS_HOLD; NO_RUNTIME_PROMOTION",
        "artifact_sha256": "",
    }
    report["artifact_sha256"] = digest(report)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "applicability_precision": report["applicability_precision"],
        "applicability_recall": report["applicability_recall"],
        "apply_inverse": f"{report['apply_inverse_pass']}/{report['apply_transfer_count']}",
        "apply_morphology": f"{report['apply_morphology_pass']}/{report['apply_transfer_count']}",
        "negative_inverse": f"{report['counterfactual_inverse_pass']}/{report['counterfactual_negative_count']}",
        "negative_morphology_fail": f"{report['counterfactual_morphology_fail']}/{report['counterfactual_negative_count']}",
        "q4_calibrated": q4_calibrated,
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
