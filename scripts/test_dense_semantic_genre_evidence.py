#!/usr/bin/env python3
"""Regression checks for dense semantic genre evidence."""
from __future__ import annotations

import importlib.util
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESEARCH = ROOT / "research" / "BCORE_SPARSE_BLOCK_LANGUAGE"
MINER = RESEARCH / "mine_dense_semantic_genre_evidence.py"
SOURCE = RESEARCH / "dense_semantic_genre_gold_040_ko.tsv"
ARTIFACT = RESEARCH / "dense_semantic_genre_evidence_v1.json"


def load_module():
    spec = importlib.util.spec_from_file_location("dense_semantic_miner", MINER)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main() -> None:
    miner = load_module()
    rows = miner.load_rows(SOURCE)
    generated = miner.build(rows, SOURCE)
    committed = json.loads(ARTIFACT.read_text(encoding="utf-8"))
    assert generated == committed
    assert len(rows) == 40
    assert len(committed["genre_counts"]) == 8
    assert committed["split_counts"] == {"SEALED_BLIND": 8, "TRAIN": 24, "VALIDATION": 8}
    assert committed["semantic_authority"] is False
    assert committed["causal_truth_established"] is False
    assert committed["raw_surface_runtime_storage"] == "FORBIDDEN"
    assert set(committed["node_cue_evidence"]) >= {
        "CAUSAL_MECHANISM", "STATE_TRANSITION", "CONSTRAINT", "BOTTLENECK",
        "VALUE_OR_GOAL", "TRADEOFF", "THRESHOLD", "INTERVENTION",
    }
    encoded = json.dumps(committed, ensure_ascii=False)
    for row in rows:
        assert row["text"] not in encoded
    for genre in committed["genre_counts"]:
        genre_splits = {item["split"] for item in committed["observations"] if item["genre"] == genre}
        assert genre_splits == {"TRAIN", "VALIDATION", "SEALED_BLIND"}
    print(json.dumps({
        "families": 40,
        "genres": 8,
        "split": "24/8/8",
        "raw_surface_leak": 0,
        "deterministic_replay": "PASS",
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
