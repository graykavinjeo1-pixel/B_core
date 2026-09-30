"""Render native detector channels over arbitrary PDF pages for visual review."""

from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
from pathlib import Path

import numpy as np
import paddle
import paddle.nn.functional as functional
from PIL import Image

from .models import BCoreDocumentDetector
from .scan_rule_recovery import recover_scan_rule_masks
from .table_geometry import reconstruct_table_geometry


COLORS = np.asarray(
    [
        [224, 45, 45],   # text
        [39, 190, 91],   # horizontal
        [52, 102, 235],  # vertical
        [245, 176, 32],  # junction
    ],
    dtype="float32",
)


def _render_page(pdf: Path, page_number: int, dpi: int, output: Path) -> None:
    prefix = output.with_suffix("")
    subprocess.run(
        [
            "pdftoppm",
            "-f",
            str(page_number),
            "-l",
            str(page_number),
            "-singlefile",
            "-r",
            str(dpi),
            "-png",
            str(pdf),
            str(prefix),
        ],
        check=True,
        capture_output=True,
    )


def _predict(model, source: Image.Image, canvas: int, threshold: float) -> np.ndarray:
    grayscale = source.convert("L")
    scale = min(canvas / grayscale.width, canvas / grayscale.height)
    resized_size = (max(1, round(grayscale.width * scale)), max(1, round(grayscale.height * scale)))
    resized = grayscale.resize(resized_size, Image.Resampling.BILINEAR)
    page = Image.new("L", (canvas, canvas), 255)
    offset = ((canvas - resized.width) // 2, (canvas - resized.height) // 2)
    page.paste(resized, offset)
    values = np.asarray(page, dtype="float32") / 127.5 - 1.0
    inputs = paddle.to_tensor(values[None, None, :, :])
    with paddle.no_grad():
        probabilities = functional.sigmoid(model(inputs)).numpy()[0]
    x0, y0 = offset[0] // 2, offset[1] // 2
    width, height = max(1, resized_size[0] // 2), max(1, resized_size[1] // 2)
    cropped = probabilities[:, y0 : y0 + height, x0 : x0 + width]
    channels = []
    for channel in cropped:
        mask = Image.fromarray((channel * 255).astype("uint8"), mode="L")
        mask = mask.resize(grayscale.size, Image.Resampling.BILINEAR)
        channels.append(np.asarray(mask, dtype="float32") / 255.0 >= threshold)
    return np.stack(channels, axis=0)


def _overlay(source: Image.Image, masks: np.ndarray) -> Image.Image:
    base = np.asarray(source.convert("RGB"), dtype="float32")
    for channel, color in zip(masks, COLORS):
        alpha = channel.astype("float32")[:, :, None] * 0.46
        base = base * (1.0 - alpha) + color[None, None, :] * alpha
    return Image.fromarray(np.clip(base, 0, 255).astype("uint8"), mode="RGB")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("pdf", type=Path)
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--pages", required=True, help="Comma-separated one-based pages.")
    parser.add_argument("--dpi", type=int, default=144)
    parser.add_argument("--threshold", type=float, default=0.65)
    parser.add_argument("--rotate-degrees", choices=(0, 180), type=int, default=0)
    parser.add_argument("--scan-rule-recovery", action="store_true")
    parser.add_argument("--device", default="gpu:0")
    arguments = parser.parse_args()
    paddle.set_device(arguments.device)
    metadata = json.loads((arguments.model / "model.json").read_text(encoding="utf-8"))
    model = BCoreDocumentDetector()
    model.set_state_dict(paddle.load(str(arguments.model / "detector.pdparams")))
    model.eval()
    arguments.output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="bcore-overlay-") as temporary:
        for page_number in [int(value) for value in arguments.pages.split(",")]:
            rendered = Path(temporary) / f"page-{page_number:04}.png"
            _render_page(arguments.pdf.resolve(), page_number, arguments.dpi, rendered)
            with Image.open(rendered) as source:
                normalized = source.convert("L").rotate(
                    arguments.rotate_degrees,
                    expand=False,
                    fillcolor=255,
                )
                masks = _predict(model, normalized, int(metadata["canvas"]), arguments.threshold)
                if arguments.scan_rule_recovery:
                    masks = recover_scan_rule_masks(masks, np.asarray(normalized))
                review = _overlay(normalized, masks)
                review.save(arguments.output / f"p{page_number:04}-overlay.jpg", quality=92)
                geometry = reconstruct_table_geometry(masks, np.asarray(normalized))
                geometry.update(
                    {
                        "sourcePage": page_number,
                        "threshold": arguments.threshold,
                        "maskWidth": int(masks.shape[2]),
                        "maskHeight": int(masks.shape[1]),
                        "rotateDegrees": arguments.rotate_degrees,
                        "scanRuleRecovery": arguments.scan_rule_recovery,
                    }
                )
                (arguments.output / f"p{page_number:04}-geometry.json").write_text(
                    json.dumps(geometry, ensure_ascii=False, indent=2), encoding="utf-8"
                )
    print(f"pages={arguments.pages} threshold={arguments.threshold}")


if __name__ == "__main__":
    main()
