"""B_Core-owned OCR topology implemented on the Paddle tensor runtime.

No pretrained OCR weights or third-party OCR pipeline is imported here.  The
models are deliberately small enough for fast resident inference on a desktop
GPU while exposing separate text, table and recognition heads.
"""

from __future__ import annotations

import paddle
from paddle import nn
import paddle.nn.functional as functional


class ConvNormAct(nn.Layer):
    def __init__(self, input_channels: int, output_channels: int, stride: tuple[int, int] = (1, 1)):
        super().__init__()
        self.conv = nn.Conv2D(input_channels, output_channels, 3, stride=stride, padding=1, bias_attr=False)
        self.norm = nn.BatchNorm2D(output_channels)

    def forward(self, inputs: paddle.Tensor) -> paddle.Tensor:
        return functional.silu(self.norm(self.conv(inputs)))


class ResidualBlock(nn.Layer):
    def __init__(self, channels: int):
        super().__init__()
        self.first = ConvNormAct(channels, channels)
        self.second = nn.Sequential(
            nn.Conv2D(channels, channels, 3, padding=1, bias_attr=False),
            nn.BatchNorm2D(channels),
        )

    def forward(self, inputs: paddle.Tensor) -> paddle.Tensor:
        return functional.silu(inputs + self.second(self.first(inputs)))


class BCoreOrientationClassifier(nn.Layer):
    """Small page-level orientation observer owned by B_Core.

    It sees a low-resolution whole-page image and predicts the correction
    angle 0/90/180/270.  This is intentionally separate from text recognition:
    technical drawings and sparse forms should not let a few accidental CTC
    confidences decide the entire page orientation.
    """

    def __init__(self, class_count: int = 4):
        super().__init__()
        self.visual = nn.Sequential(
            ConvNormAct(1, 16, (2, 2)),
            ConvNormAct(16, 32, (2, 2)),
            ResidualBlock(32),
            ConvNormAct(32, 64, (2, 2)),
            ResidualBlock(64),
            ConvNormAct(64, 96, (2, 2)),
            ResidualBlock(96),
            ConvNormAct(96, 128, (2, 2)),
        )
        # A global mean discards whether headings and folios are above or
        # below the page.  A small fixed spatial grid preserves that evidence
        # while remaining far cheaper than a text recognizer.
        self.spatial_pool = nn.AdaptiveAvgPool2D((4, 4))
        self.output = nn.Linear(128 * 4 * 4, class_count)

    def forward(self, images: paddle.Tensor) -> paddle.Tensor:
        features = self.visual(images)
        pooled = self.spatial_pool(features).flatten(start_axis=1)
        return self.output(pooled)


class BCoreLineRecognizer(nn.Layer):
    """Unicode-character CTC recognizer for one rectified text line."""

    def __init__(self, vocabulary_size: int, hidden_size: int = 192):
        super().__init__()
        self.visual = nn.Sequential(
            ConvNormAct(1, 48, (2, 2)),
            ResidualBlock(48),
            ConvNormAct(48, 96, (2, 2)),
            ResidualBlock(96),
            ConvNormAct(96, 192, (2, 1)),
            ResidualBlock(192),
            ConvNormAct(192, 256, (2, 1)),
        )
        self.context = nn.GRU(
            input_size=256,
            hidden_size=hidden_size,
            num_layers=2,
            direction="bidirectional",
            dropout=0.1,
        )
        self.output = nn.Linear(hidden_size * 2, vocabulary_size)

    def forward(self, images: paddle.Tensor, sequence_lengths: paddle.Tensor | None = None) -> paddle.Tensor:
        features = self.visual(images)
        features = paddle.mean(features, axis=2).transpose([0, 2, 1])
        sequence, _ = self.context(features, sequence_length=sequence_lengths)
        return self.output(sequence)


class BCoreHighResolutionLineRecognizer(nn.Layer):
    """Higher-capacity owned recognizer for visually ambiguous Korean scans.

    The original 16 MB recognizer is retained as the production baseline. This
    research topology spends capacity on the visual encoder before context so
    correct glyph alternatives can enter the N-best set; a language reranker
    cannot recover characters that the visual model never proposed.
    """

    def __init__(self, vocabulary_size: int, hidden_size: int = 320):
        super().__init__()
        self.visual = nn.Sequential(
            ConvNormAct(1, 64, (2, 2)),
            ResidualBlock(64),
            ResidualBlock(64),
            ConvNormAct(64, 128, (2, 2)),
            ResidualBlock(128),
            ResidualBlock(128),
            ConvNormAct(128, 256, (2, 1)),
            ResidualBlock(256),
            ResidualBlock(256),
            ConvNormAct(256, 384, (2, 1)),
            ResidualBlock(384),
            ResidualBlock(384),
        )
        self.context = nn.GRU(
            input_size=384,
            hidden_size=hidden_size,
            num_layers=3,
            direction="bidirectional",
            dropout=0.1,
        )
        self.output = nn.Linear(hidden_size * 2, vocabulary_size)

    def forward(
        self,
        images: paddle.Tensor,
        sequence_lengths: paddle.Tensor | None = None,
    ) -> paddle.Tensor:
        features = self.visual(images)
        features = paddle.mean(features, axis=2).transpose([0, 2, 1])
        sequence, _ = self.context(features, sequence_length=sequence_lengths)
        return self.output(sequence)


