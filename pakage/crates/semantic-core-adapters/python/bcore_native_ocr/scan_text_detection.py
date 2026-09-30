"""Language-independent text-line support detection for raster-only pages."""

from __future__ import annotations

import cv2
import numpy as np


def _runs(active: np.ndarray, maximum_gap: int) -> list[tuple[int, int]]:
    positions = np.flatnonzero(active)
    if not len(positions):
        return []
    result: list[tuple[int, int]] = []
    start = previous = int(positions[0])
    for raw_position in positions[1:]:
        position = int(raw_position)
        if position - previous > maximum_gap + 1:
            result.append((start, previous))
            start = position
        previous = position
    result.append((start, previous))
    return result


def detect_scan_text_support(gray: np.ndarray) -> tuple[np.ndarray, dict[str, int | float]]:
    """Return broad line support without reading characters or using a lexicon.

    The learned detector remains responsible for table rules.  This fallback
    only replaces its text-support channel on pages without a trustworthy PDF
    text layer, where slide backgrounds and icons otherwise fragment letters.
    """

    image = np.asarray(gray)
    if image.ndim != 2:
        raise ValueError("B_CORE_SCAN_TEXT_EXPECTS_GRAYSCALE")
    if image.dtype != np.uint8:
        image = np.clip(image, 0, 255).astype("uint8")
    height, width = image.shape
    otsu_threshold, _ = cv2.threshold(
        image, 0, 255, cv2.THRESH_BINARY_INV + cv2.THRESH_OTSU
    )
    applied_threshold = min(float(otsu_threshold), 180.0)
    _, ink = cv2.threshold(image, applied_threshold, 255, cv2.THRESH_BINARY_INV)

    # Remove page frames, side panels and photographic masses by connected
    # geometry. Thin horizontal rules remain and are rejected later by their
    # row-band height. This avoids a long morphology kernel that can erase a
    # connected bold Korean heading.
    component_count, labels, statistics, _ = cv2.connectedComponentsWithStats(
        (ink > 0).astype("uint8"), connectivity=8
    )
    glyph_ink = np.zeros_like(ink)
    removed_large_components = 0
    for component in range(1, component_count):
        x, y, component_width, component_height, area = statistics[component]
        if (
            component_height > height * 0.14
            or component_width > width * 0.96
            or area > height * width * 0.06
        ):
            removed_large_components += 1
            continue
        glyph_ink[labels == component] = 255

    support = np.zeros_like(ink)
    accepted = 0
    rejected = 0
    # A page frame can contribute several dark pixels to every row. Requiring
    # a small width-relative ink count prevents that frame from merging the
    # entire page into one false text band while remaining far below one word.
    minimum_row_ink = max(6, round(width * 0.006))
    active_rows = np.count_nonzero(glyph_ink, axis=1) >= minimum_row_ink
    row_bands = _runs(active_rows, maximum_gap=max(1, round(height * 0.0015)))
    join_width = max(10, round(height * 0.012))
    for y0, y1 in row_bands:
        box_height = y1 - y0 + 1
        if box_height < max(5, round(height * 0.003)) or box_height > height * 0.09:
            rejected += 1
            continue
        active_columns = np.any(glyph_ink[y0 : y1 + 1], axis=0)
        for x0, x1 in _runs(active_columns, maximum_gap=max(join_width, round(box_height * 0.8))):
            box_width = x1 - x0 + 1
            if box_width < max(6, round(width * 0.004)) or box_width > width * 0.96:
                rejected += 1
                continue
            source_ink = glyph_ink[y0 : y1 + 1, x0 : x1 + 1]
            density = float(np.count_nonzero(source_ink)) / max(1, box_width * box_height)
            if density < 0.015 or density > 0.72:
                rejected += 1
                continue
            padding_x = max(2, round(box_height * 0.18))
            padding_y = max(1, round(box_height * 0.10))
            cv2.rectangle(
                support,
                (max(0, x0 - padding_x), max(0, y0 - padding_y)),
                (min(width - 1, x1 + padding_x), min(height - 1, y1 + padding_y)),
                255,
                thickness=-1,
            )
            accepted += 1
    return support.astype(bool), {
        "otsuThreshold": float(otsu_threshold),
        "appliedThreshold": applied_threshold,
        "joinWidth": join_width,
        "removedLargeComponents": removed_large_components,
        "acceptedComponents": accepted,
        "rejectedComponents": rejected,
    }


