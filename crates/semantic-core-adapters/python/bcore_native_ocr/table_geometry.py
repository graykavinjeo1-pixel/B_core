"""Compile detector masks into document-independent table geometry."""

from __future__ import annotations

from typing import Any

import numpy as np


def _runs(values: np.ndarray, maximum_gap: int) -> list[tuple[int, int]]:
    positions = np.flatnonzero(values)
    if not len(positions):
        return []
    groups: list[tuple[int, int]] = []
    start = previous = int(positions[0])
    for position_value in positions[1:]:
        position = int(position_value)
        if position - previous > maximum_gap + 1:
            groups.append((start, previous))
            start = position
        previous = position
    groups.append((start, previous))
    return groups


def extract_axis_segments(
    mask: np.ndarray,
    *,
    horizontal: bool,
    minimum_length: int = 12,
    maximum_gap: int = 4,
) -> list[dict[str, int]]:
    """Return long line segments while joining broken scan pixels."""

    values = np.asarray(mask, dtype=bool)
    if not horizontal:
        values = values.T
    active: list[dict[str, int]] = []
    finished: list[dict[str, int]] = []
    for axis, row in enumerate(values):
        row_segments = [(start, end) for start, end in _runs(row, maximum_gap) if end - start + 1 >= minimum_length]
        next_active: list[dict[str, int]] = []
        used = set()
        for segment in active:
            best = None
            best_overlap = 0
            for candidate_index, (start, end) in enumerate(row_segments):
                if candidate_index in used:
                    continue
                overlap = max(0, min(segment["end"], end) - max(segment["start"], start) + 1)
                if overlap > best_overlap:
                    best = candidate_index
                    best_overlap = overlap
            if best is not None and best_overlap >= max(3, min(segment["end"] - segment["start"] + 1, row_segments[best][1] - row_segments[best][0] + 1) // 3):
                start, end = row_segments[best]
                segment["axisEnd"] = axis
                segment["start"] = min(segment["start"], start)
                segment["end"] = max(segment["end"], end)
                next_active.append(segment)
                used.add(best)
            else:
                finished.append(segment)
        for candidate_index, (start, end) in enumerate(row_segments):
            if candidate_index not in used:
                next_active.append({"axisStart": axis, "axisEnd": axis, "start": start, "end": end})
        active = next_active
    finished.extend(active)
    result = []
    for segment in finished:
        axis = round((segment["axisStart"] + segment["axisEnd"]) / 2)
        if horizontal:
            result.append({"y": axis, "x0": segment["start"], "x1": segment["end"]})
        else:
            result.append({"x": axis, "y0": segment["start"], "y1": segment["end"]})
    if horizontal:
        canonical = [dict(axis=segment["y"], start=segment["x0"], end=segment["x1"]) for segment in result]
    else:
        canonical = [dict(axis=segment["x"], start=segment["y0"], end=segment["y1"]) for segment in result]
    merged: list[dict[str, int]] = []
    for segment in sorted(canonical, key=lambda item: (item["axis"], item["start"])):
        match = None
        for existing in merged:
            same_band = abs(existing["axis"] - segment["axis"]) <= 3
            touching = segment["start"] <= existing["end"] + maximum_gap and segment["end"] >= existing["start"] - maximum_gap
            if same_band and touching:
                match = existing
                break
        if match is None:
            merged.append(dict(segment))
        else:
            match["axis"] = round((match["axis"] + segment["axis"]) / 2)
            match["start"] = min(match["start"], segment["start"])
            match["end"] = max(match["end"], segment["end"])
    if horizontal:
        return sorted(
            ({"y": segment["axis"], "x0": segment["start"], "x1": segment["end"]} for segment in merged),
            key=lambda segment: (segment["y"], segment["x0"]),
        )
    return sorted(
        ({"x": segment["axis"], "y0": segment["start"], "y1": segment["end"]} for segment in merged),
        key=lambda segment: (segment["x"], segment["y0"]),
    )


def extract_text_regions(
    mask: np.ndarray,
    maximum_line_gap: int = 2,
    maximum_word_gap: int = 10,
    minimum_row_pixels: int = 4,
) -> list[dict[str, int]]:
    """Group dense text pixels into line-local regions without reading text."""

    values = np.asarray(mask, dtype=bool)
    # One or two residual rule pixels must not bridge otherwise independent
    # text lines across a tall/merged cell.
    active_rows = np.sum(values, axis=1) >= minimum_row_pixels
    row_bands = _runs(active_rows, maximum_line_gap)
    regions: list[dict[str, int]] = []
    for y0, y1 in row_bands:
        columns = np.any(values[y0 : y1 + 1], axis=0)
        for x0, x1 in _runs(columns, maximum_word_gap):
            if x1 - x0 >= 2 and y1 - y0 >= 1:
                regions.append({"x0": x0, "y0": y0, "x1": x1, "y1": y1})
    return regions


def _consolidate_rule_segments(
    segments: list[dict[str, int]],
    *,
    horizontal: bool,
    tolerance: int,
) -> list[dict[str, int]]:
    """Collapse thick/broken scan rules into one geometric boundary.

    A scanned table rule is commonly predicted as several nearby parallel
    strokes.  Treating each stroke as a separate row creates tiny phantom
    cells and lets text regions span real rows.  Consolidation uses geometry
    only; it does not inspect language or document-specific coordinates.
    """

    axis_key = "y" if horizontal else "x"
    start_key, end_key = ("x0", "x1") if horizontal else ("y0", "y1")
    groups: list[list[dict[str, int]]] = []
    for segment in sorted(segments, key=lambda item: (item[axis_key], item[start_key])):
        chosen = None
        for group in reversed(groups):
            group_axis = round(sum(item[axis_key] for item in group) / len(group))
            if segment[axis_key] - group_axis > tolerance:
                break
            overlaps = any(
                min(segment[end_key], item[end_key]) - max(segment[start_key], item[start_key])
                >= min(segment[end_key] - segment[start_key], item[end_key] - item[start_key]) * 0.25
                for item in group
            )
            continues = any(
                max(segment[start_key], item[start_key])
                - min(segment[end_key], item[end_key])
                <= max(4, tolerance * 2)
                for item in group
            )
            if abs(segment[axis_key] - group_axis) <= tolerance and (overlaps or continues):
                chosen = group
                break
        if chosen is None:
            groups.append([segment])
        else:
            chosen.append(segment)
    result = []
    for group in groups:
        weights = [max(1, item[end_key] - item[start_key] + 1) for item in group]
        axis = round(sum(item[axis_key] * weight for item, weight in zip(group, weights)) / sum(weights))
        result.append(
            {
                axis_key: axis,
                start_key: min(item[start_key] for item in group),
                end_key: max(item[end_key] for item in group),
            }
        )
    return sorted(result, key=lambda item: (item[axis_key], item[start_key]))


def _snap_rule_segments_to_source(
    horizontal: list[dict[str, int]],
    vertical: list[dict[str, int]],
    source_gray: np.ndarray,
    *,
    maximum_offset: int,
) -> tuple[list[dict[str, int]], list[dict[str, int]]]:
    """Move predicted rule axes to the nearest strong source-ink axis.

    This never creates or lengthens a rule.  It only corrects the few-pixel
    localization error of an already predicted segment, so the detector still
    owns table discovery and the source image only refines geometry.
    """

    ink = np.asarray(source_gray) < 210

    def snap_horizontal(segment: dict[str, int]) -> dict[str, int]:
        x0 = max(0, segment["x0"])
        x1 = min(ink.shape[1] - 1, segment["x1"])
        candidates = range(max(0, segment["y"] - maximum_offset), min(ink.shape[0], segment["y"] + maximum_offset + 1))
        axis = max(candidates, key=lambda y: (int(ink[y, x0 : x1 + 1].sum()), -abs(y - segment["y"])))
        return {**segment, "y": int(axis)}

    def snap_vertical(segment: dict[str, int]) -> dict[str, int]:
        y0 = max(0, segment["y0"])
        y1 = min(ink.shape[0] - 1, segment["y1"])
        candidates = range(max(0, segment["x"] - maximum_offset), min(ink.shape[1], segment["x"] + maximum_offset + 1))
        axis = max(candidates, key=lambda x: (int(ink[y0 : y1 + 1, x].sum()), -abs(x - segment["x"])))
        return {**segment, "x": int(axis)}

    return [snap_horizontal(segment) for segment in horizontal], [snap_vertical(segment) for segment in vertical]


def _extract_text_between_boundaries(mask: np.ndarray, boundaries: list[int]) -> list[dict[str, int]]:
    """Extract text independently between table-row boundaries.

    This prevents a faint vertical-rule response in the text channel from
    joining several physical table rows into one crop.
    """

    height = mask.shape[0]
    cuts = sorted({0, height, *(max(0, min(height, value)) for value in boundaries)})
    regions: list[dict[str, int]] = []
    minimum_height = max(2, round(mask.shape[0] * 0.002))
    minimum_width = max(3, round(mask.shape[1] * 0.003))
    for top, bottom in zip(cuts, cuts[1:]):
        if bottom - top < 3:
            continue
        inset_top = top + (1 if top else 0)
        inset_bottom = bottom - 1 if bottom < height else bottom
        if inset_bottom - inset_top < 2:
            continue
        word_gap = max(10, round(mask.shape[1] * 0.01))
        for region in extract_text_regions(
            mask[inset_top:inset_bottom],
            maximum_word_gap=word_gap,
        ):
            height = region["y1"] - region["y0"] + 1
            width = region["x1"] - region["x0"] + 1
            ink_pixels = int(
                mask[
                    region["y0"] + inset_top : region["y1"] + inset_top + 1,
                    region["x0"] : region["x1"] + 1,
                ].sum()
            )
            if height < minimum_height or width < minimum_width or ink_pixels < minimum_height * 2:
                continue
            padding = max(2, round(height * 0.12))
            regions.append(
                {
                    "x0": max(0, region["x0"] - padding),
                    "y0": max(inset_top, region["y0"] + inset_top - padding),
                    "x1": min(mask.shape[1] - 1, region["x1"] + padding),
                    "y1": min(inset_bottom - 1, region["y1"] + inset_top + padding),
                }
            )
    return regions


def _attach_regions(cells: list[dict[str, int]], regions: list[dict[str, int]]) -> tuple[list[dict[str, Any]], list[dict[str, int]]]:
    attached = [dict(cell, textRegions=[]) for cell in cells]
    unassigned = []
    for region in regions:
        center_x = (region["x0"] + region["x1"]) / 2
        center_y = (region["y0"] + region["y1"]) / 2
        candidates = [
            cell
            for cell in attached
            if cell["x0"] <= center_x <= cell["x1"] and cell["y0"] <= center_y <= cell["y1"]
        ]
        if not candidates:
            unassigned.append(region)
            continue
        target = min(candidates, key=lambda cell: (cell["x1"] - cell["x0"]) * (cell["y1"] - cell["y0"]))
        target["textRegions"].append(region)
    return attached, unassigned


def _text_rows(regions: list[dict[str, int]]) -> list[dict[str, Any]]:
    grouped: dict[tuple[int, int], list[dict[str, int]]] = {}
    for region in regions:
        grouped.setdefault((region["y0"], region["y1"]), []).append(region)
    return [
        {
            "y0": y0,
            "y1": y1,
            "regions": sorted(items, key=lambda item: item["x0"]),
        }
        for (y0, y1), items in sorted(grouped.items())
    ]


def replace_text_regions(
    geometry: dict[str, Any],
    regions: list[dict[str, int]],
) -> dict[str, Any]:
    """Replace only text geometry while preserving independently found rules.

    This allows a second visual algorithm to contribute better word/line boxes
    without recomputing, copying or weakening table-rule evidence.
    """

    cells, unassigned = _attach_regions(geometry.get("cells", []), regions)
    return {
        **geometry,
        "textRegions": regions,
        "textRows": _text_rows(regions),
        "cells": cells,
        "unassignedTextRegions": unassigned,
    }


def _connected_rule_components(
    horizontal: list[dict[str, int]],
    vertical: list[dict[str, int]],
    *,
    tolerance: int,
) -> list[tuple[list[dict[str, int]], list[dict[str, int]]]]:
    """Group only physically connected horizontal/vertical rule segments.

    Pairing every horizontal line on a page with the next one accidentally
    turns unrelated slide callouts into table rows.  A table grid must be a
    connected bipartite graph of crossing horizontal and vertical rules.
    This is purely geometric and independent of document wording or layout.
    """

    count = len(horizontal) + len(vertical)
    parents = list(range(count))

    def find(index: int) -> int:
        while parents[index] != index:
            parents[index] = parents[parents[index]]
            index = parents[index]
        return index

    def union(left: int, right: int) -> None:
        left_root, right_root = find(left), find(right)
        if left_root != right_root:
            parents[right_root] = left_root

    for horizontal_index, line_h in enumerate(horizontal):
        for vertical_index, line_v in enumerate(vertical):
            intersects_x = line_h["x0"] - tolerance <= line_v["x"] <= line_h["x1"] + tolerance
            intersects_y = line_v["y0"] - tolerance <= line_h["y"] <= line_v["y1"] + tolerance
            if intersects_x and intersects_y:
                union(horizontal_index, len(horizontal) + vertical_index)

    grouped: dict[int, tuple[list[dict[str, int]], list[dict[str, int]]]] = {}
    for index, line in enumerate(horizontal):
        root = find(index)
        grouped.setdefault(root, ([], []))[0].append(line)
    for index, line in enumerate(vertical):
        root = find(len(horizontal) + index)
        grouped.setdefault(root, ([], []))[1].append(line)
    return [
        (
            sorted(component_horizontal, key=lambda item: (item["y"], item["x0"])),
            sorted(component_vertical, key=lambda item: (item["x"], item["y0"])),
        )
        for component_horizontal, component_vertical in grouped.values()
        if component_horizontal and component_vertical
    ]


def _compile_grid_component(
    horizontal: list[dict[str, int]],
    vertical: list[dict[str, int]],
) -> tuple[list[dict[str, int]], list[dict[str, int]]]:
    """Compile one connected rule component and reject one-dimensional boxes."""

    component_rows: list[dict[str, int]] = []
    component_cells: list[dict[str, int]] = []
    cells_per_row: list[int] = []
    for top, bottom in zip(horizontal, horizontal[1:]):
        if bottom["y"] - top["y"] < 4:
            continue
        x0 = max(top["x0"], bottom["x0"])
        x1 = min(top["x1"], bottom["x1"])
        if x1 - x0 < 12:
            continue
        boundaries = [x0, x1]
        for line in vertical:
            covers_band = line["y0"] <= top["y"] + 3 and line["y1"] >= bottom["y"] - 3
            if covers_band and x0 - 3 <= line["x"] <= x1 + 3:
                boundaries.append(min(x1, max(x0, line["x"])))
        boundaries = sorted(set(boundaries))
        row_cells = [
            {"x0": left, "y0": top["y"], "x1": right, "y1": bottom["y"]}
            for left, right in zip(boundaries, boundaries[1:])
            if right - left >= 4
        ]
        if not row_cells:
            continue
        component_rows.append({"x0": x0, "y0": top["y"], "x1": x1, "y1": bottom["y"]})
        component_cells.extend(row_cells)
        cells_per_row.append(len(row_cells))

    # The corpus contract defines a table as a two-dimensional grid: at least
    # two row bands, with at least one band split into two columns.  Frames,
    # banners and stacked callouts are therefore retained as layout elements
    # but never promoted to table cells.
    if len(component_rows) < 2 or max(cells_per_row, default=0) < 2:
        return [], []
    return component_rows, component_cells


def _attach_adjacent_merged_boundaries(
    component_horizontal: list[dict[str, int]],
    all_horizontal: list[dict[str, int]],
    all_vertical: list[dict[str, int]],
    text_support: np.ndarray,
) -> list[dict[str, int]]:
    """Attach disconnected full-width merged rows to an established grid.

    A table title/header row can legitimately contain no internal vertical
    divider.  In scanned documents its outer border can also be broken, so
    that horizontal rule is disconnected from the multi-column body even
    though the two form one table.  Once a two-dimensional grid has already
    been established, admit an immediately adjacent boundary only when its
    span, row spacing and text occupancy agree with that grid.  This remains
    a geometry/ink rule; no document name, coordinate or vocabulary is used.
    """

    if len(component_horizontal) < 2:
        return component_horizontal
    attached = list(component_horizontal)
    attached_keys = {(line["y"], line["x0"], line["x1"]) for line in attached}
    gaps = [
        lower["y"] - upper["y"]
        for upper, lower in zip(attached, attached[1:])
        if lower["y"] - upper["y"] >= 4
    ]
    if not gaps:
        return component_horizontal
    typical_gap = float(np.median(gaps))
    maximum_gap = max(12.0, typical_gap * 1.65)

    # Grow one boundary at a time so consecutive merged rows can be attached
    # without ever scanning arbitrary page bands as standalone tables.
    changed = True
    while changed:
        changed = False
        attached.sort(key=lambda item: (item["y"], item["x0"]))
        top, bottom = attached[0], attached[-1]
        for candidate in all_horizontal:
            key = (candidate["y"], candidate["x0"], candidate["x1"])
            if key in attached_keys:
                continue
            if candidate["y"] < top["y"]:
                neighbour = top
                gap = top["y"] - candidate["y"]
            elif candidate["y"] > bottom["y"]:
                neighbour = bottom
                gap = candidate["y"] - bottom["y"]
            else:
                continue
            if gap < 4 or gap > maximum_gap:
                continue
            overlap = max(
                0,
                min(candidate["x1"], neighbour["x1"])
                - max(candidate["x0"], neighbour["x0"]),
            )
            candidate_width = max(1, candidate["x1"] - candidate["x0"])
            neighbour_width = max(1, neighbour["x1"] - neighbour["x0"])
            if overlap / min(candidate_width, neighbour_width) < 0.90:
                continue
            if min(candidate_width, neighbour_width) / max(candidate_width, neighbour_width) < 0.75:
                continue
            y0, y1 = sorted((candidate["y"], neighbour["y"]))
            x0 = max(candidate["x0"], neighbour["x0"])
            x1 = min(candidate["x1"], neighbour["x1"])
            edge_tolerance = max(4, round(min(candidate_width, neighbour_width) * 0.012))
            # Require a surviving outer-border stroke across the proposed
            # band.  This distinguishes a damaged merged table row from a
            # nearby heading underline or decorative separator.
            has_bridging_outer_border = any(
                line["y0"] <= y0 + 3
                and line["y1"] >= y1 - 3
                and (
                    max(abs(line["x"] - candidate["x0"]), abs(line["x"] - neighbour["x0"]))
                    <= edge_tolerance
                    or max(abs(line["x"] - candidate["x1"]), abs(line["x"] - neighbour["x1"]))
                    <= edge_tolerance
                )
                for line in all_vertical
            )
            if not has_bridging_outer_border:
                continue
            interior = text_support[min(y1 - 1, y0 + 2) : max(y0 + 2, y1 - 1), x0:x1]
            minimum_ink = max(24, round((x1 - x0) * 0.012))
            if interior.size == 0 or int(np.count_nonzero(interior)) < minimum_ink:
                continue
            attached.append(candidate)
            attached_keys.add(key)
            changed = True
            break
    return sorted(attached, key=lambda item: (item["y"], item["x0"]))


def reconstruct_table_geometry(
    masks: np.ndarray,
    source_gray: np.ndarray | None = None,
    *,
    snap_to_source: bool = True,
    subtract_rule_masks_from_text: bool = True,
) -> dict[str, Any]:
    """Convert four detector channels into rows and ruled-cell rectangles.

    The result contains no apartment-, phrase- or coordinate-specific rules.
    Merged cells emerge naturally when a vertical boundary does not cross the
    row band between two horizontal rules.
    """

    values = np.asarray(masks, dtype=bool)
    if values.shape[0] != 4:
        raise ValueError("B_CORE_TABLE_MASK_REQUIRES_FOUR_CHANNELS")
    horizontal = _consolidate_rule_segments(
        extract_axis_segments(values[1], horizontal=True),
        horizontal=True,
        tolerance=max(3, round(values.shape[1] * 0.004)),
    )
    vertical = _consolidate_rule_segments(
        extract_axis_segments(values[2], horizontal=False),
        horizontal=False,
        tolerance=max(3, round(values.shape[2] * 0.004)),
    )
    if source_gray is not None:
        grayscale = np.asarray(source_gray)
        if grayscale.shape != values.shape[1:]:
            raise ValueError("B_CORE_TABLE_SOURCE_SHAPE_MISMATCH")
        if np.issubdtype(grayscale.dtype, np.floating) and float(grayscale.max(initial=0.0)) <= 1.0:
            grayscale = grayscale * 255.0
        if snap_to_source:
            horizontal, vertical = _snap_rule_segments_to_source(
                horizontal,
                vertical,
                grayscale,
                maximum_offset=max(2, round(max(values.shape[1:]) * 0.003)),
            )
    cleaned_text = values[0]
    if subtract_rule_masks_from_text:
        cleaned_text = cleaned_text & ~values[1] & ~values[2]
    if source_gray is not None:
        # The detector deliberately learns broad text support regions.  Use
        # the source image only to recover actual ink inside that support;
        # this is layout refinement, not a second language/OCR heuristic.
        cleaned_text &= grayscale < 225.0
    rows: list[dict[str, int]] = []
    cells: list[dict[str, int]] = []
    accepted_components = []
    for component_horizontal, component_vertical in _connected_rule_components(
        horizontal,
        vertical,
        tolerance=max(3, round(max(values.shape[1:]) * 0.004)),
    ):
        component_rows, component_cells = _compile_grid_component(
            component_horizontal,
            component_vertical,
        )
        if not component_rows:
            continue
        component_horizontal = _attach_adjacent_merged_boundaries(
            component_horizontal,
            horizontal,
            vertical,
            cleaned_text,
        )
        component_rows, component_cells = _compile_grid_component(
            component_horizontal,
            component_vertical,
        )
        component_index = len(accepted_components)
        row_keys = sorted({(cell["y0"], cell["y1"]) for cell in component_cells})
        column_starts = sorted({cell["x0"] for cell in component_cells})
        for cell in component_cells:
            cell["tableComponentIndex"] = component_index
            cell["rowIndex"] = row_keys.index((cell["y0"], cell["y1"]))
            cell["columnIndex"] = column_starts.index(cell["x0"])
        rows.extend(component_rows)
        cells.extend(component_cells)
        accepted_components.append(
            {
                "horizontalSegments": component_horizontal,
                "verticalSegments": component_vertical,
                "rows": len(component_rows),
                "cells": len(component_cells),
            }
        )
    rows.sort(key=lambda item: (item["y0"], item["x0"]))
    cells.sort(key=lambda item: (item["y0"], item["x0"]))
    text_regions = _extract_text_between_boundaries(
        cleaned_text,
        [coordinate for row in rows for coordinate in (row["y0"], row["y1"])],
    )
    cells_with_text, unassigned_text = _attach_regions(cells, text_regions)
    text_by_row: dict[tuple[int, int], int] = {}
    for cell in cells_with_text:
        key = (cell["y0"], cell["y1"])
        text_by_row[key] = text_by_row.get(key, 0) + len(cell["textRegions"])
    occupied_heights = [
        row["y1"] - row["y0"]
        for row in rows
        if text_by_row.get((row["y0"], row["y1"]), 0) > 0
    ]
    if occupied_heights:
        median_occupied_height = float(np.median(occupied_heights))
        kept_row_keys = {
            (row["y0"], row["y1"])
            for row in rows
            if text_by_row.get((row["y0"], row["y1"]), 0) > 0
            or row["y1"] - row["y0"] >= median_occupied_height * 0.55
        }
        rows = [row for row in rows if (row["y0"], row["y1"]) in kept_row_keys]
        cells_with_text = [
            cell for cell in cells_with_text if (cell["y0"], cell["y1"]) in kept_row_keys
        ]
    return {
        "schema": "B_CORE_NATIVE_OCR_TABLE_GEOMETRY_1",
        "horizontalSegments": horizontal,
        "verticalSegments": vertical,
        "tableComponents": accepted_components,
        "rows": rows,
        "textRegions": text_regions,
        "textRows": _text_rows(text_regions),
        "cells": cells_with_text,
        "unassignedTextRegions": unassigned_text,
    }
