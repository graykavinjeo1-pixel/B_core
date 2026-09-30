"""Compositional Hangul CTC codec for sample-efficient Korean OCR."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

from .codec import ASCII, DOCUMENT_SYMBOLS, KOREAN_JAMO


LEADS = tuple("ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ")
VOWELS = tuple("ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ")
TAILS = tuple("ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ")


@dataclass(frozen=True)
class BCoreHangulComponentCodec:
    tokens: tuple[str, ...]

    @classmethod
    def standard(cls) -> "BCoreHangulComponentCodec":
        tokens = ["<END>"]
        tokens.extend(f"L:{index}" for index in range(len(LEADS)))
        tokens.extend(f"V:{index}" for index in range(len(VOWELS)))
        tokens.extend(f"T:{index + 1}" for index in range(len(TAILS)))
        tokens.extend(f"C:{character}" for character in dict.fromkeys(ASCII + KOREAN_JAMO + DOCUMENT_SYMBOLS))
        return cls(tuple(tokens))

    @property
    def size(self) -> int:
        return len(self.tokens) + 1

    def supports(self, text: str) -> bool:
        literals = {token[2:] for token in self.tokens if token.startswith("C:")}
        return all("가" <= character <= "힣" or character in literals for character in text)

    def encode(self, text: str) -> list[int]:
        lookup = {token: index + 1 for index, token in enumerate(self.tokens)}
        encoded: list[int] = []
        for character in text:
            codepoint = ord(character)
            if 0xAC00 <= codepoint <= 0xD7A3:
                offset = codepoint - 0xAC00
                lead = offset // (21 * 28)
                vowel = (offset % (21 * 28)) // 28
                tail = offset % 28
                encoded.extend((lookup[f"L:{lead}"], lookup[f"V:{vowel}"]))
                if tail:
                    encoded.append(lookup[f"T:{tail}"])
                encoded.append(lookup["<END>"])
            else:
                token = f"C:{character}"
                if token not in lookup:
                    raise ValueError(f"B_CORE_OCR_UNSUPPORTED_CHARACTER:{character}")
                encoded.append(lookup[token])
        return encoded

    def _collapsed_tokens(self, token_ids: list[int]) -> list[str]:
        collapsed: list[str] = []
        previous = -1
        for token_id in token_ids:
            if token_id != 0 and token_id != previous:
                if token_id < 0 or token_id > len(self.tokens):
                    raise ValueError(f"B_CORE_OCR_TOKEN_OUT_OF_RANGE:{token_id}")
                collapsed.append(self.tokens[token_id - 1])
            previous = token_id
        return collapsed

    def decode_ctc(self, token_ids: list[int]) -> str:
        output: list[str] = []
        lead: int | None = None
        vowel: int | None = None
        tail = 0

        def flush() -> None:
            nonlocal lead, vowel, tail
            if lead is not None and vowel is not None and 0 <= lead < 19 and 0 <= vowel < 21 and 0 <= tail < 28:
                output.append(chr(0xAC00 + (lead * 21 + vowel) * 28 + tail))
            lead = None
            vowel = None
            tail = 0

        for token in self._collapsed_tokens(token_ids):
            if token == "<END>":
                flush()
            elif token.startswith("L:"):
                # A new initial consonant is also a safe implicit syllable
                # boundary when the model omitted <END>.
                if lead is not None:
                    flush()
                lead = int(token[2:])
            elif token.startswith("V:"):
                if lead is not None and vowel is None:
                    vowel = int(token[2:])
            elif token.startswith("T:"):
                if lead is not None and vowel is not None and tail == 0:
                    tail = int(token[2:])
            elif token.startswith("C:"):
                flush()
                output.append(token[2:])
        flush()
        return "".join(output)

    def save(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            json.dumps(
                {
                    "schema": "B_CORE_OCR_HANGUL_COMPONENT_CODEC_1",
                    "blankTokenId": 0,
                    "unit": "hangul_component_or_literal",
                    "tokens": list(self.tokens),
                },
                ensure_ascii=False,
                separators=(",", ":"),
            ),
            encoding="utf-8",
        )

    @classmethod
    def load(cls, path: Path) -> "BCoreHangulComponentCodec":
        payload = json.loads(path.read_text(encoding="utf-8"))
        if payload.get("schema") != "B_CORE_OCR_HANGUL_COMPONENT_CODEC_1":
            raise ValueError("B_CORE_OCR_COMPONENT_CODEC_INVALID")
        return cls(tuple(str(token) for token in payload["tokens"]))


def load_codec(path: Path):
    payload = json.loads(path.read_text(encoding="utf-8"))
    if payload.get("schema") == "B_CORE_OCR_HANGUL_COMPONENT_CODEC_1":
        return BCoreHangulComponentCodec.load(path)
    from .codec import BCoreKoreanCodec

    return BCoreKoreanCodec.load(path)
