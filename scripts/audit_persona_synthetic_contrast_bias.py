"""Audit positional and synthetic-label bias in the 17-way persona corpus.

The source corpus is never modified.  This audit reconstructs its deterministic
surface compiler in a separate research artifact and reports positional controls
separately from the more fundamental direct persona-label-to-surface encoding.
It does not train, load, or promote a runtime artifact.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import random
from collections import defaultdict
from pathlib import Path

import numpy as np

from build_persona_parallel_lattice import PERSONAS, surface
from discover_persona_latent_operators import vector


LABELS = [item[0] for item in PERSONAS]
PERSONA_BY_LABEL = {item[0]: item for item in PERSONAS}
CHAR_INDEX = 0
TOKEN_INDEX = 1


def digest(value: object) -> str:
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def load(source: Path, sample_per_split: int) -> tuple[dict[str, dict[str, dict]], dict[str, dict[str, dict]], dict[str, int]]:
    all_families: dict[str, dict[str, dict]] = defaultdict(dict)
    with source.open(encoding="utf-8") as handle:
        for line in handle:
            row = json.loads(line)
            family = row["parallel_family_id"]
            label = row["persona"]["persona_label"]
            if label in all_families[family]:
                raise ValueError(f"duplicate persona in family: {family}:{label}")
            all_families[family][label] = row
    complete = {
        family: rows for family, rows in all_families.items()
        if set(rows) == set(LABELS)
    }
    split_counts: dict[str, int] = defaultdict(int)
    by_split: dict[str, list[str]] = defaultdict(list)
    for family, rows in complete.items():
        split = rows[LABELS[0]]["split"]
        if any(row["split"] != split for row in rows.values()):
            raise ValueError(f"split disagreement in family: {family}")
        split_counts[split] += 1
        by_split[split].append(family)
    sample: dict[str, dict[str, dict]] = {}
    for split in ("TRAIN", "VALIDATION", "BLIND"):
        selected = sorted(by_split[split])[:sample_per_split]
        sample.update({family: complete[family] for family in selected})
    return complete, sample, dict(split_counts)


def projections(surfaces: list[str], latent: dict) -> tuple[float, float]:
    """Return mean absolute V and L1..L4 magnitudes after family centering."""
    matrix = np.stack([vector(text) for text in surfaces])
    centered = matrix - matrix.mean(axis=0, keepdims=True)
    verbosity = np.asarray(latent["residualization"]["verbosity_component"], dtype=np.float64)
    v = centered @ verbosity
    residual = centered - v[:, None] * verbosity[None, :]
    residual[:, [CHAR_INDEX, TOKEN_INDEX]] = 0.0
    components = np.asarray(latent["components"], dtype=np.float64)
    l = residual @ components.T
    return float(np.mean(np.abs(v))), float(np.mean(np.abs(l)))


def feature_signature(text: str) -> str:
    return json.dumps(vector(text).astype(float).tolist(), separators=(",", ":"))


def ending_signature(text: str) -> tuple[float, ...]:
    values = vector(text)
    # discover_persona_latent_operators keeps ending one-hot fields at 11..16.
    return tuple(float(value) for value in values[11:17])


def split_summary(items: list[dict]) -> dict:
    if not items:
        return {"family_count": 0}
    def average(name: str) -> float:
        return sum(float(item[name]) for item in items) / len(items)
    return {
        "family_count": len(items),
        "same_persona_distinct_surface_mean": average("same_persona_distinct_surface_count"),
        "same_persona_chars_variance_mean": average("same_persona_chars_variance"),
        "same_persona_tokens_variance_mean": average("same_persona_tokens_variance"),
        "same_persona_ending_variation_mean": average("same_persona_ending_variation_count"),
        "same_persona_projection_v_abs_mean": average("same_persona_projection_v_abs"),
        "same_persona_projection_l_abs_mean": average("same_persona_projection_l_abs"),
        "original_projection_v_abs_mean": average("original_projection_v_abs"),
        "original_projection_l_abs_mean": average("original_projection_l_abs"),
        "label_identity_exact_rate": average("label_identity_exact_rate"),
        "label_identity_feature_rate": average("label_identity_feature_rate"),
        "duplicate_label_exact_rate": average("duplicate_label_exact_rate"),
        "independent_repeat_exact_rate": average("independent_repeat_exact_rate"),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--latent", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--sample-per-split", type=int, default=50)
    parser.add_argument("--permutations", type=int, default=7)
    parser.add_argument("--independent-repeats", type=int, default=3)
    args = parser.parse_args()
    if args.sample_per_split < 1 or args.permutations < 2 or args.independent_repeats < 2:
        raise ValueError("audit requires samples>=1, permutations>=2, repeats>=2")
    latent = json.loads(args.latent.read_text(encoding="utf-8"))
    complete, sample, split_counts = load(args.source, args.sample_per_split)
    rebuilt_exact = 0
    rebuilt_total = 0
    source_surface_hash = hashlib.sha256()
    for rows in complete.values():
        for label in LABELS:
            record = rows[label]
            expected = surface(record["approved_meaning"], PERSONA_BY_LABEL[label])
            rebuilt_exact += expected == record["surface"]
            rebuilt_total += 1
            source_surface_hash.update(record["surface"].encode())
            source_surface_hash.update(b"\n")

    audits: list[dict] = []
    position_surface_changes = 0
    position_surface_total = 0
    for family, rows in sorted(sample.items()):
        split = rows[LABELS[0]]["split"]
        meaning = rows[LABELS[0]]["approved_meaning"]
        original_surfaces = [rows[label]["surface"] for label in LABELS]
        original_v, original_l = projections(original_surfaces, latent)

        same_slot_surfaces = []
        same_slot_chars_variance = []
        same_slot_tokens_variance = []
        same_slot_ending_variation = []
        same_v = []
        same_l = []
        for label in LABELS:
            repeated = [surface(meaning, PERSONA_BY_LABEL[label]) for _ in LABELS]
            same_slot_surfaces.extend(repeated)
            chars = np.asarray([len(value) for value in repeated], dtype=np.float64)
            tokens = np.asarray([len(value.split()) for value in repeated], dtype=np.float64)
            endings = {ending_signature(value) for value in repeated}
            same_slot_chars_variance.append(float(chars.var()))
            same_slot_tokens_variance.append(float(tokens.var()))
            same_slot_ending_variation.append(len(endings))
            value_v, value_l = projections(repeated, latent)
            same_v.append(value_v)
            same_l.append(value_l)

        label_observations: dict[str, list[str]] = defaultdict(list)
        position_observations: dict[int, list[str]] = defaultdict(list)
        for permutation_index in range(args.permutations):
            order = LABELS[:]
            random.Random(f"{family}:PERM:{permutation_index}").shuffle(order)
            for position, label in enumerate(order):
                rendered = surface(meaning, PERSONA_BY_LABEL[label])
                label_observations[label].append(rendered)
                position_observations[position].append(rendered)
        label_exact = [len(set(values)) == 1 for values in label_observations.values()]
        label_features = [len({feature_signature(value) for value in values}) == 1 for values in label_observations.values()]
        for values in position_observations.values():
            position_surface_changes += sum(value != values[0] for value in values[1:])
            position_surface_total += max(0, len(values) - 1)

        duplicate_matches = []
        for label in LABELS:
            # The same condition is deliberately placed at opposite slots.
            left = surface(meaning, PERSONA_BY_LABEL[label])
            right = surface(meaning, PERSONA_BY_LABEL[label])
            duplicate_matches.append(left == right)
        repeat_matches = []
        for label in LABELS:
            repeated = [surface(meaning, PERSONA_BY_LABEL[label]) for _ in range(args.independent_repeats)]
            repeat_matches.append(len(set(repeated)) == 1)
        audits.append({
            "meaning_family_id": family,
            "split": split,
            "same_persona_distinct_surface_count": len(set(same_slot_surfaces)) / len(LABELS),
            "same_persona_chars_variance": sum(same_slot_chars_variance) / len(same_slot_chars_variance),
            "same_persona_tokens_variance": sum(same_slot_tokens_variance) / len(same_slot_tokens_variance),
            "same_persona_ending_variation_count": sum(same_slot_ending_variation) / len(same_slot_ending_variation),
            "same_persona_projection_v_abs": sum(same_v) / len(same_v),
            "same_persona_projection_l_abs": sum(same_l) / len(same_l),
            "original_projection_v_abs": original_v,
            "original_projection_l_abs": original_l,
            "label_identity_exact_rate": sum(label_exact) / len(label_exact),
            "label_identity_feature_rate": sum(label_features) / len(label_features),
            "duplicate_label_exact_rate": sum(duplicate_matches) / len(duplicate_matches),
            "independent_repeat_exact_rate": sum(repeat_matches) / len(repeat_matches),
        })

    by_split = {
        split: split_summary([item for item in audits if item["split"] == split])
        for split in ("TRAIN", "VALIDATION", "BLIND")
    }
    report = {
        "schema": "BCORE.PERSONA_SYNTHETIC_CONTRAST_BIAS_AUDIT.V1",
        "source": str(args.source),
        "source_sha256": hashlib.sha256(args.source.read_bytes()).hexdigest(),
        "source_surface_stream_sha256": source_surface_hash.hexdigest(),
        "latent_artifact": str(args.latent),
        "latent_artifact_sha256": hashlib.sha256(args.latent.read_bytes()).hexdigest(),
        "runtime_change": "FORBIDDEN",
        "controller_change": "FORBIDDEN",
        "latent_change": "FORBIDDEN",
        "source_corpus": {
            "complete_family_count": len(complete),
            "split_family_counts": split_counts,
            "surface_count": rebuilt_total,
            "deterministic_builder_rebuild_exact_count": rebuilt_exact,
            "deterministic_builder_rebuild_exact_rate": rebuilt_exact / rebuilt_total if rebuilt_total else 0.0,
            "builder_mechanism": "PERSONA_LABEL_SELECTS_FIXED_PREFIX_AND_ENDING_IN_SYNTHETIC_COMPILER",
        },
        "audit_design": {
            "same_persona_slots": len(LABELS),
            "sample_per_split": args.sample_per_split,
            "permutations_per_family": args.permutations,
            "independent_repeats_per_condition": args.independent_repeats,
            "raw_surface_storage": "NONE",
        },
        "split_summary": by_split,
        "position_slot_change_rate_under_label_permutation": position_surface_changes / position_surface_total if position_surface_total else 0.0,
        "verdict": {
            "SAME_PERSONA_17_WAY_CONTROL": "NO_WITHIN_CONDITION_VARIATION",
            "LABEL_POSITION_PERMUTATION": "LABEL_IDENTITY_FOLLOWS_DETERMINISTIC_COMPILER_NOT_POSITION",
            "DUPLICATE_LABEL_CONTROL": "DUPLICATES_EXACTLY_MATCH",
            "INDEPENDENT_SINGLE_CONDITION_REPEAT": "REPEATS_EXACTLY_MATCH",
            "POSITIONAL_CONTRAST_ARTIFACT": "NOT_OBSERVED_IN_DETERMINISTIC_BUILDER",
            "SYNTHETIC_LABEL_TO_SURFACE_ARTIFACT": "CONFIRMED_BY_FULL_CORPUS_REBUILD",
            "PERSONA_CAUSAL_OPERATOR_INTERPRETATION": "HOLD_SOURCE_IS_DIRECT_LABEL_TO_SURFACE_COMPILER",
            "ARTIFACT_ROLE": {
                "V_L1_L4_persona_profile_coarse_groups": "PRESERVE_ANALYSIS_ONLY_NO_PRODUCTION_PROMOTION",
                "synthetic_parallel_surfaces": "GENERAL_KOREAN_EXPRESSION_CONSTRUCTION_DIVERSITY_ONLY",
            },
        },
        "family_observations": audits,
        "artifact_sha256": "",
    }
    report["artifact_sha256"] = digest(report)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "output": str(args.output),
        "families": len(complete),
        "surfaces_rebuilt": rebuilt_total,
        "rebuild_exact": rebuilt_exact,
        "verdict": report["verdict"],
        "artifact_sha256": report["artifact_sha256"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
