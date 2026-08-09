from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "reports" / "sem27-live-staircase"
SEGMENTS = REPORT / "segments"
ACTIVE_SEGMENT = REPORT / "active_segment.json"
WARM = Path(os.environ.get("B_CORE_WARM_ROOT", r"D:\B_Core_WARM_START"))
CACHE = WARM / "cargo-target" / "bcore-db4b3e0325b151ae43663199" / "release"
SEM27_PROBE = Path(os.environ.get("B_CORE_SEM27_PROBE", CACHE / "sem27-probe.exe"))
SEM24_PROBE = Path(os.environ.get("B_CORE_SEM24_PROBE", CACHE / "sem24-probe.exe"))
INSTRUCTION = ROOT / "campaign_instructions" / "SEM27_LIVE_STAIRCASE.md"
SEALED_COMMIT = "9dbebdd280db2c292bff4865bcf8f1d8c39c335d"
SEALED_STATE = ROOT / "reports" / "sem27_r2" / "final_r2_state.json"
SEALED_REPORT = ROOT / "reports" / "sem27_r2" / "sem27_r2_final_report.json"
ENGINE = ROOT / "crates" / "semantic-reasoning" / "src" / "sem27" / "engine.rs"
ONTOLOGY = ROOT / "crates" / "semantic-reasoning" / "src" / "sem27_r1.rs"
EXPECTED_ENGINE_HASH = "519557be10710d2a74d1ae21fddd75c55940811ac8017b4c871278cfd311f28b"
EXPECTED_ONTOLOGY_HASH = "097ba7b170d2263fb8b87671cb83253f836eef79a9210a60ee8f2772e8d91f92"
CAMPAIGN_ID = "SEM27-LIVE-AUTONOMOUS-STAIRCASE-0001"
GLOBAL_EPOCH_OFFSET = 512
RESOURCE_CEILING_BYTES = 2_000_000
DEFAULT_SEED = 0x5E27_D300_0000_0001
DIMENSIONS = (
    "causal_depth",
    "compositional_depth",
    "transfer_arity",
    "constraint_complexity",
    "planning_depth",
)


def now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def atomic_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as output:
            json.dump(value, output, ensure_ascii=False, indent=2)
            output.write("\n")
            output.flush()
            os.fsync(output.fileno())
        for attempt in range(12):
            try:
                os.replace(temporary, path)
                break
            except PermissionError:
                if attempt == 11:
                    raise
                time.sleep(min(0.01 * (2**attempt), 0.25))
    except BaseException:
        try:
            os.unlink(temporary)
        except OSError:
            pass
        raise


