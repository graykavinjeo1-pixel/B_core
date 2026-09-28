"""Compare controller OFF/ON against a sealed gold-projection reference.

Arm C is intentionally an evaluation oracle: it is a feature-space reference
derived from train/validation gold and never produces runtime text or stores a
surface lookup template.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import defaultdict
from pathlib import Path

import numpy as np


MARKERS = {
    "directness": ("바로 말하면", "핵심은", "요점을 정리하면", "정리해 보면", "먼저 말하면"),
    "warmth": ("괜찮", "다행", "함께", "걱정", "안심"),
    "emotionality": ("아이구", "참", "정말", "좋아", "답답", "속상"),
    "hedge": ("아마", "것 같", "가능", "대체로", "아마도"),
    "discourse": ("먼저", "정리하면", "요점을", "참고로", "결론적으로", "그러니까"),
}
FEATURE_NAMES = ("chars", "tokens", "sentences", "newline", "markdown", *MARKERS,
                 "subject_front", "ending_FORMAL", "ending_POLITE", "ending_INFORMAL",
                 "ending_PLAIN", "ending_ELDER", "ending_OTHER")


def vector(text: str) -> np.ndarray:
    tail = text.rstrip()
    ending = "OTHER"
    for name, suffixes in {"FORMAL": ("습니다", "습니까"), "POLITE": ("어요", "아요", "해요"),
                           "INFORMAL": ("야", "어", "아", "해"), "PLAIN": ("다",),
                           "ELDER": ("구먼",)}.items():
        if any(tail.endswith(s + p) for s in suffixes for p in ("", ".", "!", "?")):
            ending = name
            break
    values = [len(text), len(text.split()), len(re.findall(r"[.!?。！？]", text)), int("\n" in text),
              int(any(m in text for m in ("#", "- ", "1.")))]
    values.extend(int(any(m in text for m in markers)) for markers in MARKERS.values())
    values.append(int(bool(re.search(r"^[^,，]+(?:이|가|은|는)", text))))
    values.extend(int(ending == name) for name in ("FORMAL", "POLITE", "INFORMAL", "PLAIN", "ELDER", "OTHER"))
    return np.asarray(values, dtype=np.float64)


def cosine(a: np.ndarray, b: np.ndarray) -> float:
    denom = float(np.linalg.norm(a) * np.linalg.norm(b))
    return float(np.dot(a, b) / denom) if denom else 1.0


def load_reports(paths: list[Path]) -> dict[str, dict]:
    return {json.loads(path.read_text(encoding="utf-8"))["source_response_sha256"]: json.loads(path.read_text(encoding="utf-8")) for path in paths}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--gold", type=Path, required=True)
    parser.add_argument("--off", nargs="+", type=Path, required=True)
    parser.add_argument("--on", nargs="+", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    gold = json.loads(args.gold.read_text(encoding="utf-8"))
    off = load_reports(args.off)
    on = load_reports(args.on)
    gold_rows = gold["rows"]
    oracle: dict[tuple[str, str], np.ndarray] = {}
    for factor, value in sorted({(row["factor"], row["value"]) for row in gold_rows}):
        rows = [row for row in gold_rows if row["factor"] == factor and row["value"] == value and row["source_split"] in ("TRAIN", "VALIDATION")]
        if rows:
            oracle[(factor, value)] = np.mean([vector(row["surface"]) - vector(row["baseline_surface"]) for row in rows], axis=0)

    metrics = []
    for row in gold_rows:
        if row["source_split"] != "BLIND":
            continue
        source_hash = row["meaning_family_id"]
        off_report = off.get(source_hash)
        on_report = on.get(source_hash)
        if not off_report or not on_report:
            continue
        def find(report: dict) -> dict | None:
            for factor in report.get("factors", []):
                if factor["factor"] != row["factor"]:
                    continue
                for variant in factor["variants"]:
                    if variant["value"] == row["value"]:
                        return variant["output"]
            return None
        off_output = find(off_report)
        on_output = find(on_report)
        if not off_output or not on_output:
            continue
        gold_delta = vector(row["surface"]) - vector(row["baseline_surface"])
        oracle_delta = oracle.get((row["factor"], row["value"]), np.zeros_like(gold_delta))
        metrics.append({
            "meaning_family_id": source_hash,
            "factor": row["factor"], "value": row["value"],
            "gold_surface_changed": row["surface_changed"],
            "arm_a_off_surface_changed": off_output["markdown"] != row["baseline_surface"],
            "arm_b_on_surface_changed": on_output["markdown"] != row["baseline_surface"],
            "arm_a_semantic_inverse": bool(off_output["semantic_inverse"]),
            "arm_b_semantic_inverse": bool(on_output["semantic_inverse"]),
            "arm_c_oracle_delta_norm": float(np.linalg.norm(oracle_delta)),
            "arm_a_gold_style_cosine": cosine(vector(off_output["markdown"]) - vector(row["baseline_surface"]), gold_delta),
            "arm_b_gold_style_cosine": cosine(vector(on_output["markdown"]) - vector(row["baseline_surface"]), gold_delta),
            "arm_b_oracle_style_cosine": cosine(vector(on_output["markdown"]) - vector(row["baseline_surface"]), oracle_delta),
        })

    def summary(key: str) -> dict:
        values = [row[key] for row in metrics]
        return {"n": len(values), "mean": float(np.mean(values)) if values else 0.0,
                "changed_rate": float(np.mean(values)) if values else 0.0}
    factor_summary = {}
    for factor in sorted({row["factor"] for row in metrics}):
        subset = [row for row in metrics if row["factor"] == factor]
        factor_summary[factor] = {
            "n": len(subset),
            "gold_changed_rate": float(np.mean([row["gold_surface_changed"] for row in subset])),
            "arm_a_off_changed_rate": float(np.mean([row["arm_a_off_surface_changed"] for row in subset])),
            "arm_b_on_changed_rate": float(np.mean([row["arm_b_on_surface_changed"] for row in subset])),
            "arm_a_gold_style_cosine": float(np.mean([row["arm_a_gold_style_cosine"] for row in subset])),
            "arm_b_gold_style_cosine": float(np.mean([row["arm_b_gold_style_cosine"] for row in subset])),
            "arm_b_oracle_style_cosine": float(np.mean([row["arm_b_oracle_style_cosine"] for row in subset])),
            "semantic_inverse_all": all(row["arm_a_semantic_inverse"] and row["arm_b_semantic_inverse"] for row in subset),
        }
    blind_composition = []
    for source_hash, report in off.items():
        if source_hash not in on:
            continue
        blind_composition.append({
            "meaning_family_id": source_hash,
            "arm_a_semantic_inverse": bool(report["blind_multi_factor_composition"]["output"]["semantic_inverse"]),
            "arm_b_semantic_inverse": bool(on[source_hash]["blind_multi_factor_composition"]["output"]["semantic_inverse"]),
            "arm_a_markdown": report["blind_multi_factor_composition"]["output"]["markdown"],
            "arm_b_markdown": on[source_hash]["blind_multi_factor_composition"]["output"]["markdown"],
        })
    all_variant_total = 0
    all_variant_changed = 0
    all_variant_inverse = True
    for source_hash, report in off.items():
        if source_hash not in on:
            continue
        other = on[source_hash]
        for left_factor, right_factor in zip(report.get("factors", []), other.get("factors", [])):
            for left_variant, right_variant in zip(left_factor.get("variants", []), right_factor.get("variants", [])):
                all_variant_total += 1
                all_variant_changed += int(left_variant["output"]["markdown"] != right_variant["output"]["markdown"])
                all_variant_inverse = all_variant_inverse and bool(right_variant["output"]["semantic_inverse"])
    result = {
        "schema": "BCORE.LANGUAGE_STATE_CONTROLLER_ARMS.V1",
        "arm_a": "CONTROLLER_OFF",
        "arm_b": "LEARNED_CONTROLLER_ON",
        "arm_c": "ORACLE_LANGUAGE_STATE_PROJECTION_REFERENCE_ONLY",
        "oracle_surface_authority": "EVALUATION_REFERENCE_NO_RUNTIME_OUTPUT",
        "sealed_blind_rows": len(metrics),
        "factor_summary": factor_summary,
        "blind_multi_factor_composition": blind_composition,
        "semantic_inverse_arm_a": all(row["arm_a_semantic_inverse"] for row in metrics),
        "semantic_inverse_arm_b": all(row["arm_b_semantic_inverse"] for row in metrics),
        "state_sensitive_surface_variation_arm_a": sum(row["arm_a_off_surface_changed"] for row in metrics),
        "state_sensitive_surface_variation_arm_b": sum(row["arm_b_on_surface_changed"] for row in metrics),
        "existing_405_runtime_baseline": {
            "variant_outputs": all_variant_total,
            "controller_changed_outputs": all_variant_changed,
            "controller_changed_rate": all_variant_changed / all_variant_total if all_variant_total else 0.0,
            "semantic_inverse_all": all_variant_inverse,
        },
        "metrics": metrics,
        "promotion": "RESEARCH_SHADOW_ONLY",
        "artifact_sha256": "",
    }
    result["artifact_sha256"] = hashlib.sha256(json.dumps(result, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"output": str(args.output), "sealed_blind_rows": len(metrics), "artifact_sha256": result["artifact_sha256"]}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
