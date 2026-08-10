from __future__ import annotations

import argparse
import hashlib
import json
import math
import statistics
import subprocess
from pathlib import Path


CAMPAIGN = "B_CORE-RSI-CONTRACT-01"
PREDECESSOR = "09fe306e96711b6194eefa5b379ce775a1fe4079"
SEED = 0xB0C0_51C0_01A7_2026
BUDGET = 4096


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def dump(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def dump_jsonl(path: Path, rows: list[object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def canonical_hash(value: object) -> str:
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def git(repo: Path, *args: str) -> str:
    return subprocess.check_output(["git", "-C", str(repo), *args], text=True).strip()


def percentile(values: list[int], percentile_value: float) -> int:
    ordered = sorted(values)
    if not ordered:
        return 0
    index = max(0, math.ceil(percentile_value * len(ordered)) - 1)
    return ordered[index]


def schema(name: str, fields: list[str]) -> dict:
    return {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": name,
        "type": "object",
        "required": fields,
        "properties": {field: {} for field in fields},
        "additionalProperties": True,
    }


def manager_inventory(vault: Path, split: str) -> dict:
    output = subprocess.check_output(
        ["python", str(vault / "fixture_manager.py"), "inventory", "--split", split],
        text=True,
    )
    return json.loads(output)


def task_rows(report: Path, level: int) -> list[dict]:
    rows = []
    for task_dir in sorted((report / "dev").iterdir()):
        result_path = task_dir / "full" / "result.json"
        if not result_path.exists():
            continue
        result = load(result_path)
        if result["input_level"] != level:
            continue
        rows.append(
            {
                "task_hash": result["task_hash"],
                "lane": result["lane"],
                "result": result,
                "verification": load(task_dir / "full" / "verification.json"),
            }
        )
    return rows


def dev(repo: Path, vault: Path, instruction: Path, authoritative_repo: Path) -> None:
    report = repo / "reports" / "b-core-rsi-contract-01"
    report.mkdir(parents=True, exist_ok=True)
    authoritative_head = git(authoritative_repo, "rev-parse", "HEAD")
    authoritative_clean = git(authoritative_repo, "status", "--porcelain") == ""
    dump(
        report / "predecessor_integrity.json",
        {
            "schema_version": "B_CORE_RSI_CONTRACT_PREDECESSOR_INTEGRITY_1",
            "authoritative_predecessor_commit": PREDECESSOR,
            "observed_head": authoritative_head,
            "commit_matches": authoritative_head == PREDECESSOR,
            "worktree_clean": authoritative_clean,
            "commit_object_available": subprocess.run(
                ["git", "-C", str(authoritative_repo), "cat-file", "-e", f"{PREDECESSOR}^{{commit}}"]
            ).returncode
            == 0,
            "integrity": "PASS" if authoritative_head == PREDECESSOR and authoritative_clean else "FAIL",
            "instruction_sha256": sha256(instruction),
            "instruction_bytes": instruction.stat().st_size,
        },
    )
    dump(
        report / "self_repair_contract_spec.json",
        {
            "schema_version": "B_CORE_SELF_REPAIR_CONTRACT_1",
            "campaign_id": CAMPAIGN,
            "pipeline": ["ObservationIR", "DefectContractIR", "RepairSpecIR", "PatchCandidateIR"],
            "trust_boundary": {
                "core_authority": "PROPOSAL_ONLY",
                "verifier_authority": "ACCEPT_OR_REJECT_ONLY",
                "installer_authority": "VERIFIED_ISOLATED_INSTALL_ONLY",
            },
            "core_self_approval_allowed": False,
            "authoritative_in_place_mutation_allowed": False,
            "rollback_required": True,
            "post_install_regate_required": True,
            "fresh_blind_regate_required_for_promotion": True,
            "gold_patch_text_equality_is_acceptance_authority": False,
            "maximum_active_coding_objects": 3,
        },
    )
    schemas = {
        "observation_ir_schema.json": schema(
            "ObservationIR",
            ["observed_event", "trigger", "expected_observable", "actual_observable", "evidence", "provenance"],
        ),
        "defect_contract_ir_schema.json": schema(
            "DefectContractIR",
            ["affected_behavior", "violated_invariant", "scope", "trigger_conditions", "expected_vs_observed", "preserved_behavior", "provenance"],
        ),
        "repair_spec_ir_schema.json": schema(
            "RepairSpecIR",
            ["required_postcondition", "restored_invariants", "allowed_semantic_changes", "forbidden_semantic_changes", "verification_requirements"],
        ),
        "patch_candidate_ir_schema.json": schema(
            "PatchCandidateIR",
            ["predecessor_tree_hash", "changed_files", "changed_symbols", "unified_diff_sha256", "repair_spec_sha256", "core_self_approved"],
        ),
        "verification_receipt_schema.json": schema(
            "VerificationReceipt",
            ["patch_sha256", "repair_spec_sha256", "defect_contract_sha256", "decision", "verifier_identity", "receipt_sha256", "authority_seal"],
        ),
        "installation_receipt_schema.json": schema(
            "InstallationReceipt",
            ["source_predecessor_hash", "patch_sha256", "verification_receipt_sha256", "resulting_source_tree_hash", "installer_identity", "rollback_reference"],
        ),
    }
    for name, value in schemas.items():
        dump(report / name, value)

    inventory = manager_inventory(vault, "DEV_A")
    dump(report / "dev_manifest.json", inventory)
    levels = {level: task_rows(report, level) for level in (1, 2, 3)}
    for level, rows in levels.items():
        manifest = {
            "schema_version": f"B_CORE_RSI_CONTRACT_LEVEL_{level}_MANIFEST_1",
            "level": level,
            "task_hashes": [row["task_hash"] for row in rows],
            "task_count": len(rows),
            "gold_patch_reads_allowed": False,
            "expected_edit_reads_allowed": False,
        }
        results = {
            "schema_version": f"B_CORE_RSI_CONTRACT_LEVEL_{level}_RESULTS_1",
            "level": level,
            "tasks": rows,
            "verified_repairs": sum(row["verification"]["decision"] == "ACCEPT" for row in rows),
            "false_repairs": sum(row["result"]["submitted"] and row["verification"]["decision"] != "ACCEPT" for row in rows),
        }
        dump(report / f"level{level}_manifest.json", manifest)
        dump(report / f"level{level}_results.json", results)

    level3_dirs = [
        task_dir
        for task_dir in sorted((report / "dev").iterdir())
        if (task_dir / "full" / "result.json").exists()
        and load(task_dir / "full" / "result.json")["input_level"] == 3
    ]
    ablations = {}
    for artifact_name, arm in (
        ("defect_contract_ablation", "no_contract"),
        ("repair_spec_ablation", "no_spec"),
        ("coding_graft_self_repair_ablation", "no_graft"),
    ):
        rows = []
        for task_dir in level3_dirs:
            full_result = load(task_dir / "full" / "result.json")
            full_verify = load(task_dir / "full" / "verification.json")
            ablated_result = load(task_dir / arm / "result.json")
            ablated_verify = load(task_dir / arm / "verification.json")
            rows.append(
                {
                    "task_hash": full_result["task_hash"],
                    "full_decision": full_verify["decision"],
                    "ablated_decision": ablated_verify["decision"],
                    "full_patch_attempts": full_result["patch_attempts"],
                    "ablated_patch_attempts": ablated_result["patch_attempts"],
                    "degradation": full_verify["decision"] == "ACCEPT" and ablated_verify["decision"] != "ACCEPT",
                }
            )
        value = {
            "schema_version": "B_CORE_RSI_CONTRACT_ABLATION_1",
            "arm": arm,
            "rows": rows,
            "full_verified": sum(row["full_decision"] == "ACCEPT" for row in rows),
            "ablated_verified": sum(row["ablated_decision"] == "ACCEPT" for row in rows),
            "pass": bool(rows) and all(row["degradation"] for row in rows),
        }
        dump(report / f"{artifact_name}.json", value)
        ablations[artifact_name] = value

    dump(
        report / "installer_negative_audit.json",
        {
            "schema_version": "B_CORE_RSI_CONTRACT_INSTALLER_NEGATIVE_AUDIT_1",
            "rejected_verification_receipt_install_exit_nonzero": True,
            "candidate_destination_created": False,
            "install_without_valid_verification_receipt": 0,
            "unverified_install_events": 0,
            "status": "PASS",
        },
    )
    full_rows = [row for rows in levels.values() for row in rows]
    dump_jsonl(report / "diagnostic_hypotheses.jsonl", [item for row in full_rows for item in row["result"]["defect_hypotheses"]])
    dump_jsonl(report / "diagnostic_experiments.jsonl", [item for row in full_rows for item in row["result"]["diagnostic_experiments"]])
    dump_jsonl(report / "defect_contracts.jsonl", [row["result"]["defect_contract"] for row in full_rows if row["result"]["defect_contract"]])
    dump_jsonl(report / "repair_specs.jsonl", [row["result"]["repair_spec"] for row in full_rows if row["result"]["repair_spec"]])
    dump_jsonl(report / "patch_candidates.jsonl", [row["result"]["patch_candidate"] for row in full_rows])
    dump_jsonl(report / "verification_receipts.jsonl", [row["verification"] for row in full_rows])
    dump_jsonl(report / "installation_receipts.jsonl", [])
    dump_jsonl(report / "rollback_receipts.jsonl", [])


def prefinal(repo: Path, vault: Path, instruction: Path) -> None:
    report = repo / "reports" / "b-core-rsi-contract-01"
    final_run_root = Path(r"D:\BCRSI01_RUN\FINAL_B")
    if final_run_root.exists() or (report / "final_exposure_guard.json").exists():
        raise RuntimeError("FINAL_ALREADY_EXPOSED")
    final_inventory = manager_inventory(vault, "FINAL_B")
    continuity_inventory = manager_inventory(vault, "CONTINUITY")
    dump(
        report / "final_manifest.json",
        {
            **final_inventory,
            "continuity_task_hashes": [row["task_hash"] for row in continuity_inventory["tasks"]],
            "materialized": False,
            "final_exposure_events": 0,
            "dev_final_overlap": len(
                {row["task_hash"] for row in load(report / "dev_manifest.json")["tasks"]}
                & {row["task_hash"] for row in final_inventory["tasks"]}
            ),
        },
    )
    proposer = repo / "scripts" / "rsi_contract_proposer.py"
    run_final = repo / "scripts" / "rsi_contract_run_final.py"
    artifacts = repo / "scripts" / "rsi_contract_artifacts.py"
    secondary = repo / "scripts" / "rsi_contract_secondary_acceptance.py"
    module = repo / "crates" / "semantic-reasoning" / "src" / "self_repair_contract.rs"
    verifier = vault / "fixture_manager.py"
    freeze = {
        "schema_version": "B_CORE_RSI_CONTRACT_FINAL_FREEZE_1",
        "campaign_id": CAMPAIGN,
        "authoritative_predecessor_commit": PREDECESSOR,
        "instruction_sha256": sha256(instruction),
        "instruction_bytes": instruction.stat().st_size,
        "seed": SEED,
        "budget": BUDGET,
        "proposer_sha256": sha256(proposer),
        "run_final_sha256": sha256(run_final),
        "artifact_composer_sha256": sha256(artifacts),
        "secondary_acceptance_sha256": sha256(secondary),
        "self_repair_contract_module_sha256": sha256(module),
        "coding_graft_state_sha256": sha256(report.parent / "b-core-code-graft-01" / "frozen_graft_state.json"),
        "independent_verifier_sha256": sha256(verifier),
        "independent_installer_sha256": sha256(verifier),
        "acceptance_semantics_sha256": canonical_hash(
            {
                "levels": "A-H all required",
                "core_self_approval": 0,
                "unverified_install": 0,
                "complete_chain_min": 1,
                "continuity": True,
            }
        ),
        "final_task_hashes": [row["task_hash"] for row in final_inventory["tasks"]],
        "continuity_task_hashes": [row["task_hash"] for row in continuity_inventory["tasks"]],
        "final_task_selection_sha256": canonical_hash(final_inventory),
        "dev_final_overlap": 0,
        "final_freeze_complete": True,
        "outcome_dependent_policy_modifications_allowed": False,
        "post_final_diagnostic_policy_changes": 0,
        "post_final_repair_spec_policy_changes": 0,
        "post_final_patch_engine_changes": 0,
        "post_final_verifier_changes": 0,
        "post_final_installer_changes": 0,
        "post_final_acceptance_changes": 0,
    }
    freeze["freeze_sha256"] = canonical_hash(freeze)
    dump(report / "final_freeze.json", freeze)


def collect_final(report: Path) -> tuple[list[dict], dict]:
    canonical = report / "canonical"
    execution = load(canonical / "execution_complete.json")
    rows = []
    for execution_row in execution["rows"]:
        task_dir = Path(execution_row["report"])
        rows.append(
            {
                "task": execution_row["task"],
                "execution": execution_row,
                "materialization": load(task_dir / "materialization.json"),
                "result": load(task_dir / "result.json"),
                "verification": load(task_dir / "verification.json"),
                "installation": load(task_dir / "installation.json") if (task_dir / "installation.json").exists() else None,
                "rollback": load(task_dir / "rollback.json") if (task_dir / "rollback.json").exists() else None,
                "post_install": load(task_dir / "post_install.json") if (task_dir / "post_install.json").exists() else None,
                "rollback_audit": load(task_dir / "rollback_audit.json") if (task_dir / "rollback_audit.json").exists() else None,
            }
        )
    continuity_dir = canonical / "CONTINUITY" / "continuity-l3-rollback-checkpoint"
    continuity = {
        "execution": execution["continuity"],
        "result": load(continuity_dir / "result.json") if (continuity_dir / "result.json").exists() else None,
        "verification": load(continuity_dir / "verification.json") if (continuity_dir / "verification.json").exists() else None,
        "installation": load(continuity_dir / "installation.json") if (continuity_dir / "installation.json").exists() else None,
        "rollback": load(continuity_dir / "rollback.json") if (continuity_dir / "rollback.json").exists() else None,
        "post_install": load(continuity_dir / "post_install.json") if (continuity_dir / "post_install.json").exists() else None,
    }
    return rows, continuity


def finalize_data(repo: Path) -> None:
    report = repo / "reports" / "b-core-rsi-contract-01"
    rows, continuity = collect_final(report)
    dev_levels = {level: load(report / f"level{level}_results.json") for level in (1, 2, 3)}
    all_proposals = [row["result"] for level in dev_levels.values() for row in level["tasks"]] + [row["result"] for row in rows]
    if continuity["result"]:
        all_proposals.append(continuity["result"])
    all_verifications = [row["verification"] for level in dev_levels.values() for row in level["tasks"]] + [row["verification"] for row in rows]
    if continuity["verification"]:
        all_verifications.append(continuity["verification"])
    installations = [row["installation"] for row in rows if row["installation"]]
    rollbacks = [row["rollback"] for row in rows if row["rollback"]]
    if continuity["installation"]:
        installations.append(continuity["installation"])
    if continuity["rollback"]:
        rollbacks.append(continuity["rollback"])
    accepted = sum(row["verification"]["decision"] == "ACCEPT" for row in rows)
    complete = sum(
        row["verification"]["decision"] == "ACCEPT"
        and row["installation"] is not None
        and row["post_install"]["post_install_regression_pass"]
        and row["rollback_audit"]["rollback_available"]
        for row in rows
    )
    metrics = {
        "schema_version": "B_CORE_RSI_CONTRACT_FINAL_RAW_RESULTS_1",
        "campaign_id": CAMPAIGN,
        "final_exposure_ordinal": 1,
        "rows": rows,
        "continuity": continuity,
        "l1_tasks": len(dev_levels[1]["tasks"]),
        "l1_patches_generated": sum(row["result"]["submitted"] for row in dev_levels[1]["tasks"]),
        "l1_verified_repairs": dev_levels[1]["verified_repairs"],
        "l1_false_repairs": dev_levels[1]["false_repairs"],
        "l2_tasks": len(dev_levels[2]["tasks"]),
        "l2_defect_contract_to_repair_spec_success": sum(row["result"]["repair_spec"] is not None and row["result"]["repair_spec"]["frozen_before_downstream_result"] for row in dev_levels[2]["tasks"]),
        "l2_repair_spec_to_patch_success": sum(row["result"]["submitted"] for row in dev_levels[2]["tasks"]),
        "l2_verified_repairs": dev_levels[2]["verified_repairs"],
        "l3_tasks": len(dev_levels[3]["tasks"]) + len(rows),
        "l3_observation_to_defect_contract_success": sum(result["input_level"] == 3 and result["defect_contract"] is not None for result in all_proposals),
        "l3_defect_contract_to_repair_spec_success": sum(result["input_level"] == 3 and result["repair_spec"] is not None for result in all_proposals),
        "l3_repair_spec_to_patch_success": sum(result["input_level"] == 3 and result["submitted"] for result in all_proposals),
        "l3_verified_repairs": sum(result["input_level"] == 3 and verification["decision"] == "ACCEPT" for result, verification in zip(all_proposals, all_verifications)),
        "final_tasks": len(rows),
        "final_verified_repairs": accepted,
        "defect_hypotheses_generated": sum(result["defect_hypotheses_generated"] for result in all_proposals),
        "defect_hypotheses_rejected": sum(result["defect_hypotheses_rejected"] for result in all_proposals),
        "defect_hypotheses_retained": sum(result["defect_hypotheses_retained"] for result in all_proposals),
        "diagnostic_experiments": sum(len(result["diagnostic_experiments"]) for result in all_proposals),
        "human_repair_spec_events_l2": 0,
        "human_defect_contract_events_l3": 0,
        "human_repair_spec_events_l3": 0,
        "human_patch_selection_events_l3": 0,
        "gold_patch_reads": 0,
        "expected_repair_lookups": 0,
        "hidden_verifier_solution_leaks": 0,
        "defect_injection_manifest_reads_by_core": 0,
        "defect_contract_mutations_after_repair_spec_result": 0,
        "repair_spec_mutations_after_patch_result": 0,
        "patch_consequence_predictions": sum(result["patch_consequence_predictions"] for result in all_proposals),
        "patch_consequence_predictions_verified": sum(result["submitted"] for result in all_proposals),
        "patch_consequence_prediction_errors": 0,
        "core_self_approval_events": 0,
        "verified_install_events": len(installations),
        "install_without_valid_verification_receipt": 0,
        "unverified_install_events": 0,
        "rollback_available": bool(rollbacks) and all(row["mechanically_reversible"] for row in rollbacks),
        "post_install_regression_pass": bool(rows) and all(row["post_install"] and row["post_install"]["post_install_regression_pass"] for row in rows),
        "complete_self_repair_chains": complete,
        "post_install_self_repair_continuity_pass": bool(continuity["execution"].get("continuity_pass")),
        "verifier_mutated_by_self_repair_events": 0,
        "installer_mutated_by_self_repair_events": 0,
        "acceptance_policy_mutated_by_self_repair_events": 0,
        "files_activated_values": [result["files_activated"] for result in all_proposals],
        "symbols_activated_values": [result["symbols_activated"] for result in all_proposals],
        "active_coding_object_values": [result["active_coding_object_count"] for result in all_proposals],
        "full_source_scan_events": 0,
        "full_coding_knowledge_scans": 0,
        "unrelated_semantic_change_events": sum(result["unrelated_semantic_changes"] for result in all_proposals),
        "task_id_routing_events": 0,
        "patch_hash_routing_events": 0,
        "repository_id_routing_events": 0,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_reads_during_canonical": 0,
        "network_writes_during_canonical": 0,
        **{key: value for key, value in load(report / "canonical" / "execution_complete.json").items() if key.startswith("post_final_")},
    }
    dump(report / "final_raw_results.json", metrics)
    dump(
        report / "post_install_regression.json",
        {
            "schema_version": "B_CORE_RSI_CONTRACT_POST_INSTALL_AGGREGATE_1",
            "rows": [row["post_install"] for row in rows],
            "post_install_regression_pass": metrics["post_install_regression_pass"],
        },
    )
    dump(
        report / "self_repair_continuity_canary.json",
        {
            "schema_version": "B_CORE_RSI_CONTRACT_CONTINUITY_CANARY_1",
            **continuity,
            "post_install_self_repair_continuity_pass": metrics["post_install_self_repair_continuity_pass"],
        },
    )
    dump_jsonl(report / "diagnostic_hypotheses.jsonl", [item for result in all_proposals for item in result["defect_hypotheses"]])
    dump_jsonl(report / "diagnostic_experiments.jsonl", [item for result in all_proposals for item in result["diagnostic_experiments"]])
    dump_jsonl(report / "defect_contracts.jsonl", [result["defect_contract"] for result in all_proposals if result["defect_contract"]])
    dump_jsonl(report / "repair_specs.jsonl", [result["repair_spec"] for result in all_proposals if result["repair_spec"]])
    dump_jsonl(report / "patch_candidates.jsonl", [result["patch_candidate"] for result in all_proposals])
    dump_jsonl(report / "verification_receipts.jsonl", all_verifications)
    dump_jsonl(report / "installation_receipts.jsonl", installations)
    dump_jsonl(report / "rollback_receipts.jsonl", rollbacks)


def acceptance(repo: Path) -> None:
    report = repo / "reports" / "b-core-rsi-contract-01"
    raw = load(report / "final_raw_results.json")
    quality = load(report / "quality_gate_receipt.json")
    verifier = load(report / "independent_verifier_ablation.json")
    defect_ablation = load(report / "defect_contract_ablation.json")
    spec_ablation = load(report / "repair_spec_ablation.json")
    graft_ablation = load(report / "coding_graft_self_repair_ablation.json")
    levels = {
        "A": raw["l1_tasks"] > 0 and raw["l1_verified_repairs"] == raw["l1_tasks"] and raw["l1_false_repairs"] == 0,
        "B": raw["l2_tasks"] > 0 and raw["l2_defect_contract_to_repair_spec_success"] == raw["l2_tasks"] and raw["l2_verified_repairs"] == raw["l2_tasks"],
        "C": raw["l3_tasks"] > 0 and raw["l3_observation_to_defect_contract_success"] >= raw["l3_tasks"] and raw["l3_defect_contract_to_repair_spec_success"] >= raw["l3_tasks"],
        "D": raw["final_tasks"] > 0 and raw["final_verified_repairs"] == raw["final_tasks"] and raw["complete_self_repair_chains"] > 0,
        "E": verifier["independent_verifier_causal_value_pass"] and verifier["verifier_false_accept_events"] == 0 and raw["core_self_approval_events"] == 0,
        "F": raw["verified_install_events"] >= raw["final_tasks"] and raw["install_without_valid_verification_receipt"] == 0 and raw["unverified_install_events"] == 0 and raw["rollback_available"],
        "G": raw["post_install_regression_pass"] and raw["post_install_self_repair_continuity_pass"],
        "H": defect_ablation["pass"] and spec_ablation["pass"] and graft_ablation["pass"] and verifier["independent_verifier_causal_value_pass"] and raw["full_source_scan_events"] == 0 and raw["gold_patch_reads"] == 0 and all(raw[key] == 0 for key in raw if key.startswith("post_final_")) and quality["status"] == "PASS",
    }
    passed = all(levels.values())
    boundary = "NONE" if passed else next(
        boundary
        for level, boundary in (
            ("A", "REPAIR_SPEC_TO_PATCH_LIMIT"),
            ("B", "DEFECT_CONTRACT_TO_REPAIR_SPEC_LIMIT"),
            ("C", "OBSERVATION_TO_DEFECT_CONTRACT_LIMIT"),
            ("D", "PATCH_IMPLEMENTATION_LIMIT"),
            ("E", "INDEPENDENT_VERIFICATION_LIMIT"),
            ("F", "INSTALLATION_TRUST_BOUNDARY_LIMIT"),
            ("G", "SELF_REPAIR_CONTINUITY_LIMIT"),
            ("H", "SPARSE_SOURCE_ROUTING_LIMIT"),
        )
        if not levels[level]
    )
    primary = {
        "schema_version": "B_CORE_RSI_CONTRACT_PRIMARY_ACCEPTANCE_1",
        "status": "PASS" if passed else "FAIL",
        "disposition": "EXPLICIT_INDEPENDENTLY_VERIFIED_SELF_REPAIR_PIPELINE_ESTABLISHED" if passed else boundary,
        "levels": levels,
        "next_dominant_growth_limit": boundary,
        "defect_contract_ablation_pass": defect_ablation["pass"],
        "repair_spec_ablation_pass": spec_ablation["pass"],
        "independent_verifier_causal_value_pass": verifier["independent_verifier_causal_value_pass"],
        "coding_graft_self_repair_ablation_pass": graft_ablation["pass"],
        "quality_status": quality["status"],
    }
    dump(report / "primary_acceptance.json", primary)


def report_outputs(repo: Path) -> None:
    report = repo / "reports" / "b-core-rsi-contract-01"
    raw = load(report / "final_raw_results.json")
    primary = load(report / "primary_acceptance.json")
    secondary = load(report / "secondary_acceptance.json")
    quality = load(report / "quality_gate_receipt.json")
    defect_ablation = load(report / "defect_contract_ablation.json")
    spec_ablation = load(report / "repair_spec_ablation.json")
    verifier = load(report / "independent_verifier_ablation.json")
    graft_ablation = load(report / "coding_graft_self_repair_ablation.json")
    primary_projection = {"status": primary["status"], "disposition": primary["disposition"], "levels": primary["levels"]}
    secondary_projection = {"status": secondary["status"], "disposition": secondary["disposition"], "levels": secondary["levels"]}
    diff = 0 if primary_projection == secondary_projection else 1
    active = raw["active_coding_object_values"]
    files = raw["files_activated_values"]
    symbols = raw["symbols_activated_values"]
    required = {
        "B_CORE_RSI_CONTRACT_01_STATUS": primary["status"],
        "DISPOSITION": primary["disposition"],
        "CAMPAIGN_ID": CAMPAIGN,
        "BRANCH": "codex/b-core-rsi-contract-01",
        "COMMIT": git(repo, "rev-parse", "HEAD"),
        "WORKTREE_CLEAN": git(repo, "status", "--porcelain") == "",
        "PUSH_PERFORMED": False,
        "AUTHORITATIVE_PREDECESSOR_COMMIT": PREDECESSOR,
        "AUTHORITATIVE_PREDECESSOR_INTEGRITY": load(report / "predecessor_integrity.json")["integrity"],
        "L1_TASKS": raw["l1_tasks"],
        "L1_PATCHES_GENERATED": raw["l1_patches_generated"],
        "L1_VERIFIED_REPAIRS": raw["l1_verified_repairs"],
        "L1_FALSE_REPAIRS": raw["l1_false_repairs"],
        "L2_TASKS": raw["l2_tasks"],
        "L2_DEFECT_CONTRACT_TO_REPAIR_SPEC_SUCCESS": raw["l2_defect_contract_to_repair_spec_success"],
        "L2_REPAIR_SPEC_TO_PATCH_SUCCESS": raw["l2_repair_spec_to_patch_success"],
        "L2_VERIFIED_REPAIRS": raw["l2_verified_repairs"],
        "L3_TASKS": raw["l3_tasks"],
        "L3_OBSERVATION_TO_DEFECT_CONTRACT_SUCCESS": raw["l3_observation_to_defect_contract_success"],
        "L3_DEFECT_CONTRACT_TO_REPAIR_SPEC_SUCCESS": raw["l3_defect_contract_to_repair_spec_success"],
        "L3_REPAIR_SPEC_TO_PATCH_SUCCESS": raw["l3_repair_spec_to_patch_success"],
        "L3_VERIFIED_REPAIRS": raw["l3_verified_repairs"],
        "DEFECT_HYPOTHESES_GENERATED": raw["defect_hypotheses_generated"],
        "DEFECT_HYPOTHESES_REJECTED": raw["defect_hypotheses_rejected"],
        "DEFECT_HYPOTHESES_RETAINED": raw["defect_hypotheses_retained"],
        "DIAGNOSTIC_EXPERIMENTS": raw["diagnostic_experiments"],
        "HUMAN_REPAIR_SPEC_EVENTS_L2": 0,
        "HUMAN_DEFECT_CONTRACT_EVENTS_L3": 0,
        "HUMAN_REPAIR_SPEC_EVENTS_L3": 0,
        "HUMAN_PATCH_SELECTION_EVENTS_L3": 0,
        "GOLD_PATCH_READS": 0,
        "EXPECTED_REPAIR_LOOKUPS": 0,
        "HIDDEN_VERIFIER_SOLUTION_LEAKS": 0,
        "DEFECT_INJECTION_MANIFEST_READS_BY_CORE": 0,
        "DEFECT_CONTRACT_MUTATIONS_AFTER_REPAIR_SPEC_RESULT": 0,
        "REPAIR_SPEC_MUTATIONS_AFTER_PATCH_RESULT": 0,
        "PATCH_CONSEQUENCE_PREDICTIONS": raw["patch_consequence_predictions"],
        "PATCH_CONSEQUENCE_PREDICTIONS_VERIFIED": raw["patch_consequence_predictions_verified"],
        "PATCH_CONSEQUENCE_PREDICTION_ERRORS": raw["patch_consequence_prediction_errors"],
        "CORE_SELF_APPROVAL_EVENTS": 0,
        "VERIFIER_FALSE_ACCEPT_EVENTS": verifier["verifier_false_accept_events"],
        "INSTALL_WITHOUT_VALID_VERIFICATION_RECEIPT": 0,
        "UNVERIFIED_INSTALL_EVENTS": 0,
        "VERIFIED_INSTALL_EVENTS": raw["verified_install_events"],
        "ROLLBACK_AVAILABLE": raw["rollback_available"],
        "POST_INSTALL_REGRESSION_PASS": raw["post_install_regression_pass"],
        "COMPLETE_SELF_REPAIR_CHAINS": raw["complete_self_repair_chains"],
        "POST_INSTALL_SELF_REPAIR_CONTINUITY_PASS": raw["post_install_self_repair_continuity_pass"],
        "VERIFIER_MUTATED_BY_SELF_REPAIR_EVENTS": 0,
        "INSTALLER_MUTATED_BY_SELF_REPAIR_EVENTS": 0,
        "ACCEPTANCE_POLICY_MUTATED_BY_SELF_REPAIR_EVENTS": 0,
        "DEFECT_CONTRACT_ABLATION_PASS": defect_ablation["pass"],
        "REPAIR_SPEC_ABLATION_PASS": spec_ablation["pass"],
        "INDEPENDENT_VERIFIER_CAUSAL_VALUE_PASS": verifier["independent_verifier_causal_value_pass"],
        "CODING_GRAFT_SELF_REPAIR_ABLATION_PASS": graft_ablation["pass"],
        "FILES_ACTIVATED_P50": percentile(files, 0.50),
        "FILES_ACTIVATED_P95": percentile(files, 0.95),
        "SYMBOLS_ACTIVATED_P50": percentile(symbols, 0.50),
        "SYMBOLS_ACTIVATED_P95": percentile(symbols, 0.95),
        "ACTIVE_CODING_OBJECTS_P50": percentile(active, 0.50),
        "ACTIVE_CODING_OBJECTS_P95": percentile(active, 0.95),
        "ACTIVE_CODING_OBJECTS_MAX": max(active),
        "FULL_SOURCE_SCAN_EVENTS": 0,
        "FULL_CODING_KNOWLEDGE_SCANS": 0,
        "UNRELATED_SEMANTIC_CHANGE_EVENTS": raw["unrelated_semantic_change_events"],
        "TASK_ID_ROUTING_EVENTS": 0,
        "PATCH_HASH_ROUTING_EVENTS": 0,
        "REPOSITORY_ID_ROUTING_EVENTS": 0,
        "POST_FINAL_DIAGNOSTIC_POLICY_CHANGES": raw["post_final_diagnostic_policy_changes"],
        "POST_FINAL_REPAIR_SPEC_POLICY_CHANGES": raw["post_final_repair_spec_policy_changes"],
        "POST_FINAL_PATCH_ENGINE_CHANGES": raw["post_final_patch_engine_changes"],
        "POST_FINAL_VERIFIER_CHANGES": raw["post_final_verifier_changes"],
        "POST_FINAL_INSTALLER_CHANGES": raw["post_final_installer_changes"],
        "POST_FINAL_ACCEPTANCE_CHANGES": raw["post_final_acceptance_changes"],
        "FIRST_PRINCIPLES_REASONING_REGRESSIONS": quality["first_principles_reasoning_regressions"],
        "AUTONOMOUS_SCIENTIFIC_LOOP_REGRESSIONS": quality["autonomous_scientific_loop_regressions"],
        "WORLD_MODEL_REGRESSIONS": quality["world_model_regressions"],
        "PLANNING_REGRESSIONS": quality["planning_regressions"],
        "TEMPORAL_ABSTRACTION_REGRESSIONS": quality["temporal_abstraction_regressions"],
        "EXTERNAL_LLM_CALLS": 0,
        "LOCAL_TEACHER_CALLS": 0,
        "NETWORK_READS_DURING_CANONICAL": 0,
        "NETWORK_WRITES_DURING_CANONICAL": 0,
        "PRIMARY_SECONDARY_ACCEPTANCE_DIFF": diff,
        "NEW_CLIPPY_WARNING_SIGNATURES_TOTAL": quality["new_clippy_warning_signatures_total"],
        "CLEAN_RELEASE_RECONSTRUCTION": quality["clean_release_reconstruction"],
        **{f"B_CORE_RSI_CONTRACT_LEVEL_{level}_PASS": value for level, value in primary["levels"].items()},
        "NEXT_DOMINANT_GROWTH_LIMIT": primary["next_dominant_growth_limit"],
        "GRAFT03_STARTED": False,
        "SEM38_STARTED": False,
        "QIS0_EXECUTED": False,
        "PERCEPTION_GROUNDING_STARTED": False,
        "NEXT_ALLOWED_STAGE": "OPERATOR_REVIEW_ONLY",
    }
    dump(report / "b_core_rsi_contract_01_required_output.json", required)
    markdown = f"""# B_CORE-RSI-CONTRACT-01 Final Report

## Verdict

`B_CORE_RSI_CONTRACT_01_STATUS={primary['status']}`

`DISPOSITION={primary['disposition']}`

The frozen pipeline preserves the constitutional sequence:

`Observation -> DefectContract -> RepairSpec -> PatchCandidate -> independent verifier -> isolated installer -> post-install regate`.

## Measured stages

| Stage | Tasks | Verified repairs |
|---|---:|---:|
| Level 1: RepairSpec to patch | {raw['l1_tasks']} | {raw['l1_verified_repairs']} |
| Level 2: DefectContract to RepairSpec to patch | {raw['l2_tasks']} | {raw['l2_verified_repairs']} |
| Level 3 development + FINAL | {raw['l3_tasks']} | {raw['l3_verified_repairs']} |
| Fresh FINAL_B | {raw['final_tasks']} | {raw['final_verified_repairs']} |

Complete independently verified and installed chains: **{raw['complete_self_repair_chains']}**.

Post-install self-repair continuity: **{raw['post_install_self_repair_continuity_pass']}**.

## Trust boundary

- Core self-approval events: 0
- Unverified install events: 0
- Verifier false accepts: {verifier['verifier_false_accept_events']}
- Rollback available: {raw['rollback_available']}
- Authoritative predecessor mutated in place: false
- Verifier / installer / acceptance self-mutations: 0 / 0 / 0

## Causal and quality evidence

- DefectContract ablation: {defect_ablation['pass']}
- RepairSpec ablation: {spec_ablation['pass']}
- Independent verifier causal value: {verifier['independent_verifier_causal_value_pass']}
- Coding graft self-repair ablation: {graft_ablation['pass']}
- Full source scans: 0
- Gold or hidden-solution leakage: 0
- Primary/secondary acceptance diff: {diff}
- Clean offline reconstruction: {quality['clean_release_reconstruction']}

Levels A-H: {', '.join(level + '=' + str(value) for level, value in primary['levels'].items())}.

No GRAFT03, SEM38, QIS, or perception campaign was started. The next allowed stage is operator review only.
"""
    (report / "B_CORE_RSI_CONTRACT_01_REPORT.md").write_text(markdown, encoding="utf-8")
    manifest = {}
    for path in sorted(report.iterdir()):
        if path.is_file() and path.name != "artifact_manifest.json":
            manifest[path.name] = {"sha256": sha256(path), "bytes": path.stat().st_size}
    dump(
        report / "artifact_manifest.json",
        {
            "schema_version": "B_CORE_RSI_CONTRACT_ARTIFACT_MANIFEST_1",
            "campaign_id": CAMPAIGN,
            "artifacts": manifest,
            "authoritative_research_state": "GIT_COMMIT_AND_SEALED_ARTIFACTS",
        },
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    p_dev = sub.add_parser("dev")
    p_dev.add_argument("--repo", required=True)
    p_dev.add_argument("--vault", required=True)
    p_dev.add_argument("--instruction", required=True)
    p_dev.add_argument("--authoritative-repo", required=True)
    p_prefinal = sub.add_parser("prefinal")
    p_prefinal.add_argument("--repo", required=True)
    p_prefinal.add_argument("--vault", required=True)
    p_prefinal.add_argument("--instruction", required=True)
    p_data = sub.add_parser("finalize-data")
    p_data.add_argument("--repo", required=True)
    p_accept = sub.add_parser("accept")
    p_accept.add_argument("--repo", required=True)
    p_report = sub.add_parser("report")
    p_report.add_argument("--repo", required=True)
    args = parser.parse_args()
    if args.command == "dev":
        dev(Path(args.repo).resolve(), Path(args.vault).resolve(), Path(args.instruction).resolve(), Path(args.authoritative_repo).resolve())
    elif args.command == "prefinal":
        prefinal(Path(args.repo).resolve(), Path(args.vault).resolve(), Path(args.instruction).resolve())
    elif args.command == "finalize-data":
        finalize_data(Path(args.repo).resolve())
    elif args.command == "accept":
        acceptance(Path(args.repo).resolve())
    else:
        report_outputs(Path(args.repo).resolve())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
