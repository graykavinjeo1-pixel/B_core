"""Recover repeated numeric tables that intentionally have no vertical rules."""

from __future__ import annotations

from collections import Counter
from decimal import Decimal, InvalidOperation
from itertools import combinations, product
import re

from .text_validation import extract_numeric_tokens, validate_ocr_text


GROUPING_PUNCTUATION = re.compile(r"^\d{1,3}(?:[.,]\d{3})+$")
COMPLETE_NUMERIC_ROW = re.compile(r"^[0-9][0-9\s,.-]*$")
REPAIRABLE_NUMERIC_PUNCTUATION_ISSUES = {
    "AMBIGUOUS_NUMERIC_PUNCTUATION",
    "DECIMAL_BEFORE_GROUP_SEPARATOR",
    "INVALID_THOUSANDS_GROUPING",
    "MULTIPLE_DECIMAL_POINTS",
}


def _plain_integer(token: str) -> int | None:
    normalized = token.replace(",", "")
    return int(normalized) if normalized.isdigit() else None


def _number(token: str) -> Decimal | None:
    try:
        return Decimal(token.rstrip(".,").replace(",", ""))
    except InvalidOperation:
        return None


def _relation_indexes(relation: dict) -> tuple[int, ...]:
    indexes = [int(relation["left"]), int(relation["right"])]
    if relation.get("kind") == "percent_product":
        indexes.append(int(relation["rate"]))
    indexes.append(int(relation["result"]))
    return tuple(indexes)


def _relation_expected(values: list[Decimal | None], relation: dict) -> Decimal | None:
    indexes = _relation_indexes(relation)
    if any(index >= len(values) or values[index] is None for index in indexes):
        return None
    left = values[int(relation["left"])]
    right = values[int(relation["right"])]
    assert left is not None and right is not None
    kind = relation.get("kind", "add")
    if kind == "add":
        return left + right
    if kind == "multiply":
        return left * right
    if kind == "percent_product":
        rate = values[int(relation["rate"])]
        assert rate is not None
        return left * right * rate / Decimal("100")
    raise ValueError("B_CORE_NATIVE_OCR_ARITHMETIC_RELATION_KIND_INVALID")


def _relation_holds(values: list[Decimal | None], relation: dict) -> bool:
    expected = _relation_expected(values, relation)
    result_index = int(relation["result"])
    return (
        expected is not None
        and result_index < len(values)
        and values[result_index] is not None
        and expected == values[result_index]
    )


def _arithmetic_holds(tokens: list[str], relations: list[dict]) -> bool:
    if any(max(_relation_indexes(relation)) >= len(tokens) for relation in relations):
        return False
    values = [_number(token) for token in tokens]
    for relation in relations:
        if not _relation_holds(values, relation):
            return False
    return bool(relations)


def arithmetic_relations_hold(tokens: list[str], relations: list[dict]) -> bool:
    """Public read-only predicate for target-free candidate rankers."""

    return _arithmetic_holds(tokens, relations)


