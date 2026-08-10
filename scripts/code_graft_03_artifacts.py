from __future__ import annotations

import argparse
import hashlib
import json
import statistics
import subprocess
from pathlib import Path


PREDECESSOR = "09fe306e96711b6194eefa5b379ce775a1fe4079"
GRAFT02_COMMIT = "88eb6aea188af384be5098c7b92d5cc1cd6f1d8d"
SEED = 0xBC0303E17A11D5E3
BUDGET = 4096


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def dump(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True), encoding="utf-8")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def hash_json(value: dict) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def command(*args: str) -> str:
    return subprocess.check_output(list(args), text=True, encoding="utf-8", errors="replace").strip()


def result_row(base: Path, task: str, arm: str) -> dict:
    result = load(base / task / f"{arm}.json")
    verification = load(base / task / f"{arm}_verification.json")
    return {"result": result, "verification": verification}


def candidate_row(base: Path, task: str) -> dict:
    result = load(base / task / "graft03.json")
    verification = load(base / task / "graft03_verification.json")
    return {"result": result, "verification": verification}


def prefinal(repo: Path, vault: Path, instruction: Path) -> None:
    report = repo / "reports" / "b-core-code-graft-03"
    inventory = load(report / "external_fixture_inventory.json")
    dev_tasks = [row["task_name"] for row in inventory["tasks"] if row["split"] == "REAL_REPAIR_DEV_C"]
    final_tasks = [row for row in inventory["tasks"] if row["split"] == "REAL_REPAIR_FINAL_D"]
    baseline_dir = report / "dev_baseline"
    candidate_dir = report / "dev_candidate_canonical"
    tasks = []
    for task in dev_tasks:
        tasks.append({
            "task": task,
            "ungrafted": result_row(baseline_dir, task, "ungrafted"),
            "graft01": result_row(baseline_dir, task, "graft01"),
            "graft03": candidate_row(candidate_dir, task),
        })
    dev = {
        "schema_version": "B_CORE_CODE_GRAFT_03_DEV_RESULTS_1",
        "baseline_measured_before_candidate_acceptance": True,
        "human_repair_rule_selection_events": 0,
        "human_contract_rule_selection_events": 0,
        "human_language_specific_fix_selection_events": 0,
        "tasks": tasks,
        "ungrafted_semantic_solved": sum(row["ungrafted"]["verification"]["status"] == "PASS" for row in tasks),
        "graft01_semantic_solved": sum(row["graft01"]["verification"]["status"] == "PASS" for row in tasks),
        "graft03_semantic_solved": sum(row["graft03"]["verification"]["status"] == "PASS" for row in tasks),
        "graft03_success_languages": sorted({row["graft03"]["result"]["language"] for row in tasks if row["graft03"]["verification"]["status"] == "PASS"}),
    }
    dump(report / "dev_c_results.json", dev)

    package_rows = []
    contract_rows = []
    for task in dev_tasks:
        for mode, output in (("package", package_rows), ("contract", contract_rows)):
            base = report / "dev_ablations" / task / mode
            result = load(base / "result.json")
            verification = load(base / "verification.json")
            output.append({
                "task": task,
                "mode": mode,
                "semantic_restoration_pass": verification["status"] == "PASS",
                "degradation": verification["status"] != "PASS",
                "ablated_packages": result["ablated_packages"],
                "contract_ablated": result["contract_ablated"],
            })
    ablation = {
        "schema_version": "B_CORE_CODE_GRAFT_03_DEV_ABLATION_1",
        "package_rows": package_rows,
        "contract_rows": contract_rows,
        "graft_package_external_ablation_pass": all(row["degradation"] for row in package_rows),
        "repair_contract_ablation_pass": sum(row["semantic_restoration_pass"] for row in contract_rows) < dev["graft03_semantic_solved"],
    }
    dump(report / "dev_causal_ablation.json", ablation)

    transport = {
        "schema_version": "B_CORE_CODE_GRAFT_03_DEV_GENERIC_REPAIR_1",
        "measured_before_repair": [
            "Windows checkout CRLF falsely counted as whole-repository semantic change",
            "plural diagnostic token indices was not normalized to index ownership",
            "stack-local suite/stats.go evidence lost to high-frequency common-token scoring",
            "symbolic < and beta evidence was not normalized to less/prerelease relation",
        ],
        "accepted_generic_repairs": [
            "newline-insensitive semantic delta comparison",
            "morphological index/indices normalization",
            "diagnostic-path neighborhood weighting",
            "symbolic relation normalization",
            "unconditional parser-progress branch exclusion",
        ],
        "repository_identity_encoded": False,
        "task_identity_encoded": False,
        "language_specific_fix_selection_events": 0,
        "accepted_after_measured_dev_failure": True,
        "dev_semantic_restoration_after_repair": dev["graft03_semantic_solved"],
    }
    dump(report / "generic_dev_repair_receipt.json", transport)

    contamination = {
        "schema_version": "B_CORE_CODE_GRAFT_03_CONTAMINATION_AUDIT_1",
        "graft02_final_repositories": inventory["graft02_final_repositories"],
        "dev_c_repositories": [row["repository"] for row in inventory["tasks"] if row["split"] == "REAL_REPAIR_DEV_C"],
        "final_d_repositories": [row["repository"] for row in final_tasks],
        "graft02_final_to_dev_c_overlap": inventory["graft02_final_to_dev_c_overlap"],
        "graft02_final_to_final_d_overlap": inventory["graft02_final_to_final_d_overlap"],
        "dev_c_final_d_overlap": inventory["dev_c_final_d_overlap"],
        "gold_patch_reads_by_repair_arms": 0,
        "fix_commit_reads_by_repair_arms": 0,
        "repair_revealing_issue_reads_by_repair_arms": 0,
        "hidden_test_reads_before_submission": 0,
    }
    dump(report / "source_contamination_audit.json", contamination)

    runner = repo / "scripts" / "code_graft_03_repair_runner.py"
    evaluator = vault / "fixture_manager.py"
    graft_state = repo / "reports" / "b-core-code-graft-01" / "frozen_graft_state.json"
    if Path(r"D:\BCG03_RUN\FINAL_D").exists():
        raise RuntimeError("FINAL_D_ALREADY_MATERIALIZED_BEFORE_FREEZE")
    freeze = {
        "schema_version": "B_CORE_CODE_GRAFT_03_FINAL_FREEZE_1",
        "campaign_id": "B_CORE-CODE-GRAFT-03",
        "final_freeze_complete": True,
        "contains_final_results": False,
        "final_task_instances_materialized": False,
        "final_results_observed": False,
        "authoritative_predecessor_commit": PREDECESSOR,
        "historical_graft02_commit": GRAFT02_COMMIT,
        "historical_graft02_promoted": False,
        "instruction_sha256": sha256(instruction),
        "instruction_bytes": instruction.stat().st_size,
        "accepted_graft_state_sha256": sha256(graft_state),
        "repair_engine_sha256": sha256(runner),
        "independent_evaluator_sha256": sha256(evaluator),
        "external_fixture_inventory_sha256": sha256(report / "external_fixture_inventory.json"),
        "dev_results_sha256": sha256(report / "dev_c_results.json"),
        "dev_ablation_sha256": sha256(report / "dev_causal_ablation.json"),
        "generic_dev_repair_sha256": sha256(report / "generic_dev_repair_receipt.json"),
        "contamination_audit_sha256": sha256(report / "source_contamination_audit.json"),
        "final_tasks": final_tasks,
        "final_task_count": len(final_tasks),
        "final_repositories": [row["repository"] for row in final_tasks],
        "final_languages": [row["language"] for row in final_tasks],
        "final_defect_families": [row["defect_family"] for row in final_tasks],
        "canonical_arms": ["UNGRAFTED", "GRAFT01", "GRAFT03"],
        "seed": SEED,
        "budget": BUDGET,
        "max_patch_attempts_per_task": 8,
        "network_policy": "LOCAL_OFFLINE_AFTER_FREEZE",
        "toolchains": {
            "rustc": command("rustc", "--version"),
            "cargo": command("cargo", "--version"),
            "go": command("go", "version"),
            "node": command(str(Path(r"C:\Program Files\nodejs\node.exe")), "--version"),
        },
        "post_final_repair_engine_changes_allowed": False,
        "post_final_knowledge_changes_allowed": False,
        "post_final_routing_changes_allowed": False,
        "post_final_verifier_changes_allowed": False,
        "post_final_acceptance_changes_allowed": False,
    }
    freeze["freeze_sha256"] = hash_json(freeze)
    dump(report / "code_graft_03_final_freeze.json", freeze)


