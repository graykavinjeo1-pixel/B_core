"""Summarize the speech-act state-relevance canary without storing surfaces."""
from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--coverage", type=Path, required=True)
    ap.add_argument("--gated", type=Path, required=True)
    ap.add_argument("--pairs", type=Path, required=False)
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()
    coverage = json.loads(args.coverage.read_text(encoding="utf-8"))
    gated = json.loads(args.gated.read_text(encoding="utf-8"))
    pairs = json.loads(args.pairs.read_text(encoding="utf-8")) if args.pairs else {"pairs": []}
    by_act = defaultdict(lambda: {"candidates": 0, "approved": 0, "rejected": 0})
    for row in gated:
        act = row["speech_act"]
        by_act[act]["candidates"] += 1
        by_act[act]["approved"] += row["approval"] == "APPROVED_STATE_CONTRAST_GOLD"
        by_act[act]["rejected"] += row["approval"] == "REJECTED"
    result = {
        "schema": "BCORE.STATE_RELEVANCE_BY_SPEECH_ACT_CANARY.V1",
        "coverage_schema": coverage["schema"],
        "coverage_suite_id": coverage["suite_id"],
        "coverage_row_count": coverage["row_count"],
        "coverage_speech_acts": [row["approved_response_ir"]["speech_act"] for row in coverage["rows"]],
        "candidate_count": len(gated),
        "approved_count": sum(item["approved"] for item in by_act.values()),
        "rejected_count": sum(item["rejected"] for item in by_act.values()),
        "speech_act_summary": dict(sorted(by_act.items())),
        "reject_reason_counts": dict(Counter(row["reject_reason"] for row in gated)),
        "semantic_authority": "B_CORE_CANONICAL_INVERSE",
        "teacher_surface_runtime_installation": "FORBIDDEN",
        "paired_contrast_count": len(pairs["pairs"]),
        "blind_pair_count": sum(row["split"] == "BLIND" for row in pairs["pairs"]),
        "blind_surface_change_count": sum(
            row["split"] == "BLIND" and row["surface_pair_changed"] for row in pairs["pairs"]
        ),
        "state_relevance_status": (
            "CANONICAL_COVERAGE_PARTIAL_NO_BLIND_EFFECT"
            if pairs["pairs"] and not any(
                row["split"] == "BLIND" and row["surface_pair_changed"] for row in pairs["pairs"]
            )
            else "CANONICAL_COVERAGE_READY_FOR_LARGER_CONTRAST"
        ),
        "controller_status": "RESEARCH_SHADOW_ONLY",
        "artifact_sha256": "",
    }
    result["artifact_sha256"] = hashlib.sha256(
        json.dumps(result, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"output": str(args.output), "artifact_sha256": result["artifact_sha256"]}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
