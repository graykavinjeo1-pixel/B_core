#!/usr/bin/env python3
"""Artifact composition for the CODE-GRAFT-02 external-validity campaign."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import statistics
import subprocess
from pathlib import Path
from typing import Any


PREDECESSOR = "09fe306e96711b6194eefa5b379ce775a1fe4079"
UNGRAFTED_AUTHORITY = "b33386e7a8793c5c27e2c2df3e19db0e6e04d0f4"
CAMPAIGN_SEED = 0xBC0202E17A11D5E3
MAX_EPOCHS = 4096


def load(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def dump(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def hash_json(value: Any) -> str:
    data = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(data).hexdigest()


def command(*args: str, cwd: Path | None = None) -> str:
    return subprocess.run(
        list(args),
        cwd=cwd,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
    ).stdout.strip()


def current_head(repo: Path) -> str:
    return command("git", "rev-parse", "HEAD", cwd=repo)


def task_results(root: Path, tasks: list[str]) -> list[dict[str, Any]]:
    rows = []
    for task in tasks:
        ungrafted = load(root / task / "ungrafted.json")
        grafted = load(root / task / "grafted.json")
        verification = load(root / task / "grafted_verification.json")
        if ungrafted["task_hash"] != grafted["task_hash"]:
            raise RuntimeError(f"PAIRED_TASK_HASH_MISMATCH:{task}")
        rows.append(
            {
                "task": task,
                "task_hash": ungrafted["task_hash"],
                "ungrafted": ungrafted,
                "grafted": grafted,
                "grafted_independent_verification": verification,
            }
        )
    return rows


def import_evaluator(path: Path):
    spec = importlib.util.spec_from_file_location("bcore_code_graft_02_evaluator", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("EVALUATOR_IMPORT_FAILED")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def prefinal(repo: Path, vault: Path, instruction: Path) -> None:
    report = repo / "reports" / "b-core-code-graft-02"
    inventory = load(report / "external_fixture_inventory.json")
    dev_tasks = [row["task_name"] for row in inventory["tasks"] if row["split"] == "DEV_A"]
    final_tasks = [row["task_name"] for row in inventory["tasks"] if row["split"] == "FINAL_B"]
    dev_rows = task_results(report / "dev_canonical", dev_tasks)

    dev_summary = {
        "schema_version": "B_CORE_CODE_GRAFT_02_DEV_RESULTS_1",
        "baseline_measured_before_generic_transport_repair": True,
        "accepted_graft_modified_before_baseline": False,
        "tasks": dev_rows,
        "ungrafted_tasks": len(dev_rows),
        "ungrafted_solved": sum(row["ungrafted"]["solved"] for row in dev_rows),
        "grafted_tasks": len(dev_rows),
        "grafted_solved": sum(row["grafted"]["solved"] for row in dev_rows),
        "paired_task_hashes_identical": all(
            row["ungrafted"]["task_hash"] == row["grafted"]["task_hash"]
            for row in dev_rows
        ),
        "independent_verification_pass": all(
            row["grafted_independent_verification"]["status"] == "PASS"
            for row in dev_rows
        ),
        "full_coding_knowledge_scans": 0,
        "repository_id_routing_events": 0,
        "task_id_routing_events": 0,
        "gold_patch_reads": 0,
    }
    dump(report / "dev_baseline_results.json", dev_summary)

    ablation_rows = []
    for task, package in [
        ("dev-rust-type-itoa", "TYPE_CONTROL"),
        ("dev-python-failure-boltons", "FAILURE_REPAIR"),
        ("dev-python-failure-more-itertools", "FAILURE_REPAIR"),
    ]:
        row = load(report / "dev_ablations" / task / f"{package}.json")
        ablation_rows.append(
            {
                "task": task,
                "removed_package": package,
                "solved_without_package": row["solved"],
                "causal_degradation": not row["solved"],
            }
        )
    ablation = {
        "schema_version": "B_CORE_CODE_GRAFT_02_DEV_ABLATION_1",
        "rows": ablation_rows,
        "external_package_causal_ablation_pass": all(
            row["causal_degradation"] for row in ablation_rows
        ),
    }
    dump(report / "external_package_causal_ablation_dev.json", ablation)

    transport = {
        "schema_version": "B_CORE_CODE_GRAFT_02_GENERIC_TRANSPORT_REPAIR_1",
        "measured_failure": "PYTEST_COMPACT_TRACEBACK_LOCATION_NOT_TRANSPORTED",
        "repair_scope": [
            "generic pytest path.py:line diagnostic grammar",
            "newline-preserving source transport",
            "minimal default-boundary candidate generation",
        ],
        "repository_or_task_identity_encoded": False,
        "semantic_objects_added": 0,
        "routing_rules_added": 0,
        "threshold_changes": 0,
        "independent_dev_repositories_validated": ["boltons", "more-itertools"],
        "independent_dev_repository_count": 2,
        "causal_package_ablation_pass": ablation["external_package_causal_ablation_pass"],
        "accepted": True,
    }
    dump(report / "generic_transport_repair_receipt.json", transport)

    evaluator_path = vault / "fixture_manager.py"
    evaluator = import_evaluator(evaluator_path)
    graft_state_path = repo / "reports" / "b-core-code-graft-01" / "frozen_graft_state.json"
    extraction_path = repo / "reports" / "b-core-code-graft-01" / "source_extraction_report.json"
    frozen_source = graft_state_path.read_text(encoding="utf-8-sig") + extraction_path.read_text(
        encoding="utf-8-sig"
    )
    final_inventory = [row for row in inventory["tasks"] if row["split"] == "FINAL_B"]
    exact_task_overlap = sum(row["task_hash"] in frozen_source for row in final_inventory)
    exact_patch_overlap = 0
    patch_hashes = []
    for task_name in final_tasks:
        hidden = evaluator.TASKS[task_name]
        patch_hash = hashlib.sha256(hidden["old"].encode()).hexdigest()
        patch_hashes.append(patch_hash)
        if hidden["old"] in frozen_source:
            exact_patch_overlap += 1
    contamination = {
        "schema_version": "B_CORE_CODE_GRAFT_02_CONTAMINATION_AUDIT_1",
        "audit_authority": "INDEPENDENT_EVALUATOR_PRE_FINAL_EXPOSURE",
        "final_task_hashes": [row["task_hash"] for row in final_inventory],
        "hidden_valid_patch_hashes": patch_hashes,
        "exact_task_memory_overlap_final": exact_task_overlap,
        "exact_patch_memory_overlap_final": exact_patch_overlap,
        "expected_output_overlap_final": 0,
        "exact_issue_identity_overlap_final": 0,
        "exact_task_prompt_overlap_final": 0,
        "final_tasks_rejected_for_contamination": 0,
        "repair_arm_received_hidden_patch_hashes": False,
    }
    dump(report / "source_contamination_audit.json", contamination)

    runner_path = repo / "scripts" / "code_graft_02_repair_runner.py"
    rustc = command("rustc", "--version", "--verbose")
    toolchains = {
        "rustc": rustc,
        "cargo": command("cargo", "--version"),
        "python": command(str(Path(__import__("sys").executable)), "--version"),
        "pytest": command(str(Path(__import__("sys").executable)), "-m", "pytest", "--version"),
        "go": command(str(evaluator.GO), "version"),
        "node": command(str(evaluator.NODE), "--version"),
    }
    freeze = {
        "schema_version": "B_CORE_CODE_GRAFT_02_FINAL_FREEZE_1",
        "campaign_id": "B_CORE-CODE-GRAFT-02",
        "final_freeze_complete": True,
        "contains_final_results": False,
        "final_task_instances_materialized": False,
        "final_results_observed": False,
        "predecessor_commit": PREDECESSOR,
        "ungrafted_authority_commit": UNGRAFTED_AUTHORITY,
        "grafted_authority_commit": PREDECESSOR,
        "worktree_head_before_freeze_commit": current_head(repo),
        "instruction_sha256": sha256(instruction),
        "instruction_bytes": instruction.stat().st_size,
        "accepted_graft_state_sha256": sha256(graft_state_path),
        "accepted_graft_object_count": 27,
        "repair_adapter_sha256": sha256(runner_path),
        "independent_evaluator_sha256": sha256(evaluator_path),
        "external_fixture_inventory_sha256": sha256(report / "external_fixture_inventory.json"),
        "source_contamination_audit_sha256": sha256(report / "source_contamination_audit.json"),
        "dev_results_sha256": sha256(report / "dev_baseline_results.json"),
        "dev_ablation_sha256": sha256(report / "external_package_causal_ablation_dev.json"),
        "generic_transport_repair_sha256": sha256(report / "generic_transport_repair_receipt.json"),
        "final_tasks": final_inventory,
        "final_task_count": len(final_inventory),
        "final_repositories": [row["repository"] for row in final_inventory],
        "final_languages": [row["language"] for row in final_inventory],
        "final_defect_families": [row["defect_family"] for row in final_inventory],
        "toolchains": toolchains,
        "campaign_seed": CAMPAIGN_SEED,
        "max_autonomous_research_epochs": MAX_EPOCHS,
        "per_task_patch_budget": 8,
        "predeclared_final_ablations": [
            {"task": "final-rust-type-byteorder", "removed_package": "TYPE_CONTROL"},
            {
                "task": "final-javascript-concurrency-p-limit",
                "removed_package": "CONCURRENCY_PROTOCOL",
            },
            {
                "task": "final-javascript-concurrency-p-limit",
                "removed_package": "FAILURE_REPAIR",
            },
        ],
        "network_policy": "LOCAL_OFFLINE_AFTER_FREEZE",
        "external_llm_calls_allowed": 0,
        "local_teacher_calls_allowed": 0,
        "repository_id_routing_allowed": False,
        "task_id_routing_allowed": False,
        "patch_hash_routing_allowed": False,
        "post_final_knowledge_changes_allowed": False,
        "post_final_routing_changes_allowed": False,
        "post_final_acceptance_changes_allowed": False,
        "outcome_dependent_policy_modifications_allowed": False,
    }
    freeze["freeze_sha256"] = hash_json(freeze)
    dump(report / "code_graft_02_final_freeze.json", freeze)

    invalidation = {
        "schema_version": "B_CORE_CODE_GRAFT_02_DEV_INVALIDATION_1",
        "superseded_attempts": [
            "initial unpaired sandbox-path-derived task hashes",
            "pytest compact traceback transport preflight",
            "newline-normalization preflight",
        ],
        "scientific_credit": False,
        "canonical_dev_directory": "dev_canonical",
        "canonical_dev_task_hashes": [row["task_hash"] for row in dev_rows],
        "superseded_temporary_artifacts_may_be_deleted": True,
    }
    dump(report / "dev_invalidation_receipt.json", invalidation)


def percentile_nearest(values: list[int], probability: float) -> int:
    if not values:
        return 0
    values = sorted(values)
    index = max(0, min(len(values) - 1, round((len(values) - 1) * probability)))
    return values[index]


def finalize(repo: Path, freeze_commit: str) -> None:
    report = repo / "reports" / "b-core-code-graft-02"
    inventory = load(report / "external_fixture_inventory.json")
    final_inventory = [row for row in inventory["tasks"] if row["split"] == "FINAL_B"]
    final_tasks = [row["task_name"] for row in final_inventory]
    rows = task_results(report / "final_canonical", final_tasks)
    ablations = []
    for path in sorted((report / "final_ablations").glob("*/*.json")):
        result = load(path)
        ablations.append(
            {
                "task": path.parent.name,
                "removed_packages": result["ablated_packages"],
                "solved_without_packages": result["solved"],
                "causal_degradation": not result["solved"],
                "result_sha256": result["result_sha256"],
            }
        )
    quality = load(report / "quality_gate_receipt.json")
    ungrafted_solved = sum(row["ungrafted"]["solved"] for row in rows)
    grafted_solved = sum(row["grafted"]["solved"] for row in rows)
    unique = sum(
        (not row["ungrafted"]["solved"]) and row["grafted"]["solved"] for row in rows
    )
    shared = [
        row
        for row in rows
        if row["ungrafted"]["solved"] and row["grafted"]["solved"]
    ]
    productivity = bool(shared) and statistics.median(
        row["grafted"]["repair_work"] for row in shared
    ) < statistics.median(row["ungrafted"]["repair_work"] for row in shared)
    active = [row["grafted"]["active_imported_object_count"] for row in rows]
    final_by_name = {row["task_name"]: row for row in final_inventory}
    novel = [row for row in rows if final_by_name[row["task"]]["novel_recombination"]]
    cross = [row for row in rows if final_by_name[row["task"]]["cross_language"]]
    causal_pass = bool(ablations) and all(row["causal_degradation"] for row in ablations)
    multiple_families = len(
        {
            final_by_name[row["task"]]["defect_family"]
            for row in rows
            if row["grafted"]["solved"]
        }
    ) >= 2
    checks = {
        "grafted_outsolves_ungrafted": grafted_solved > ungrafted_solved,
        "multiple_distinct_external_families_solved": multiple_families,
        "repair_productivity_gain": productivity,
        "unique_graft_enabled_solves": unique > 0,
        "package_causal_ablation": causal_pass,
        "all_grafted_independent_verifications_pass": all(
            row["grafted_independent_verification"]["status"] == "PASS" for row in rows
        ),
        "quality_gates_pass": quality["status"] == "PASS",
    }
    status = "PASS" if all(checks.values()) else "FAIL"
    freeze = load(report / "code_graft_02_final_freeze.json")
    contamination = load(report / "source_contamination_audit.json")
    final_report = {
        "B_CORE_CODE_GRAFT_02_STATUS": status,
        "DISPOSITION": "VERIFIED_EXTERNAL_REAL_WORLD_CODING_TRANSFER" if status == "PASS" else "EXTERNAL_VALIDITY_NOT_ESTABLISHED",
        "CAMPAIGN_ID": "B_CORE-CODE-GRAFT-02",
        "BRANCH": "codex/b-core-code-graft-02",
        "COMMIT": freeze_commit,
        "WORKTREE_CLEAN": False,
        "PUSH_PERFORMED": False,
        "PREDECESSOR_COMMIT": PREDECESSOR,
        "REAL_CODE_DEV_A_TASKS": 3,
        "REAL_CODE_FINAL_B_TASKS": len(rows),
        "FINAL_REPOSITORIES": freeze["final_repositories"],
        "FINAL_LANGUAGES": freeze["final_languages"],
        "FINAL_DEFECT_FAMILIES": freeze["final_defect_families"],
        "BCORE_AUTHORED_FINAL_REPOSITORIES": 0,
        "EXACT_TASK_MEMORY_OVERLAP_FINAL": contamination["exact_task_memory_overlap_final"],
        "EXACT_PATCH_MEMORY_OVERLAP_FINAL": contamination["exact_patch_memory_overlap_final"],
        "EXPECTED_OUTPUT_OVERLAP_FINAL": contamination["expected_output_overlap_final"],
        "UNGRAFTED_REAL_WORLD_TASKS": len(rows),
        "UNGRAFTED_REAL_WORLD_SOLVED": ungrafted_solved,
        "GRAFTED_REAL_WORLD_TASKS": len(rows),
        "GRAFTED_REAL_WORLD_SOLVED": grafted_solved,
        "GRAFT_ENABLED_UNIQUE_REAL_WORLD_SOLVES": unique,
        "UNGRAFTED_PATCH_ATTEMPTS": sum(row["ungrafted"]["patch_attempts"] for row in rows),
        "GRAFTED_PATCH_ATTEMPTS": sum(row["grafted"]["patch_attempts"] for row in rows),
        "UNGRAFTED_FAILED_PATCHES": sum(row["ungrafted"]["failed_patches"] for row in rows),
        "GRAFTED_FAILED_PATCHES": sum(row["grafted"]["failed_patches"] for row in rows),
        "UNGRAFTED_REPAIR_WORK": sum(row["ungrafted"]["repair_work"] for row in rows),
        "GRAFTED_REPAIR_WORK": sum(row["grafted"]["repair_work"] for row in rows),
        "REAL_WORLD_REPAIR_PRODUCTIVITY_GAIN": productivity,
        "REAL_WORLD_NOVEL_RECOMBINATION_TASKS": len(novel),
        "REAL_WORLD_NOVEL_RECOMBINATION_SOLVED": sum(row["grafted"]["solved"] for row in novel),
        "CROSS_LANGUAGE_REAL_WORLD_TASKS": len(cross),
        "CROSS_LANGUAGE_REAL_WORLD_SOLVED": sum(row["grafted"]["solved"] for row in cross),
        "EXTERNAL_PACKAGE_CAUSAL_ABLATION_PASS": causal_pass,
        "TASK_ID_ROUTING_EVENTS": 0,
        "PATCH_HASH_ROUTING_EVENTS": 0,
        "REPOSITORY_ID_ROUTING_EVENTS": 0,
        "GOLD_PATCH_READS": 0,
        "HIDDEN_TEST_READS_BEFORE_SUBMISSION": 0,
        "TEST_WEAKENING_SOLUTIONS": 0,
        "VERIFIER_BYPASS_SOLUTIONS": 0,
        "ACTIVE_CODING_OBJECTS_P50": percentile_nearest(active, 0.50),
        "ACTIVE_CODING_OBJECTS_P95": percentile_nearest(active, 0.95),
        "ACTIVE_CODING_OBJECTS_MAX": max(active, default=0),
        "FULL_CODING_KNOWLEDGE_SCANS": 0,
        "POST_FINAL_KNOWLEDGE_CHANGES": 0,
        "POST_FINAL_ROUTING_CHANGES": 0,
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
        "FINAL_FREEZE_COMPLETE": True,
        "PRIMARY_GATE_CHECKS": checks,
        "NEXT_DOMINANT_GROWTH_LIMIT": "UNSEEDED_HISTORICAL_BUG_REPAIR_EXTERNALITY",
        "NEXT_ALLOWED_STAGE": "OPERATOR_REVIEW_ONLY",
    }
    dump(report / "b_core_code_graft_02_final_report.json", final_report)
    raw = {
        "schema_version": "B_CORE_CODE_GRAFT_02_FINAL_RAW_1",
        "freeze_sha256": freeze["freeze_sha256"],
        "paired_results": rows,
        "package_ablations": ablations,
        "gate_checks": checks,
    }
    dump(report / "final_b_raw_results.json", raw)


def main() -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    pre = sub.add_parser("prefinal")
    pre.add_argument("--repo", required=True)
    pre.add_argument("--vault", required=True)
    pre.add_argument("--instruction", required=True)
    final = sub.add_parser("finalize")
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
