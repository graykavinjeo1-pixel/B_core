from __future__ import annotations

import unittest

import paddle

from bcore_native_ocr.recognize_page_tables import (
    _decode_logits,
    _decode_logits_batch,
)


class _Codec:
    def decode_ctc(self, tokens):
        return "".join("" if token == 0 else chr(64 + token) for token in tokens)


class RecognizerBatchDecodeTests(unittest.TestCase):
    def test_batch_decode_matches_per_row_reference_with_padding(self):
        paddle.seed(20261002)
        logits = paddle.randn([3, 11, 8], dtype="float32")
        lengths = [11, 7, 3]
        codec = _Codec()
        expected = [
            _decode_logits(logits[index, :length], codec)
            for index, length in enumerate(lengths)
        ]
        actual = _decode_logits_batch(logits, lengths, codec)
        self.assertEqual([text for text, _ in actual], [text for text, _ in expected])
        for (_, actual_confidence), (_, expected_confidence) in zip(actual, expected):
            self.assertAlmostEqual(actual_confidence, expected_confidence, places=7)


if __name__ == "__main__":
    unittest.main()
