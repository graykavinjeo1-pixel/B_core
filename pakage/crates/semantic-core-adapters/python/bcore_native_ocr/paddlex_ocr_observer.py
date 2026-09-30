"""Optional independent OCR evidence for hard-page review and distillation.

This module does not own final facts.  It reads an already-installed local OCR
pipeline, records boxes/text/confidence with provenance, and aligns them to
B_Core rows.  Agreement can strengthen evidence; disagreement is review or
future training material and never silently overwrites a number.
"""

from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image

from .text_normalization import canonicalize_ocr_content
from .specialist_routing import candidate_preserves_visible_facts
from .text_validation import extract_numeric_facts, validate_ocr_text


REQUIRED_MODEL_FILES = ("inference.json", "inference.pdiparams", "inference.yml")


def _validate_model(path: Path, error_code: str) -> None:
    if not path.is_dir() or any(not (path / name).is_file() for name in REQUIRED_MODEL_FILES):
        raise FileNotFoundError(error_code)


def load_local_paddlex_ocr_observer(
    detection_model: Path,
    recognition_model: Path,
    *,
    limit_side_len: int = 1280,
    recognition_batch_size: int = 16,
):
    """Load local files only; never trigger a network model download."""

    _validate_model(detection_model, "B_CORE_OCR_OBSERVER_DETECTOR_NOT_INSTALLED")
    _validate_model(recognition_model, "B_CORE_OCR_OBSERVER_RECOGNIZER_NOT_INSTALLED")
    from paddleocr import PaddleOCR

    detection_model_name = detection_model.name
    recognition_model_name = recognition_model.name
    observer = PaddleOCR(
        text_detection_model_name=detection_model_name,
        text_detection_model_dir=str(detection_model.resolve()),
        text_recognition_model_name=recognition_model_name,
        text_recognition_model_dir=str(recognition_model.resolve()),
        use_doc_orientation_classify=False,
        use_doc_unwarping=False,
        use_textline_orientation=False,
        text_det_limit_side_len=limit_side_len,
        text_recognition_batch_size=recognition_batch_size,
    )
    observer._bcore_detection_model_name = detection_model_name
    observer._bcore_recognition_model_name = recognition_model_name
    return observer


def observe_page(observer, source: Image.Image, minimum_confidence: float = 0.0) -> dict:
    detection_model_name = getattr(
        observer, "_bcore_detection_model_name", "unknown_detection_model"
    )
    recognition_model_name = getattr(
        observer, "_bcore_recognition_model_name", "unknown_recognition_model"
    )
    provenance_group = f"{detection_model_name}+{recognition_model_name}"
    results = list(observer.predict(np.asarray(source.convert("RGB"))))
    if len(results) != 1:
        raise RuntimeError("B_CORE_OCR_OBSERVER_PAGE_RESULT_COUNT")
    payload = results[0].json
    if callable(payload):
        payload = payload()
    result = payload.get("res", payload)
    boxes = result.get("rec_boxes") or []
    polygons = result.get("rec_polys") or []
    texts = result.get("rec_texts") or []
    scores = result.get("rec_scores") or []
    if not (len(boxes) == len(polygons) == len(texts) == len(scores)):
        raise RuntimeError("B_CORE_OCR_OBSERVER_PAGE_RESULT_SHAPE")
    observations = []
    for box, polygon, text, confidence in zip(boxes, polygons, texts, scores):
        normalized = canonicalize_ocr_content(str(text or ""))
        confidence = float(confidence)
        if not normalized or confidence < minimum_confidence:
            continue
        numeric_facts = extract_numeric_facts(normalized)
        observations.append(
            {
                "box": [int(value) for value in box],
                "polygon": [[int(value) for value in point] for point in polygon],
                "text": normalized,
                "confidence": confidence,
                "numericFacts": numeric_facts,
                "status": "observer_only",
                "provenanceGroup": provenance_group,
            }
        )
    return {
        "schema": "B_CORE_NATIVE_OCR_INDEPENDENT_OBSERVER_1",
        "observer": provenance_group,
        "minimumConfidence": minimum_confidence,
        "observations": observations,
        "count": len(observations),
        "numericObservationCount": sum(bool(item["numericFacts"]) for item in observations),
        "factAuthority": False,
    }


def _row_box(row: dict) -> list[int] | None:
    regions = row.get("regions") or []
    if not regions:
        return None
    return [
        min(int(region["x0"]) for region in regions),
        int(row["y0"]),
        max(int(region["x1"]) for region in regions),
        int(row["y1"]),
    ]