def detect_component_cluster_text_support(
    gray: np.ndarray,
) -> tuple[np.ndarray, dict[str, object]]:
    """Detect scan text by clustering glyph components into visual lines.

    This is intentionally independent from :func:`detect_scan_text_support`'s
    row projection.  It is useful when two tightly stacked lines overlap in the
    horizontal projection and would otherwise become one over-tall crop.
    Language, expected words and document templates are never consulted.
    """

    image = np.asarray(gray)
    if image.ndim != 2:
        raise ValueError("B_CORE_SCAN_TEXT_EXPECTS_GRAYSCALE")
    if image.dtype != np.uint8:
        image = np.clip(image, 0, 255).astype("uint8")
    height, width = image.shape
    otsu_threshold, ink = cv2.threshold(
        image, 0, 255, cv2.THRESH_BINARY_INV + cv2.THRESH_OTSU
    )
    component_count, _, statistics, _ = cv2.connectedComponentsWithStats(
        (ink > 0).astype("uint8"), connectivity=8
    )
    components = []
    minimum_height = max(4, round(height * 0.002))
    maximum_height = max(24, minimum_height + 1, round(height * 0.055))
    for component in range(1, component_count):
        x, y, component_width, component_height, area = (
            int(value) for value in statistics[component]
        )
        density = area / max(1, component_width * component_height)
        if component_height < minimum_height or component_height > maximum_height:
            continue
        if component_width < 1 or component_width > width * 0.18:
            continue
        if area < 5 or density < 0.025:
            continue
        components.append(
            {
                "x0": x,
                "y0": y,
                "x1": x + component_width,
                "y1": y + component_height,
                "cx": x + component_width / 2.0,
                "cy": y + component_height / 2.0,
                "height": component_height,
                "width": component_width,
            }
        )

    lines: list[dict[str, object]] = []
    for component in sorted(components, key=lambda item: (item["cy"], item["x0"])):
        best = None
        best_distance = float("inf")
        for line in lines:
            line_height = float(np.median([item["height"] for item in line["components"]]))
            line_center = float(np.median([item["cy"] for item in line["components"]]))
            distance = abs(float(component["cy"]) - line_center)
            # Compare against the robust center/height of member glyphs, not
            # the accumulated line envelope.  Envelope overlap permits one
            # noisy bridging component to chain two stacked text lines.
            if distance <= max(float(component["height"]), line_height) * 0.52:
                if distance < best_distance:
                    best = line
                    best_distance = distance
        if best is None:
            lines.append(
                {
                    "components": [component],
                    "y0": component["y0"],
                    "y1": component["y1"],
                }
            )
        else:
            best["components"].append(component)
            best["y0"] = min(int(best["y0"]), int(component["y0"]))
            best["y1"] = max(int(best["y1"]), int(component["y1"]))

    support = np.zeros_like(ink)
    regions: list[dict[str, int]] = []
    accepted_groups = 0
    rejected_groups = 0
    multi_group_lines = 0
    for line in sorted(lines, key=lambda item: int(item["y0"])):
        line_components = sorted(line["components"], key=lambda item: item["x0"])
        median_height = float(np.median([item["height"] for item in line_components]))
        maximum_gap = max(4, round(median_height * 0.82))
        groups: list[list[dict]] = []
        for component in line_components:
            if not groups or int(component["x0"]) - int(groups[-1][-1]["x1"]) > maximum_gap:
                groups.append([component])
            else:
                groups[-1].append(component)
        if len(groups) > 1:
            multi_group_lines += 1
        for group in groups:
            x0 = min(int(item["x0"]) for item in group)
            y0 = int(line["y0"])
            x1 = max(int(item["x1"]) for item in group)
            y1 = int(line["y1"])
            group_width = x1 - x0
            group_height = y1 - y0
            if group_width < max(4, round(width * 0.0025)) or group_height < minimum_height:
                rejected_groups += 1
                continue
            padding_x = max(2, round(median_height * 0.16))
            padding_y = max(1, round(median_height * 0.06))
            region = {
                "x0": max(0, x0 - padding_x),
                "y0": max(0, y0 - padding_y),
                "x1": min(width - 1, x1 + padding_x),
                "y1": min(height - 1, y1 + padding_y),
            }
            regions.append(region)
            cv2.rectangle(
                support,
                (region["x0"], region["y0"]),
                (region["x1"], region["y1"]),
                255,
                thickness=-1,
            )
            accepted_groups += 1
    return support.astype(bool), {
        "otsuThreshold": float(otsu_threshold),
        "acceptedGlyphComponents": len(components),
        "lineClusters": len(lines),
        "acceptedGroups": accepted_groups,
        "rejectedGroups": rejected_groups,
        "multiGroupLines": multi_group_lines,
        "algorithm": "connected-component-centerline-clustering",
        "regions": regions,
    }


