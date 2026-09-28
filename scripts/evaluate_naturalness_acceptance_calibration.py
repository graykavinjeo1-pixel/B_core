"""Evaluate human/corpus acceptance evidence for the frozen EditPrograms."""
from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path

from naturalness_verifier import digest


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--completed-human", type=Path, required=True)
    ap.add_argument("--human-certificate", type=Path, required=True)
    ap.add_argument("--transfers", type=Path, required=True)
    ap.add_argument("--transfers-gated", type=Path, required=True)
    ap.add_argument("--corpus-index", type=Path, required=True)
    ap.add_argument("--automatic-report", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()

    human = json.loads(args.completed_human.read_text(encoding="utf-8"))
    certificate = json.loads(args.human_certificate.read_text(encoding="utf-8"))
    transfers = json.loads(args.transfers.read_text(encoding="utf-8"))["rows"]
    gated = {row["prompt_sha256"]: row for row in json.loads(args.transfers_gated.read_text(encoding="utf-8"))}
    corpus = json.loads(args.corpus_index.read_text(encoding="utf-8"))
    automatic = json.loads(args.automatic_report.read_text(encoding="utf-8"))
    rating = {row["surface_sha256"]: row for row in human["items"]}

    groups = defaultdict(list)
    for row in transfers:
        if row["value"] != "APPLY_TRANSFER":
            continue
        key = (row["conditions"]["program_cluster_id"], row["conditions"]["direction"])
        sha = hashlib.sha256(row["surface"].encode()).hexdigest()
        review = rating.get(sha)
        relation_shape = tuple(sorted(claim["relation"] for claim in row["approved_response_ir"].get("claims", [])))
        groups[key].append({
            "meaning_family_id": row["meaning_family_id"], "split": row["split"],
            "speech_act": row["speech_act"], "relation_shape": relation_shape,
            "inverse_pass": bool(gated.get(row["prompt_sha256"], {}).get("canonical_inverse")),
            "human": review,
        })

    program_results = []
    for key, rows in sorted(groups.items()):
        reviewed = [row for row in rows if row["human"] is not None]
        acceptability = Counter(row["human"]["acceptability"] for row in reviewed)
        by_relation = defaultdict(Counter)
        for row in reviewed:
            by_relation["+".join(row["relation_shape"])][row["human"]["acceptability"]] += 1
        natural = acceptability["NATURAL"]
        all_natural = bool(reviewed) and natural == len(reviewed)
        corpus_ready = corpus.get("status") == "READY"
        enough = len({row["meaning_family_id"] for row in reviewed}) >= 8
        promotion = (
            all(row["inverse_pass"] for row in rows)
            and all_natural and enough and corpus_ready
            and certificate.get("status") == "CALIBRATED"
            and certificate.get("hard_negative_false_pass") == 0
        )
        if not reviewed:
            naturalness_state = "INSUFFICIENT_HUMAN_POSITIVE_EVIDENCE"
        elif not all_natural:
            naturalness_state = "NATURALNESS_DOMAIN_MIXED"
        elif not enough:
            naturalness_state = "INSUFFICIENT_UNRELATED_MEANING_COVERAGE"
        elif not corpus_ready:
            naturalness_state = "CORPUS_ATTESTATION_UNKNOWN"
        else:
            naturalness_state = "HUMAN_EVIDENCE_AVAILABLE"
        program_results.append({
            "program_cluster_id": key[0], "direction": key[1],
            "transfer_count": len(rows), "inverse_pass": sum(row["inverse_pass"] for row in rows),
            "human_reviewed_transfer_count": len(reviewed),
            "human_reviewed_family_count": len({row["meaning_family_id"] for row in reviewed}),
            "human_acceptability": dict(acceptability),
            "human_acceptability_by_relation_shape": {name: dict(counts) for name, counts in sorted(by_relation.items())},
            "naturalness_state": naturalness_state,
            "promotion": "REUSABLE_EXPRESSION_PROGRAM" if promotion else "HOLD",
        })

    report = {
        "schema": "BCORE.NATURALNESS_ACCEPTANCE_CALIBRATION.V1",
        "runtime_installation": "FORBIDDEN",
        "production_promotion": "HOLD",
        "frozen_naturalness_verifier_changed": False,
        "human_calibration": {
            "status": certificate["status"],
            "benchmark_size": certificate["benchmark_size"],
            "positive_accepted": certificate["positive_accepted"],
            "positive_count": certificate["positive_count"],
            "automatic_hard_negative_false_pass": certificate["hard_negative_false_pass"],
            "human_only_negative_accept": certificate["human_only_negative_accept"],
        },
        "automatic_gate": {
            "morphology_error_sensitivity": automatic["benchmark"]["morphology_error_sensitivity"],
            "construction_incompatibility_sensitivity": automatic["benchmark"]["construction_incompatibility_sensitivity"],
        },
        "corpus_attestation": {"status": corpus.get("status"), "source_count": len(corpus.get("authoritative_sources", []))},
        "program_results": program_results,
        "promoted_program_count": sum(row["promotion"] == "REUSABLE_EXPRESSION_PROGRAM" for row in program_results),
        "conclusion": "POSITIVE_ACCEPTANCE_NOT_CALIBRATED; PROGRAMS_REMAIN_NATURALNESS_UNVERIFIED; NO_PRODUCTION_PROMOTION",
        "artifact_sha256": "",
    }
    report["artifact_sha256"] = digest(report)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "human_calibration": certificate["status"],
        "programs": {f"{row['program_cluster_id'][:8]}:{row['direction']}": row["naturalness_state"] for row in program_results},
        "promoted": report["promoted_program_count"],
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
