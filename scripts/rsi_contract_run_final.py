from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path


FINAL_TASKS = [
    ("f1", "final-l3-retry-budget"),
    ("f2", "final-l3-generation-install"),
    ("f3", "final-l3-resource-budget"),
]
CONTINUITY_TASK = ("c1", "continuity-l3-rollback-checkpoint")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def dump(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def invoke(command: list[str], env: dict[str, str]) -> None:
    completed = subprocess.run(command, env=env, stdout=subprocess.DEVNULL, check=False)
    if completed.returncode != 0:
        raise RuntimeError(f"COMMAND_FAILED:{completed.returncode}:{command[1:4]}")


def run_task(
    python: str,
    manager: Path,
    proposer: Path,
    task: str,
    alias: str,
    sandbox_root: Path,
    report_root: Path,
    installed_root: Path,
    env: dict[str, str],
) -> dict:
    sandbox = sandbox_root / alias
    task_report = report_root / task
    task_report.mkdir(parents=True, exist_ok=False)
    materialization = task_report / "materialization.json"
    result = task_report / "result.json"
    verification = task_report / "verification.json"
    invoke(
        [
            python,
            str(manager),
            "materialize",
            "--task",
            task,
            "--arm",
            "FULL",
            "--destination",
            str(sandbox),
            "--output",
            str(materialization),
        ],
        env,
    )
    invoke(
        [python, str(proposer), "--manifest", str(sandbox / "public_task_manifest.json"), "--result", str(result)],
        env,
    )
    invoke(
        [
            python,
            str(manager),
            "verify",
            "--task",
            task,
            "--destination",
            str(sandbox),
            "--result",
            str(result),
            "--output",
            str(verification),
        ],
        env,
    )
    verification_value = json.loads(verification.read_text(encoding="utf-8"))
    row = {
        "task": task,
        "alias": alias,
        "sandbox": str(sandbox),
        "report": str(task_report),
        "decision": verification_value["decision"],
        "installed": False,
        "post_install_regression_pass": False,
        "rollback_available": False,
    }
    if verification_value["decision"] == "ACCEPT":
        installed = installed_root / f"r1-{alias}"
        installation = task_report / "installation.json"
        rollback = task_report / "rollback.json"
        post_install = task_report / "post_install.json"
        rollback_audit = task_report / "rollback_audit.json"
        invoke(
            [
                python,
                str(manager),
                "install",
                "--task",
                task,
                "--source",
                str(sandbox),
                "--result",
                str(result),
                "--verification",
                str(verification),
                "--proposer",
                str(proposer),
                "--destination",
                str(installed),
                "--installation-output",
                str(installation),
                "--rollback-output",
                str(rollback),
            ],
            env,
        )
        invoke(
            [
                python,
                str(manager),
                "post-install",
                "--task",
                task,
                "--destination",
                str(installed),
                "--result",
                str(result),
                "--output",
                str(post_install),
            ],
            env,
        )
        invoke(
            [
                python,
                str(manager),
                "rollback-audit",
                "--installation",
                str(installation),
                "--rollback",
                str(rollback),
                "--output",
                str(rollback_audit),
            ],
            env,
        )
        post_value = json.loads(post_install.read_text(encoding="utf-8"))
        rollback_value = json.loads(rollback_audit.read_text(encoding="utf-8"))
        row.update(
            {
                "installed": True,
                "installed_root": str(installed),
                "installed_proposer": str(installed / ".bcore_pipeline" / "rsi_contract_proposer.py"),
                "post_install_regression_pass": post_value["post_install_regression_pass"],
                "rollback_available": rollback_value["rollback_available"],
            }
        )
    return row


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", required=True)
    parser.add_argument("--vault", required=True)
    parser.add_argument("--run-root", required=True)
    parser.add_argument("--report-root", required=True)
    parser.add_argument("--mode", choices=["canary", "final"], default="final")
    parser.add_argument("--canary-task", default="dev-l1-capacity-boundary")
    args = parser.parse_args()

    repo = Path(args.repo).resolve(strict=True)
    vault = Path(args.vault).resolve(strict=True)
    run_root = Path(args.run_root).resolve()
    report_root = Path(args.report_root).resolve()
    manager = vault / "fixture_manager.py"
    proposer = repo / "scripts" / "rsi_contract_proposer.py"
    python = os.environ.get("B_CORE_PYTHON", "python")
    env = os.environ.copy()
    env.update(
        {
            "CARGO_NET_OFFLINE": "true",
            "CARGO_TARGET_DIR": str(vault / "build-cache"),
            "HTTP_PROXY": "",
            "HTTPS_PROXY": "",
            "NO_PROXY": "*",
        }
    )
    if run_root.exists():
        raise RuntimeError("RUN_ROOT_ALREADY_EXISTS")
    if report_root.exists():
        raise RuntimeError("REPORT_ROOT_ALREADY_EXISTS")
    run_root.mkdir(parents=True)
    report_root.mkdir(parents=True)

    if args.mode == "canary":
        row = run_task(
            python,
            manager,
            proposer,
            args.canary_task,
            "canary",
            run_root / "tasks",
            report_root / "tasks",
            run_root / "installed",
            env,
        )
        dump(report_root / "execution_complete.json", {"mode": "canary", "rows": [row]})
        print(json.dumps(row, indent=2, sort_keys=True))
        return 0

    freeze_path = repo / "reports" / "b-core-rsi-contract-01" / "final_freeze.json"
    freeze = json.loads(freeze_path.read_text(encoding="utf-8"))
    if not freeze["final_freeze_complete"]:
        raise RuntimeError("FINAL_FREEZE_INCOMPLETE")
    if freeze["proposer_sha256"] != sha256(proposer):
        raise RuntimeError("POST_FREEZE_PROPOSER_DRIFT")
    if freeze["independent_verifier_sha256"] != sha256(manager):
        raise RuntimeError("POST_FREEZE_VERIFIER_DRIFT")

    guard_path = repo / "reports" / "b-core-rsi-contract-01" / "final_exposure_guard.json"
    if guard_path.exists():
        raise RuntimeError("FINAL_ALREADY_EXPOSED")
    dump(
        guard_path,
        {
            "schema_version": "B_CORE_RSI_CONTRACT_FINAL_EXPOSURE_GUARD_1",
            "final_exposure_ordinal": 1,
            "freeze_sha256": freeze["freeze_sha256"],
            "post_final_changes_allowed": False,
        },
    )

    rows = []
    for alias, task in FINAL_TASKS:
        rows.append(
            run_task(
                python,
                manager,
                proposer,
                task,
                alias,
                run_root / "FINAL_B",
                report_root / "FINAL_B",
                run_root / "installed",
                env,
            )
        )

    continuity = {
        "task": CONTINUITY_TASK[1],
        "decision": "NOT_RUN",
        "continuity_pass": False,
        "reason": "NO_VERIFIED_R1",
    }
    first_installed = next((row for row in rows if row["installed"]), None)
    if first_installed is not None:
        installed_proposer = Path(first_installed["installed_proposer"])
        continuity_row = run_task(
            python,
            manager,
            installed_proposer,
            CONTINUITY_TASK[1],
            CONTINUITY_TASK[0],
            run_root / "CONTINUITY",
            report_root / "CONTINUITY",
            run_root / "installed_continuity",
            env,
        )
        continuity = {
            **continuity_row,
            "continuity_pass": continuity_row["decision"] == "ACCEPT"
            and continuity_row["post_install_regression_pass"],
            "r1_proposer_sha256": sha256(installed_proposer),
            "frozen_proposer_sha256": sha256(proposer),
            "r1_proposer_matches_frozen": sha256(installed_proposer) == sha256(proposer),
        }

    execution = {
        "schema_version": "B_CORE_RSI_CONTRACT_FINAL_EXECUTION_1",
        "final_exposure_ordinal": 1,
        "rows": rows,
        "continuity": continuity,
        "post_final_diagnostic_policy_changes": 0,
        "post_final_repair_spec_policy_changes": 0,
        "post_final_patch_engine_changes": 0,
        "post_final_verifier_changes": 0,
        "post_final_installer_changes": 0,
        "post_final_acceptance_changes": 0,
        "verifier_mutated_by_self_repair_events": 0,
        "installer_mutated_by_self_repair_events": 0,
        "acceptance_policy_mutated_by_self_repair_events": 0,
    }
    dump(report_root / "execution_complete.json", execution)
    print(json.dumps(execution, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
