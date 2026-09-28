"""Measure reusable persona operators from 17-way meaning-family contrasts.

The source surface is evidence only.  The emitted artifact contains aggregate
typed features and no sentence cache, so it can guide a runtime operator
implementation without becoming a lookup table.
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import re
from pathlib import Path


MARKERS = {
    "directness": ("바로 말하면", "핵심은", "요점을 정리하면", "정리해 보면", "먼저 말하면"),
    "warmth": ("괜찮", "다행", "함께", "걱정", "안심"),
    "emotionality": ("아이구", "참", "정말", "좋아", "답답", "속상"),
    "hedge": ("아마", "것 같", "가능", "대체로", "아마도"),
    "discourse": ("먼저", "정리하면", "요점을", "참고로", "결론적으로", "그러니까"),
}
ENDINGS = (("FORMAL", "습니다"), ("POLITE", "어요"), ("INFORMAL", "야"), ("PLAIN", "다"), ("ELDER", "구먼"))
NUMERIC_AXES = ("chars", "tokens", "sentences", "newline", "markdown", *MARKERS, "subject_front")


def features(text: str) -> dict[str, int | str]:
    return {
        "chars": len(text),
        "tokens": len(text.split()),
        "sentences": len(re.findall(r"[.!?。！？]", text)),
        "newline": int("\n" in text),
        "markdown": int(any(marker in text for marker in ("#", "- ", "1."))),
        **{name: int(any(marker in text for marker in markers)) for name, markers in MARKERS.items()},
        "ending": next((name for name, suffix in ENDINGS if text.rstrip().endswith((suffix, suffix + "."))), "OTHER"),
        "subject_front": int(bool(re.search(r"^[^,，]+(?:이|가|은|는)", text))),
    }


def distance(left: dict[str, float], right: dict[str, float]) -> float:
    return sum(((left[key] - right[key]) / (10 if key == "chars" else 2)) ** 2 for key in NUMERIC_AXES)


def build(source: Path) -> dict:
    families: dict[str, list[tuple[str, dict[str, int | str]]]] = collections.defaultdict(list)
    with source.open(encoding="utf-8") as stream:
        for line in stream:
            record = json.loads(line)
            persona = record.get("persona", {})
            family = record.get("parallel_family_id")
            label = persona.get("persona_label")
            surface = record.get("surface", "")
            if family and label and surface:
                families[family].append((label, features(surface)))

    complete = {
        family: rows
        for family, rows in families.items()
        if len({label for label, _ in rows}) == 17
    }
    labels = sorted({label for rows in complete.values() for label, _ in rows})
    per_persona = {label: {axis: [] for axis in NUMERIC_AXES} for label in labels}
    endings = {label: collections.Counter() for label in labels}
    for rows in complete.values():
        for label, item in rows:
            for axis in NUMERIC_AXES:
                per_persona[label][axis].append(float(item[axis]))
            endings[label][str(item["ending"])] += 1

    centroids = {
        label: {axis: sum(values) / len(values) for axis, values in axes.items()}
        for label, axes in per_persona.items()
    }
    correct = total = 0
    for rows in complete.values():
        for label, item in rows:
            predicted = min(labels, key=lambda candidate: distance(item, centroids[candidate]))
            correct += predicted == label
            total += 1

    axis_report = {}
    consistency_scores = []
    for axis in NUMERIC_AXES:
        means = [centroids[label][axis] for label in labels]
        grand_mean = sum(means) / len(means)
        between = sum((value - grand_mean) ** 2 for value in means) / len(means)
        within = []
        for rows in complete.values():
            values = [float(item[axis]) for _, item in rows]
            mean = sum(values) / len(values)
            within.append(sum((value - mean) ** 2 for value in values) / len(values))
        within_mean = sum(within) / len(within)
        all_values = [value for label in labels for value in per_persona[label][axis]]
        all_mean = sum(all_values) / len(all_values)
        total_variance = sum((value - all_mean) ** 2 for value in all_values) / len(all_values)
        consistency = 1.0 if total_variance == 0 else max(0.0, 1.0 - within_mean / total_variance)
        consistency_scores.append(consistency)
        axis_report[axis] = {
            "between_persona_variance": between,
            "within_family_variance": within_mean,
            "ratio": between / (within_mean + 1e-9),
            "persona_consistency": consistency,
        }

    return {
        "schema": "BCORE.PERSONA_OPERATOR_ANALYSIS.V1",
        "source_path": str(source),
        "family_count": len(complete),
        "persona_count": len(labels),
        "training_unit": "17_WAY_MEANING_FAMILY_CONTRAST_SET",
        "axes": axis_report,
        "persona_summary": {
            label: {
                axis: {"mean": centroids[label][axis]} for axis in NUMERIC_AXES
            }
            | {"ending_distribution": dict(endings[label])}
            for label in labels
        },
        "nearest_centroid_accuracy": correct / total if total else 0.0,
        "persona_consistency": sum(consistency_scores) / len(consistency_scores),
        "persona_separability": correct / total if total else 0.0,
        "surface_authority": "NONAUTHORITATIVE_ANALYSIS_ONLY",
        "operator_promotion_rule": "PROMOTE_ONLY_IF_REPEATED_ACROSS_FAMILIES_AND_SEPARABLE",
        "artifact_sha256": "",
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    artifact = build(args.source)
    canonical = json.dumps({**artifact, "artifact_sha256": ""}, ensure_ascii=False, separators=(",", ":")).encode()
    artifact["artifact_sha256"] = hashlib.sha256(canonical).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(artifact, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "output": str(args.output),
        "family_count": artifact["family_count"],
        "persona_count": artifact["persona_count"],
        "nearest_centroid_accuracy": artifact["nearest_centroid_accuracy"],
        "artifact_sha256": artifact["artifact_sha256"],
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
