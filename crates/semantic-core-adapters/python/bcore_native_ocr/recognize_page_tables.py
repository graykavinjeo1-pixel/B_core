"""Run the owned detector, geometry compiler and recognizer on PDF tables."""

from __future__ import annotations

import argparse
import json
import logging
import math
import statistics
import tempfile
import time
from collections import Counter
from pathlib import Path

import numpy as np
import paddle
import pdfplumber
from PIL import Image, ImageEnhance, ImageFilter, ImageOps

try:
    import cv2
except ModuleNotFoundError:  # pragma: no cover - the bounded fallback is tested below.
    cv2 = None

from .borderless_table import (
    arithmetic_candidate_expansion_indices,
    reconstruct_repeated_numeric_tables,
    synchronize_reconstructed_rows,
)
from .build_pdf_corpus import trustworthy_text_layer
from .structured_evidence import (
    combine_table_evidence,
    compile_borderless_table_evidence,
    compile_ruled_table_evidence,
)
from .component_codec import load_codec
from .device_runtime import synchronize_accelerator
from .evidence_compiler import compile_approved_text_evidence
from .models import (
    BCoreDocumentDetector,
    BCoreLineRecognizer,
    BCoreVisualSpecialistPair,
    forward_recognizer_masked,
)
from .ocr_language_model import CharacterLanguageModel
from .ocr_lexicon import OcrLexicon
from .domain_ontology import load_long_term_repair_ontology
from .orientation_ensemble import (
    choose_orientation,
    load_paddlex_orientation_observer,
    observe_orientation,
)
from .paddlex_ocr_observer import (
    align_observer_to_rows,
    load_local_paddlex_ocr_observer,
    observe_page as observe_independent_ocr_page,
    quarantine_verified_numeric_disagreements,
    resolve_review_with_owned_model_consensus,
    verify_observations_with_bcore,
)
from .predict_table_overlay import _predict, _render_pages
from .recognition_geometry import (
    HORIZONTAL_SCALE,
    LINE_HEIGHT,
    MAX_LINE_WIDTH,
    resized_line_width,
)
from .scan_rule_recovery import recover_scan_rule_masks
from .scan_text_detection import (
    detect_component_cluster_text_support,
    detect_multiview_component_text_support,
    detect_scan_text_support,
    horizontal_text_coverage,
)
from .specialist_routing import (
    candidate_preserves_visible_facts,
    infer_numeric_column_consensus,
    normalize_numeric_glyph_confusions,
    normalize_numeric_token_glyph_confusions,
    mixed_script_specialist_wakeup_eligible,
    select_conservative_specialist,
    select_numeric_by_multiview_consensus,
    specialist_wakeup_eligible,
)
from .table_geometry import reconstruct_table_geometry, replace_text_regions
from .table_constraint_fusion import apply_geometry_cell_arithmetic_constraints
from .text_normalization import canonicalize_ocr_content
from .text_validation import (
    SEMANTIC_CHARACTER,
    beam_recovery_is_eligible,
    extract_numeric_facts,
    extract_numeric_tokens,
    validate_ocr_text,
)


_PRIMARY_MODEL_CACHE: dict[
    tuple[str, str, str],
    tuple[dict, BCoreDocumentDetector, object, BCoreLineRecognizer],
] = {}
_ORIENTATION_OBSERVER_CACHE: dict[str, object] = {}


def load_warm_primary_models(
    detector_path: Path,
    recognizer_path: Path,
    device: str,
) -> tuple[dict, BCoreDocumentDetector, object, BCoreLineRecognizer]:
    """Load and warm the owned OCR pair once per process.

    The HTTP runtime calls the existing reader repeatedly in one process.  A
    path/device keyed cache prevents every document from rereading weights and
    reallocating the same GPU tensors.
    """
    detector_path = detector_path.resolve()
    recognizer_path = recognizer_path.resolve()
    key = (str(detector_path), str(recognizer_path), device)
    cached = _PRIMARY_MODEL_CACHE.get(key)
    if cached is not None:
        return cached
    paddle.set_device(device)
    detector_metadata = json.loads(
        (detector_path / "model.json").read_text(encoding="utf-8")
    )
    detector = BCoreDocumentDetector()
    detector.set_state_dict(paddle.load(str(detector_path / "detector.pdparams")))
    detector.eval()
    codec = load_codec(recognizer_path / "codec.json")
    recognizer = BCoreLineRecognizer(codec.size)
    recognizer.set_state_dict(
        paddle.load(str(recognizer_path / "recognizer.pdparams"))
    )
    recognizer.eval()
    with paddle.no_grad():
        detector(
            paddle.zeros(
                [
                    1,
                    1,
                    int(detector_metadata["canvas"]),
                    int(detector_metadata["canvas"]),
                ],
                dtype="float32",
            )
        )
        recognizer(paddle.zeros([1, 1, 48, 128], dtype="float32"))
        synchronize_accelerator(device)
    loaded = (detector_metadata, detector, codec, recognizer)
    _PRIMARY_MODEL_CACHE[key] = loaded
    return loaded


def load_warm_orientation_observer(model_path: Path) -> object:
    """Load the optional local orientation observer once per worker process."""

    key = str(model_path.resolve())
    cached = _ORIENTATION_OBSERVER_CACHE.get(key)
    if cached is not None:
        return cached
    observer = load_paddlex_orientation_observer(model_path)
    _ORIENTATION_OBSERVER_CACHE[key] = observer
    return observer


logging.getLogger("pdfminer").setLevel(logging.ERROR)


def raster_row_confidence_floor(page_kind: str) -> float:
    """Return the minimum confidence for a row to own facts by itself.

    A trustworthy vector page has an independent embedded-text authority.  A
    scan row does not, so a weak single reading must remain review-only until
    another B_Core-owned view/model supplies matching evidence.  This prevents
    decorative page elements from becoming invented text while still allowing
    the evidence compiler to promote exact independent agreement later.
    """

    return 0.80 if page_kind == "scan" else 0.55


def resolve_scan_text_mode(
    requested: str,
    *,
    page_kind: str,
    useful_ratio: float,
    median_confidence: float,
) -> str:
    """Choose a B_Core-owned geometry path without inspecting page text.

    The legacy row-projection fallback is fast but can merge adjacent table
    lines into one tall crop.  Component/centerline clustering preserves the
    glyph-scale geometry needed by the recognizer and has now passed both a
    dense table page and a decorative slide-page check.  Easy pages keep the
    learned detector, so the extra component pass remains sparse.
    """

    if requested != "auto":
        return requested
    if page_kind == "scan" and (
        useful_ratio < 0.60 or median_confidence < 0.80
    ):
        return "multiview"
    return "learned"


def scan_row_language_recovery_is_grounded(row: dict, confidence_floor: float) -> bool:
    """Prevent a language beam from turning weak decorative ink into a fact.

    Beam search may choose among visually supported characters, but its
    language score must not be the only reason a scan row crosses the fact
    threshold.  At least one non-beam B_Core visual reading must already meet
    the same confidence floor.  Exact independent span agreement can still
    promote the row later through the evidence compiler.
    """

    if row.get("selectedReading") != "ctc_beam":
        return True
    return max(
        float(row.get("directConfidence", 0.0)),
        float(row.get("regionJoinedConfidence", 0.0)),
    ) >= confidence_floor


def classify_page_kind(vector_word_count: int, minimum_vector_words: int = 8) -> str:
    """Classify label authority without reading document names or phrases."""

    return "vector" if vector_word_count >= minimum_vector_words else "scan"


