"""Build provenance-bound, primitive-specific rewrite calibration jobs.

The structural anchor bank remains immutable.  This only rehydrates the
Approved Response IR from the source artifact, selects eligible source
surfaces per operation, and records why an operation cannot yet be tested.
No surface is installed into the runtime.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path


SPLIT_QUOTA = {"TRAIN": 4, "VALIDATION": 2, "BLIND": 2}
HOLD_CONTEXT_REQUIRED = "SUBJECT_OMISSION"


def digest(value: object) -> str:
    return hashlib.sha256(
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def rehydrate(bank: dict) -> tuple[list[dict], Counter]:
    caches: dict[Path, dict[str, dict]] = {}
    result: list[dict] = []
    skipped: Counter = Counter()
    for anchor in bank["anchors"]:
        path = Path(anchor["source_candidate_path"])
        if path not in caches:
            try:
                candidates = json.loads(path.read_text(encoding="utf-8"))
                caches[path] = {row.get("prompt_sha256"): row for row in candidates.get("rows", [])}
            except (OSError, json.JSONDecodeError):
                caches[path] = {}
        source = caches[path].get(anchor["source_prompt_sha256"])
        if source is None:
            skipped["MISSING_SOURCE_PROVENANCE"] += 1
            continue
        ir = source.get("approved_response_ir")
        if not ir or source.get("surface") != anchor["surface"]:
            skipped["SOURCE_SURFACE_OR_IR_MISMATCH"] += 1
            continue
        split = source.get("split")
        if split not in SPLIT_QUOTA:
            skipped["MISSING_SPLIT"] += 1
            continue
        result.append({**anchor, "split": split, "approved_response_ir": ir})
    return result, skipped


def has_relation(anchor: dict) -> bool:
    relation = str(anchor["structure"].get("discourse_relation", "STATEMENT"))
    return relation not in {"STATEMENT", "UNKNOWN", "NONE", ""}


def eligible(operation: str, anchor: dict) -> bool:
    structure = anchor["structure"]
    ending = str(structure.get("ending", "OTHER"))
    clauses = int(structure.get("semantic_clause_count", 1))
    speech_act = str(anchor.get("speech_act") or "")
    if operation == "FORMALITY_UP":
        return ending != "FORMAL"
    if operation == "POLITENESS_UP":
        # There is no plain source in v2; do not silently relabel a polite
        # source as lower politeness.
        return ending in {"PLAIN", "INFORMAL"}
    if operation == "CONCISE":
        return clauses >= 2 or bool(structure.get("modifier_rich"))
    if operation == "EXPLANATORY":
        return clauses >= 2 or has_relation(anchor)
    if operation == "DISCOURSE_MARKER_INSERT":
        return has_relation(anchor) and structure.get("discourse_marker") == "NONE"
    if operation == "DISCOURSE_MARKER_REMOVE":
        return structure.get("discourse_marker") != "NONE"
    if operation == "LEXICAL_REGISTER_FORMAL":
        return ending != "FORMAL" and bool(structure.get("modifier_rich"))
    if operation == "EMOTIONAL_EXPRESSIVITY":
        return speech_act in {"QUERY", "REQUEST", "PROMISE", "REASSURE"}
    if operation in {"DIRECT", "SOFTEN", "HEDGE"}:
        return True
    raise ValueError(operation)


def select(operation: str, anchors: list[dict]) -> tuple[list[dict], dict]:
    candidates = [anchor for anchor in anchors if eligible(operation, anchor)]
    by_split: dict[str, list[dict]] = defaultdict(list)
    seen: set[tuple[str, str]] = set()
    for anchor in sorted(candidates, key=lambda row: (row["split"], row["meaning_family_id"], row["surface_sha256"])):
        # One source per family in a split prevents a repeated source variant
        # from inflating recurrence evidence.
        key = (anchor["split"], anchor["meaning_family_id"])
        if key not in seen:
            by_split[anchor["split"]].append(anchor)
            seen.add(key)
    counts = {split: len(by_split[split]) for split in SPLIT_QUOTA}
    ready = all(counts[split] >= quota for split, quota in SPLIT_QUOTA.items())
    selected: list[dict] = []
    selected_families: set[str] = set()
    if ready:
        # A family may appear in historical artifacts under multiple splits.
        # This calibration must nevertheless keep its own train/validation/
        # blind family boundary intact.
        for split, quota in SPLIT_QUOTA.items():
            available = [row for row in by_split[split] if row["meaning_family_id"] not in selected_families]
            if len(available) < quota:
                return [], {
                    "eligible_unique_family_count_by_split": counts,
                    "required_unique_family_count_by_split": SPLIT_QUOTA,
                    "status": "INSUFFICIENT_CROSS_SPLIT_FAMILY_SEPARATION",
                }
            chosen = available[:quota]
            selected.extend(chosen)
            selected_families.update(row["meaning_family_id"] for row in chosen)
    return selected, {
        "eligible_unique_family_count_by_split": counts,
        "required_unique_family_count_by_split": SPLIT_QUOTA,
        "status": "READY" if ready else "INSUFFICIENT_ELIGIBLE_SOURCES",
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bank", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    bank = json.loads(args.bank.read_text(encoding="utf-8"))
    anchors, skipped = rehydrate(bank)
    operations = (
        "FORMALITY_UP", "POLITENESS_UP", "CONCISE", "EXPLANATORY",
        "DISCOURSE_MARKER_INSERT", "DISCOURSE_MARKER_REMOVE",
        "LEXICAL_REGISTER_FORMAL", "EMOTIONAL_EXPRESSIVITY", "DIRECT", "SOFTEN", "HEDGE",
    )
    operation_results = {}
    jobs = []
    for operation in operations:
        selected, metadata = select(operation, anchors)
        operation_results[operation] = metadata
        for anchor in selected:
            source_id = hashlib.sha256(
                f"{anchor['source_prompt_sha256']}:{anchor['surface_sha256']}".encode()
            ).hexdigest()
            jobs.append({
                "job_id": hashlib.sha256(f"{operation}:{source_id}".encode()).hexdigest(),
                "source_id": source_id,
                "operation": operation,
                "meaning_family_id": anchor["meaning_family_id"],
                "split": anchor["split"],
                "speech_act": anchor.get("speech_act"),
                "surface": anchor["surface"],
                "surface_sha256": anchor["surface_sha256"],
                "approved_response_ir": anchor["approved_response_ir"],
                "source_provenance": {
                    "source_artifact": anchor["source_artifact"],
                    "source_candidate_path": anchor["source_candidate_path"],
                    "source_gate_path": anchor["source_gate_path"],
                    "source_prompt_sha256": anchor["source_prompt_sha256"],
                    "canonical_hash": anchor["canonical_hash"],
                },
                "structure": anchor["structure"],
            })
    sources = {job["source_id"] for job in jobs}
    artifact = {
        "schema": "BCORE.PRIMITIVE_SPECIFIC_REWRITE_JOBS.V1",
        "anchor_bank": str(args.bank),
        "anchor_bank_sha256": hashlib.sha256(args.bank.read_bytes()).hexdigest(),
        "global_structural_readiness": bank["readiness"],
        "readiness_policy": "PRIMITIVE_SPECIFIC_ONLY",
        "runtime_installation": "FORBIDDEN",
        "expression_primitive_promotion": "FORBIDDEN",
        "subject_omission": {
            "classification": "HOLD_CONTEXT_REQUIRED",
            "blocker": "CONTEXT_AWARE_REFERENCE_RECOVERY_AND_CONTEXT_AWARE_INVERSE_REQUIRED",
            "q4_rewrite_requested": False,
        },
        "rehydrated_anchor_count": len(anchors),
        "rehydration_skipped": dict(skipped),
        "operation_readiness": operation_results,
        "rewrite_job_count": len(jobs),
        "unique_control_source_count": len(sources),
        "jobs": jobs,
        "artifact_sha256": "",
    }
    artifact["artifact_sha256"] = digest(artifact)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(artifact, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "output": str(args.output),
        "jobs": len(jobs),
        "controls": len(sources),
        "operation_readiness": {key: value["status"] for key, value in operation_results.items()},
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
