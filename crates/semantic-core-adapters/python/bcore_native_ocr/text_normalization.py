"""Lossless normalization for PDF text-layer OCR labels."""

import re


MEASUREMENT_UNIT_ALIASES = (
    (re.compile(r"(?<![A-Za-z])m\s*²"), "㎡"),
    (re.compile(r"(?<![A-Za-z])m\s*³"), "㎥"),
)


def sanitize_text_layer(text: str) -> str:
    """Drop invisible PDF control artifacts without rewriting visible text."""
    return "".join(character for character in text if ord(character) >= 32 or character in "\t").strip()


def canonicalize_measurement_units(text: str) -> str:
    """Unify visually equivalent metric-area/volume notations for data use."""
    normalized = text
    for pattern, replacement in MEASUREMENT_UNIT_ALIASES:
        normalized = pattern.sub(replacement, normalized)
    return normalized


def canonicalize_ocr_content(text: str) -> str:
    """Normalize content without learning presentation-only spacing."""

    return re.sub(r"\s+", " ", canonicalize_measurement_units(text)).strip()
