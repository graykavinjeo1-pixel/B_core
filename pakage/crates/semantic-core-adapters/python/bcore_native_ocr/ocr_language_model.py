"""Small train-only character language model for OCR candidate calibration."""

from __future__ import annotations

import json
import math
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable


@dataclass
class CharacterLanguageModel:
    order: int
    vocabulary_size: int
    unigram: dict[str, int]
    ngram: dict[str, int]
    context: dict[str, int]
    alpha: float = 0.15
    calibration: dict[str, float] = field(default_factory=dict)

    @classmethod
    def from_texts(cls, texts: Iterable[str], order: int = 3) -> "CharacterLanguageModel":
        unigram: Counter[str] = Counter()
        ngram: Counter[str] = Counter()
        context: Counter[str] = Counter()
        for text in set(value.strip() for value in texts if value.strip()):
            sequence = "^" * (order - 1) + text + "$"
            unigram.update(text + "$")
            for index in range(order - 1, len(sequence)):
                history = sequence[index - order + 1 : index]
                token = sequence[index]
                context[history] += 1
                ngram[history + token] += 1
        return cls(
            order=order,
            vocabulary_size=max(1, len(unigram)),
            unigram=dict(unigram),
            ngram=dict(ngram),
            context=dict(context),
        )

    def score(self, text: str) -> float:
        value = text.strip()
        if not value:
            return -20.0
        sequence = "^" * (self.order - 1) + value + "$"
        unigram_total = sum(self.unigram.values())
        scores = []
        for index in range(self.order - 1, len(sequence)):
            history = sequence[index - self.order + 1 : index]
            token = sequence[index]
            count = self.ngram.get(history + token, 0)
            history_count = self.context.get(history, 0)
            if history_count:
                probability = (count + self.alpha) / (history_count + self.alpha * self.vocabulary_size)
            else:
                probability = (self.unigram.get(token, 0) + self.alpha) / (
                    unigram_total + self.alpha * self.vocabulary_size
                )
            scores.append(math.log(max(probability, 1e-12)))
        return sum(scores) / max(1, len(scores))

    def save(self, path: Path, provenance: dict) -> None:
        path.write_text(
            json.dumps(
                {
                    "schema": "B_CORE_NATIVE_OCR_CHARACTER_LM_1",
                    "order": self.order,
                    "vocabularySize": self.vocabulary_size,
                    "alpha": self.alpha,
                    "unigram": self.unigram,
                    "ngram": self.ngram,
                    "context": self.context,
                    "calibration": self.calibration,
                    "provenance": provenance,
                },
                ensure_ascii=False,
                separators=(",", ":"),
            ),
            encoding="utf-8",
        )

    @classmethod
    def load(cls, path: Path) -> "CharacterLanguageModel":
        payload = json.loads(path.read_text(encoding="utf-8"))
        if payload.get("schema") != "B_CORE_NATIVE_OCR_CHARACTER_LM_1":
            raise ValueError("B_CORE_OCR_LANGUAGE_MODEL_SCHEMA")
        return cls(
            order=int(payload["order"]),
            vocabulary_size=int(payload["vocabularySize"]),
            alpha=float(payload["alpha"]),
            unigram={str(key): int(value) for key, value in payload["unigram"].items()},
            ngram={str(key): int(value) for key, value in payload["ngram"].items()},
            context={str(key): int(value) for key, value in payload["context"].items()},
            calibration={str(key): float(value) for key, value in payload.get("calibration", {}).items()},
        )
