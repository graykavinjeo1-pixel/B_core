#!/usr/bin/env python3
"""Regression checks for Kakao-style causal dialogue evidence."""
from __future__ import annotations

import importlib.util
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESEARCH = ROOT / "research" / "BCORE_SPARSE_BLOCK_LANGUAGE"
MINER = RESEARCH / "mine_kakao_colloquial_causal_evidence.py"
SOURCE = RESEARCH / "kakao_colloquial_causal_dialogue_020_ko.tsv"
ARTIFACT = RESEARCH / "kakao_colloquial_causal_dialogue_evidence_v1.json"


def load_module():
    spec = importlib.util.spec_from_file_location("kakao_causal_miner", MINER)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main() -> None:
    miner = load_module()
    families = miner.load_families(SOURCE)
    generated = miner.build(families, SOURCE)
    committed = json.loads(ARTIFACT.read_text(encoding="utf-8"))
    assert generated == committed
    assert committed["source"]["family_count"] == 20
    assert committed["source"]["turn_count"] == 81
    assert len(committed["topology_counts"]) == 7
    assert committed["split_counts"] == {"SEALED_BLIND": 7, "TRAIN": 7, "VALIDATION": 6}
    assert committed["semantic_authority"] is False
    assert committed["causal_truth_established"] is False
    cue_names = set(committed["normalized_cue_recurrence"])
    assert {
        "COLLOQUIAL_CUES::LAUGHTER_JAMO",
        "COLLOQUIAL_CUES::ELLIPSIS",
        "COLLOQUIAL_CUES::NOMINALIZED_CHAT_ENDING",
        "REASONING_CUES::THRESHOLD",
        "REASONING_CUES::PROXY_DISTORTION",
    } <= cue_names
    encoded = json.dumps(committed, ensure_ascii=False)
    for family in families:
        for _, surface in family["turns"]:
            assert surface not in encoded
    print(json.dumps({
        "families": 20,
        "turns": 81,
        "topologies": 7,
        "split": "7/6/7",
        "raw_surface_leak": 0,
        "deterministic_replay": "PASS",
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