class BCoreResidualRefinementLineRecognizer(nn.Layer):
    """Preserve the mature recognizer and learn only residual CTC corrections.

    ``visual``, ``context`` and ``output`` intentionally keep the baseline key
    names and tensor shapes.  A compatible initializer can therefore copy the
    complete mature recognizer without approximation.  The zero-initialized
    correction head makes the first forward pass numerically identical to the
    source checkpoint; new capacity is introduced only through supervised
    residual learning.
    """

    def __init__(
        self,
        vocabulary_size: int,
        hidden_size: int = 192,
        refinement_size: int = 128,
    ):
        super().__init__()
        baseline = BCoreLineRecognizer(vocabulary_size, hidden_size)
        self.visual = baseline.visual
        self.context = baseline.context
        self.output = baseline.output
        self.refinement = nn.GRU(
            input_size=hidden_size * 2,
            hidden_size=refinement_size,
            num_layers=1,
            direction="bidirectional",
        )
        self.correction = nn.Linear(refinement_size * 2, vocabulary_size)
        nn.initializer.Constant(0.0)(self.correction.weight)
        nn.initializer.Constant(0.0)(self.correction.bias)

    def forward(
        self,
        images: paddle.Tensor,
        sequence_lengths: paddle.Tensor | None = None,
    ) -> paddle.Tensor:
        features = self.visual(images)
        features = paddle.mean(features, axis=2).transpose([0, 2, 1])
        sequence, _ = self.context(features, sequence_length=sequence_lengths)
        refinement, _ = self.refinement(
            sequence, sequence_length=sequence_lengths
        )
        return self.output(sequence) + self.correction(refinement)


class BCoreVisualResidualRefinementLineRecognizer(nn.Layer):
    """Add trainable glyph capacity while preserving a mature full recognizer.

    All tensors from ``BCoreResidualRefinementLineRecognizer`` retain their
    names and shapes.  The new visual correction is zero-initialized, making a
    compatible initialization exactly equal to its source before adaptation.
    """

    def __init__(
        self,
        vocabulary_size: int,
        hidden_size: int = 192,
        refinement_size: int = 128,
    ):
        super().__init__()
        baseline = BCoreResidualRefinementLineRecognizer(
            vocabulary_size,
            hidden_size=hidden_size,
            refinement_size=refinement_size,
        )
        self.visual = baseline.visual
        self.context = baseline.context
        self.output = baseline.output
        self.refinement = baseline.refinement
        self.correction = baseline.correction
        self.visual_refinement = nn.Sequential(
            ResidualBlock(256),
            ResidualBlock(256),
        )
        self.visual_correction = nn.Conv2D(256, 256, 1)
        nn.initializer.Constant(0.0)(self.visual_correction.weight)
        nn.initializer.Constant(0.0)(self.visual_correction.bias)

    def forward(
        self,
        images: paddle.Tensor,
        sequence_lengths: paddle.Tensor | None = None,
    ) -> paddle.Tensor:
        features = self.visual(images)
        visual_delta = self.visual_correction(self.visual_refinement(features))
        features = features + visual_delta
        features = paddle.mean(features, axis=2).transpose([0, 2, 1])
        sequence, _ = self.context(features, sequence_length=sequence_lengths)
        refinement, _ = self.refinement(
            sequence, sequence_length=sequence_lengths
        )
        return self.output(sequence) + self.correction(refinement)


LINE_RECOGNIZER_TOPOLOGIES = (
    "conv-residual-bigru-ctc",
    "highres-deepconv-bigru-ctc",
    "conv-residual-bigru-refinement-ctc",
    "conv-visual-residual-bigru-refinement-ctc",
)


