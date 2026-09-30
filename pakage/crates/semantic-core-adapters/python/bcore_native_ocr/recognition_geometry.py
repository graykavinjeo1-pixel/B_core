"""Versioned image/sequence geometry contracts for native OCR checkpoints."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path


LINE_HEIGHT = 48
MAX_LINE_WIDTH = 960
HORIZONTAL_DOWNSAMPLE = 4
HORIZONTAL_SCALE = 1.0
EXTENDED_MAX_LINE_WIDTH = 1600
EXTENDED_HORIZONTAL_SCALE = 1.25


@dataclass(frozen=True)
class RecognizerGeometry:
    line_height: int = LINE_HEIGHT
    max_width: int = MAX_LINE_WIDTH
    horizontal_scale: float = HORIZONTAL_SCALE
    horizontal_downsample: int = HORIZONTAL_DOWNSAMPLE


def load_checkpoint_geometry(checkpoint: Path) -> RecognizerGeometry:
    """Load geometry written with a checkpoint; old checkpoints stay legacy."""

    metadata_path = checkpoint / "model.json"
    if not metadata_path.exists():
        return RecognizerGeometry()
    payload = json.loads(metadata_path.read_text(encoding="utf-8"))
    geometry = payload.get("recognitionGeometry") or {}
    return RecognizerGeometry(
        line_height=int(geometry.get("lineHeight", LINE_HEIGHT)),
        max_width=int(geometry.get("maxWidth", MAX_LINE_WIDTH)),
        horizontal_scale=float(geometry.get("horizontalScale", HORIZONTAL_SCALE)),
        horizontal_downsample=int(
            geometry.get("horizontalDownsample", HORIZONTAL_DOWNSAMPLE)
        ),
    )


def minimum_ctc_steps(token_ids: list[int]) -> int:
    """Return the shortest CTC path that can emit the encoded label."""

    return len(token_ids) + sum(
        left == right for left, right in zip(token_ids, token_ids[1:])
    )


def resized_line_width(
    crop_width: int,
    crop_height: int,
    *,
    line_height: int = LINE_HEIGHT,
    max_width: int = MAX_LINE_WIDTH,
    horizontal_scale: float = HORIZONTAL_SCALE,
) -> int:
    return min(
        max_width,
        max(
            8,
            round(
                crop_width
                * line_height
                * horizontal_scale
                / max(1, crop_height)
            ),
        ),
    )


def recognizer_time_steps(resized_width: int) -> int:
    return max(1, resized_width // HORIZONTAL_DOWNSAMPLE)
