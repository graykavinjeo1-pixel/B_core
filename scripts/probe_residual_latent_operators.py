"""Probe residual latent effects without assigning semantic names or caching text."""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
from pathlib import Path

import numpy as np

from discover_persona_latent_operators import FEATURE_NAMES, vector


def cosine(left: np.ndarray, right: np.ndarray) -> float:
    denominator = float(np.linalg.norm(left) * np.linalg.norm(right))
    return float(np.dot(left, right) / denominator) if denominator else 0.0


def probe(source: Path, grounded: Path, artifact_path: Path) -> dict:
    artifact = json.loads(artifact_path.read_text(encoding="utf-8"))
    components = np.asarray(artifact["components"], dtype=np.float64)
    verbosity_component = np.asarray(artifact["residualization"]["verbosity_component"], dtype=np.float64)
    remove_indices = [FEATURE_NAMES.index(name) for name in ("chars", "tokens")]
    families: dict[str, dict[str, tuple[str, np.ndarray]]] = collections.defaultdict(dict)
    example_to_family: dict[str, str] = {}
    with source.open(encoding="utf-8") as stream:
        for line in stream:
            record = json.loads(line)
            family = record.get("parallel_family_id")
            label = record.get("persona", {}).get("persona_label")
            if family and label and record.get("surface"):
                example_to_family[record["example_id"]] = family
                families[family][label] = (record.get("split", "TRAIN"), vector(record["surface"]))
    grounded_families: set[str] = set()
    with grounded.open(encoding="utf-8") as stream:
        for line in stream:
            record = json.loads(line)
            family = example_to_family.get(record.get("source_example_id", ""))
            if family in families:
                grounded_families.add(family)
                split = record["canonical_record"]["split"]
                families[family] = {label: (split, item[1]) for label, item in families[family].items()}
    families = {family: rows for family, rows in families.items() if family in grounded_families and len(rows) == 17}
    labels = sorted(next(iter(families.values())).keys())
    split_families = {
        split: {family: rows for family, rows in families.items() if next(iter(rows.values()))[0] == split}
        for split in ("TRAIN", "VALIDATION", "BLIND")
    }

    def residual(rows: dict[str, tuple[str, np.ndarray]]) -> np.ndarray:
        matrix = np.stack([rows[label][1] for label in labels])
        matrix = matrix - matrix.mean(axis=0, keepdims=True)
        matrix = matrix - (matrix @ verbosity_component)[:, None] * verbosity_component[None, :]
        matrix[:, remove_indices] = 0.0
        return matrix

    values = []
    for split, split_rows in split_families.items():
        split_latent = []
        split_feature = []
        for rows in split_rows.values():
            matrix = residual(rows)
            split_latent.append(matrix @ components.T)
            split_feature.append(matrix)
        latent = np.concatenate(split_latent, axis=0)
        features = np.concatenate(split_feature, axis=0)
        latent_results = []
        for latent_index in range(components.shape[0]):
            score = latent[:, latent_index]
            feature_effects = []
            for feature_index, feature_name in enumerate(FEATURE_NAMES):
                feature = features[:, feature_index]
                if np.std(score) == 0 or np.std(feature) == 0:
                    correlation = 0.0
                else:
                    correlation = float(np.corrcoef(score, feature)[0, 1])
                feature_effects.append({
                    "feature": feature_name,
                    "correlation": correlation,
                    "delta_at_minus_1": float(-correlation * np.std(feature)),
                    "delta_at_plus_1": float(correlation * np.std(feature)),
                })
            feature_effects.sort(key=lambda item: abs(item["correlation"]), reverse=True)
            latent_results.append({
                "latent_index": latent_index + 1,
                "intervention_values": [-1.0, -0.5, 0.0, 0.5, 1.0],
                "dominant_effects": feature_effects[:8],
                "composite_candidate": len([item for item in feature_effects if abs(item["correlation"]) >= 0.25]) > 1,
                "semantic_inverse": "UNCHANGED_CANONICAL_IR",
            })
        values.append((split, latent_results))

    report = {
        "schema": "BCORE.RESIDUAL_LATENT_CAUSAL_PROBE.V1",
        "source_path": str(source),
        "grounded_path": str(grounded),
        "latent_artifact_sha256": artifact["artifact_sha256"],
        "family_counts": {split: len(rows) for split, rows in split_families.items()},
        "intervention_values": [-1.0, -0.5, 0.0, 0.5, 1.0],
        "split_results": {split: result for split, result in values},
        "surface_authority": "NONAUTHORITATIVE_FEATURE_PROBE; NO_SENTENCE_CACHE",
        "runtime_promotion": "HOLD_UNTIL_RUNTIME_SURFACE_CAUSAL_VALIDATION",
        "artifact_sha256": "",
    }
    canonical = json.dumps({**report, "artifact_sha256": ""}, ensure_ascii=False, separators=(",", ":")).encode()
    report["artifact_sha256"] = hashlib.sha256(canonical).hexdigest()
    return report


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--grounded", type=Path, required=True)
    parser.add_argument("--artifact", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = probe(args.source, args.grounded, args.artifact)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "output": str(args.output),
        "family_counts": report["family_counts"],
        "artifact_sha256": report["artifact_sha256"],
        "runtime_promotion": report["runtime_promotion"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
