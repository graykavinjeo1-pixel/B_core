"""Sparse arithmetic correction for already localized table-cell readings.

The solver learns repeated equations from the current table only.  It never
uses a Korean header, apartment name, expected answer, or fixed column number.
An OCR value changes only when a one-character visual near-match is the unique
assignment satisfying every independently learned compatible equation.
"""

from __future__ import annotations

from copy import deepcopy
from collections import defaultdict

from .borderless_table import (
    discover_candidate_lattice_relations,
    discover_arithmetic_relations,
    select_unique_arithmetic_candidate_tuple,
)
from .text_validation import extract_numeric_tokens


def _table_key(record: dict) -> tuple[str, int, int] | None:
    document = record.get("documentSha256") or record.get("sourceName")
    page = record.get("page")
    table = record.get("tableIndex")
    if document is None or page is None or table is None:
        return None
    return str(document), int(page), int(table)


def _single_numeric_token(text: str) -> str | None:
    tokens = extract_numeric_tokens(text)
    return tokens[0] if len(tokens) == 1 else None


def _owned_numeric_candidates(observation: dict, prediction_field: str) -> list[str]:
    values = [str(observation.get(prediction_field) or "")]
    for candidate in observation.get("visualCandidates") or observation.get("candidates") or []:
        text = str(candidate.get("text") if isinstance(candidate, dict) else candidate or "")
        if text:
            values.append(text)
    tokens = []
    for value in values:
        token = _single_numeric_token(value)
        if token is not None and token not in tokens:
            tokens.append(token)
    return tokens[:8]


def apply_table_arithmetic_constraints(
    observations: list[dict],
    *,
    prediction_field: str = "finalPrediction",
) -> tuple[list[dict], dict]:
    """Apply generic row equations to table observations.

    ``observations`` may contain ordinary text and multi-line cells.  Only the
    first line of a cell with exactly one visible numeric token participates.
    All other content is preserved byte-for-byte.
    """

    finalized = deepcopy(observations)
    tables: dict[tuple[str, int, int], list[int]] = defaultdict(list)
    for index, observation in enumerate(finalized):
        record = observation.get("record") or {}
        key = _table_key(record)
        if key is None or int(record.get("lineIndexWithinCell") or 0) != 0:
            continue
        if record.get("rowIndex") is None or record.get("columnIndex") is None:
            continue
        tables[key].append(index)

    table_receipts = []
    total_changes = 0
    for key, indexes in sorted(tables.items()):
        by_row: dict[int, dict[int, int]] = defaultdict(dict)
        maximum_column = -1
        owned_candidates: dict[int, list[str]] = {}
        for index in indexes:
            record = finalized[index]["record"]
            row = int(record["rowIndex"])
            column = int(record["columnIndex"])
            candidates = _owned_numeric_candidates(finalized[index], prediction_field)
            if not candidates:
                continue
            owned_candidates[index] = candidates
            by_row[row][column] = index
            maximum_column = max(maximum_column, column)
        if maximum_column < 2 or len(by_row) < 3:
            continue
        rows = []
        candidate_rows = []
        ordered_rows = []
        for row_index, column_map in sorted(by_row.items()):
            tokens = [""] * (maximum_column + 1)
            candidates_by_column = [[] for _ in range(maximum_column + 1)]
            for column, observation_index in column_map.items():
                token = _single_numeric_token(
                    str(finalized[observation_index].get(prediction_field) or "")
                )
                candidates = owned_candidates[observation_index]
                tokens[column] = token if token is not None else candidates[0]
                candidates_by_column[column] = candidates
            rows.append({"tokens": tokens})
            candidate_rows.append(candidates_by_column)
            ordered_rows.append((row_index, column_map, tokens))
        relations = discover_arithmetic_relations(
            rows,
            maximum_column + 1,
            first_value_column=0,
        )
        # Addition among ordinals or adjacent years can be coincidental.  In a
        # schema-free table, automatic correction is restricted to product
        # equations; additive totals remain available to typed compilers.
        automatic_relations = [
            relation
            for relation in relations
            if relation.get("kind") in {"multiply", "percent_product"}
        ]
        lattice_relations = discover_candidate_lattice_relations(
            candidate_rows,
            maximum_column + 1,
        )
        relation_by_key = {
            (
                relation["kind"],
                relation["left"],
                relation["right"],
                relation.get("rate"),
                relation["result"],
            ): relation
            for relation in automatic_relations
        }
        for relation in lattice_relations:
            relation_by_key.setdefault(
                (
                    relation["kind"],
                    relation["left"],
                    relation["right"],
                    relation.get("rate"),
                    relation["result"],
                ),
                relation,
            )
        automatic_relations = list(relation_by_key.values())
        changes = []
        tuple_status_counts: dict[str, int] = defaultdict(int)
        for row_index, column_map, tokens in ordered_rows:
            compatible = [
                relation
                for relation in automatic_relations
                if all(
                    index < len(tokens) and bool(tokens[index])
                    for index in (
                        int(relation["left"]),
                        int(relation["right"]),
                        *(
                            (int(relation["rate"]),)
                            if relation.get("kind") == "percent_product"
                            else ()
                        ),
                        int(relation["result"]),
                    )
                )
            ]
            if not compatible:
                continue
            candidate_sets = [
                _owned_numeric_candidates(finalized[column_map[column]], prediction_field)
                if column in column_map
                else [token]
                for column, token in enumerate(tokens)
            ]
            tuple_selection, tuple_receipt = select_unique_arithmetic_candidate_tuple(
                candidate_sets,
                compatible,
                maximum_candidates_per_cell=8,
            )
            tuple_status_counts[str(tuple_receipt["status"])] += 1
            if tuple_selection is not None:
                repaired = tuple_selection
                repairs = []
                for column, after in enumerate(repaired):
                    observation_index = column_map.get(column)
                    if observation_index is None:
                        continue
                    source_text = str(
                        finalized[observation_index].get(prediction_field) or ""
                    )
                    source_token = _single_numeric_token(source_text)
                    before = source_token if source_token is not None else source_text
                    if before == after:
                        continue
                    repairs.append(
                        {
                            "index": column,
                            "before": before,
                            "after": after,
                            "proof": (
                                "table_arithmetic_unique_visual_tuple"
                                if tuple_receipt["status"] == "unique_visual_tuple"
                                else "table_arithmetic_visual_numeric_recovery"
                            ),
                        }
                    )
            else:
                repaired, repairs = list(tokens), []
            for repair in repairs:
                column = int(repair["index"])
                observation_index = column_map.get(column)
                if observation_index is None:
                    continue
                observation = finalized[observation_index]
                before_text = str(observation.get(prediction_field) or "")
                before_token = str(repair["before"])
                if before_text.count(before_token) != 1:
                    continue
                after_text = before_text.replace(before_token, str(repair["after"]), 1)
                observation[prediction_field] = after_text
                observation.setdefault("structuralCorrections", []).append(
                    {
                        **repair,
                        "tableKey": list(key),
                        "rowIndex": row_index,
                        "columnIndex": column,
                        "relationKinds": sorted(
                            {relation.get("kind", "add") for relation in compatible}
                        ),
                        "lazyCandidateTuple": tuple_receipt,
                        "targetUsed": False,
                    }
                )
                changes.append(
                    {
                        "rowIndex": row_index,
                        "columnIndex": column,
                        "before": before_text,
                        "after": after_text,
                    }
                )
                total_changes += 1
        table_receipts.append(
            {
                "tableKey": list(key),
                "numericRows": len(by_row),
                "relations": relations,
                "candidateLatticeRelations": lattice_relations,
                "automaticRelationCount": len(automatic_relations),
                "tupleStatusCounts": dict(sorted(tuple_status_counts.items())),
                "changes": changes,
            }
        )
    return finalized, {
        "schema": "B_CORE_NATIVE_OCR_TABLE_CONSTRAINT_FUSION_1",
        "selectionUsesTarget": False,
        "externalOcrInvocations": 0,
        "tablesConsidered": len(tables),
        "tablesWithRelations": sum(
            bool(receipt["relations"]) for receipt in table_receipts
        ),
        "changes": total_changes,
        "tables": table_receipts,
        "contract": (
            "Only a unique one-character visible numeric near-match satisfying "
            "every compatible repeated product equation may change a value."
        ),
    }


