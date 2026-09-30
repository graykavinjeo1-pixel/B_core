"""Vocabulary-independent integrity checks for OCR text before fact promotion."""

from __future__ import annotations

import re


NUMBER_TOKEN = re.compile(r"(?<![A-Za-z0-9])[0-9][0-9,.]*")
NUMERIC_FACT = re.compile(
    r"(?<![A-Za-z0-9])([0-9][0-9,.]*)(?:\s*(mm|cm|m²|m³|m|㎡|㎥|KW|kW|HP|식|대|개|원|%|년))?"
)
SEMANTIC_CHARACTER = re.compile(r"[0-9A-Za-z가-힣]")


def beam_recovery_is_eligible(text: str, confidence: float, language_score: float, language_floor: float) -> bool:
    """Allow beam recovery only when the crop already contains evidence."""

    substantive = len(SEMANTIC_CHARACTER.findall(text)) >= 2
    return substantive and confidence >= 0.55 and language_score < language_floor


def extract_numeric_tokens(text: str) -> list[str]:
    """Return normalized visible numeric tokens without assigning semantics."""

    return [match.group(0).rstrip(".,") for match in NUMBER_TOKEN.finditer(text) if match.group(0).rstrip(".,")]


def extract_numeric_facts(text: str) -> list[str]:
    """Return visible numeric values together with an attached measurement unit.

    Sentence-final punctuation is deliberately excluded while internal decimal,
    date and thousands separators remain.  This gives OCR routing a fact-level
    comparison instead of treating a trailing full stop as a changed value.
    """

    facts: list[str] = []
    unit_aliases = {"m²": "㎡", "m³": "㎥", "KW": "kW"}
    for match in NUMERIC_FACT.finditer(text):
        number = match.group(1).rstrip(".,")
        if not number:
            continue
        unit = unit_aliases.get(match.group(2) or "", match.group(2) or "")
        facts.append(f"{number}{unit}")
    return facts


def validate_ocr_text(text: str) -> list[str]:
    issues: list[str] = []
    if text.count("(") != text.count(")") or text.count("[") != text.count("]"):
        issues.append("UNBALANCED_DELIMITERS")
    if re.search(r"[,.]{2,}|[,][.]|[.][,]", text):
        issues.append("AMBIGUOUS_NUMERIC_PUNCTUATION")
    for match in NUMBER_TOKEN.finditer(text):
        token = match.group(0).rstrip(".,")
        if not token:
            continue
        if token.count(".") > 1:
            issues.append("MULTIPLE_DECIMAL_POINTS")
            break
        if "." in token and "," in token and token.index(".") < token.index(","):
            issues.append("DECIMAL_BEFORE_GROUP_SEPARATOR")
            break
        whole = token.split(".", 1)[0]
        if "," in whole:
            groups = whole.split(",")
            if not 1 <= len(groups[0]) <= 3 or any(len(group) != 3 for group in groups[1:]):
                issues.append("INVALID_THOUSANDS_GROUPING")
                break
    return sorted(set(issues))
