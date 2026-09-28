"""Analyze state-conditioned gold without promoting a runtime operator."""
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
ENDINGS = ("FORMAL", "POLITE", "INFORMAL", "PLAIN", "ELDER", "OTHER")
FEATURE_NAMES = ("chars", "tokens", "sentences", "newline", "markdown", *MARKERS,
                 "subject_front", *(f"ending_{ending}" for ending in ENDINGS))


def surface_vector(text: str) -> np.ndarray:
    tail = text.rstrip()
    ending = "OTHER"
    for name, suffixes in {"FORMAL": ("습니다", "습니까"), "POLITE": ("어요", "아요", "해요"),
                           "INFORMAL": ("야", "어", "아", "해"), "PLAIN": ("다",),
                           "ELDER": ("구먼",)}.items():
        if any(tail.endswith(suffix + punct) for suffix in suffixes for punct in ("", ".", "!", "?")):
            ending = name
            break
    values = [len(text), len(text.split()), len(re.findall(r"[.!?。！？]", text)), int("\n" in text),
              int(any(marker in text for marker in ("#", "- ", "1.")))]
    values.extend(int(any(marker in text for marker in markers)) for markers in MARKERS.values())
    values.append(int(bool(re.search(r"^[^,，]+(?:이|가|은|는)", text))))
    values.extend(int(ending == name) for name in ENDINGS)
    return np.asarray(values, dtype=np.float64)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--gold", type=Path, required=True)
    parser.add_argument("--latent", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    gold = json.loads(args.gold.read_text(encoding="utf-8"))
    latent = json.loads(args.latent.read_text(encoding="utf-8"))
    components = np.asarray(latent["components"], dtype=np.float64)
    verbosity = np.asarray(latent["residualization"]["verbosity_component"], dtype=np.float64)
    grouped: dict[tuple[str, str, str], list[dict]] = defaultdict(list)
    rows = gold["rows"]
    for row in rows:
        grouped[(row["factor"], row["value"], row["source_split"])].append(row)
    recurrence = []
    for factor, value in sorted({(r["factor"], r["value"]) for r in rows}):
        split_metrics = {}
        for split in ("TRAIN", "VALIDATION", "BLIND"):
            items = grouped[(factor, value, split)]
            if not items:
                continue
            explained = []
            residuals = []
            changed = 0
            for row in items:
                delta = surface_vector(row["surface"]) - surface_vector(row["baseline_surface"])
                residual = delta.copy()
                residual[:2] = 0.0
                projection = (residual @ components.T) @ components if components.size else np.zeros_like(residual)
                denom = float(np.dot(residual, residual))
                explained.append(float(np.dot(projection, projection) / denom) if denom else 1.0)
                residuals.append(float(np.linalg.norm(residual - projection)))
                changed += int(row["surface_changed"])
            split_metrics[split] = {
                "observations": len(items),
                "surface_changed": changed,
                "surface_change_rate": changed / len(items),
                "existing_basis_explained_mean": float(np.mean(explained)),
                "unexplained_residual_norm_mean": float(np.mean(residuals)),
                "semantic_inverse_all": all(row["semantic_inverse"] and row["baseline_semantic_inverse"] for row in items),
                "construction_markers": sorted({row["construction_marker"] for row in items}),
            }
        recurrence.append({"factor": factor, "value": value, "splits": split_metrics,
                           "family_independent_recurrence": len(split_metrics) == 3 and
                           len({round(split_metrics[s]["surface_change_rate"], 6) for s in split_metrics}) >= 1})
    by_factor = defaultdict(list)
    for item in recurrence:
        by_factor[item["factor"]].append(item)
    factor_summary = {}
    for factor, items in by_factor.items():
        all_splits = [v for item in items for v in item["splits"].values()]
        factor_summary[factor] = {
            "values": len(items),
            "observations": sum(v["observations"] for v in all_splits),
            "surface_change_rate": float(np.mean([v["surface_change_rate"] for v in all_splits])),
            "basis_explained_mean": float(np.mean([v["existing_basis_explained_mean"] for v in all_splits])),
            "unexplained_residual_mean": float(np.mean([v["unexplained_residual_norm_mean"] for v in all_splits])),
            "semantic_inverse_all": all(v["semantic_inverse_all"] for v in all_splits),
            "construction_reuse_markers": sorted({m for v in all_splits for m in v["construction_markers"]}),
        }
    result = {
        "schema": "BCORE.STATE_CONDITIONED_GOLD_LANGUAGE_CONTRAST_ANALYSIS.V1",
        "gold_artifact_sha256": gold["artifact_sha256"],
        "latent_artifact_sha256": latent["artifact_sha256"],
        "factor_summary": factor_summary,
        "factor_value_recurrence": recurrence,
        "promotion": "RESEARCH_SHADOW_ONLY",
        "new_latent": "HOLD_UNTIL_STABLE_RESIDUAL_CAUSAL_BLIND",
        "surface_authority": "RESEARCH_GOLD_ONLY_NO_RUNTIME_TEMPLATE",
        "artifact_sha256": "",
    }
    result["artifact_sha256"] = hashlib.sha256(json.dumps(result, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"output": str(args.output), "artifact_sha256": result["artifact_sha256"], "factors": len(factor_summary)}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