def discover_arithmetic_relations(
    rows: list[dict],
    expected_count: int,
    *,
    first_value_column: int = 2,
) -> list[dict]:
    """Learn exact row equations without assigning domain field names.

    Addition keeps the historical tolerant gate because it predates this
    solver. Multiplication and percentage-product relations are admitted only
    when at least three rows support the equation with no counterexample. This
    prevents a coincidental row from becoming a repair rule.
    """

    parsed_rows = [
        [_number(str(token)) for token in row.get("tokens") or []]
        for row in rows
        if len(row.get("tokens") or []) == expected_count
    ]
    relations: list[dict] = []
    value_indexes = range(first_value_column, expected_count)
    for left, right in combinations(value_indexes, 2):
        for result in range(right + 1, expected_count):
            eligible = [
                values
                for values in parsed_rows
                if all(values[index] is not None for index in (left, right, result))
            ]
            additive_support = sum(
                values[left] + values[right] == values[result]
                for values in eligible
            )
            if len(eligible) >= 2 and additive_support / len(eligible) >= 0.75:
                relations.append(
                    {
                        "kind": "add",
                        "left": left,
                        "right": right,
                        "result": result,
                        "support": additive_support,
                        "eligible": len(eligible),
                    }
                )
            nondegenerate = [
                values
                for values in eligible
                if values[left] not in {Decimal("0"), Decimal("1")}
                and values[right] not in {Decimal("0"), Decimal("1")}
            ]
            multiplicative_support = sum(
                values[left] * values[right] == values[result]
                for values in nondegenerate
            )
            multiplicative_failures = len(nondegenerate) - multiplicative_support
            if (
                multiplicative_support >= 3
                and multiplicative_failures <= 1
                and multiplicative_support / len(nondegenerate) >= 0.75
            ):
                relations.append(
                    {
                        "kind": "multiply",
                        "left": left,
                        "right": right,
                        "result": result,
                        "support": multiplicative_support,
                        "eligible": len(nondegenerate),
                    }
                )

    # Quantity × unit price × rate / 100 = amount is represented as a generic
    # four-column equation.  The rate column is inferred from values alone;
    # no Korean header or apartment-specific vocabulary is inspected.
    for left, right in combinations(value_indexes, 2):
        for rate in value_indexes:
            if rate in {left, right}:
                continue
            for result in value_indexes:
                if result in {left, right, rate}:
                    continue
                eligible = [
                    values
                    for values in parsed_rows
                    if all(values[index] is not None for index in (left, right, rate, result))
                    and Decimal("0") < values[rate] <= Decimal("100")
                    and values[rate] != Decimal("100")
                    and values[left] not in {Decimal("0"), Decimal("1")}
                    and values[right] not in {Decimal("0"), Decimal("1")}
                ]
                support = sum(
                    values[left] * values[right] * values[rate] / Decimal("100")
                    == values[result]
                    for values in eligible
                )
                failures = len(eligible) - support
                if support >= 3 and failures <= 1 and support / len(eligible) >= 0.75:
                    relations.append(
                        {
                            "kind": "percent_product",
                            "left": left,
                            "right": right,
                            "rate": rate,
                            "result": result,
                            "support": support,
                            "eligible": len(eligible),
                        }
                    )
    unique = {
        (
            relation["kind"],
            relation["left"],
            relation["right"],
            relation.get("rate"),
            relation["result"],
        ): relation
        for relation in relations
    }
    return sorted(
        unique.values(),
        key=lambda item: (
            item["result"],
            item["kind"],
            item["left"],
            item["right"],
            item.get("rate", -1),
        ),
    )


def _candidate_relation_holds(
    candidate_row: list[list[str]],
    *,
    kind: str,
    left: int,
    right: int,
    result: int,
    rate: int | None = None,
) -> bool:
    indexes = [left, right, result] + ([rate] if rate is not None else [])
    if any(index is None or index >= len(candidate_row) for index in indexes):
        return False
    values = []
    for index in indexes:
        parsed = {
            number
            for token in candidate_row[int(index)]
            if (number := _number(token)) is not None
        }
        if not parsed:
            return False
        values.append(parsed)
    left_values, right_values, result_values = values[:3]
    if kind == "add":
        return any(
            left_value + right_value in result_values
            for left_value in left_values
            for right_value in right_values
        )
    if kind == "multiply":
        return any(
            left_value not in {Decimal("0"), Decimal("1")}
            and right_value not in {Decimal("0"), Decimal("1")}
            and left_value * right_value in result_values
            for left_value in left_values
            for right_value in right_values
        )
    if kind == "percent_product":
        rate_values = values[3]
        return any(
            left_value not in {Decimal("0"), Decimal("1")}
            and right_value not in {Decimal("0"), Decimal("1")}
            and Decimal("0") < rate_value < Decimal("100")
            and left_value * right_value * rate_value / Decimal("100")
            in result_values
            for left_value in left_values
            for right_value in right_values
            for rate_value in rate_values
        )
    raise ValueError("B_CORE_NATIVE_OCR_CANDIDATE_RELATION_KIND_INVALID")


