# B_Core native OCR adapter

This directory contains the bounded Python perception adapter used by the
optional Cargo feature `python-bcore-native-ocr`.  It is outside the semantic
core: it emits pixel-grounded observations, confidence, geometry, review
status and provenance; it cannot update facts or authorize actions.

The runtime contains no model download and no external OCR fallback.  Configure
the separately versioned owned artifacts with:

- `B_CORE_NATIVE_OCR_PYTHON`
- `B_CORE_NATIVE_OCR_DETECTOR`
- `B_CORE_NATIVE_OCR_RECOGNIZER`
- optional `B_CORE_NATIVE_OCR_DEVICE` (default `gpu:0`)
- optional `B_CORE_NATIVE_OCR_HOME` (defaults to this `python` directory)

Build the adapter explicitly with
`--features python-bcore-native-ocr`.  The Rust-only default remains unchanged.
Rows not accepted by the native structural verifier are deliberately exported
with non-authoritative confidence and remain review evidence.
