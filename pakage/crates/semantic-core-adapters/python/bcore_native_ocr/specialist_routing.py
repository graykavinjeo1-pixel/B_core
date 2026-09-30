"""Source-independent routing contract for narrow OCR experts."""

from __future__ import annotations

import re
from collections import Counter
from collections.abc import Mapping

from .text_validation import (
    SEMANTIC_CHARACTER,
    extract_numeric_facts,
    extract_numeric_tokens,
    validate_ocr_text,
)


NUMBER_ONLY = re.compile(r"^[0-9][0-9,.]*$")
MEASUREMENT = re.compile(r"^[0-9][0-9,.]*\s*(?:㎡|㎥|mm|cm|m|KW|kW|HP|%|식|대|개|원|층|동)$")
UNIT_SUFFIX = re.compile(r"(㎡|㎥|mm|cm|m|KW|kW|HP|%|식|대|개|원|층|동)$")
LEADING_STRUCTURE = re.compile(r"^\s*([^0-9A-Za-z가-힣]+)")
VISIBLE_NUMERIC_LITERAL = re.compile(
    r"\d[\d,.]*(?:\s*(?:mm|cm|m²|m³|m|㎡|㎥|KW|kW|HP|식|대|개|원|%|년))?"
)
OPAQUE_ASCII_IDENTIFIER = re.compile(r"^[A-Z0-9][A-Z0-9._/-]*$")
EMBEDDED_ASCII_IDENTIFIER = re.compile(
    r"(?<![A-Za-z0-9])[A-Z][A-Z0-9._/-]{1,}(?![A-Za-z0-9])"
)
ROMAN_HEADING = re.compile(
    r"(?<![A-Za-z])(?=[MDCLXVI]+[.)]?(?![A-Za-z]))"
    r"(?:M{0,4}(?:CM|CD|D?C{0,3})(?:XC|XL|L?X{0,3})(?:IX|IV|V?I{0,3}))[.)]?(?![A-Za-z])"
)
HANGUL_SYLLABLE = re.compile(r"[가-힣]")
HANGUL_WORD = re.compile(r"^[가-힣]{2,12}$")
PARENTHETICAL_SIGNATURE = re.compile(r"\([^()\r\n]{1,64}\)")
ASCII_RUN = re.compile(r"[A-Za-z]{3,}")
URL_SCHEME = re.compile(r"(?i)\bhttps?://")
IDENTIFIER_SEPARATOR = re.compile(r"[@/_\\-]")


def candidate_preserves_known_word(
    anchor_text: str,
    candidate_text: str,
    lexicon_counts: Mapping[str, int] | None,
) -> tuple[bool, str]:
    """Protect a train-only known label from an unseen single-word mutation.

    This is deliberately narrower than general spell correction.  It does not
    invent text and does not inspect the document family or the target.  It
    only fails closed when the anchor is an independently learned Korean word
    while the proposed replacement has no lexical evidence at all.
    """

    if not lexicon_counts:
        return True, "lexicon_not_available"
    anchor = anchor_text.strip()
    candidate = candidate_text.strip()
    if not HANGUL_WORD.fullmatch(anchor) or not HANGUL_WORD.fullmatch(candidate):
        return True, "lexicon_not_applicable"
    if lexicon_counts.get(anchor, 0) >= 2 and lexicon_counts.get(candidate, 0) == 0:
        return False, "known_word_guard"
    return True, "known_word_preserved"