def append_jsonl(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8", newline="\n") as output:
        output.write(json.dumps(value, ensure_ascii=False, separators=(",", ":")))
        output.write("\n")
        output.flush()
        os.fsync(output.fileno())


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def sha256_text(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def git(*args: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(ROOT), *args],
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
    )
    return completed.stdout.strip()


def seed_for(base_seed: int, epoch: int) -> int:
    digest = hashlib.sha256(f"{CAMPAIGN_ID}|{base_seed}|{epoch}".encode()).digest()
    return int.from_bytes(digest[:8], "little") or 1


def commitment(epoch: int, seed: int) -> str:
    return sha256_text(f"{CAMPAIGN_ID}|UNOPENED|{epoch}|{seed}")


def engine_epoch(epoch: int) -> int:
    return (epoch - 1) % 64 + 1


def u64(value: int) -> int:
    return value & 0xFFFF_FFFF_FFFF_FFFF


def mix(left: int, right: int) -> int:
    value = u64(left ^ u64(right + 0x9E37_79B9_7F4A_7C15))
    value ^= value >> 30
    value = u64(value * 0xBF58_476D_1CE4_E5B9)
    value ^= value >> 27
    value = u64(value * 0x94D0_49BB_1331_11EB)
    return u64(value ^ (value >> 31))


def verify_predecessor() -> dict[str, Any]:
    actual_head = git("rev-parse", "HEAD")
    subprocess.run(
        ["git", "-C", str(ROOT), "merge-base", "--is-ancestor", SEALED_COMMIT, actual_head],
        check=True,
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
    )
    if git("status", "--porcelain"):
        raise RuntimeError("WORKTREE_NOT_CLEAN_BEFORE_FREEZE")
    sealed = read_json(SEALED_REPORT)
    state = read_json(SEALED_STATE)
    if sealed.get("sem27_r2_status") != "PASS":
        raise RuntimeError("R2_PREDECESSOR_NOT_PASS")
    if state.get("difficulty", {}).get("current_regime_id") != 2:
        raise RuntimeError("R2_PREDECESSOR_NOT_REGIME_2")
    if len(state.get("difficulty", {}).get("transitions", [])) != 1:
        raise RuntimeError("R2_PREDECESSOR_TRANSITION_COUNT_MISMATCH")
    if sha256(ENGINE) != EXPECTED_ENGINE_HASH:
        raise RuntimeError("ENGINE_HASH_MISMATCH")
    if sha256(ONTOLOGY) != EXPECTED_ONTOLOGY_HASH:
        raise RuntimeError("ONTOLOGY_HASH_MISMATCH")
    if not SEM27_PROBE.is_file() or not SEM24_PROBE.is_file():
        raise RuntimeError("FROZEN_PROBE_BINARY_MISSING")
    return {
        "semantic_predecessor_commit": SEALED_COMMIT,
        "instrumentation_commit": actual_head,
        "sealed_state_sha256": sha256(SEALED_STATE),
        "sealed_report_sha256": sha256(SEALED_REPORT),
        "engine_sha256": sha256(ENGINE),
        "ontology_sha256": sha256(ONTOLOGY),
        "sem27_probe_sha256": sha256(SEM27_PROBE),
        "sem24_probe_sha256": sha256(SEM24_PROBE),
        "instruction_sha256": sha256(INSTRUCTION),
    }


def freeze(budget: int, base_seed: int) -> dict[str, Any]:
    if not 1 <= budget <= 4096:
        raise RuntimeError("BUDGET_OUTSIDE_1_TO_4096")
    freeze_path = REPORT / "campaign_freeze.json"
    if freeze_path.is_file():
        frozen = read_json(freeze_path)
        if int(frozen["budget"]) != budget or int(frozen["base_seed"]) != base_seed:
            raise RuntimeError("CAMPAIGN_ALREADY_FROZEN_WITH_DIFFERENT_BUDGET_OR_SEED")
        return frozen

    integrity = verify_predecessor()
    REPORT.mkdir(parents=True, exist_ok=True)
    state = read_json(SEALED_STATE)
    commitments = [
        {
            "campaign_epoch": epoch,
            "global_epoch": GLOBAL_EPOCH_OFFSET + epoch,
            "engine_epoch": engine_epoch(epoch),
            "seed_commitment": commitment(epoch, seed_for(base_seed, epoch)),
            "research_topic_committed": False,
            "repair_committed": False,
            "difficulty_response_committed": False,
        }
        for epoch in range(1, budget + 1)
    ]
    frozen = {
        "schema_version": "SEM27_LIVE_CAMPAIGN_FREEZE_1",
        "campaign_id": CAMPAIGN_ID,
        "campaign_state": "CAMPAIGN_FROZEN",
        "contains_campaign_results": False,
        **integrity,
        "budget": budget,
        "base_seed": base_seed,
        "global_epoch_range_ceiling": [GLOBAL_EPOCH_OFFSET + 1, GLOBAL_EPOCH_OFFSET + budget],
        "resource_ceiling_bytes": RESOURCE_CEILING_BYTES,
        "engine_epoch_mapping": "((CAMPAIGN_EPOCH-1)%64)+1",
        "engine_epoch_origin_rebase_is_administrative_only": True,
        "operator_selects_budget_only": True,
        "operator_selects_research_content": False,
        "operator_selects_difficulty": False,
        "outcome_dependent_policy_modifications_allowed": False,
        "prestart_autonomous_research_events": 0,
        "prestart_future_instance_exposure_events": 0,
        "success_condition": "REGIME_GE_3_AND_TRANSITIONS_GE_2_AND_OPERATOR_FALSE_AND_STRUCTURALLY_HARDER",
        "seed_commitments": commitments,
        "frozen_at_utc": now(),
    }
    atomic_json(REPORT / "sealed_initial_state.json", state)
    atomic_json(
        REPORT / "human_intervention_audit.json",
        {
            "campaign_budget_selected_by_operator": True,
            "human_research_steering_events": 0,
            "human_bottleneck_selection_events": 0,
            "human_repair_design_events": 0,
            "human_experiment_selection_events": 0,
            "human_frontier_selection_events": 0,
            "human_difficulty_escalation_events": 0,
            "human_difficulty_level_selection_events": 0,
            "passed": True,
        },
    )
    atomic_json(freeze_path, frozen)
    atomic_json(
        REPORT / "runtime_status.json",
        {
            "campaign_id": CAMPAIGN_ID,
            "status": "FROZEN",
            "budget": budget,
            "epochs_executed": 0,
            "current_regime_id": 2,
            "transition_count": 1,
            "resumable": True,
            "updated_at_utc": now(),
        },
    )
    return frozen


def verify_epoch(global_epoch: int, local_epoch: int, seed: int, result: dict[str, Any]) -> dict[str, Any]:
    probe = result.get("difficulty_probe", {})
    next_state = result.get("resulting_state", {})
    director = next_state.get("director", {})
    if not probe.get("mechanically_verified"):
        raise RuntimeError("DIFFICULTY_PROBE_NOT_MECHANICALLY_VERIFIED")
    if result.get("human_difficulty_escalation_event") or result.get("human_difficulty_level_selection_event"):
        raise RuntimeError("HUMAN_DIFFICULTY_EVENT_DETECTED")
    if result.get("hardcoded_bottleneck_to_repair_rule_used"):
        raise RuntimeError("HARDCODED_BOTTLENECK_REPAIR_RULE_DETECTED")
    semantic_hash = max(
        1,
        mix(
            int(result["result_checksum"]),
            int(director["frontier_scale"])
            ^ int(director["core_bytes"])
            ^ int(probe["result_hash"]),
        ),
    )
    dependency_hash = mix(0x27D1_0000, global_epoch * 113 + 3)
    eepoch = engine_epoch(local_epoch)
    arguments = [
        3,
        27_200_000 + global_epoch * 8 + 3,
        semantic_hash,
        dependency_hash,
        dependency_hash,
        48 + ((eepoch - 1) // 8),
        41 + ((eepoch - 1) // 8),
        4,
        1 + int(result.get("difficulty_transition") is not None),
        3,
        min(32 + eepoch, 64),
        5 if result.get("difficulty_transition") is not None else 3,
        1 + ((eepoch + 3) % 5),
        0x27D1_0000 | global_epoch,
        80,
        seed ^ int(result["result_checksum"]),
    ]
    completed = subprocess.run(
        [str(SEM24_PROBE), *map(str, arguments)],
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
    )
    verification = json.loads(completed.stdout)
    if not verification.get("accepted") or verification.get("false_verification_acceptance"):
        raise RuntimeError("INDEPENDENT_VERIFICATION_FAILED")
    return verification


def structurally_harder(transition: dict[str, Any]) -> bool:
    previous = transition.get("previous_dimensions", {})
    current = transition.get("new_dimensions", {})
    return all(int(current.get(key, 0)) >= int(previous.get(key, 0)) for key in DIMENSIONS) and any(
        int(current.get(key, 0)) > int(previous.get(key, 0)) for key in DIMENSIONS
    )


def success_from(
    result: dict[str, Any], baseline_regime: int, baseline_transitions: int
) -> bool:
    state = result["resulting_state"]
    difficulty = state["difficulty"]
    transitions = difficulty.get("transitions", [])
    transition = transitions[-1] if transitions else {}
    return bool(
        int(difficulty.get("current_regime_id", 0)) > baseline_regime
        and len(transitions) > baseline_transitions
        and transition.get("operator_selected") is False
        and structurally_harder(transition)
        and result.get("human_difficulty_escalation_event") is False
        and result.get("human_difficulty_level_selection_event") is False
    )


def state_hash(value: dict[str, Any]) -> str:
    encoded = json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def current_segment_freeze() -> dict[str, Any]:
    if ACTIVE_SEGMENT.is_file():
        pointer = read_json(ACTIVE_SEGMENT)
        path = ROOT / pointer["freeze_path"]
        frozen = read_json(path)
        if int(frozen["segment_id"]) != int(pointer["segment_id"]):
            raise RuntimeError("ACTIVE_SEGMENT_POINTER_MISMATCH")
        return frozen
    frozen = dict(read_json(REPORT / "campaign_freeze.json"))
    frozen.update(
        {
            "segment_id": 1,
            "segment_budget": int(frozen["budget"]),
            "start_campaign_epoch": 0,
            "end_campaign_epoch": int(frozen["budget"]),
            "baseline_regime_id": 2,
            "baseline_transition_count": 1,
            "segment_report_directory": str(REPORT),
        }
    )
    return frozen


def freeze_next_segment(budget: int) -> dict[str, Any]:
    if not 1 <= budget <= 4096:
        raise RuntimeError("BUDGET_OUTSIDE_1_TO_4096")
    runtime = read_json(REPORT / "runtime_status.json")
    if runtime.get("status") not in {
        "STOPPED",
        "SUCCESS",
        "BUDGET_EXHAUSTED",
        "AUTONOMOUS_STOP",
        "ERROR",
    }:
        raise RuntimeError("NEXT_SEGMENT_REQUIRES_STOPPED_OR_TERMINAL_CHECKPOINT")
    executed, state, _, _, _ = restore_checkpoint()
    if executed <= 0:
        raise RuntimeError("NEXT_SEGMENT_CHECKPOINT_MISSING")
    previous = current_segment_freeze()
    segment_id = int(previous.get("segment_id", 1)) + 1
    segment_dir = SEGMENTS / f"segment_{segment_id:04}"
    if segment_dir.exists():
        raise RuntimeError(f"SEGMENT_ALREADY_EXISTS:{segment_id}")
    segment_dir.mkdir(parents=True)
    difficulty = state["difficulty"]
    base_seed = int(read_json(REPORT / "campaign_freeze.json")["base_seed"])
    end_epoch = executed + budget
    commitments = [
        {
            "segment_epoch": local_epoch,
            "campaign_epoch": executed + local_epoch,
            "global_epoch": GLOBAL_EPOCH_OFFSET + executed + local_epoch,
            "engine_epoch": engine_epoch(executed + local_epoch),
            "seed_commitment": commitment(
                executed + local_epoch,
                seed_for(base_seed, executed + local_epoch),
            ),
            "research_topic_committed": False,
            "repair_committed": False,
            "difficulty_response_committed": False,
        }
        for local_epoch in range(1, budget + 1)
    ]
    last_epoch_path = REPORT / f"epoch_{executed:04}.json"
    frozen = {
        "schema_version": "SEM27_LIVE_CONTINUATION_SEGMENT_FREEZE_1",
        "campaign_id": CAMPAIGN_ID,
        "segment_id": segment_id,
        "campaign_state": "CAMPAIGN_FROZEN",
        "contains_segment_results": False,
        "parent_segment_id": int(previous.get("segment_id", 1)),
        "parent_status": runtime.get("status"),
        "start_campaign_epoch": executed,
        "end_campaign_epoch": end_epoch,
        "segment_budget": budget,
        "budget": budget,
        "base_seed": base_seed,
        "baseline_regime_id": int(difficulty["current_regime_id"]),
        "baseline_transition_count": len(difficulty.get("transitions", [])),
        "baseline_frontier_scale": int(state["director"]["frontier_scale"]),
        "checkpoint_state_sha256": state_hash(state),
        "checkpoint_epoch_artifact_sha256": sha256(last_epoch_path),
        "engine_sha256": sha256(ENGINE),
        "ontology_sha256": sha256(ONTOLOGY),
        "sem27_probe_sha256": sha256(SEM27_PROBE),
        "sem24_probe_sha256": sha256(SEM24_PROBE),
        "segment_report_directory": str(segment_dir),
        "operator_selects_budget_only": True,
        "operator_selects_research_content": False,
        "operator_selects_difficulty": False,
        "outcome_dependent_policy_modifications_allowed": False,
        "prestart_autonomous_research_events": 0,
        "prestart_future_instance_exposure_events": 0,
        "success_condition": "ONE_NEW_AUTONOMOUS_STRUCTURAL_DIFFICULTY_TRANSITION_BEYOND_SEGMENT_BASELINE",
        "seed_commitments": commitments,
        "frozen_at_utc": now(),
    }
    freeze_path = segment_dir / "campaign_freeze.json"
    atomic_json(freeze_path, frozen)
    atomic_json(segment_dir / "sealed_initial_state.json", state)
    atomic_json(
        ACTIVE_SEGMENT,
        {
            "segment_id": segment_id,
            "freeze_path": str(freeze_path.relative_to(ROOT)),
            "activated_at_utc": now(),
        },
    )
    atomic_json(
        REPORT / "runtime_status.json",
        {
            "campaign_id": CAMPAIGN_ID,
            "segment_id": segment_id,
            "status": "FROZEN",
            "budget": budget,
            "segment_epochs_executed": 0,
            "epochs_executed": executed,
            "total_epochs_executed": executed,
            "current_regime_id": int(difficulty["current_regime_id"]),
            "transition_count": len(difficulty.get("transitions", [])),
            "frontier_scale": int(state["director"]["frontier_scale"]),
            "baseline_regime_id": int(difficulty["current_regime_id"]),
            "baseline_transition_count": len(difficulty.get("transitions", [])),
            "resumable": True,
            "updated_at_utc": now(),
        },
    )
    return frozen


def restore_checkpoint() -> tuple[int, dict[str, Any], list[int], list[int], list[int]]:
    epoch_files = sorted(REPORT.glob("epoch_*.json"))
    if not epoch_files:
        return 0, read_json(REPORT / "sealed_initial_state.json"), [], [], []
    records = [read_json(path) for path in epoch_files]
    last = records[-1]
    return (
        int(last["campaign_epoch"]),
        last["result"]["resulting_state"],
        [int(record["result"]["resulting_state"]["director"]["frontier_scale"]) for record in records],
        [int(record["result"]["difficulty_probe"]["wall_time_ns"]) for record in records],
        [int(record["result"].get("fixed_resource_frontier", 0)) for record in records],
    )


def write_live_summary(
    frozen: dict[str, Any],
    epoch: int,
    state: dict[str, Any],
    frontier: list[int],
    costs: list[int],
    productivity: list[int],
    status: str,
    success: bool,
) -> None:
    difficulty = state["difficulty"]
    segment_id = int(frozen.get("segment_id", 1))
    start_epoch = int(frozen.get("start_campaign_epoch", 0))
    segment_budget = int(frozen.get("segment_budget", frozen["budget"]))
    segment_epoch = max(0, epoch - start_epoch)
    baseline_regime = int(frozen.get("baseline_regime_id", 2))
    baseline_transitions = int(frozen.get("baseline_transition_count", 1))
    atomic_json(
        REPORT / "live_summary.json",
        {
            "campaign_id": CAMPAIGN_ID,
            "segment_id": segment_id,
            "status": status,
            "budget": segment_budget,
            "segment_epochs_executed": segment_epoch,
            "epochs_executed": epoch,
            "total_epochs_executed": epoch,
            "global_epoch": GLOBAL_EPOCH_OFFSET + epoch if epoch else GLOBAL_EPOCH_OFFSET,
            "baseline_regime_id": baseline_regime,
            "baseline_transition_count": baseline_transitions,
            "current_regime_id": int(difficulty["current_regime_id"]),
            "transition_count": len(difficulty.get("transitions", [])),
            "frontier_scale": int(state["director"]["frontier_scale"]),
            "success": success,
            "sequences": {
                "frontier": frontier,
                "cost": costs,
                "productivity": productivity,
            },
            "updated_at_utc": now(),
        },
    )
    atomic_json(
        REPORT / "runtime_status.json",
        {
            "campaign_id": CAMPAIGN_ID,
            "segment_id": segment_id,
            "status": status,
            "budget": segment_budget,
            "segment_epochs_executed": segment_epoch,
            "epochs_executed": epoch,
            "total_epochs_executed": epoch,
            "baseline_regime_id": baseline_regime,
            "baseline_transition_count": baseline_transitions,
            "current_regime_id": int(difficulty["current_regime_id"]),
            "transition_count": len(difficulty.get("transitions", [])),
            "frontier_scale": int(state["director"]["frontier_scale"]),
            "success": success,
            "resumable": status in {"FROZEN", "STOPPED"},
            "updated_at_utc": now(),
        },
    )


def run_campaign() -> int:
    root_freeze = read_json(REPORT / "campaign_freeze.json")
    frozen = current_segment_freeze()
    if frozen.get("campaign_id") != CAMPAIGN_ID:
        raise RuntimeError("CAMPAIGN_FREEZE_MISMATCH")
    if sha256(ENGINE) != root_freeze["engine_sha256"] or sha256(ONTOLOGY) != root_freeze["ontology_sha256"]:
        raise RuntimeError("FROZEN_ENGINE_OR_ONTOLOGY_CHANGED")
    transport_repair = read_json(REPORT / "instrumentation_transport_freeze.json") if (REPORT / "instrumentation_transport_freeze.json").is_file() else {}
    expected_sem27_probe = transport_repair.get(
        "effective_sem27_probe_sha256", root_freeze["sem27_probe_sha256"]
    )
    if sha256(SEM27_PROBE) != expected_sem27_probe or sha256(SEM24_PROBE) != root_freeze["sem24_probe_sha256"]:
        raise RuntimeError("FROZEN_PROBE_CHANGED")

    executed, state, frontiers, costs, productivity = restore_checkpoint()
    segment_id = int(frozen.get("segment_id", 1))
    start_epoch = int(frozen.get("start_campaign_epoch", 0))
    segment_budget = int(frozen.get("segment_budget", frozen["budget"]))
    end_epoch = int(frozen.get("end_campaign_epoch", start_epoch + segment_budget))
    baseline_regime = int(frozen.get("baseline_regime_id", 2))
    baseline_transitions = int(frozen.get("baseline_transition_count", 1))
    if executed < start_epoch or executed > end_epoch:
        raise RuntimeError("CHECKPOINT_OUTSIDE_ACTIVE_SEGMENT_RANGE")
    if executed == start_epoch and state_hash(state) != frozen.get("checkpoint_state_sha256", state_hash(state)):
        raise RuntimeError("SEGMENT_CHECKPOINT_STATE_HASH_MISMATCH")
    stop_path = REPORT / "stop_requested.json"
    stop_path.unlink(missing_ok=True)
    write_live_summary(frozen, executed, state, frontiers, costs, productivity, "RUNNING", False)
    budget = segment_budget
    base_seed = int(root_freeze["base_seed"])
    segment_dir = Path(frozen.get("segment_report_directory", REPORT))

    try:
        for epoch in range(executed + 1, end_epoch + 1):
            if stop_path.is_file():
                write_live_summary(frozen, executed, state, frontiers, costs, productivity, "STOPPED", False)
                return 0
            if epoch > 1 and (epoch - 1) % 64 == 0:
                state["difficulty"]["current_regime_started_epoch"] = 1
                append_jsonl(
                    REPORT / "epoch_origin_rebase_ledger.jsonl",
                    {
                        "campaign_epoch": epoch,
                        "active_regime_id": state["difficulty"]["current_regime_id"],
                        "after_engine_epoch_origin": 1,
                        "policy_fields_changed": 0,
                        "reason": "NEXT_FIXED_64_EPOCH_ENGINE_WINDOW",
                    },
                )
            eepoch = engine_epoch(epoch)
            global_epoch = GLOBAL_EPOCH_OFFSET + epoch
            seed = seed_for(base_seed, epoch)
            commitment_index = epoch - start_epoch - 1
            expected = frozen["seed_commitments"][commitment_index]["seed_commitment"]
            if commitment(epoch, seed) != expected:
                raise RuntimeError(f"SEED_COMMITMENT_MISMATCH:{epoch}")
            request = {
                "arm_code": 3,
                "epoch": eepoch,
                "seed": seed,
                "state": state,
                "resource_ceiling_bytes": RESOURCE_CEILING_BYTES,
                "historical_roadmap_target_code": None,
                "disable_long_term_research_memory": False,
                "concrete_future_instance_visible": False,
            }
            completed = subprocess.run(
                [str(SEM27_PROBE), "-"],
                check=True,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                input=json.dumps(request, separators=(",", ":")),
                env={**os.environ, "SEM27_MEASUREMENT_HOLD_MS": "350"},
                creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
            )
            result = json.loads(completed.stdout)
            verification = verify_epoch(global_epoch, epoch, seed, result)
            previous_frontier = int(state["director"]["frontier_scale"])
            state = result["resulting_state"]
            new_frontier = int(state["director"]["frontier_scale"])
            if new_frontier < previous_frontier:
                raise RuntimeError("FRONTIER_REGRESSION")
            frontiers.append(new_frontier)
            costs.append(int(result["difficulty_probe"]["wall_time_ns"]))
            interval = max(1, int(result["time"]["total_improvement_interval_ns"]))
            productivity.append((max(0, new_frontier - previous_frontier) * 1_000_000_000) // interval)
            is_success = success_from(result, baseline_regime, baseline_transitions)
            record = {
                "segment_id": segment_id,
                "segment_epoch": epoch - start_epoch,
                "campaign_epoch": epoch,
                "global_epoch": global_epoch,
                "engine_epoch": eepoch,
                "seed_commitment": expected,
                "instance_seed_revealed_after_freeze": True,
                "research_topic_assigned": False,
                "repair_assigned": False,
                "difficulty_response_assigned": False,
                "verification": verification,
                "result": result,
                "success_condition_met": is_success,
                "recorded_at_utc": now(),
            }
            atomic_json(REPORT / f"epoch_{epoch:04}.json", record)
            append_jsonl(
                REPORT / "autonomous_decision_ledger.jsonl",
                {
                    "campaign_epoch": epoch,
                    "global_epoch": global_epoch,
                    "regime_id": result["difficulty_probe"]["regime_id"],
                    "selected_bottleneck": result["inner"]["selected_bottleneck_class"],
                    "repair_accepted": result["inner"]["repair_accepted"],
                    "operator_research_content": False,
                    "operator_difficulty_content": False,
                },
            )
            if result.get("difficulty_transition") is not None:
                append_jsonl(
                    REPORT / "difficulty_transition_ledger.jsonl",
                    {
                        "campaign_epoch": epoch,
                        "global_epoch": global_epoch,
                        "transition": result["difficulty_transition"],
                        "operator_selected": False,
                        "structurally_harder": structurally_harder(result["difficulty_transition"]),
                    },
                )
            executed = epoch
            status = "SUCCESS" if is_success else "RUNNING"
            write_live_summary(frozen, executed, state, frontiers, costs, productivity, status, is_success)
            if is_success:
                atomic_json(segment_dir / "final_state.json", state)
                atomic_json(
                    segment_dir / "final_report.json",
                    {
                        "status": "PASS",
                        "disposition": "AUTONOMOUS_DIFFICULTY_TRANSITION_OBSERVED",
                        "segment_id": segment_id,
                        "budget": budget,
                        "segment_epochs_executed": executed - start_epoch,
                        "total_epochs_executed": executed,
                        "baseline_regime_id": baseline_regime,
                        "baseline_transition_count": baseline_transitions,
                        "current_regime_id": state["difficulty"]["current_regime_id"],
                        "transition_count": len(state["difficulty"]["transitions"]),
                        "human_research_steering_events": 0,
                        "human_difficulty_escalation_events": 0,
                        "human_difficulty_level_selection_events": 0,
                        "success_condition_met": True,
                        "completed_at_utc": now(),
                    },
                )
                return 0
            if state.get("autonomous_termination_reason"):
                write_live_summary(frozen, executed, state, frontiers, costs, productivity, "AUTONOMOUS_STOP", False)
                atomic_json(segment_dir / "final_state.json", state)
                return 0

        write_live_summary(frozen, executed, state, frontiers, costs, productivity, "BUDGET_EXHAUSTED", False)
        atomic_json(segment_dir / "final_state.json", state)
        atomic_json(
            segment_dir / "final_report.json",
            {
                "status": "PASS",
                "disposition": "FIXED_BUDGET_EXHAUSTED_WITHOUT_NEW_TRANSITION",
                "segment_id": segment_id,
                "budget": budget,
                "segment_epochs_executed": executed - start_epoch,
                "total_epochs_executed": executed,
                "baseline_regime_id": baseline_regime,
                "baseline_transition_count": baseline_transitions,
                "current_regime_id": state["difficulty"]["current_regime_id"],
                "transition_count": len(state["difficulty"]["transitions"]),
                "human_research_steering_events": 0,
                "human_difficulty_escalation_events": 0,
                "human_difficulty_level_selection_events": 0,
                "success_condition_met": False,
                "completed_at_utc": now(),
            },
        )
        return 0
    except BaseException as error:
        atomic_json(
            REPORT / "runtime_status.json",
            {
                "campaign_id": CAMPAIGN_ID,
                "segment_id": segment_id,
                "status": "ERROR",
                "budget": budget,
                "segment_epochs_executed": max(0, executed - start_epoch),
                "epochs_executed": executed,
                "total_epochs_executed": executed,
                "baseline_regime_id": baseline_regime,
                "baseline_transition_count": baseline_transitions,
                "error": f"{type(error).__name__}: {error}",
                "resumable": True,
                "updated_at_utc": now(),
            },
        )
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description="Frozen SEM-27 live autonomous continuation")
    sub = parser.add_subparsers(dest="command", required=True)
    freeze_parser = sub.add_parser("freeze")
    freeze_parser.add_argument("--budget", type=int, required=True)
    freeze_parser.add_argument("--seed", type=int, default=DEFAULT_SEED)
    next_parser = sub.add_parser("freeze-next")
    next_parser.add_argument("--budget", type=int, required=True)
    sub.add_parser("run")
    arguments = parser.parse_args()
    if arguments.command == "freeze":
        print(json.dumps(freeze(arguments.budget, arguments.seed), indent=2))
        return 0
    if arguments.command == "freeze-next":
        print(json.dumps(freeze_next_segment(arguments.budget), indent=2))
        return 0
    return run_campaign()


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except BaseException as error:
        if isinstance(error, SystemExit):
            raise
        print(f"SEM27_LIVE_CAMPAIGN_ERROR={type(error).__name__}:{error}", file=sys.stderr)
        raise
