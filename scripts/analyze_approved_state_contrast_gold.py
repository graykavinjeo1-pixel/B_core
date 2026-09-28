"""Project approved teacher contrasts onto frozen V+L1..L4 basis."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import defaultdict
from pathlib import Path

import numpy as np


def vector(text: str) -> np.ndarray:
    tail = text.rstrip()
    ending = "OTHER"
    for name, suffixes in {"FORMAL": ("습니다", "습니까"), "POLITE": ("어요", "아요", "해요"), "INFORMAL": ("야", "어", "아", "해"), "PLAIN": ("다",), "ELDER": ("구먼",)}.items():
        if any(tail.endswith(s + p) for s in suffixes for p in ("", ".", "!", "?")):
            ending = name
            break
    markers = (("바로 말하면", "핵심은", "요점을 정리하면", "정리해 보면", "먼저 말하면"), ("괜찮", "다행", "함께", "걱정", "안심"), ("아이구", "참", "정말", "좋아", "답답", "속상"), ("아마", "것 같", "가능", "대체로", "아마도"), ("먼저", "정리하면", "요점을", "참고로", "결론적으로", "그러니까"))
    values = [len(text), len(text.split()), len(re.findall(r"[.!?。！？]", text)), int("\n" in text), int(any(m in text for m in ("#", "- ", "1.")))]
    values.extend(int(any(m in text for m in group)) for group in markers)
    values.append(int(bool(re.search(r"^[^,，]+(?:이|가|은|는)", text))))
    values.extend(int(ending == name) for name in ("FORMAL", "POLITE", "INFORMAL", "PLAIN", "ELDER", "OTHER"))
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
    rows = []
    for pair in gold["pairs"]:
        delta = vector(pair["right_surface"]) - vector(pair["left_surface"])
        delta[:2] = 0.0
        projection = (delta @ components.T) @ components if components.size else np.zeros_like(delta)
        denom = float(np.dot(delta, delta))
        rows.append({
            "meaning_family_id": pair["meaning_family_id"], "split": pair["split"], "factor": pair["factor"],
            "left_value": pair["left_value"], "right_value": pair["right_value"],
            "surface_pair_changed": pair["surface_pair_changed"],
            "semantic_invariance": pair["left_inverse"] and pair["right_inverse"] and pair["claim_same"] and pair["polarity_same"] and pair["modality_same"] and pair["event_phase_same"] and pair["unsupported_fact_count"] == 0,
            "basis_explained_variance": float(np.dot(projection, projection) / denom) if denom else 1.0,
            "unexplained_residual_norm": float(np.linalg.norm(delta - projection)),
            "delta_norm": float(np.linalg.norm(delta)),
        })
    by_factor = defaultdict(list)
    for row in rows:
        by_factor[row["factor"]].append(row)
    summary = {}
    for factor, items in sorted(by_factor.items()):
        summary[factor] = {
            "pairs": len(items), "split_counts": {s: sum(r["split"] == s for r in items) for s in ("TRAIN", "VALIDATION", "BLIND")},
            "surface_pair_changed_rate": float(np.mean([r["surface_pair_changed"] for r in items])),
            "basis_explained_variance_mean": float(np.mean([r["basis_explained_variance"] for r in items])),
            "unexplained_residual_norm_mean": float(np.mean([r["unexplained_residual_norm"] for r in items])),
            "semantic_invariance_all": all(r["semantic_invariance"] for r in items),
            "blind_semantic_invariance": all(r["semantic_invariance"] for r in items if r["split"] == "BLIND"),
        }
    result = {
        "schema": "BCORE.APPROVED_STATE_CONTRAST_GOLD_ANALYSIS.V1",
        "gold_artifact_sha256": gold["artifact_sha256"], "latent_artifact_sha256": latent["artifact_sha256"],
        "pair_count": len(rows), "factor_summary": summary, "pairs": rows,
        "new_latent": "HOLD_UNTIL_STABLE_RESIDUAL_CAUSAL_BLIND", "promotion": "RESEARCH_SHADOW_ONLY", "artifact_sha256": "",
    }
    result["artifact_sha256"] = hashlib.sha256(json.dumps(result, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"output": str(args.output), "pairs": len(rows), "factors": len(summary), "artifact_sha256": result["artifact_sha256"]}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
