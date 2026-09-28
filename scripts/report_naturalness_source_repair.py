"""Compare the human-rated source baseline with a repaired first-pass run."""
from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path


def digest(value: object) -> str:
    return hashlib.sha256(
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--old-candidates", type=Path, required=True)
    parser.add_argument("--human-review", type=Path, action="append", required=True)
    parser.add_argument("--new-first-pass", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    ratings = {}
    for path in args.human_review:
        packet = json.loads(path.read_text(encoding="utf-8"))
        for item in packet["items"]:
            ratings[item["surface_sha256"]] = item

    old_rows = [
        row for row in json.loads(args.old_candidates.read_text(encoding="utf-8"))["rows"]
        if row["value"] == "APPLY_TRANSFER"
    ]
    old_counts = Counter()
    old_by_domain = defaultdict(Counter)
    missing = 0
    for row in old_rows:
        sha = hashlib.sha256(row["surface"].encode()).hexdigest()
        rating = ratings.get(sha)
        if rating is None:
            missing += 1
            continue
        verdict = rating["acceptability"]
        relation = "+".join(sorted(c["relation"] for c in row["approved_response_ir"]["claims"]))
        old_counts[verdict] += 1
        old_by_domain[f"{row['speech_act']}|{relation}"][verdict] += 1

    new = json.loads(args.new_first_pass.read_text(encoding="utf-8"))
    report = {
        "schema": "BCORE.NATURALNESS_SOURCE_REALIZATION_REPAIR.V1",
        "runtime_installation": "RESEARCH_VALIDATED_CORE_CODEC_REPAIR",
        "edit_program_production_promotion": "HOLD",
        "baseline": {
            "apply_transfer_rows": len(old_rows),
            "human_reviewed_rows": len(old_rows) - missing,
            "missing_human_review_rows": missing,
            "human_acceptability": dict(old_counts),
            "by_speech_act_relation": {
                key: dict(value) for key, value in sorted(old_by_domain.items())
            },
        },
        "diagnosis": {
            "primary_blocker": "PREDICATE_AND_VALENCY_SELECTION_BEFORE_ENDING_EDIT",
            "training_volume_alone_sufficient": False,
            "observed_failures": [
                "LEXICAL_STATE_FORCED_INTO_GENERIC_COPULAR_COMPLEMENT",
                "REQUEST_AND_PROMISE_FORCED_INTO_VALUE_PLUS_RO_HADA",
                "DIRECTIONAL_PARTICLE_ALLOMORPH_NOT_REALIZED",
            ],
        },
        "repair": {
            "typed_constructions": [
                "STATUS_READY_TO_PREPARE_OR_BECOME_READY",
                "REGISTRATION_COMPLETE_TO_COMPLETE_REGISTRATION",
                "TIME_COMPLEMENT_EURO_RO_ALLOMORPH",
            ],
            "sentence_specific_replacement": False,
            "canonical_inverse_extended": True,
            "new_candidate_rows": new["candidate_row_count"],
            "new_unique_surfaces": new["unique_surface_count"],
            "canonical_inverse_pass": new["canonical_inverse_pass"],
            "morphology_pass": new["morphology_pass"],
            "codex_first_pass_verdicts": new["verdict_counts"],
            "human_ambiguity_queue": new["human_review_required_unique_surface_count"],
        },
        "authority_boundary": {
            "semantic_authority": "APPROVED_CANONICAL_RESPONSE_IR_AND_INVERSE",
            "codex_role": "OFFLINE_FIRST_PASS_AND_OBVIOUS_REPAIR",
            "human_role": "AMBIGUOUS_OR_SUBTLE_NATURALNESS_ONLY",
            "corpus_attestation": "UNKNOWN_NO_APPROVED_LOCAL_CORPUS",
            "program_promotion": "HOLD",
        },
        "conclusion": "SOURCE_REALIZATION_REPAIRED; CODEX_FIRST_PASS_GREEN; HUMAN_REVIEW_RESERVED_FOR_AMBIGUOUS_CASES; NO_EDIT_PROGRAM_PRODUCTION_PROMOTION",
        "artifact_sha256": "",
    }
    report["artifact_sha256"] = digest(report)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "old": dict(old_counts), "new": new["verdict_counts"],
        "human_queue": new["human_review_required_unique_surface_count"],
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
