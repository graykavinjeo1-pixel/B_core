"""Compile OCR table observations into a fail-closed evidence contract.

This module does not assign apartment-domain field names.  It preserves the
observed row structure, validation state and provenance so a downstream
document compiler can reason about data instead of copying a rendered table.
"""

from __future__ import annotations

import hashlib
import json
from collections import defaultdict


DECISION_PRESENTATION = {
    "AUTO_CONFIRMED": {
        "highlightTone": "success",
        "highlightColor": "green",
        "reviewRequired": False,
    },
    "FORMULA_REPAIRED": {
        "highlightTone": "info",
        "highlightColor": "blue",
        "reviewRequired": False,
    },
    "MANUAL_REVIEW": {
        "highlightTone": "danger",
        "highlightColor": "red",
        "reviewRequired": True,
    },
}


def _apply_decision_status(record: dict, decision_status: str) -> None:
    """Expose one stable UI status without changing evidence authority."""

    record["decisionStatus"] = decision_status
    record["reviewPresentation"] = {
        "status": decision_status,
        **DECISION_PRESENTATION[decision_status],
    }


def _canonical_numeric_tokens(tokens: list[str], column_contracts: list[dict]) -> tuple[list[str], list[dict]]:
    """Format proven grouped-integer columns while preserving raw OCR tokens.

    This is deliberately not a general punctuation repair.  A token is given a
    canonical display only when repeated rows established that its column uses
    grouped integers.  The source token remains untouched for audit and review.
    """

    contracts = {
        int(contract["index"]): contract
        for contract in column_contracts
        if contract.get("index") is not None
    }
    canonical = list(tokens)
    normalizations: list[dict] = []
    for index, token in enumerate(tokens):
        contract = contracts.get(index)
        if not contract or contract.get("valueType") != "grouped_integer":
            continue
        compact = token.replace(",", "").strip()
        if not compact.isdigit():
            continue
        formatted = f"{int(compact):,}"
        canonical[index] = formatted
        if formatted != token:
            normalizations.append(
                {
                    "index": index,
                    "raw": token,
                    "canonical": formatted,
                    "proof": "repeated_column_grouped_integer_contract",
                    "rawPreserved": True,
                }
            )
    return canonical, normalizations


def _decision_counts(*groups: list[dict]) -> dict[str, int]:
    counts = {status: 0 for status in DECISION_PRESENTATION}
    for record in (record for group in groups for record in group):
        status = record.get("decisionStatus")
        if status in counts:
            counts[status] += 1
    return counts


def _evidence_id(source: str, page: int, table_index: int, row_index: int, tokens: list[str]) -> str:
    payload = json.dumps(
        [source, page, table_index, row_index, tokens],
        ensure_ascii=False,
        separators=(",", ":"),
    ).encode("utf-8")
    return f"native-ocr-table-{hashlib.sha256(payload).hexdigest()[:20]}"


