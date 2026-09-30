"""Experimental scan-rule recovery using geometric and neural agreement.

This module is deliberately separate from the canonical geometry compiler.
It may only be promoted after both raster and vector regression gates pass.
"""

from __future__ import annotations

import numpy as np

from .table_geometry import extract_axis_segments


def _runs(values: np.ndarray, maximum_gap: int = 0) -> list[tuple[int, int]]:
    positions = np.flatnonzero(values)
    if not len(positions):
        return []
    result: list[tuple[int, int]] = []
    start = previous = int(positions[0])
    for value in positions[1:]:
        position = int(value)
        if position - previous > maximum_gap + 1:
            result.append((start, previous))
            start = position
        previous = position
    result.append((start, previous))
    return result


def _thin_horizontal_segments(dark: np.ndarray) -> list[dict[str, int]]:
    """Find page-spanning thin strokes without mistaking text lines for rules."""

    height, width = dark.shape
    active_rows = np.sum(dark, axis=1) >= width * 0.18
    segments: list[dict[str, int]] = []
    for top, bottom in _runs(active_rows, maximum_gap=1):
        # Glyph rows remain active across their full character height, while a
        # scanned rule occupies only a narrow band even after blur.
        if bottom - top + 1 > max(12, round(height * 0.005)):
            continue
        axis = max(range(top, bottom + 1), key=lambda row: int(dark[row].sum()))
        occupied = np.flatnonzero(np.any(dark[top : bottom + 1], axis=0))
        if len(occupied) and int(occupied[-1]) - int(occupied[0]) >= width * 0.35:
            segments.append(
                {"y": axis, "x0": int(occupied[0]), "x1": int(occupied[-1])}
            )
    merged: list[dict[str, int]] = []
    for segment in segments:
        match = next(
            (
                existing
                for existing in merged
                if abs(existing["y"] - segment["y"]) <= 12
                and max(0, min(existing["x1"], segment["x1"]) - max(existing["x0"], segment["x0"]))
                >= 0.55 * min(existing["x1"] - existing["x0"], segment["x1"] - segment["x0"])
            ),
            None,
        )
        if match is None:
            merged.append(dict(segment))
        elif segment["x1"] - segment["x0"] > match["x1"] - match["x0"]:
            match.update(segment)
    return merged


def _merge_vertical_fragments(segments: list[dict[str, int]]) -> list[dict[str, int]]:
    merged: list[dict[str, int]] = []
    for segment in sorted(segments, key=lambda item: (item["x"], item["y0"])):
        match = next(
            (
                existing
                for existing in merged
                if abs(existing["x"] - segment["x"]) <= 6
                and segment["y0"] <= existing["y1"] + 12
                and segment["y1"] >= existing["y0"] - 12
            ),
            None,
        )
        if match is None:
            merged.append(dict(segment))
        else:
            match["x"] = round((match["x"] + segment["x"]) / 2)
            match["y0"] = min(match["y0"], segment["y0"])
            match["y1"] = max(match["y1"], segment["y1"])
    return merged


def _horizontal_support(mask: np.ndarray, segment: dict[str, int], radius: int = 3) -> float:
    y0 = max(0, segment["y"] - radius)
    y1 = min(mask.shape[0], segment["y"] + radius + 1)
    values = mask[y0:y1, segment["x0"] : segment["x1"] + 1]
    return float(values.mean()) if values.size else 0.0


def _vertical_support(mask: np.ndarray, segment: dict[str, int], radius: int = 3) -> float:
    x0 = max(0, segment["x"] - radius)
    x1 = min(mask.shape[1], segment["x"] + radius + 1)
    values = mask[segment["y0"] : segment["y1"] + 1, x0:x1]
    return float(values.mean()) if values.size else 0.0


def _source_vertical_continuity(
    grayscale: np.ndarray,
    segment: dict[str, int],
    *,
    radius: int = 2,
    dark_threshold: float = 225.0,
) -> float:
    """Reject drifting glyph-stroke chains masquerading as vertical rules."""

    x0 = max(0, segment["x"] - radius)
    x1 = min(grayscale.shape[1], segment["x"] + radius + 1)
    band = grayscale[segment["y0"] : segment["y1"] + 1, x0:x1]
    if not band.size:
        return 0.0
    return float(np.mean(np.any(band < dark_threshold, axis=1)))


