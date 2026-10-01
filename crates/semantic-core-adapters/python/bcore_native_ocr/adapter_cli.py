"""Stable B_Core process boundary for the native document OCR runtime.

This adapter intentionally emits observations, confidence and provenance only.
It never turns a low-confidence OCR row into an authoritative fact.
"""

from __future__ import annotations

import argparse
import contextlib
import io
import json
import os
import sys
import tempfile
import time
from pathlib import Path

import pdfplumber


@contextlib.contextmanager
def _suppress_native_stdout():
    """Keep C/CUDA library diagnostics out of the NDJSON worker channel.

    ``redirect_stdout`` only replaces Python's text stream.  Some native OCR
    dependencies write directly to file descriptor 1 while loading models,
    which would corrupt the Rust worker's one-line response protocol.  This
    process is request-serial, so temporarily redirecting that descriptor is
    safe and always restored before the protocol response is emitted.
    """

    stdout_fd = sys.stdout.fileno()
    sys.stdout.flush()
    saved_stdout = os.dup(stdout_fd)
    try:
        with open(os.devnull, "wb") as sink:
            os.dup2(sink.fileno(), stdout_fd)
            try:
                yield
            finally:
                sys.stdout.flush()
                os.dup2(saved_stdout, stdout_fd)
    finally:
        os.close(saved_stdout)


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
    # Keep recognition in this process.  The reader owns a path/device keyed
    # model cache, but a subprocess boundary used to discard that cache after
    # every document.  The adapter's worker mode keeps this bounded process
    # alive while the Rust side serializes GPU use through one local worker.
    from .recognize_page_tables import main as recognize_pages

    command = [
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
        "--orientation-observer",
        "off",
        "--high-confidence-row-fast-path",
        "--high-confidence-row-floor",
        "0.95",
        "--orientation-zero-degree-fast-path",
    ]
    # `recognize_page_tables` emits per-page research receipts to stdout.
    # Those receipts remain in its temporary output files; the process
    # boundary exposes only the one structured adapter response.
    try:
        with contextlib.redirect_stdout(io.StringIO()):
            recognize_pages(command)
    except SystemExit as error:
        raise RuntimeError(f"NATIVE_OCR_FAILED:ARGUMENTS:{error.code}") from error


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


def _read_native_ocr(
    pdf: Path,
    detector: Path,
    recognizer: Path,
    device: str,
    pages: list[int] | None,
) -> list[dict[str, object]]:
    if pdf.suffix.lower() != ".pdf":
        raise RuntimeError("NATIVE_OCR_INPUT_NOT_PDF")
    for label, path in (("detector", detector), ("recognizer", recognizer)):
        if not (path / "model.json").is_file():
            raise RuntimeError(f"NATIVE_OCR_{label.upper()}_NOT_CONFIGURED")
    with tempfile.TemporaryDirectory(prefix="bcore-native-ocr-") as directory:
        output = Path(directory)
        _run_native_reader(pdf, detector, recognizer, output, device, pages)
        return _collect_rows(output)


def _serve() -> None:
    """Serve bounded newline-delimited local OCR requests.

    This is deliberately a local process protocol rather than a network
    service.  Each request has its own input/output evidence boundary, while
    the owned model cache and CUDA allocations remain warm inside this one
    process.  A malformed request yields a fail-closed error response and
    cannot affect the following request.
    """

    for raw_line in sys.stdin:
        started = time.perf_counter()
        request: object = None
        try:
            request = json.loads(raw_line)
            if not isinstance(request, dict):
                raise ValueError("REQUEST_NOT_OBJECT")
            request_id = request.get("requestId")
            if not isinstance(request_id, str) or not request_id:
                raise ValueError("REQUEST_ID_REQUIRED")
            raw_pages = request.get("pages")
            if raw_pages is not None and (
                not isinstance(raw_pages, list)
                or any(not isinstance(page, int) for page in raw_pages)
            ):
                raise ValueError("REQUEST_PAGES_INVALID")
            with _suppress_native_stdout():
                rows = _read_native_ocr(
                    Path(str(request["input"])),
                    Path(str(request["detector"])),
                    Path(str(request["recognizer"])),
                    str(request.get("device", "gpu:0")),
                    raw_pages,
                )
            response: dict[str, object] = {
                "requestId": request_id,
                "status": "ok",
                "rows": rows,
            }
        except Exception as error:
            response = {
                "requestId": request.get("requestId")
                if isinstance(request, dict)
                else None,
                "status": "error",
                "code": type(error).__name__,
                "detail": str(error),
            }
        response["elapsedMilliseconds"] = round(
            (time.perf_counter() - started) * 1000,
            3,
        )
        # Windows inherits the console code page for ``sys.stdout`` in some
        # launch contexts.  The Rust protocol is explicitly UTF-8, so bypass
        # that text encoder and write one UTF-8 NDJSON frame directly.
        frame = (json.dumps(response, ensure_ascii=False, separators=(",", ":")) + "\n").encode(
            "utf-8"
        )
        sys.stdout.buffer.write(frame)
        sys.stdout.buffer.flush()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path, nargs="?")
    parser.add_argument("--detector", type=Path)
    parser.add_argument("--recognizer", type=Path)
    parser.add_argument("--device", default="gpu:0")
    parser.add_argument("--serve", action="store_true")
    parser.add_argument(
        "--pages",
        help="Optional comma-separated one-based pages; omitted means every page.",
    )
    arguments = parser.parse_args()

    if arguments.serve:
        if arguments.input is not None or arguments.pages:
            parser.error("--serve accepts requests through stdin only")
        _serve()
        return
    if arguments.input is None:
        parser.error("input is required unless --serve is selected")
    if arguments.detector is None or arguments.recognizer is None:
        parser.error("--detector and --recognizer are required unless --serve is selected")
    pages = [int(value) for value in arguments.pages.split(",")] if arguments.pages else None
    try:
        rows = _read_native_ocr(
            arguments.input,
            arguments.detector,
            arguments.recognizer,
            arguments.device,
            pages,
        )
    except RuntimeError as error:
        parser.error(str(error))
    print(
        "B_CORE_NATIVE_OCR_JSON="
        + json.dumps(rows, ensure_ascii=False, separators=(",", ":"))
    )


if __name__ == "__main__":
    main()
