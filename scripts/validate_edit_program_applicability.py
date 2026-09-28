"""Learn and exercise trace/IR applicability contracts for fixed EditPrograms.

No expression program is discovered here.  The three frozen recurrent ending
programs are evaluated in both directions against fresh, planner-grounded
sources.  Contract decisions use typed IR and RealizationTrace features only.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path

from realization_trace_execution_substrate import (
    ending,
    execute,
    particle_morphology_valid,
    realize,
    trace,
)


def digest(value: object) -> str:
    return hashlib.sha256(
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def features(ir: dict, realization: dict, surface: str) -> dict:
    terminal = next((c for c in reversed(surface.strip()) if c in ".?!"), None)
    return {
        "speech_act": ir["speech_act"],
        "operation": ir["operation"],
        "discourse_relation": ir["discourse_relation"],
        "claim_count": len(ir["claims"]),
        "modalities": sorted({claim["modality"] for claim in ir["claims"]}),
        "polarities": sorted({claim["polarity"] for claim in ir["claims"]}),
        "current_ending": ending(realization),
        "clause_count": len(realization["clause_structure"]),
        "terminal_punctuation": terminal,
        "honorific_morphology": realization["honorific_morphology"],
        "surface_roles": sorted({item["semantic_role"] for item in realization["slot_realizations"]}),
        "connective_count": len(realization["connectives"]),
        "source_morphology_valid": particle_morphology_valid(surface),
    }


def contract_for(program: dict, direction: str, observed: list[dict]) -> dict:
    op = program["representative_program"]["operations"][0]
    source_ending, target_ending = (op["from"], op["to"]) if direction == "FORWARD" else (op["to"], op["from"])
    source_rows = [row for row in observed if row["features"]["current_ending"] == source_ending]
    if not source_rows:
        raise RuntimeError(f"NO_OBSERVED_SOURCE:{program['program_cluster_id']}:{direction}")
    def one(name: str):
        values = {row["features"][name] for row in source_rows}
        if len(values) != 1:
            raise RuntimeError(f"NON_UNIQUE_CONTRACT_FEATURE:{name}:{sorted(values)}")
        return next(iter(values))
    return {
        "schema": "BCORE.EDIT_PROGRAM_APPLICABILITY_CONTRACT.V1",
        "program_cluster_id": program["program_cluster_id"],
        "direction": direction,
        "source_ending": source_ending,
        "target_ending": target_ending,
        "required_speech_act": one("speech_act"),
        "required_operation": one("operation"),
        "required_discourse_relation": one("discourse_relation"),
        "allowed_modalities": sorted({tuple(row["features"]["modalities"]) for row in source_rows}),
        "observed_clause_counts": sorted({row["features"]["clause_count"] for row in source_rows}),
        "observed_terminal_punctuation": sorted({row["features"]["terminal_punctuation"] for row in source_rows}),
        "requires_source_morphology_valid": True,
        "raw_surface_dependency": False,
        "observed_family_count": len({row["meaning_family_id"] for row in source_rows}),
    }


def decide(contract: dict, value: dict) -> tuple[str, list[str]]:
    reasons = []
    for feature, required in (
        ("current_ending", contract["source_ending"]),
        ("speech_act", contract["required_speech_act"]),
        ("operation", contract["required_operation"]),
        ("discourse_relation", contract["required_discourse_relation"]),
    ):
        if value[feature] != required:
            reasons.append(f"{feature.upper()}_MISMATCH")
    if contract["requires_source_morphology_valid"] and not value["source_morphology_valid"]:
        reasons.append("SOURCE_MORPHOLOGY_INVALID")
    return ("APPLY" if not reasons else "REJECT", reasons)


def candidate(row: dict, contract: dict, value: str, surface: str, source_hash: str, reasons: list[str]) -> dict:
    ir = row["approved_response_ir"]
    return {
        "meaning_family_id": ir["semantic_sha256"],
        "split": row["split"],
        "factor": "EDIT_PROGRAM_APPLICABILITY",
        "value": value,
        "baseline_value": "TRUSTED_SOURCE",
        "speech_act": ir["speech_act"],
        "approved_response_ir": ir,
        "surface": surface,
        "teacher_model": "NONE_OFFLINE_GENERIC_EXECUTOR",
        "prompt_sha256": digest({"source": source_hash, "contract": contract, "value": value, "surface": surface}),
        "conditions": {
            "program_cluster_id": contract["program_cluster_id"],
            "direction": contract["direction"],
            "contract_decision": "APPLY" if value == "APPLY_TRANSFER" else "REJECT",
            "contract_reasons": ",".join(reasons),
            "source_surface_sha256": source_hash,
        },
    }


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--substrate", type=Path, required=True)
    ap.add_argument("--classes", type=Path, required=True)
    ap.add_argument("--fresh", type=Path, required=True)
    ap.add_argument("--contracts-output", type=Path, required=True)
    ap.add_argument("--candidates-output", type=Path, required=True)
    args = ap.parse_args()
    substrate = json.loads(args.substrate.read_text(encoding="utf-8"))
    classes = json.loads(args.classes.read_text(encoding="utf-8"))
    fresh = json.loads(args.fresh.read_text(encoding="utf-8"))
    class_by_family = {item["meaning_family_id"]: item for item in classes["classes"]}
    trace_by_hash = {item["surface_sha256"]: item for item in substrate["traces"]}
    observed_by_cluster = defaultdict(list)
    for row in substrate["programs"]:
        ir = class_by_family[row["meaning_family_id"]]["approved_response_ir"]
        for surface_hash in (row["source_trace_sha256"], row["target_trace_sha256"]):
            realization = trace_by_hash[surface_hash]
            surface = realize(realization)
            observed_by_cluster[row["program_cluster_id"]].append({
                "meaning_family_id": row["meaning_family_id"],
                "features": features(ir, realization, surface),
            })

    contracts = []
    for program in substrate["recurrent_programs"]:
        for direction in ("FORWARD", "REVERSE"):
            contracts.append(contract_for(program, direction, observed_by_cluster[program["program_cluster_id"]]))

    decisions = []
    candidates = []
    for row in fresh["rows"]:
        ir = row["approved_response_ir"]
        source_surface = row["trusted_source_surface"]
        source_trace = trace(source_surface, ir)
        source_features = features(ir, source_trace, source_surface)
        source_hash = hashlib.sha256(source_surface.encode()).hexdigest()
        for contract in contracts:
            decision, reasons = decide(contract, source_features)
            expected_positive = (
                source_features["speech_act"] == contract["required_speech_act"]
                and source_features["current_ending"] == contract["source_ending"]
                and source_features["source_morphology_valid"]
            )
            record = {
                "meaning_family_id": ir["semantic_sha256"],
                "split": row["split"],
                "register_condition": row["register_condition"],
                "program_cluster_id": contract["program_cluster_id"],
                "direction": contract["direction"],
                "features": source_features,
                "expected_positive": expected_positive,
                "decision": decision,
                "reasons": reasons,
                "source_surface_sha256": source_hash,
            }
            decisions.append(record)
            if decision == "APPLY":
                program = {"schema": "BCORE.REALIZATION_EDIT_PROGRAM.V1", "operations": [{
                    "op": "CHANGE_ENDING", "from": contract["source_ending"], "to": contract["target_ending"]
                }]}
                transformed, executable = execute(source_trace, program)
                record["executable"] = executable
                if executable:
                    candidates.append(candidate(row, contract, "APPLY_TRANSFER", realize(transformed), source_hash, reasons))
            elif (
                source_features["speech_act"] == contract["required_speech_act"]
                and source_features["current_ending"] == contract["source_ending"]
                and not source_features["source_morphology_valid"]
            ):
                # Counterfactual control: execute despite the contract's
                # morphology rejection.  This tests whether inverse alone can
                # accidentally approve an unnatural realization.
                program = {"schema": "BCORE.REALIZATION_EDIT_PROGRAM.V1", "operations": [{
                    "op": "CHANGE_ENDING", "from": contract["source_ending"], "to": contract["target_ending"]
                }]}
                transformed, executable = execute(source_trace, program)
                record["forced_counterfactual_executable"] = executable
                if executable:
                    candidates.append(candidate(row, contract, "FORCED_REJECT_CONTROL", realize(transformed), source_hash, reasons))

    counts = Counter((row["split"], row["expected_positive"], row["decision"]) for row in decisions)
    valid = [row for row in decisions if row["features"]["source_morphology_valid"]]
    tp = sum(row["expected_positive"] and row["decision"] == "APPLY" for row in valid)
    fp = sum(not row["expected_positive"] and row["decision"] == "APPLY" for row in valid)
    fn = sum(row["expected_positive"] and row["decision"] == "REJECT" for row in valid)
    artifact = {
        "schema": "BCORE.EDIT_PROGRAM_APPLICABILITY_VALIDATION.V1",
        "runtime_installation": "FORBIDDEN",
        "program_discovery_expanded": False,
        "raw_surface_dependency": False,
        "contracts": contracts,
        "decision_count": len(decisions),
        "decision_counts": {"|".join(map(str, key)): value for key, value in sorted(counts.items())},
        "valid_source_precision": tp / (tp + fp) if tp + fp else None,
        "valid_source_recall": tp / (tp + fn) if tp + fn else None,
        "invalid_source_reject_count": sum(not row["features"]["source_morphology_valid"] and row["decision"] == "REJECT" for row in decisions),
        "decisions": decisions,
        "artifact_sha256": "",
    }
    artifact["artifact_sha256"] = digest(artifact)
    args.contracts_output.parent.mkdir(parents=True, exist_ok=True)
    args.contracts_output.write_text(json.dumps(artifact, ensure_ascii=False, indent=2), encoding="utf-8")
    args.candidates_output.write_text(json.dumps({
        "schema": "BCORE.EDIT_PROGRAM_APPLICABILITY_CANDIDATES.V1",
        "runtime_installation": "FORBIDDEN",
        "rows": candidates,
    }, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "contracts": len(contracts), "decisions": len(decisions), "candidates": len(candidates),
        "precision": artifact["valid_source_precision"], "recall": artifact["valid_source_recall"],
        "invalid_rejected": artifact["invalid_source_reject_count"],
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
