"""Build a normalized, non-generative Korean construction attestation index.

Only manifest sources with an explicit approval receipt are read. Exact source
sentences are neither emitted nor used as generation templates.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

from naturalness_verifier import corpus_attestation_signature, digest, normalized_ending_family
from realization_trace_execution_substrate import CONNECTIVES, ENDINGS, PARTICLES


def walk_text(value: object, fields: set[str]):
    if isinstance(value, dict):
        for key, child in value.items():
            if key in fields and isinstance(child, str):
                yield child
            else:
                yield from walk_text(child, fields)
    elif isinstance(value, list):
        for child in value:
            yield from walk_text(child, fields)


def sentences(path: Path, fields: set[str], tsv_text_column: int | None = None):
    if path.suffix.lower() == ".jsonl":
        with path.open(encoding="utf-8-sig") as handle:
            for line in handle:
                if line.strip():
                    yield from walk_text(json.loads(line), fields)
    elif path.suffix.lower() in {".txt", ".tsv"}:
        with path.open(encoding="utf-8-sig") as handle:
            for line in handle:
                line = line.strip()
                if not line:
                    continue
                columns = line.split("\t")
                if tsv_text_column is None:
                    yield columns[-1] if path.suffix.lower() == ".tsv" else line.split("\t", 1)[-1]
                    continue
                if tsv_text_column < 0 or tsv_text_column >= len(columns):
                    raise ValueError(f"TSV text column {tsv_text_column} missing in {path}")
                yield columns[tsv_text_column]
    else:
        yield from walk_text(json.loads(path.read_text(encoding="utf-8-sig")), fields)


def ending_of(surface: str) -> str | None:
    return normalized_ending_family(surface)


def surface_pattern(surface: str) -> dict:
    ending = ending_of(surface)
    terminal = next((char for char in reversed(surface.strip()) if char in ".?!"), None)
    particles = re.findall(rf"(?:{'|'.join(map(re.escape, PARTICLES))})(?=\s|[,.?!]|$)", surface)
    connectives = [item for item in CONNECTIVES if re.search(rf"(?:^|\s){re.escape(item)}(?:\s|,)", surface)]
    return {
        "clause_type": "INTERROGATIVE" if terminal == "?" else "DECLARATIVE",
        "ending_family": ending,
        "terminal": terminal,
        "honorific_morphology": bool(re.search(r"(?:시|십)(?:어요|니다|니까|시오|세요)", surface)),
        "connective_pattern": sorted(connectives),
        "argument_particle_pattern": particles[-4:],
        "clause_count_bucket": "3+" if len(re.findall(r"[.?!]", surface)) >= 3 else str(max(1, len(re.findall(r"[.?!]", surface)))),
    }


def corpus_pattern(surface: str) -> dict:
    pattern = surface_pattern(surface)
    return corpus_attestation_signature({
        "clause_count_bucket": pattern["clause_count_bucket"],
        "final_ending": pattern["ending_family"],
        "terminal": pattern["terminal"],
        "honorific": pattern["honorific_morphology"],
        "connective_present": bool(pattern["connective_pattern"]),
    })


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--manifest", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    approved = []
    pattern_docs: dict[str, set[str]] = defaultdict(set)
    pattern_counts = Counter()
    rejected = []
    for source in manifest.get("sources", []):
        path_value = source.get("local_path")
        path = Path(path_value) if path_value else None
        if path is not None and not path.is_absolute():
            path = args.manifest.parent / path
        receipt_value = source.get("approval_receipt_path")
        receipt_path = Path(receipt_value) if receipt_value else None
        if receipt_path is not None and not receipt_path.is_absolute():
            receipt_path = args.manifest.parent / receipt_path
        if not source.get("approved_for_research") or not source.get("approval_receipt_sha256"):
            rejected.append({"source_id": source.get("source_id"), "reason": "APPROVAL_RECEIPT_REQUIRED"})
            continue
        if receipt_path is None or not receipt_path.exists():
            rejected.append({"source_id": source.get("source_id"), "reason": "APPROVAL_RECEIPT_FILE_MISSING"})
            continue
        receipt_sha256 = hashlib.sha256(receipt_path.read_bytes()).hexdigest()
        if receipt_sha256 != source["approval_receipt_sha256"]:
            rejected.append({"source_id": source.get("source_id"), "reason": "APPROVAL_RECEIPT_HASH_MISMATCH"})
            continue
        if path is None or not path.exists():
            rejected.append({"source_id": source.get("source_id"), "reason": "LOCAL_CORPUS_FILE_MISSING"})
            continue
        file_sha256 = hashlib.sha256(path.read_bytes()).hexdigest()
        if file_sha256 != source.get("source_file_sha256"):
            rejected.append({"source_id": source.get("source_id"), "reason": "SOURCE_FILE_HASH_MISMATCH"})
            continue
        source_record = {
            "source_id": source["source_id"], "corpus_name": source["corpus_name"],
            "file_sha256": file_sha256,
            "approval_receipt_sha256": receipt_sha256,
        }
        approved.append(source_record)
        fields = set(source.get("text_fields", ["form"]))
        tsv_text_column = source.get("tsv_text_column")
        if tsv_text_column is not None and not isinstance(tsv_text_column, int):
            rejected.append({"source_id": source.get("source_id"), "reason": "INVALID_TSV_TEXT_COLUMN"})
            approved.pop()
            continue
        for ordinal, surface in enumerate(sentences(path, fields, tsv_text_column)):
            if not re.search(r"[가-힣]", surface):
                continue
            pattern = corpus_pattern(surface)
            if pattern["final_ending"] is None:
                # An unparsed ending cannot safely attest another unparsed
                # ending.  Preserve fail-closed behavior instead of letting
                # all unknown morphology collapse into one positive bucket.
                continue
            key = digest(pattern)
            pattern_counts[key] += 1
            pattern_docs[key].add(f"{source['source_id']}:{ordinal}")
    patterns = {
        key: {"occurrences": count, "document_count": len(pattern_docs[key])}
        for key, count in sorted(pattern_counts.items())
    }
    artifact = {
        "schema": "BCORE.KOREAN_CORPUS_ATTESTATION_INDEX.V4",
        "generation_use": "FORBIDDEN",
        "exact_sentences_stored": False,
        "authoritative_sources": approved,
        "rejected_sources": rejected,
        "pattern_schema": ["clause_count_bucket", "final_ending", "terminal", "honorific", "connective_present"],
        "pattern_count": len(patterns),
        "patterns": patterns,
        "status": "READY" if approved else "UNKNOWN_NO_APPROVED_LOCAL_CORPUS",
        "artifact_sha256": "",
    }
    artifact["artifact_sha256"] = digest(artifact)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(artifact, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"status": artifact["status"], "sources": len(approved), "patterns": len(patterns), "rejected": len(rejected)}, ensure_ascii=False))


if __name__ == "__main__":
    main()