def discover_candidate_lattice_relations(
    candidate_rows: list[list[list[str]]],
    expected_count: int,
    *,
    first_value_column: int = 0,
    allowed_kinds: tuple[str, ...] = ("multiply", "percent_product"),
    minimum_support: int = 3,
    maximum_failures: int = 1,
    minimum_support_ratio: float = 0.75,
) -> list[dict]:
    """Discover generic arithmetic from visual N-best values without targets.

    Candidate sets are collapsed to numeric sets before hypothesis testing, so
    discovery grows polynomially with the number of columns rather than as a
    Cartesian product of every OCR string.  This stage creates hypotheses only;
    a value is materialized later only when the owned lattice has one solution.
    """

    if expected_count < 3 or len(candidate_rows) < minimum_support:
        return []
    eligible_columns = [
        column
        for column in range(first_value_column, expected_count)
        if sum(column < len(row) and bool(row[column]) for row in candidate_rows)
        >= minimum_support
    ]
    relations: list[dict] = []

    def add_if_supported(kind: str, left: int, right: int, result: int, rate=None):
        indexes = (left, right, result) if rate is None else (left, right, rate, result)
        eligible = [
            row
            for row in candidate_rows
            if all(index < len(row) and bool(row[index]) for index in indexes)
        ]
        support = sum(
            _candidate_relation_holds(
                row,
                kind=kind,
                left=left,
                right=right,
                result=result,
                rate=rate,
            )
            for row in eligible
        )
        failures = len(eligible) - support
        if (
            support >= minimum_support
            and failures <= maximum_failures
            and support / max(1, len(eligible)) >= minimum_support_ratio
        ):
            relation = {
                "kind": kind,
                "left": left,
                "right": right,
                "result": result,
                "support": support,
                "eligible": len(eligible),
                "evidence": "owned_visual_candidate_lattice",
            }
            if rate is not None:
                relation["rate"] = rate
            relations.append(relation)

    for left, right in combinations(eligible_columns, 2):
        for result in eligible_columns:
            if result in {left, right}:
                continue
            for kind in ("add", "multiply"):
                if kind in allowed_kinds:
                    add_if_supported(kind, left, right, result)
    if "percent_product" in allowed_kinds:
        for left, right in combinations(eligible_columns, 2):
            for rate in eligible_columns:
                if rate in {left, right}:
                    continue
                for result in eligible_columns:
                    if result not in {left, right, rate}:
                        add_if_supported(
                            "percent_product", left, right, result, rate
                        )
    unique = {
        (
            relation["kind"],
            relation["left"],
            relation["right"],
            relation.get("rate"),
            relation["result"],
        ): relation
        for relation in relations
    }
    return sorted(
        unique.values(),
        key=lambda item: (
            item["result"],
            item["kind"],
            item["left"],
            item["right"],
            item.get("rate", -1),
        ),
    )


def _edit_distance(left: str, right: str) -> int:
    previous = list(range(len(right) + 1))
    for left_index, left_character in enumerate(left, start=1):
        current = [left_index]
        for right_index, right_character in enumerate(right, start=1):
            current.append(
                min(
                    current[-1] + 1,
                    previous[right_index] + 1,
                    previous[right_index - 1] + (left_character != right_character),
                )
            )
        previous = current
    return previous[-1]


def repair_grouping_punctuation_by_arithmetic(
    tokens: list[str], relations: list[dict]
) -> tuple[list[str], list[dict]]:
    """Repair dot/comma ambiguity only when the table arithmetic proves it."""

    if not relations or any(max(_relation_indexes(relation)) >= len(tokens) for relation in relations):
        return tokens, []
    candidate_sets = []
    for token in tokens:
        alternatives = [token]
        if GROUPING_PUNCTUATION.fullmatch(token):
            canonical_grouping = token.replace(".", ",")
            if canonical_grouping != token:
                alternatives.append(canonical_grouping)
        candidate_sets.append(alternatives)
    passing = {
        tuple(candidate)
        for candidate in product(*candidate_sets)
        if tuple(candidate) != tuple(tokens) and _arithmetic_holds(list(candidate), relations)
    }
    if len(passing) != 1:
        return tokens, []
    repaired = list(next(iter(passing)))
    changes = [
        {"index": index, "before": before, "after": after, "proof": "table_arithmetic"}
        for index, (before, after) in enumerate(zip(tokens, repaired))
        if before != after
    ]
    return repaired, changes


