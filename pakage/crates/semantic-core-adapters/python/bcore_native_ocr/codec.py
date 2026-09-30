"""Stable Unicode character contract for the native Korean OCR recognizer."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path


ASCII = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~"
KOREAN_JAMO = "".join(chr(codepoint) for codepoint in range(0x3131, 0x318F + 1))
HANGUL_SYLLABLES = "".join(chr(codepoint) for codepoint in range(0xAC00, 0xD7A3 + 1))
DOCUMENT_SYMBOLS = "㎡㎥㎜㎝㎞㎾㎿℃ℓ·…•‧∙․▪◦→←↔▶▷◀◁※○●□■△▲▽▼★☆✓☑☞☎①②③④⑤⑥⑦⑧⑨⑩—±「」×“”÷ＡＢＣＤ㈜∼"


@dataclass(frozen=True)
class BCoreKoreanCodec:
    """Character-level CTC codec.

    Korean syllables are kept as Unicode characters.  UTF-8 bytes are never
    generated independently, so the decoder cannot emit broken Korean.
    Index zero is reserved for the CTC blank token.
    """

    characters: str

    @classmethod
    def standard(cls) -> "BCoreKoreanCodec":
        ordered = dict.fromkeys(ASCII + KOREAN_JAMO + HANGUL_SYLLABLES + DOCUMENT_SYMBOLS)
        return cls("".join(ordered))

    @property
    def size(self) -> int:
        return len(self.characters) + 1

    def encode(self, text: str) -> list[int]:
        lookup = {character: index + 1 for index, character in enumerate(self.characters)}
        missing = sorted({character for character in text if character not in lookup})
        if missing:
            raise ValueError(f"BCORE_OCR_UNSUPPORTED_CHARACTERS:{''.join(missing)}")
        return [lookup[character] for character in text]

    def supports(self, text: str) -> bool:
        allowed = set(self.characters)
        return all(character in allowed for character in text)

    def decode_ctc(self, token_ids: list[int]) -> str:
        output: list[str] = []
        previous = -1
        for token_id in token_ids:
            if token_id != 0 and token_id != previous:
                if token_id < 0 or token_id > len(self.characters):
                    raise ValueError(f"BCORE_OCR_TOKEN_OUT_OF_RANGE:{token_id}")
                output.append(self.characters[token_id - 1])
            previous = token_id
        return "".join(output)

    def save(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            json.dumps(
                {
                    "schema": "B_CORE_OCR_CODEC_1",
                    "blankTokenId": 0,
                    "unit": "unicode_character",
                    "characters": self.characters,
                },
                ensure_ascii=False,
                separators=(",", ":"),
            ),
            encoding="utf-8",
        )

    @classmethod
    def load(cls, path: Path) -> "BCoreKoreanCodec":
        payload = json.loads(path.read_text(encoding="utf-8"))
        if payload.get("schema") != "B_CORE_OCR_CODEC_1" or payload.get("blankTokenId") != 0:
            raise ValueError("BCORE_OCR_CODEC_INVALID")
        return cls(str(payload["characters"]))