def _intersection_over_observer(row_box: list[int], observer_box: list[int]) -> float:
    x0 = max(row_box[0], observer_box[0])
    y0 = max(row_box[1], observer_box[1])
    x1 = min(row_box[2], observer_box[2])
    y1 = min(row_box[3], observer_box[3])
    intersection = max(0, x1 - x0) * max(0, y1 - y0)
    observer_area = max(1, observer_box[2] - observer_box[0]) * max(
        1, observer_box[3] - observer_box[1]
    )
    return intersection / observer_area


def align_observer_to_rows(rows: list[dict], observations: list[dict]) -> dict:
    """Attach observer spans to rows without using either output as truth."""

    matches = []
    unmatched = []
    row_boxes = [_row_box(row) for row in rows]
    for observation_index, observation in enumerate(observations):
        scored = [
            (index, _intersection_over_observer(box, observation["box"]))
            for index, box in enumerate(row_boxes)
            if box is not None
        ]
        row_index, overlap = max(scored, key=lambda item: item[1], default=(-1, 0.0))
        if overlap < 0.50:
            unmatched.append(observation_index)
            continue
        row = rows[row_index]
        candidates = {
            canonicalize_ocr_content(str(row.get(key) or ""))
            for key in ("text", "directText", "regionJoinedText", "beamText")
            if str(row.get(key) or "").strip()
        }
        exact_agreement = observation["text"] in candidates
        # An observer span commonly covers one cell while the native row spans
        # many numeric cells.  Requiring whole-row equality therefore creates
        # false conflicts.  Every fact in the local span must instead occur in
        # at least one native candidate.
        fact_agreement = any(
            all(
                fact in extract_numeric_facts(candidate)
                for fact in observation["numericFacts"]
            )
            for candidate in candidates
        ) if observation["numericFacts"] else None
        matches.append(
            {
                "observationIndex": observation_index,
                "rowIndex": row_index,
                "overlapOverObserver": overlap,
                "exactTextAgreement": exact_agreement,
                "numericFactAgreement": fact_agreement,
                "rowStatus": row.get("status"),
            }
        )
    return {
        "matches": matches,
        "unmatchedObservationIndices": unmatched,
        "exactAgreementCount": sum(item["exactTextAgreement"] for item in matches),
        "numericFactAgreementCount": sum(
            item["numericFactAgreement"] is True for item in matches
        ),
        "numericFactConflictCount": sum(
            item["numericFactAgreement"] is False for item in matches
        ),
    }


def quarantine_verified_numeric_disagreements(
    rows: list[dict],
    observations: list[dict],
    alignment: dict,
    verified_observation_indices: set[int],
) -> dict:
    """Fail closed when two independent pixel readings disagree on a fact.

    Only observer spans that were independently re-read by B_Core are allowed
    to challenge an accepted native row.  Unverified observer output has no
    authority.  The function never chooses either value; it moves the native
    row back to review and records both provenances.
    """

    quarantined = []
    for match in alignment.get("matches") or []:
        observation_index = int(match["observationIndex"])
        row_index = int(match["rowIndex"])
        if observation_index not in verified_observation_indices:
            continue
        if match.get("numericFactAgreement") is not False:
            continue
        row = rows[row_index]
        if row.get("status") != "accepted":
            continue
        observation = observations[observation_index]
        row["validationIssues"] = sorted(
            set(row.get("validationIssues") or [])
            | {"INDEPENDENT_NUMERIC_FACT_DISAGREEMENT"}
        )
        row["status"] = "needs_review"
        row.setdefault("independentDisagreements", []).append(
            {
                "observerText": observation.get("text"),
                "observerNumericFacts": observation.get("numericFacts") or [],
                "observerBox": observation.get("box"),
                "observerProvenance": observation.get("provenanceGroup"),
                "overlapOverObserver": match.get("overlapOverObserver"),
            }
        )
        quarantined.append(row_index)
    return {
        "quarantinedRowIndices": sorted(set(quarantined)),
        "quarantinedCount": len(set(quarantined)),
        "contract": (
            "Only independently B_Core-verified observer spans can quarantine "
            "a native accepted row; conflicts are reviewed, never auto-resolved."
        ),
    }


def _content_key(text: str) -> str:
    """Compare recognised content independently of segmentation whitespace."""

    return "".join(canonicalize_ocr_content(text).split())


