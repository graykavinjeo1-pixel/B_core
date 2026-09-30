"""Independent page-orientation evidence and conservative adjudication.

The native OCR recognizer is useful evidence, but technical drawings can make
upside-down small labels look more confident than the few real headings.  A
separate document-orientation classifier therefore acts as an independent
observer.  It never silently wins a strong disagreement: weak conflicts are
retained as review evidence, while an ambiguous native decision may be
corrected by an observer that clears an explicit confidence gate.
"""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

import numpy as np
import paddle
from PIL import Image, ImageOps

from .models import BCoreOrientationClassifier


VALID_ORIENTATIONS = {0, 90, 180, 270}
ORDERED_ORIENTATIONS = (0, 90, 180, 270)


@dataclass(frozen=True)
class OrientationDecision:
    selected_degrees: int
    reason: str
    needs_review: bool
    native_degrees: int
    native_margin: float
    observer_degrees: int | None
    observer_confidence: float | None

    def receipt(self) -> dict[str, Any]:
        payload = asdict(self)
        return {
            "selectedDegrees": payload.pop("selected_degrees"),
            "reason": payload.pop("reason"),
            "needsReview": payload.pop("needs_review"),
            "nativeDegrees": payload.pop("native_degrees"),
            "nativeMargin": payload.pop("native_margin"),
            "observerDegrees": payload.pop("observer_degrees"),
            "observerConfidence": payload.pop("observer_confidence"),
        }


def choose_orientation(
    *,
    native_degrees: int,
    native_margin: float,
    native_candidates: set[int],
    observer_degrees: int | None,
    observer_confidence: float | None,
    observer_minimum_confidence: float = 0.55,
    observer_strong_confidence: float = 0.80,
    native_ambiguity_margin: float = 0.15,
    review_confidence: float = 0.70,
) -> OrientationDecision:
    """Combine two independent orientation algorithms without majority theatre.

    The observer may correct a weak native result, or a strong observer may
    correct an axis error.  Any other disagreement remains native and is
    explicitly quarantined for review rather than being hidden.
    """

    if native_degrees not in VALID_ORIENTATIONS:
        raise ValueError("B_CORE_OCR_NATIVE_ORIENTATION_INVALID")
    if not 0.0 <= native_margin:
        raise ValueError("B_CORE_OCR_NATIVE_ORIENTATION_MARGIN_INVALID")
    if observer_degrees is None or observer_confidence is None:
        return OrientationDecision(
            native_degrees,
            "native_only",
            native_margin < native_ambiguity_margin,
            native_degrees,
            native_margin,
            None,
            None,
        )
    if observer_degrees not in VALID_ORIENTATIONS:
        raise ValueError("B_CORE_OCR_OBSERVER_ORIENTATION_INVALID")
    if not 0.0 <= observer_confidence <= 1.0:
        raise ValueError("B_CORE_OCR_OBSERVER_CONFIDENCE_INVALID")
    if observer_degrees == native_degrees:
        return OrientationDecision(
            native_degrees,
            "independent_agreement",
            observer_confidence < observer_minimum_confidence
            and native_margin < native_ambiguity_margin,
            native_degrees,
            native_margin,
            observer_degrees,
            observer_confidence,
        )
    native_is_ambiguous = native_margin < native_ambiguity_margin
    observer_is_usable = observer_confidence >= observer_minimum_confidence
    observer_is_strong = observer_confidence >= observer_strong_confidence
    observer_in_native_axis = observer_degrees in native_candidates
    if observer_is_strong or (
        native_is_ambiguous and observer_is_usable and observer_in_native_axis
    ):
        return OrientationDecision(
            observer_degrees,
            (
                "strong_observer_override"
                if observer_is_strong
                else "ambiguous_native_observer_override"
            ),
            observer_confidence < review_confidence,
            native_degrees,
            native_margin,
            observer_degrees,
            observer_confidence,
        )
    return OrientationDecision(
        native_degrees,
        "unresolved_orientation_disagreement",
        True,
        native_degrees,
        native_margin,
        observer_degrees,
        observer_confidence,
    )


def load_paddlex_orientation_observer(model_dir: Path):
    """Load an already-installed observer without permitting a download."""

    required = ("inference.json", "inference.pdiparams", "inference.yml")
    if not model_dir.is_dir() or any(not (model_dir / name).is_file() for name in required):
        raise FileNotFoundError("B_CORE_OCR_ORIENTATION_OBSERVER_NOT_INSTALLED")
    from paddlex import create_model

    return create_model(
        "PP-LCNet_x1_0_doc_ori",
        model_dir=str(model_dir.resolve()),
    )


def observe_orientation(model, source: Image.Image) -> tuple[int, float]:
    """Read one orientation prediction from a separately trained observer."""

    results = list(
        model.predict(np.asarray(source.convert("RGB")), batch_size=1)
    )
    if len(results) != 1:
        raise RuntimeError("B_CORE_OCR_ORIENTATION_OBSERVER_RESULT_COUNT")
    payload = results[0].json
    if callable(payload):
        payload = payload()
    result = payload.get("res", payload)
    labels = result.get("label_names") or []
    scores = result.get("scores") or []
    if len(labels) != 1 or len(scores) != 1:
        raise RuntimeError("B_CORE_OCR_ORIENTATION_OBSERVER_RESULT_SHAPE")
    degrees = int(labels[0])
    confidence = float(scores[0])
    if degrees not in VALID_ORIENTATIONS or not 0.0 <= confidence <= 1.0:
        raise RuntimeError("B_CORE_OCR_ORIENTATION_OBSERVER_RESULT_INVALID")
    return degrees, confidence


def prepare_orientation_page(source: Image.Image, size: int = 256) -> Image.Image:
    """Letterbox a whole page without destroying its spatial orientation cues."""

    grayscale = ImageOps.autocontrast(source.convert("L"), cutoff=1)
    scale = min(size / grayscale.width, size / grayscale.height)
    resized = grayscale.resize(
        (
            max(1, round(grayscale.width * scale)),
            max(1, round(grayscale.height * scale)),
        ),
        Image.Resampling.BILINEAR,
    )
    canvas = Image.new("L", (size, size), 255)
    canvas.paste(
        resized,
        ((size - resized.width) // 2, (size - resized.height) // 2),
    )
    return canvas


def load_bcore_orientation_observer(model_dir: Path) -> BCoreOrientationClassifier:
    metadata = json.loads((model_dir / "model.json").read_text(encoding="utf-8"))
    if metadata.get("architecture") != "BCoreOrientationClassifier":
        raise ValueError("B_CORE_OCR_ORIENTATION_MODEL_ARCHITECTURE_INVALID")
    model = BCoreOrientationClassifier()
    model.set_state_dict(paddle.load(str(model_dir / "orientation.pdparams")))
    model.eval()
    return model


@paddle.no_grad()
def observe_bcore_orientation(
    model: BCoreOrientationClassifier, source: Image.Image, size: int = 256
) -> tuple[int, float]:
    prepared = np.asarray(
        prepare_orientation_page(source, size), dtype="float32"
    )
    tensor = paddle.to_tensor((255.0 - prepared)[None, None, :, :] / 255.0)
    probabilities = paddle.nn.functional.softmax(model(tensor), axis=-1)[0]
    index = int(paddle.argmax(probabilities).numpy())
    return ORDERED_ORIENTATIONS[index], float(probabilities[index].numpy())
