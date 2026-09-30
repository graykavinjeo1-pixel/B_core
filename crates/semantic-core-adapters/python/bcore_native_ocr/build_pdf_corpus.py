"""Build leakage-safe native OCR line crops from PDF text coordinates.

Documents are assigned to train/dev/test before lines are produced.  A PDF's
pages and paraphernalia can therefore never leak across splits.  Scanned PDFs
without a trustworthy text layer are recorded for a later verified-label pass;
they are not silently pseudo-labelled here.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import tempfile
from collections import defaultdict
from pathlib import Path
from typing import Any

import pdfplumber
from PIL import Image

from .text_normalization import sanitize_text_layer


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def document_split(document_hash: str) -> str:
    bucket = int(document_hash[:8], 16) % 100
    if bucket < 80:
        return "train"
    if bucket < 90:
        return "dev"
    return "test"


def readable_ratio(text: str) -> float:
    if not text:
        return 0.0
    readable = sum(
        character.isspace()
        or character.isascii()
        or "가" <= character <= "힣"
        or "ㄱ" <= character <= "ㆎ"
        or character in "㎡㎥㎜㎝㎞㎾㎿℃ℓ·…→←↔※○●□■△▲▽▼"
        for character in text
    )
    return readable / len(text)


def trustworthy_text_layer(text: str) -> bool:
    """Reject extraction artifacts before they can become OCR ground truth."""

    if not text or "\ufffd" in text:
        return False
    if re.search(r"\(cid:\d+\)", text, flags=re.IGNORECASE):
        return False
    if any(0xD800 <= ord(character) <= 0xDFFF for character in text):
        return False
    return readable_ratio(text) >= 0.96


def group_lines(words: list[dict[str, Any]], tolerance: float = 4.0) -> list[list[dict[str, Any]]]:
    lines: list[list[dict[str, Any]]] = []
    for word in sorted(words, key=lambda item: (float(item["top"]), float(item["x0"]))):
        centre = (float(word["top"]) + float(word["bottom"])) / 2
        target = next(
            (
                line
                for line in lines
                if abs(
                    sum((float(item["top"]) + float(item["bottom"])) / 2 for item in line) / len(line)
                    - centre
                )
                <= tolerance
            ),
            None,
        )
        if target is None:
            lines.append([word])
        else:
            target.append(word)
    return [sorted(line, key=lambda item: float(item["x0"])) for line in lines]


def render_page(pdf: Path, page_number: int, dpi: int, output: Path) -> None:
    executable = shutil.which("pdftoppm")
    if executable is None:
        raise RuntimeError("PDFTOPPM_NOT_AVAILABLE")
    prefix = output.with_suffix("")
    subprocess.run(
        [
            executable,
            "-f",
            str(page_number),
            "-l",
            str(page_number),
            "-r",
            str(dpi),
            "-png",
            "-singlefile",
            str(pdf),
            str(prefix),
        ],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )


def stratified_page_numbers(total_pages: int, maximum_pages: int) -> list[int]:
    """Select deterministic page coverage across an entire document."""

    if maximum_pages <= 0 or total_pages <= maximum_pages:
        return list(range(1, total_pages + 1))
    if maximum_pages == 1:
        return [1]
    return sorted(
        {
            1 + round(index * (total_pages - 1) / (maximum_pages - 1))
            for index in range(maximum_pages)
        }
    )


def build_document(
    pdf: Path,
    output_root: Path,
    dpi: int,
    split_override: str | None = None,
    maximum_pages: int = 0,
) -> dict[str, Any]:
    document_hash = sha256(pdf)
    split = split_override or document_split(document_hash)
    if split not in {"train", "dev", "test"}:
        raise ValueError(f"B_CORE_NATIVE_OCR_SPLIT_INVALID:{split}")
    crop_root = output_root / "images" / split / document_hash[:16]
    crop_root.mkdir(parents=True, exist_ok=True)
    records: list[dict[str, Any]] = []
    scan_pages: list[int] = []
    with pdfplumber.open(pdf) as document, tempfile.TemporaryDirectory(prefix="bcore-native-ocr-") as temp:
        temp_root = Path(temp)
        total_pages = len(document.pages)
        selected_pages = set(stratified_page_numbers(total_pages, maximum_pages))
        for page_index, page in enumerate(document.pages, start=1):
            if page_index not in selected_pages:
                continue
            words = page.extract_words(x_tolerance=1.5, y_tolerance=3.0, keep_blank_chars=False)
            lines = group_lines(words)
            accepted = []
            for line in lines:
                text = sanitize_text_layer(
                    " ".join(str(word.get("text", "")).strip() for word in line)
                )
                if len(text) < 2 or not trustworthy_text_layer(text):
                    continue
                accepted.append((line, text))
            if not accepted:
                scan_pages.append(page_index)
                continue
            rendered = temp_root / f"page-{page_index:04}.png"
            render_page(pdf, page_index, dpi, rendered)
            with Image.open(rendered) as image:
                scale_x = image.width / float(page.width)
                scale_y = image.height / float(page.height)
                for line_index, (line, text) in enumerate(accepted):
                    x0 = max(0, int(min(float(word["x0"]) for word in line) * scale_x) - 6)
                    y0 = max(0, int(min(float(word["top"]) for word in line) * scale_y) - 4)
                    x1 = min(image.width, int(max(float(word["x1"]) for word in line) * scale_x) + 6)
                    y1 = min(image.height, int(max(float(word["bottom"]) for word in line) * scale_y) + 4)
                    if x1 - x0 < 8 or y1 - y0 < 8:
                        continue
                    relative = Path("images") / split / document_hash[:16] / f"p{page_index:04}-l{line_index:04}.png"
                    image.crop((x0, y0, x1, y1)).convert("L").save(output_root / relative)
                    records.append(
                        {
                            "schema": "B_CORE_NATIVE_OCR_LINE_1",
                            "split": split,
                            "image": relative.as_posix(),
                            "text": text,
                            "documentSha256": document_hash,
                            "sourceName": pdf.name,
                            "page": page_index,
                            "bboxPoints": [
                                min(float(word["x0"]) for word in line),
                                min(float(word["top"]) for word in line),
                                max(float(word["x1"]) for word in line),
                                max(float(word["bottom"]) for word in line),
                            ],
                            "labelSource": "pdf_text_layer",
                        }
                    )
    manifest = output_root / f"{split}.jsonl"
    with manifest.open("a", encoding="utf-8") as stream:
        for record in records:
            stream.write(json.dumps(record, ensure_ascii=False, separators=(",", ":")) + "\n")
    return {
        "name": pdf.name,
        "sha256": document_hash,
        "split": split,
        "lineCrops": len(records),
        "scanPagesNeedingVerifiedLabels": scan_pages,
        "selectedPages": len(selected_pages),
        "totalPages": total_pages,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--dpi", type=int, default=300)
    parser.add_argument("--split-map", type=Path)
    parser.add_argument(
        "--max-pages-per-document",
        type=int,
        default=0,
        help="Deterministically sample this many pages across each document; 0 keeps all pages.",
    )
    parser.add_argument("pdfs", nargs="+", type=Path)
    arguments = parser.parse_args()
    arguments.output.mkdir(parents=True, exist_ok=True)
    split_map: dict[str, str] = {}
    if arguments.split_map:
        split_map = {
            str(key).lower(): str(value)
            for key, value in json.loads(arguments.split_map.read_text(encoding="utf-8")).items()
        }
    receipt_path = arguments.output / "corpus-receipt.json"
    existing_documents: list[dict[str, Any]] = []
    if receipt_path.exists():
        existing = json.loads(receipt_path.read_text(encoding="utf-8"))
        if existing.get("schema") != "B_CORE_NATIVE_OCR_CORPUS_RECEIPT_1":
            raise ValueError("B_CORE_NATIVE_OCR_CORPUS_RECEIPT_INVALID")
        if int(existing.get("dpi", 0)) != arguments.dpi:
            raise ValueError("B_CORE_NATIVE_OCR_CORPUS_DPI_MISMATCH")
        existing_documents = list(existing.get("documents", []))
    known_hashes = {str(document.get("sha256")) for document in existing_documents}
    results = []
    for path in arguments.pdfs:
        resolved = path.resolve(strict=True)
        if sha256(resolved) in known_hashes:
            continue
        resolved_hash = sha256(resolved)
        result = build_document(
            resolved,
            arguments.output,
            arguments.dpi,
            split_map.get(resolved_hash),
            arguments.max_pages_per_document,
        )
        results.append(result)
        known_hashes.add(str(result["sha256"]))
    all_documents = existing_documents + results
    totals = defaultdict(int)
    for split in ("train", "dev", "test"):
        manifest = arguments.output / f"{split}.jsonl"
        if manifest.exists():
            totals[split] = sum(1 for line in manifest.open("r", encoding="utf-8") if line.strip())
    receipt = {
        "schema": "B_CORE_NATIVE_OCR_CORPUS_RECEIPT_1",
        "dpi": arguments.dpi,
        "maxPagesPerDocument": arguments.max_pages_per_document,
        "documents": all_documents,
        "lineCropsBySplit": dict(totals),
        "leakageBoundary": "document_sha256",
    }
    receipt_path.write_text(
        json.dumps(receipt, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    print(json.dumps(receipt, ensure_ascii=False))


if __name__ == "__main__":
    main()