def candidate_preserves_visible_facts(base_text: str, candidate_text: str) -> tuple[bool, str]:
    """Reject target-free recovery candidates that can alter visible facts.

    Language scores may rank two spellings, but they are not evidence that a
    number, punctuation-bearing quantity, or opaque identifier changed.  This
    shared gate keeps beam recovery and specialist routing on the same
    fail-closed integrity contract.
    """

    if extract_numeric_tokens(base_text) != extract_numeric_tokens(candidate_text):
        return False, "numeric_fact_guard"
    if visible_numeric_signature(base_text) != visible_numeric_signature(candidate_text):
        return False, "visible_numeric_signature_guard"
    if OPAQUE_ASCII_IDENTIFIER.fullmatch(base_text.strip()) and HANGUL_SYLLABLE.search(
        candidate_text
    ):
        return False, "identifier_script_guard"
    if ROMAN_HEADING.findall(base_text) != ROMAN_HEADING.findall(candidate_text):
        return False, "roman_heading_guard"
    if EMBEDDED_ASCII_IDENTIFIER.findall(base_text) != EMBEDDED_ASCII_IDENTIFIER.findall(
        candidate_text
    ):
        return False, "embedded_identifier_guard"
    if PARENTHETICAL_SIGNATURE.findall(base_text) != PARENTHETICAL_SIGNATURE.findall(
        candidate_text
    ):
        # Parenthesized model variants, colours and qualifiers are often
        # material facts.  A second recognizer's confidence is not enough
        # evidence to rewrite them.
        return False, "parenthetical_fact_guard"
    base_marker = leading_structure_marker(base_text)
    candidate_marker = leading_structure_marker(candidate_text)
    if base_marker and candidate_marker != base_marker:
        return False, "leading_structure_guard"
    base_words = base_text.strip().split(maxsplit=1)
    candidate_words = candidate_text.strip().split(maxsplit=1)
    if (
        len(base_words) == 2
        and len(base_words[0]) <= 2
        and (
            (
                len(candidate_words) == 2
                and base_words[1] == candidate_words[1]
                and base_words[0] != candidate_words[0]
                and len(candidate_words[0]) <= 2
            )
            or candidate_text.strip() == base_words[1]
        )
    ):
        # A damaged checkbox/bullet is often decoded as a one-syllable word.
        # With no target or layout proof, changing only that leading fragment
        # is not evidence-backed recovery and must fail closed.
        return False, "leading_fragment_guard"
    if candidate_text.strip() and not SEMANTIC_CHARACTER.search(candidate_text):
        return False, "nonsemantic_output_guard"
    if validate_ocr_text(candidate_text):
        return False, "candidate_integrity_guard"
    return True, "visible_facts_preserved"


def visible_numeric_signature(text: str) -> tuple[str, ...]:
    """Preserve visible numeric punctuation while routing without a target."""

    return tuple(VISIBLE_NUMERIC_LITERAL.findall(text))


def leading_structure_marker(text: str) -> str:
    match = LEADING_STRUCTURE.match(text)
    return match.group(1).strip() if match else ""


def disagreement_edits(left: str, right: str) -> int:
    """Measure candidate disagreement without consulting a target transcript."""

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


def specialist_wakeup_eligible(
    anchor_text: str, anchor_confidence: float, confidence_threshold: float
) -> bool:
    """Wake a specialist only for uncertain or structurally invalid output."""

    if not 0.0 <= confidence_threshold <= 1.0:
        raise ValueError("B_CORE_NATIVE_OCR_SPECIALIST_WAKE_THRESHOLD_INVALID")
    return anchor_confidence < confidence_threshold or bool(
        validate_ocr_text(anchor_text)
    )


def mixed_script_specialist_wakeup_eligible(
    anchor_text: str,
    anchor_confidence: float,
    confidence_threshold: float,
    *,
    minimum_ascii_run: int = 3,
    special_character_ratio: float = 0.20,
) -> bool:
    """Wake a mixed-script expert from generic observable surface structure.

    No document name, target transcript or domain phrase participates.  The
    route is activated by script transitions, identifier punctuation, dense
    symbols, or the same confidence/integrity fallback used by the base sparse
    router.  ``minimum_ascii_run`` remains explicit for reproducible sweeps.
    """

    if not 0.0 <= confidence_threshold <= 1.0:
        raise ValueError("B_CORE_NATIVE_OCR_SPECIALIST_WAKE_THRESHOLD_INVALID")
    if minimum_ascii_run < 1 or not 0.0 <= special_character_ratio <= 1.0:
        raise ValueError("B_CORE_NATIVE_OCR_MIXED_WAKE_CONFIGURATION_INVALID")
    compact = "".join(character for character in anchor_text if not character.isspace())
    special_count = sum(not character.isalnum() for character in compact)
    special_ratio = special_count / max(1, len(compact))
    ascii_run = re.search(rf"[A-Za-z]{{{minimum_ascii_run},}}", anchor_text)
    has_hangul = bool(HANGUL_SYLLABLE.search(anchor_text))
    has_latin = bool(re.search(r"[A-Za-z]", anchor_text))
    mixed_script = has_hangul and has_latin
    identifier_shape = bool(
        URL_SCHEME.search(anchor_text)
        or "@" in anchor_text
        or (has_latin and IDENTIFIER_SEPARATOR.search(anchor_text))
    )
    return bool(
        anchor_confidence < confidence_threshold
        or validate_ocr_text(anchor_text)
        or ascii_run
        or mixed_script
        or identifier_shape
        or special_ratio > special_character_ratio
    )