def repair_single_numeric_token_by_arithmetic_near_match(
    tokens: list[str], relations: list[dict]
) -> tuple[list[str], list[dict]]:
    """Repair one near-miss digit only when every learned relation proves it.

    This is deliberately narrower than generic arithmetic completion: it never
    invents a missing column and accepts only a unique one-edit correction of
    an already visible numeric token.
    """

    if not relations or _arithmetic_holds(tokens, relations):
        return tokens, []
    values = [_number(token) for token in tokens]
    passing: set[tuple[str, ...]] = set()
    for relation in relations:
        left = int(relation["left"])
        right = int(relation["right"])
        result = int(relation["result"])
        if max(_relation_indexes(relation)) >= len(tokens):
            continue
        kind = relation.get("kind", "add")
        if kind == "add":
            proposals = (
                (left, values[result] - values[right] if values[result] is not None and values[right] is not None else None),
                (right, values[result] - values[left] if values[result] is not None and values[left] is not None else None),
                (result, values[left] + values[right] if values[left] is not None and values[right] is not None else None),
            )
        elif kind == "multiply":
            proposals = (
                (left, values[result] / values[right] if values[result] is not None and values[right] not in {None, Decimal("0")} else None),
                (right, values[result] / values[left] if values[result] is not None and values[left] not in {None, Decimal("0")} else None),
                (result, values[left] * values[right] if values[left] is not None and values[right] is not None else None),
            )
        elif kind == "percent_product":
            rate = int(relation["rate"])
            denominator_left = (
                values[right] * values[rate]
                if values[right] is not None and values[rate] is not None
                else None
            )
            denominator_right = (
                values[left] * values[rate]
                if values[left] is not None and values[rate] is not None
                else None
            )
            denominator_rate = (
                values[left] * values[right]
                if values[left] is not None and values[right] is not None
                else None
            )
            proposals = (
                (left, values[result] * 100 / denominator_left if values[result] is not None and denominator_left not in {None, Decimal("0")} else None),
                (right, values[result] * 100 / denominator_right if values[result] is not None and denominator_right not in {None, Decimal("0")} else None),
                (rate, values[result] * 100 / denominator_rate if values[result] is not None and denominator_rate not in {None, Decimal("0")} else None),
                (result, values[left] * values[right] * values[rate] / 100 if values[left] is not None and values[right] is not None and values[rate] is not None else None),
            )
        else:
            raise ValueError("B_CORE_NATIVE_OCR_ARITHMETIC_RELATION_KIND_INVALID")
        for index, proposed in proposals:
            if proposed is None or proposed < 0 or proposed != proposed.to_integral_value():
                continue
            replacement = f"{int(proposed):,}" if re.search(r"[.,]", tokens[index]) else str(int(proposed))
            visible_digits = re.sub(r"\D", "", tokens[index])
            replacement_digits = re.sub(r"\D", "", replacement)
            if _edit_distance(visible_digits, replacement_digits) != 1:
                continue
            candidate = list(tokens)
            candidate[index] = replacement
            if _arithmetic_holds(candidate, relations):
                passing.add(tuple(candidate))
    if len(passing) != 1:
        return tokens, []
    repaired = list(next(iter(passing)))
    changes = [
        {
            "index": index,
            "before": before,
            "after": after,
            "proof": "table_arithmetic_unique_one_edit",
        }
        for index, (before, after) in enumerate(zip(tokens, repaired))
        if before != after
    ]
    return repaired, changes


def select_unique_arithmetic_candidate_tuple(
    candidate_sets: list[list[str]],
    relations: list[dict],
    *,
    maximum_candidates_per_cell: int = 8,
) -> tuple[list[str] | None, dict]:
    """Lazily select one visual candidate tuple proven by table arithmetic.

    Candidate strings must already exist in B_Core-owned visual N-best output.
    The first candidate in each cell is the fast path.  A Cartesian search is
    performed only when that tuple violates a learned equation, and selection
    succeeds only when exactly one distinct tuple satisfies every relation.
    """

    bounded = []
    for values in candidate_sets:
        if not values:
            bounded.append([])
            continue
        nonempty = list(dict.fromkeys(str(value) for value in values if str(value)))
        # Non-numeric columns outside the discovered relation remain fixed
        # empty placeholders.  They do not participate in the equation.
        bounded.append((nonempty or [""])[:maximum_candidates_per_cell])
    receipt = {
        "schema": "B_CORE_NATIVE_OCR_LAZY_ARITHMETIC_TUPLE_1",
        "selectionUsesTarget": False,
        "externalOcrInvocations": 0,
        "maximumCandidatesPerCell": maximum_candidates_per_cell,
        "fastPath": False,
        "searchedCombinations": 0,
        "satisfyingTuples": 0,
        "status": "not_applicable",
    }
    if not relations or not bounded or any(not values for values in bounded):
        return None, receipt
    primary = [values[0] for values in bounded]
    if _arithmetic_holds(primary, relations):
        receipt.update({"fastPath": True, "status": "primary_satisfied", "satisfyingTuples": 1})
        return primary, receipt
    passing: set[tuple[str, ...]] = set()
    for candidate in product(*bounded):
        receipt["searchedCombinations"] += 1
        if _arithmetic_holds(list(candidate), relations):
            passing.add(tuple(candidate))
            if len(passing) > 1:
                # Uniqueness is already disproven; do not spend the remaining
                # product space merely to count ambiguous alternatives.
                break
    receipt["satisfyingTuples"] = len(passing)
    if len(passing) != 1:
        receipt["status"] = "ambiguous" if passing else "unsatisfied"
        return None, receipt
    receipt["status"] = "unique_visual_tuple"
    return list(next(iter(passing))), receipt


