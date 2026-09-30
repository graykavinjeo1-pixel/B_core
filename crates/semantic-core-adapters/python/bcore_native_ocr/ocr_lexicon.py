"""Train-only Korean lexical memory for conservative OCR error recovery."""

from __future__ import annotations

import json
import math
import re
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable

from .ocr_language_model import CharacterLanguageModel


KOREAN_WORD = re.compile(r"[가-힣]{2,}")
FRAGMENTED_KOREAN_RUN = re.compile(
    r"(?<![가-힣])(?P<run>[가-힣](?:[ \t]+[가-힣]){1,7})(?![가-힣])"
)


def _within_one_edit(left: str, right: str) -> bool:
    if abs(len(left) - len(right)) > 1:
        return False
    if len(left) == len(right):
        return sum(a != b for a, b in zip(left, right)) <= 1
    shorter, longer = (left, right) if len(left) < len(right) else (right, left)
    short_index = 0
    long_index = 0
    skipped = 0
    while short_index < len(shorter) and long_index < len(longer):
        if shorter[short_index] == longer[long_index]:
            short_index += 1
            long_index += 1
        else:
            skipped += 1
            long_index += 1
            if skipped > 1:
                return False
    return True


def _deletions(word: str) -> set[str]:
    return {word, *(word[:index] + word[index + 1 :] for index in range(len(word)))}


@dataclass
class OcrLexicon:
    counts: dict[str, int]
    minimum_word_frequency: int = 2
    provenance: dict = field(default_factory=dict)
    _index: dict[str, tuple[str, ...]] = field(default_factory=dict, init=False, repr=False)

    @classmethod
    def from_texts(cls, texts: Iterable[str], minimum_word_frequency: int = 2) -> "OcrLexicon":
        counts: Counter[str] = Counter()
        for text in texts:
            counts.update(KOREAN_WORD.findall(text))
        return cls(
            counts={word: count for word, count in counts.items() if count >= minimum_word_frequency},
            minimum_word_frequency=minimum_word_frequency,
        )

    def _ensure_index(self) -> None:
        if self._index:
            return
        index: dict[str, list[str]] = defaultdict(list)
        for word in self.counts:
            for deletion in _deletions(word):
                index[deletion].append(word)
        self._index = {key: tuple(values) for key, values in index.items()}

    def candidates(self, word: str) -> list[str]:
        self._ensure_index()
        candidates = {
            candidate
            for deletion in _deletions(word)
            for candidate in self._index.get(deletion, ())
            if candidate != word and _within_one_edit(word, candidate)
        }
        return sorted(candidates, key=lambda value: (-self.counts[value], value))

    def recover_fragmented_spacing(
        self,
        text: str,
        *,
        minimum_frequency: int = 1,
        maximum_changes: int = 3,
    ) -> tuple[str, list[dict]]:
        """Join visually fragmented Korean syllables only when memory knows the word.

        OCR sometimes inserts spaces between every glyph of a short printed
        heading.  The correction is target-free: it considers only runs made
        entirely of isolated Hangul syllables and joins a span only when the
        resulting word already exists in independently built lexical memory.
        Normal multi-syllable word boundaries and unknown names are untouched.
        """

        stripped = text.strip()
        match = FRAGMENTED_KOREAN_RUN.fullmatch(stripped)
        if match is None:
            return text, []
        candidate = re.sub(r"[ \t]+", "", match.group("run"))
        frequency = self.counts.get(candidate, 0)
        if frequency < minimum_frequency:
            return text, []
        start = text.find(stripped)
        recovered = text[:start] + candidate + text[start + len(stripped) :]
        return recovered, [
            {
                "observed": stripped,
                "candidate": candidate,
                "candidateFrequency": frequency,
                "evidence": "whole_line_isolated_syllables_and_independent_lexical_memory",
            }
        ]

    def correct(
        self,
        text: str,
        language_model: CharacterLanguageModel,
        *,
        minimum_language_gain: float = 0.035,
        minimum_frequency_ratio: float = 3.0,
        maximum_changes: int = 3,
    ) -> tuple[str, list[dict]]:
        """Correct only independently learned words with stronger context.

        The original OCR string remains authoritative unless both the lexical
        frequency gate and a full-string train-only language score improve.
        """

        current = text
        decisions = []
        for _ in range(maximum_changes):
            current_score = language_model.score(current)
            best = None
            for match in KOREAN_WORD.finditer(current):
                observed = match.group(0)
                observed_frequency = self.counts.get(observed, 0)
                for candidate in self.candidates(observed):
                    candidate_frequency = self.counts[candidate]
                    required_frequency = max(
                        self.minimum_word_frequency,
                        math.ceil(max(1, observed_frequency) * minimum_frequency_ratio),
                    )
                    if candidate_frequency < required_frequency:
                        continue
                    replacement = current[: match.start()] + candidate + current[match.end() :]
                    replacement_score = language_model.score(replacement)
                    gain = replacement_score - current_score
                    if gain < minimum_language_gain:
                        continue
                    rank = gain + 0.01 * math.log1p(candidate_frequency)
                    if best is None or rank > best[0]:
                        best = (
                            rank,
                            replacement,
                            {
                                "observed": observed,
                                "candidate": candidate,
                                "languageGain": gain,
                                "candidateFrequency": candidate_frequency,
                                "observedFrequency": observed_frequency,
                            },
                        )
            if best is None:
                break
            current = best[1]
            decisions.append(best[2])
        return current, decisions

    def save(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            json.dumps(
                {
                    "schema": "B_CORE_NATIVE_OCR_LEXICON_1",
                    "minimumWordFrequency": self.minimum_word_frequency,
                    "counts": self.counts,
                    "provenance": self.provenance,
                },
                ensure_ascii=False,
                separators=(",", ":"),
            ),
            encoding="utf-8",
        )

    @classmethod
    def load(cls, path: Path) -> "OcrLexicon":
        payload = json.loads(path.read_text(encoding="utf-8"))
        if payload.get("schema") != "B_CORE_NATIVE_OCR_LEXICON_1":
            raise ValueError("B_CORE_OCR_LEXICON_SCHEMA")
        return cls(
            counts={str(word): int(count) for word, count in payload["counts"].items()},
            minimum_word_frequency=int(payload["minimumWordFrequency"]),
            provenance=dict(payload.get("provenance", {})),
        )
