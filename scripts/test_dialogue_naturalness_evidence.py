from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent
sys.path.insert(0, str(SCRIPT_DIR))

from build_korean_corpus_attestation_index import ending_of, sentences


class DialogueNaturalnessEvidenceTests(unittest.TestCase):
    source = REPO_ROOT / "research/BCORE_SPARSE_BLOCK_LANGUAGE/dialogue_deliberation_gold_025_ko.tsv"
    artifact = REPO_ROOT / "research/BCORE_SPARSE_BLOCK_LANGUAGE/dialogue_deliberation_construction_evidence_v1.json"

    def test_dialogue_tsv_extracts_only_the_surface_column(self) -> None:
        rows = list(sentences(self.source, set(), 3))
        self.assertEqual(len(rows), 100)
        self.assertTrue(all("\t" not in row for row in rows))
        self.assertTrue(all(ending_of(row) is not None for row in rows))

    def test_conversational_ending_families_are_structural(self) -> None:
        cases = {
            "그렇게 해 볼까 싶어.": "CASUAL_DELIBERATIVE",
            "시간이 더 걸릴걸?": "CASUAL_PREDICTIVE",
            "그게 원인이야.": "CASUAL_COPULAR",
            "일단 쉬어 봐.": "CASUAL_SUGGESTIVE",
            "그런 것 같더라고.": "CASUAL_RECALL",
            "계속 확인해야 해.": "CASUAL_FINITE",
        }
        for surface, expected in cases.items():
            with self.subTest(surface=surface):
                self.assertEqual(ending_of(surface), expected)

    def test_mined_artifact_contains_no_raw_dialogue(self) -> None:
        payload = json.loads(self.artifact.read_text(encoding="utf-8"))
        serialized = json.dumps(payload, ensure_ascii=False)
        rows = list(sentences(self.source, set(), 3))
        self.assertTrue(all(row not in serialized for row in rows))
        self.assertEqual(payload["split_counts"], {"SEALED_BLIND": 3, "TRAIN": 18, "VALIDATION": 4})
        self.assertEqual(payload["source"]["turn_count"], 100)


if __name__ == "__main__":
    unittest.main()
