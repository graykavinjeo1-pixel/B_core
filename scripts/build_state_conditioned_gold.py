"""Build research-only state-conditioned parallel gold from canonical runtime reports.

The stored surfaces are evidence for analysis and are explicitly forbidden as
runtime lookup templates.  Every row remains tied to the Approved Response IR
hash and the source runtime report.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


PRIORITY = (
    "relationship_social_distance",
    "inferred_user_emotion",
    "dialogue_context",
    "age_register_condition",
    "register_condition",
    "region_dialect_condition",
)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--reports", nargs="+", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    rows: list[dict] = []
    split_counts: dict[str, int] = {}
    for report_path in args.reports:
        report = json.loads(report_path.read_text(encoding="utf-8"))
        split = str(report["source_split"])
        split_counts[split] = split_counts.get(split, 0) + 1
        for factor in report.get("factors", []):
            if factor["factor"] not in PRIORITY:
                continue
            variants = factor["variants"]
            baseline = variants[0]
            for variant in variants[1:]:
                output = variant["output"]
                rows.append(
                    {
                        "meaning_family_id": report["source_response_sha256"],
                        "source_split": split,
                        "factor": factor["factor"],
                        "value": variant["value"],
                        "baseline_value": baseline["value"],
                        "baseline_surface": baseline["output"]["markdown"],
                        "surface": output["markdown"],
                        "surface_changed": output["markdown"] != baseline["output"]["markdown"],
                        "semantic_inverse": bool(output["semantic_inverse"]),
                        "baseline_semantic_inverse": bool(baseline["output"]["semantic_inverse"]),
                        "construction_marker": output.get("marker"),
                        "source_report": str(report_path),
                        "source_note": factor.get("note", ""),
                    }
                )

    accepted = sum(row["semantic_inverse"] and row["baseline_semantic_inverse"] for row in rows)
    result = {
        "schema": "BCORE.STATE_CONDITIONED_GOLD_LANGUAGE_CONTRAST.V1",
        "purpose": "CONTROLLED_PARALLEL_GOLD_RESEARCH_ONLY",
        "meaning_authority": "APPROVED_CANONICAL_RESPONSE_IR_SHA256",
        "surface_authority": "EVIDENCE_ONLY_NO_RUNTIME_TEMPLATE_OR_LOOKUP_CACHE",
        "one_factor_at_a_time": True,
        "priority_factors": list(PRIORITY),
        "source_report_count": len(args.reports),
        "source_split_counts": split_counts,
        "row_count": len(rows),
        "semantic_inverse_pass_count": accepted,
        "semantic_inverse_pass_rate": accepted / len(rows) if rows else 1.0,
        "rows": rows,
        "artifact_sha256": "",
    }
    canonical = json.dumps(result, ensure_ascii=False, separators=(",", ":")).encode()
    result["artifact_sha256"] = hashlib.sha256(canonical).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({k: result[k] for k in ("output", "row_count", "semantic_inverse_pass_count", "artifact_sha256") if k in result} | {"output": str(args.output)}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