def recover_scan_rule_masks(
    masks: np.ndarray,
    source_gray: np.ndarray,
    diagnostics: dict | None = None,
) -> np.ndarray:
    """Recover continuous scan rules only when a table network supports them.

    Unlike the rejected pixel-count fusion, text-heavy rows are never promoted
    to rules.  A source stroke must itself be continuous and must either agree
    with the neural rule channel or intersect a small table-like rule network.
    """

    values = np.asarray(masks, dtype=bool).copy()
    grayscale = np.asarray(source_gray)
    if grayscale.shape != values.shape[1:]:
        raise ValueError("B_CORE_TABLE_SOURCE_SHAPE_MISMATCH")
    if np.issubdtype(grayscale.dtype, np.floating) and float(grayscale.max(initial=0.0)) <= 1.0:
        grayscale = grayscale * 255.0
    dark = grayscale < 250.0
    height, width = dark.shape
    horizontal = _thin_horizontal_segments(dark)
    vertical_candidates = _merge_vertical_fragments(
        extract_axis_segments(
            dark,
            horizontal=False,
            minimum_length=max(24, round(height * 0.045)),
            maximum_gap=4,
        )
    )
    vertical_continuity = [
        _source_vertical_continuity(grayscale, segment)
        for segment in vertical_candidates
    ]
    vertical = [
        segment
        for segment, continuity in zip(vertical_candidates, vertical_continuity)
        if continuity >= 0.55
    ]

    def crosses(horizontal_segment: dict[str, int], vertical_segment: dict[str, int]) -> bool:
        return (
            horizontal_segment["x0"] - 3 <= vertical_segment["x"] <= horizontal_segment["x1"] + 3
            and vertical_segment["y0"] - 3 <= horizontal_segment["y"] <= vertical_segment["y1"] + 3
        )

    accepted_horizontal = [
        segment
        for segment in horizontal
        if _horizontal_support(values[1], segment) >= 0.08
        or sum(crosses(segment, candidate) for candidate in vertical) >= 2
    ]
    accepted_vertical = [
        segment
        for segment in vertical
        if _vertical_support(values[2], segment) >= 0.08
        or sum(crosses(candidate, segment) for candidate in accepted_horizontal) >= 2
    ]
    # Recheck source horizontals against the accepted vertical network.  This
    # removes isolated underlines that happened to cross glyph strokes.
    accepted_horizontal = [
        segment
        for segment in accepted_horizontal
        if _horizontal_support(values[1], segment) >= 0.08
        or sum(crosses(segment, candidate) for candidate in accepted_vertical) >= 2
    ]

    if diagnostics is not None:
        diagnostics.update(
            {
                "sourceHorizontalCandidates": len(horizontal),
                "sourceVerticalCandidates": len(vertical_candidates),
                "sourceVerticalContinuousCandidates": len(vertical),
                "sourceVerticalContinuity": vertical_continuity,
                "acceptedHorizontalRules": len(accepted_horizontal),
                "acceptedVerticalRules": len(accepted_vertical),
                "horizontalSegments": accepted_horizontal,
                "verticalSegments": accepted_vertical,
            }
        )

    # A sufficiently connected source network is stronger geometric evidence
    # than scattered neural fragments.  Replace (rather than OR) neural rules
    # only inside each proven grid component, leaving borderless regions and
    # all content outside the component untouched.
    components: list[tuple[list[dict[str, int]], list[dict[str, int]]]] = []
    unseen_horizontal = set(range(len(accepted_horizontal)))
    while unseen_horizontal:
        pending_horizontal = [unseen_horizontal.pop()]
        horizontal_indexes: set[int] = set()
        vertical_indexes: set[int] = set()
        while pending_horizontal:
            horizontal_index = pending_horizontal.pop()
            if horizontal_index in horizontal_indexes:
                continue
            horizontal_indexes.add(horizontal_index)
            for vertical_index, vertical_segment in enumerate(accepted_vertical):
                if crosses(accepted_horizontal[horizontal_index], vertical_segment):
                    if vertical_index not in vertical_indexes:
                        vertical_indexes.add(vertical_index)
                        for candidate_index, candidate in enumerate(accepted_horizontal):
                            if candidate_index not in horizontal_indexes and crosses(candidate, vertical_segment):
                                pending_horizontal.append(candidate_index)
                                unseen_horizontal.discard(candidate_index)
        if len(horizontal_indexes) >= 2 and len(vertical_indexes) >= 2:
            components.append(
                (
                    [accepted_horizontal[index] for index in sorted(horizontal_indexes)],
                    [accepted_vertical[index] for index in sorted(vertical_indexes)],
                )
            )

    for horizontal_component, vertical_component in components:
        x0 = max(0, min(segment["x0"] for segment in horizontal_component) - 4)
        x1 = min(width, max(segment["x1"] for segment in horizontal_component) + 5)
        y0 = max(0, min(segment["y"] for segment in horizontal_component) - 4)
        y1 = min(height, max(segment["y"] for segment in horizontal_component) + 5)
        values[1:4, y0:y1, x0:x1] = False

    for segment in accepted_horizontal:
        y0 = max(0, segment["y"] - 2)
        y1 = min(height, segment["y"] + 3)
        values[1, y0:y1, segment["x0"] : segment["x1"] + 1] = True
    for segment in accepted_vertical:
        x0 = max(0, segment["x"] - 2)
        x1 = min(width, segment["x"] + 3)
        values[2, segment["y0"] : segment["y1"] + 1, x0:x1] = True
    for horizontal_segment in accepted_horizontal:
        for vertical_segment in accepted_vertical:
            if crosses(horizontal_segment, vertical_segment):
                y0 = max(0, horizontal_segment["y"] - 3)
                y1 = min(height, horizontal_segment["y"] + 4)
                x0 = max(0, vertical_segment["x"] - 3)
                x1 = min(width, vertical_segment["x"] + 4)
                values[3, y0:y1, x0:x1] = True
    if diagnostics is not None:
        diagnostics["replacedGridComponents"] = len(components)
    return values
