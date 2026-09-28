"""Analyze paired teacher state-sensitivity evidence without runtime installation.

The report intentionally records structural transition signatures rather than
raw teacher surfaces.  Canonical inverse approval is supplied by the separate
Rust gate and remains the semantic authority.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter, defaultdict
from pathlib import Path


ENDING_FAMILIES = {
    "FORMAL": ("입니까", "습니까", "입니다", "습니다", "십시오", "하겠습니다", "겠습니다"),
    "POLITE": ("인가요", "이에요", "예요", "세요", "할게요"),
    "PLAIN": ("다",),
    "INFORMAL": ("야", "어", "아", "해"),
}
DISCOURSE_MARKERS = ("우선", "먼저", "다만", "참고로", "그래도", "정리하면", "그러니까")
HEDGE_MARKERS = ("혹시", "가능하면", "괜찮으시면", "아마", "것 같")
EMOTION_MARKERS = ("걱정", "안심", "괜찮", "다행")
DEFERENCE_MARKERS = ("주십시오", "주세요", "주시겠", "부탁", "드립니다", "드릴게")
ROLE_MARKERS = ("시간", "상태", "등록 상태", "확정 여부", "위치")


def ending_family(text: str) -> str:
    compact = text.rstrip(".?!。！？ ")
    for family, endings in ENDING_FAMILIES.items():
        if compact.endswith(endings):
            return family
    return "OTHER"


def first_role_index(text: str) -> int:
    positions = [text.find(marker) for marker in ROLE_MARKERS if text.find(marker) >= 0]
    return min(positions) if positions else -1


def features(text: str) -> dict[str, object]:
    compact = text.strip()
    role_index = first_role_index(compact)
    tokens = re.findall(r"[가-힣A-Za-z0-9]+", compact)
    return {
        "ending_family": ending_family(compact),
        "honorific": bool(re.search(r"(?:시|께|님)", compact)),
        "deference": any(marker in compact for marker in DEFERENCE_MARKERS),
        "hedge": any(marker in compact for marker in HEDGE_MARKERS),
        "discourse_marker": next((marker for marker in DISCOURSE_MARKERS if marker in compact), "NONE"),
        "emotion_marker": next((marker for marker in EMOTION_MARKERS if marker in compact), "NONE"),
        "clause_count": max(1, len(re.findall(r"[.!?。！？]", compact))),
        "subject_omitted": role_index == 0 or compact.startswith(tuple(DISCOURSE_PREFIXES)),
        "information_order": "ROLE_EARLY" if 0 <= role_index <= 8 else "ROLE_LATE_OR_UNKNOWN",
        "token_count": len(tokens),
        "char_count": len(compact),
    }


DISCOURSE_PREFIXES = DISCOURSE_MARKERS + ("걱정하지", "안심하", "괜찮")


def structural_delta(left: dict[str, object], right: dict[str, object]) -> dict[str, str]:
    changed: dict[str, str] = {}
    categorical = (
        "ending_family", "honorific", "deference", "hedge", "discourse_marker",
        "emotion_marker", "subject_omitted", "information_order",
    )
    for name in categorical:
        if left[name] != right[name]:
            changed[name] = f"{left[name]}->{right[name]}"
    for name in ("clause_count", "token_count", "char_count"):
        if left[name] != right[name]:
            direction = "INCREASE" if right[name] > left[name] else "DECREASE"
            changed[name] = direction
    return changed


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
    signature_occurrences: dict[tuple[str, str], list[str]] = defaultdict(list)
    direction_counts: dict[str, Counter[str]] = defaultdict(Counter)
    for pair in gold["pairs"]:
        left = features(pair["left_surface"])
        right = features(pair["right_surface"])
        delta = structural_delta(left, right)
        signature = "IDENTICAL" if not delta else "|".join(f"{k}:{v}" for k, v in sorted(delta.items()))
        semantic_invariance = (
            pair["left_inverse"]
            and pair["right_inverse"]
            and pair["claim_same"]
            and pair["polarity_same"]
            and pair["modality_same"]
            and pair["event_phase_same"]
            and pair["unsupported_fact_count"] == 0
        )
        left_register = pair.get("left_conditions", {}).get("register")
        right_register = pair.get("right_conditions", {}).get("register")
        expected_ending_direction = (
            f"{left_register}->{right_register}"
            if left_register and right_register and left_register != right_register
            else None
        )
        observed_ending_direction = delta.get("ending_family")
        expected_ending_match = (
            observed_ending_direction == expected_ending_direction
            if expected_ending_direction is not None
            else None
        )
        row = {
            "paired_observation_id": pair.get("paired_observation_id"),
            "meaning_family_id": pair["meaning_family_id"],
            "split": pair["split"],
            "factor": pair["factor"],
            "speech_act": pair.get("speech_act"),
            "surface_changed": pair["surface_pair_changed"],
            "semantic_invariance": semantic_invariance,
            "structural_difference": delta,
            "transition_signature": signature,
            "expected_ending_direction": expected_ending_direction,
            "observed_ending_direction": observed_ending_direction,
            "expected_ending_match": expected_ending_match,
        }
        rows.append(row)
        signature_occurrences[(pair["factor"], signature)].append(pair["split"])
        for feature, direction in delta.items():
            direction_counts[pair["factor"]][f"{feature}:{direction}"] += 1

    factor_summary = {}
    for factor in sorted({row["factor"] for row in rows}):
        items = [row for row in rows if row["factor"] == factor]
        changed = [row for row in items if row["surface_changed"]]
        recurring = {
            signature: Counter(splits)
            for (candidate_factor, signature), splits in signature_occurrences.items()
            if candidate_factor == factor and signature != "IDENTICAL" and len(set(splits)) > 1
        }
        blind = [row for row in items if row["split"] == "BLIND"]
        blind_recurrent = sum(
            row["transition_signature"] != "IDENTICAL"
            and any(
                other["factor"] == factor
                and other["split"] != "BLIND"
                and other["transition_signature"] == row["transition_signature"]
                for other in items
            )
            for row in blind
        )
        register_controlled = [row for row in items if row["expected_ending_direction"] is not None]
        ending_observed = [row for row in register_controlled if row["observed_ending_direction"] is not None]
        expected_matches = [row for row in ending_observed if row["expected_ending_match"]]
        signature_counts = Counter(row["transition_signature"] for row in changed)
        dominant_signature, dominant_count = signature_counts.most_common(1)[0] if signature_counts else ("IDENTICAL", 0)
        factor_summary[factor] = {
            "pair_count": len(items),
            "split_counts": {split: sum(row["split"] == split for row in items) for split in ("TRAIN", "VALIDATION", "BLIND")},
            "surface_change_count": len(changed),
            "surface_change_rate": len(changed) / len(items) if items else 0.0,
            "semantic_invariance_all": all(row["semantic_invariance"] for row in items),
            "structural_feature_transition_counts": dict(direction_counts[factor]),
            "recurring_transition_signatures": {key: dict(value) for key, value in recurring.items()},
            "blind_pair_count": len(blind),
            "blind_surface_change_count": sum(row["surface_changed"] for row in blind),
            "blind_recurrent_transition_count": blind_recurrent,
            "dominant_transition_signature": dominant_signature,
            "dominant_transition_consistency": dominant_count / len(changed) if changed else 0.0,
            "register_controlled_pair_count": len(register_controlled),
            "observed_ending_transition_count": len(ending_observed),
            "expected_register_to_ending_match_count": len(expected_matches),
            "expected_register_to_ending_match_rate": len(expected_matches) / len(ending_observed) if ending_observed else None,
            "state_direction_consistency": {
                key: count / len(changed) if changed else 0.0
                for key, count in direction_counts[factor].items()
            },
        }

    parsed = candidates["parsed_observation_count"]
    malformed = candidates["malformed_observation_count"]
    approved = sum(row["approval"] == "APPROVED_STATE_CONTRAST_GOLD" for row in gated)
    result = {
        "schema": "BCORE.TEACHER_STATE_SENSITIVITY_CALIBRATION_REPORT.V1",
        "acquisition_protocol": candidates["acquisition_protocol"],
        "teacher_model": candidates["teacher_model"],
        "bf16_loaded": candidates["bf16_loaded"],
        "runtime_installation": "FORBIDDEN",
        "requested_observation_count": candidates["requested_family_count"],
        "parsed_observation_count": parsed,
        "malformed_observation_count": malformed,
        "candidate_surface_count": len(candidates["rows"]),
        "inverse_approved_surface_count": approved,
        "inverse_approval_rate_of_parsed_surfaces": approved / len(candidates["rows"]) if candidates["rows"] else 0.0,
        "approved_pair_count": len(rows),
        "incomplete_pair_count": len(gold["incomplete_pairs"]),
        "factor_summary": factor_summary,
        "pair_observations": rows,
        "controller_change": "FORBIDDEN_BY_TRANCHE",
        "new_latent": "FORBIDDEN_BY_TRANCHE",
        "status": "CALIBRATION_EVIDENCE_ONLY",
        "artifact_sha256": "",
    }
    result["artifact_sha256"] = hashlib.sha256(
        json.dumps(result, ensure_ascii=False, separators=(",", ":")).encode()
    ).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "output": str(args.output),
        "parsed": parsed,
        "approved_pairs": len(rows),
        "artifact_sha256": result["artifact_sha256"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