def compile_borderless_table_evidence(
    *, source: str, page: int, tables: list[dict]
) -> dict:
    """Return accepted evidence and review items without guessing semantics."""

    accepted: list[dict] = []
    review: list[dict] = []
    for table_index, table in enumerate(tables):
        expected_count = int(table.get("expectedTokenCount") or 0)
        relations = list(table.get("arithmeticRelations") or [])
        column_contracts = list(table.get("columnContracts") or [])
        for row_index, row in enumerate(table.get("rows") or []):
            tokens = [str(token) for token in row.get("tokens") or []]
            canonical_tokens, numeric_normalizations = _canonical_numeric_tokens(
                tokens,
                column_contracts,
            )
            issues = sorted({str(issue) for issue in row.get("validationIssues") or []})
            if expected_count <= 0 or len(tokens) != expected_count:
                issues.append("STRUCTURAL_TOKEN_COUNT_MISMATCH")
            issues = sorted(set(issues))
            record = {
                "schema": "B_CORE_NATIVE_OCR_STRUCTURED_ROW_EVIDENCE_1",
                "evidenceId": _evidence_id(source, page, table_index, row_index, tokens),
                "source": source,
                "page": page,
                "tableIndex": table_index,
                "rowIndex": row_index,
                "sourceRowIndex": row.get("sourceRowIndex"),
                "box": [None, row.get("y0"), None, row.get("y1")],
                "text": str(row.get("text") or ""),
                "tokens": tokens,
                "canonicalTokens": canonical_tokens,
                "numericDisplayNormalizations": numeric_normalizations,
                "expectedTokenCount": expected_count,
                "confidence": float(row.get("confidence") or 0.0),
                "arithmeticRelations": relations,
                "columnContracts": column_contracts,
                "structuralRepairs": list(row.get("structuralRepairs") or []),
                "validationIssues": issues,
                "semanticFieldAssignment": None,
            }
            if row.get("status") == "accepted" and not issues:
                record["status"] = "approved_evidence"
                _apply_decision_status(
                    record,
                    "FORMULA_REPAIRED"
                    if record["structuralRepairs"]
                    else "AUTO_CONFIRMED",
                )
                accepted.append(record)
            else:
                record["status"] = "needs_review"
                _apply_decision_status(record, "MANUAL_REVIEW")
                review.append(record)
    return {
        "schema": "B_CORE_NATIVE_OCR_STRUCTURED_TABLE_EVIDENCE_1",
        "source": source,
        "page": page,
        "approvedEvidence": accepted,
        "reviewQueue": review,
        "counts": {
            "tables": len(tables),
            "approved": len(accepted),
            "needsReview": len(review),
        },
        "decisionCounts": _decision_counts(accepted, review),
        "contract": (
            "Rows and generic numeric column roles are structural OCR evidence only; "
            "domain field names must be resolved from independently verified headers "
            "or document context."
        ),
    }


def compile_ruled_table_evidence(
    *, source: str, page: int, cells: list[dict], text_independently_verified: bool = False
) -> dict:
    """Compile detector-proven ruled cells without inventing field semantics.

    Geometry has already established the table component, row and column for
    every cell.  This compiler merely preserves those coordinates and the OCR
    validation state.  Merged cells are represented by their observed start
    column; missing column numbers are therefore not treated as an error.
    """

    components: dict[int, list[dict]] = defaultdict(list)
    for cell in cells:
        try:
            component_index = int(cell["tableComponentIndex"])
            int(cell["rowIndex"])
            int(cell["columnIndex"])
        except (KeyError, TypeError, ValueError):
            continue
        components[component_index].append(cell)

    accepted: list[dict] = []
    review: list[dict] = []
    for component_index in sorted(components):
        component_cells = components[component_index]
        rows: dict[int, list[dict]] = defaultdict(list)
        for cell in component_cells:
            rows[int(cell["rowIndex"])].append(cell)
        for row_index in sorted(rows):
            ordered = sorted(rows[row_index], key=lambda item: int(item["columnIndex"]))
            populated = [cell for cell in ordered if str(cell.get("text") or "").strip()]
            if not populated:
                continue
            issues = sorted(
                {
                    str(issue)
                    for cell in populated
                    for issue in (cell.get("validationIssues") or [])
                }
            )
            if any(cell.get("status") != "accepted" for cell in populated):
                issues.append("CELL_REQUIRES_REVIEW")
            if any(
                cell.get("ontologyResolution", {}).get("status") == "suggested"
                for cell in populated
            ):
                issues.append("ONTOLOGY_SUGGESTION_REQUIRES_REVIEW")
            if any(
                cell.get("ontologyResolution", {}).get("status") == "ambiguous"
                for cell in populated
            ):
                issues.append("ONTOLOGY_AMBIGUOUS")
            if not text_independently_verified:
                issues.append("RULED_CELL_TEXT_UNVERIFIED")
            issues = sorted(set(issues))
            cell_records = [
                {
                    "columnIndex": int(cell["columnIndex"]),
                    "box": [
                        int(cell["x0"]),
                        int(cell["y0"]),
                        int(cell["x1"]),
                        int(cell["y1"]),
                    ],
                    "text": str(cell.get("text") or ""),
                    "confidence": float(cell.get("confidence") or 0.0),
                    "numericTokens": [str(token) for token in cell.get("numericTokens") or []],
                    "validationIssues": [
                        str(issue) for issue in cell.get("validationIssues") or []
                    ],
                    "ontologyResolution": cell.get("ontologyResolution"),
                    "canonicalSemanticValue": (
                        cell.get("ontologyResolution", {}).get("canonical")
                        if cell.get("ontologyResolution", {}).get("status") == "resolved"
                        else None
                    ),
                    "proposedCanonicalSemanticValue": (
                        cell.get("ontologyResolution", {}).get("canonical")
                        if cell.get("ontologyResolution", {}).get("status") == "suggested"
                        else None
                    ),
                }
                for cell in populated
            ]
            tokens = [cell["text"] for cell in cell_records]
            confidence = sum(cell["confidence"] for cell in cell_records) / len(cell_records)
            box = [
                min(cell["box"][0] for cell in cell_records),
                min(cell["box"][1] for cell in cell_records),
                max(cell["box"][2] for cell in cell_records),
                max(cell["box"][3] for cell in cell_records),
            ]
            record = {
                "schema": "B_CORE_NATIVE_OCR_STRUCTURED_ROW_EVIDENCE_1",
                "evidenceId": _evidence_id(
                    source,
                    page,
                    component_index,
                    row_index,
                    ["ruled", *tokens],
                ),
                "source": source,
                "page": page,
                "sourceKind": "ruled_table",
                "geometryStatus": "detector_proven",
                "textVerification": (
                    "independently_verified"
                    if text_independently_verified
                    else "requires_independent_verification"
                ),
                "tableIndex": component_index,
                "rowIndex": row_index,
                "sourceRowIndex": row_index,
                "box": box,
                "text": " | ".join(tokens),
                "tokens": tokens,
                "cells": cell_records,
                "expectedTokenCount": None,
                "confidence": confidence,
                "arithmeticRelations": [],
                "structuralRepairs": [],
                "validationIssues": issues,
                "semanticFieldAssignment": None,
            }
            if not issues:
                record["status"] = "approved_evidence"
                formula_repaired = any(
                    cell.get("structuralCorrections") for cell in populated
                )
                _apply_decision_status(
                    record,
                    "FORMULA_REPAIRED" if formula_repaired else "AUTO_CONFIRMED",
                )
                accepted.append(record)
            else:
                record["status"] = "needs_review"
                _apply_decision_status(record, "MANUAL_REVIEW")
                review.append(record)
    return {
        "schema": "B_CORE_NATIVE_OCR_STRUCTURED_TABLE_EVIDENCE_1",
        "source": source,
        "page": page,
        "approvedEvidence": accepted,
        "reviewQueue": review,
        "counts": {
            "tables": len(components),
            "approved": len(accepted),
            "needsReview": len(review),
        },
        "decisionCounts": _decision_counts(accepted, review),
        "contract": (
            "Rows are structural OCR evidence only; domain field names must be "
            "resolved from independently verified headers or document context."
        ),
    }


