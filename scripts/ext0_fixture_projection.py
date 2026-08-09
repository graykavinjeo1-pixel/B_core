#!/usr/bin/env python3
"""Project one frozen SWE-bench row without exposing the reference solution.

The public projection is suitable for the system under evaluation. The evaluator
projection is restricted to the official harness and intentionally omits both
the reference patch and hints.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path

import pyarrow.parquet as pq


PUBLIC_COLUMNS = (
    "repo",
    "instance_id",
    "base_commit",
    "problem_statement",
    "created_at",
    "version",
    "environment_setup_commit",
    "difficulty",
)

EVALUATOR_COLUMNS = (
    "repo",
    "instance_id",
    "base_commit",
    "problem_statement",
    "test_patch",
    "version",
    "FAIL_TO_PASS",
    "PASS_TO_PASS",
    "environment_setup_commit",
)

FORBIDDEN_COLUMNS = frozenset({"patch", "hints_text"})


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--parquet", type=Path, required=True)
    parser.add_argument("--instance-id", required=True)
    parser.add_argument("--mode", choices=("public", "evaluator"), required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    columns = PUBLIC_COLUMNS if args.mode == "public" else EVALUATOR_COLUMNS
    if FORBIDDEN_COLUMNS.intersection(columns):
        raise RuntimeError("forbidden reference columns requested")

    table = pq.read_table(args.parquet, columns=list(columns))
    rows = [
        row
        for row in table.to_pylist()
        if row.get("instance_id") == args.instance_id
    ]
    if len(rows) != 1:
        raise RuntimeError(f"expected one selected row, found {len(rows)}")

    payload = rows[0]
    payload["projection_mode"] = args.mode.upper()
    payload["reference_patch_included"] = False
    payload["hints_included"] = False
    args.output.parent.mkdir(parents=True, exist_ok=True)
    temporary = args.output.with_suffix(args.output.suffix + ".tmp")
    serialized = payload if args.mode == "public" else [payload]
    temporary.write_text(
        json.dumps(serialized, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    os.replace(temporary, args.output)


if __name__ == "__main__":
    main()
