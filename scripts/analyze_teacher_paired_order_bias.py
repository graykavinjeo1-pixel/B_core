"""Analyze Q4 paired normal/reversed/same-state order-bias evidence.

The report deliberately normalizes by *condition identity* (FORMAL/POLITE),
not output side.  It is evidence-only: it neither trains nor installs any
language-state component.
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
    return [right_value - left_value for left_value, right_value in zip(left, right)]


def cosine(left: list[float], right: list[float]) -> float | None:
    left_norm = math.sqrt(sum(value * value for value in left))
    right_norm = math.sqrt(sum(value * value for value in right))
    if not left_norm or not right_norm:
        return None
    return sum(a * b for a, b in zip(left, right)) / (left_norm * right_norm)


def invariant(pair: dict) -> bool:
    return (
        pair["left_inverse"] and pair["right_inverse"] and pair["claim_same"]
        and pair["polarity_same"] and pair["modality_same"] and pair["event_phase_same"]
        and pair["unsupported_fact_count"] == 0
    )


def condition_surfaces(pair: dict) -> dict[str, str] | None:
    left_register = pair["left_conditions"].get("register")
    right_register = pair["right_conditions"].get("register")
    if left_register is None or right_register is None:
        return None
    if left_register == right_register:
        return {left_register: pair["left_surface"], f"{left_register}_B": pair["right_surface"]}
    return {left_register: pair["left_surface"], right_register: pair["right_surface"]}


def signature(left_surface: str, right_surface: str) -> tuple[dict[str, str], str, list[float]]:
    delta = structural_delta(features(left_surface), features(right_surface))
    label = "IDENTICAL" if not delta else "|".join(f"{key}:{value}" for key, value in sorted(delta.items()))
    return delta, label, subtract(vector(features(right_surface)), vector(features(left_surface)))


def rate(count: int, total: int) -> float | None:
    return count / total if total else None


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

    expected_kinds = {
        "REGISTER_NORMAL", "REGISTER_REVERSED", "REGISTER_SAME_FORMAL", "REGISTER_SAME_POLITE"
    }
    by_family: dict[tuple[str, str], dict[str, dict]] = defaultdict(dict)
    malformed_gold_kinds: list[str] = []
    for pair in gold["pairs"]:
        kind = pair.get("contrast_kind")
        if kind not in expected_kinds:
            malformed_gold_kinds.append(str(kind))
            continue
        key = (pair["speech_act"], pair["meaning_family_id"])
        if kind in by_family[key]:
            raise ValueError(f"duplicate pair kind for {key}: {kind}")
        by_family[key][kind] = pair

    family_rows: list[dict] = []
    for (speech_act, meaning_family_id), kinds in sorted(by_family.items()):
        normal = kinds.get("REGISTER_NORMAL")
        reversed_pair = kinds.get("REGISTER_REVERSED")
        same_formal = kinds.get("REGISTER_SAME_FORMAL")
        same_polite = kinds.get("REGISTER_SAME_POLITE")
        split_candidates = {pair["split"] for pair in kinds.values()}
        split = next(iter(split_candidates)) if len(split_candidates) == 1 else "INCONSISTENT"
        complete = all((normal, reversed_pair, same_formal, same_polite))
        row = {
            "speech_act": speech_act,
            "meaning_family_id": meaning_family_id,
            "split": split,
            "complete_four_way": complete,
            "semantic_invariance_all_available": all(invariant(pair) for pair in kinds.values()),
            "available_kinds": sorted(kinds),
        }
        if complete:
            normal_surfaces = condition_surfaces(normal)
            reversed_surfaces = condition_surfaces(reversed_pair)
            same_formal_surfaces = condition_surfaces(same_formal)
            same_polite_surfaces = condition_surfaces(same_polite)
            if not normal_surfaces or not reversed_surfaces or not same_formal_surfaces or not same_polite_surfaces:
                raise ValueError(f"register conditions missing for {meaning_family_id}")
            formal_normal = normal_surfaces.get("FORMAL")
            formal_reversed = reversed_surfaces.get("FORMAL")
            polite_normal = normal_surfaces.get("POLITE")
            polite_reversed = reversed_surfaces.get("POLITE")
            if not all((formal_normal, formal_reversed, polite_normal, polite_reversed)):
                raise ValueError(f"normal/reversed conditions not normalized for {meaning_family_id}")
            formal_delta, formal_signature, _ = signature(formal_normal, formal_reversed)
            polite_delta, polite_signature, _ = signature(polite_normal, polite_reversed)
            normal_delta, normal_signature, normal_vector = signature(formal_normal, polite_normal)
            reversed_delta, reversed_signature, reversed_vector = signature(formal_reversed, polite_reversed)
            same_formal_delta, same_formal_signature, _ = signature(
                same_formal_surfaces["FORMAL"], same_formal_surfaces["FORMAL_B"]
            )
            same_polite_delta, same_polite_signature, _ = signature(
                same_polite_surfaces["POLITE"], same_polite_surfaces["POLITE_B"]
            )
            row.update({
                "formal_identity_exact_across_order": formal_normal == formal_reversed,
                "polite_identity_exact_across_order": polite_normal == polite_reversed,
                "formal_identity_structural_difference": formal_delta,
                "polite_identity_structural_difference": polite_delta,
                "formal_identity_structural_signature": formal_signature,
                "polite_identity_structural_signature": polite_signature,
                "formal_to_polite_normal_signature": normal_signature,
                "formal_to_polite_reversed_signature": reversed_signature,
                "formal_to_polite_signature_match": normal_signature == reversed_signature,
                "formal_to_polite_delta_cosine": cosine(normal_vector, reversed_vector),
                "same_formal_exact": same_formal["left_surface"] == same_formal["right_surface"],
                "same_polite_exact": same_polite["left_surface"] == same_polite["right_surface"],
                "same_formal_structural_signature": same_formal_signature,
                "same_polite_structural_signature": same_polite_signature,
                "same_state_structural_difference": same_formal_signature != "IDENTICAL" or same_polite_signature != "IDENTICAL",
                "position_signal_structural_difference": formal_signature != "IDENTICAL" or polite_signature != "IDENTICAL",
            })
        family_rows.append(row)

    summaries: dict[str, dict] = {}
    for speech_act in ("REQUEST", "PROMISE"):
        rows = [row for row in family_rows if row["speech_act"] == speech_act]
        complete = [row for row in rows if row["complete_four_way"]]
        by_split = {
            split: [row for row in complete if row["split"] == split]
            for split in ("TRAIN", "VALIDATION", "BLIND")
        }
        def compact(items: list[dict]) -> dict:
            cosines = [item["formal_to_polite_delta_cosine"] for item in items if item["formal_to_polite_delta_cosine"] is not None]
            return {
                "complete_family_count": len(items),
                "semantic_invariance_all": all(item["semantic_invariance_all_available"] for item in items),
                "formal_identity_exact_count": sum(item["formal_identity_exact_across_order"] for item in items),
                "polite_identity_exact_count": sum(item["polite_identity_exact_across_order"] for item in items),
                "condition_identity_exact_both_count": sum(item["formal_identity_exact_across_order"] and item["polite_identity_exact_across_order"] for item in items),
                "condition_identity_structural_shift_count": sum(item["position_signal_structural_difference"] for item in items),
                "same_state_exact_both_count": sum(item["same_formal_exact"] and item["same_polite_exact"] for item in items),
                "same_state_structural_difference_count": sum(item["same_state_structural_difference"] for item in items),
                "formal_to_polite_transition_signature_match_count": sum(item["formal_to_polite_signature_match"] for item in items),
                "formal_to_polite_transition_cosine_mean": sum(cosines) / len(cosines) if cosines else None,
                "formal_to_polite_transition_nonzero_cosine_count": len(cosines),
            }
        all_summary = compact(complete)
        all_summary["split"] = {name: compact(items) for name, items in by_split.items()}
        all_summary["incomplete_family_count"] = len(rows) - len(complete)
        summaries[speech_act] = all_summary

    request = summaries["REQUEST"]
    promise = summaries["PROMISE"]
    request_status = "REGISTER_SIGNAL_UNRESOLVED"
    if request["complete_family_count"] and request["same_state_structural_difference_count"]:
        request_status = "REGISTER_SIGNAL_POSITIONALLY_CONFOUNDED"
    elif request["complete_family_count"] and request["condition_identity_structural_shift_count"] == 0:
        request_status = "REGISTER_SIGNAL_CONFIRMED"
    promise_status = "PROMISE_STATE_FACTOR_ENTANGLED"
    if promise["complete_family_count"] and not promise["same_state_structural_difference_count"] and promise["condition_identity_structural_shift_count"] == 0:
        promise_status = "PROMISE_REGISTER_SIGNAL_CONFIRMED"

    result = {
        "schema": "BCORE.TEACHER_PAIRED_ORDER_BIAS_AUDIT_REPORT.V1",
        "acquisition_protocol": candidates["acquisition_protocol"],
        "teacher_model": candidates["teacher_model"],
        "bf16_loaded": candidates["bf16_loaded"],
        "runtime_installation": "FORBIDDEN",
        "requested_observation_count": candidates["requested_observation_count"],
        "parsed_observation_count": candidates["parsed_observation_count"],
        "malformed_observation_count": candidates["malformed_observation_count"],
        "candidate_surface_count": len(candidates["rows"]),
        "inverse_approved_surface_count": sum(row["approval"] == "APPROVED_STATE_CONTRAST_GOLD" for row in gated),
        "approved_pair_count": len(gold["pairs"]),
        "incomplete_pair_count": len(gold["incomplete_pairs"]),
        "unexpected_gold_contrast_kinds": malformed_gold_kinds,
        "condition_identity_normalization": "FORMAL_NORMAL vs FORMAL_REVERSED; POLITE_NORMAL vs POLITE_REVERSED",
        "same_state_controls": "FORMAL/FORMAL and POLITE/POLITE use independent paired calls",
        "speech_act_summary": summaries,
        "verdict": {
            "REQUEST_REGISTER_ONLY": request_status,
            "PROMISE_REGISTER_ONLY": promise_status,
            "controller_change": "FORBIDDEN_BY_TRANCHE",
            "latent_change": "FORBIDDEN_BY_TRANCHE",
            "runtime_inventory_installation": "FORBIDDEN_BY_TRANCHE",
        },
        "family_observations": family_rows,
        "artifact_sha256": "",
    }
    result["artifact_sha256"] = hashlib.sha256(
        json.dumps(result, ensure_ascii=False, separators=(",", ":")).encode()
    ).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "output": str(args.output),
        "request": request_status,
        "promise": promise_status,
        "complete_families": {act: data["complete_family_count"] for act, data in summaries.items()},
        "artifact_sha256": result["artifact_sha256"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
