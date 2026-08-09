#!/usr/bin/env python3
"""Build a task-neutral external adapter request from a public fixture."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--public-fixture", type=Path, required=True)
    parser.add_argument("--image-sha256", required=True)
    parser.add_argument("--image-bytes", type=int, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    fixture = json.loads(args.public_fixture.read_text(encoding="utf-8"))
    if fixture.get("reference_patch_included") or fixture.get("hints_included"):
        raise RuntimeError("public fixture authority boundary violated")
    payload = {
        "request_id": f"EXT0::{fixture['instance_id']}",
        "problem_class": "REPOSITORY_REPAIR",
        "issue_text": fixture["problem_statement"],
        "repository_revision": fixture["base_commit"],
        "repository_artifacts": [
            {
                "relative_path": "sealed_fixture/repository_image",
                "content_sha256": args.image_sha256,
                "byte_length": args.image_bytes,
                "executable": False,
            }
        ],
        "executable_observations": [],
        "constraints": [
            "CANONICAL_NETWORK_DISABLED",
            "GOLD_REFERENCE_FORBIDDEN",
            "HIDDEN_TESTS_EVALUATOR_ONLY",
            "PATCH_MUST_BE_PRODUCED_BY_B_CORE",
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    temporary = args.output.with_suffix(args.output.suffix + ".tmp")
    temporary.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    os.replace(temporary, args.output)


if __name__ == "__main__":
    main()
