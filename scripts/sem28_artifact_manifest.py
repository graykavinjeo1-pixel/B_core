#!/usr/bin/env python3
"""Create a deterministic hash inventory for sealed SEM-28 report artifacts."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import tempfile


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> None:
    root = Path(__file__).resolve().parents[1]
    report = root / "reports" / "sem28"
    destination = report / "artifact_manifest.json"
    artifacts = []
    for path in sorted(report.rglob("*")):
        if not path.is_file() or path == destination:
            continue
        artifacts.append(
            {
                "path": path.relative_to(root).as_posix(),
                "bytes": path.stat().st_size,
                "sha256": sha256(path),
            }
        )
    value = {
        "schema_version": "SEM28_ARTIFACT_MANIFEST_1",
        "campaign_id": "SEM28-AUTONOMOUS-SUBSTRATE-GENESIS-0001",
        "artifact_count": len(artifacts),
        "artifacts": artifacts,
    }
    payload = json.dumps(value, indent=2, sort_keys=True).encode("utf-8") + b"\n"
    handle, temporary_name = tempfile.mkstemp(
        prefix=".artifact_manifest.", suffix=".tmp", dir=report
    )
    try:
        with os.fdopen(handle, "wb") as temporary:
            temporary.write(payload)
            temporary.flush()
            os.fsync(temporary.fileno())
        os.replace(temporary_name, destination)
    finally:
        if os.path.exists(temporary_name):
            os.unlink(temporary_name)


if __name__ == "__main__":
    main()