def build_line_recognizer(vocabulary_size: int, topology: str = LINE_RECOGNIZER_TOPOLOGIES[0]):
    if topology == "conv-residual-bigru-ctc":
        return BCoreLineRecognizer(vocabulary_size)
    if topology == "highres-deepconv-bigru-ctc":
        return BCoreHighResolutionLineRecognizer(vocabulary_size)
    if topology == "conv-residual-bigru-refinement-ctc":
        return BCoreResidualRefinementLineRecognizer(vocabulary_size)
    if topology == "conv-visual-residual-bigru-refinement-ctc":
        return BCoreVisualResidualRefinementLineRecognizer(vocabulary_size)
    raise ValueError(f"B_CORE_NATIVE_OCR_UNKNOWN_RECOGNIZER_TOPOLOGY:{topology}")


def _mask_feature_width(features: paddle.Tensor, widths: paddle.Tensor) -> paddle.Tensor:
    positions = paddle.arange(features.shape[-1], dtype=widths.dtype).reshape([1, 1, 1, -1])
    mask = positions < widths.reshape([-1, 1, 1, 1])
    return features * mask.astype(features.dtype)


def _masked_residual_forward(
    block: ResidualBlock, inputs: paddle.Tensor, widths: paddle.Tensor
) -> paddle.Tensor:
    first = block.first(inputs)
    first = _mask_feature_width(first, widths)
    second = block.second(first)
    second = _mask_feature_width(second, widths)
    return _mask_feature_width(functional.silu(inputs + second), widths)


def forward_visual_masked(
    visual: nn.Sequential,
    images: paddle.Tensor,
    input_widths: paddle.Tensor,
    *,
    start_index: int = 0,
) -> tuple[paddle.Tensor, paddle.Tensor]:
    """Match independent-width convolution boundaries inside a padded batch."""

    features = images
    widths = input_widths
    for local_index, layer in enumerate(visual):
        index = start_index + local_index
        if isinstance(layer, ResidualBlock):
            features = _masked_residual_forward(layer, features, widths)
            continue
        features = layer(features)
        if index in (0, 2):
            widths = (widths + 1) // 2
        features = _mask_feature_width(features, widths)
    return features, widths


def forward_recognizer_masked(
    recognizer: BCoreLineRecognizer,
    images: paddle.Tensor,
    input_widths: paddle.Tensor,
) -> tuple[paddle.Tensor, paddle.Tensor]:
    features, feature_widths = forward_visual_masked(
        recognizer.visual, images, input_widths
    )
    features = paddle.mean(features, axis=2).transpose([0, 2, 1])
    sequence, _ = recognizer.context(features, sequence_length=feature_widths)
    return recognizer.output(sequence), feature_widths


class BCoreVisualSpecialistPair(nn.Layer):
    """Two visual encoders sharing one language context and output head.

    Module-delta specialists only replace ``visual.*``. Keeping duplicate GRU
    and output layers would waste memory and launch the same kernels twice.
    The pair emits anchor and specialist logits while preserving an explicit
    boundary between their visual evidence.
    """

    def __init__(self, vocabulary_size: int, hidden_size: int = 192):
        super().__init__()
        anchor = BCoreLineRecognizer(vocabulary_size, hidden_size)
        specialist = BCoreLineRecognizer(vocabulary_size, hidden_size)
        self.anchor_visual = anchor.visual
        self.specialist_visual = specialist.visual
        self.context = anchor.context
        self.output = anchor.output

    def forward_pair(
        self,
        images: paddle.Tensor,
        sequence_lengths: paddle.Tensor | None = None,
        input_widths: paddle.Tensor | None = None,
    ) -> tuple[paddle.Tensor, paddle.Tensor]:
        if input_widths is None:
            anchor_visual = self.anchor_visual(images)
            specialist_visual = self.specialist_visual(images)
        else:
            anchor_visual, inferred_lengths = forward_visual_masked(
                self.anchor_visual, images, input_widths
            )
            specialist_visual, specialist_lengths = forward_visual_masked(
                self.specialist_visual, images, input_widths
            )
            if sequence_lengths is None:
                sequence_lengths = inferred_lengths
            if not paddle.equal_all(inferred_lengths, specialist_lengths):
                raise ValueError("B_CORE_NATIVE_OCR_VISUAL_LENGTH_MISMATCH")
        anchor_features = paddle.mean(anchor_visual, axis=2).transpose([0, 2, 1])
        specialist_features = paddle.mean(specialist_visual, axis=2).transpose([0, 2, 1])
        batch_size = anchor_features.shape[0]
        combined = paddle.concat([anchor_features, specialist_features], axis=0)
        combined_lengths = (
            paddle.concat([sequence_lengths, sequence_lengths], axis=0)
            if sequence_lengths is not None
            else None
        )
        sequence, _ = self.context(combined, sequence_length=combined_lengths)
        logits = self.output(sequence)
        return logits[:batch_size], logits[batch_size:]