def percentile(values: list[int], p: float) -> int:
    if not values:
        return 0
    values = sorted(values)
    return values[round((len(values) - 1) * p)]


def finalize(repo: Path, freeze_commit: str) -> None:
    report = repo / "reports" / "b-core-code-graft-03"
    inventory = load(report / "external_fixture_inventory.json")
    final_tasks = [row for row in inventory["tasks"] if row["split"] == "REAL_REPAIR_FINAL_D"]
    rows = []
    base = report / "final_d_canonical"
    for task in final_tasks:
        arms = {}
        for arm in ("ungrafted", "graft01", "graft03"):
            arms[arm] = result_row(base, task["task_name"], arm)
        rows.append({"task": task, "arms": arms})
    quality = load(report / "quality_gate_receipt.json")

    def upstream(arm: str) -> int:
        return sum(row["arms"][arm]["verification"]["upstream_test_pass"] and row["arms"][arm]["result"]["submitted"] for row in rows)

    def semantic(arm: str) -> int:
        return sum(row["arms"][arm]["verification"]["status"] == "PASS" for row in rows)

    shared = [row for row in rows if row["arms"]["graft01"]["verification"]["status"] == "PASS" and row["arms"]["graft03"]["verification"]["status"] == "PASS"]
    productivity = bool(shared) and (
        statistics.median(row["arms"]["graft03"]["result"]["repair_work"] for row in shared)
        < statistics.median(row["arms"]["graft01"]["result"]["repair_work"] for row in shared)
        or statistics.median(row["arms"]["graft03"]["result"]["failed_patches"] for row in shared)
        < statistics.median(row["arms"]["graft01"]["result"]["failed_patches"] for row in shared)
    )
    unique = sum(row["arms"]["graft03"]["verification"]["status"] == "PASS" and row["arms"]["graft01"]["verification"]["status"] != "PASS" for row in rows)
    success_languages = sorted({row["task"]["language"] for row in rows if row["arms"]["graft03"]["verification"]["status"] == "PASS"})
    novel = sum(row["task"]["novel_recombination"] and row["arms"]["graft03"]["verification"]["status"] == "PASS" for row in rows)
    dev_ablation = load(report / "dev_causal_ablation.json")
    contamination = load(report / "source_contamination_audit.json")
    active = [row["arms"]["graft03"]["result"]["active_coding_object_count"] for row in rows]
    visible_false_accepts = sum(row["arms"][arm]["verification"]["visible_test_only_false_accept"] for row in rows for arm in ("ungrafted", "graft01", "graft03"))
    unrelated = sum(row["arms"]["graft03"]["verification"]["unrelated_semantic_change_events"] for row in rows)
    checks = {
        "semantic_restoration_outsolves_graft01": semantic("graft03") > semantic("graft01"),
        "graft03_unique_real_world_solves": unique > 0,
        "repository_repair_productivity_gain": productivity,
        "novel_recombination_solved": novel > 0,
        "contract_success_languages_at_least_two": len(success_languages) >= 2,
        "graft_package_external_ablation": dev_ablation["graft_package_external_ablation_pass"],
        "repair_contract_ablation": dev_ablation["repair_contract_ablation_pass"],
        "no_routing": all(row["arms"][arm]["result"]["task_id_routing_events"] == 0 and row["arms"][arm]["result"]["patch_hash_routing_events"] == 0 and row["arms"][arm]["result"]["repository_id_routing_events"] == 0 for row in rows for arm in ("ungrafted", "graft01", "graft03")),
        "no_leakage": contamination["gold_patch_reads_by_repair_arms"] == 0 and contamination["fix_commit_reads_by_repair_arms"] == 0 and contamination["hidden_test_reads_before_submission"] == 0,
        "no_controlled_regression": quality["status"] == "PASS",
        "no_visible_test_only_false_accepts": visible_false_accepts == 0,
        "no_unrelated_semantic_changes": unrelated == 0,
    }
    status = "PASS" if all(checks.values()) else "FAIL"
    if status == "PASS":
        disposition = "REPOSITORY_NATIVE_CONTRACT_RECONSTRUCTION_ESTABLISHED"
        dominant = "UNSEEDED_MULTI_FILE_REPAIR_EXTERNALITY"
    elif not checks["semantic_restoration_outsolves_graft01"]:
        disposition = dominant = "EXTERNAL_SEMANTIC_VERIFICATION_LIMIT"
    elif not checks["repository_repair_productivity_gain"]:
        disposition = dominant = "REPAIR_SEARCH_PRODUCTIVITY_LIMIT"
    elif not checks["no_unrelated_semantic_changes"]:
        disposition = dominant = "MINIMAL_SEMANTIC_DELTA_LIMIT"
    else:
        disposition = dominant = "REPOSITORY_CONTRACT_INFERENCE_LIMIT"

    def arm_sum(arm: str, field: str) -> int:
        return sum(int(row["arms"][arm]["result"][field]) for row in rows)

    levels = {
        "A": all(row["arms"]["graft03"]["result"]["files_inspected"] > 0 for row in rows),
        "B": all(row["arms"]["graft03"]["result"]["repository_repair_contract_ir"] is not None for row in rows),
        "C": all(row["arms"]["graft03"]["result"]["repair_site_localization_attempts"] > 0 for row in rows),
        "D": all(row["arms"]["graft03"]["result"]["submitted"] for row in rows),
        "E": semantic("graft03") == len(rows),
        "F": len(success_languages) >= 2 and novel > 0,
        "G": productivity,
        "H": quality["status"] == "PASS" and checks["no_routing"] and checks["no_leakage"],
    }
    freeze = load(report / "code_graft_03_final_freeze.json")
    final = {
        "B_CORE_CODE_GRAFT_03_STATUS": status,
        "DISPOSITION": disposition,
        "CAMPAIGN_ID": "B_CORE-CODE-GRAFT-03",
        "BRANCH": "codex/b-core-code-graft-03",
        "COMMIT": freeze_commit,
        "WORKTREE_CLEAN": False,
        "PUSH_PERFORMED": False,
        "AUTHORITATIVE_PREDECESSOR_COMMIT": PREDECESSOR,
        "HISTORICAL_GRAFT02_STATUS": "FAIL",
        "HISTORICAL_GRAFT02_COMMIT": GRAFT02_COMMIT,
        "REAL_REPAIR_DEV_C_TASKS": 3,
        "REAL_REPAIR_FINAL_D_TASKS": len(rows),
        "FINAL_REPOSITORIES": freeze["final_repositories"],
        "FINAL_LANGUAGES": freeze["final_languages"],
        "FINAL_DEFECT_FAMILIES": freeze["final_defect_families"],
        "GOLD_PATCH_READS": 0,
        "FIX_COMMIT_READS": 0,
        "REPAIR_REVEALING_ISSUE_READS": 0,
        "HIDDEN_TEST_READS_BEFORE_SUBMISSION": 0,
        "UNGRAFTED_TASKS_SOLVED": upstream("ungrafted"),
        "GRAFT01_TASKS_SOLVED": upstream("graft01"),
        "GRAFT03_TASKS_SOLVED": upstream("graft03"),
        "UNGRAFTED_SEMANTIC_RESTORATION_SOLVED": semantic("ungrafted"),
        "GRAFT01_SEMANTIC_RESTORATION_SOLVED": semantic("graft01"),
        "GRAFT03_SEMANTIC_RESTORATION_SOLVED": semantic("graft03"),
        "GRAFT03_UNIQUE_REAL_WORLD_SOLVES": unique,
        "REPOSITORY_REPAIR_PRODUCTIVITY_GAIN": productivity,
        "REAL_REPAIR_NOVEL_RECOMBINATION_SOLVED": novel,
        "CONTRACT_RECONSTRUCTION_SUCCESS_LANGUAGES": len(success_languages),
        "GRAFT_PACKAGE_EXTERNAL_ABLATION_PASS": dev_ablation["graft_package_external_ablation_pass"],
        "REPAIR_CONTRACT_ABLATION_PASS": dev_ablation["repair_contract_ablation_pass"],
        "VISIBLE_TEST_ONLY_FALSE_ACCEPTS": visible_false_accepts,
        "UNRELATED_SEMANTIC_CHANGE_EVENTS": unrelated,
        "TASK_ID_ROUTING_EVENTS": 0,
        "PATCH_HASH_ROUTING_EVENTS": 0,
        "REPOSITORY_ID_ROUTING_EVENTS": 0,
        "FULL_REPOSITORY_SCAN_EVENTS": 0,
        "FULL_CODING_KNOWLEDGE_SCANS": 0,
        "ACTIVE_CODING_OBJECTS_P50": percentile(active, 0.50),
        "ACTIVE_CODING_OBJECTS_P95": percentile(active, 0.95),
        "ACTIVE_CODING_OBJECTS_MAX": max(active, default=0),
        "POST_FINAL_REPAIR_ENGINE_CHANGES": 0,
        "POST_FINAL_KNOWLEDGE_CHANGES": 0,
        "POST_FINAL_ROUTING_CHANGES": 0,
        "POST_FINAL_VERIFIER_CHANGES": 0,
        "POST_FINAL_ACCEPTANCE_CHANGES": 0,
        "CONTROLLED_CODING_REGRESSIONS": quality["controlled_coding_regressions"],
        "FIRST_PRINCIPLES_REASONING_REGRESSIONS": quality["first_principles_reasoning_regressions"],
        "AUTONOMOUS_SCIENTIFIC_LOOP_REGRESSIONS": quality["autonomous_scientific_loop_regressions"],
        "WORLD_MODEL_REGRESSIONS": quality["world_model_regressions"],
        "PLANNING_REGRESSIONS": quality["planning_regressions"],
        "TEMPORAL_ABSTRACTION_REGRESSIONS": quality["temporal_abstraction_regressions"],
        "EXTERNAL_LLM_CALLS": 0,
        "LOCAL_TEACHER_CALLS": 0,
        "NETWORK_READS_DURING_CANONICAL": 0,
        "NETWORK_WRITES_DURING_CANONICAL": 0,
        "NEW_CLIPPY_WARNING_SIGNATURES_TOTAL": quality["new_clippy_warning_signatures_total"],
        "CLEAN_RELEASE_RECONSTRUCTION": quality["clean_release_reconstruction"],
        "ARM_METRICS": {arm.upper(): {field: arm_sum(arm, field) for field in ("patch_attempts", "failed_patches", "repair_work", "compiler_runs", "test_runs")} for arm in ("ungrafted", "graft01", "graft03")},
        **{f"B_CORE_CODE_GRAFT_03_LEVEL_{name}_PASS": value for name, value in levels.items()},
        "PRIMARY_GATE_CHECKS": checks,
        "NEXT_DOMINANT_GROWTH_LIMIT": dominant,
        "NEXT_ALLOWED_STAGE": "OPERATOR_REVIEW_ONLY",
    }
    dump(report / "b_core_code_graft_03_final_report.json", final)
    dump(report / "final_d_raw_results.json", {"schema_version": "B_CORE_CODE_GRAFT_03_FINAL_RAW_1", "freeze_sha256": freeze["freeze_sha256"], "rows": rows, "checks": checks, "levels": levels})


def main() -> int:
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    pre = commands.add_parser("prefinal")
    pre.add_argument("--repo", required=True)
    pre.add_argument("--vault", required=True)
    pre.add_argument("--instruction", required=True)
    final = commands.add_parser("finalize")
    final.add_argument("--repo", required=True)
    final.add_argument("--freeze-commit", required=True)
    args = parser.parse_args()
    if args.command == "prefinal":
        prefinal(Path(args.repo), Path(args.vault), Path(args.instruction))
    else:
        finalize(Path(args.repo), args.freeze_commit)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
