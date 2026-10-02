from __future__ import annotations

import unittest

import numpy as np

from bcore_native_ocr.recognize_page_tables import _uint8_linear_quantile


class Uint8QuantileTests(unittest.TestCase):
    def test_matches_numpy_linear_quantile_for_diverse_uint8_images(self):
        random = np.random.default_rng(20261002)
        for shape in ((1,), (2,), (17,), (64, 48), (313, 709)):
            pixels = random.integers(0, 256, size=shape, dtype=np.uint8)
            for probability in (0.0, 0.01, 0.2, 0.22, 0.5, 0.85, 1.0):
                self.assertAlmostEqual(
                    _uint8_linear_quantile(pixels, probability),
                    float(np.quantile(pixels, probability)),
                    places=7,
                )


if __name__ == "__main__":
    unittest.main()