def apply_geometry_cell_arithmetic_constraints(
    cells: list[dict],
    *,
    source_key: str,
    page: int,
) -> tuple[list[dict], dict]:
    """Run the same target-free constraint solver on page-geometry cells.

    Page recognition and the standalone evaluator historically used different
    shapes, which meant N-best visual candidates reached the evaluator but not
    the real page path.  This adapter is deliberately mechanical: it copies
    geometry coordinates into the existing generic observation contract, runs
    the shared solver, and copies only its selected text and proof receipt back.
    It does not infer headers, field names, or document-specific semantics.
    """

    observations = []
    for cell in cells:
        observations.append(
            {
                "record": {
                    "sourceName": source_key,
                    "page": int(page),
                    "tableIndex": int(cell.get("tableComponentIndex") or 0),
                    "rowIndex": cell.get("rowIndex"),
                    "columnIndex": cell.get("columnIndex"),
                    "lineIndexWithinCell": 0,
                },
                "finalPrediction": str(cell.get("text") or ""),
                "visualCandidates": list(cell.get("visualCandidates") or []),
            }
        )
    corrected, receipt = apply_table_arithmetic_constraints(
        observations,
        prediction_field="finalPrediction",
    )
    finalized = deepcopy(cells)
    for cell, observation in zip(finalized, corrected):
        cell["text"] = str(observation.get("finalPrediction") or "")
        if observation.get("structuralCorrections"):
            cell.setdefault("structuralCorrections", []).extend(
                observation["structuralCorrections"]
            )
    receipt = {
        **receipt,
        "adapter": "page_geometry_cells",
        "sourceKey": source_key,
        "page": int(page),
    }
    return finalized, receipt
