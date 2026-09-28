"""Discover corpus-supported persona dimensions without making MBTI a runtime axis.

Each 17-way family is centered before analysis, removing meaning-dependent
surface effects.  The artifact contains numeric residual operators only; it
never stores a sentence or a family lookup cache.
"""

from __future__ import annotations

import argparse
import collections
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


def vector(text: str) -> np.ndarray:
    ending = "OTHER"
    for name, suffix in (("FORMAL", "습니다"), ("POLITE", "어요"), ("INFORMAL", "야"), ("PLAIN", "다"), ("ELDER", "구먼")):
        if text.rstrip().endswith((suffix, suffix + ".")):
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


def cosine(left: np.ndarray, right: np.ndarray) -> float:
    denominator = float(np.linalg.norm(left) * np.linalg.norm(right))
    return float(np.dot(left, right) / denominator) if denominator else 0.0


def discover(source: Path, grounded: Path | None = None) -> dict:
    families: dict[str, dict[str, tuple[str, np.ndarray, str]]] = collections.defaultdict(dict)
    example_to_family: dict[str, str] = {}
    grounded_families: set[str] | None = None
    with source.open(encoding="utf-8") as stream:
        for line in stream:
            record = json.loads(line)
            label = record.get("persona", {}).get("persona_label")
            family = record.get("parallel_family_id")
            if label and family and record.get("surface"):
                example_to_family[record.get("example_id", "")] = family
                families[family][label] = (record.get("split", "TRAIN"), vector(record["surface"]), record.get("record_sha256", ""))
    if grounded is not None:
        grounded_families = set()
        with grounded.open(encoding="utf-8") as stream:
            for line in stream:
                record = json.loads(line)
                family = example_to_family.get(record.get("source_example_id", ""))
                if family in families:
                    grounded_families.add(family)
                    split = record["canonical_record"]["split"]
                    families[family] = {
                        label: (split, item[1], item[2]) for label, item in families[family].items()
                    }
    complete = {
        family: rows
        for family, rows in families.items()
        if len(rows) == 17 and (grounded_families is None or family in grounded_families)
    }
    labels = sorted(next(iter(complete.values())).keys())
    split_families = {
        split: {family: rows for family, rows in complete.items() if next(iter(rows.values()))[0] == split}
        for split in ("TRAIN", "VALIDATION", "BLIND")
    }

    def family_centered_matrix(family_rows: dict[str, tuple[str, np.ndarray, str]]) -> np.ndarray:
        matrix = np.stack([family_rows[label][1] for label in labels])
        return matrix - matrix.mean(axis=0, keepdims=True)

    base_stack = np.stack([family_centered_matrix(rows) for rows in split_families["TRAIN"].values()])
    base_profile = base_stack.mean(axis=0)
    base_centered = base_profile - base_profile.mean(axis=0, keepdims=True)
    _, base_singular, base_vt = np.linalg.svd(base_centered, full_matrices=False)
    verbosity_component = base_vt[0]
    verbosity_indices = [FEATURE_NAMES.index(name) for name in ("chars", "tokens")]

    def residualize(family_rows: dict[str, tuple[str, np.ndarray, str]]) -> np.ndarray:
        residual = family_centered_matrix(family_rows)
        projection = residual @ verbosity_component
        residual = residual - projection[:, None] * verbosity_component[None, :]
        residual[:, verbosity_indices] = 0.0
        return residual

    train_stack = np.stack([residualize(rows) for rows in split_families["TRAIN"].values()])
    train_profile = train_stack.mean(axis=0)
    centered = train_profile - train_profile.mean(axis=0, keepdims=True)
    u, singular, vt = np.linalg.svd(centered, full_matrices=False)
    variance = singular**2
    explained = variance / variance.sum() if variance.sum() else variance
    cumulative = np.cumsum(explained)
    latent_count = int(np.searchsorted(cumulative, 0.8) + 1) if len(cumulative) else 0
    latent_count = max(1, min(latent_count, len(singular))) if len(singular) else 0
    components = vt[:latent_count]
    train_latent = centered @ components.T

    split_report = {}
    for split, split_rows in split_families.items():
        vectors = []
        profile_cosines = []
        nearest = 0
        total = 0
        for rows in split_rows.values():
            latent = residualize(rows) @ components.T
            vectors.append(latent)
            for index, label in enumerate(labels):
                profile_cosines.append(cosine(latent[index], train_latent[index]))
                nearest_label = min(range(len(labels)), key=lambda candidate: float(np.linalg.norm(latent[index] - train_latent[candidate])))
                nearest += nearest_label == index
                total += 1
        if vectors:
            observed = np.concatenate(vectors, axis=0)
            consistency = float(np.mean(profile_cosines))
            recurrence = float(np.mean(np.asarray(profile_cosines) >= 0.9))
            split_profile = np.stack(vectors).mean(axis=0)
            split_profile_latent = split_profile - split_profile.mean(axis=0, keepdims=True)
            latent_stability = cosine(split_profile_latent.ravel(), train_latent.ravel())
        else:
            consistency = 0.0
            recurrence = 0.0
            latent_stability = 0.0
        split_report[split] = {
            "family_count": len(split_rows),
            "nearest_centroid_accuracy": nearest / total if total else 0.0,
            "heldout_profile_cosine": consistency,
            "family_independent_recurrence": recurrence,
            "latent_stability": latent_stability,
        }

    artifact = {
        "schema": "BCORE.PERSONA_LATENT_OPERATOR_DISCOVERY.V1",
        "source_path": str(source),
        "feature_names": list(FEATURE_NAMES),
        "persona_labels": labels,
        "family_counts": {split: len(rows) for split, rows in split_families.items()},
        "training_unit": "FAMILY_CENTERED_17_WAY_RESIDUAL",
        "residualization": {
            "removed_candidate": "LATENT_01_VERBOSITY_CANDIDATE",
            "removed_features": ["chars", "tokens"],
            "verbosity_component": verbosity_component.tolist(),
            "verbosity_singular_value": float(base_singular[0]),
        },
        "latent_count_80pct_variance": latent_count,
        "singular_values": singular.tolist(),
        "explained_variance": explained.tolist(),
        "cumulative_explained_variance": cumulative.tolist(),
        "components": components.tolist(),
        "train_persona_latent_centroids": train_latent.tolist(),
        "split_report": split_report,
        "surface_authority": "NONAUTHORITATIVE_ANALYSIS_ONLY",
        "promotion_status": "HOLD_UNTIL_HELDOUT_REPRODUCIBILITY_AND_SEPARABILITY_PASS",
        "artifact_sha256": "",
    }
    canonical = json.dumps({**artifact, "artifact_sha256": ""}, ensure_ascii=False, separators=(",", ":")).encode()
    artifact["artifact_sha256"] = hashlib.sha256(canonical).hexdigest()
    return artifact


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--grounded", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    artifact = discover(args.source, args.grounded)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(artifact, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "output": str(args.output),
        "family_counts": artifact["family_counts"],
        "latent_count_80pct_variance": artifact["latent_count_80pct_variance"],
        "split_report": artifact["split_report"],
        "artifact_sha256": artifact["artifact_sha256"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
