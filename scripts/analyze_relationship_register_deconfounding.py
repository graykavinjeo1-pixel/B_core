"""Report relationship/register main effects and blind interaction evidence.

This is an evidence-only analyzer.  It never loads a surface into the runtime
and does not update the frozen language-state controller or its latent basis.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from collections import Counter, defaultdict
from pathlib import Path

from analyze_paired_teacher_state_sensitivity import features, structural_delta


ENDING_ORDER = ("FORMAL", "POLITE", "PLAIN", "INFORMAL", "OTHER")
DISCOURSE_ORDER = ("NONE", "우선", "먼저", "다만", "참고로", "그래도", "정리하면", "그러니까")
EMOTION_ORDER = ("NONE", "걱정", "안심", "괜찮", "다행")


def vector(feature: dict[str, object]) -> list[float]:
    result = [float(feature["ending_family"] == item) for item in ENDING_ORDER]
    result.extend(float(feature[name]) for name in ("honorific", "deference", "hedge", "subject_omitted"))
    result.extend(float(feature["discourse_marker"] == item) for item in DISCOURSE_ORDER)
    result.extend(float(feature["emotion_marker"] == item) for item in EMOTION_ORDER)
    result.extend(float(feature[name]) for name in ("clause_count", "token_count", "char_count"))
    return result


def subtract(right: list[float], left: list[float]) -> list[float]:
    return [a - b for a, b in zip(right, left)]


def add(left: list[float], right: list[float]) -> list[float]:
    return [a + b for a, b in zip(left, right)]


def mean(vectors: list[list[float]]) -> list[float]:
    if not vectors:
        return []
    return [sum(vector[index] for vector in vectors) / len(vectors) for index in range(len(vectors[0]))]


def cosine(left: list[float], right: list[float]) -> float | None:
    norm_left = math.sqrt(sum(value * value for value in left))
    norm_right = math.sqrt(sum(value * value for value in right))
    if not norm_left or not norm_right:
        return None
    return sum(a * b for a, b in zip(left, right)) / (norm_left * norm_right)


def expected_boundary(kind: str, left: dict[str, str], right: dict[str, str]) -> bool:
    changed = {key for key in set(left) | set(right) if left.get(key) != right.get(key)}
    expected = {
        "RELATIONSHIP_MAIN": {"relationship"},
        "REGISTER_MAIN": {"register"},
        "INTERACTION_HOLDOUT": {"relationship", "register"},
    }[kind]
    return changed == expected


def semantic_invariance(pair: dict) -> bool:
    return (
        pair["left_inverse"]
        and pair["right_inverse"]
        and pair["claim_same"]
        and pair["polarity_same"]
        and pair["modality_same"]
        and pair["event_phase_same"]
        and pair["unsupported_fact_count"] == 0
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidates", type=Path, required=True)
    parser.add_argument("--gated", type=Path, required=True)
    parser.add_argument("--gold", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    candidates = json.loads(args.candidates.read_text(encoding="utf-8"))
    gated = json.loads(args.gated.read_text(encoding="utf-8"))
    gold = json.loads(args.gold.read_text(encoding="utf-8"))

    rows = []
    for pair in gold["pairs"]:
        kind = pair["contrast_kind"]
        if kind not in {"RELATIONSHIP_MAIN", "REGISTER_MAIN", "INTERACTION_HOLDOUT"}:
            raise ValueError(f"unexpected contrast kind: {kind}")
        left = features(pair["left_surface"])
        right = features(pair["right_surface"])
        delta = structural_delta(left, right)
        signature = "IDENTICAL" if not delta else "|".join(f"{key}:{value}" for key, value in sorted(delta.items()))
        rows.append({
            "paired_observation_id": pair["paired_observation_id"],
            "meaning_family_id": pair["meaning_family_id"],
            "speech_act": pair["speech_act"],
            "split": pair["split"],
            "contrast_kind": kind,
            "interaction_holdout": pair["interaction_holdout"],
            "condition_boundary_valid": expected_boundary(kind, pair["left_conditions"], pair["right_conditions"]),
            "surface_changed": pair["surface_pair_changed"],
            "semantic_invariance": semantic_invariance(pair),
            "structural_difference": delta,
            "transition_signature": signature,
            "delta_vector": subtract(vector(right), vector(left)),
            "left_register": pair["left_conditions"].get("register"),
            "right_register": pair["right_conditions"].get("register"),
            "observed_ending_direction": delta.get("ending_family"),
        })

    by_group: dict[tuple[str, str], list[dict]] = defaultdict(list)
    for row in rows:
        by_group[(row["speech_act"], row["contrast_kind"])].append(row)

    summary = {}
    for (speech_act, kind), items in sorted(by_group.items()):
        changed = [item for item in items if item["surface_changed"]]
        signatures = Counter(item["transition_signature"] for item in changed)
        dominant, dominant_count = signatures.most_common(1)[0] if signatures else ("IDENTICAL", 0)
        train_signatures = {item["transition_signature"] for item in items if item["split"] == "TRAIN"}
        blind = [item for item in items if item["split"] == "BLIND"]
        register_changed = [item for item in items if item["left_register"] != item["right_register"]]
        register_ending_observed = [item for item in register_changed if item["observed_ending_direction"] is not None]
        register_direction_matches = [
            item for item in register_ending_observed
            if item["observed_ending_direction"] == f"{item['left_register']}->{item['right_register']}"
        ]
        fixed_register = [item for item in items if item["left_register"] == item["right_register"]]
        fixed_register_ending_changed = [item for item in fixed_register if item["observed_ending_direction"] is not None]
        summary[f"{speech_act}:{kind}"] = {
            "pair_count": len(items),
            "split_counts": {split: sum(item["split"] == split for item in items) for split in ("TRAIN", "VALIDATION", "BLIND")},
            "condition_boundary_all_valid": all(item["condition_boundary_valid"] for item in items),
            "semantic_invariance_all": all(item["semantic_invariance"] for item in items),
            "surface_change_count": len(changed),
            "surface_change_rate": len(changed) / len(items) if items else 0.0,
            "structural_transition_counts": dict(Counter(
                key + ":" + value for item in changed for key, value in item["structural_difference"].items()
            )),
            "dominant_transition_signature": dominant,
            "dominant_transition_consistency": dominant_count / len(changed) if changed else 0.0,
            "blind_pair_count": len(blind),
            "blind_recurrent_transition_count": sum(
                item["transition_signature"] != "IDENTICAL" and item["transition_signature"] in train_signatures
                for item in blind
            ),
            "register_changed_pair_count": len(register_changed),
            "register_ending_transition_count": len(register_ending_observed),
            "register_expected_ending_match_count": len(register_direction_matches),
            "register_expected_ending_match_rate": len(register_direction_matches) / len(register_ending_observed) if register_ending_observed else None,
            "fixed_register_pair_count": len(fixed_register),
            "fixed_register_ending_changed_count": len(fixed_register_ending_changed),
        }

    train_effects: dict[tuple[str, str], list[list[float]]] = defaultdict(list)
    for row in rows:
        if row["split"] == "TRAIN" and row["contrast_kind"] in {"RELATIONSHIP_MAIN", "REGISTER_MAIN"}:
            train_effects[(row["speech_act"], row["contrast_kind"])].append(row["delta_vector"])

    interaction = []
    for row in rows:
        if row["contrast_kind"] != "INTERACTION_HOLDOUT":
            continue
        relationship = mean(train_effects[(row["speech_act"], "RELATIONSHIP_MAIN")])
        register = mean(train_effects[(row["speech_act"], "REGISTER_MAIN")])
        predicted = add(relationship, register) if relationship and register else []
        actual = row["delta_vector"]
        mean_absolute_error = (
            sum(abs(a - b) for a, b in zip(actual, predicted)) / len(actual)
            if predicted else None
        )
        sign_total = sum(bool(a) or bool(b) for a, b in zip(actual, predicted))
        sign_matches = sum(
            (a == 0 and b == 0) or (a * b > 0)
            for a, b in zip(actual, predicted)
            if bool(a) or bool(b)
        )
        interaction.append({
            "paired_observation_id": row["paired_observation_id"],
            "meaning_family_id": row["meaning_family_id"],
            "speech_act": row["speech_act"],
            "semantic_invariance": row["semantic_invariance"],
            "surface_changed": row["surface_changed"],
            "additive_cosine": cosine(actual, predicted),
            "additive_mean_absolute_error": mean_absolute_error,
            "additive_sign_agreement": sign_matches / sign_total if sign_total else None,
        })

    by_act_interaction: dict[str, list[dict]] = defaultdict(list)
    for item in interaction:
        by_act_interaction[item["speech_act"]].append(item)
    interaction_summary = {
        speech_act: {
            "blind_interaction_count": len(items),
            "semantic_invariance_all": all(item["semantic_invariance"] for item in items),
            "surface_change_count": sum(item["surface_changed"] for item in items),
            "additive_cosine_mean": mean([ [item["additive_cosine"]] for item in items if item["additive_cosine"] is not None])[0] if any(item["additive_cosine"] is not None for item in items) else None,
            "additive_mean_absolute_error_mean": mean([ [item["additive_mean_absolute_error"]] for item in items if item["additive_mean_absolute_error"] is not None])[0] if any(item["additive_mean_absolute_error"] is not None for item in items) else None,
            "additive_sign_agreement_mean": mean([ [item["additive_sign_agreement"]] for item in items if item["additive_sign_agreement"] is not None])[0] if any(item["additive_sign_agreement"] is not None for item in items) else None,
        }
        for speech_act, items in sorted(by_act_interaction.items())
    }

    approved = sum(row["approval"] == "APPROVED_STATE_CONTRAST_GOLD" for row in gated)
    result = {
        "schema": "BCORE.RELATIONSHIP_REGISTER_DECONFOUNDING_REPORT.V1",
        "acquisition_protocol": candidates["acquisition_protocol"],
        "teacher_model": candidates["teacher_model"],
        "bf16_loaded": candidates["bf16_loaded"],
        "runtime_installation": "FORBIDDEN",
        "requested_observation_count": candidates["requested_observation_count"],
        "parsed_observation_count": candidates["parsed_observation_count"],
        "malformed_observation_count": candidates["malformed_observation_count"],
        "candidate_surface_count": len(candidates["rows"]),
        "inverse_approved_surface_count": approved,
        "inverse_approval_rate": approved / len(candidates["rows"]) if candidates["rows"] else 0.0,
        "approved_pair_count": len(rows),
        "incomplete_pair_count": len(gold["incomplete_pairs"]),
        "main_effect_summary": summary,
        "blind_interaction_summary": interaction_summary,
        "interaction_observations": interaction,
        "controller_change": "FORBIDDEN_BY_TRANCHE",
        "language_state_mapping_training": "NOT_STARTED_PENDING_DECONFOUNDED_BLIND_EVIDENCE",
        "new_latent": "HOLD",
        "status": "DECONFOUNDING_EVIDENCE_ONLY",
        "artifact_sha256": "",
    }
    result["artifact_sha256"] = hashlib.sha256(
        json.dumps(result, ensure_ascii=False, separators=(",", ":")).encode()
    ).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"output": str(args.output), "approved_pairs": len(rows), "artifact_sha256": result["artifact_sha256"]}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
