#!/usr/bin/env python3
"""Regression checks for non-authoritative causal-dialogue evidence."""
from __future__ import annotations

import importlib.util
import json
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESEARCH = ROOT / "research" / "BCORE_SPARSE_BLOCK_LANGUAGE"
MINER_PATH = RESEARCH / "mine_causal_dialogue_topologies.py"
SOURCE_PATH = RESEARCH / "causal_dialogue_gold_024_ko.txt"
ARTIFACT_PATH = RESEARCH / "causal_dialogue_topology_evidence_v1.json"
TSV_PATH = RESEARCH / "causal_dialogue_gold_024_ko.tsv"


def load_miner():
    spec = importlib.util.spec_from_file_location("causal_dialogue_miner", MINER_PATH)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main() -> None:
    miner = load_miner()
    rows = miner.parse(SOURCE_PATH)
    assert len(rows) == 24
    assert sum(len(row["turns"]) for row in rows) == 96

    artifact = miner.build(rows, SOURCE_PATH)
    committed = json.loads(ARTIFACT_PATH.read_text(encoding="utf-8"))
    assert artifact == committed, "committed artifact is not deterministically reproducible"
    assert committed["semantic_authority"] is False
    assert committed["causal_truth_established"] is False
    assert committed["raw_surface_runtime_storage"] == "FORBIDDEN"
    assert committed["split_counts"] == {
        "SEALED_BLIND": 6,
        "TRAIN": 12,
        "VALIDATION": 6,
    }
    assert set(committed["topology_counts"].values()) == {4}
    assert len(committed["topology_counts"]) == 6

    by_topology: dict[str, set[str]] = {}
    for observation in committed["observations"]:
        by_topology.setdefault(observation["topology"], set()).add(observation["split"])
        assert "turns" not in observation
        assert "surface" not in observation
    expected_splits = {"TRAIN", "VALIDATION", "SEALED_BLIND"}
    assert all(splits == expected_splits for splits in by_topology.values())

    serialized = json.dumps(committed, ensure_ascii=False)
    for row in rows:
        for turn in row["turns"]:
            assert turn not in serialized, "raw dialogue leaked into evidence artifact"

    with tempfile.TemporaryDirectory() as temp_dir:
        copy_path = Path(temp_dir) / "artifact.json"
        tsv_path = Path(temp_dir) / "dialogue.tsv"
        copy_path.write_text(
            json.dumps(miner.build(rows, SOURCE_PATH), ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
        assert copy_path.read_bytes() == ARTIFACT_PATH.read_bytes()
        miner.write_dialogue_tsv(rows, tsv_path)
        assert tsv_path.read_bytes() == TSV_PATH.read_bytes()

    print(
        json.dumps(
            {
                "families": 24,
                "turns": 96,
                "topologies": 6,
                "split_contract": "12/6/6",
                "raw_surface_leak": 0,
                "deterministic_replay": "PASS",
            },
            ensure_ascii=False,
        )
    )


if __name__ == "__main__":
    main()