def hangul_dominant(text: str, *, minimum_syllables: int = 3, minimum_ratio: float = 0.6) -> bool:
    """Identify Korean-dominant output for a Korean-only visual specialist."""

    semantic = [character for character in text if character.isalnum()]
    hangul = [character for character in semantic if "가" <= character <= "힣"]
    return (
        len(hangul) >= minimum_syllables
        and len(hangul) / max(1, len(semantic)) >= minimum_ratio
    )


def select_conservative_specialist(
    anchor_text: str,
    anchor_confidence: float,
    anchor_language: float,
    specialist_text: str,
    specialist_confidence: float,
    specialist_language: float,
    *,
    confidence_margin: float = 0.01,
    language_tolerance: float = 0.0,
    max_disagreement_edits: int | None = None,
    lexicon_counts: Mapping[str, int] | None = None,
    require_hangul_dominant: bool = False,
) -> tuple[str, str]:
    """Prefer adaptation only when it cannot silently change visible facts."""

    if specialist_text == anchor_text:
        return anchor_text, "agreement"
    if require_hangul_dominant and not hangul_dominant(specialist_text):
        return anchor_text, "specialist_script_scope_guard"
    known_word_preserved, known_word_reason = candidate_preserves_known_word(
        anchor_text, specialist_text, lexicon_counts
    )
    if not known_word_preserved:
        return anchor_text, known_word_reason
    compatible, compatibility_reason = candidate_preserves_visible_facts(
        anchor_text, specialist_text
    )
    if not compatible:
        return anchor_text, compatibility_reason
    anchor_issues = validate_ocr_text(anchor_text)
    specialist_issues = validate_ocr_text(specialist_text)
    if specialist_issues:
        return anchor_text, "specialist_integrity_guard"
    if anchor_issues and not specialist_issues and specialist_confidence >= anchor_confidence:
        return specialist_text, "anchor_integrity_recovery"
    if (
        max_disagreement_edits is not None
        and disagreement_edits(anchor_text, specialist_text) > max_disagreement_edits
    ):
        return anchor_text, "disagreement_budget_guard"
    if specialist_confidence < anchor_confidence + confidence_margin:
        return anchor_text, "confidence_guard"
    if specialist_language <= anchor_language - language_tolerance:
        return anchor_text, "language_guard"
    return specialist_text, "specialist_selected"


def select_sparse_numeric_anchor(
    primary_text: str,
    anchor_text: str | None,
    *,
    structure_requires_numeric: bool = False,
) -> tuple[str, str, bool]:
    """Wake a stable anchor only where the primary may carry visible facts.

    The primary recognizer handles every line.  Numeric-looking output (or an
    independently typed numeric cell) activates the anchor; a disagreement in
    extracted values fails closed to the anchor.  No target label, document
    name or domain phrase participates in the decision.
    """

    primary_signature = visible_numeric_signature(primary_text)
    wake_anchor = bool(primary_signature) or structure_requires_numeric
    if not wake_anchor:
        return primary_text, "primary_no_numeric", False
    if anchor_text is None:
        raise ValueError("B_CORE_NATIVE_OCR_REQUIRED_ANCHOR_MISSING")
    if visible_numeric_signature(anchor_text) != primary_signature:
        return anchor_text, "numeric_anchor_fallback", True
    return primary_text, "numeric_anchor_agreement", True


def select_numeric_by_multiview_consensus(
    primary_text: str,
    anchor_text: str,
    observed_texts: list[str],
    *,
    structure_requires_numeric: bool = False,
) -> tuple[str, str]:
    """Resolve a numeric disagreement from repeated visual observations.

    Only the two baseline fact candidates are eligible.  Alternative views
    provide votes but cannot inject a third unverified value.  The newer
    primary is changed only when the anchor has repeated support and a clear
    margin; ambiguity does not cause a gratuitous rollback.
    """

    primary_facts = tuple(extract_numeric_facts(primary_text))
    anchor_facts = tuple(extract_numeric_facts(anchor_text))
    if primary_facts == anchor_facts:
        return primary_text, "numeric_multiview_agreement"
    if len(primary_facts) != len(anchor_facts) and not structure_requires_numeric:
        return primary_text, "numeric_fact_cardinality_guard"
    if (
        not structure_requires_numeric
        and NUMBER_ONLY.fullmatch(primary_text.strip())
        and NUMBER_ONLY.fullmatch(anchor_text.strip())
    ):
        return primary_text, "numeric_isolated_ambiguity_guard"
    votes = Counter(tuple(extract_numeric_facts(text)) for text in observed_texts)
    primary_votes = votes[primary_facts]
    anchor_votes = votes[anchor_facts]
    if anchor_votes >= 3 and anchor_votes >= primary_votes + 2:
        return anchor_text, "numeric_multiview_anchor"
    return primary_text, "numeric_multiview_primary"


