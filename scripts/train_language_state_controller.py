"""Train a compact state-to-[V,L1..L4] controller from controlled contrasts.

Only train meaning families are used to estimate factor/value deltas.  The
validation and blind reports are evaluation-only and never select parameters.
No surface text is stored in the controller artifact.
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
from pathlib import Path

import numpy as np


def load_rows(paths: list[Path], analysis_path: Path) -> tuple[dict, dict[str, list[np.ndarray]]]:
    analysis = json.loads(analysis_path.read_text(encoding="utf-8"))
    # split_analysis is aligned with source_reports in the analysis artifact.
    rows: dict[str, list[np.ndarray]] = collections.defaultdict(list)
    for split in analysis["split_analysis"]:
        for factor in split["factors"]:
            for effect in factor["effects"]:
                target = np.asarray(
                    [effect["verbosity_projection_delta"], *effect["existing_latent_projection_delta"]],
                    dtype=np.float64,
                )
                rows[f"{factor['factor']}\t{effect['value']}"] .append(target)
    return analysis, rows


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--analysis", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    analysis = json.loads(args.analysis.read_text(encoding="utf-8"))
    train_rows: dict[str, list[np.ndarray]] = collections.defaultdict(list)
    eval_rows: dict[str, dict[str, list[np.ndarray]]] = collections.defaultdict(lambda: collections.defaultdict(list))
    for split in analysis["split_analysis"]:
        for factor in split["factors"]:
            for effect in factor["effects"]:
                key = f"{factor['factor']}\t{effect['value']}"
                target = np.asarray(
                    [effect["verbosity_projection_delta"], *effect["existing_latent_projection_delta"]],
                    dtype=np.float64,
                )
                if split["source_split"] == "TRAIN":
                    train_rows[key].append(target)
                else:
                    eval_rows[split["source_split"]][key].append(target)

    scales = np.asarray([1.0, 1_000.0, 1_000.0, 1_000.0, 1_000.0], dtype=np.float64)
    entries = []
    train_predictions = {}
    for key in sorted(train_rows):
        factor, value = key.split("\t", 1)
        mean_target = np.mean(np.stack(train_rows[key]), axis=0)
        runtime_delta = np.rint(mean_target * scales).astype(np.int64).clip(-1000, 1000)
        entries.append({
            "factor": factor,
            "value": value,
            "delta_millis": [int(item) for item in runtime_delta],
        })
        train_predictions[key] = runtime_delta / scales

    def metrics(split_name: str, rows: dict[str, list[np.ndarray]]) -> dict[str, float]:
        observations = 0
        errors = []
        sign_hits = []
        inverse_failures = 0
        for key, targets in rows.items():
            if key not in train_predictions:
                continue
            prediction = train_predictions[key]
            for target in targets:
                observations += 1
                errors.append(float(np.mean((prediction - target) ** 2)))
                sign_hits.extend(
                    float(np.sign(prediction[index]) == np.sign(target[index]))
                    for index in range(len(prediction))
                    if target[index] != 0
                )
        return {
            "observations": float(observations),
            "mean_squared_error": float(np.mean(errors)) if errors else 0.0,
            "directional_agreement": float(np.mean(sign_hits)) if sign_hits else 1.0,
            "semantic_inverse_failures": float(inverse_failures),
        }

    train_metrics = metrics("TRAIN", train_rows)
    validation_metrics = metrics("VALIDATION", eval_rows["VALIDATION"])
    blind_metrics = metrics("BLIND", eval_rows["BLIND"])
    artifact = {
        "schema": "BCORE.LANGUAGE_STATE_CONTROLLER.V1",
        "basis": ["V", "L1", "L2", "L3", "L4"],
        "basis_scales": scales.tolist(),
        "entries": entries,
        "train_observation_count": sum(len(items) for items in train_rows.values()),
        "validation_metrics": validation_metrics,
        "blind_metrics": blind_metrics,
        "promotion": "RESEARCH_SHADOW_ONLY",
        "surface_authority": "COMPACT_STATE_DELTAS_ONLY_NO_SURFACE_CACHE",
        "artifact_sha256": "",
    }
    canonical = json.dumps(artifact, ensure_ascii=False, separators=(",", ":")).encode()
    artifact["artifact_sha256"] = hashlib.sha256(canonical).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(artifact, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "output": str(args.output),
        "entry_count": len(entries),
        "train_metrics": train_metrics,
        "validation_metrics": validation_metrics,
        "blind_metrics": blind_metrics,
        "artifact_sha256": artifact["artifact_sha256"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