def _schedule_signature(text: str) -> tuple[int, int, list[str]] | None:
    tokens = extract_numeric_tokens(text)
    if len(tokens) < 4:
        return None
    ordinal = _plain_integer(tokens[0])
    year = _plain_integer(tokens[1])
    if ordinal is None or year is None or not 1900 <= year <= 2200:
        return None
    return ordinal, year, tokens


def infer_numeric_column_contracts(
    expected_count: int,
    rows: list[dict],
    arithmetic_relations: list[dict],
) -> list[dict]:
    """Describe observed numeric structure without assigning domain names."""

    complete = [
        [str(token) for token in row.get("tokens") or []]
        for row in rows
        if len(row.get("tokens") or []) == expected_count
    ]
    if expected_count <= 0 or len(complete) < 2:
        return []
    contracts = []
    for index in range(expected_count):
        tokens = [row[index] for row in complete]
        integers = [_plain_integer(token.rstrip(".,")) for token in tokens]
        value_type = (
            "grouped_integer"
            if sum("," in token for token in tokens) >= max(1, len(tokens) // 2)
            else "integer"
        )
        contracts.append(
            {
                "index": index,
                "valueType": value_type,
                "structuralRoles": [],
                "evidence": [],
            }
        )
        if all(value is not None for value in integers):
            if all(left < right for left, right in zip(integers, integers[1:])):
                contracts[index]["evidence"].append("strictly_increasing")
            if all(1900 <= value <= 2200 for value in integers):
                contracts[index]["evidence"].append("calendar_year_range")
    if (
        expected_count >= 2
        and "strictly_increasing" in contracts[0]["evidence"]
    ):
        contracts[0]["structuralRoles"].append("sequence_ordinal")
    if (
        expected_count >= 2
        and "strictly_increasing" in contracts[1]["evidence"]
        and "calendar_year_range" in contracts[1]["evidence"]
    ):
        contracts[1]["structuralRoles"].append("calendar_year")
    for relation_index, relation in enumerate(arithmetic_relations):
        kind = relation.get("kind", "add")
        group = f"{kind}_relation_{relation_index}"
        role_indexes = [
            ("operand", int(relation["left"])),
            ("operand", int(relation["right"])),
        ]
        if kind == "percent_product":
            role_indexes.append(("percentage_rate", int(relation["rate"])))
        role_indexes.append(("result", int(relation["result"])))
        for role, index in role_indexes:
            if 0 <= index < len(contracts):
                contracts[index]["structuralRoles"].append(f"{group}:{role}")
                # Preserve the original additive vocabulary for downstream
                # consumers while exposing the generic operand/result roles.
                if kind == "add" and role == "operand":
                    contracts[index]["structuralRoles"].append(f"{group}:addend")
                if kind == "add" and role == "result":
                    contracts[index]["structuralRoles"].append(f"{group}:sum")
    return contracts


def _row_candidates(row: dict) -> list[dict]:
    candidates = []
    readings = [
        ("direct_row", "directText", "directConfidence"),
        ("joined_regions", "regionJoinedText", "regionJoinedConfidence"),
        ("ctc_beam", "beamText", "beamConfidence"),
    ]
    observed = []
    for mode, text_key, confidence_key in readings:
        observed.append(
            (mode, str(row.get(text_key) or "").strip(), float(row.get(confidence_key) or 0.0))
        )
    for index, candidate in enumerate(row.get("beamCandidates") or []):
        observed.append(
            (
                f"ctc_nbest_{index}",
                str(candidate.get("text") or "").strip(),
                float(candidate.get("confidence") or 0.0),
            )
        )
    seen = set()
    for mode, text, confidence in observed:
        if not text or text in seen:
            continue
        seen.add(text)
        signature = _schedule_signature(text)
        issues = validate_ocr_text(text)
        if not COMPLETE_NUMERIC_ROW.fullmatch(text) or signature is None or any(
            issue not in REPAIRABLE_NUMERIC_PUNCTUATION_ISSUES for issue in issues
        ):
            continue
        candidates.append(
            {
                "mode": mode,
                "text": text,
                "confidence": confidence,
                "ordinal": signature[0],
                "year": signature[1],
                "tokens": signature[2],
                "textValidationIssues": issues,
            }
        )
    return candidates


def _numeric_row_surface_rank(candidate: dict) -> tuple[int, int, float]:
    """Rank equal numeric facts by structural surface integrity only."""

    text = str(candidate.get("text") or "").strip()
    dangling_punctuation = len(re.findall(r"(?<=\d)[.,](?=\s|$)", text))
    return (-dangling_punctuation, -len(text), float(candidate.get("confidence") or 0.0))


def reconstruct_repeated_numeric_tables(rows: list[dict]) -> list[dict]:
    """Build schedule-like tables from repeated row structure, not vocabulary.

    A row qualifies only when independent OCR candidates agree that its first
    two numeric values are an ordinal and a calendar year.  Groups split when
    the ordinal/year sequence restarts or the visual gap is too large.  Within
    each group, the modal token count can recover a small value omitted by one
    segmentation path without inventing a value.
    """

    qualified = []
    for index, row in enumerate(rows):
        candidates = _row_candidates(row)
        if not candidates:
            continue
        representative = max(candidates, key=lambda item: (len(item["tokens"]), item["confidence"]))
        qualified.append(
            {
                "index": index,
                "row": row,
                "candidates": candidates,
                "ordinal": representative["ordinal"],
                "year": representative["year"],
            }
        )

    groups: list[list[dict]] = []
    for item in qualified:
        if not groups:
            groups.append([item])
            continue
        previous = groups[-1][-1]
        vertical_gap = int(item["row"]["y0"]) - int(previous["row"]["y1"])
        sequence_restarted = (
            item["ordinal"] <= previous["ordinal"]
            or item["year"] <= previous["year"]
        )
        if vertical_gap > 150 or sequence_restarted:
            groups.append([item])
        else:
            groups[-1].append(item)

    tables = []
    for group in groups:
        if len(group) < 2:
            continue
        token_counts = [
            len(candidate["tokens"])
            for item in group
            for candidate in item["candidates"]
        ]
        expected_count = Counter(token_counts).most_common(1)[0][0]
        structured_rows = []
        for item in group:
            same_identity = [
                candidate
                for candidate in item["candidates"]
                if candidate["ordinal"] == item["ordinal"]
                and candidate["year"] == item["year"]
            ]
            exact_count = [
                candidate
                for candidate in same_identity
                if len(candidate["tokens"]) == expected_count
            ]
            selected = max(
                exact_count or same_identity,
                key=lambda candidate: (
                    len(candidate["tokens"]) == expected_count,
                    not candidate["textValidationIssues"],
                    candidate["confidence"],
                ),
            )
            row = item["row"]
            previous_text = str(row.get("text") or "")
            if selected["text"] != previous_text:
                row["preStructuralText"] = previous_text
                row["text"] = selected["text"]
                row["confidence"] = selected["confidence"]
                row["selectedReading"] = selected["mode"]
                row["structuralSelection"] = "repeated-numeric-token-count-consensus"
                row["numericTokens"] = selected["tokens"]
            structured_rows.append(
                {
                    "sourceRowIndex": item["index"],
                    "y0": int(row["y0"]),
                    "y1": int(row["y1"]),
                    "text": row["text"],
                    "tokens": selected["tokens"],
                    "confidence": float(row.get("confidence") or selected["confidence"]),
                    "selectedReading": row.get("selectedReading"),
                    "sourceValidationIssues": list(row.get("validationIssues") or []),
                    "numericMultiviewAudit": row.get("numericMultiviewAudit"),
                }
            )
        complete_rows = [
            row for row in structured_rows if len(row["tokens"]) == expected_count
        ]
        arithmetic_relations = discover_arithmetic_relations(
            complete_rows,
            expected_count,
        )
        candidate_rows = []
        for item in group:
            token_sets = [[] for _ in range(expected_count)]
            for candidate in item["candidates"]:
                if len(candidate["tokens"]) != expected_count:
                    continue
                for column, token in enumerate(candidate["tokens"]):
                    if token not in token_sets[column]:
                        token_sets[column].append(token)
            candidate_rows.append(token_sets)
        lattice_relations = discover_candidate_lattice_relations(
            candidate_rows,
            expected_count,
            first_value_column=2,
            allowed_kinds=("add", "multiply", "percent_product"),
            minimum_support_ratio=0.9,
        )
        relation_by_key = {
            (
                relation["kind"], relation["left"], relation["right"],
                relation.get("rate"), relation["result"],
            ): relation
            for relation in arithmetic_relations
        }
        for relation in lattice_relations:
            relation_by_key.setdefault(
                (
                    relation["kind"], relation["left"], relation["right"],
                    relation.get("rate"), relation["result"],
                ),
                relation,
            )
        arithmetic_relations = list(relation_by_key.values())
        lattice_selections = 0
        lattice_ambiguous = 0
        lattice_unsatisfied = 0
        for row in structured_rows:
            # Rank complete whole-row visual candidates by the learned table
            # relation.  A distinct numeric tuple is selected only when it is
            # the sole satisfying tuple in the owned lattice.
            item = next(item for item in group if item["index"] == row["sourceRowIndex"])
            satisfying: dict[tuple[str, ...], dict] = {}
            for candidate in item["candidates"]:
                tokens = list(candidate["tokens"])
                if len(tokens) != expected_count or not _arithmetic_holds(
                    tokens, arithmetic_relations
                ):
                    continue
                key = tuple(tokens)
                current = satisfying.get(key)
                if current is None or _numeric_row_surface_rank(
                    candidate
                ) > _numeric_row_surface_rank(current):
                    satisfying[key] = candidate
            primary_holds = _arithmetic_holds(
                list(row["tokens"]), arithmetic_relations
            )
            if primary_holds:
                row["candidateLatticeDecision"] = "primary_satisfied"
            else:
                if len(satisfying) == 1:
                    selected_candidate = next(iter(satisfying.values()))
                    row["preCandidateLatticeText"] = row["text"]
                    row["text"] = selected_candidate["text"]
                    row["tokens"] = list(selected_candidate["tokens"])
                    row["confidence"] = selected_candidate["confidence"]
                    row["selectedReading"] = selected_candidate["mode"]
                    row["structuralSelection"] = "candidate-lattice-arithmetic-unique"
                    row["candidateLatticeDecision"] = "unique_visual_tuple"
                    lattice_selections += 1
                elif len(satisfying) > 1:
                    row["candidateLatticeDecision"] = "ambiguous"
                    lattice_ambiguous += 1
                else:
                    row["candidateLatticeDecision"] = "unsatisfied"
                    lattice_unsatisfied += 1
            repaired_tokens, repairs = repair_grouping_punctuation_by_arithmetic(
                list(row["tokens"]), arithmetic_relations
            )
            if repairs:
                repaired_text = str(row["text"])
                for repair in repairs:
                    repaired_text = repaired_text.replace(
                        str(repair["before"]), str(repair["after"]), 1
                    )
                row["preArithmeticRepairText"] = row["text"]
                row["text"] = repaired_text
                row["tokens"] = repaired_tokens
                row["structuralRepairs"] = repairs
            visible_text = str(row["text"]).strip()
            normalized_surface = re.sub(r"(?<=\d)\.(?=\s|$)", "", visible_text)
            if (
                COMPLETE_NUMERIC_ROW.fullmatch(visible_text)
                and normalized_surface != visible_text
                and _arithmetic_holds(list(row["tokens"]), arithmetic_relations)
            ):
                # A dot not followed by a fractional digit is not part of a
                # numeric fact. Remove it only after the table equation has
                # independently validated the complete numeric tuple.
                row["preNumericSurfaceNormalizationText"] = row["text"]
                row["text"] = normalized_surface
                row["tokens"] = [
                    token[:-1] if str(token).endswith(".") else token
                    for token in row["tokens"]
                ]
                row.setdefault("structuralRepairs", []).append(
                    {
                        "index": len(row["tokens"]) - 1,
                        "before": visible_text,
                        "after": row["text"],
                        "proof": "typed_numeric_row_terminal_punctuation",
                    }
                )
            issues = list(row.get("sourceValidationIssues") or [])
            if row.get("structuralSelection") or row.get("structuralRepairs"):
                issues = [
                    issue
                    for issue in issues
                    if issue not in REPAIRABLE_NUMERIC_PUNCTUATION_ISSUES
                ]
            if len(row["tokens"]) != expected_count:
                issues.append("STRUCTURAL_TOKEN_COUNT_MISMATCH")
            else:
                for relation in arithmetic_relations:
                    values = [_number(token) for token in row["tokens"]]
                    if any(values[index] is None for index in _relation_indexes(relation)):
                        issues.append("STRUCTURAL_NUMBER_PARSE_FAILURE")
                        continue
                    if not _relation_holds(values, relation):
                        issues.append("STRUCTURAL_ARITHMETIC_MISMATCH")
            issues.extend(
                f"STRUCTURAL_TEXT_{issue}"
                for issue in validate_ocr_text(str(row["text"]))
            )
            row["validationIssues"] = sorted(set(issues))
            row["status"] = "accepted" if not row["validationIssues"] else "needs_review"

        column_contracts = infer_numeric_column_contracts(
            expected_count,
            structured_rows,
            arithmetic_relations,
        )
        tables.append(
            {
                "schema": "B_CORE_NATIVE_OCR_BORDERLESS_NUMERIC_TABLE_1",
                "expectedTokenCount": expected_count,
                "arithmeticRelations": arithmetic_relations,
                "candidateLatticeRelations": lattice_relations,
                "candidateLatticeStats": {
                    "rows": len(structured_rows),
                    "relationHypotheses": len(lattice_relations),
                    "uniqueSelections": lattice_selections,
                    "ambiguousRows": lattice_ambiguous,
                    "unsatisfiedRows": lattice_unsatisfied,
                    "selectionUsesTarget": False,
                    "externalOcrInvocations": 0,
                },
                "columnContracts": column_contracts,
                "rows": structured_rows,
                "y0": min(row["y0"] for row in structured_rows),
                "y1": max(row["y1"] for row in structured_rows),
            }
        )
    return tables


def arithmetic_candidate_expansion_indices(tables: list[dict]) -> list[int]:
    """Return only rows whose established table equation still fails.

    This is the sparse wake-up gate for a wider visual candidate search.  It
    depends solely on a generic structural validation issue produced by the
    first pass; document names, headers and target transcripts are not read.
    """

    return sorted(
        {
            int(row["sourceRowIndex"])
            for table in tables
            for row in table.get("rows") or []
            if row.get("sourceRowIndex") is not None
            and row.get("candidateLatticeDecision") in {"ambiguous", "unsatisfied"}
        }
    )


def synchronize_reconstructed_rows(
    text_rows: list[dict], tables: list[dict]
) -> list[dict]:
    """Commit proven table-row selections to the page observation stream.

    The raw row is retained in ``preStructuredSelectionText``.  This prevents
    the page-level evidence compiler from publishing a stale pre-CSP reading
    while the structured table receipt contains the corrected visual tuple.
    """

    synchronized: list[dict] = []
    for table in tables:
        for structured_row in table.get("rows") or []:
            source_index = structured_row.get("sourceRowIndex")
            if source_index is None or not 0 <= int(source_index) < len(text_rows):
                continue
            source_row = text_rows[int(source_index)]
            selected_text = str(structured_row.get("text") or "")
            if selected_text and selected_text != str(source_row.get("text") or ""):
                source_row.setdefault(
                    "preStructuredSelectionText", str(source_row.get("text") or "")
                )
                source_row["text"] = selected_text
            for key in (
                "tokens",
                "validationIssues",
                "status",
                "structuralRepairs",
                "structuralSelection",
                "candidateLatticeDecision",
            ):
                if key in structured_row:
                    source_row[key] = structured_row[key]
            source_row["accepted"] = source_row.get("status") == "accepted"
            source_row["numericTokens"] = extract_numeric_tokens(
                str(source_row.get("text") or "")
            )
            synchronized.append(
                {
                    "sourceRowIndex": int(source_index),
                    "text": str(source_row.get("text") or ""),
                    "status": source_row.get("status"),
                }
            )
    return synchronized
