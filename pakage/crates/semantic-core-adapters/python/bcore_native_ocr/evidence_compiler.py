"""Coordinate-aware compiler for independently produced OCR evidence.

Whole OCR outputs are never voted on here. Character-identical evidence over
the same pixels can be collapsed, while nested spans remain auditable without
being counted as a second fact.
"""

from __future__ import annotations

from copy import deepcopy
from hashlib import sha256

from .text_normalization import canonicalize_ocr_content


def _content_key(text: str) -> str:
    return "".join(canonicalize_ocr_content(str(text or "")).split())


def _area(box: list[int]) -> int:
    return max(0, int(box[2]) - int(box[0])) * max(0, int(box[3]) - int(box[1]))


def _intersection(first: list[int], second: list[int]) -> int:
    return max(0, min(first[2], second[2]) - max(first[0], second[0])) * max(
        0, min(first[3], second[3]) - max(first[1], second[1])
    )


def _containment(first: list[int], second: list[int]) -> float:
    return _intersection(first, second) / max(1, min(_area(first), _area(second)))


def _evidence_id(item: dict) -> str:
    payload = f"{_content_key(item.get('text', ''))}|{','.join(map(str, item['box']))}"
    return "ocr-" + sha256(payload.encode("utf-8")).hexdigest()[:16]


def compile_approved_text_evidence(rows: list[dict]) -> dict:
    """Deduplicate equivalent evidence and mark nested support as non-additive."""

    compiled: list[dict] = []
    duplicate_count = 0
    for raw in rows:
        item = deepcopy(raw)
        item.setdefault("granularity", "row")
        item.setdefault("factContribution", "primary")
        item.setdefault("provenance", [str(item.get("source") or "unknown")])
        if item.get("status") != "accepted":
            item["factContribution"] = "review_only"
        key = _content_key(item.get("text", ""))
        duplicate = next(
            (
                existing
                for existing in compiled
                if _content_key(existing.get("text", "")) == key
                and _containment(existing["box"], item["box"]) >= 0.90
            ),
            None,
        )
        if duplicate is not None:
            duplicate_count += 1
            duplicate["provenance"] = sorted(
                set(duplicate.get("provenance") or [])
                | set(item.get("provenance") or [])
            )
            duplicate["confidence"] = max(
                float(duplicate.get("confidence", 0.0)),
                float(item.get("confidence", 0.0)),
            )
            duplicate["independentAgreementCount"] = len(duplicate["provenance"])
            if (
                item.get("status") == "accepted"
                and duplicate["independentAgreementCount"] >= 2
            ):
                duplicate["status"] = "accepted"
                duplicate["factContribution"] = "primary"
            continue
        item["evidenceId"] = _evidence_id(item)
        item["independentAgreementCount"] = len(set(item["provenance"]))
        compiled.append(item)

    for child in compiled:
        if child.get("granularity") != "span":
            continue
        parents = [
            parent
            for parent in compiled
            if parent is not child
            and parent.get("granularity") == "row"
            and parent.get("status") == "accepted"
            and _containment(parent["box"], child["box"]) >= 0.65
        ]
        if parents:
            parent = min(parents, key=lambda candidate: _area(candidate["box"]))
            child["factContribution"] = "non_additive_support"
            child["nestedUnderEvidenceId"] = parent["evidenceId"]

    for item in compiled:
        item["factAuthority"] = bool(
            item.get("status") == "accepted"
            and item.get("factContribution") == "primary"
        )

    compiled.sort(key=lambda item: (item["box"][1], item["box"][0], item["granularity"]))
    return {
        "rows": compiled,
        "inputCount": len(rows),
        "compiledCount": len(compiled),
        "deduplicatedCount": duplicate_count,
        "primaryFactEvidenceCount": sum(
            item.get("factContribution") == "primary" for item in compiled
        ),
        "supportOnlyCount": sum(
            item.get("factContribution") == "non_additive_support" for item in compiled
        ),
        "reviewOnlyCount": sum(
            item.get("factContribution") == "review_only" for item in compiled
        ),
        "contract": (
            "Only character-identical, coordinate-equivalent evidence is collapsed; "
            "nested spans remain auditable but cannot double-count visible facts."
        ),
    }
