# B_Core-owned native OCR assets

This directory contains the frozen visual checkpoints used by the
`python-bcore-native-ocr` adapter. They are B_Core runtime assets, not an
external OCR service.

- `models/table-detector-scanaug-0003`: document text/table geometry detector
- `models/bootstrap-component-refinement-v60-v61-a050-fixed`: Korean line
  recognizer

The checkpoints remain fail-closed research assets. A row becomes an
authoritative fact only after the runtime confidence, structure, provenance,
and arithmetic gates accept it. Uncertain rows must stay in manual review.

Environment variables may override the bundled paths for controlled testing:

- `B_CORE_NATIVE_OCR_DETECTOR`
- `B_CORE_NATIVE_OCR_RECOGNIZER`
- `B_CORE_NATIVE_OCR_ASSET_ROOT`

Without an override, the Rust adapter resolves these bundled assets relative
to the B_Core repository/package layout.
