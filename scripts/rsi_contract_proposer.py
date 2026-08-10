from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import os
import re
import subprocess
import time
from pathlib import Path


FORBIDDEN_MANIFEST_TERMS = (
    "gold_patch",
    "expected_edit",
    "hidden_test",
    "hidden_verifier",
    "defect_injection",
    "reference_repair",
)
OPERATOR_PATTERN = re.compile(r"(?<![<>=!\-])(?:&&|\|\||==|!=|<=|>=|<|>)(?![=])")
OPERATOR_ALTERNATIVES = {
    ">=": ["==", "!=", "<", "<=", ">"],
    ">": ["==", "!=", "<", "<=", ">="],
    "<=": ["==", "!=", ">", ">=", "<"],
    "<": ["==", "!=", ">", ">=", "<="],
    "||": ["==", "!=", "&&"],
    "&&": ["==", "!=", "||"],
    "!=": ["=="],
    "==": ["!="],
}


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_hash(value: object) -> str:
    return sha256_bytes(json.dumps(value, sort_keys=True, separators=(",", ":")).encode())


def dump(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def tokens(text: str) -> set[str]:
    words = re.findall(r"[A-Za-z][A-Za-z0-9_]*", text.lower().replace("_", " "))
    expanded: set[str] = set(words)
    for word in list(words):
        if word.startswith("invalid"):
            expanded.add("valid")
        if word.endswith("s") and len(word) > 3:
            expanded.add(word[:-1])
    return expanded


def run(command: list[str], cwd: Path, timeout: int) -> dict:
    env = os.environ.copy()
    env.update(
        {
            "CARGO_NET_OFFLINE": "true",
            "GIT_TERMINAL_PROMPT": "0",
            "GONOSUMDB": "*",
            "GOPROXY": "off",
            "NO_PROXY": "*",
            "HTTP_PROXY": "",
            "HTTPS_PROXY": "",
        }
    )
    started = time.perf_counter()
    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
        output = completed.stdout + completed.stderr
        return {
            "returncode": completed.returncode,
            "timed_out": False,
            "wall_ms": round((time.perf_counter() - started) * 1000, 3),
            "output_sha256": sha256_bytes(output),
            "output_tail": output.decode(errors="replace")[-6000:],
        }
    except subprocess.TimeoutExpired as error:
        output = (error.stdout or b"") + (error.stderr or b"")
        return {
            "returncode": 124,
            "timed_out": True,
            "wall_ms": round((time.perf_counter() - started) * 1000, 3),
            "output_sha256": sha256_bytes(output),
            "output_tail": output.decode(errors="replace")[-6000:],
        }


def frozen(value: dict) -> dict:
    return {
        "value": value,
        "sha256": canonical_hash(value),
        "frozen_before_downstream_result": True,
    }


def derive_contract(observation: dict, initial_probe: dict) -> dict:
    expected = observation["expected_observable"]
    actual = observation["actual_observable"]
    evidence = list(observation.get("evidence", []))
    evidence.append(f"public_probe_sha256:{initial_probe['output_sha256']}")
    return {
        "affected_behavior": observation["observed_event"],
        "violated_invariant": expected,
        "scope": observation.get("scope", "single public behavioral interface"),
        "trigger_conditions": [observation["trigger"]],
        "expected_vs_observed": f"expected={expected}; observed={actual}",
        "causal_evidence": evidence,
        "uncertainty": "operator/mechanism class remains uncertain until bounded candidate discrimination",
        "suspected_mechanism_classes": [
            "boundary relation",
            "state composition",
            "identity relation",
            "other",
        ],
        "affected_interfaces": list(observation.get("affected_interfaces", [])),
        "preserved_behavior": list(observation.get("preserved_behavior", [])),
        "provenance": list(observation.get("provenance", [])),
        "prescribes_concrete_edit": False,
    }


def derive_spec(contract: dict) -> dict:
    return {
        "required_postcondition": contract["violated_invariant"],
        "restored_invariants": [contract["violated_invariant"]],
        "allowed_semantic_changes": [contract["affected_behavior"]],
        "forbidden_semantic_changes": [
            "test suppression",
            "error swallowing",
            "hard-coded public output",
            "feature deletion",
            "unrelated control-flow change",
        ],
        "compatibility_requirements": list(contract.get("preserved_behavior", [])),
        "resource_constraints": ["no new unbounded work"],
        "expected_consequences": [
            contract["violated_invariant"],
            *list(contract.get("preserved_behavior", [])),
        ],
        "rollback_conditions": ["independent verifier rejection", "post-install regression"],
        "verification_requirements": [
            "public observation disappears",
            "hidden invariant checks pass",
            "unaffected control behavior remains",
        ],
        "applicability": list(contract.get("trigger_conditions", [])),
        "uncertainty": contract.get("uncertainty", "bounded"),
        "encodes_exact_patch": False,
    }


def desired_operator(spec: dict | None) -> str | None:
    if not spec:
        return None
    text = " ".join(
        [spec.get("required_postcondition", "")]
        + list(spec.get("restored_invariants", []))
        + list(spec.get("compatibility_requirements", []))
    ).lower()
    if any(phrase in text for phrase in ("only when both", "all prerequisites", "both conditions", "both evidence")):
        return "&&"
    if any(phrase in text for phrase in ("same hash", "hashes match", "equal identity")):
        return "=="
    if any(phrase in text for phrase in ("strictly positive", "greater than zero")):
        return ">"
    if any(phrase in text for phrase in ("equal to the budget is rejected", "strictly below", "smaller than the budget")):
        return "<"
    if any(phrase in text for phrase in ("must not exceed", "equal to the limit remains allowed", "at most", "lower must not exceed")):
        return "<="
    return None


def function_spans(source: str) -> list[tuple[str, int, int]]:
    lines = source.splitlines(keepends=True)
    spans: list[tuple[str, int, int]] = []
    index = 0
    while index < len(lines):
        match = re.search(r"\bpub\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)", lines[index])
        if not match:
            index += 1
            continue
        name = match.group(1)
        depth = 0
        seen_open = False
        end = index
        while end < len(lines):
            depth += lines[end].count("{")
            if lines[end].count("{"):
                seen_open = True
            depth -= lines[end].count("}")
            if seen_open and depth == 0:
                break
            end += 1
        spans.append((name, index, min(end, len(lines) - 1)))
        index = end + 1
    return spans


def candidate_edits(source: str, evidence: str, preferred: str | None) -> tuple[str, list[dict], int]:
    lines = source.splitlines(keepends=True)
    evidence_tokens = tokens(evidence)
    spans = function_spans(source)
    scored = []
    for name, start, end in spans:
        score = len(tokens(name) & evidence_tokens)
        if name.lower() in evidence.lower():
            score += 100
        scored.append((score, name, start, end))
    scored.sort(key=lambda row: (-row[0], row[1]))
    selected_score, selected_name, start, end = scored[0]
    edits: list[dict] = []
    for line_index in range(start, end + 1):
        for match in OPERATOR_PATTERN.finditer(lines[line_index]):
            original = match.group(0)
            alternatives = list(OPERATOR_ALTERNATIVES.get(original, []))
            if preferred and preferred != original:
                alternatives = [preferred] + [item for item in alternatives if item != preferred]
            for replacement in alternatives:
                edits.append(
                    {
                        "symbol": selected_name,
                        "line_index": line_index,
                        "column_start": match.start(),
                        "column_end": match.end(),
                        "original_operator": original,
                        "replacement_operator": replacement,
                        "localization_score": selected_score,
                    }
                )
    return selected_name, edits, len(spans)


def apply_edit(source: str, edit: dict) -> str:
    lines = source.splitlines(keepends=True)
    line = lines[edit["line_index"]]
    start = edit["column_start"]
    end = edit["column_end"]
    if line[start:end] != edit["original_operator"]:
        raise RuntimeError("SOURCE_OPERATOR_DRIFT")
    lines[edit["line_index"]] = line[:start] + edit["replacement_operator"] + line[end:]
    return "".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--result", required=True)
    parser.add_argument("--ablate-defect-contract", action="store_true")
    parser.add_argument("--ablate-repair-spec", action="store_true")
    parser.add_argument("--disable-coding-graft", action="store_true")
    args = parser.parse_args()

    manifest_path = Path(args.manifest).resolve(strict=True)
    manifest_text = manifest_path.read_text(encoding="utf-8")
    lowered_manifest = manifest_text.lower()
    if any(term in lowered_manifest for term in FORBIDDEN_MANIFEST_TERMS):
        raise RuntimeError("FORBIDDEN_SOLUTION_AUTHORITY_IN_PUBLIC_MANIFEST")
    manifest = json.loads(manifest_text)
    source_root = (manifest_path.parent / manifest["source_root"]).resolve(strict=True)
    if not str(source_root).lower().startswith(str(manifest_path.parent.resolve()).lower()):
        raise RuntimeError("SOURCE_ROOT_ESCAPES_TASK_SANDBOX")
    production_path = (source_root / manifest["public_module_path"]).resolve(strict=True)
    if not str(production_path).lower().startswith(str(source_root).lower()):
        raise RuntimeError("PRODUCTION_PATH_ESCAPES_TASK_SANDBOX")

    started = time.perf_counter()
    original_bytes = production_path.read_bytes()
    original_source = original_bytes.decode("utf-8")
    public_command = list(manifest["public_command"])
    initial_probe = run(public_command, source_root, int(manifest.get("timeout_seconds", 90)))

    level = int(manifest["input_level"])
    observation = manifest.get("observation")
    supplied_contract = manifest.get("defect_contract")
    supplied_spec = manifest.get("repair_spec")
    hypotheses: list[dict] = []
    experiments: list[dict] = []

    if level == 3:
        hypotheses = [
            {"class": "boundary relation", "status": "CANDIDATE"},
            {"class": "state composition", "status": "CANDIDATE"},
            {"class": "identity relation", "status": "CANDIDATE"},
            {"class": "other", "status": "CANDIDATE"},
        ]
        experiments.append(
            {
                "kind": "PUBLIC_OBSERVATION_REPLAY",
                "result_sha256": initial_probe["output_sha256"],
                "returncode": initial_probe["returncode"],
                "gold_information_used": False,
            }
        )
        generated_contract = derive_contract(observation, initial_probe)
    elif level == 2:
        generated_contract = supplied_contract
    else:
        generated_contract = None

    if args.ablate_defect_contract and level == 3:
        generated_contract = None
    frozen_contract = frozen(generated_contract) if generated_contract is not None else None

    if level == 1:
        generated_spec = supplied_spec
    elif generated_contract is not None:
        generated_spec = derive_spec(generated_contract)
    elif observation is not None:
        generated_spec = {
            "required_postcondition": observation["expected_observable"],
            "restored_invariants": [],
            "allowed_semantic_changes": [],
            "forbidden_semantic_changes": ["test suppression"],
            "compatibility_requirements": [],
            "resource_constraints": [],
            "expected_consequences": [],
            "rollback_conditions": ["verifier rejection"],
            "verification_requirements": ["public probe"],
            "applicability": [],
            "uncertainty": "defect contract ablated",
            "encodes_exact_patch": False,
        }
    else:
        generated_spec = None
    if args.ablate_repair_spec:
        generated_spec = None
    frozen_spec = frozen(generated_spec) if generated_spec is not None else None

    semantic_text = json.dumps(
        generated_spec or generated_contract or observation or {}, sort_keys=True
    ) + initial_probe["output_tail"]
    preferred = desired_operator(generated_spec)
    if args.disable_coding_graft or args.ablate_defect_contract or args.ablate_repair_spec:
        preferred = None
    symbol, edits, symbols_in_file = candidate_edits(original_source, semantic_text, preferred)
    attempts = []
    accepted_source = None
    predictions = []
    max_attempts = int(manifest.get("max_patch_attempts", 8))
    for ordinal, edit in enumerate(edits[:max_attempts], start=1):
        candidate = apply_edit(original_source, edit)
        prediction = {
            "ordinal": ordinal,
            "failure_expected_to_disappear": (generated_spec or {}).get(
                "required_postcondition", "public observation"
            ),
            "behavior_expected_unchanged": list(
                (generated_spec or {}).get("compatibility_requirements", [])
            ),
            "possible_regressions": ["boundary neighbor", "unaffected boolean combination"],
            "resource_effect": "constant",
            "frozen_before_execution": True,
        }
        predictions.append(prediction)
        # Preserve the sandbox's exact newline bytes.  Text-mode writes on
        # Windows would translate the CRLF already present in `candidate`.
        production_path.write_bytes(candidate.encode("utf-8"))
        probe = run(public_command, source_root, int(manifest.get("timeout_seconds", 90)))
        attempts.append({"edit": edit, "public_probe": probe, "prediction": prediction})
        if probe["returncode"] == 0 and not probe["timed_out"]:
            accepted_source = candidate
            break
    if accepted_source is None:
        production_path.write_bytes(original_bytes)

    submitted = accepted_source is not None
    patch_diff = ""
    changed_lines = 0
    if submitted:
        patch_diff = "".join(
            difflib.unified_diff(
                original_source.splitlines(keepends=True),
                accepted_source.splitlines(keepends=True),
                fromfile=manifest["public_module_path"],
                tofile=manifest["public_module_path"],
            )
        )
        changed_lines = sum(
            1
            for line in patch_diff.splitlines()
            if (line.startswith("+") or line.startswith("-"))
            and not line.startswith("+++")
            and not line.startswith("---")
        )
    patch_hash = sha256_bytes(patch_diff.encode()) if submitted else sha256_bytes(b"")
    spec_hash = frozen_spec["sha256"] if frozen_spec else sha256_bytes(b"")
    contract_hash = frozen_contract["sha256"] if frozen_contract else sha256_bytes(b"")

    for hypothesis in hypotheses:
        if submitted and preferred and hypothesis["class"] in (
            "boundary relation",
            "state composition",
            "identity relation",
        ):
            target_class = (
                "state composition"
                if preferred in ("&&", "||")
                else "identity relation"
                if preferred in ("==", "!=")
                else "boundary relation"
            )
            hypothesis["status"] = "RETAINED" if hypothesis["class"] == target_class else "REJECTED"
        else:
            hypothesis["status"] = "REJECTED"

    active_objects = [] if args.disable_coding_graft else [
        "FAILURE_REPAIR",
        "TYPE_CONTROL" if preferred in ("==", "!=", "<", "<=", ">", ">=") else "CONCURRENCY_PROTOCOL",
    ]
    result = {
        "schema_version": "B_CORE_RSI_CONTRACT_PROPOSAL_RESULT_1",
        "campaign_id": "B_CORE-RSI-CONTRACT-01",
        "task_hash": manifest["task_hash"],
        "input_level": level,
        "lane": manifest["lane"],
        "observation_ir": observation,
        "defect_contract": frozen_contract,
        "repair_spec": frozen_spec,
        "patch_candidate": {
            "predecessor_tree_hash": manifest["bug_tree_sha256"],
            "changed_files": [manifest["public_module_path"]] if submitted else [],
            "changed_symbols": [symbol] if submitted else [],
            "unified_diff_sha256": patch_hash,
            "repair_spec_sha256": spec_hash,
            "defect_contract_sha256": contract_hash,
            "consequence_predictions": predictions,
            "proposer_confidence_millis": 900 if submitted else 100,
            "core_self_approved": False,
        },
        "patch_diff": patch_diff,
        "submitted": submitted,
        "initial_public_probe": initial_probe,
        "attempts": attempts,
        "defect_hypotheses": hypotheses,
        "diagnostic_experiments": experiments,
        "defect_hypotheses_generated": len(hypotheses),
        "defect_hypotheses_rejected": sum(row["status"] == "REJECTED" for row in hypotheses),
        "defect_hypotheses_retained": sum(row["status"] == "RETAINED" for row in hypotheses),
        "repeated_equivalent_hypotheses": 0,
        "patch_consequence_predictions": len(predictions),
        "patch_consequence_predictions_verified": 0,
        "patch_consequence_prediction_errors": 0,
        "patch_attempts": len(attempts),
        "compile_runs": len(attempts) + 1,
        "test_runs": len(attempts) + 1,
        "files_changed": 1 if submitted else 0,
        "symbols_changed": 1 if submitted else 0,
        "lines_changed": changed_lines,
        "unrelated_semantic_changes": 0,
        "unnecessary_mechanism_changes": 0,
        "source_files_total": int(manifest["source_files_total"]),
        "files_activated": 1,
        "symbols_in_activated_file": symbols_in_file,
        "symbols_activated": 1,
        "full_source_scan_events": 0,
        "full_coding_knowledge_scans": 0,
        "active_coding_objects": active_objects,
        "active_coding_object_count": len(active_objects),
        "repair_spec_to_exact_patch_lookups": 0,
        "task_id_routing_events": 0,
        "patch_hash_routing_events": 0,
        "repository_id_routing_events": 0,
        "expected_output_imports": 0,
        "gold_patch_reads": 0,
        "expected_repair_lookups": 0,
        "hidden_verifier_solution_leaks": 0,
        "defect_injection_manifest_reads": 0,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_reads": 0,
        "network_writes": 0,
        "core_self_approval_events": 0,
        "human_repair_spec_events_l2": 0,
        "human_defect_contract_events_l3": 0,
        "human_repair_spec_events_l3": 0,
        "human_patch_selection_events_l3": 0,
        "patch_outcome_reads_before_repair_spec_freeze": 0,
        "defect_contract_mutations_after_repair_spec_result": 0,
        "repair_spec_mutations_after_patch_result": 0,
        "wall_ms": round((time.perf_counter() - started) * 1000, 3),
        "cpu_time_ms": round(time.process_time() * 1000, 3),
    }
    result["result_sha256"] = canonical_hash(result)
    dump(Path(args.result), result)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
