"""Build provenance-preserving positives and controlled Korean hard negatives."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

from naturalness_verifier import PARTICLE_PAIRS, digest, final_hangul_jongseong, morphology_verdict


def row_from(source: dict, surface: str, kind: str, corruption: str | None, index: int) -> dict:
    ir = source["approved_response_ir"]
    return {
        "meaning_family_id": ir["semantic_sha256"],
        "split": source["split"],
        "factor": "NATURALNESS_VERIFICATION",
        "value": kind,
        "baseline_value": "TRUSTED_BCORE_ORDINARY_REALIZATION",
        "approved_response_ir": ir,
        "surface": surface,
        "teacher_model": "NONE_CONTROLLED_CORRUPTION",
        "prompt_sha256": digest({"family": ir["semantic_sha256"], "surface": surface, "kind": kind, "index": index}),
        "conditions": {
            "benchmark_kind": kind,
            "corruption_type": corruption or "NONE",
            "source_surface_sha256": hashlib.sha256(source["trusted_source_surface"].encode()).hexdigest(),
            "source_provenance": source["source_path"],
        },
    }


def particle_swap(surface: str) -> str | None:
    pairs = (("으로", "로"), ("로", "으로"), ("은", "는"), ("는", "은"),
             ("이", "가"), ("가", "이"), ("을", "를"), ("를", "을"))
    for left, right in pairs:
        matches = re.finditer(rf"([가-힣A-Za-z0-9]+?){left}(?=\s|[,.?!]|$)", surface)
        for match in matches:
            jong = final_hangul_jongseong(match.group(1))
            if jong is None:
                continue
            requires_jong, rieul_exception = PARTICLE_PAIRS[left]
            if requires_jong != (jong != 0 and not (rieul_exception and jong == 8)):
                continue
            start, end = match.span()
            token = match.group(0)
            return surface[:start] + token[:-len(left)] + right + surface[end:]
    return None


def ending_stack(surface: str) -> str | None:
    replacements = (("할게요", "할게요요"), ("하겠습니다", "하겠습니다요"),
                    ("해 주세요", "해 주세요요"), ("해 주십시오", "해 주십시오요"),
                    ("인가요", "인가요요"), ("입니까", "입니까요"))
    for left, right in replacements:
        if left in surface:
            return surface.replace(left, right, 1)
    return None


def honorific_break(surface: str) -> str | None:
    replacements = (("해 주십시오", "해 주십세요"), ("해 주세요", "해 주시세요"),
                    ("하십시오", "하십세요"), ("하세요", "하시세요"))
    for left, right in replacements:
        if left in surface:
            return surface.replace(left, right, 1)
    return None


def speech_act_ending_conflict(surface: str, speech_act: str) -> str | None:
    if speech_act == "QUERY":
        for ending in ("입니까", "인가요"):
            if ending in surface:
                return surface.replace(ending, "해 주세요", 1).replace("?", ".")
    if speech_act == "REQUEST":
        for ending in ("해 주십시오", "해 주세요"):
            if ending in surface:
                return surface.replace(ending, "입니까", 1).replace(".", "?")
    if speech_act == "PROMISE":
        for ending in ("하겠습니다", "할게요"):
            if ending in surface:
                return surface.replace(ending, "입니까", 1).replace(".", "?")
    return None


def connective_break(surface: str) -> str:
    return "그리고 그리고 " + surface


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--fresh", type=Path, required=True)
    ap.add_argument("--existing-transfers", type=Path)
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()
    fresh = json.loads(args.fresh.read_text(encoding="utf-8"))["rows"]
    rows = []
    positive_count = 0
    negative_counts: dict[str, int] = {}
    for index, source in enumerate(fresh):
        surface = source["trusted_source_surface"]
        if morphology_verdict(surface)["verdict"] == "PASS":
            rows.append(row_from(source, surface, "TRUSTED_POSITIVE", None, index))
            positive_count += 1
        corruptions = {
            "PARTICLE_ALLOMORPH": particle_swap(surface),
            "ENDING_STACK": ending_stack(surface),
            "HONORIFIC_MISMATCH": honorific_break(surface),
            "ENDING_SPEECH_ACT_CONFLICT": speech_act_ending_conflict(surface, source["approved_response_ir"]["speech_act"]),
            "MALFORMED_CONNECTIVE": connective_break(surface),
        }
        for name, corrupted in corruptions.items():
            if not corrupted or corrupted == surface:
                continue
            rows.append(row_from(source, corrupted, "HARD_NEGATIVE", name, index))
            negative_counts[name] = negative_counts.get(name, 0) + 1
    # Preserve the already observed boundary cases: the semantic inverse accepts
    # these because their propositions are intact, while Korean 조사 morphology
    # is invalid (for example, `15분로`, `확정로`).
    if args.existing_transfers and args.existing_transfers.exists():
        existing = json.loads(args.existing_transfers.read_text(encoding="utf-8"))["rows"]
        for index, source in enumerate(row for row in existing if row["value"] == "FORCED_REJECT_CONTROL"):
            copied = json.loads(json.dumps(source, ensure_ascii=False))
            copied["factor"] = "NATURALNESS_VERIFICATION"
            copied["value"] = "HARD_NEGATIVE"
            copied["conditions"]["benchmark_kind"] = "HARD_NEGATIVE"
            copied["conditions"]["corruption_type"] = "OBSERVED_PARTICLE_ALLOMORPH"
            copied["prompt_sha256"] = digest({
                "prior_prompt": source["prompt_sha256"], "benchmark": "OBSERVED_PARTICLE_ALLOMORPH", "index": index,
            })
            rows.append(copied)
            negative_counts["OBSERVED_PARTICLE_ALLOMORPH"] = negative_counts.get("OBSERVED_PARTICLE_ALLOMORPH", 0) + 1
    artifact = {
        "schema": "BCORE.NATURALNESS_VERIFICATION_BENCHMARK.V1",
        "runtime_installation": "FORBIDDEN",
        "positive_authority": "BCORE_ORDINARY_REALIZATION_WITH_CANONICAL_PROVENANCE_NOT_HUMAN_NATURALNESS_LABEL",
        "generated_program_candidates_used_as_positive": False,
        "trusted_positive_count": positive_count,
        "hard_negative_count": sum(negative_counts.values()),
        "hard_negative_counts": negative_counts,
        "rows": rows,
        "artifact_sha256": "",
    }
    artifact["artifact_sha256"] = digest(artifact)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(artifact, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"positive": positive_count, "negative": sum(negative_counts.values()), "types": negative_counts}, ensure_ascii=False))


if __name__ == "__main__":
    main()
