"""Evaluate the fail-closed naturalness layer and the frozen 56 transfers."""
from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path

from naturalness_verifier import (
    CorpusAttestationIndex,
    HumanCalibration,
    NaturalnessVerifier,
    build_construction_index,
    digest,
    load_optional,
)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--substrate", type=Path, required=True)
    ap.add_argument("--classes", type=Path, required=True)
    ap.add_argument("--benchmark", type=Path, required=True)
    ap.add_argument("--benchmark-gated", type=Path, required=True)
    ap.add_argument("--transfers", type=Path, required=True)
    ap.add_argument("--transfers-gated", type=Path, required=True)
    ap.add_argument("--corpus-index", type=Path)
    ap.add_argument("--human-calibration", type=Path)
    ap.add_argument("--construction-index-output", type=Path, required=True)
    ap.add_argument("--review-packet-output", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()

    substrate = json.loads(args.substrate.read_text(encoding="utf-8"))
    classes = json.loads(args.classes.read_text(encoding="utf-8"))
    benchmark = json.loads(args.benchmark.read_text(encoding="utf-8"))
    benchmark_gated = json.loads(args.benchmark_gated.read_text(encoding="utf-8"))
    transfers = json.loads(args.transfers.read_text(encoding="utf-8"))["rows"]
    transfers_gated = json.loads(args.transfers_gated.read_text(encoding="utf-8"))
    inverse_by_prompt = {row["prompt_sha256"]: row["canonical_inverse"] for row in benchmark_gated}
    transfer_inverse = {row["prompt_sha256"]: row["canonical_inverse"] for row in transfers_gated}

    construction_index = build_construction_index(substrate, classes)
    corpus_payload = load_optional(args.corpus_index)
    human_payload = load_optional(args.human_calibration)
    verifier = NaturalnessVerifier(
        construction_index,
        CorpusAttestationIndex(corpus_payload),
        HumanCalibration.from_payload(human_payload),
    )

    benchmark_results = []
    for row in benchmark["rows"]:
        result = verifier.verify(row["surface"], row["approved_response_ir"])
        result.update({
            "prompt_sha256": row["prompt_sha256"], "value": row["value"],
            "corruption_type": row["conditions"]["corruption_type"],
            "inverse_pass": bool(inverse_by_prompt.get(row["prompt_sha256"], False)),
        })
        benchmark_results.append(result)

    negatives = [row for row in benchmark_results if row["value"] == "HARD_NEGATIVE"]
    positives = [row for row in benchmark_results if row["value"] == "TRUSTED_POSITIVE"]
    semantic_hard_negatives = [row for row in negatives if row["inverse_pass"]]
    negative_by_type = defaultdict(list)
    for row in negatives:
        negative_by_type[row["corruption_type"]].append(row)

    transfer_results = []
    for row in transfers:
        if row["value"] != "APPLY_TRANSFER":
            continue
        result = verifier.verify(row["surface"], row["approved_response_ir"])
        result.update({
            "meaning_family_id": row["meaning_family_id"], "split": row["split"],
            "speech_act": row["speech_act"], "program_cluster_id": row["conditions"]["program_cluster_id"],
            "direction": row["conditions"]["direction"],
            "inverse_pass": bool(transfer_inverse.get(row["prompt_sha256"], False)),
        })
        transfer_results.append(result)

    program_groups = defaultdict(list)
    for row in transfer_results:
        program_groups[(row["program_cluster_id"], row["direction"])].append(row)
    program_results = []
    for key, rows in sorted(program_groups.items()):
        verdicts = Counter(row["verdict"] for row in rows)
        program_results.append({
            "program_cluster_id": key[0], "direction": key[1], "transfer_count": len(rows),
            "inverse_pass": sum(row["inverse_pass"] for row in rows),
            "naturalness_verdicts": dict(verdicts),
            "state": "REUSABLE_EXPRESSION_PROGRAM_CANDIDATE" if verdicts == {"PASS": len(rows)} else "EXECUTION_VALID_APPLICABILITY_VALID_NATURALNESS_UNVERIFIED",
            "promotion": "HOLD" if verdicts != {"PASS": len(rows)} else "CANDIDATE_ONLY",
        })

    review_items = []
    benchmark_surface = {hashlib.sha256(row["surface"].encode()).hexdigest(): row["surface"] for row in benchmark["rows"]}
    for row in (semantic_hard_negatives[:12] + positives[:12]):
        review_items.append({
            "review_id": digest({"surface_sha256": row["surface_sha256"], "packet": "V1"})[:16],
            "surface_sha256": row["surface_sha256"],
            "surface": benchmark_surface[row["surface_sha256"]],
            "human_verdict": None,
            "naturalness_score_1_to_5": None,
            "notes": None,
        })
    review_packet = {
        "schema": "BCORE.HUMAN_NATURALNESS_CALIBRATION_PACKET.V1",
        "runtime_approval_required": False,
        "purpose": "OFFLINE_AUTOMATIC_GATE_PRECISION_CALIBRATION",
        "blinded_from_expected_class_and_automatic_verdict": True,
        "items": review_items,
    }

    hard_negative_rejected = sum(row["verdict"] == "FAIL" for row in semantic_hard_negatives)
    positive_pass = sum(row["verdict"] == "PASS" for row in positives)
    positive_unknown = sum(row["verdict"] == "UNKNOWN" for row in positives)
    positive_false_reject = sum(row["verdict"] == "FAIL" for row in positives)
    morphology_negative_types = {"PARTICLE_ALLOMORPH", "OBSERVED_PARTICLE_ALLOMORPH", "ENDING_STACK", "HONORIFIC_MISMATCH"}
    construction_negative_types = {"ENDING_SPEECH_ACT_CONFLICT", "MALFORMED_CONNECTIVE"}
    morphology_negatives = [row for row in negatives if row["corruption_type"] in morphology_negative_types]
    construction_negatives = [row for row in negatives if row["corruption_type"] in construction_negative_types]
    morphology_caught = sum(row["layers"][0]["verdict"] == "FAIL" for row in morphology_negatives)
    construction_caught = sum(row["layers"][1]["verdict"] == "FAIL" for row in construction_negatives)
    report = {
        "schema": "BCORE.NATURALNESS_VERIFICATION_LAYER.V1",
        "runtime_installation": "FORBIDDEN",
        "production_promotion": "HOLD",
        "verdict_contract": {"PASS": "ALL_FOUR_LAYERS_PASS", "FAIL": "ANY_LAYER_FAIL", "UNKNOWN": "NO_FAILURE_BUT_EVIDENCE_INCOMPLETE", "production_unknown_action": "KEEP_ORIGINAL_SOURCE_REALIZATION"},
        "q4_judge_authority": "EXCLUDED",
        "semantic_inverse_is_naturalness_authority": False,
        "construction_index": {
            "pattern_count": construction_index.artifact()["pattern_count"],
            "recurrent_pattern_count": construction_index.artifact()["recurrent_pattern_count"],
            "raw_sentences_stored": False,
        },
        "corpus_attestation": {
            "authoritative_source_count": len(corpus_payload.get("authoritative_sources", [])),
            "status": "AVAILABLE" if corpus_payload.get("authoritative_sources") else "UNAVAILABLE_FAIL_CLOSED",
        },
        "human_calibration": HumanCalibration.from_payload(human_payload).evaluate(),
        "benchmark": {
            "trusted_positive_count": len(positives),
            "trusted_positive_pass": positive_pass,
            "trusted_positive_unknown": positive_unknown,
            "trusted_positive_false_reject": positive_false_reject,
            "trusted_positive_acceptance": positive_pass / len(positives) if positives else None,
            "unknown_rate": positive_unknown / len(positives) if positives else None,
            "false_reject_rate": positive_false_reject / len(positives) if positives else None,
            "hard_negative_count": len(negatives),
            "semantic_inverse_pass_hard_negative_count": len(semantic_hard_negatives),
            "semantic_hard_negative_rejected": hard_negative_rejected,
            "hard_negative_rejection": hard_negative_rejected / len(semantic_hard_negatives) if semantic_hard_negatives else None,
            "morphology_error_sensitivity": morphology_caught / len(morphology_negatives) if morphology_negatives else None,
            "morphology_error_caught": morphology_caught,
            "morphology_error_total": len(morphology_negatives),
            "construction_incompatibility_sensitivity": construction_caught / len(construction_negatives) if construction_negatives else None,
            "construction_incompatibility_caught": construction_caught,
            "construction_incompatibility_total": len(construction_negatives),
            "by_corruption": {
                name: {
                    "count": len(rows),
                    "inverse_pass": sum(row["inverse_pass"] for row in rows),
                    "fail": sum(row["verdict"] == "FAIL" for row in rows),
                    "unknown": sum(row["verdict"] == "UNKNOWN" for row in rows),
                }
                for name, rows in sorted(negative_by_type.items())
            },
        },
        "frozen_program_transfer": {
            "count": len(transfer_results),
            "semantic_pass": sum(row["inverse_pass"] for row in transfer_results),
            "morphology_pass": sum(row["layers"][0]["verdict"] == "PASS" for row in transfer_results),
            "construction_pass": sum(row["layers"][1]["verdict"] == "PASS" for row in transfer_results),
            "naturalness_verdicts": dict(Counter(row["verdict"] for row in transfer_results)),
            "program_results": program_results,
        },
        "conclusion": "NATURALNESS_LAYER_IMPLEMENTED_FAIL_CLOSED; PROGRAMS_NATURALNESS_UNVERIFIED; NO_RUNTIME_PROMOTION",
        "benchmark_results": benchmark_results,
        "transfer_results": transfer_results,
        "artifact_sha256": "",
    }
    report["artifact_sha256"] = digest(report)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    construction_artifact = construction_index.artifact()
    construction_artifact["artifact_sha256"] = digest(construction_artifact)
    args.construction_index_output.write_text(json.dumps(construction_artifact, ensure_ascii=False, indent=2), encoding="utf-8")
    args.review_packet_output.write_text(json.dumps(review_packet, ensure_ascii=False, indent=2), encoding="utf-8")
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "positives": {"pass": positive_pass, "unknown": positive_unknown, "fail": positive_false_reject},
        "semantic_hard_negatives": {"rejected": hard_negative_rejected, "total": len(semantic_hard_negatives)},
        "transfers": dict(Counter(row["verdict"] for row in transfer_results)),
        "promotion": report["production_promotion"],
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
