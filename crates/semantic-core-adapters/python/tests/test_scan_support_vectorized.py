from __future__ import annotations

import unittest

import cv2
import numpy as np

from bcore_native_ocr.scan_text_detection import (
    _region_overlap,
    detect_scan_text_support,
    merge_component_region_views,
)


def reference_scan_text_support(gray: np.ndarray):
    """The pre-optimization component loop, retained only as a test oracle."""
    height, width = gray.shape
    otsu_threshold, _ = cv2.threshold(
        gray, 0, 255, cv2.THRESH_BINARY_INV + cv2.THRESH_OTSU
    )
    applied_threshold = min(float(otsu_threshold), 180.0)
    _, ink = cv2.threshold(gray, applied_threshold, 255, cv2.THRESH_BINARY_INV)
    component_count, labels, stats, _ = cv2.connectedComponentsWithStats(
        (ink > 0).astype("uint8"), connectivity=8
    )
    glyph_ink = np.zeros_like(ink)
    removed = 0
    for component in range(1, component_count):
        _, _, component_width, component_height, area = stats[component]
        if (
            component_height > height * 0.14
            or component_width > width * 0.96
            or area > height * width * 0.06
        ):
            removed += 1
            continue
        glyph_ink[labels == component] = 255
    return glyph_ink, removed


def reference_merge_component_region_views(region_views):
    """The unindexed fusion loop, kept as an ordering-sensitive test oracle."""
    clusters = []
    for view_index, regions in enumerate(region_views):
        for region in regions:
            eligible = [
                cluster for cluster in clusters
                if view_index not in cluster["views"]
                and _region_overlap(cluster["representative"], region) >= 0.55
            ]
            match = (
                max(eligible, key=lambda cluster: _region_overlap(cluster["representative"], region))
                if view_index > 0 and eligible else None
            )
            if match is None:
                clusters.append({
                    "representative": dict(region), "regions": [dict(region)],
                    "views": {view_index}, "hasOriginal": view_index == 0,
                })
            else:
                match["regions"].append(dict(region))
                match["views"].add(view_index)
                match["hasOriginal"] = bool(match["hasOriginal"] or view_index == 0)
    accepted = []
    original_count = enhanced_count = 0
    for cluster in clusters:
        if not cluster["hasOriginal"] and len(cluster["views"]) < 2:
            continue
        members = cluster["regions"]
        accepted.append({
            key: int(round(float(np.median([member[key] for member in members]))))
            for key in ("x0", "y0", "x1", "y1")
        })
        original_count += bool(cluster["hasOriginal"])
        enhanced_count += not bool(cluster["hasOriginal"])
    accepted.sort(key=lambda item: (item["y0"], item["x0"], item["y1"], item["x1"]))
    return accepted, {
        "clusters": len(clusters), "original": original_count,
        "enhancedConsensus": enhanced_count,
        "rejectedSingleViewArtifacts": len(clusters) - len(accepted),
    }


class ScanSupportVectorizedTests(unittest.TestCase):
    def test_spatial_region_index_matches_full_scan(self):
        rng = np.random.default_rng(812)
        originals = []
        for _ in range(220):
            x, y = rng.integers(0, 1500, size=2)
            width, height = rng.integers(5, 120, size=2)
            originals.append({"x0": int(x), "y0": int(y),
                              "x1": int(x + width), "y1": int(y + height)})
        views = [originals]
        for _ in range(2):
            view = []
            for region in originals:
                if rng.random() < 0.6:
                    shift_x, shift_y = rng.integers(-3, 4, size=2)
                    view.append({key: int(value + (shift_x if key.startswith("x") else shift_y))
                                 for key, value in region.items()})
            views.append(view)
        self.assertEqual(
            merge_component_region_views(views),
            reference_merge_component_region_views(views),
        )

    def test_component_mask_matches_prior_loop(self):
        gray = np.full((300, 600), 255, dtype=np.uint8)
        rng = np.random.default_rng(1234)
        for x, y in rng.integers([1, 1], [590, 290], size=(650, 2)):
            cv2.rectangle(gray, (int(x), int(y)), (int(x + 3), int(y + 3)), 0, -1)
        cv2.rectangle(gray, (0, 10), (599, 11), 0, -1)
        expected_ink, expected_removed = reference_scan_text_support(gray)
        # Reproduce the optimized label lookup independently to check that the
        # formerly repeated whole-page comparisons select identical pixels.
        otsu_threshold, _ = cv2.threshold(
            gray, 0, 255, cv2.THRESH_BINARY_INV + cv2.THRESH_OTSU
        )
        _, ink = cv2.threshold(
            gray, min(float(otsu_threshold), 180.0), 255, cv2.THRESH_BINARY_INV
        )
        count, labels, stats, _ = cv2.connectedComponentsWithStats(
            (ink > 0).astype("uint8"), connectivity=8
        )
        rejected = (
            (stats[:, cv2.CC_STAT_HEIGHT] > gray.shape[0] * 0.14)
            | (stats[:, cv2.CC_STAT_WIDTH] > gray.shape[1] * 0.96)
            | (stats[:, cv2.CC_STAT_AREA] > gray.size * 0.06)
        )
        keep = ~rejected
        keep[0] = False
        actual_ink = keep[labels].astype(np.uint8) * 255
        self.assertEqual(count, len(stats))
        self.assertTrue(np.array_equal(actual_ink, expected_ink))
        self.assertEqual(int(np.count_nonzero(rejected[1:])), expected_removed)
        support, diagnostics = detect_scan_text_support(gray)
        self.assertEqual(support.shape, gray.shape)
        self.assertEqual(diagnostics["removedLargeComponents"], expected_removed)


if __name__ == "__main__":
    unittest.main()
