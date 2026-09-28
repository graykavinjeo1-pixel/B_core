"""Analyze controlled one-factor language-state runtime contrasts.

The input reports contain one canonical meaning per split and explicit runtime
state variants.  This script measures feature movement and projects it onto the
sealed persona residual latent artifact.  It never creates or promotes an
operator and stores no lookup cache.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
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
FEATURE_NAMES = (
    "chars", "tokens", "sentences", "newline", "markdown", *MARKERS,
    "subject_front", *(f"ending_{ending}" for ending in ENDINGS),
)


def surface_vector(text: str) -> np.ndarray:
    tail = text.rstrip()
    ending = "OTHER"
    for name, suffixes in {
        "FORMAL": ("습니다", "습니까"),
        "POLITE": ("어요", "아요", "해요"),
        "INFORMAL": ("야", "어", "아", "해"),
        "PLAIN": ("다",),
        "ELDER": ("구먼",),
    }.items():
        if any(tail.endswith(suffix + punct) for suffix in suffixes for punct in ("", ".", "!", "?")):
            ending = name
            break
    values = [
        len(text), len(text.split()), len(re.findall(r"[.!?。！？]", text)),
        int("\n" in text), int(any(marker in text for marker in ("#", "- ", "1."))),
    ]
    values.extend(int(any(marker in text for marker in markers)) for markers in MARKERS.values())
    values.append(int(bool(re.search(r"^[^,，]+(?:이|가|은|는)", text))))
    values.extend(int(ending == name) for name in ENDINGS)
    return np.asarray(values, dtype=np.float64)


def cosine(a: np.ndarray, b: np.ndarray) -> float:
    denom = float(np.linalg.norm(a) * np.linalg.norm(b))
    return float(np.dot(a, b) / denom) if denom else 1.0


def analyze_report(report: dict, components: np.ndarray, verbosity_component: np.ndarray) -> dict:
    result = {"source_split": report["source_split"], "source_response_sha256": report["source_response_sha256"], "factors": []}
    for factor in report["factors"]:
        rows = factor["variants"]
        baseline = rows[0]
        base_text = baseline["output"]["markdown"]
        base_vec = surface_vector(base_text)
        effects = []
        for row in rows[1:]:
            text = row["output"]["markdown"]
            vec = surface_vector(text)
            delta = vec - base_vec
            residual_delta = delta.copy()
            for index, name in enumerate(FEATURE_NAMES):
                if name in {"chars", "tokens"}:
                    residual_delta[index] = 0.0
            projections = residual_delta @ components.T if components.size else np.zeros(0)
            effects.append({
                "value": row["value"],
                "surface_changed": text != base_text,
                "length_delta": int(delta[0]),
                "feature_delta": {name: float(delta[index]) for index, name in enumerate(FEATURE_NAMES) if delta[index] != 0},
                "existing_latent_projection_delta": [float(value) for value in projections],
                "verbosity_projection_delta": float(delta @ verbosity_component),
                "unexplained_residual_norm": float(np.linalg.norm(residual_delta - projections @ components)) if components.size else float(np.linalg.norm(residual_delta)),
                "semantic_inverse": bool(row["output"]["semantic_inverse"]),
            })
        result["factors"].append({
            "factor": factor["factor"],
            "note": factor["note"],
            "baseline": {"markdown": base_text, "semantic_inverse": bool(baseline["output"]["semantic_inverse"])},
            "effects": effects,
        })
    blind = report["blind_multi_factor_composition"]
    result["blind_multi_factor_composition"] = {
        "value": blind["value"],
        "markdown": blind["output"]["markdown"],
        "semantic_inverse": bool(blind["output"]["semantic_inverse"]),
        "features": {name: float(value) for name, value in zip(FEATURE_NAMES, surface_vector(blind["output"]["markdown"]))},
    }
    return result


def combine(reports: list[dict]) -> dict:
    by_factor = {}
    for report in reports:
        for factor in report["factors"]:
            by_factor.setdefault(factor["factor"], {}).setdefault(report["source_split"], []).append(factor)
    recurrence = []
    for name, split_rows in by_factor.items():
        values = sorted({effect["value"] for rows in split_rows.values() for row in rows for effect in row["effects"]})
        for value in values:
            effects = {
                split: [effect for row in rows for effect in row["effects"] if effect["value"] == value]
                for split, rows in split_rows.items()
            }
            effects = {split: rows for split, rows in effects.items() if rows}
            all_effects = [effect for rows in effects.values() for effect in rows]
            signs = []
            keys = sorted({key for effect in all_effects for key in effect["feature_delta"]})
            for key in keys:
                values_for_key = [np.sign(effect["feature_delta"].get(key, 0.0)) for effect in all_effects]
                nonzero = [value for value in values_for_key if value != 0]
                if nonzero:
                    majority = 1.0 if sum(value > 0 for value in nonzero) >= sum(value < 0 for value in nonzero) else -1.0
                    signs.append(sum(value == majority for value in nonzero) / len(nonzero))
            recurrence.append({
                "factor": name,
                "value": value,
                "splits": sorted(effects),
                "feature_direction_recurrence": float(np.mean(signs)) if signs else 1.0,
                "semantic_inverse_all": all(effect["semantic_inverse"] for effect in all_effects),
                "observations_per_split": {split: len(rows) for split, rows in effects.items()},
                "existing_latent_projection": {split: [effect["existing_latent_projection_delta"] for effect in rows] for split, rows in effects.items()},
                "unexplained_residual_norm": {split: [effect["unexplained_residual_norm"] for effect in rows] for split, rows in effects.items()},
            })
    return {"factor_value_recurrence": recurrence}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--reports", nargs="+", type=Path, required=True)
    parser.add_argument("--latent", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    reports = [json.loads(path.read_text(encoding="utf-8")) for path in args.reports]
    artifact = json.loads(args.latent.read_text(encoding="utf-8"))
    components = np.asarray(artifact["components"], dtype=np.float64)
    verbosity_component = np.asarray(artifact["residualization"]["verbosity_component"], dtype=np.float64)
    split_analysis = [analyze_report(report, components, verbosity_component) for report in reports]
    result = {
        "schema": "BCORE.MULTI_FACTOR_LANGUAGE_STATE_ANALYSIS.V1",
        "source_reports": [str(path) for path in args.reports],
        "latent_artifact_sha256": artifact["artifact_sha256"],
        "split_analysis": split_analysis,
        **combine(split_analysis),
        "promotion": "DISCOVERY_ONLY_NO_OPERATOR_PROMOTION",
        "new_residual_latent": "HOLD_UNTIL_MORE_THAN_ONE_MEANING_AND_BLIND_COMPOSITION",
        "surface_authority": "RUNTIME_CANARY_OUTPUTS_ONLY",
        "artifact_sha256": "",
    }
    canonical = json.dumps(result, ensure_ascii=False, separators=(",", ":")).encode()
    result["artifact_sha256"] = hashlib.sha256(canonical).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"output": str(args.output), "artifact_sha256": result["artifact_sha256"], "factors": len(result["factor_value_recurrence"])}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
