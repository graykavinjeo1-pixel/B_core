"""Build 17-way Korean persona-parallel surface gold from structured meaning.

This is an offline *data* compiler.  It never treats generated text as
knowledge or as a runtime phrase store.  Every output family has exactly one
structured meaning and seventeen persona realizations, making persona effects
identifiable during later canonical construction learning.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


PERSONAS = [
    ("INTJ", "FORMAL", "PEER", "BALANCED", "요점을 정리하면, ", "입니다."),
    ("INTP", "INFORMAL", "PEER", "BALANCED", "정리해 보면, ", "야."),
    ("ENTJ", "INFORMAL", "PEER", "DIRECT", "핵심만 말하면, ", "야."),
    ("ENTP", "INFORMAL", "PEER", "LIVELY", "오, 참고로, ", "거든."),
    ("INFJ", "INFORMAL", "PEER", "GENTLE", "조심스럽게 말씀드리면, ", "요."),
    ("INFP", "INFORMAL", "PEER", "GENTLE", "혹시 도움이 될까 해서 말씀드리면, ", "요."),
    ("ENFJ", "POLITE", "PEER", "GENTLE", "함께 확인해 보면, ", "요."),
    ("ENFP", "INFORMAL", "PEER", "LIVELY", "좋은 소식부터 말하면, ", "요!"),
    ("ISTJ", "FORMAL", "PEER", "BALANCED", "확인된 기준으로, ", "습니다."),
    ("ISFJ", "POLITE", "PEER", "GENTLE", "미리 안내드리면, ", "요."),
    ("ESTJ", "INFORMAL", "PEER", "DIRECT", "바로 안내할게, ", "야."),
    ("ESFJ", "POLITE", "PEER", "GENTLE", "챙겨서 말씀드리면, ", "요."),
    ("ISTP", "INFORMAL", "PEER", "DIRECT", "확인했어. ", "야."),
    ("ISFP", "INFORMAL", "PEER", "GENTLE", "살펴보니, ", "요."),
    ("ESTP", "INFORMAL", "PEER", "LIVELY", "바로 말하면, ", "야."),
    ("ESFP", "POLITE", "PEER", "LIVELY", "알기 쉽게 말하면, ", "요!"),
    ("COARSE_ELDER", "INFORMAL", "CLOSE", "DIRECT", "아이구, 들어 봐. ", "다."),
]

STATE = {
    "SCHEDULED": "예정돼 있",
    "CANCELLED": "취소됐",
    "RECEIVED": "받았",
    "SENT": "보냈",
    "DELAYED": "지연됐",
    "MOVED": "옮겨졌",
    "READY": "준비됐",
    "REMINDER": "일정이 잡혀 있",
    "AVAILABLE": "사용할 수 있",
    "STARTED": "시작됐",
    "UPDATED": "갱신됐",
    "CONFIRMED": "확정됐",
    "PAYMENT_COMPLETE": "결제가 끝났",
}
ACTION = {"CONTACT": "다시 연락", "CHECK": "확인", "ENABLE": "사용 가능하게 설정", "REMIND": "알림"}
PROPERTY = {"ARRIVAL": "도착 여부", "VACANCY": "빈자리 여부", "OPEN": "운영 여부", "COMPLETION": "완료 여부", "USAGE": "사용 방법"}


def digest(value: object) -> str:
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def subject(m: dict) -> str:
    return m.get("event") or m.get("theme") or m.get("feature") or m.get("place") or m.get("meal") or m.get("action") or "날씨"


def ending(stem: str, persona: str) -> str:
    # Stems are deliberately semantic-neutral Korean predicate stems. This is
    # a corpus constructor, not a production realization rule.
    if persona in {"INTJ", "ISTJ"}:
        return stem + "습니다."
    if persona == "COARSE_ELDER":
        return stem + "다."
    if persona in {"ENTJ", "ESTJ", "ISTP", "ESTP", "INTP"}:
        return stem + "어."
    # `있` combines as `있어요`; completed stems such as `됐`, `받았`,
    # and `보냈` combine as `됐어요`, `받았어요`, `보냈어요`.
    return stem + "어요."


def neutral_body(m: dict, persona: str) -> str:
    s = subject(m)
    act = m["act"]
    if act == "INFORM":
        if m.get("operation") == "REVISED" and m.get("time"):
            return ending(f"{s} 시간이 {m['time']}로 바뀌", persona)
        if m.get("weather") == "RAIN_LIKELY":
            return ending(f"{m.get('day', '오늘')} 비가 올 가능성이 있", persona)
        if m.get("delay_minutes") is not None:
            why = " 교통 상황 때문에" if m.get("reason") == "TRAFFIC" else ""
            return ending(f"{s}이{why} {m['delay_minutes']}분 정도 늦", persona)
        if m.get("place") and m.get("state") == "MOVED":
            return ending(f"{s} 장소가 {m['place']}로 바뀌", persona)
        if m.get("channel") == "EMAIL" and m.get("state") == "SENT":
            return ending(f"{s}을 이메일로 보냈", persona)
        if m.get("time") and m.get("state"):
            return ending(f"{s}은 {m['time']}에 {STATE.get(m['state'], m['state'])}", persona)
        if m.get("state"):
            return ending(f"{s}은 {STATE.get(m['state'], m['state'])}", persona)
    if act == "QUERY":
        target = PROPERTY.get(m.get("property", ""), m.get("property", "정보"))
        if m.get("meal"):
            return f"{m['meal']} {m.get('time', '')}에 먹을 수 있을까요?"
        return f"{s}의 {target}를 알 수 있을까요?"
    if act == "REQUEST":
        action = ACTION.get(m.get("action", ""), m.get("action", "처리"))
        return f"{s} {action}해 줄래요?"
    if act == "PROMISE":
        time = " ".join(x for x in [m.get("day"), m.get("time")] if x)
        return ending(f"{time}에 {ACTION.get(m.get('action', ''), m.get('action', '처리'))}하겠", persona)
    if act == "REASSURE":
        return ending(f"{m.get('action', s)}은 서두르지 않아도 괜찮", persona)
    raise ValueError(f"unknown act {act}")


def surface(m: dict, persona: tuple[str, str, str, str, str, str]) -> str:
    label, _, _, _, prefix, _ = persona
    body = neutral_body(m, label)
    if label == "COARSE_ELDER":
        # Character signal without a targeted insult or a new factual claim.
        return prefix + body.replace("요.", "다.")
    return prefix + body


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", default=r"D:\B_Core_validation\corpora\persona_synthetic_ko_v1\persona_synthetic_ko_v1.jsonl")
    parser.add_argument("--output", default=r"D:\B_Core_validation\corpora\persona_parallel_ko_v2\persona_parallel_ko_v2.jsonl")
    args = parser.parse_args()
    source, output = Path(args.source), Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    tmp = output.with_suffix(".tmp")
    rows = 0
    unique_meaning_rows = 0
    duplicate_source_rows = 0
    seen_meanings: set[str] = set()
    with source.open(encoding="utf-8") as src, tmp.open("w", encoding="utf-8", newline="\n") as dst:
        for source_index, line in enumerate(src):
            original = json.loads(line)
            meaning = original["approved_meaning"]
            meaning_sha = digest(meaning)
            if meaning_sha in seen_meanings:
                duplicate_source_rows += 1
                continue
            seen_meanings.add(meaning_sha)
            unique_meaning_rows += 1
            # Identical meanings may occur in distinct original campaign
            # instances.  Preserve that instance boundary so every training
            # family is exactly the intended 17-way parallel set.
            family_id = digest({"source_meaning_index": source_index + 1, "meaning": meaning})
            for persona_index, p in enumerate(PERSONAS):
                label, register, relationship, voice, _, _ = p
                row = {
                    "schema": "BCORE.PERSONA_PARALLEL_GOLD.V2",
                    "example_id": f"PERS-PAR-KO-V2-{source_index + 1:05d}-{persona_index + 1:02d}",
                    "parallel_family_id": family_id,
                    "source_meaning_index": source_index + 1,
                    "split": original["split"],
                    "approved_meaning": meaning,
                    "language": "KOREAN",
                    "persona": {"persona_label": label, "register": register, "relationship": relationship, "voice": voice},
                    "surface": surface(meaning, p),
                    "surface_authority": "NONAUTHORITATIVE_CONSTRUCTION_CANDIDATE; REQUIRES_CANONICAL_INVERSE",
                }
                row["record_sha256"] = digest(row)
                dst.write(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n")
                rows += 1
    tmp.replace(output)
    receipt = {
        "schema": "BCORE.PERSONA_PARALLEL_GOLD_RECEIPT.V2",
        "source_rows": source_index + 1,
        "unique_meaning_rows": unique_meaning_rows,
        "duplicate_source_rows_excluded": duplicate_source_rows,
        "persona_count": len(PERSONAS),
        "generated_rows": rows,
        "parallel_family_semantics": "ONE_STRUCTURED_MEANING_PER_FAMILY; 17_PERSONA_SURFACES_PER_SOURCE_ROW",
        "surface_authority": "NONAUTHORITATIVE_CONSTRUCTION_CANDIDATE_ONLY",
        "output_sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
    }
    receipt["receipt_sha256"] = digest(receipt)
    output.with_name("persona_parallel_ko_v2_receipt.json").write_text(json.dumps(receipt, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(receipt, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