def _region_overlap(left: dict[str, int], right: dict[str, int]) -> float:
    x0 = max(left["x0"], right["x0"])
    y0 = max(left["y0"], right["y0"])
    x1 = min(left["x1"], right["x1"])
    y1 = min(left["y1"], right["y1"])
    intersection = max(0, x1 - x0) * max(0, y1 - y0)
    left_area = max(1, left["x1"] - left["x0"]) * max(
        1, left["y1"] - left["y0"]
    )
    right_area = max(1, right["x1"] - right["x0"]) * max(
        1, right["y1"] - right["y0"]
    )
    return intersection / max(1, min(left_area, right_area))


def merge_component_region_views(
    region_views: list[list[dict[str, int]]],
    *,
    minimum_enhanced_support: int = 2,
) -> tuple[list[dict[str, int]], dict[str, int]]:
    """Fuse same-coordinate regions from independent image-filter views.

    Original-view geometry is always retained.  A region visible only after
    filtering is admitted only when at least two independently transformed
    views recover the same page location.  Filters therefore recover faint
    glyphs without allowing one enhancement artifact to invent a text box.
    """

    if not region_views:
        return [], {"clusters": 0, "original": 0, "enhancedConsensus": 0}
    clusters: list[dict[str, object]] = []
    for view_index, regions in enumerate(region_views):
        for region in regions:
            eligible = [
                cluster
                for cluster in clusters
                if view_index not in cluster["views"]
                and _region_overlap(cluster["representative"], region) >= 0.55
            ]
            match = (
                max(
                    eligible,
                    key=lambda cluster: _region_overlap(
                        cluster["representative"], region
                    ),
                )
                if view_index > 0 and eligible
                else None
            )
            if match is None:
                clusters.append(
                    {
                        "representative": dict(region),
                        "regions": [dict(region)],
                        "views": {view_index},
                        "hasOriginal": view_index == 0,
                    }
                )
                continue
            match["regions"].append(dict(region))
            match["views"].add(view_index)
            match["hasOriginal"] = bool(match["hasOriginal"] or view_index == 0)

    accepted: list[dict[str, int]] = []
    original_count = 0
    enhanced_count = 0
    for cluster in clusters:
        views = cluster["views"]
        if not cluster["hasOriginal"] and len(views) < minimum_enhanced_support:
            continue
        members = cluster["regions"]
        accepted.append(
            {
                key: int(round(float(np.median([member[key] for member in members]))))
                for key in ("x0", "y0", "x1", "y1")
            }
        )
        if cluster["hasOriginal"]:
            original_count += 1
        else:
            enhanced_count += 1
    accepted.sort(key=lambda item: (item["y0"], item["x0"], item["y1"], item["x1"]))
    return accepted, {
        "clusters": len(clusters),
        "original": original_count,
        "enhancedConsensus": enhanced_count,
        "rejectedSingleViewArtifacts": len(clusters) - len(accepted),
    }


def detect_multiview_component_text_support(
    gray: np.ndarray,
) -> tuple[np.ndarray, dict[str, object]]:
    """Recover faint scan text through filtered-view spatial consensus."""

    image = np.asarray(gray)
    if image.ndim != 2:
        raise ValueError("B_CORE_SCAN_TEXT_EXPECTS_GRAYSCALE")
    if image.dtype != np.uint8:
        image = np.clip(image, 0, 255).astype("uint8")
    clahe = cv2.createCLAHE(clipLimit=2.0, tileGridSize=(8, 8)).apply(image)
    blurred = cv2.GaussianBlur(image, (0, 0), sigmaX=1.0)
    sharpened = cv2.addWeighted(image, 1.65, blurred, -0.65, 0)
    observations = []
    diagnostics = []
    for name, view in (("original", image), ("clahe", clahe), ("unsharp", sharpened)):
        _, details = detect_component_cluster_text_support(view)
        regions = list(details.pop("regions"))
        observations.append(regions)
        diagnostics.append({"view": name, "regions": len(regions), **details})
    regions, fusion = merge_component_region_views(observations)
    support = np.zeros_like(image)
    for region in regions:
        cv2.rectangle(
            support,
            (region["x0"], region["y0"]),
            (region["x1"], region["y1"]),
            255,
            thickness=-1,
        )
    return support.astype(bool), {
        "algorithm": "multiview-component-spatial-consensus",
        "views": diagnostics,
        "fusion": fusion,
        "acceptedGroups": len(regions),
        "regions": regions,
    }


def horizontal_text_coverage(gray: np.ndarray) -> float:
    """Measure wide text-line support for coarse orientation selection."""

    support, _ = detect_scan_text_support(gray)
    contours, _ = cv2.findContours(
        (support.astype("uint8") * 255), cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE
    )
    width = max(1, support.shape[1])
    wide_width = sum(
        box_width
        for contour in contours
        for _, _, box_width, box_height in [cv2.boundingRect(contour)]
        if box_width >= box_height * 2
    )
    return min(1.0, wide_width / width)