def prediction_shape(text: str) -> str:
    normalized = text.strip()
    if NUMBER_ONLY.fullmatch(normalized):
        return "numeric_only"
    if MEASUREMENT.fullmatch(normalized):
        return "measurement"
    if 1 <= len(normalized) <= 4:
        return "short_text"
    return "open_text"


def unit_suffix(text: str) -> str | None:
    match = UNIT_SUFFIX.search(text.strip())
    return match.group(1) if match else None


def infer_column_unit_consensus(
    texts: list[str],
    *,
    minimum_support: int = 3,
    minimum_dominance: float = 0.8,
) -> str | None:
    units = [unit for text in texts if (unit := unit_suffix(text))]
    if len(units) < minimum_support:
        return None
    unit, count = Counter(units).most_common(1)[0]
    return unit if count >= minimum_support and count / len(units) >= minimum_dominance else None


_NUMERIC_GLYPH_LIKE = re.compile(r"^[0-9Oo,.\s]+$")
_NUMERIC_TOKEN_WITH_GLYPH_CONFUSION = re.compile(
    r"(?<![0-9A-Za-z])(?=[0-9Oo,.]*[0-9])[0-9Oo,.]*[Oo][0-9Oo,.]*(?![A-Za-z])"
)


def infer_numeric_column_consensus(
    texts: list[str],
    *,
    minimum_support: int = 3,
    minimum_dominance: float = 0.75,
) -> bool:
    """Identify numeric table columns without inspecting evaluation labels."""

    nonempty = [text.strip() for text in texts if text.strip()]
    if not nonempty:
        return False
    numeric_like = [
        text
        for text in nonempty
        if _NUMERIC_GLYPH_LIKE.fullmatch(text) and any(character.isdigit() for character in text)
    ]
    pure_numeric = [text for text in numeric_like if "O" not in text and "o" not in text]
    return (
        len(numeric_like) >= minimum_support
        and len(pure_numeric) >= 2
        and len(numeric_like) / len(nonempty) >= minimum_dominance
    )


def normalize_numeric_glyph_confusions(text: str, *, proven_numeric_column: bool) -> str:
    """Resolve glyph ambiguity only inside a structurally proven numeric column."""

    stripped = text.strip()
    if (
        not proven_numeric_column
        or not stripped
        or not _NUMERIC_GLYPH_LIKE.fullmatch(stripped)
        or not any(character.isdigit() for character in stripped)
    ):
        return text
    return text.replace("O", "0").replace("o", "0")


def normalize_numeric_token_glyph_confusions(text: str) -> str:
    """Resolve O/0 only inside an independently delimited numeric token.

    This is a script-level OCR invariant, not a vocabulary rule: a run must
    already contain a decimal digit, must contain the ambiguous glyph and may
    not be embedded in a Latin identifier.  Korean words, opaque product IDs
    and standalone Latin text are therefore left untouched.
    """

    return _NUMERIC_TOKEN_WITH_GLYPH_CONFUSION.sub(
        lambda match: match.group(0).replace("O", "0").replace("o", "0"),
        text,
    )


def table_cell_specialist_is_eligible(
    base_text: str,
    specialist_text: str,
    *,
    proven_table_cell: bool,
    column_consensus_unit: str | None = None,
) -> bool:
    """Allow a specialist only after geometry proves a table-cell boundary.

    The router does not inspect document names, phrases, or ground truth.  Both
    recognizers must independently agree on the value shape so the expert
    cannot turn an open-ended prose crop into a typed value.
    """

    if not proven_table_cell or not specialist_text.strip():
        return False
    base_shape = prediction_shape(base_text)
    specialist_shape = prediction_shape(specialist_text)
    # Blind decomposition showed stable benefit only for already-typed
    # measurements.  Numeric-only cells were tied and short labels mixed wins
    # with regressions, so neither may wake the specialist without additional
    # independent evidence.
    if base_shape == specialist_shape == "measurement":
        return True
    if not column_consensus_unit or unit_suffix(specialist_text) != column_consensus_unit:
        return False
    specialist_without_unit = specialist_text.strip()[: -len(column_consensus_unit)].strip()
    if not base_text.strip() and not specialist_without_unit:
        return True
    return base_shape == "numeric_only" and specialist_without_unit == base_text.strip()