def compile_vector_text_evidence(words: list[dict]) -> dict:
    """Preserve trustworthy PDF text without mixing it into OCR inference.

    Coordinates remain in PDF page-point space.  The evidence is emitted as a
    parallel source so B_Core can prefer exact embedded text for facts while
    still using raster OCR and geometry for scans and layout verification.
    """

    normalized = [
        {
            "text": str(word.get("text", "")).strip(),
            "x0": float(word.get("x0", 0.0)),
            "top": float(word.get("top", 0.0)),
            "x1": float(word.get("x1", 0.0)),
            "bottom": float(word.get("bottom", 0.0)),
        }
        for word in words
        if str(word.get("text", "")).strip()
    ]
    layer_text = " ".join(word["text"] for word in normalized)
    layer_trustworthy = trustworthy_text_layer(layer_text)
    if not normalized:
        return {
            "coordinateSpace": "pdf-page-points",
            "words": [],
            "rows": [],
            "wordCount": 0,
            "preferredForTextFacts": False,
            "trustworthy": False,
        }
    heights = sorted(max(1.0, word["bottom"] - word["top"]) for word in normalized)
    tolerance = max(2.0, heights[len(heights) // 2] * 0.62)
    rows: list[dict] = []
    for word in sorted(
        normalized,
        key=lambda item: ((item["top"] + item["bottom"]) / 2, item["x0"]),
    ):
        center = (word["top"] + word["bottom"]) / 2
        row = next(
            (candidate for candidate in rows if abs(candidate["center"] - center) <= tolerance),
            None,
        )
        if row is None:
            rows.append({"center": center, "words": [word]})
        else:
            row["words"].append(word)
            row["center"] = sum(
                (item["top"] + item["bottom"]) / 2 for item in row["words"]
            ) / len(row["words"])
    compiled_rows = []
    for row in sorted(rows, key=lambda item: item["center"]):
        ordered = sorted(row["words"], key=lambda item: item["x0"])
        compiled_rows.append(
            {
                "text": " ".join(item["text"] for item in ordered),
                "box": [
                    min(item["x0"] for item in ordered),
                    min(item["top"] for item in ordered),
                    max(item["x1"] for item in ordered),
                    max(item["bottom"] for item in ordered),
                ],
                "source": "embedded_pdf_text",
            }
        )
    return {
        "coordinateSpace": "pdf-page-points",
        "words": normalized,
        "rows": compiled_rows,
        "wordCount": len(normalized),
        "trustworthy": layer_trustworthy,
        "preferredForTextFacts": (
            classify_page_kind(len(normalized)) == "vector" and layer_trustworthy
        ),
    }


def select_approved_text_evidence(
    vector_evidence: dict,
    raster_rows: list[dict],
) -> dict:
    """Choose the least lossy trustworthy text source without merging facts."""

    if vector_evidence.get("preferredForTextFacts"):
        return {
            "source": "embedded_pdf_text",
            "coordinateSpace": vector_evidence["coordinateSpace"],
            "rows": [
                {
                    **row,
                    "status": "accepted",
                    "granularity": "row",
                    "factContribution": "primary",
                    "provenance": ["trustworthy_embedded_pdf_text"],
                }
                for row in vector_evidence["rows"]
            ],
            "reason": "trustworthy_embedded_text_layer",
        }
    return {
        "source": "native_raster_ocr",
        "coordinateSpace": "rendered-page-pixels",
        "rows": [
            {
                "text": str(row.get("text", "")),
                "box": [
                    min((region["x0"] for region in row.get("regions", [])), default=0),
                    row.get("y0", 0),
                    max((region["x1"] for region in row.get("regions", [])), default=0),
                    row.get("y1", 0),
                ],
                "confidence": float(row.get("confidence", 0.0)),
                "status": row.get("status", "needs_review"),
                "source": "native_raster_ocr",
                "granularity": "row",
                "factContribution": "primary",
                "provenance": ["native_raster_ocr"],
            }
            for row in raster_rows
            if str(row.get("text", "")).strip()
        ],
        "reason": "missing_or_untrustworthy_embedded_text_layer",
    }


def compile_native_region_consensus(
    regions: list[dict],
    anchor_readings: list[tuple[str, float]],
    specialist_readings: list[tuple[str, float]],
    *,
    anchor_model_name: str,
    specialist_model_name: str,
    minimum_confidence: float = 0.90,
) -> dict:
    """Promote plain scan labels found and reread entirely by B_Core.

    Geometry comes from B_Core's language-independent scan detector.  Two
    separately trained owned recognizers must then agree exactly.  Quantities,
    dates, punctuation-bearing values and parenthetical facts remain excluded
    until an equally strong owned numeric specialist exists.
    """

    if not (len(regions) == len(anchor_readings) == len(specialist_readings)):
        raise ValueError("B_CORE_NATIVE_REGION_CONSENSUS_SHAPE")
    approved = []
    review_reasons: Counter[str] = Counter()
    for region, anchor, specialist in zip(
        regions,
        anchor_readings,
        specialist_readings,
    ):
        anchor_text = canonicalize_ocr_content(str(anchor[0] or ""))
        specialist_text = canonicalize_ocr_content(str(specialist[0] or ""))
        reasons = []
        if float(anchor[1]) < minimum_confidence:
            reasons.append("anchor_confidence_guard")
        if float(specialist[1]) < minimum_confidence:
            reasons.append("specialist_confidence_guard")
        if not anchor_text or anchor_text != specialist_text:
            reasons.append("owned_model_disagreement")
        if extract_numeric_facts(anchor_text):
            reasons.append("numeric_fact_requires_owned_numeric_consensus")
        if any(character in anchor_text for character in "()[]{}"):
            reasons.append("parenthetical_fact_requires_stronger_consensus")
        if validate_ocr_text(anchor_text):
            reasons.append("candidate_integrity_guard")
        if not region.get("visualTextEvidence", {}).get("hasGlyphEvidence"):
            reasons.append("no_visual_glyph_evidence")
        if reasons:
            review_reasons.update(reasons)
            continue
        approved.append(
            {
                "text": anchor_text,
                "box": [
                    int(region["x0"]),
                    int(region["y0"]),
                    int(region["x1"]),
                    int(region["y1"]),
                ],
                "confidence": min(float(anchor[1]), float(specialist[1])),
                "status": "accepted",
                "granularity": "span",
                "factContribution": "primary",
                "source": "bcore_cv_detector_plus_owned_recognizer_consensus",
                "provenance": [
                    "bcore_language_independent_scan_detector",
                    anchor_model_name,
                    specialist_model_name,
                ],
                "independentAgreementCount": 3,
            }
        )
    return {
        "approvedEvidence": approved,
        "counts": {
            "candidates": len(regions),
            "approved": len(approved),
            "needsReview": len(regions) - len(approved),
        },
        "reviewReasons": dict(review_reasons),
        "contract": (
            "B_Core CV geometry plus exact high-confidence agreement between two "
            "owned recognizers; numeric and parenthetical facts remain fail-closed."
        ),
    }


def rotate_for_orientation(source: Image.Image, degrees: int) -> Image.Image:
    """Rotate a rendered page without clipping portrait/landscape transitions."""

    normalized = degrees % 360
    if normalized not in (0, 90, 180, 270):
        raise ValueError(f"unsupported page orientation: {degrees}")
    if normalized == 0:
        return source
    return source.rotate(
        normalized,
        expand=normalized in (90, 270),
        fillcolor=255,
    )


def _orientation_candidate_angles(
    source: Image.Image,
    minimum_axis_margin: float = 0.15,
    maximum_probe_dimension: int = 768,
) -> tuple[tuple[int, ...], dict[str, object]]:
    """Discard the impossible orientation axis before neural scoring.

    A 180-degree turn preserves horizontal text support, so this inexpensive
    stage only chooses between the 0/180 and 90/270 axes.  Ambiguous pages keep
    the full four-way recognizer consensus path.
    """

    grayscale = source.convert("L")
    probe = grayscale.copy()
    if max(probe.size) > maximum_probe_dimension:
        scale = maximum_probe_dimension / max(probe.size)
        probe = probe.resize(
            (
                max(1, round(probe.width * scale)),
                max(1, round(probe.height * scale)),
            ),
            Image.Resampling.BILINEAR,
        )
    original_coverage = horizontal_text_coverage(np.asarray(probe))
    quarter_turn = rotate_for_orientation(probe, 90)
    quarter_coverage = horizontal_text_coverage(np.asarray(quarter_turn))
    margin = abs(original_coverage - quarter_coverage)
    if max(original_coverage, quarter_coverage) < 0.05 or margin < minimum_axis_margin:
        angles = (0, 90, 180, 270)
        decision = "ambiguous_four_way"
    elif original_coverage > quarter_coverage:
        angles = (0, 180)
        decision = "horizontal_axis"
    else:
        angles = (90, 270)
        decision = "quarter_turn_axis"
    return angles, {
        "decision": decision,
        "candidateAngles": list(angles),
        "originalCoverage": original_coverage,
        "quarterTurnCoverage": quarter_coverage,
        "coverageMargin": margin,
        "minimumAxisMargin": minimum_axis_margin,
        "probeSize": list(probe.size),
    }


def _recognizer_input(
    image: Image.Image,
    height: int = LINE_HEIGHT,
    max_width: int = MAX_LINE_WIDTH,
    horizontal_scale: float = HORIZONTAL_SCALE,
) -> np.ndarray:
    grayscale, _ = normalize_text_polarity(image)
    width = resized_line_width(
        grayscale.width,
        grayscale.height,
        line_height=height,
        max_width=max_width,
        horizontal_scale=horizontal_scale,
    )
    resized = grayscale.resize((width, height), Image.Resampling.LANCZOS)
    return (np.asarray(resized, dtype="float32") / 127.5 - 1.0)[None, None, :, :]


def normalize_text_polarity(image: Image.Image) -> tuple[Image.Image, bool]:
    """Normalize rare light-on-dark text from visual evidence alone.

    The histogram mode estimates the crop background.  Inversion is allowed
    only when brighter foreground pixels dominate darker deviations by a wide
    margin.  Ordinary black text on white or shaded cells therefore remains
    byte-for-byte on the original path.
    """

    grayscale = image.convert("L")
    pixels = np.asarray(grayscale)
    if pixels.size == 0:
        return grayscale, False
    mode = int(np.bincount(pixels.ravel(), minlength=256).argmax())
    if mode >= 220:
        return grayscale, False
    darker = int((pixels < mode - 25).sum())
    brighter = int((pixels > mode + 25).sum())
    minimum_foreground = max(8, round(pixels.size * 0.01))
    if brighter < minimum_foreground or brighter < max(1, darker) * 2:
        return grayscale, False
    return ImageOps.autocontrast(ImageOps.invert(grayscale), cutoff=1), True


def _split_long_line_crop(
    crop: Image.Image,
    *,
    height: int = LINE_HEIGHT,
    max_width: int = MAX_LINE_WIDTH,
    horizontal_scale: float = HORIZONTAL_SCALE,
    width_utilization: float = 0.82,
    maximum_source_height_ratio: float = 0.78,
) -> list[Image.Image]:
    """Split an over-compressed line only at visually proven blank columns.

    This is deliberately language-agnostic.  It does not inspect characters,
    words, document names or labels.  If a safe whitespace boundary cannot be
    found, the original crop is returned unchanged rather than cutting glyphs.
    """

    grayscale = crop.convert("L")
    # Crops already rendered near the recognizer's native height do not gain
    # visual information by being divided and enlarged again.  Segmentation is
    # reserved for genuinely small source text that would otherwise be both
    # upscaled vertically and crushed horizontally into max_width.
    if grayscale.height >= height * maximum_source_height_ratio:
        return [grayscale]
    natural_width = round(
        grayscale.width * height * horizontal_scale / max(1, grayscale.height)
    )
    if natural_width <= max_width:
        return [grayscale]
    if not 0.5 <= width_utilization <= 1.0 or not 0.5 <= maximum_source_height_ratio <= 1.0:
        raise ValueError("B_CORE_NATIVE_OCR_LONG_LINE_UTILIZATION_INVALID")
    maximum_source_span = max(
        24,
        int(
            max_width
            * grayscale.height
            * width_utilization
            / max(1.0, height * horizontal_scale)
        ),
    )
    if maximum_source_span >= grayscale.width:
        return [grayscale]

    pixels = np.asarray(grayscale)
    ink_per_column = (pixels < 210).sum(axis=0)
    maximum_blank_ink = max(1, round(grayscale.height * 0.04))
    minimum_span = max(20, round(maximum_source_span * 0.45))
    boundaries = [0]
    start = 0
    while grayscale.width - start > maximum_source_span:
        search_left = start + minimum_span
        search_right = min(
            grayscale.width - minimum_span,
            start + maximum_source_span,
        )
        if search_right <= search_left:
            return [grayscale]
        candidates = np.flatnonzero(
            ink_per_column[search_left : search_right + 1] <= maximum_blank_ink
        )
        if not len(candidates):
            return [grayscale]
        absolute = candidates + search_left
        runs: list[tuple[int, int]] = []
        run_start = int(absolute[0])
        previous = run_start
        for value in absolute[1:]:
            value = int(value)
            if value != previous + 1:
                runs.append((run_start, previous))
                run_start = value
            previous = value
        runs.append((run_start, previous))
        # A glyph can contain a one- or two-column internal void after scan
        # resampling.  Require whitespace proportional to source text height
        # so a Korean word such as `상품권` is never split inside a syllable.
        minimum_blank_run = max(3, round(grayscale.height * 0.10))
        runs = [
            run
            for run in runs
            if run[1] - run[0] + 1 >= minimum_blank_run
        ]
        if not runs:
            return [grayscale]
        desired = start + maximum_source_span
        split = min(
            ((left + right) // 2 for left, right in runs),
            key=lambda value: abs(desired - value),
        )
        if split <= start or split >= grayscale.width:
            return [grayscale]
        boundaries.append(split)
        start = split
    boundaries.append(grayscale.width)
    if len(boundaries) <= 2:
        return [grayscale]
    return [
        grayscale.crop((left, 0, right, grayscale.height))
        for left, right in zip(boundaries, boundaries[1:])
    ]


def _split_multiline_row_crop(
    crop: Image.Image,
    *,
    minimum_height: int = 64,
) -> list[Image.Image]:
    """Split detector-merged text lines from visual row bands only.

    Scanned shaded table headers can be returned as one tall detector row.
    Compressing that block to the recognizer's 48-pixel input destroys every
    line.  This routine removes visually continuous horizontal rules, finds
    separated ink bands and returns each band in reading order.  It does not
    inspect language, document names or expected labels.
    """

    grayscale, _ = normalize_text_polarity(crop)
    if grayscale.height < minimum_height or grayscale.width < 16:
        return [grayscale]
    pixels = np.asarray(grayscale)
    mode = int(np.bincount(pixels.ravel(), minlength=256).argmax())
    foreground = pixels[pixels < mode - 12]
    if foreground.size < max(8, round(pixels.size * 0.0005)):
        return [grayscale]
    foreground_ceiling = float(np.percentile(foreground, 90))
    threshold = min(225.0, max(45.0, (foreground_ceiling + mode) / 2))
    ink = pixels < threshold
    # Table rules are not text. Removing only rows spanning nearly half the
    # crop and columns spanning nearly half the crop keeps glyph strokes while
    # preventing horizontal rules or section bars from joining text bands.
    rule_rows = ink.mean(axis=1) >= 0.45
    rule_columns = ink.mean(axis=0) >= 0.45
    ink[rule_rows, :] = False
    ink[:, rule_columns] = False

    def remove_long_runs(values: np.ndarray, minimum: int) -> None:
        indexes = np.flatnonzero(values)
        if not len(indexes):
            return
        start = int(indexes[0])
        previous = start
        for current_value in indexes[1:]:
            current = int(current_value)
            if current != previous + 1:
                if previous - start + 1 >= minimum:
                    values[start : previous + 1] = False
                start = current
            previous = current
        if previous - start + 1 >= minimum:
            values[start : previous + 1] = False

    for row_index in range(ink.shape[0]):
        remove_long_runs(ink[row_index], max(12, round(grayscale.width * 0.12)))
    for column_index in range(ink.shape[1]):
        remove_long_runs(ink[:, column_index], max(12, round(grayscale.height * 0.35)))
    minimum_row_ink = max(3, round(grayscale.width * 0.003))
    active = ink.sum(axis=1) >= minimum_row_ink
    active_rows = np.flatnonzero(active)
    if not len(active_rows):
        return [grayscale]
    raw_bands: list[tuple[int, int]] = []
    start = int(active_rows[0])
    previous = start
    for value in active_rows[1:]:
        value = int(value)
        if value != previous + 1:
            raw_bands.append((start, previous))
            start = value
        previous = value
    raw_bands.append((start, previous))
    raw_bands = [
        band for band in raw_bands if band[1] - band[0] + 1 >= 4
    ]
    maximum_internal_gap = max(2, round(grayscale.height * 0.02))
    bands: list[tuple[int, int]] = []
    for band in raw_bands:
        merge_with_previous = False
        if bands and band[0] - bands[-1][1] - 1 <= maximum_internal_gap:
            previous = bands[-1]
            previous_columns = np.flatnonzero(
                ink[previous[0] : previous[1] + 1].any(axis=0)
            )
            current_columns = np.flatnonzero(
                ink[band[0] : band[1] + 1].any(axis=0)
            )
            if len(previous_columns) and len(current_columns):
                previous_left, previous_right = (
                    int(previous_columns[0]), int(previous_columns[-1])
                )
                current_left, current_right = (
                    int(current_columns[0]), int(current_columns[-1])
                )
                overlap = max(
                    0,
                    min(previous_right, current_right)
                    - max(previous_left, current_left)
                    + 1,
                )
                smaller_span = max(
                    1,
                    min(
                        previous_right - previous_left + 1,
                        current_right - current_left + 1,
                    ),
                )
                merge_with_previous = overlap / smaller_span >= 0.25
        if merge_with_previous:
            bands[-1] = (bands[-1][0], band[1])
        else:
            bands.append(band)
    if len(bands) < 2:
        return [grayscale]
    segments = []
    for top, bottom in bands:
        top = max(0, top - 3)
        bottom = min(grayscale.height - 1, bottom + 3)
        band_ink = ink[top : bottom + 1]
        minimum_column_ink = max(3, round(band_ink.shape[0] * 0.25))
        columns = np.flatnonzero(
            band_ink.sum(axis=0) >= minimum_column_ink
        )
        if not len(columns):
            continue
        column_runs: list[tuple[int, int]] = []
        run_start = int(columns[0])
        previous_column = run_start
        for column_value in columns[1:]:
            column = int(column_value)
            if column != previous_column + 1:
                column_runs.append((run_start, previous_column))
                run_start = column
            previous_column = column
        column_runs.append((run_start, previous_column))
        maximum_text_stroke_width = max(20, band_ink.shape[0] * 8)
        text_runs = [
            run
            for run in column_runs
            if run[1] - run[0] + 1 <= maximum_text_stroke_width
        ]
        if not text_runs:
            continue
        left = max(0, min(run[0] for run in text_runs) - 4)
        right = min(grayscale.width, max(run[1] for run in text_runs) + 5)
        segments.append(grayscale.crop((left, top, right, bottom + 1)))
    return segments if len(segments) >= 2 else [grayscale]


def _split_multicolumn_header_crop(crop: Image.Image) -> list[Image.Image]:
    """Decompose a visually tabular multi-line header into reading-order tiles.

    The decision uses only whitespace gutters and ink bands. It is unavailable
    to ordinary short lines and fails closed unless at least three columns and
    repeated multi-line structure are visible.
    """

    grayscale = crop.convert("L")
    if grayscale.height < round(LINE_HEIGHT * 1.15):
        return [grayscale]
    if grayscale.width / max(1, grayscale.height) < 8.0:
        return [grayscale]
    pixels = np.asarray(grayscale)
    ink = (pixels < 220).copy()

    def remove_long_runs(values: np.ndarray, minimum: int) -> None:
        indices = np.flatnonzero(values)
        if not len(indices):
            return
        start = previous = int(indices[0])
        for raw_value in indices[1:]:
            value = int(raw_value)
            if value != previous + 1:
                if previous - start + 1 >= minimum:
                    values[start : previous + 1] = False
                start = value
            previous = value
        if previous - start + 1 >= minimum:
            values[start : previous + 1] = False

    for row_index in range(ink.shape[0]):
        remove_long_runs(ink[row_index], max(12, round(grayscale.width * 0.12)))
    maximum_blank_ink = max(1, round(grayscale.height * 0.03))
    blank_columns = np.flatnonzero(ink.sum(axis=0) <= maximum_blank_ink)
    if not len(blank_columns):
        return [grayscale]
    minimum_gutter = max(16, round(grayscale.width * 0.035))
    gutters: list[tuple[int, int]] = []
    start = previous = int(blank_columns[0])
    for raw_value in blank_columns[1:]:
        value = int(raw_value)
        if value != previous + 1:
            if previous - start + 1 >= minimum_gutter:
                gutters.append((start, previous))
            start = value
        previous = value
    if previous - start + 1 >= minimum_gutter:
        gutters.append((start, previous))
    internal_gutters = [
        gutter
        for gutter in gutters
        if gutter[0] > 4 and gutter[1] < grayscale.width - 5
    ]
    if len(internal_gutters) < 2 or len(internal_gutters) > 5:
        return [grayscale]
    cuts = [0] + [
        (left + right) // 2 for left, right in internal_gutters
    ] + [grayscale.width]
    columns = [
        grayscale.crop((cuts[index], 0, cuts[index + 1], grayscale.height))
        for index in range(len(cuts) - 1)
        if cuts[index + 1] - cuts[index] > 20
    ]
    if len(columns) < 3:
        return [grayscale]
    column_groups = [_split_multiline_row_crop(column) for column in columns]
    if sum(len(group) > 1 for group in column_groups) < 2:
        return [grayscale]
    return [segment for group in column_groups for segment in group]


def _crop(source: Image.Image, region: dict[str, int], padding: int = 3) -> Image.Image:
    return source.crop(
        (
            max(0, region["x0"] - padding),
            max(0, region["y0"] - padding),
            min(source.width, region["x1"] + padding + 1),
            min(source.height, region["y1"] + padding + 1),
        )
    )


def _trim_overlapping_row_boxes(
    row_boxes: list[dict[str, int]],
    *,
    minimum_tall_height: int = 96,
) -> list[dict[str, int]]:
    """Prevent a detector row from rereading the next row's visible glyphs."""

    trimmed = [dict(box) for box in row_boxes]
    for index in range(len(trimmed) - 1):
        current = trimmed[index]
        following = trimmed[index + 1]
        if (
            current["y1"] - current["y0"] + 1 >= minimum_tall_height
            and current["y1"] >= following["y0"]
        ):
            current["y1"] = max(current["y0"], following["y0"] - 1)
    return trimmed


def _regions_for_joined_row(
    regions: list[dict],
    *,
    page_width: int,
) -> list[dict]:
    """Exclude weak page-edge rule fragments from a joined text reading.

    The detector deliberately keeps uncertain regions as evidence. A narrow,
    low-confidence vertical fragment at the extreme page edge, however, must
    not be appended to an otherwise well-supported row as if it were text.
    This filter only affects the joined candidate; the source region remains
    in geometry for audit and review.
    """

    visible = [region for region in regions if region.get("text")]
    if len(visible) < 2:
        return visible
    substantive = [
        region
        for region in visible
        if len(str(region.get("text", "")).strip()) >= 2
        and float(region.get("confidence", 0.0)) >= 0.75
    ]
    if not substantive:
        return visible
    filtered = []
    for region in visible:
        width = int(region["x1"]) - int(region["x0"]) + 1
        height = int(region["y1"]) - int(region["y0"]) + 1
        at_page_edge = (
            int(region["x0"]) <= round(page_width * 0.08)
            or int(region["x1"]) >= round(page_width * 0.92)
        )
        weak_vertical_fragment = (
            len(str(region.get("text", "")).strip()) <= 1
            and float(region.get("confidence", 0.0)) < 0.50
            and width <= max(32, round(page_width * 0.025))
            and height >= max(12, round(width * 1.5))
        )
        if at_page_edge and weak_vertical_fragment:
            continue
        filtered.append(region)
    return filtered or visible


def _read_crop(
    recognizer,
    codec,
    crop: Image.Image,
    *,
    height: int = LINE_HEIGHT,
    max_width: int = MAX_LINE_WIDTH,
    horizontal_scale: float = HORIZONTAL_SCALE,
) -> tuple[str, float]:
    checkbox_prefix, recognition_crop = _extract_leading_hollow_square(crop)
    logits = recognizer(
        paddle.to_tensor(
            _recognizer_input(
                recognition_crop,
                height=height,
                max_width=max_width,
                horizontal_scale=horizontal_scale,
            )
        )
    )
    text, confidence = _decode_logits(logits[0], codec)
    return canonicalize_ocr_content(checkbox_prefix + text), confidence


def _connected_components(mask: np.ndarray) -> list[tuple[int, int, int, int, int]]:
    """Return 8-connected component boxes as x0, y0, x1, y1, area."""

    # Glyph evidence and checkbox recovery are invoked for every region and
    # row.  The original Python flood fill made their cost proportional to
    # every ink pixel, often exceeding the recognizer's own GPU work on a
    # clean page.  OpenCV's native connected-components implementation has
    # the same 8-neighbour semantics and returns the same geometric contract.
    # It is an optional acceleration only: a minimal local runtime without
    # OpenCV keeps the exact bounded Python implementation below.
    if cv2 is not None:
        source = np.ascontiguousarray(mask.astype(np.uint8, copy=False))
        component_count, _labels, statistics, _centroids = cv2.connectedComponentsWithStats(
            source,
            connectivity=8,
        )
        return [
            (
                int(statistics[label, cv2.CC_STAT_LEFT]),
                int(statistics[label, cv2.CC_STAT_TOP]),
                int(statistics[label, cv2.CC_STAT_LEFT]
                    + statistics[label, cv2.CC_STAT_WIDTH]),
                int(statistics[label, cv2.CC_STAT_TOP]
                    + statistics[label, cv2.CC_STAT_HEIGHT]),
                int(statistics[label, cv2.CC_STAT_AREA]),
            )
            for label in range(1, component_count)
        ]

    height, width = mask.shape
    visited = np.zeros(mask.shape, dtype=bool)
    components = []
    for y, x in zip(*np.where(mask)):
        if visited[y, x]:
            continue
        stack = [(int(y), int(x))]
        visited[y, x] = True
        left = right = int(x)
        top = bottom = int(y)
        area = 0
        while stack:
            row, column = stack.pop()
            area += 1
            left = min(left, column)
            right = max(right, column)
            top = min(top, row)
            bottom = max(bottom, row)
            for next_row in range(max(0, row - 1), min(height, row + 2)):
                for next_column in range(max(0, column - 1), min(width, column + 2)):
                    if mask[next_row, next_column] and not visited[next_row, next_column]:
                        visited[next_row, next_column] = True
                        stack.append((next_row, next_column))
        components.append((left, top, right + 1, bottom + 1, area))
    return components


def visual_glyph_evidence(crop: Image.Image) -> dict[str, int | float | bool]:
    """Verify that a crop contains glyph-like ink rather than only page rules.

    This gate does not read language. Nearly full-height/width strokes are
    removed as structural rules, after which at least one small connected ink
    component must remain. A short digit-like stroke survives because it does
    not span almost the entire padded crop.
    """

    grayscale = crop.convert("L")
    pixels = np.asarray(grayscale)
    threshold = min(210, int(np.quantile(pixels, 0.20)) + 70)
    ink = pixels < threshold
    original_ink = int(ink.sum())
    rule_rows = ink.mean(axis=1) >= 0.88
    rule_columns = ink.mean(axis=0) >= 0.88
    residual = ink.copy()
    residual[rule_rows, :] = False
    residual[:, rule_columns] = False
    components = [
        component
        for component in _connected_components(residual)
        if component[4] >= 3
    ]
    residual_ink = int(residual.sum())
    return {
        "hasGlyphEvidence": bool(components and residual_ink >= 4),
        "threshold": threshold,
        "originalInkPixels": original_ink,
        "residualInkPixels": residual_ink,
        "componentCount": len(components),
        "removedRuleRows": int(rule_rows.sum()),
        "removedRuleColumns": int(rule_columns.sum()),
    }


def _extract_leading_visual_marker(crop: Image.Image) -> tuple[str, Image.Image]:
    """Classify a visually isolated leading square/circle marker.

    No target text, phrase, font or document name participates.  Recovery is
    allowed only for a leftmost near-square component with an independently
    visible whitespace gap before substantive remainder.  Hollow/filled and
    square/circle decisions come from centre, edge and corner occupancy.
    Ambiguous glyphs stay on the neural recognizer path unchanged.
    """

    grayscale = crop.convert("L")
    pixels = np.asarray(grayscale)
    if pixels.size == 0 or grayscale.width < 16 or grayscale.height < 10:
        return "", grayscale
    background = int(np.quantile(pixels, 0.85))
    threshold = min(210, background - 35)
    mask = pixels < threshold
    # Only a leading glyph can satisfy this contract.  Restrict flood filling
    # to a small visual prefix so long paragraphs do not turn a constant-time
    # symbol check into a full-line connected-component scan.
    probe_width = min(grayscale.width, max(40, round(grayscale.height * 2.2)))
    components = [
        component
        for component in _connected_components(mask[:, :probe_width])
        if component[4] >= 8
    ]
    if not components:
        return "", grayscale
    components.sort(key=lambda component: (component[0], component[1]))
    left, top, right, bottom, area = components[0]
    width = right - left
    height = bottom - top
    if left > max(8, round(grayscale.width * 0.08)):
        return "", grayscale
    if not (8 <= width <= max(12, round(grayscale.height * 0.65))):
        return "", grayscale
    if not 0.78 <= width / max(1, height) <= 1.28:
        return "", grayscale
    box = mask[top:bottom, left:right]
    border = max(1, round(min(width, height) * 0.18))
    inner = box[border:-border, border:-border]
    if not inner.size:
        return "", grayscale
    top_edge = float(box[:border].mean())
    bottom_edge = float(box[-border:].mean())
    left_edge = float(box[:, :border].mean())
    right_edge = float(box[:, -border:].mean())
    edge_occupancy = (top_edge, bottom_edge, left_edge, right_edge)
    corner_span = max(2, round(min(width, height) * 0.22))
    corner_occupancy = (
        float(box[:corner_span, :corner_span].mean()),
        float(box[:corner_span, -corner_span:].mean()),
        float(box[-corner_span:, :corner_span].mean()),
        float(box[-corner_span:, -corner_span:].mean()),
    )
    fill = area / max(1, width * height)
    inner_occupancy = float(inner.mean())
    symbol = ""
    if (
        inner_occupancy <= 0.08
        and min(edge_occupancy) >= 0.28
        and 0.14 <= fill <= 0.58
    ):
        if min(corner_occupancy) >= 0.45:
            symbol = "□"
        elif max(corner_occupancy) <= 0.42:
            symbol = "○"
    elif inner_occupancy >= 0.55 and 0.52 <= fill <= 1.0:
        if min(corner_occupancy) >= 0.42:
            symbol = "■"
        elif max(corner_occupancy) <= 0.40:
            symbol = "●"
    if not symbol:
        return "", grayscale
    following_columns = np.where(mask[:, right:].any(axis=0))[0]
    if not len(following_columns):
        return "", grayscale
    gap = int(following_columns[0])
    minimum_gap = 0.12 if symbol == "□" else 0.35
    if gap < max(2, round(width * minimum_gap)):
        return "", grayscale
    remainder_left = right + gap
    if remainder_left >= grayscale.width - 3:
        return "", grayscale
    return f"{symbol} ", grayscale.crop(
        (remainder_left, 0, grayscale.width, grayscale.height)
    )


def _extract_leading_hollow_square(crop: Image.Image) -> tuple[str, Image.Image]:
    """Compatibility gate: production currently authorizes only ``□``."""

    prefix, remainder = _extract_leading_visual_marker(crop)
    if prefix.strip() == "□":
        return prefix, remainder
    return "", crop.convert("L")


def _read_crop_segmented(
    recognizer,
    codec,
    crop: Image.Image,
    *,
    height: int = LINE_HEIGHT,
    max_width: int = MAX_LINE_WIDTH,
    horizontal_scale: float = HORIZONTAL_SCALE,
) -> tuple[str, float]:
    """Read long lines at native character scale and join at visual spaces."""

    segments = _split_long_line_crop(
        crop,
        height=height,
        max_width=max_width,
        horizontal_scale=horizontal_scale,
    )
    if len(segments) == 1:
        return _read_crop(
            recognizer,
            codec,
            crop,
            height=height,
            max_width=max_width,
            horizontal_scale=horizontal_scale,
        )
    readings = [
        _read_crop(
            recognizer,
            codec,
            segment,
            height=height,
            max_width=max_width,
            horizontal_scale=horizontal_scale,
        )
        for segment in segments
    ]
    text = canonicalize_ocr_content(" ".join(value for value, _ in readings if value))
    weights = [max(1, len(value)) for value, _ in readings]
    confidence = sum(
        confidence * weight
        for (_, confidence), weight in zip(readings, weights)
    ) / max(1, sum(weights))
    return text, confidence


def _join_segmented_readings(
    readings: list[tuple[str, float]], segment_counts: list[int]
) -> list[tuple[str, float]]:
    """Restore one reading per source crop after sparse long-line splitting."""

    restored = []
    offset = 0
    for count in segment_counts:
        values = readings[offset : offset + count]
        offset += count
        text = canonicalize_ocr_content(" ".join(value for value, _ in values if value))
        weights = [max(1, len(value)) for value, _ in values]
        confidence = sum(
            confidence * weight
            for (_, confidence), weight in zip(values, weights)
        ) / max(1, sum(weights))
        restored.append((text, confidence))
    if offset != len(readings):
        raise ValueError("B_CORE_NATIVE_OCR_SEGMENT_RESTORE_MISMATCH")
    return restored


def _split_crops_for_recognition(
    crops: list[Image.Image], *, max_width: int
) -> tuple[list[Image.Image], list[int]]:
    groups = [
        _split_long_line_crop(crop, max_width=max_width)
        for crop in crops
    ]
    return [segment for group in groups for segment in group], [len(group) for group in groups]


def _decode_logits(logits: paddle.Tensor, codec) -> tuple[str, float]:
    probabilities = paddle.nn.functional.softmax(logits, axis=-1)
    token_ids = probabilities.argmax(axis=-1).numpy().tolist()
    maximum = probabilities.max(axis=-1).numpy().tolist()
    emitted_confidence = []
    previous = None
    for token, confidence in zip(token_ids, maximum):
        if token != 0 and token != previous:
            emitted_confidence.append(float(confidence))
        previous = token
    text = canonicalize_ocr_content(codec.decode_ctc(token_ids))
    return text, sum(emitted_confidence) / max(1, len(emitted_confidence))


def _read_crops_batch(
    recognizer,
    codec,
    crops: list[Image.Image],
    batch_size: int = 12,
    width_bin: int = 32,
    max_width: int = MAX_LINE_WIDTH,
    segment_long_lines: bool = False,
) -> list[tuple[str, float]]:
    if not crops:
        return []
    prefixed = [_extract_leading_hollow_square(crop) for crop in crops]
    prefixes = [prefix for prefix, _ in prefixed]
    crops = [normalized for _, normalized in prefixed]
    if segment_long_lines:
        segments, counts = _split_crops_for_recognition(crops, max_width=max_width)
        if len(segments) != len(crops):
            joined = _join_segmented_readings(
                _read_crops_batch(
                    recognizer,
                    codec,
                    segments,
                    batch_size=batch_size,
                    width_bin=width_bin,
                    max_width=max_width,
                    segment_long_lines=False,
                ),
                counts,
            )
            return [
                (canonicalize_ocr_content(prefix + text), confidence)
                for prefix, (text, confidence) in zip(prefixes, joined)
            ]
    prepared = [_recognizer_input(crop, max_width=max_width)[0] for crop in crops]
    buckets: dict[int, list[int]] = {}
    for index, image in enumerate(prepared):
        width = image.shape[2]
        bucket = (width + width_bin - 1) // width_bin
        buckets.setdefault(bucket, []).append(index)
    results: list[tuple[str, float] | None] = [None] * len(crops)

    def read_indices(batch_indices: list[int]) -> None:
        """Read one width bucket and back off safely under memory pressure."""

        maximum_width = max(
            prepared[source_index].shape[2] for source_index in batch_indices
        )
        try:
            values = np.zeros(
                (len(batch_indices), 1, 48, maximum_width), dtype="float32"
            )
            input_widths = []
            for batch_index, source_index in enumerate(batch_indices):
                image = prepared[source_index]
                width = image.shape[2]
                values[batch_index, :, :, :width] = image
                input_widths.append(width)
            logits, feature_widths = forward_recognizer_masked(
                recognizer,
                paddle.to_tensor(values),
                paddle.to_tensor(input_widths, dtype="int64"),
            )
            lengths = feature_widths.numpy().tolist()
            decoded = [
                _decode_logits(logits[index, : lengths[index]], codec)
                for index in range(len(batch_indices))
            ]
        except MemoryError:
            if len(batch_indices) == 1:
                raise
            midpoint = max(1, len(batch_indices) // 2)
            read_indices(batch_indices[:midpoint])
            read_indices(batch_indices[midpoint:])
            return
        for source_index, reading in zip(batch_indices, decoded):
            results[source_index] = reading

    for indices in buckets.values():
        for start in range(0, len(indices), batch_size):
            batch_indices = indices[start : start + batch_size]
            read_indices(batch_indices)
    finalized = [result if result is not None else ("", 0.0) for result in results]
    return [
        (canonicalize_ocr_content(prefix + text), confidence)
        for prefix, (text, confidence) in zip(prefixes, finalized)
    ]


def _dense_title_visual_metrics(image: Image.Image) -> dict[str, float]:
    """Measure dense display text without inspecting its language or target."""

    pixels = np.asarray(image.convert("L"))
    threshold = min(210, int(np.quantile(pixels, 0.22)) + 70)
    ink = pixels < threshold
    rows, columns = np.where(ink)
    if not len(rows):
        return {"inkDensity": 0.0, "tightAspect": 0.0, "tightHeight": 0.0}
    top, bottom = int(rows.min()), int(rows.max()) + 1
    left, right = int(columns.min()), int(columns.max()) + 1
    tight = ink[top:bottom, left:right]
    height, width = tight.shape
    return {
        "inkDensity": float(tight.mean()),
        "tightAspect": float(width / max(1, height)),
        "tightHeight": float(height),
    }


def _dense_title_normalization_candidates(image: Image.Image) -> list[Image.Image]:
    grayscale = image.convert("L")
    thinned = grayscale.filter(ImageFilter.MaxFilter(5))
    return [
        thinned,
        grayscale.resize(
            (max(8, round(grayscale.width * 0.75)), grayscale.height),
            Image.Resampling.LANCZOS,
        ),
        thinned.resize(
            (max(8, round(thinned.width * 0.75)), thinned.height),
            Image.Resampling.LANCZOS,
        ),
    ]


def recover_dense_title_readings(
    recognizer,
    codec,
    crops: list[Image.Image],
    primary_readings: list[tuple[str, float]],
    *,
    batch_size: int = 12,
    width_bin: int = 32,
    max_width: int = MAX_LINE_WIDTH,
) -> tuple[list[tuple[str, float]], list[dict]]:
    """Reread only visually dense titles through audited deterministic views.

    Selection is target-free and fail-closed.  A relative confidence gain can
    never select an empty/short or low-absolute-confidence hallucination, and
    visible numeric facts must remain unchanged.
    """

    if len(crops) != len(primary_readings):
        raise ValueError("crops and primary_readings must have identical lengths")
    eligible: list[tuple[int, dict[str, float], list[Image.Image]]] = []
    for index, crop in enumerate(crops):
        # The selector requires an absolute +0.04 confidence increase.  Since
        # confidence is bounded by one, a reading above 0.96 can never be
        # replaced.  Do not run the three deterministic rereads when their
        # result is mathematically unable to affect the evidence.
        if float(primary_readings[index][1]) > 0.96:
            continue
        metrics = _dense_title_visual_metrics(crop)
        if (
            metrics["inkDensity"] >= 0.44
            and metrics["tightAspect"] >= 2.5
            and metrics["tightHeight"] >= 36
        ):
            eligible.append(
                (index, metrics, _dense_title_normalization_candidates(crop))
            )
    if not eligible:
        return list(primary_readings), []
    alternatives = _read_crops_batch(
        recognizer,
        codec,
        [candidate for _, _, candidates in eligible for candidate in candidates],
        batch_size=batch_size,
        width_bin=width_bin,
        max_width=max_width,
    )
    recovered = list(primary_readings)
    audit: list[dict] = []
    offset = 0
    for index, metrics, candidates in eligible:
        candidate_readings = alternatives[offset : offset + len(candidates)]
        offset += len(candidates)
        primary_text, primary_confidence = primary_readings[index]
        selected_text, selected_confidence = max(
            candidate_readings, key=lambda reading: reading[1]
        )
        facts_preserved, fact_reason = candidate_preserves_visible_facts(
            primary_text, selected_text
        )
        reasons = []
        if selected_confidence < primary_confidence + 0.04:
            reasons.append("insufficient-relative-confidence")
        if selected_confidence < 0.90:
            reasons.append("insufficient-absolute-confidence")
        if len(selected_text.strip()) < 4:
            reasons.append("incomplete-candidate")
        if not facts_preserved:
            reasons.append(fact_reason)
        selected = not reasons
        if selected:
            recovered[index] = (selected_text, selected_confidence)
        audit.append(
            {
                "regionIndex": index,
                "metrics": metrics,
                "primaryText": primary_text,
                "primaryConfidence": primary_confidence,
                "candidateText": selected_text,
                "candidateConfidence": selected_confidence,
                "selected": selected,
                "reasons": reasons or ["audited-dense-title-recovery"],
            }
        )
    return recovered, audit


def _numeric_alternate_views(crop: Image.Image) -> list[Image.Image]:
    grayscale = crop.convert("L")
    sharpened = ImageEnhance.Contrast(grayscale).enhance(1.35)
    sharpened = sharpened.filter(ImageFilter.UnsharpMask(radius=1.0, percent=130, threshold=2))
    return [ImageOps.autocontrast(grayscale, cutoff=1), sharpened]


def summarize_numeric_multiview_audit(
    primary_text: str,
    alternate_texts: list[str],
) -> dict:
    """Report numeric agreement without choosing or inventing a value.

    This is deliberately an evidence gate rather than another OCR router. A
    numeric fact is stable only when both deterministic visual rereads expose
    exactly the same ordered numeric facts as the selected primary reading.
    Disagreement sends the observation to review; it never selects the most
    popular value or synthesizes a replacement.
    """

    primary_signature = tuple(extract_numeric_facts(primary_text))
    observed_signatures = [
        tuple(extract_numeric_facts(text)) for text in alternate_texts
    ]
    stable = bool(primary_signature) and len(observed_signatures) == 2 and all(
        signature == primary_signature for signature in observed_signatures
    )
    return {
        "status": "stable" if stable else "needs_review",
        "primaryFacts": list(primary_signature),
        "alternateFacts": [list(signature) for signature in observed_signatures],
        "alternateTexts": list(alternate_texts),
        "requiredAgreement": "primary-plus-two-deterministic-visual-rereads",
        "valueSelection": "none",
    }


def _read_crops_sparse_numeric_anchor_batch(
    primary,
    primary_codec,
    anchor,
    anchor_codec,
    crops: list[Image.Image],
    *,
    confidence_threshold: float = 0.93,
    batch_size: int = 12,
    width_bin: int = 32,
    max_width: int = MAX_LINE_WIDTH,
    segment_long_lines: bool = False,
) -> tuple[list[tuple[str, float]], list[str]]:
    """Run the anchor only for low-confidence visible numeric facts."""

    primary_readings = _read_crops_batch(
        primary,
        primary_codec,
        crops,
        batch_size=batch_size,
        width_bin=width_bin,
        max_width=max_width,
        segment_long_lines=segment_long_lines,
    )
    reasons = ["primary_fast_path"] * len(crops)
    wake_indices = [
        index
        for index, (text, confidence) in enumerate(primary_readings)
        if extract_numeric_facts(text) and confidence < confidence_threshold
    ]
    if not wake_indices:
        return primary_readings, reasons
    anchor_readings = _read_crops_batch(
        anchor,
        anchor_codec,
        [crops[index] for index in wake_indices],
        batch_size=batch_size,
        width_bin=width_bin,
        max_width=max_width,
        segment_long_lines=segment_long_lines,
    )
    anchor_by_index = dict(zip(wake_indices, anchor_readings))
    disagreement_indices = []
    for index in wake_indices:
        primary_facts = extract_numeric_facts(primary_readings[index][0])
        anchor_facts = extract_numeric_facts(anchor_by_index[index][0])
        if primary_facts == anchor_facts:
            reasons[index] = "numeric_anchor_agreement"
        elif len(primary_facts) != len(anchor_facts):
            reasons[index] = "numeric_fact_cardinality_guard"
        else:
            disagreement_indices.append(index)
    alternate_crops = [
        view for index in disagreement_indices for view in _numeric_alternate_views(crops[index])
    ]
    primary_alternates = _read_crops_batch(
        primary,
        primary_codec,
        alternate_crops,
        batch_size=batch_size,
        width_bin=width_bin,
        max_width=max_width,
        segment_long_lines=segment_long_lines,
    )
    anchor_alternates = _read_crops_batch(
        anchor,
        anchor_codec,
        alternate_crops,
        batch_size=batch_size,
        width_bin=width_bin,
        max_width=max_width,
        segment_long_lines=segment_long_lines,
    )
    routed = list(primary_readings)
    for offset, index in enumerate(disagreement_indices):
        observations = [primary_readings[index][0], anchor_by_index[index][0]]
        for primary_reading, anchor_reading in zip(
            primary_alternates[offset * 2 : offset * 2 + 2],
            anchor_alternates[offset * 2 : offset * 2 + 2],
        ):
            observations.extend([primary_reading[0], anchor_reading[0]])
        selected, reason = select_numeric_by_multiview_consensus(
            primary_readings[index][0], anchor_by_index[index][0], observations
        )
        confidence = (
            anchor_by_index[index][1]
            if selected == anchor_by_index[index][0]
            else primary_readings[index][1]
        )
        routed[index] = (selected, confidence)
        reasons[index] = reason
    return routed, reasons


def _read_crops_sparse_language_specialist_batch(
    anchor,
    anchor_codec,
    specialist,
    specialist_codec,
    language_model: CharacterLanguageModel,
    crops: list[Image.Image],
    *,
    wake_threshold: float = 0.80,
    routing_profile: str = "uncertainty",
    confidence_margin: float = 0.01,
    language_tolerance: float = 0.0,
    max_disagreement_edits: int | None = 2,
    require_hangul_dominant: bool = True,
    batch_size: int = 12,
    width_bin: int = 32,
    max_width: int = MAX_LINE_WIDTH,
    segment_long_lines: bool = False,
    lexicon_counts: dict[str, int] | None = None,
) -> tuple[list[tuple[str, float]], list[str]]:
    """Wake a complete Korean specialist only for uncertain anchor lines.

    The global anchor reads every line. The specialist is not evaluated for
    confident, structurally valid output, and a specialist result is accepted
    only through the frozen target-free fact, lexicon, script, confidence and
    language guards. This keeps the page path faithful to the line experiment
    instead of silently substituting the older always-on visual-only pair.
    """

    anchor_readings = _read_crops_batch(
        anchor,
        anchor_codec,
        crops,
        batch_size=batch_size,
        width_bin=width_bin,
        max_width=max_width,
        segment_long_lines=segment_long_lines,
    )
    reasons = ["anchor_fast_path"] * len(crops)
    if routing_profile not in {"uncertainty", "mixed-script"}:
        raise ValueError("B_CORE_NATIVE_OCR_SPECIALIST_ROUTING_PROFILE_INVALID")
    wake_indices = []
    for index, (text, confidence) in enumerate(anchor_readings):
        eligible = (
            mixed_script_specialist_wakeup_eligible(
                text,
                confidence,
                wake_threshold,
            )
            if routing_profile == "mixed-script"
            else specialist_wakeup_eligible(text, confidence, wake_threshold)
        )
        if eligible:
            wake_indices.append(index)
    if not wake_indices:
        return anchor_readings, reasons
    specialist_readings = _read_crops_batch(
        specialist,
        specialist_codec,
        [crops[index] for index in wake_indices],
        batch_size=batch_size,
        width_bin=width_bin,
        max_width=max_width,
        segment_long_lines=segment_long_lines,
    )
    routed = list(anchor_readings)
    for index, (specialist_text, specialist_confidence) in zip(
        wake_indices, specialist_readings
    ):
        anchor_text, anchor_confidence = anchor_readings[index]
        selected, reason = select_conservative_specialist(
            anchor_text,
            anchor_confidence,
            language_model.score(anchor_text),
            specialist_text,
            specialist_confidence,
            language_model.score(specialist_text),
            confidence_margin=confidence_margin,
            language_tolerance=language_tolerance,
            max_disagreement_edits=max_disagreement_edits,
            lexicon_counts=lexicon_counts,
            require_hangul_dominant=require_hangul_dominant,
        )
        routed[index] = (
            selected,
            (
                specialist_confidence
                if reason in {"specialist_selected", "anchor_integrity_recovery"}
                else anchor_confidence
            ),
        )
        reasons[index] = reason
    return routed, reasons


def _load_visual_specialist_pair(
    anchor_path: Path,
    specialist_path: Path,
    vocabulary_size: int,
) -> BCoreVisualSpecialistPair:
    """Load a visual-only specialist while sharing the anchor decoder."""

    anchor_state = paddle.load(str(anchor_path / "recognizer.pdparams"))
    specialist_state = paddle.load(str(specialist_path / "recognizer.pdparams"))
    state = {}
    for key, value in anchor_state.items():
        if key.startswith("visual."):
            state[f"anchor_visual.{key[len('visual.'): ]}"] = value
        elif key.startswith("context.") or key.startswith("output."):
            state[key] = value
    for key, value in specialist_state.items():
        if key.startswith("visual."):
            state[f"specialist_visual.{key[len('visual.'): ]}"] = value
    pair = BCoreVisualSpecialistPair(vocabulary_size)
    pair.set_state_dict(state)
    pair.eval()
    return pair


def _read_crops_routed_batch(
    pair: BCoreVisualSpecialistPair,
    codec,
    language_model: CharacterLanguageModel,
    crops: list[Image.Image],
    *,
    confidence_margin: float,
    max_disagreement_edits: int | None = 1,
    batch_size: int = 12,
    width_bin: int = 32,
    max_width: int = MAX_LINE_WIDTH,
    segment_long_lines: bool = False,
    lexicon_counts: dict[str, int] | None = None,
) -> tuple[list[tuple[str, float]], list[str]]:
    """Run both visual streams and select the specialist fail-closed."""

    if not crops:
        return [], []
    if segment_long_lines:
        segments, counts = _split_crops_for_recognition(crops, max_width=max_width)
        if len(segments) != len(crops):
            readings, segment_reasons = _read_crops_routed_batch(
                pair,
                codec,
                language_model,
                segments,
                confidence_margin=confidence_margin,
                max_disagreement_edits=max_disagreement_edits,
                batch_size=batch_size,
                width_bin=width_bin,
                max_width=max_width,
                segment_long_lines=False,
                lexicon_counts=lexicon_counts,
            )
            restored_reasons = []
            offset = 0
            for count in counts:
                values = segment_reasons[offset : offset + count]
                offset += count
                restored_reasons.append(
                    values[0] if len(set(values)) == 1 else "segmented:" + ",".join(values)
                )
            return _join_segmented_readings(readings, counts), restored_reasons
    prepared = [_recognizer_input(crop, max_width=max_width)[0] for crop in crops]
    buckets: dict[int, list[int]] = {}
    for index, image in enumerate(prepared):
        width = image.shape[2]
        bucket = (width + width_bin - 1) // width_bin
        buckets.setdefault(bucket, []).append(index)
    readings: list[tuple[str, float] | None] = [None] * len(crops)
    reasons: list[str | None] = [None] * len(crops)
    for indices in buckets.values():
        for start in range(0, len(indices), batch_size):
            batch_indices = indices[start : start + batch_size]
            maximum_width = max(prepared[source_index].shape[2] for source_index in batch_indices)
            values = np.zeros((len(batch_indices), 1, 48, maximum_width), dtype="float32")
            input_widths = []
            for batch_index, source_index in enumerate(batch_indices):
                image = prepared[source_index]
                width = image.shape[2]
                values[batch_index, :, :, :width] = image
                input_widths.append(width)
            width_tensor = paddle.to_tensor(input_widths, dtype="int64")
            anchor_logits, specialist_logits = pair.forward_pair(
                paddle.to_tensor(values),
                input_widths=width_tensor,
            )
            feature_widths = ((width_tensor + 3) // 4).numpy().tolist()
            for batch_index, source_index in enumerate(batch_indices):
                length = feature_widths[batch_index]
                anchor_text, anchor_confidence = _decode_logits(
                    anchor_logits[batch_index, :length], codec
                )
                specialist_text, specialist_confidence = _decode_logits(
                    specialist_logits[batch_index, :length], codec
                )
                selected, reason = select_conservative_specialist(
                    anchor_text,
                    anchor_confidence,
                    language_model.score(anchor_text),
                    specialist_text,
                    specialist_confidence,
                    language_model.score(specialist_text),
                    confidence_margin=confidence_margin,
                    max_disagreement_edits=max_disagreement_edits,
                    lexicon_counts=lexicon_counts,
                )
                selected_confidence = (
                    specialist_confidence
                    if reason in {"specialist_selected", "anchor_integrity_recovery"}
                    else anchor_confidence
                )
                readings[source_index] = (selected, selected_confidence)
                reasons[source_index] = reason
    return (
        [reading if reading is not None else ("", 0.0) for reading in readings],
        [reason if reason is not None else "missing" for reason in reasons],
    )


def _orientation_score(
    detector,
    recognizer,
    codec,
    source: Image.Image,
    canvas: int,
    threshold: float,
    language_model: CharacterLanguageModel | None,
    recognizer_batch_size: int = 12,
    recognizer_width_bin: int = 32,
    scan_layout_hint: bool = False,
    maximum_regions: int = 32,
    masks: np.ndarray | None = None,
) -> tuple[float, dict[str, float | int], np.ndarray]:
    if masks is None:
        masks = _predict(detector, source, canvas, threshold)
    geometry = reconstruct_table_geometry(masks, np.asarray(source))
    candidates = [
        region
        for region in geometry["textRegions"]
        if region["x1"] - region["x0"] >= 6 and region["y1"] - region["y0"] >= 3
    ]
    if len(candidates) > maximum_regions:
        step = len(candidates) / maximum_regions
        candidates = [
            candidates[min(len(candidates) - 1, int(index * step))]
            for index in range(maximum_regions)
        ]
    readings = _read_crops_batch(
        recognizer,
        codec,
        [_crop(source, region) for region in candidates],
        batch_size=recognizer_batch_size,
        width_bin=recognizer_width_bin,
    )
    useful = [(text, confidence) for text, confidence in readings if SEMANTIC_CHARACTER.search(text)]
    if not useful:
        return (
            -100.0,
            {"regions": len(candidates), "usefulReadings": 0, "medianConfidence": 0.0},
            masks,
        )
    confidences = [confidence for _, confidence in useful]
    language_scores = [language_model.score(text) for text, _ in useful] if language_model else []
    useful_ratio = len(useful) / max(1, len(candidates))
    median_confidence = statistics.median(confidences)
    median_language = statistics.median(language_scores) if language_scores else 0.0
    layout_coverage = (
        horizontal_text_coverage(np.asarray(source)) if scan_layout_hint else 0.0
    )
    score = (
        median_confidence
        + useful_ratio * 0.20
        + max(-6.0, min(0.0, median_language)) * 0.045
        + layout_coverage * 0.20
    )
    return (
        score,
        {
            "regions": len(candidates),
            "usefulReadings": len(useful),
            "usefulRatio": useful_ratio,
            "medianConfidence": median_confidence,
            "medianLanguageScore": median_language,
            "horizontalTextCoverage": layout_coverage,
        },
        masks,
    )


def _normalize_orientation(
    detector,
    recognizer,
    codec,
    source: Image.Image,
    canvas: int,
    threshold: float,
    language_model: CharacterLanguageModel | None,
    recognizer_batch_size: int = 12,
    recognizer_width_bin: int = 32,
    scan_layout_hint: bool = False,
) -> tuple[Image.Image, dict[str, object], np.ndarray]:
    evaluations: dict[str, dict[str, object]] = {}
    candidate_angles, axis_prefilter = _orientation_candidate_angles(source)
    candidates = {
        angle: rotate_for_orientation(source, angle)
        for angle in candidate_angles
    }
    # A full audit changes only how many text regions are sampled.  It must
    # inspect the same detector output as the probe, rather than issue another
    # equivalent GPU forward pass for every candidate rotation.
    candidate_masks = {
        angle: _predict(detector, candidate, canvas, threshold)
        for angle, candidate in candidates.items()
    }
    # Most ordinary documents need only discriminate upright from upside-down.
    # Probe evenly distributed regions first and accept that result only when
    # every observed signal is already decisive.  Any weak, sparse, or
    # conflicting page falls through to the existing full 32-region audit.
    probe_evaluations: dict[str, dict[str, object]] = {}
    probe_masks: dict[int, np.ndarray] = {}
    for angle, candidate in candidates.items():
        score, metrics, masks = _orientation_score(
            detector,
            recognizer,
            codec,
            candidate,
            canvas,
            threshold,
            language_model,
            recognizer_batch_size,
            recognizer_width_bin,
            scan_layout_hint,
            maximum_regions=6,
            masks=candidate_masks[angle],
        )
        probe_evaluations[str(angle)] = {"score": score, **metrics}
        probe_masks[angle] = masks
    probe_selected = max(
        candidates,
        key=lambda angle: float(probe_evaluations[str(angle)]["score"]),
    )
    probe_scores = sorted(
        (float(value["score"]) for value in probe_evaluations.values()),
        reverse=True,
    )
    probe_margin = (
        probe_scores[0] - probe_scores[1] if len(probe_scores) > 1 else float("inf")
    )
    probe_metrics = probe_evaluations[str(probe_selected)]
    if (
        probe_margin >= 0.25
        and int(probe_metrics["regions"]) >= 4
        and float(probe_metrics["usefulRatio"]) >= 0.95
        and float(probe_metrics["medianConfidence"]) >= 0.985
    ):
        return (
            candidates[probe_selected],
            {
                "mode": "native-high-confidence-probe",
                "selectedDegrees": probe_selected,
                "axisPrefilter": axis_prefilter,
                "candidates": probe_evaluations,
                "probeMargin": probe_margin,
                "fullAuditSkipped": True,
            },
            probe_masks[probe_selected],
        )
    full_masks: dict[int, np.ndarray] = {}
    for angle, candidate in candidates.items():
        score, metrics, masks = _orientation_score(
            detector,
            recognizer,
            codec,
            candidate,
            canvas,
            threshold,
            language_model,
            recognizer_batch_size,
            recognizer_width_bin,
            scan_layout_hint,
            masks=candidate_masks[angle],
        )
        evaluations[str(angle)] = {"score": score, **metrics}
        full_masks[angle] = masks
    selected_angle = max(candidates, key=lambda angle: float(evaluations[str(angle)]["score"]))
    return (
        candidates[selected_angle],
        {
            "mode": "native-recognizer-language-consensus",
            "selectedDegrees": selected_angle,
            "axisPrefilter": axis_prefilter,
            "candidates": evaluations,
            "probe": probe_evaluations,
            "probeMargin": probe_margin,
            "fullAuditSkipped": False,
        },
        full_masks[selected_angle],
    )


def _recover_region_padding(
    recognizer,
    codec,
    source: Image.Image,
    regions: list[dict[str, int]],
    readings: list[tuple[str, float]],
    language_model: CharacterLanguageModel | None,
    language_weight: float,
    recognizer_batch_size: int = 12,
    recognizer_width_bin: int = 32,
) -> tuple[list[tuple[str, float]], int]:
    """Retry language-poor regions with scale-aware context padding."""

    if language_model is None:
        return readings, 0
    floor = language_model.calibration.get("trainP01", -20.0)
    retry_indices = []
    retry_crops = []
    for index, (region, (text, confidence)) in enumerate(zip(regions, readings)):
        score = language_model.score(text)
        semantic_length = len(SEMANTIC_CHARACTER.findall(text))
        if confidence < 0.85 or score >= floor or not 2 <= semantic_length <= 4:
            continue
        height = region["y1"] - region["y0"] + 1
        padding = min(18, max(6, round(height * 0.18)))
        retry_indices.append(index)
        retry_crops.append(_crop(source, region, padding=padding))
    recovered = list(readings)
    alternatives = _read_crops_batch(
        recognizer,
        codec,
        retry_crops,
        batch_size=recognizer_batch_size,
        width_bin=recognizer_width_bin,
    )
    for index, alternative in zip(retry_indices, alternatives):
        current = recovered[index]
        current_language = language_model.score(current[0])
        alternative_language = language_model.score(alternative[0])
        current_rank = math.log(max(current[1], 1e-6)) + language_weight * current_language
        alternative_rank = math.log(max(alternative[1], 1e-6)) + language_weight * alternative_language
        # Context expansion must improve both independent signals.  A gain in
        # neural confidence is not allowed to buy a less plausible string (or
        # vice versa), which prevents local recovery from regressing another
        # document family.
        if (
            alternative_rank > current_rank
            and alternative[1] >= current[1] + 0.01
            and alternative_language >= current_language + 0.02
        ):
            recovered[index] = alternative
    return recovered, len(retry_indices)


def _decode_ctc_max_path_candidates(
    log_probabilities: np.ndarray,
    codec,
    language_model: CharacterLanguageModel,
    language_weight: float,
    beam_width: int = 12,
    token_top_k: int = 4,
    candidate_limit: int = 8,
) -> list[dict[str, object]]:
    """Keep diverse high-scoring CTC paths without probability aggregation."""

    beams: dict[tuple[tuple[int, ...], int], float] = {((), 0): 0.0}
    for row in log_probabilities:
        count = min(token_top_k, len(row))
        token_ids = np.argpartition(-row, count - 1)[:count].tolist()
        if 0 not in token_ids:
            token_ids.append(0)
        expanded: dict[tuple[tuple[int, ...], int], float] = {}
        for (sequence, previous), score in beams.items():
            for token_value in token_ids:
                token = int(token_value)
                next_sequence = sequence
                if token != 0 and token != previous:
                    next_sequence = sequence + (token,)
                key = (next_sequence, token)
                candidate_score = score + float(row[token])
                if candidate_score > expanded.get(key, -float("inf")):
                    expanded[key] = candidate_score
        beams = dict(
            sorted(expanded.items(), key=lambda item: item[1], reverse=True)[:beam_width]
        )
    candidates: dict[str, tuple[float, float, float]] = {}
    time_steps = max(1, len(log_probabilities))
    for (sequence, _), acoustic_total in beams.items():
        raw_path = [token for item in sequence for token in (item, 0)]
        text = canonicalize_ocr_content(codec.decode_ctc(raw_path))
        if not text:
            continue
        acoustic_score = acoustic_total / time_steps
        language_score = language_model.score(text)
        rank = acoustic_score + language_weight * language_score
        if rank > candidates.get(text, (-float("inf"), 0.0, 0.0))[0]:
            candidates[text] = (rank, acoustic_score, language_score)
    ordered = sorted(candidates.items(), key=lambda item: item[1][0], reverse=True)
    return [
        {
            "text": text,
            "confidence": math.exp(min(0.0, acoustic_score)),
            "acousticScore": acoustic_score,
            "languageScore": language_score,
            "combinedScore": rank,
            "provenance": "owned_ctc_visual_path",
            "ctcDecoder": "max_path_diversity",
        }
        for text, (rank, acoustic_score, language_score) in ordered[:candidate_limit]
    ]


def _decode_ctc_prefix_candidates(
    log_probabilities: np.ndarray,
    codec,
    language_model: CharacterLanguageModel,
    language_weight: float,
    beam_width: int = 12,
    token_top_k: int = 4,
    candidate_limit: int = 8,
) -> list[dict[str, object]]:
    """Return only strings that are reachable from recognizer CTC paths.

    Document context may later *rank* these alternatives, but it must never
    introduce a spelling that the pixels did not produce.  Keeping acoustic
    and language evidence separate also makes context-driven corrections
    auditable instead of silently rewriting the OCR result.
    """

    negative_infinity = -float("inf")

    def log_add(*values: float) -> float:
        finite = [value for value in values if math.isfinite(value)]
        if not finite:
            return negative_infinity
        maximum = max(finite)
        return maximum + math.log(sum(math.exp(value - maximum) for value in finite))

    # Each prefix keeps separate probability mass for paths ending in blank
    # and non-blank.  Multiple pixel paths that collapse to the same text are
    # summed instead of retaining only their single best path.
    beams: dict[tuple[int, ...], tuple[float, float]] = {
        (): (0.0, negative_infinity)
    }
    for row in log_probabilities:
        count = min(token_top_k, len(row))
        token_ids = np.argpartition(-row, count - 1)[:count].tolist()
        if 0 not in token_ids:
            token_ids.append(0)
        expanded: dict[tuple[int, ...], tuple[float, float]] = {}

        def update(
            prefix: tuple[int, ...],
            *,
            blank: float = negative_infinity,
            nonblank: float = negative_infinity,
        ) -> None:
            old_blank, old_nonblank = expanded.get(
                prefix, (negative_infinity, negative_infinity)
            )
            expanded[prefix] = (
                log_add(old_blank, blank),
                log_add(old_nonblank, nonblank),
            )

        for prefix, (blank_score, nonblank_score) in beams.items():
            total_score = log_add(blank_score, nonblank_score)
            update(prefix, blank=total_score + float(row[0]))
            for token_value in token_ids:
                token = int(token_value)
                if token == 0:
                    continue
                token_score = float(row[token])
                if prefix and token == prefix[-1]:
                    # Repeating without an intervening blank collapses to the
                    # existing prefix; repeating after blank creates a second
                    # visible token.
                    update(prefix, nonblank=nonblank_score + token_score)
                    update(
                        prefix + (token,),
                        nonblank=blank_score + token_score,
                    )
                else:
                    update(
                        prefix + (token,),
                        nonblank=total_score + token_score,
                    )
        beams = dict(
            sorted(
                expanded.items(),
                key=lambda item: log_add(item[1][0], item[1][1]),
                reverse=True,
            )[:beam_width]
        )
    candidates: dict[str, tuple[float, float, float]] = {}
    time_steps = max(1, len(log_probabilities))
    for sequence, (blank_score, nonblank_score) in beams.items():
        acoustic_total = log_add(blank_score, nonblank_score)
        raw_path = [token for item in sequence for token in (item, 0)]
        text = canonicalize_ocr_content(codec.decode_ctc(raw_path))
        if not text:
            continue
        acoustic_score = acoustic_total / time_steps
        language_score = language_model.score(text)
        rank = acoustic_score + language_weight * language_score
        if rank > candidates.get(text, (-float("inf"), 0.0, 0.0))[0]:
            candidates[text] = (rank, acoustic_score, language_score)
    ordered = sorted(candidates.items(), key=lambda item: item[1][0], reverse=True)
    return [
        {
            "text": text,
            "confidence": math.exp(min(0.0, acoustic_score)),
            "acousticScore": acoustic_score,
            "languageScore": language_score,
            "combinedScore": rank,
            "provenance": "owned_ctc_visual_path",
            "ctcDecoder": "prefix_probability_sum",
        }
        for text, (rank, acoustic_score, language_score) in ordered[:candidate_limit]
    ]


def _decode_ctc_beam_candidates(
    log_probabilities: np.ndarray,
    codec,
    language_model: CharacterLanguageModel,
    language_weight: float,
    beam_width: int = 12,
    token_top_k: int = 4,
    candidate_limit: int = 8,
    decoder_mode: str = "max_path",
) -> list[dict[str, object]]:
    """Fuse two independent owned CTC searches in a comparable rank space.

    Prefix probability summation is calibrated but tends to collapse diversity;
    max-path search preserves alternate glyph readings.  Relative per-decoder
    scores allow their candidate sets to be merged without pretending their raw
    acoustic scales are identical.
    """

    if decoder_mode == "max_path":
        return _decode_ctc_max_path_candidates(
            log_probabilities,
            codec,
            language_model,
            language_weight,
            beam_width,
            token_top_k,
            candidate_limit,
        )
    if decoder_mode == "prefix":
        return _decode_ctc_prefix_candidates(
            log_probabilities,
            codec,
            language_model,
            language_weight,
            beam_width,
            token_top_k,
            candidate_limit,
        )
    if decoder_mode != "dual":
        raise ValueError("B_CORE_NATIVE_OCR_CTC_DECODER_MODE_INVALID")
    decoder_results = (
        _decode_ctc_prefix_candidates(
            log_probabilities,
            codec,
            language_model,
            language_weight,
            beam_width,
            token_top_k,
            candidate_limit,
        ),
        _decode_ctc_max_path_candidates(
            log_probabilities,
            codec,
            language_model,
            language_weight,
            beam_width,
            token_top_k,
            candidate_limit,
        ),
    )
    merged: dict[str, dict[str, object]] = {}
    support: dict[str, set[str]] = {}
    for candidates in decoder_results:
        if not candidates:
            continue
        top_score = float(candidates[0]["combinedScore"])
        for rank_index, candidate in enumerate(candidates):
            text = str(candidate["text"])
            decoder = str(candidate["ctcDecoder"])
            relative_score = float(candidate["combinedScore"]) - top_score
            support.setdefault(text, set()).add(decoder)
            current = merged.get(text)
            if current is None or relative_score > float(current["decoderRelativeScore"]):
                merged[text] = {
                    **candidate,
                    "decoderRawCombinedScore": candidate["combinedScore"],
                    "decoderRelativeScore": relative_score,
                    "decoderRank": rank_index,
                }
    fused: list[dict[str, object]] = []
    for text, candidate in merged.items():
        decoders = sorted(support[text])
        consensus_bonus = 0.025 * math.log1p(max(0, len(decoders) - 1))
        fused.append(
            {
                **candidate,
                "decoderSupport": decoders,
                "decoderSupportCount": len(decoders),
                "decoderConsensusBonus": consensus_bonus,
                "combinedScore": float(candidate["decoderRelativeScore"])
                + consensus_bonus,
            }
        )
    fused.sort(key=lambda item: float(item["combinedScore"]), reverse=True)
    return fused[:candidate_limit]


def _beam_read_crop_candidates(
    recognizer,
    codec,
    crop: Image.Image,
    language_model: CharacterLanguageModel,
    language_weight: float,
    beam_width: int = 12,
    token_top_k: int = 4,
    candidate_limit: int = 8,
    max_width: int = MAX_LINE_WIDTH,
) -> list[dict[str, object]]:
    logits = recognizer(paddle.to_tensor(_recognizer_input(crop, max_width=max_width)))
    log_probabilities = paddle.nn.functional.log_softmax(logits, axis=-1).numpy()[0]
    return _decode_ctc_beam_candidates(
        log_probabilities,
        codec,
        language_model,
        language_weight,
        beam_width=beam_width,
        token_top_k=token_top_k,
        candidate_limit=candidate_limit,
    )


def _beam_read_crop(
    recognizer,
    codec,
    crop: Image.Image,
    language_model: CharacterLanguageModel,
    language_weight: float,
    beam_width: int = 12,
    token_top_k: int = 4,
    max_width: int = MAX_LINE_WIDTH,
) -> tuple[str, float, float]:
    candidates = _beam_read_crop_candidates(
        recognizer,
        codec,
        crop,
        language_model,
        language_weight,
        beam_width=beam_width,
        token_top_k=token_top_k,
        max_width=max_width,
    )
    if not candidates:
        return "", 0.0, -20.0
    best = candidates[0]
    return (
        str(best["text"]),
        float(best["confidence"]),
        float(best["languageScore"]),
    )


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("pdf", type=Path)
    parser.add_argument("--detector", type=Path, required=True)
    parser.add_argument("--recognizer", type=Path, required=True)
    parser.add_argument(
        "--specialist-recognizer",
        type=Path,
        help="Optional visual-only specialist. Remains fail-closed behind the conservative router.",
    )
    parser.add_argument(
        "--korean-language-specialist",
        type=Path,
        help=(
            "Optional complete Korean specialist woken only for uncertain anchor lines. "
            "Unlike --specialist-recognizer, this preserves the specialist's full decoder."
        ),
    )
    parser.add_argument("--korean-specialist-wake-threshold", type=float, default=0.80)
    parser.add_argument(
        "--korean-specialist-routing-profile",
        choices=("uncertainty", "mixed-script"),
        default="uncertainty",
        help="Target-free sparse wake-up profile; the default preserves existing behavior.",
    )
    parser.add_argument("--korean-specialist-confidence-margin", type=float, default=0.01)
    parser.add_argument("--korean-specialist-language-tolerance", type=float, default=0.0)
    parser.add_argument(
        "--korean-specialist-max-disagreement-edits", type=int, default=2
    )
    parser.add_argument(
        "--korean-specialist-require-hangul-dominant",
        action=argparse.BooleanOptionalAction,
        default=True,
    )
    parser.add_argument(
        "--numeric-anchor-recognizer",
        type=Path,
        help="Optional full recognizer woken only for low-confidence numeric facts.",
    )
    parser.add_argument("--numeric-anchor-confidence-threshold", type=float, default=0.93)
    parser.add_argument(
        "--numeric-multiview-audit",
        choices=("off", "review"),
        default="review",
        help=(
            "For scan pages, reread numeric rows through two deterministic visual views. "
            "Disagreement is review-only and never changes a value."
        ),
    )
    parser.add_argument("--specialist-confidence-margin", type=float, default=0.0125)
    parser.add_argument(
        "--specialist-max-disagreement-edits",
        type=int,
        default=1,
        help="Reject open-text specialist changes larger than this target-free edit budget.",
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--pages", required=True)
    parser.add_argument("--dpi", type=int, default=144)
    parser.add_argument("--threshold", type=float, default=0.65)
    parser.add_argument("--language-model", type=Path)
    parser.add_argument(
        "--lexicon",
        type=Path,
        help="Optional train-only/B_Core lexical memory used only as a fail-closed routing guard.",
    )
    parser.add_argument("--language-weight", type=float, default=0.18)
    parser.add_argument(
        "--arithmetic-candidate-expansion",
        choices=("off", "review"),
        default="review",
        help=(
            "Lazily expand owned CTC candidates only for rows that violate a "
            "repeated table equation after the fast pass."
        ),
    )
    parser.add_argument("--arithmetic-candidate-limit", type=int, default=32)
    parser.add_argument("--arithmetic-beam-width", type=int, default=32)
    parser.add_argument(
        "--domain-ontology",
        type=Path,
        help=(
            "Optional data-defined vertical ontology. It annotates canonical "
            "semantic values but never overwrites OCR source text."
        ),
    )
    parser.add_argument(
        "--domain-language-terms",
        type=Path,
        help="Optional companion terminology JSON for --domain-ontology.",
    )
    parser.add_argument(
        "--domain-ontology-aliases",
        type=Path,
        help=(
            "Optional field-synonym JSON for --domain-ontology. Aliases annotate "
            "canonical semantic values and never overwrite OCR source text."
        ),
    )
    parser.add_argument("--orientation", choices=("auto", "0", "90", "180", "270"), default="auto")
    parser.add_argument(
        "--orientation-observer",
        choices=("auto", "off", "paddlex"),
        default="auto",
        help=(
            "Use an already-installed independent document-orientation classifier. "
            "Auto never downloads a model and falls back to native-only evidence."
        ),
    )
    parser.add_argument(
        "--orientation-observer-model",
        type=Path,
        default=Path.home()
        / ".paddlex"
        / "official_models"
        / "PP-LCNet_x1_0_doc_ori",
    )
    parser.add_argument("--orientation-observer-min-confidence", type=float, default=0.55)
    parser.add_argument("--orientation-observer-strong-confidence", type=float, default=0.80)
    parser.add_argument("--orientation-native-ambiguity-margin", type=float, default=0.15)
    parser.add_argument("--orientation-observer-review-confidence", type=float, default=0.70)
    parser.add_argument(
        "--independent-ocr-observer",
        choices=("off", "auto", "review"),
        default="off",
        help=(
            "External OCR is disabled by default. Auto/review are research-only teacher "
            "modes; they never own final facts."
        ),
    )
    parser.add_argument(
        "--independent-ocr-detector",
        type=Path,
        default=Path.home()
        / ".paddlex"
        / "official_models"
        / "PP-OCRv5_mobile_det",
    )
    parser.add_argument(
        "--independent-ocr-recognizer",
        type=Path,
        default=Path.home()
        / ".paddlex"
        / "official_models"
        / "korean_PP-OCRv5_mobile_rec",
    )
    parser.add_argument("--independent-ocr-min-confidence", type=float, default=0.75)
    parser.add_argument(
        "--independent-ocr-bcore-min-confidence",
        type=float,
        default=0.80,
        help=(
            "Minimum confidence required when B_Core independently rereads an "
            "observer-proposed pixel crop."
        ),
    )
    parser.add_argument("--independent-ocr-review-ratio", type=float, default=0.40)
    parser.add_argument("--independent-ocr-limit-side-len", type=int, default=1280)
    parser.add_argument(
        "--scan-rule-recovery",
        choices=("auto", "on", "off"),
        default="auto",
        help="Recover continuous scan rules only on pages without a trustworthy PDF text layer.",
    )
    parser.add_argument(
        "--scan-text-detection",
        choices=("auto", "learned", "cv", "component", "multiview", "hybrid"),
        default="auto",
        help="Use CV text lines only when the frozen recognizer says learned scan geometry is poor.",
    )
    parser.add_argument("--device", default="gpu:0")
    parser.add_argument(
        "--recognizer-batch-size",
        type=int,
        default=12,
        help="Native recognizer batch size. Use 1 with width-bin 1 for the exact-width reference path.",
    )
    parser.add_argument(
        "--recognizer-width-bin",
        type=int,
        default=32,
        help="Group crops by this input-width quantum before masked batching.",
    )
    parser.add_argument(
        "--row-recognizer-max-width",
        type=int,
        default=MAX_LINE_WIDTH,
        help="Preserve additional horizontal detail for long reconstructed rows.",
    )
    parser.add_argument(
        "--long-line-segmentation",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Split only low-resolution, over-compressed rows at proven visual whitespace.",
    )
    parser.add_argument(
        "--dense-title-normalization",
        action=argparse.BooleanOptionalAction,
        default=True,
        help=(
            "Reread visually dense title crops through audited deterministic views; "
            "selection remains confidence-, completeness-, and fact-gated."
        ),
    )
    parser.add_argument(
        "--high-confidence-row-fast-path",
        action=argparse.BooleanOptionalAction,
        default=False,
        help=(
            "For non-tabular scan pages only, stop after every direct row reading "
            "independently satisfies the normal evidence gate. Ambiguous pages keep "
            "the full region-plus-row verification path."
        ),
    )
    parser.add_argument(
        "--high-confidence-row-floor",
        type=float,
        default=0.985,
        help="Minimum direct-row confidence for the non-tabular fast-path gate.",
    )
    arguments = parser.parse_args(argv)
    if arguments.recognizer_batch_size < 1:
        parser.error("--recognizer-batch-size must be at least 1")
    if not 0.0 <= arguments.high_confidence_row_floor <= 1.0:
        parser.error("--high-confidence-row-floor must be between 0 and 1")
    if arguments.arithmetic_candidate_limit < 1:
        parser.error("--arithmetic-candidate-limit must be at least 1")
    if arguments.arithmetic_beam_width < 1:
        parser.error("--arithmetic-beam-width must be at least 1")
    if arguments.recognizer_width_bin < 1:
        parser.error("--recognizer-width-bin must be at least 1")
    if arguments.row_recognizer_max_width < 64:
        parser.error("--row-recognizer-max-width must be at least 64")
    if arguments.specialist_max_disagreement_edits < 0:
        parser.error("--specialist-max-disagreement-edits must be non-negative")
    if arguments.korean_specialist_max_disagreement_edits < 0:
        parser.error("--korean-specialist-max-disagreement-edits must be non-negative")
    if not 0.0 <= arguments.korean_specialist_wake_threshold <= 1.0:
        parser.error("--korean-specialist-wake-threshold must be between 0 and 1")
    for name in (
        "orientation_observer_min_confidence",
        "orientation_observer_strong_confidence",
        "orientation_observer_review_confidence",
    ):
        if not 0.0 <= getattr(arguments, name) <= 1.0:
            parser.error(f"--{name.replace('_', '-')} must be between 0 and 1")
    if arguments.orientation_native_ambiguity_margin < 0.0:
        parser.error("--orientation-native-ambiguity-margin must be non-negative")
    if not 0.0 <= arguments.independent_ocr_min_confidence <= 1.0:
        parser.error("--independent-ocr-min-confidence must be between 0 and 1")
    if not 0.0 <= arguments.independent_ocr_bcore_min_confidence <= 1.0:
        parser.error("--independent-ocr-bcore-min-confidence must be between 0 and 1")
    if not 0.0 <= arguments.independent_ocr_review_ratio <= 1.0:
        parser.error("--independent-ocr-review-ratio must be between 0 and 1")
    if arguments.independent_ocr_limit_side_len < 320:
        parser.error("--independent-ocr-limit-side-len must be at least 320")
    optional_experts = sum(
        value is not None
        for value in (
            arguments.specialist_recognizer,
            arguments.korean_language_specialist,
            arguments.numeric_anchor_recognizer,
        )
    )
    if optional_experts > 1:
        parser.error(
            "choose only one of --specialist-recognizer, "
            "--korean-language-specialist or --numeric-anchor-recognizer"
        )
    detector_metadata, detector, codec, recognizer = load_warm_primary_models(
        arguments.detector,
        arguments.recognizer,
        arguments.device,
    )
    language_model = CharacterLanguageModel.load(arguments.language_model) if arguments.language_model else None
    lexicon = OcrLexicon.load(arguments.lexicon) if arguments.lexicon else None
    domain_ontology = (
        load_long_term_repair_ontology(
            arguments.domain_ontology,
            arguments.domain_language_terms,
            arguments.domain_ontology_aliases,
        )
        if arguments.domain_ontology
        else None
    )
    specialist_pair = None
    korean_language_specialist = None
    korean_language_specialist_codec = None
    numeric_anchor = None
    numeric_anchor_codec = None
    orientation_observer = None
    orientation_observer_status = "disabled"
    if arguments.orientation == "auto" and arguments.orientation_observer != "off":
        try:
            orientation_observer = load_warm_orientation_observer(
                arguments.orientation_observer_model
            )
            orientation_observer_status = "loaded-local-model"
        except FileNotFoundError:
            if arguments.orientation_observer == "paddlex":
                parser.error(
                    "--orientation-observer paddlex requires a locally installed model"
                )
            orientation_observer_status = "local-model-unavailable"
    independent_ocr_observer = None
    independent_ocr_observer_status = (
        "disabled"
        if arguments.independent_ocr_observer == "off"
        else "not-yet-woken"
    )
    if arguments.specialist_recognizer:
        if language_model is None:
            parser.error("--specialist-recognizer requires --language-model")
        specialist_codec = load_codec(arguments.specialist_recognizer / "codec.json")
        if codec.tokens != specialist_codec.tokens:
            parser.error("specialist and anchor codecs do not match")
        specialist_pair = _load_visual_specialist_pair(
            arguments.recognizer,
            arguments.specialist_recognizer,
            codec.size,
        )
    if arguments.korean_language_specialist:
        if language_model is None or lexicon is None:
            parser.error(
                "--korean-language-specialist requires both --language-model and --lexicon"
            )
        korean_language_specialist_codec = load_codec(
            arguments.korean_language_specialist / "codec.json"
        )
        if codec.tokens != korean_language_specialist_codec.tokens:
            parser.error("Korean specialist and anchor codecs do not match")
        korean_language_specialist = BCoreLineRecognizer(codec.size)
        korean_language_specialist.set_state_dict(
            paddle.load(
                str(arguments.korean_language_specialist / "recognizer.pdparams")
            )
        )
        korean_language_specialist.eval()
    if arguments.numeric_anchor_recognizer:
        numeric_anchor_codec = load_codec(arguments.numeric_anchor_recognizer / "codec.json")
        if codec.tokens != numeric_anchor_codec.tokens:
            parser.error("numeric anchor and primary codecs do not match")
        numeric_anchor = BCoreLineRecognizer(codec.size)
        numeric_anchor.set_state_dict(
            paddle.load(str(arguments.numeric_anchor_recognizer / "recognizer.pdparams"))
        )
        numeric_anchor.eval()
    with paddle.no_grad():
        if specialist_pair is not None:
            specialist_pair.forward_pair(
                paddle.zeros([1, 1, 48, 128], dtype="float32"),
                input_widths=paddle.to_tensor([128], dtype="int64"),
            )
        if korean_language_specialist is not None:
            korean_language_specialist(paddle.zeros([1, 1, 48, 128], dtype="float32"))
        if numeric_anchor is not None:
            numeric_anchor(paddle.zeros([1, 1, 48, 128], dtype="float32"))
        synchronize_accelerator(arguments.device)
    arguments.output.mkdir(parents=True, exist_ok=True)
    with (
        tempfile.TemporaryDirectory(prefix="bcore-table-read-") as temporary,
        paddle.no_grad(),
        pdfplumber.open(arguments.pdf.resolve()) as pdf_document,
    ):
        page_numbers = [int(value) for value in arguments.pages.split(",")]
        rendered_pages = _render_pages(
            arguments.pdf.resolve(),
            page_numbers,
            arguments.dpi,
            Path(temporary),
        )
        for page_number in page_numbers:
            rendered = rendered_pages[page_number]
            with Image.open(rendered) as opened:
                source = opened.convert("L")
                vector_words = pdf_document.pages[page_number - 1].extract_words(
                    x_tolerance=1.5,
                    y_tolerance=3.0,
                    keep_blank_chars=False,
                )
                vector_word_count = len(vector_words)
                vector_text_evidence = compile_vector_text_evidence(vector_words)
                page_kind = (
                    "vector"
                    if vector_word_count >= 8 and vector_text_evidence["trustworthy"]
                    else "scan"
                )
                page_started = time.perf_counter()
                orientation_detector_masks: np.ndarray | None = None
                if arguments.orientation == "auto":
                    unrotated_source = source
                    source, orientation, orientation_detector_masks = _normalize_orientation(
                        detector,
                        recognizer,
                        codec,
                        source,
                        int(detector_metadata["canvas"]),
                        arguments.threshold,
                        language_model,
                        arguments.recognizer_batch_size,
                        arguments.recognizer_width_bin,
                        page_kind == "scan",
                    )
                    candidate_scores = [
                        float(candidate["score"])
                        for candidate in orientation["candidates"].values()
                    ]
                    score_margin = max(candidate_scores) - min(candidate_scores)
                    orientation["scoreMargin"] = score_margin
                    # A PDF is a container, not an orientation contract. Scanned
                    # reports commonly mix portrait, landscape and upside-down
                    # inserts. Reusing one page's decision silently corrupts later
                    # pages, so every page must establish its own orientation.
                    orientation["decisionScope"] = "page"
                    observer_degrees = None
                    observer_confidence = None
                    if orientation_observer is not None:
                        observer_degrees, observer_confidence = observe_orientation(
                            orientation_observer, unrotated_source
                        )
                    decision = choose_orientation(
                        native_degrees=int(orientation["selectedDegrees"]),
                        native_margin=score_margin,
                        native_candidates={
                            int(value) for value in orientation["candidates"]
                        },
                        observer_degrees=observer_degrees,
                        observer_confidence=observer_confidence,
                        observer_minimum_confidence=arguments.orientation_observer_min_confidence,
                        observer_strong_confidence=arguments.orientation_observer_strong_confidence,
                        native_ambiguity_margin=arguments.orientation_native_ambiguity_margin,
                        review_confidence=arguments.orientation_observer_review_confidence,
                    )
                    orientation["nativeSelectedDegrees"] = orientation["selectedDegrees"]
                    orientation["selectedDegrees"] = decision.selected_degrees
                    orientation["ensemble"] = {
                        **decision.receipt(),
                        "observer": "PP-LCNet_x1_0_doc_ori",
                        "observerStatus": orientation_observer_status,
                        "modelPath": (
                            str(arguments.orientation_observer_model.resolve())
                            if orientation_observer is not None
                            else None
                        ),
                    }
                    if decision.selected_degrees != decision.native_degrees:
                        source = rotate_for_orientation(
                            unrotated_source, decision.selected_degrees
                        )
                        # The native scorer's detector output belongs to a
                        # different page rotation.  Never reuse it across an
                        # observer override, even when the two images look
                        # superficially similar.
                        orientation_detector_masks = None
                else:
                    selected_degrees = int(arguments.orientation)
                    source = rotate_for_orientation(source, selected_degrees)
                    orientation = {"mode": "forced", "selectedDegrees": selected_degrees}
                resolved_scan_text_mode = arguments.scan_text_detection
                if arguments.scan_text_detection == "auto":
                    selected_metrics = (
                        orientation.get("candidates", {}).get(
                            str(orientation.get("selectedDegrees", 0)), {}
                        )
                        if isinstance(orientation, dict)
                        else {}
                    )
                    useful_ratio = float(selected_metrics.get("usefulRatio", 1.0))
                    median_confidence = float(
                        selected_metrics.get("medianConfidence", 1.0)
                    )
                    resolved_scan_text_mode = resolve_scan_text_mode(
                        arguments.scan_text_detection,
                        page_kind=page_kind,
                        useful_ratio=useful_ratio,
                        median_confidence=median_confidence,
                    )
                orientation_at = time.perf_counter()
                started = time.perf_counter()
                reused_orientation_detector_masks = orientation_detector_masks is not None
                masks = (
                    orientation_detector_masks
                    if orientation_detector_masks is not None
                    else _predict(
                        detector,
                        source,
                        int(detector_metadata["canvas"]),
                        arguments.threshold,
                    )
                )
                recovery_applied = arguments.scan_rule_recovery == "on" or (
                    arguments.scan_rule_recovery == "auto" and page_kind == "scan"
                )
                scan_rule_diagnostics: dict[str, object] = {}
                if recovery_applied:
                    masks = recover_scan_rule_masks(
                        masks,
                        np.asarray(source),
                        diagnostics=scan_rule_diagnostics,
                    )
                scan_text_diagnostics: dict[str, object] = {
                    "requestedMode": arguments.scan_text_detection,
                    "mode": resolved_scan_text_mode,
                    "applied": False,
                }
                component_text_regions = None
                if page_kind == "scan" and resolved_scan_text_mode != "learned":
                    if resolved_scan_text_mode in ("component", "multiview"):
                        scan_support, detected_text_diagnostics = (
                            detect_multiview_component_text_support(np.asarray(source))
                            if resolved_scan_text_mode == "multiview"
                            else detect_component_cluster_text_support(np.asarray(source))
                        )
                        component_text_regions = detected_text_diagnostics.pop(
                            "regions"
                        )
                    else:
                        scan_support, detected_text_diagnostics = detect_scan_text_support(
                            np.asarray(source)
                        )
                    if resolved_scan_text_mode in ("cv", "component", "multiview"):
                        masks[0] = scan_support
                    else:
                        masks[0] = masks[0] | scan_support
                    scan_text_diagnostics.update(detected_text_diagnostics)
                    scan_text_diagnostics["applied"] = True
                detected_at = time.perf_counter()
                geometry = reconstruct_table_geometry(
                    masks,
                    np.asarray(source),
                    snap_to_source=not recovery_applied,
                    subtract_rule_masks_from_text=not (
                        page_kind == "scan" and resolved_scan_text_mode != "learned"
                    ),
                )
                if component_text_regions is not None:
                    geometry = replace_text_regions(
                        geometry,
                        component_text_regions,
                    )
                    scan_text_diagnostics["directRegionGeometry"] = True
                    scan_text_diagnostics["directRegionCount"] = len(
                        component_text_regions
                    )
                geometry_at = time.perf_counter()
                routing_reasons: list[str] = []
                dense_title_audit: list[dict] = []
                lexical_spacing_recoveries: list[dict] = []

                def recover_fragmented_spacing(text: str) -> str:
                    if lexicon is None:
                        return text
                    recovered, decisions = lexicon.recover_fragmented_spacing(text)
                    lexical_spacing_recoveries.extend(decisions)
                    return recovered

                def read_primary(
                    crops: list[Image.Image], *, max_width: int = MAX_LINE_WIDTH
                ) -> list[tuple[str, float]]:
                    if numeric_anchor is not None:
                        readings, reasons = _read_crops_sparse_numeric_anchor_batch(
                            recognizer,
                            codec,
                            numeric_anchor,
                            numeric_anchor_codec,
                            crops,
                            confidence_threshold=arguments.numeric_anchor_confidence_threshold,
                            batch_size=arguments.recognizer_batch_size,
                            width_bin=arguments.recognizer_width_bin,
                            max_width=max_width,
                            segment_long_lines=arguments.long_line_segmentation,
                        )
                        routing_reasons.extend(reasons)
                    elif korean_language_specialist is not None:
                        readings, reasons = _read_crops_sparse_language_specialist_batch(
                            recognizer,
                            codec,
                            korean_language_specialist,
                            korean_language_specialist_codec,
                            language_model,
                            crops,
                            wake_threshold=arguments.korean_specialist_wake_threshold,
                            routing_profile=arguments.korean_specialist_routing_profile,
                            confidence_margin=arguments.korean_specialist_confidence_margin,
                            language_tolerance=arguments.korean_specialist_language_tolerance,
                            max_disagreement_edits=arguments.korean_specialist_max_disagreement_edits,
                            require_hangul_dominant=arguments.korean_specialist_require_hangul_dominant,
                            batch_size=arguments.recognizer_batch_size,
                            width_bin=arguments.recognizer_width_bin,
                            max_width=max_width,
                            segment_long_lines=arguments.long_line_segmentation,
                            lexicon_counts=lexicon.counts,
                        )
                        routing_reasons.extend(reasons)
                    elif specialist_pair is None:
                        readings = _read_crops_batch(
                            recognizer,
                            codec,
                            crops,
                            batch_size=arguments.recognizer_batch_size,
                            width_bin=arguments.recognizer_width_bin,
                            max_width=max_width,
                            segment_long_lines=arguments.long_line_segmentation,
                        )
                    else:
                        readings, reasons = _read_crops_routed_batch(
                            specialist_pair,
                            codec,
                            language_model,
                            crops,
                            confidence_margin=arguments.specialist_confidence_margin,
                            max_disagreement_edits=arguments.specialist_max_disagreement_edits,
                            batch_size=arguments.recognizer_batch_size,
                            width_bin=arguments.recognizer_width_bin,
                            max_width=max_width,
                            segment_long_lines=arguments.long_line_segmentation,
                            lexicon_counts=lexicon.counts if lexicon else None,
                        )
                        routing_reasons.extend(reasons)
                    if arguments.dense_title_normalization:
                        readings, audit = recover_dense_title_readings(
                            recognizer,
                            codec,
                            crops,
                            readings,
                            batch_size=arguments.recognizer_batch_size,
                            width_bin=arguments.recognizer_width_bin,
                            max_width=max_width,
                        )
                        dense_title_audit.extend(audit)
                    return readings

                region_crops = [_crop(source, region) for region in geometry["textRegions"]]
                region_visual_evidence = [
                    visual_glyph_evidence(crop) for crop in region_crops
                ]
                region_routing_start = len(routing_reasons)
                region_readings = read_primary(region_crops)
                region_routing_reasons = routing_reasons[region_routing_start:]
                region_readings, padding_recovery_regions = _recover_region_padding(
                    recognizer,
                    codec,
                    source,
                    geometry["textRegions"],
                    region_readings,
                    language_model,
                    arguments.language_weight,
                    arguments.recognizer_batch_size,
                    arguments.recognizer_width_bin,
                )
                region_reading_modes = {
                    (region["x0"], region["y0"], region["x1"], region["y1"]): "greedy_or_padding"
                    for region in geometry["textRegions"]
                }
                region_visual_candidates = {
                    (region["x0"], region["y0"], region["x1"], region["y1"]): [
                        {
                            "text": text,
                            "confidence": float(confidence),
                            "source": "greedy_or_padding",
                        }
                    ]
                    for region, (text, confidence) in zip(
                        geometry["textRegions"], region_readings
                    )
                }
                cell_beam_attempts = 0
                cell_beam_selections = 0
                cell_beam_milliseconds = 0.0
                cell_beam_guard_reasons: Counter[str] = Counter()
                if language_model is not None:
                    table_region_keys = {
                        (region["x0"], region["y0"], region["x1"], region["y1"])
                        for cell in geometry["cells"]
                        for region in cell["textRegions"]
                    }
                    language_floor = language_model.calibration.get("trainP01", -20.0)
                    recovered_region_readings = list(region_readings)
                    for index, (region, crop, (text, confidence)) in enumerate(
                        zip(geometry["textRegions"], region_crops, region_readings)
                    ):
                        key = (region["x0"], region["y0"], region["x1"], region["y1"])
                        language_score = language_model.score(text)
                        if key not in table_region_keys or not beam_recovery_is_eligible(
                            text,
                            confidence,
                            language_score,
                            language_floor,
                        ):
                            continue
                        cell_beam_attempts += 1
                        beam_started = time.perf_counter()
                        beam_candidates = _beam_read_crop_candidates(
                            recognizer,
                            codec,
                            crop,
                            language_model,
                            arguments.language_weight,
                        )
                        region_visual_candidates[key].extend(
                            candidate
                            for candidate in beam_candidates
                            if candidate.get("text")
                            and candidate.get("text")
                            not in {
                                existing.get("text")
                                for existing in region_visual_candidates[key]
                            }
                        )
                        if beam_candidates:
                            best_beam = beam_candidates[0]
                            alternative_text = str(best_beam["text"])
                            alternative_confidence = float(best_beam["confidence"])
                            alternative_language = float(best_beam["languageScore"])
                        else:
                            alternative_text = ""
                            alternative_confidence = 0.0
                            alternative_language = -20.0
                        cell_beam_milliseconds += (time.perf_counter() - beam_started) * 1000
                        current_rank = math.log(max(confidence, 1e-6)) + (
                            arguments.language_weight * language_score
                        )
                        alternative_rank = math.log(max(alternative_confidence, 1e-6)) + (
                            arguments.language_weight * alternative_language
                        )
                        fact_safe, fact_guard_reason = candidate_preserves_visible_facts(
                            text, alternative_text
                        )
                        cell_beam_guard_reasons[fact_guard_reason] += 1
                        if fact_safe and alternative_rank > current_rank:
                            recovered_region_readings[index] = (
                                alternative_text,
                                alternative_confidence,
                            )
                            region_reading_modes[key] = "table_cell_ctc_beam"
                            cell_beam_selections += 1
                    region_readings = recovered_region_readings
                region_readings = [
                    (recover_fragmented_spacing(text), confidence)
                    for text, confidence in region_readings
                ]
                recognition_by_region = {
                    (region["x0"], region["y0"], region["x1"], region["y1"]): reading
                    for region, reading in zip(geometry["textRegions"], region_readings)
                }
                visual_evidence_by_region = {
                    (region["x0"], region["y0"], region["x1"], region["y1"]): evidence
                    for region, evidence in zip(
                        geometry["textRegions"], region_visual_evidence
                    )
                }
                synchronize_accelerator(arguments.device)
                regions_at = time.perf_counter()
                beam_rows = 0
                beam_milliseconds = 0.0
                numeric_audit_milliseconds = 0.0
                numeric_audit_rows = 0
                numeric_audit_disagreements = 0
                arithmetic_expansion_milliseconds = 0.0
                arithmetic_expansion_rows = 0
                arithmetic_expansion_candidates = 0
                for region_index, region in enumerate(geometry["textRegions"]):
                    key = (region["x0"], region["y0"], region["x1"], region["y1"])
                    region["text"], region["confidence"] = recognition_by_region[key]
                    region["selectedReading"] = region_reading_modes[key]
                    region["visualTextEvidence"] = visual_evidence_by_region[key]
                    region["visualCandidates"] = region_visual_candidates[key]
                    if region_routing_reasons:
                        region["specialistRouting"] = region_routing_reasons[region_index]
                native_region_consensus: dict[str, object] = {
                    "status": "not_available",
                    "approvedEvidence": [],
                    "counts": {
                        "candidates": len(geometry["textRegions"]),
                        "approved": 0,
                        "needsReview": len(geometry["textRegions"]),
                    },
                }
                if page_kind == "scan" and korean_language_specialist is not None:
                    anchor_consensus_readings = _read_crops_batch(
                        recognizer,
                        codec,
                        region_crops,
                        batch_size=arguments.recognizer_batch_size,
                        width_bin=arguments.recognizer_width_bin,
                    )
                    specialist_consensus_readings = _read_crops_batch(
                        korean_language_specialist,
                        korean_language_specialist_codec,
                        region_crops,
                        batch_size=arguments.recognizer_batch_size,
                        width_bin=arguments.recognizer_width_bin,
                    )
                    native_region_consensus = {
                        "status": "evaluated",
                        **compile_native_region_consensus(
                            geometry["textRegions"],
                            anchor_consensus_readings,
                            specialist_consensus_readings,
                            anchor_model_name=arguments.recognizer.name,
                            specialist_model_name=arguments.korean_language_specialist.name,
                        ),
                    }
                row_boxes = [
                    {
                        "x0": min(region["x0"] for region in row["regions"]),
                        "y0": row["y0"],
                        "x1": max(region["x1"] for region in row["regions"]),
                        "y1": row["y1"],
                    }
                    for row in geometry["textRows"]
                ]
                row_boxes = _trim_overlapping_row_boxes(row_boxes)
                row_crops = [_crop(source, row_box, padding=4) for row_box in row_boxes]
                row_visual_evidence = [visual_glyph_evidence(crop) for crop in row_crops]
                primary_row_groups = [
                    _split_multiline_row_crop(crop) for crop in row_crops
                ]
                row_crop_groups = [
                    [
                        tile
                        for segment in group
                        for tile in _split_multicolumn_header_crop(segment)
                    ]
                    for group in primary_row_groups
                ]
                row_segment_counts = [len(group) for group in row_crop_groups]
                multiline_rows = sum(len(group) > 1 for group in primary_row_groups)
                multicolumn_rows = sum(
                    len(refined) > len(primary)
                    for primary, refined in zip(primary_row_groups, row_crop_groups)
                )
                fast_region_only_rows = [False] * len(geometry["textRows"])
                if (
                    arguments.high_confidence_row_fast_path
                    and not geometry["cells"]
                    and specialist_pair is None
                    and korean_language_specialist is None
                    and numeric_anchor is None
                    and language_model is None
                ):
                    for row_index, (row, segment_count) in enumerate(
                        zip(geometry["textRows"], row_segment_counts)
                    ):
                        joined_regions = _regions_for_joined_row(
                            row["regions"], page_width=source.width
                        )
                        joined_text = " ".join(
                            str(region.get("text") or "")
                            for region in joined_regions
                        ).strip()
                        normalized_text = recover_fragmented_spacing(
                            normalize_numeric_token_glyph_confusions(joined_text)
                        )
                        fast_region_only_rows[row_index] = bool(
                            segment_count == 1
                            and len(joined_regions) == len(row["regions"])
                            and joined_text
                            and not validate_ocr_text(normalized_text)
                            and all(
                                float(region.get("confidence") or 0.0)
                                >= arguments.high_confidence_row_floor
                                and bool(
                                    region.get("visualTextEvidence", {}).get(
                                        "hasGlyphEvidence"
                                    )
                                )
                                for region in joined_regions
                            )
                        )
                fallback_row_indices = [
                    index
                    for index, is_fast in enumerate(fast_region_only_rows)
                    if not is_fast
                ]
                row_routing_start = len(routing_reasons)
                fallback_segment_counts = [
                    row_segment_counts[index] for index in fallback_row_indices
                ]
                direct_segment_readings = read_primary(
                    [
                        segment
                        for index in fallback_row_indices
                        for segment in row_crop_groups[index]
                    ],
                    max_width=arguments.row_recognizer_max_width,
                )
                fallback_direct_readings = _join_segmented_readings(
                    direct_segment_readings,
                    fallback_segment_counts,
                )
                direct_readings = [("", 0.0)] * len(geometry["textRows"])
                for row_index, reading in zip(
                    fallback_row_indices, fallback_direct_readings
                ):
                    direct_readings[row_index] = reading
                segment_routing_reasons = routing_reasons[row_routing_start:]
                row_routing_reasons: list[str] = []
                if segment_routing_reasons:
                    reason_offset = 0
                    for row_index, count in enumerate(row_segment_counts):
                        if fast_region_only_rows[row_index]:
                            row_routing_reasons.append("high_confidence_region_join")
                            continue
                        grouped = segment_routing_reasons[
                            reason_offset : reason_offset + count
                        ]
                        reason_offset += count
                        row_routing_reasons.append(
                            grouped[0]
                            if len(grouped) == 1
                            else "multiline[" + ",".join(grouped) + "]"
                        )
                for row_index, (row, row_box, direct_reading) in enumerate(
                    zip(geometry["textRows"], row_boxes, direct_readings)
                ):
                    if row_routing_reasons:
                        row["directSpecialistRouting"] = row_routing_reasons[row_index]
                    for region in row["regions"]:
                        key = (region["x0"], region["y0"], region["x1"], region["y1"])
                        region["text"], region["confidence"] = recognition_by_region[key]
                    joined_regions = _regions_for_joined_row(
                        row["regions"], page_width=source.width
                    )
                    row["regionJoinedText"] = " ".join(
                        region["text"] for region in joined_regions
                    )
                    total_weight = sum(
                        max(1, len(region["text"])) for region in joined_regions
                    )
                    row["regionJoinedConfidence"] = sum(
                        region["confidence"] * max(1, len(region["text"]))
                        for region in joined_regions
                    ) / max(1, total_weight)
                    row["directText"], row["directConfidence"] = direct_reading
                    row["directLanguageScore"] = language_model.score(row["directText"]) if language_model else 0.0
                    row["regionJoinedLanguageScore"] = (
                        language_model.score(row["regionJoinedText"]) if language_model else 0.0
                    )
                    direct_rank = math.log(max(row["directConfidence"], 1e-6)) + (
                        arguments.language_weight * row["directLanguageScore"]
                    )
                    joined_rank = math.log(max(row["regionJoinedConfidence"], 1e-6)) + (
                        arguments.language_weight * row["regionJoinedLanguageScore"]
                    )
                    beam_rank = -float("inf")
                    language_floor = (
                        language_model.calibration.get("trainP01", -20.0) if language_model else -20.0
                    )
                    if language_model and beam_recovery_is_eligible(
                        row["directText"],
                        row["directConfidence"],
                        row["directLanguageScore"],
                        language_floor,
                    ):
                        beam_started = time.perf_counter()
                        row["beamCandidates"] = _beam_read_crop_candidates(
                            recognizer,
                            codec,
                            _crop(source, row_box, padding=4),
                            language_model,
                            arguments.language_weight,
                            max_width=arguments.row_recognizer_max_width,
                        )
                        if row["beamCandidates"]:
                            best_beam = row["beamCandidates"][0]
                            row["beamText"] = str(best_beam["text"])
                            row["beamConfidence"] = float(best_beam["confidence"])
                            row["beamLanguageScore"] = float(best_beam["languageScore"])
                        else:
                            row["beamText"] = ""
                            row["beamConfidence"] = 0.0
                            row["beamLanguageScore"] = -20.0
                        beam_milliseconds += (time.perf_counter() - beam_started) * 1000
                        beam_rows += 1
                        beam_rank = math.log(max(row["beamConfidence"], 1e-6)) + (
                            arguments.language_weight * row["beamLanguageScore"]
                        )
                        non_beam_text = (
                            row["directText"]
                            if direct_rank >= joined_rank
                            else row["regionJoinedText"]
                        )
                        beam_fact_safe, beam_guard_reason = candidate_preserves_visible_facts(
                            non_beam_text, row["beamText"]
                        )
                        row["beamFactGuard"] = beam_guard_reason
                        if not beam_fact_safe:
                            beam_rank = -float("inf")
                    if direct_rank >= joined_rank and direct_rank >= beam_rank:
                        row["text"] = row["directText"]
                        row["confidence"] = row["directConfidence"]
                        row["selectedReading"] = "direct_row"
                    elif joined_rank >= beam_rank:
                        row["text"] = row["regionJoinedText"]
                        row["confidence"] = row["regionJoinedConfidence"]
                        row["selectedReading"] = "joined_regions"
                    else:
                        row["text"] = row["beamText"]
                        row["confidence"] = row["beamConfidence"]
                        row["selectedReading"] = "ctc_beam"
                    row["text"] = normalize_numeric_token_glyph_confusions(row["text"])
                    row["text"] = recover_fragmented_spacing(row["text"])
                    row["selectionScore"] = max(direct_rank, joined_rank, beam_rank)
                    selected_language_score = {
                        "direct_row": row["directLanguageScore"],
                        "joined_regions": row["regionJoinedLanguageScore"],
                        "ctc_beam": row.get("beamLanguageScore", -20.0),
                    }[row["selectedReading"]]
                    row["validationIssues"] = validate_ocr_text(row["text"])
                    if orientation.get("ensemble", {}).get("needsReview"):
                        row["validationIssues"] = sorted(
                            set(row["validationIssues"])
                            | {"ORIENTATION_DISAGREEMENT"}
                        )
                    row["visualTextEvidence"] = row_visual_evidence[row_index]
                    if row["text"] and not row_visual_evidence[row_index]["hasGlyphEvidence"]:
                        row["validationIssues"] = sorted(
                            set(row["validationIssues"]) | {"NO_VISUAL_GLYPH_EVIDENCE"}
                        )
                    row["numericTokens"] = extract_numeric_tokens(row["text"])
                    acceptance_confidence_floor = raster_row_confidence_floor(
                        page_kind
                    )
                    language_recovery_grounded = (
                        page_kind != "scan"
                        or scan_row_language_recovery_is_grounded(
                            row, acceptance_confidence_floor
                        )
                    )
                    if not language_recovery_grounded:
                        row["validationIssues"] = sorted(
                            set(row["validationIssues"])
                            | {"BEAM_ONLY_SCAN_FACT"}
                        )
                    row["accepted"] = (
                        row["confidence"] >= acceptance_confidence_floor
                        and selected_language_score >= language_floor
                        and language_recovery_grounded
                        and not row["validationIssues"]
                    )
                    row["status"] = "accepted" if row["accepted"] else "needs_review"
                    row["languageFloor"] = language_floor
                    row["acceptanceConfidenceFloor"] = (
                        acceptance_confidence_floor
                    )

                geometry["borderlessTables"] = reconstruct_repeated_numeric_tables(
                    geometry["textRows"]
                )
                if (
                    language_model is not None
                    and arguments.arithmetic_candidate_expansion == "review"
                ):
                    expansion_indices = arithmetic_candidate_expansion_indices(
                        geometry["borderlessTables"]
                    )
                    if expansion_indices:
                        expansion_started = time.perf_counter()
                        for row_index in expansion_indices:
                            row = geometry["textRows"][row_index]
                            wide_candidates = _beam_read_crop_candidates(
                                recognizer,
                                codec,
                                _crop(source, row_boxes[row_index], padding=4),
                                language_model,
                                arguments.language_weight,
                                beam_width=arguments.arithmetic_beam_width,
                                token_top_k=8,
                                candidate_limit=arguments.arithmetic_candidate_limit,
                                max_width=arguments.row_recognizer_max_width,
                            )
                            existing = {
                                str(candidate.get("text") or "")
                                for candidate in row.get("beamCandidates") or []
                            }
                            additions = [
                                candidate
                                for candidate in wide_candidates
                                if candidate.get("text")
                                and str(candidate.get("text")) not in existing
                            ]
                            row.setdefault("beamCandidates", []).extend(additions)
                            row["arithmeticCandidateExpansion"] = {
                                "candidateLimit": arguments.arithmetic_candidate_limit,
                                "beamWidth": arguments.arithmetic_beam_width,
                                "addedCandidates": len(additions),
                                "selectionUsesTarget": False,
                            }
                            arithmetic_expansion_rows += 1
                            arithmetic_expansion_candidates += len(additions)
                        arithmetic_expansion_milliseconds = (
                            time.perf_counter() - expansion_started
                        ) * 1000
                        geometry["borderlessTables"] = (
                            reconstruct_repeated_numeric_tables(geometry["textRows"])
                        )
                if page_kind == "scan" and arguments.numeric_multiview_audit == "review":
                    structured_source_indices = {
                        int(row["sourceRowIndex"])
                        for table in geometry["borderlessTables"]
                        for row in (table.get("rows") or [])
                        if row.get("sourceRowIndex") is not None
                    }
                    approved_structured_indices = {
                        int(row["sourceRowIndex"])
                        for table in geometry["borderlessTables"]
                        for row in (table.get("rows") or [])
                        if row.get("sourceRowIndex") is not None
                        and row.get("status") == "accepted"
                    }
                    numeric_row_indices = [
                        index
                        for index, row in enumerate(geometry["textRows"])
                        if extract_numeric_facts(str(row.get("text") or ""))
                        and (
                            index in approved_structured_indices
                            or (
                                index not in structured_source_indices
                                and row.get("status") == "accepted"
                            )
                        )
                    ]
                    if numeric_row_indices:
                        numeric_audit_started = time.perf_counter()
                        alternate_crops = [
                            view
                            for index in numeric_row_indices
                            for view in _numeric_alternate_views(
                                _crop(source, row_boxes[index], padding=4)
                            )
                        ]
                        alternate_readings = _read_crops_batch(
                            recognizer,
                            codec,
                            alternate_crops,
                            batch_size=arguments.recognizer_batch_size,
                            width_bin=arguments.recognizer_width_bin,
                            max_width=arguments.row_recognizer_max_width,
                            segment_long_lines=arguments.long_line_segmentation,
                        )
                        numeric_audit_milliseconds = (
                            time.perf_counter() - numeric_audit_started
                        ) * 1000
                        for offset, row_index in enumerate(numeric_row_indices):
                            row = geometry["textRows"][row_index]
                            audit = summarize_numeric_multiview_audit(
                                str(row.get("text") or ""),
                                [
                                    text
                                    for text, _ in alternate_readings[
                                        offset * 2 : offset * 2 + 2
                                    ]
                                ],
                            )
                            row["numericMultiviewAudit"] = audit
                            numeric_audit_rows += 1
                            if audit["status"] != "stable":
                                numeric_audit_disagreements += 1
                                row["validationIssues"] = sorted(
                                    set(row.get("validationIssues") or [])
                                    | {"NUMERIC_MULTIVIEW_DISAGREEMENT"}
                                )
                                row["accepted"] = False
                                row["status"] = "needs_review"
                    for table in geometry["borderlessTables"]:
                        for structured_row in table.get("rows") or []:
                            source_index = structured_row.get("sourceRowIndex")
                            if source_index is None:
                                continue
                            source_row = geometry["textRows"][int(source_index)]
                            audit = source_row.get("numericMultiviewAudit")
                            if audit is None:
                                continue
                            structured_row["numericMultiviewAudit"] = audit
                            if audit["status"] != "stable":
                                structured_row["validationIssues"] = sorted(
                                    set(structured_row.get("validationIssues") or [])
                                    | {"NUMERIC_MULTIVIEW_DISAGREEMENT"}
                                )
                                structured_row["status"] = "needs_review"
                synchronize_reconstructed_rows(
                    geometry["textRows"], geometry["borderlessTables"]
                )
                for cell in geometry["cells"]:
                    for region in cell["textRegions"]:
                        key = (region["x0"], region["y0"], region["x1"], region["y1"])
                        region["text"], region["confidence"] = recognition_by_region[key]
                        region["selectedReading"] = region_reading_modes[key]
                        region["visualTextEvidence"] = visual_evidence_by_region[key]
                        region["visualCandidates"] = region_visual_candidates[key]
                    cell["text"] = " ".join(region["text"] for region in cell["textRegions"] if region["text"])
                    if len(cell["textRegions"]) == 1:
                        cell["visualCandidates"] = list(
                            cell["textRegions"][0].get("visualCandidates") or []
                        )
                    else:
                        # Do not create a combinatorial cross product for a
                        # multiline/merged cell.  Its joined primary remains a
                        # visual candidate and uncertain fragments stay review-only.
                        cell["visualCandidates"] = [
                            {
                                "text": cell["text"],
                                "confidence": min(
                                    (
                                        float(region.get("confidence") or 0.0)
                                        for region in cell["textRegions"]
                                    ),
                                    default=0.0,
                                ),
                                "source": "joined_regions",
                            }
                        ]
                    cell["confidence"] = (
                        sum(region["confidence"] for region in cell["textRegions"]) / len(cell["textRegions"])
                        if cell["textRegions"]
                        else 0.0
                    )
                cell_columns: dict[tuple[int, int], list[str]] = {}
                for cell in geometry["cells"]:
                    key = (cell["tableComponentIndex"], cell["columnIndex"])
                    cell_columns.setdefault(key, []).append(cell["text"])
                numeric_columns = {
                    key
                    for key, texts in cell_columns.items()
                    if infer_numeric_column_consensus(texts)
                }
                for cell in geometry["cells"]:
                    key = (cell["tableComponentIndex"], cell["columnIndex"])
                    normalized_text = normalize_numeric_glyph_confusions(
                        cell["text"],
                        proven_numeric_column=key in numeric_columns,
                    )
                    normalized_text = normalize_numeric_token_glyph_confusions(normalized_text)
                    if normalized_text != cell["text"]:
                        cell["text"] = normalized_text
                        cell["structuralNormalization"] = "numeric-column-glyph-consensus"
                    normalized_candidates = []
                    seen_candidates = set()
                    for candidate in cell.get("visualCandidates") or []:
                        candidate_text = str(candidate.get("text") or "")
                        candidate_text = normalize_numeric_glyph_confusions(
                            candidate_text,
                            proven_numeric_column=key in numeric_columns,
                        )
                        candidate_text = normalize_numeric_token_glyph_confusions(
                            candidate_text
                        )
                        if not candidate_text or candidate_text in seen_candidates:
                            continue
                        seen_candidates.add(candidate_text)
                        normalized_candidates.append(
                            {**candidate, "text": candidate_text}
                        )
                    cell["visualCandidates"] = normalized_candidates
                    cell["validationIssues"] = validate_ocr_text(cell["text"])
                    if orientation.get("ensemble", {}).get("needsReview"):
                        cell["validationIssues"] = sorted(
                            set(cell["validationIssues"])
                            | {"ORIENTATION_DISAGREEMENT"}
                        )
                    cell["visualTextEvidence"] = {
                        "hasGlyphEvidence": any(
                            bool(
                                region.get("visualTextEvidence", {}).get(
                                    "hasGlyphEvidence"
                                )
                            )
                            for region in cell["textRegions"]
                        )
                    }
                    if cell["text"] and not cell["visualTextEvidence"]["hasGlyphEvidence"]:
                        cell["validationIssues"] = sorted(
                            set(cell["validationIssues"]) | {"NO_VISUAL_GLYPH_EVIDENCE"}
                        )
                    cell["numericTokens"] = extract_numeric_tokens(cell["text"])
                    cell["status"] = (
                        "accepted"
                        if cell["text"] and cell["confidence"] >= 0.55 and not cell["validationIssues"]
                        else "needs_review"
                    )
                    if domain_ontology is not None and cell["text"]:
                        resolution = domain_ontology.resolve(cell["text"])
                        if resolution["status"] != "unresolved":
                            cell["ontologyResolution"] = resolution
                geometry["cells"], cell_constraint_fusion = (
                    apply_geometry_cell_arithmetic_constraints(
                        geometry["cells"],
                        source_key=arguments.pdf.name,
                        page=page_number,
                    )
                )
                # Constraint changes can affect validation/status and numeric
                # evidence.  Recompute those fields from the selected visual
                # candidate before structured evidence is compiled.
                for cell in geometry["cells"]:
                    cell["validationIssues"] = validate_ocr_text(cell["text"])
                    cell["numericTokens"] = extract_numeric_tokens(cell["text"])
                    cell["status"] = (
                        "accepted"
                        if cell["text"]
                        and cell["confidence"] >= 0.55
                        and not cell["validationIssues"]
                        else "needs_review"
                    )
                disputed_numeric_rows = [
                    row
                    for row in geometry["textRows"]
                    if "NUMERIC_MULTIVIEW_DISAGREEMENT"
                    in (row.get("validationIssues") or [])
                ]
                for cell in geometry["cells"]:
                    if not cell.get("numericTokens"):
                        continue
                    if any(
                        int(cell["y1"]) >= int(row["y0"])
                        and int(cell["y0"]) <= int(row["y1"])
                        for row in disputed_numeric_rows
                    ):
                        cell["validationIssues"] = sorted(
                            set(cell.get("validationIssues") or [])
                            | {"NUMERIC_MULTIVIEW_DISAGREEMENT"}
                        )
                        cell["status"] = "needs_review"
                structured_evidence = combine_table_evidence(
                    compile_borderless_table_evidence(
                        source=arguments.pdf.name,
                        page=page_number,
                        tables=geometry["borderlessTables"],
                    ),
                    compile_ruled_table_evidence(
                        source=arguments.pdf.name,
                        page=page_number,
                        cells=geometry["cells"],
                    ),
                )
                approved_text_evidence = select_approved_text_evidence(
                    vector_text_evidence,
                    geometry["textRows"],
                )
                if page_kind == "scan" and native_region_consensus["approvedEvidence"]:
                    approved_text_evidence["rows"].extend(
                        native_region_consensus["approvedEvidence"]
                    )
                    approved_text_evidence["source"] = (
                        "native_raster_ocr_plus_bcore_owned_region_consensus"
                    )
                    approved_text_evidence["reason"] = (
                        "Plain scan spans are promoted only after B_Core CV geometry "
                        "and exact high-confidence agreement of two owned recognizers."
                    )
                native_completed_at = time.perf_counter()
                text_rows = geometry["textRows"]
                review_ratio = (
                    sum(row.get("status") != "accepted" for row in text_rows)
                    / max(1, len(text_rows))
                )
                orientation_needs_review = bool(
                    orientation.get("ensemble", {}).get("needsReview")
                )
                observer_should_wake = (
                    arguments.independent_ocr_observer == "review"
                    or (
                        arguments.independent_ocr_observer == "auto"
                        and page_kind == "scan"
                        and (
                            orientation_needs_review
                            or review_ratio >= arguments.independent_ocr_review_ratio
                        )
                    )
                )
                independent_observer_receipt: dict[str, object] = {
                    "activated": False,
                    "mode": arguments.independent_ocr_observer,
                    "reviewRatio": review_ratio,
                    "wakeThreshold": arguments.independent_ocr_review_ratio,
                    "orientationNeedsReview": orientation_needs_review,
                    "factAuthority": False,
                    "status": independent_ocr_observer_status,
                }
                independent_observer_milliseconds = 0.0
                if observer_should_wake and page_kind == "scan":
                    observer_started = time.perf_counter()
                    try:
                        if independent_ocr_observer is None:
                            independent_ocr_observer = load_local_paddlex_ocr_observer(
                                arguments.independent_ocr_detector,
                                arguments.independent_ocr_recognizer,
                                limit_side_len=arguments.independent_ocr_limit_side_len,
                                recognition_batch_size=arguments.recognizer_batch_size,
                            )
                            independent_ocr_observer_status = "loaded-local-models"
                        observer_evidence = observe_independent_ocr_page(
                            independent_ocr_observer,
                            source,
                            minimum_confidence=arguments.independent_ocr_min_confidence,
                        )
                        observer_crops = [
                            source.crop(tuple(observation["box"]))
                            for observation in observer_evidence["observations"]
                        ]
                        bcore_observer_readings = _read_crops_batch(
                            recognizer,
                            codec,
                            observer_crops,
                            batch_size=arguments.recognizer_batch_size,
                            width_bin=arguments.recognizer_width_bin,
                            max_width=arguments.row_recognizer_max_width,
                            segment_long_lines=arguments.long_line_segmentation,
                        )
                        observer_verification = verify_observations_with_bcore(
                            observer_evidence["observations"],
                            bcore_observer_readings,
                            [visual_glyph_evidence(crop) for crop in observer_crops],
                            minimum_bcore_confidence=(
                                arguments.independent_ocr_bcore_min_confidence
                            ),
                        )
                        owned_model_consensus = {
                            "approvedEvidence": [],
                            "reviewQueue": observer_verification["reviewQueue"],
                            "counts": {
                                "approved": 0,
                                "needsReview": len(observer_verification["reviewQueue"]),
                            },
                            "status": "not_requested",
                        }
                        if (
                            korean_language_specialist is not None
                            and observer_verification["reviewQueue"]
                        ):
                            review_records = observer_verification["reviewQueue"]
                            specialist_observer_readings = _read_crops_batch(
                                korean_language_specialist,
                                codec,
                                [
                                    observer_crops[int(item["observationIndex"])]
                                    for item in review_records
                                ],
                                batch_size=arguments.recognizer_batch_size,
                                width_bin=arguments.recognizer_width_bin,
                                max_width=arguments.row_recognizer_max_width,
                                segment_long_lines=arguments.long_line_segmentation,
                            )
                            owned_model_consensus = (
                                resolve_review_with_owned_model_consensus(
                                    review_records,
                                    specialist_observer_readings,
                                    anchor_model_name=arguments.recognizer.name,
                                    specialist_model_name=(
                                        arguments.korean_language_specialist.name
                                    ),
                                    minimum_confidence=(
                                        arguments.independent_ocr_bcore_min_confidence
                                    ),
                                )
                            )
                            owned_model_consensus["status"] = "evaluated"
                            observer_verification["approvedEvidence"].extend(
                                owned_model_consensus["approvedEvidence"]
                            )
                            observer_verification["reviewQueue"] = (
                                owned_model_consensus["reviewQueue"]
                            )
                            observer_verification["counts"] = {
                                "approved": len(
                                    observer_verification["approvedEvidence"]
                                ),
                                "needsReview": len(
                                    observer_verification["reviewQueue"]
                                ),
                            }
                        observer_alignment = align_observer_to_rows(
                            text_rows, observer_evidence["observations"]
                        )
                        observer_disagreement_guard = (
                            quarantine_verified_numeric_disagreements(
                                text_rows,
                                observer_evidence["observations"],
                                observer_alignment,
                                {
                                    int(item["observationIndex"])
                                    for item in observer_verification["approvedEvidence"]
                                },
                            )
                        )
                        if observer_disagreement_guard["quarantinedCount"]:
                            approved_text_evidence = select_approved_text_evidence(
                                vector_text_evidence,
                                text_rows,
                            )
                        for verified in observer_verification["approvedEvidence"]:
                            approved_text_evidence["rows"].append(
                                {
                                    "text": verified["text"],
                                    "box": verified["box"],
                                    "confidence": min(
                                        float(verified["confidence"]),
                                        float(verified["bcoreConfidence"]),
                                    ),
                                    "status": "accepted",
                                    "granularity": "span",
                                    "factContribution": "primary",
                                    "source": verified.get("source") or (
                                        "independent_detector_recognizer_plus_"
                                        "bcore_pixel_reread"
                                    ),
                                    "provenance": verified.get("provenance") or [
                                        verified["provenanceGroup"],
                                        arguments.recognizer.name,
                                    ],
                                }
                            )
                        if observer_verification["approvedEvidence"]:
                            approved_text_evidence["source"] = (
                                "native_raster_ocr_plus_independent_verified_spans"
                            )
                            approved_text_evidence["reason"] = (
                                "Independent detector/recognizer spans are admitted only "
                                "after identical B_Core pixel reread; unresolved spans remain review."
                            )
                        independent_observer_receipt = {
                            **observer_evidence,
                            "activated": True,
                            "mode": arguments.independent_ocr_observer,
                            "activationReason": (
                                "forced-review"
                                if arguments.independent_ocr_observer == "review"
                                else (
                                    "orientation-review"
                                    if orientation_needs_review
                                    else "row-review-ratio"
                                )
                            ),
                            "reviewRatio": review_ratio,
                            "wakeThreshold": arguments.independent_ocr_review_ratio,
                            "orientationNeedsReview": orientation_needs_review,
                            "alignment": observer_alignment,
                            "numericDisagreementGuard": observer_disagreement_guard,
                            "bcorePixelRereadVerification": observer_verification,
                            "ownedModelConsensus": owned_model_consensus,
                            "status": independent_ocr_observer_status,
                            "factAuthority": False,
                        }
                    except Exception as error:
                        independent_observer_receipt = {
                            **independent_observer_receipt,
                            "activated": True,
                            "status": "observer-failed",
                            "errorType": type(error).__name__,
                            "error": str(error),
                            "factAuthority": False,
                        }
                    independent_observer_milliseconds = (
                        time.perf_counter() - observer_started
                    ) * 1000
                compiled_text_evidence = compile_approved_text_evidence(
                    approved_text_evidence["rows"]
                )
                approved_text_evidence["rows"] = compiled_text_evidence["rows"]
                approved_text_evidence["compilation"] = {
                    key: value
                    for key, value in compiled_text_evidence.items()
                    if key != "rows"
                }
                synchronize_accelerator(arguments.device)
                completed_at = time.perf_counter()
                receipt = {
                    "schema": "B_CORE_NATIVE_OCR_TABLE_READ_1",
                    "source": arguments.pdf.name,
                    "page": page_number,
                    "detector": arguments.detector.name,
                    "recognizer": arguments.recognizer.name,
                    "specialistRecognizer": (
                        arguments.specialist_recognizer.name
                        if arguments.specialist_recognizer
                        else None
                    ),
                    "koreanLanguageSpecialist": (
                        arguments.korean_language_specialist.name
                        if arguments.korean_language_specialist
                        else None
                    ),
                    "koreanLanguageSpecialistRouting": (
                        {
                            "wakeThreshold": arguments.korean_specialist_wake_threshold,
                            "confidenceMargin": arguments.korean_specialist_confidence_margin,
                            "languageTolerance": arguments.korean_specialist_language_tolerance,
                            "maxDisagreementEdits": arguments.korean_specialist_max_disagreement_edits,
                            "requireHangulDominant": arguments.korean_specialist_require_hangul_dominant,
                            "policy": "target-free-sparse-full-recognizer",
                        }
                        if arguments.korean_language_specialist
                        else None
                    ),
                    "numericAnchorRecognizer": (
                        arguments.numeric_anchor_recognizer.name
                        if arguments.numeric_anchor_recognizer
                        else None
                    ),
                    "recognizerBatching": {
                        "batchSize": arguments.recognizer_batch_size,
                        "widthBin": arguments.recognizer_width_bin,
                        "rowMaxWidth": arguments.row_recognizer_max_width,
                        "paddingMode": "layer-masked",
                        "longLineSegmentation": arguments.long_line_segmentation,
                        "longLineSegmentationContract": (
                            "low-resolution-over-compressed-visual-whitespace-only"
                        ),
                    },
                    "threshold": arguments.threshold,
                    "dpi": arguments.dpi,
                    "pageKind": page_kind,
                    "vectorTextWords": vector_word_count,
                    "vectorTextEvidence": vector_text_evidence,
                    "approvedTextEvidence": approved_text_evidence,
                    "scanRuleRecovery": {
                        "mode": arguments.scan_rule_recovery,
                        "applied": recovery_applied,
                        "diagnostics": scan_rule_diagnostics,
                    },
                    "scanTextDetection": scan_text_diagnostics,
                    "nativeRegionConsensus": {
                        key: value
                        for key, value in native_region_consensus.items()
                        if key != "approvedEvidence"
                    },
                    "orientation": orientation,
                    "elapsedMilliseconds": (completed_at - page_started) * 1000,
                    "latency": {
                        "orientationMilliseconds": (orientation_at - page_started) * 1000,
                        "detectorMilliseconds": (detected_at - started) * 1000,
                        "reusedOrientationDetectorMasks": reused_orientation_detector_masks,
                        "geometryMilliseconds": (geometry_at - detected_at) * 1000,
                        "regionRecognitionMilliseconds": (regions_at - geometry_at) * 1000,
                        "rowAndBeamMilliseconds": (native_completed_at - regions_at) * 1000,
                        "beamMilliseconds": beam_milliseconds,
                        "beamRows": beam_rows,
                        "highConfidenceRegionJoinRows": sum(fast_region_only_rows),
                        "paddingRecoveryRegions": padding_recovery_regions,
                        "cellBeamMilliseconds": cell_beam_milliseconds,
                        "cellBeamAttempts": cell_beam_attempts,
                        "cellBeamSelections": cell_beam_selections,
                        "cellBeamFactGuards": dict(cell_beam_guard_reasons),
                        "numericAuditMilliseconds": numeric_audit_milliseconds,
                        "arithmeticCandidateExpansionMilliseconds": arithmetic_expansion_milliseconds,
                        "arithmeticCandidateExpansionRows": arithmetic_expansion_rows,
                        "arithmeticCandidateExpansionCandidates": arithmetic_expansion_candidates,
                        "independentObserverMilliseconds": independent_observer_milliseconds,
                        "multilineRows": multiline_rows,
                    },
                    "geometry": geometry,
                    "structuredEvidence": structured_evidence,
                    "domainOntology": {
                        "enabled": domain_ontology is not None,
                        "source": str(arguments.domain_ontology) if arguments.domain_ontology else None,
                        "languageTermsSource": (
                            str(arguments.domain_language_terms)
                            if arguments.domain_language_terms
                            else None
                        ),
                        "aliasesSource": (
                            str(arguments.domain_ontology_aliases)
                            if arguments.domain_ontology_aliases
                            else None
                        ),
                        "rawTextOverwrites": 0,
                        "annotatedCells": sum(
                            bool(cell.get("ontologyResolution"))
                            for cell in geometry["cells"]
                        ),
                        "resolvedCells": sum(
                            cell.get("ontologyResolution", {}).get("status") == "resolved"
                            for cell in geometry["cells"]
                        ),
                        "suggestedCells": sum(
                            cell.get("ontologyResolution", {}).get("status") == "suggested"
                            for cell in geometry["cells"]
                        ),
                        "ambiguousCells": sum(
                            cell.get("ontologyResolution", {}).get("status") == "ambiguous"
                            for cell in geometry["cells"]
                        ),
                    },
                    "independentOcrObserver": independent_observer_receipt,
                    "structuralConsensus": {
                        "numericColumns": len(numeric_columns),
                        "multilineRows": multiline_rows,
                        "multicolumnHeaderRows": multicolumn_rows,
                        "cellConstraintFusion": cell_constraint_fusion,
                        "multilineRowContract": (
                            "visual-ink-bands-after-horizontal-rule-removal-plus-"
                            "repeated-whitespace-gutter-header-tiles"
                        ),
                    },
                    "lexicalSpacingRecovery": {
                        "enabled": lexicon is not None,
                        "contract": "whole-line-isolated-korean-syllables-plus-independent-lexical-memory",
                        "count": len(lexical_spacing_recoveries),
                        "decisions": lexical_spacing_recoveries,
                    },
                    "denseTitleNormalization": {
                        "enabled": arguments.dense_title_normalization,
                        "contract": (
                            "visual-density-envelope-plus-relative-and-absolute-"
                            "confidence-plus-minimum-content-plus-visible-fact-integrity"
                        ),
                        "eligible": len(dense_title_audit),
                        "selected": sum(
                            decision["selected"] for decision in dense_title_audit
                        ),
                        "reasons": dict(
                            Counter(
                                reason
                                for decision in dense_title_audit
                                for reason in decision["reasons"]
                            )
                        ),
                        "decisions": dense_title_audit,
                    },
                    "specialistRouting": {
                        "enabled": specialist_pair is not None,
                        "confidenceMargin": arguments.specialist_confidence_margin,
                        "lexicon": arguments.lexicon.name if arguments.lexicon else None,
                        "knownWordGuard": lexicon is not None,
                        "decisions": len(routing_reasons),
                        "selected": sum(
                            reason in ("specialist_selected", "anchor_integrity_recovery")
                            for reason in routing_reasons
                        ),
                        "reasons": dict(Counter(routing_reasons)),
                        "productionAuthority": False,
                    },
                    "numericAnchorRouting": {
                        "enabled": numeric_anchor is not None,
                        "confidenceThreshold": arguments.numeric_anchor_confidence_threshold,
                        "decisions": len(routing_reasons),
                        "anchorSelections": sum(
                            reason == "numeric_multiview_anchor" for reason in routing_reasons
                        ),
                        "reasons": dict(Counter(routing_reasons)),
                        "productionAuthority": False,
                    },
                    "numericMultiviewAudit": {
                        "mode": arguments.numeric_multiview_audit,
                        "scope": (
                            "scan-page-approved-structured-numeric-rows-plus-"
                            "accepted-unstructured-numeric-rows"
                        ),
                        "rows": numeric_audit_rows,
                        "stable": numeric_audit_rows - numeric_audit_disagreements,
                        "needsReview": numeric_audit_disagreements,
                        "valueSelection": "none",
                        "contract": (
                            "Numeric disagreement is quarantined for review; "
                            "alternate views never replace or invent a value."
                        ),
                    },
                    "arithmeticCandidateExpansion": {
                        "mode": arguments.arithmetic_candidate_expansion,
                        "rows": arithmetic_expansion_rows,
                        "addedCandidates": arithmetic_expansion_candidates,
                        "candidateLimit": arguments.arithmetic_candidate_limit,
                        "beamWidth": arguments.arithmetic_beam_width,
                        "selectionUsesTarget": False,
                        "externalOcrInvocations": 0,
                        "contract": (
                            "Wide visual search wakes only after a repeated generic "
                            "table equation rejects the fast-path row; only an owned "
                            "unique satisfying candidate may replace the row."
                        ),
                    },
                }
                (arguments.output / f"p{page_number:04}.json").write_text(
                    json.dumps(receipt, ensure_ascii=False, indent=2), encoding="utf-8"
                )
                print(
                    json.dumps(
                        {
                            "page": page_number,
                            "textRegions": len(geometry["textRegions"]),
                            "textRows": len(geometry["textRows"]),
                            "cells": len(geometry["cells"]),
                            "milliseconds": receipt["elapsedMilliseconds"],
                        },
                        separators=(",", ":"),
                    )
                )


if __name__ == "__main__":
    main()