class BCoreBranchedVisualSpecialistPair(nn.Layer):
    """Share an immutable early visual stem and branch only adapted late blocks."""

    def __init__(
        self, vocabulary_size: int, hidden_size: int = 192, split_index: int = 4
    ):
        super().__init__()
        anchor = BCoreLineRecognizer(vocabulary_size, hidden_size)
        specialist = BCoreLineRecognizer(vocabulary_size, hidden_size)
        if not 1 <= split_index < len(anchor.visual):
            raise ValueError("B_CORE_NATIVE_OCR_VISUAL_BRANCH_SPLIT_INVALID")
        self.split_index = split_index
        self.common_visual = nn.Sequential(
            *[anchor.visual[index] for index in range(split_index)]
        )
        self.anchor_visual_tail = nn.Sequential(
            *[anchor.visual[index] for index in range(split_index, len(anchor.visual))]
        )
        self.specialist_visual_tail = nn.Sequential(
            *[
                specialist.visual[index]
                for index in range(split_index, len(specialist.visual))
            ]
        )
        self.context = anchor.context
        self.output = anchor.output

    def forward_pair(
        self,
        images: paddle.Tensor,
        sequence_lengths: paddle.Tensor | None = None,
        input_widths: paddle.Tensor | None = None,
    ) -> tuple[paddle.Tensor, paddle.Tensor]:
        if input_widths is None:
            common = self.common_visual(images)
            anchor_visual = self.anchor_visual_tail(common)
            specialist_visual = self.specialist_visual_tail(common)
        else:
            common, common_widths = forward_visual_masked(
                self.common_visual, images, input_widths
            )
            anchor_visual, inferred_lengths = forward_visual_masked(
                self.anchor_visual_tail,
                common,
                common_widths,
                start_index=self.split_index,
            )
            specialist_visual, specialist_lengths = forward_visual_masked(
                self.specialist_visual_tail,
                common,
                common_widths,
                start_index=self.split_index,
            )
            if sequence_lengths is None:
                sequence_lengths = inferred_lengths
            if not paddle.equal_all(inferred_lengths, specialist_lengths):
                raise ValueError("B_CORE_NATIVE_OCR_VISUAL_LENGTH_MISMATCH")
        anchor_features = paddle.mean(anchor_visual, axis=2).transpose([0, 2, 1])
        specialist_features = paddle.mean(specialist_visual, axis=2).transpose(
            [0, 2, 1]
        )
        # Keep the context batch shape identical to standalone inference.  A
        # concatenated 2N batch is mathematically equivalent but can change
        # near-tie CTC argmax results through accelerator reduction order.
        anchor_sequence, _ = self.context(
            anchor_features, sequence_length=sequence_lengths
        )
        specialist_sequence, _ = self.context(
            specialist_features, sequence_length=sequence_lengths
        )
        return self.output(anchor_sequence), self.output(specialist_sequence)


class BCoreDocumentDetector(nn.Layer):
    """Full-page detector with general, non-template-specific structure heads.

    Output channels are text probability, horizontal table rule, vertical table
    rule and cell-junction probability.  Facility names or apartment templates
    are never encoded in this network contract.
    """

    def __init__(self):
        super().__init__()
        self.stem = nn.Sequential(ConvNormAct(1, 32, (2, 2)), ResidualBlock(32))
        self.level_two = nn.Sequential(ConvNormAct(32, 64, (2, 2)), ResidualBlock(64))
        self.level_three = nn.Sequential(ConvNormAct(64, 128, (2, 2)), ResidualBlock(128))
        self.bottleneck = nn.Sequential(ConvNormAct(128, 192, (2, 2)), ResidualBlock(192))
        self.up_three = nn.Conv2DTranspose(192, 128, 2, stride=2)
        self.up_two = nn.Conv2DTranspose(256, 64, 2, stride=2)
        self.up_one = nn.Conv2DTranspose(128, 32, 2, stride=2)
        self.head = nn.Conv2D(64, 4, 1)

    def forward(self, pages: paddle.Tensor) -> paddle.Tensor:
        level_one = self.stem(pages)
        level_two = self.level_two(level_one)
        level_three = self.level_three(level_two)
        bottleneck = self.bottleneck(level_three)
        decoded_three = functional.silu(self.up_three(bottleneck))
        decoded_two = functional.silu(self.up_two(paddle.concat([decoded_three, level_three], axis=1)))
        decoded_one = functional.silu(self.up_one(paddle.concat([decoded_two, level_two], axis=1)))
        return self.head(paddle.concat([decoded_one, level_one], axis=1))