def combine_table_evidence(*receipts: dict) -> dict:
    """Combine independently compiled table evidence without semantic merging."""

    usable = [receipt for receipt in receipts if receipt]
    if not usable:
        raise ValueError("B_CORE_TABLE_EVIDENCE_RECEIPT_REQUIRED")
    source = str(usable[0].get("source") or "")
    page = int(usable[0].get("page") or 0)
    if any(
        str(receipt.get("source") or "") != source
        or int(receipt.get("page") or 0) != page
        for receipt in usable
    ):
        raise ValueError("B_CORE_TABLE_EVIDENCE_PROVENANCE_MISMATCH")
    approved = [
        record for receipt in usable for record in receipt.get("approvedEvidence") or []
    ]
    review = [record for receipt in usable for record in receipt.get("reviewQueue") or []]
    return {
        "schema": "B_CORE_NATIVE_OCR_STRUCTURED_TABLE_EVIDENCE_1",
        "source": source,
        "page": page,
        "approvedEvidence": approved,
        "reviewQueue": review,
        "counts": {
            "tables": sum(int((receipt.get("counts") or {}).get("tables") or 0) for receipt in usable),
            "approved": len(approved),
            "needsReview": len(review),
        },
        "decisionCounts": _decision_counts(approved, review),
        "sourceBreakdown": [
            {
                "tables": int((receipt.get("counts") or {}).get("tables") or 0),
                "approved": len(receipt.get("approvedEvidence") or []),
                "needsReview": len(receipt.get("reviewQueue") or []),
            }
            for receipt in usable
        ],
        "contract": usable[0].get("contract"),
    }