def verify_observations_with_bcore(
    observations: list[dict],
    bcore_readings: list[tuple[str, float]],
    glyph_evidence: list[dict],
    *,
    minimum_bcore_confidence: float = 0.55,
) -> dict:
    """Promote only spans independently re-read by the owned recognizer.

    The external pipeline supplies geometry and one text hypothesis. B_Core
    separately reads the exact pixels. Whitespace segmentation may differ,
    but every non-whitespace character and every visible fact must agree.
    """

    if not (
        len(observations) == len(bcore_readings) == len(glyph_evidence)
    ):
        raise ValueError("B_CORE_OCR_OBSERVER_VERIFICATION_SHAPE")
    verified = []
    review = []
    for index, (observation, reading, visual) in enumerate(
        zip(observations, bcore_readings, glyph_evidence)
    ):
        bcore_text, bcore_confidence = reading
        observer_text = str(observation.get("text") or "")
        same_content = bool(observer_text and bcore_text) and (
            _content_key(observer_text) == _content_key(bcore_text)
        )
        facts_preserved, fact_reason = candidate_preserves_visible_facts(
            bcore_text, observer_text
        )
        issues = validate_ocr_text(observer_text)
        reasons = []
        if not visual.get("hasGlyphEvidence", False):
            reasons.append("no_visual_glyph_evidence")
        if bcore_confidence < minimum_bcore_confidence:
            reasons.append("bcore_confidence_guard")
        if not same_content:
            reasons.append("independent_text_disagreement")
        if not facts_preserved:
            reasons.append(fact_reason)
        if issues:
            reasons.append("candidate_integrity_guard")
        record = {
            **observation,
            "observationIndex": index,
            "bcoreText": bcore_text,
            "bcoreConfidence": float(bcore_confidence),
            "visualTextEvidence": visual,
            "independentCharacterAgreement": same_content,
            "visibleFactsPreserved": facts_preserved,
            "validationIssues": issues,
            "verificationReasons": reasons or ["independent_pixel_reread_agreement"],
            "status": "accepted" if not reasons else "needs_review",
            "source": "independent_detector_recognizer_plus_bcore_pixel_reread",
        }
        (verified if not reasons else review).append(record)
    return {
        "approvedEvidence": verified,
        "reviewQueue": review,
        "counts": {
            "approved": len(verified),
            "needsReview": len(review),
        },
        "contract": (
            "External geometry/text has no fact authority unless B_Core independently "
            "re-reads the same glyph pixels with identical non-whitespace characters "
            "and visible facts."
        ),
    }


def resolve_review_with_owned_model_consensus(
    review_records: list[dict],
    specialist_readings: list[tuple[str, float]],
    *,
    anchor_model_name: str,
    specialist_model_name: str,
    minimum_confidence: float = 0.80,
) -> dict:
    """Recover non-numeric open text when two owned checkpoints agree.

    This is deliberately narrower than observer agreement.  Related owned
    models are not treated as independent authority for quantities, dates,
    punctuation-bearing values or parenthetical facts.  They may recover plain
    Korean labels proposed by an independent detector when both checkpoints
    read identical glyph content at high confidence.
    """

    if len(review_records) != len(specialist_readings):
        raise ValueError("B_CORE_OCR_OWNED_CONSENSUS_SHAPE")
    resolved = []
    unresolved = []
    for record, (specialist_text, specialist_confidence) in zip(
        review_records, specialist_readings
    ):
        anchor_text = str(record.get("bcoreText") or "")
        specialist_text = canonicalize_ocr_content(str(specialist_text or ""))
        candidate = canonicalize_ocr_content(anchor_text)
        reasons = []
        if float(record.get("bcoreConfidence") or 0.0) < minimum_confidence:
            reasons.append("anchor_confidence_guard")
        if float(specialist_confidence) < minimum_confidence:
            reasons.append("specialist_confidence_guard")
        if not candidate or _content_key(candidate) != _content_key(specialist_text):
            reasons.append("owned_model_disagreement")
        if extract_numeric_facts(candidate) or extract_numeric_facts(
            str(record.get("text") or "")
        ):
            reasons.append("numeric_fact_requires_external_agreement")
        if any(character in candidate for character in "()[]{}"):
            reasons.append("parenthetical_fact_requires_external_agreement")
        if validate_ocr_text(candidate):
            reasons.append("candidate_integrity_guard")
        upgraded = {
            **record,
            "text": candidate if not reasons else record.get("text"),
            "observerText": record.get("text"),
            "specialistText": specialist_text,
            "specialistConfidence": float(specialist_confidence),
            "verificationReasons": reasons or ["owned_model_pixel_consensus"],
            "status": "accepted" if not reasons else "needs_review",
            "source": "independent_detector_plus_bcore_owned_model_consensus",
            "provenance": [
                record.get("provenanceGroup"),
                anchor_model_name,
                specialist_model_name,
            ],
        }
        (resolved if not reasons else unresolved).append(upgraded)
    return {
        "approvedEvidence": resolved,
        "reviewQueue": unresolved,
        "counts": {"approved": len(resolved), "needsReview": len(unresolved)},
        "contract": (
            "Two high-confidence owned checkpoints may recover only plain "
            "non-numeric labels from independent detector crops; numeric and "
            "parenthetical facts still require external character agreement."
        ),
    }
