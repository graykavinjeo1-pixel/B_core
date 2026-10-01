"""Stable B_Core process boundary for the native document OCR runtime.

This adapter intentionally emits observations, confidence and provenance only.
It never turns a low-confidence OCR row into an authoritative fact.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

import pdfplumber


def _page_numbers(pdf: Path) -> list[int]:
    with pdfplumber.open(pdf) as document:
        return list(range(1, len(document.pages) + 1))


def _run_native_reader(
    pdf: Path,
    detector: Path,
    recognizer: Path,
    output: Path,
    device: str,
    pages: list[int] | None = None,
) -> None:
    available_pages = _page_numbers(pdf)
    pages = pages or available_pages
    if any(page not in available_pages for page in pages):
        raise RuntimeError("REQUESTED_PAGE_OUT_OF_RANGE")
    if not pages:
        raise RuntimeError("PDF_HAS_NO_PAGES")
    command = [
        sys.executable,
        "-m",
        "bcore_native_ocr.recognize_page_tables",
        str(pdf),
        "--detector",
        str(detector),
        "--recognizer",
        str(recognizer),
        "--output",
        str(output),
        "--pages",
        ",".join(str(page) for page in pages),
        "--device",
        device,
        "--independent-ocr-observer",
        "off",
    ]
    completed = subprocess.run(command, check=False, capture_output=True, text=True)
    if completed.returncode != 0:
        detail = completed.stderr.strip().splitlines()[-1:] or ["UNKNOWN"]
        raise RuntimeError(f"NATIVE_OCR_FAILED:{detail[0]}")


def _collect_rows(output: Path) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for receipt_path in sorted(output.glob("p*.json")):
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        page = int(receipt.get("page", 0))
        evidence = receipt.get("approvedTextEvidence", {})
        for row in evidence.get("rows", []):
            text = str(row.get("text", "")).strip()
            if not text:
                continue
            source = str(row.get("source", "native_raster_ocr"))
            replacement_ratio = text.count("\ufffd") / max(1, len(text))
            encoding_integrity = replacement_ratio == 0.0
            confidence = float(row.get("confidence", 0.0))
            fact_authority = bool(row.get("factAuthority", False)) and encoding_integrity
            if fact_authority and source == "embedded_pdf_text" and confidence <= 0.0:
                confidence = 1.0
            rows.append(
                {
                    "text": text,
                    "score": confidence if fact_authority else min(confidence, 0.49),
                    "box": row.get("box", []),
                    "page": page,
                    "status": row.get("status", "needs_review"),
                    "decisionStatus": row.get("decisionStatus", "MANUAL_REVIEW"),
                    "factAuthority": fact_authority,
                    "evidenceId": row.get("evidenceId"),
                    "source": source,
                }
            )
    return rows


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument("--detector", type=Path, required=True)
    parser.add_argument("--recognizer", type=Path, required=True)
    parser.add_argument("--device", default="gpu:0")
    parser.add_argument(
        "--pages",
        help="Optional comma-separated one-based pages; omitted means every page.",
    )
    arguments = parser.parse_args()

    if arguments.input.suffix.lower() != ".pdf":
        parser.error("the native B_Core adapter currently accepts PDF input only")
    for label, path in (
        ("detector", arguments.detector),
        ("recognizer", arguments.recognizer),
    ):
        if not (path / "model.json").is_file():
            parser.error(f"--{label} must contain model.json")

    with tempfile.TemporaryDirectory(prefix="bcore-native-ocr-") as directory:
        output = Path(directory)
        pages = (
            [int(value) for value in arguments.pages.split(",")]
            if arguments.pages
            else None
        )
        _run_native_reader(
            arguments.input,
            arguments.detector,
            arguments.recognizer,
            output,
            arguments.device,
            pages,
        )
        rows = _collect_rows(output)
    print(
        "B_CORE_NATIVE_OCR_JSON="
        + json.dumps(rows, ensure_ascii=False, separators=(",", ":"))
    )


if __name__ == "__main__":
    main()
