"""Analyze primitive-specific rewrites after the canonical inverse gate.

This report records structural deltas and recurrent signatures only.  It does
not turn a teacher surface into a template or register a runtime primitive.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

from analyze_paired_teacher_state_sensitivity import features, structural_delta


CONNECTIVES = ("그리고", "하지만", "그래서", "다만", "따라서", "우선", "먼저", "그러나", "그런데")


def lexical_changed(before: str, after: str) -> bool:
    return set(re.findall(r"[가-힣A-Za-z0-9]+", before)) != set(re.findall(r"[가-힣A-Za-z0-9]+", after))


def delta_signature(before: str, after: str) -> tuple[str, ...]:
    delta = structural_delta(features(before), features(after))
    parts = [f"{name}:{direction}" for name, direction in sorted(delta.items())]
    if lexical_changed(before, after):
        parts.append("lexical_substitution")
    before_connectives = {item for item in CONNECTIVES if item in before}
    after_connectives = {item for item in CONNECTIVES if item in after}
    if before_connectives != after_connectives:
        parts.append("connective:ADD" if after_connectives - before_connectives else "connective:REMOVE")
    if len(re.findall(r"\b(\w+)\s+\1\b", after)) > len(re.findall(r"\b(\w+)\s+\1\b", before)):
        parts.append("redundancy:UP")
    return tuple(sorted(parts)) or ("IDENTICAL",)


def classify(operation: str, rows: list[dict], readiness: dict, hold: dict) -> str:
    if operation == "SUBJECT_OMISSION":
        return hold["classification"]
    if readiness[operation]["status"] != "READY":
        return "INSUFFICIENT_ELIGIBLE_SOURCES"
    if not rows:
        return "TEACHER_NOISE"
    approved = [row for row in rows if row["approval"] == "APPROVED_STATE_CONTRAST_GOLD"]
    if not approved or len(approved) / len(rows) < 0.50:
        return "SEMANTIC_RISK"
    non_identical = [row for row in approved if row["operation_delta"] != ("IDENTICAL",)]
    if not non_identical:
        return "ALREADY_SATURATED"
    # A NO-OP structural rewrite means the source itself is noisy.  It must be
    # lower than the operation effect before considering a primitive.
    operation_effect = sum(row["operation_delta"] != ("IDENTICAL",) for row in approved) / len(approved)
    control_effect = sum(row["control_delta"] != ("IDENTICAL",) for row in approved) / len(approved)
    if operation_effect <= control_effect:
        return "TEACHER_NOISE"
    signatures = Counter(row["operation_delta"] for row in non_identical)
    dominant, count = signatures.most_common(1)[0]
    has_all_splits = all(
        any(row["split"] == split and row["operation_delta"] == dominant for row in non_identical)
        for split in ("TRAIN", "VALIDATION", "BLIND")
    )
    if has_all_splits and count / len(non_identical) >= 0.60 and control_effect < operation_effect:
        return "INDEPENDENT_PRIMITIVE" if len(dominant) == 1 else "COMPOSITE_PRIMITIVE"
    return "MEANING_DEPENDENT"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--jobs", type=Path, required=True)
    parser.add_argument("--candidates", type=Path, required=True)
    parser.add_argument("--gated", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    jobs = json.loads(args.jobs.read_text(encoding="utf-8"))
    candidates = json.loads(args.candidates.read_text(encoding="utf-8"))["rows"]
    gated = json.loads(args.gated.read_text(encoding="utf-8"))
    gate_by_prompt = {row["prompt_sha256"]: row for row in gated}
    job_by_id = {job["job_id"]: job for job in jobs["jobs"]}
    source_by_id = {job["source_id"]: job for job in jobs["jobs"]}
    controls = {}
    operations = []
    for row in candidates:
        if row["value"] == "PRESERVE_AS_IS":
            source_id = row.get("conditions", {}).get("source_id")
            controls[source_id] = row
        else:
            operations.append(row)

    observations: list[dict] = []
    per_operation: dict[str, list[dict]] = defaultdict(list)
    for row in operations:
        gate = gate_by_prompt.get(row["prompt_sha256"], {})
        job = job_by_id.get(row.get("observation_id"))
        source_id = row.get("conditions", {}).get("source_id")
        source = source_by_id.get(source_id)
        control = controls.get(source_id)
        if job is None or source is None or control is None:
            continue
        observation = {
            "operation": row["value"],
            "meaning_family_id": row["meaning_family_id"],
            "split": row["split"],
            "speech_act": job.get("speech_act"),
            "source_sha256": source["surface_sha256"],
            "approval": gate.get("approval", "MISSING_GATE"),
            "canonical_inverse": gate.get("canonical_inverse", False),
            "unsupported_fact_count": gate.get("unsupported_fact_count", 1),
            "teacher_returned_no_change": row.get("teacher_returned_no_change", False),
            "applicability": row.get("applicability"),
            "control_approval": gate_by_prompt.get(control["prompt_sha256"], {}).get("approval", "MISSING_GATE"),
            "operation_delta": delta_signature(source["surface"], row["surface"]),
            "control_delta": delta_signature(source["surface"], control["surface"]),
        }
        observations.append(observation)
        per_operation[observation["operation"]].append(observation)

    summaries = {}
    signatures_to_operations: dict[str, set[str]] = defaultdict(set)
    for operation in sorted(jobs["operation_readiness"]):
        items = per_operation.get(operation, [])
        approved = [item for item in items if item["approval"] == "APPROVED_STATE_CONTRAST_GOLD"]
        changed = [item for item in approved if item["operation_delta"] != ("IDENTICAL",)]
        sigs = Counter("|".join(item["operation_delta"]) for item in changed)
        for signature in sigs:
            signatures_to_operations[signature].add(operation)
        summaries[operation] = {
            "readiness": jobs["operation_readiness"][operation],
            "requested_count": len(items),
            "approved_count": len(approved),
            "rejected_count": len(items) - len(approved),
            "operation_changed_count": len(changed),
            "operation_change_rate": len(changed) / len(approved) if approved else None,
            "control_changed_count": sum(item["control_delta"] != ("IDENTICAL",) for item in approved),
            "control_change_rate": sum(item["control_delta"] != ("IDENTICAL",) for item in approved) / len(approved) if approved else None,
            "no_change_count": sum(item["teacher_returned_no_change"] for item in items),
            "signature_counts": dict(sigs),
            "classification": classify(operation, items, jobs["operation_readiness"], jobs["subject_omission"]),
        }
    summaries["SUBJECT_OMISSION"] = {
        "readiness": {"status": jobs["subject_omission"]["classification"], "blocker": jobs["subject_omission"]["blocker"]},
        "requested_count": 0,
        "classification": jobs["subject_omission"]["classification"],
    }
    aliases = {
        signature: sorted(operations)
        for signature, operations in signatures_to_operations.items()
        if signature != "IDENTICAL" and len(operations) > 1
    }
    artifact = {
        "schema": "BCORE.PRIMITIVE_SPECIFIC_REWRITE_CALIBRATION_REPORT.V1",
        "job_plan_sha256": hashlib.sha256(args.jobs.read_bytes()).hexdigest(),
        "candidate_sha256": hashlib.sha256(args.candidates.read_bytes()).hexdigest(),
        "gated_sha256": hashlib.sha256(args.gated.read_bytes()).hexdigest(),
        "surface_authority": "STRUCTURAL_DELTA_ONLY_NO_RAW_SURFACE_CACHE",
        "runtime_installation": "FORBIDDEN",
        "expression_primitive_promotion": "FORBIDDEN",
        "operation_summary": summaries,
        "cross_instruction_alias_candidates": aliases,
        "observations": observations,
        "unseen_meaning_application": "NOT_STARTED_UNTIL_STABLE_PRIMITIVE_CONFIRMED",
        "long_composition": "NOT_STARTED_UNTIL_STABLE_PRIMITIVE_CONFIRMED",
        "artifact_sha256": "",
    }
    artifact["artifact_sha256"] = hashlib.sha256(
        json.dumps(artifact, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(artifact, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "output": str(args.output),
        "classifications": {key: value["classification"] for key, value in summaries.items()},
        "observations": len(observations),
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
