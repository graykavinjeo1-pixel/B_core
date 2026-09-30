"""B_Core-owned document vision models and promotion contracts.

The package intentionally contains no PaddleOCR dependency.  Paddle is used as
the tensor runtime during training/inference, while model topology, vocabulary,
weights, datasets, promotion rules and receipts belong to B_Core.
"""

from .codec import BCoreKoreanCodec
from .component_codec import BCoreHangulComponentCodec
from .promotion import OcrStage, PromotionMetrics, PromotionResult, evaluate_promotion

__all__ = [
    "BCoreKoreanCodec",
    "BCoreHangulComponentCodec",
    "OcrStage",
    "PromotionMetrics",
    "PromotionResult",
    "evaluate_promotion",
]
