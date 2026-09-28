"""Compile an offline Codex first-pass review before human calibration.

The reviewer is a screening/editorial stage, never semantic authority and never
a production naturalness certificate.  Every surface must already pass the
canonical inverse gate.  Only AMBIGUOUS_FOR_HUMAN items are emitted to the
human packet; obvious failures remain blocked and obvious natural candidates
remain research evidence pending the frozen corpus/human acceptance gates.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path

from naturalness_verifier import digest, morphology_verdict


VERDICTS = {"OBVIOUS_NATURAL", "OBVIOUS_UNNATURAL", "AMBIGUOUS_FOR_HUMAN"}


def surface_hash(surface: str) -> str:
    return hashlib.sha256(surface.encode()).hexdigest()


def claim_summary(ir: dict) -> list[dict]:
    return [
        {
            "subject": claim.get("subject", {}).get("canonical_lexical_label"),
            "relation": claim.get("relation"),
            "value": claim.get("value"),
            "polarity": claim.get("polarity"),
            "modality": claim.get("modality"),
        }
        for claim in ir.get("claims", [])
    ]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidates", type=Path, required=True)
    parser.add_argument("--gated", type=Path, required=True)
    parser.add_argument("--review", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--human-packet-output", type=Path, required=True)
    args = parser.parse_args()

    candidates = [
        row
        for row in json.loads(args.candidates.read_text(encoding="utf-8"))["rows"]
        if row["value"] == "APPLY_TRANSFER"
    ]
    gated = {
        row["prompt_sha256"]: row
        for row in json.loads(args.gated.read_text(encoding="utf-8"))
    }
    review = json.loads(args.review.read_text(encoding="utf-8"))
    if review.get("reviewer", {}).get("kind") != "CODEX_LANGUAGE_MODEL_FIRST_PASS":
        raise RuntimeError("CODEX_FIRST_PASS_REVIEWER_REQUIRED")

    reviewed = {}
    for item in review.get("items", []):
        if item.get("verdict") not in VERDICTS:
            raise RuntimeError(f"INVALID_FIRST_PASS_VERDICT:{item.get('surface_sha256')}")
        reviewed[item["surface_sha256"]] = item

    expected_hashes = {surface_hash(row["surface"]) for row in candidates}
    missing = expected_hashes - reviewed.keys()
    extra = reviewed.keys() - expected_hashes
    if missing or extra:
        raise RuntimeError(f"REVIEW_COVERAGE_MISMATCH:missing={len(missing)}:extra={len(extra)}")

    results = []
    by_domain = defaultdict(Counter)
    human_items = []
    for row in candidates:
        sha = surface_hash(row["surface"])
        decision = reviewed[sha]
        inverse = bool(gated.get(row["prompt_sha256"], {}).get("canonical_inverse"))
        morphology = morphology_verdict(row["surface"])
        relation = "+".join(sorted(claim["relation"] for claim in row["approved_response_ir"]["claims"]))
        domain = f"{row['speech_act']}|{relation}"
        by_domain[domain][decision["verdict"]] += 1
        result = {
            "meaning_family_id": row["meaning_family_id"],
            "split": row["split"],
            "speech_act": row["speech_act"],
            "relation_shape": relation,
            "surface": row["surface"],
            "surface_sha256": sha,
            "canonical_inverse": inverse,
            "morphology_verdict": morphology["verdict"],
            "first_pass_verdict": decision["verdict"],
            "rationale": decision.get("rationale"),
            "production_authority": False,
        }
        if not inverse or morphology["verdict"] == "FAIL":
            result["effective_disposition"] = "BLOCKED_BY_EXISTING_GATE"
        elif decision["verdict"] == "OBVIOUS_UNNATURAL":
            result["effective_disposition"] = "BLOCK_AND_REPAIR"
        elif decision["verdict"] == "AMBIGUOUS_FOR_HUMAN":
            result["effective_disposition"] = "HUMAN_REVIEW_REQUIRED"
            ir = row["approved_response_ir"]
            human_items.append({
                "review_id": digest({"surface_sha256": sha, "meaning_family_id": row["meaning_family_id"]}),
                "surface_sha256": sha,
                "surface": row["surface"],
                "canonical_summary": {
                    "speech_act": ir.get("speech_act"),
                    "operation": ir.get("operation"),
                    "discourse_relation": ir.get("discourse_relation"),
                    "claims": claim_summary(ir),
                },
                "natural_korean": None,
                "meaning_preserved": None,
                "contextually_usable": None,
                "acceptability": None,
                "notes": None,
            })
        else:
            result["effective_disposition"] = "RESEARCH_CANDIDATE_NO_PRODUCTION_PROMOTION"
        results.append(result)

    verdict_counts = Counter(item["first_pass_verdict"] for item in results)
    report = {
        "schema": "BCORE.CODEX_NATURALNESS_FIRST_PASS.V1",
        "runtime_installation": "FORBIDDEN",
        "semantic_authority": "APPROVED_CANONICAL_RESPONSE_IR_AND_INVERSE",
        "naturalness_authority": "NON_AUTHORITATIVE_OFFLINE_SCREENING",
        "production_promotion": "HOLD",
        "candidate_row_count": len(results),
        "unique_surface_count": len(expected_hashes),
        "canonical_inverse_pass": sum(item["canonical_inverse"] for item in results),
        "morphology_pass": sum(item["morphology_verdict"] == "PASS" for item in results),
        "verdict_counts": dict(verdict_counts),
        "human_review_required_unique_surface_count": len(human_items),
        "by_speech_act_relation": {key: dict(value) for key, value in sorted(by_domain.items())},
        "results": results,
        "artifact_sha256": "",
    }
    report["artifact_sha256"] = digest(report)
    human_packet = {
        "schema": "BCORE.HUMAN_NATURALNESS_CALIBRATION_PACKET.V3",
        "selection_policy": "ONLY_CODEX_FIRST_PASS_AMBIGUOUS_ITEMS",
        "allowed_acceptability": ["NATURAL", "AWKWARD_BUT_GRAMMATICAL", "UNNATURAL"],
        "reviewer": {"kind": "HUMAN", "reviewer_id": None, "attestation": None, "reviewed_at": None},
        "items": human_items,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    args.human_packet_output.write_text(json.dumps(human_packet, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "rows": len(results), "unique_surfaces": len(expected_hashes),
        "verdicts": dict(verdict_counts), "human_items": len(human_items),
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
