"""Fail-closed Korean naturalness verification for RealizationTrace surfaces.

This module does not generate text and does not use semantic inverse as a
naturalness proxy.  It combines four independent evidence layers:

* deterministic Korean morphology checks;
* normalized construction compatibility learned from trusted traces;
* normalized construction attestation from an external Korean corpus index;
* a human calibration certificate for the automatic gates.

Absent evidence is UNKNOWN.  Only an explicit contradiction is FAIL.
"""
from __future__ import annotations

import hashlib
import json
import re
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

from realization_trace_execution_substrate import ending, trace


PARTICLE_PAIRS = {
    "은": (True, False), "는": (False, False),
    "이": (True, False), "가": (False, False),
    "을": (True, False), "를": (False, False),
    "으로": (True, True), "로": (False, True),
}
PARTICLE_RE = re.compile(r"([가-힣A-Za-z0-9]+?)(으로|은|는|이|가|을|를|로)(?=\s|[,.?!]|$)")
MALFORMED_MORPHOLOGY = tuple(
    re.compile(pattern)
    for pattern in (
        r"하시세요", r"하십세요", r"해\s*주십세요", r"해\s*주시세요",
        r"(?:요요|습니다요|습니까요|입니까요|십시오요|세요요)(?=[.?!]|$)",
        r"\s+[,.?!]", r"[.?!]{2,}",
    )
)


def digest(value: object) -> str:
    return hashlib.sha256(
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def final_hangul_jongseong(text: str) -> int | None:
    for char in reversed(text):
        code = ord(char)
        if 0xAC00 <= code <= 0xD7A3:
            return (code - 0xAC00) % 28
    return None


def morphology_verdict(surface: str) -> dict:
    reasons: list[str] = []
    for match in PARTICLE_RE.finditer(surface):
        stem, particle = match.groups()
        jong = final_hangul_jongseong(stem)
        if jong is None:
            continue
        requires_jong, rieul_exception = PARTICLE_PAIRS[particle]
        has_jong = jong != 0
        expected_jong_form = has_jong and not (rieul_exception and jong == 8)
        if requires_jong != expected_jong_form:
            reasons.append(f"PARTICLE_ALLOMORPH:{stem}:{particle}")
    for pattern in MALFORMED_MORPHOLOGY:
        if pattern.search(surface):
            reasons.append(f"MALFORMED_MORPHOLOGY:{pattern.pattern}")
    return {
        "gate": "MORPHOLOGY_VALIDITY",
        "verdict": "FAIL" if reasons else "PASS",
        "reasons": sorted(set(reasons)),
    }


def _terminal(surface: str) -> str | None:
    return next((char for char in reversed(surface.strip()) if char in ".?!"), None)


NORMALIZED_ENDING_PATTERNS: tuple[tuple[str, re.Pattern[str]], ...] = (
    ("FORMAL_INTERROGATIVE", re.compile(r"(?:습니까|입니까)[?]\s*$")),
    ("FORMAL_DIRECTIVE", re.compile(r"(?:십시오|세요)[.?!]?\s*$")),
    ("FORMAL_DECLARATIVE", re.compile(r"(?:습니다|ㅂ니다|입니다|됩니다|합니다)[.?!]?\s*$")),
    ("POLITE_INTERROGATIVE", re.compile(r"(?:어요|아요|해요|예요|이에요)[?]\s*$")),
    ("POLITE_DECLARATIVE", re.compile(r"(?:어요|아요|해요|예요|이에요)[.?!]?\s*$")),
    ("CASUAL_DELIBERATIVE", re.compile(r"[가-힣]+까\s*(?:봐|싶어|해)?[.?!]?\s*$")),
    ("CASUAL_PREDICTIVE", re.compile(r"[가-힣]+걸[.?!]?\s*$")),
    ("CASUAL_EXPLANATORY", re.compile(r"거야[.?!]?\s*$")),
    ("CASUAL_BACKGROUND", re.compile(r"텐데[.?!]?\s*$")),
    ("CASUAL_JUSTIFICATION", re.compile(r"거든[.?!]?\s*$")),
    ("CASUAL_SHARED_GROUND", re.compile(r"잖아[.?!]?\s*$")),
    ("CASUAL_RECALL", re.compile(r"더라고[.?!]?\s*$")),
    ("CASUAL_DISCOVERY", re.compile(r"(?:네|보네)[.?!]?\s*$")),
    ("CASUAL_SUGGESTIVE", re.compile(r"(?:^|\s)[가-힣]*봐[.?!]?\s*$")),
    ("CASUAL_PROPOSAL", re.compile(r"보자[.?!]?\s*$")),
    ("CASUAL_COPULAR", re.compile(r"(?:^|\s)[가-힣]*(?:이야|아니야)[.?!]?\s*$")),
    ("CASUAL_EVIDENTIAL", re.compile(r"(?:^|\s)[가-힣]*더라[.?!]?\s*$")),
    ("CASUAL_OPEN_QUESTION", re.compile(r"(?:^|\s)[가-힣]*나[?]\s*$")),
    ("CASUAL_CONTEXT_FRAGMENT", re.compile(r"(?:^|\s)[가-힣]*(?:는데|인데|싶어서)[.?!]?\s*$")),
    ("CASUAL_FINITE", re.compile(r"(?:^|\s)[가-힣]*(?:아|어|여|해|돼|져|쳐|춰|려|펴|켜|떼|혀|러워|보여|들어|같아|있어|없어|않아|좋아|어려워|부끄러워|두려워|그래|나)[.?!]?\s*$")),
)


def normalized_ending_family(surface: str, realization: dict | None = None) -> str | None:
    """Return a bounded morphology family, never a semantic speech-act label."""
    for family, pattern in NORMALIZED_ENDING_PATTERNS:
        if pattern.search(surface):
            return family
    if realization is not None:
        legacy = ending(realization)
        if legacy:
            return f"LEGACY::{legacy}"
    return None


def construction_signature(ir: dict, realization: dict, surface: str) -> dict:
    clause_count = len(realization.get("clause_structure", []))
    return {
        "speech_act": ir.get("speech_act"),
        "operation": ir.get("operation"),
        "discourse_relation": ir.get("discourse_relation"),
        "clause_count_bucket": "3+" if clause_count >= 3 else str(clause_count),
        "final_ending": normalized_ending_family(surface, realization),
        "terminal": _terminal(surface),
        "slot_roles": sorted({item["semantic_role"] for item in realization.get("slot_realizations", [])}),
        "honorific": bool(realization.get("honorific_morphology")),
        "connective_present": bool(realization.get("connectives")),
    }


def signature_key(signature: dict) -> str:
    return digest(signature)


def corpus_attestation_signature(signature: dict) -> dict:
    """Project a semantic construction signature onto corpus-observable form.

    A raw corpus cannot safely supply speech act, canonical role, operation, or
    discourse authority.  It can attest only morphology and surface structure.
    Keeping this projection explicit prevents corpus text from becoming a
    semantic classifier.
    """
    return {
        "clause_count_bucket": signature.get("clause_count_bucket"),
        "final_ending": signature.get("final_ending"),
        "terminal": signature.get("terminal"),
        "honorific": bool(signature.get("honorific")),
        "connective_present": bool(signature.get("connective_present")),
    }


class ConstructionEvidenceIndex:
    """Normalized cross-family evidence; no sentence strings are retained."""

    def __init__(self, minimum_families: int = 2):
        self.minimum_families = minimum_families
        self._families: dict[str, set[str]] = defaultdict(set)
        self._signatures: dict[str, dict] = {}
        self._ending_by_act: dict[str, dict[str, set[str]]] = defaultdict(lambda: defaultdict(set))

    def add(self, family_id: str, ir: dict, realization: dict, surface: str) -> None:
        signature = construction_signature(ir, realization, surface)
        key = signature_key(signature)
        self._signatures[key] = signature
        self._families[key].add(family_id)
        if signature["final_ending"]:
            self._ending_by_act[signature["final_ending"]][signature["speech_act"]].add(family_id)

    def evaluate(self, ir: dict, realization: dict, surface: str) -> dict:
        signature = construction_signature(ir, realization, surface)
        if re.search(r"\b([가-힣]+)(?:\s+\1)+\b", surface) and any(
            f"{word} {word}" in surface for word in ("그리고", "그러나", "그런데", "그래서", "따라서", "하지만", "다만")
        ):
            return {
                "gate": "CONSTRUCTION_COMPATIBILITY", "verdict": "FAIL",
                "reasons": ["DUPLICATED_CONNECTIVE"],
                "normalized_signature": signature, "supporting_family_count": 0,
            }
        key = signature_key(signature)
        families = self._families.get(key, set())
        if len(families) >= self.minimum_families:
            return {
                "gate": "CONSTRUCTION_COMPATIBILITY", "verdict": "PASS",
                "reasons": [], "normalized_signature": signature,
                "supporting_family_count": len(families),
            }
        current_ending = signature["final_ending"]
        act = signature["speech_act"]
        ending_evidence = self._ending_by_act.get(current_ending, {})
        same_act = len(ending_evidence.get(act, set()))
        other_act = sum(len(value) for name, value in ending_evidence.items() if name != act)
        if current_ending and same_act == 0 and other_act >= self.minimum_families:
            return {
                "gate": "CONSTRUCTION_COMPATIBILITY", "verdict": "FAIL",
                "reasons": ["ENDING_SPEECH_ACT_CONTRADICTION"],
                "normalized_signature": signature, "supporting_family_count": 0,
                "contradicting_family_count": other_act,
            }
        return {
            "gate": "CONSTRUCTION_COMPATIBILITY", "verdict": "UNKNOWN",
            "reasons": ["INSUFFICIENT_CROSS_FAMILY_CONSTRUCTION_EVIDENCE"],
            "normalized_signature": signature, "supporting_family_count": len(families),
        }

    def artifact(self) -> dict:
        rows = [
            {"signature_sha256": key, "signature": self._signatures[key], "family_count": len(families)}
            for key, families in sorted(self._families.items())
        ]
        return {
            "schema": "BCORE.CONSTRUCTION_COMPATIBILITY_INDEX.V1",
            "raw_sentences_stored": False,
            "minimum_families": self.minimum_families,
            "pattern_count": len(rows),
            "recurrent_pattern_count": sum(row["family_count"] >= self.minimum_families for row in rows),
            "patterns": rows,
        }


class CorpusAttestationIndex:
    """External corpus pattern evidence. Empty is a valid fail-closed state."""

    def __init__(self, payload: dict | None = None):
        payload = payload or {}
        self.authoritative_sources = payload.get("authoritative_sources", [])
        self.patterns = payload.get("patterns", {})
        self.minimum_occurrences = int(payload.get("minimum_occurrences", 2))

    def evaluate(self, signature: dict) -> dict:
        if not self.authoritative_sources:
            return {
                "gate": "CORPUS_ATTESTATION", "verdict": "UNKNOWN",
                "reasons": ["NO_APPROVED_REAL_KOREAN_CORPUS_INDEX"], "occurrences": 0,
            }
        corpus_signature = corpus_attestation_signature(signature)
        if corpus_signature["final_ending"] is None:
            return {
                "gate": "CORPUS_ATTESTATION",
                "verdict": "UNKNOWN",
                "reasons": ["ENDING_MORPHOLOGY_NOT_NORMALIZED"],
                "occurrences": 0,
                "normalized_corpus_signature": corpus_signature,
            }
        evidence = self.patterns.get(signature_key(corpus_signature), 0)
        count = int(evidence.get("occurrences", 0) if isinstance(evidence, dict) else evidence)
        return {
            "gate": "CORPUS_ATTESTATION",
            "verdict": "PASS" if count >= self.minimum_occurrences else "UNKNOWN",
            "reasons": [] if count >= self.minimum_occurrences else ["PATTERN_NOT_SUFFICIENTLY_ATTESTED"],
            "occurrences": count,
            "normalized_corpus_signature": corpus_signature,
        }


@dataclass
class HumanCalibration:
    status: str = "NOT_CALIBRATED"
    benchmark_size: int = 0
    precision_lower_bound: float | None = None
    false_accept_upper_bound: float | None = None

    @classmethod
    def from_payload(cls, payload: dict | None) -> "HumanCalibration":
        payload = payload or {}
        return cls(
            status=payload.get("status", "NOT_CALIBRATED"),
            benchmark_size=int(payload.get("benchmark_size", 0)),
            precision_lower_bound=payload.get("precision_lower_bound"),
            false_accept_upper_bound=payload.get("false_accept_upper_bound"),
        )

    def evaluate(self) -> dict:
        calibrated = (
            self.status == "CALIBRATED"
            and self.benchmark_size >= 20
            and self.precision_lower_bound is not None
            and self.precision_lower_bound >= 0.95
            and self.false_accept_upper_bound is not None
            and self.false_accept_upper_bound <= 0.05
        )
        return {
            "gate": "HUMAN_CALIBRATED_NATURALNESS",
            "verdict": "PASS" if calibrated else "UNKNOWN",
            "reasons": [] if calibrated else ["HUMAN_CALIBRATION_CERTIFICATE_UNAVAILABLE"],
            "status": self.status, "benchmark_size": self.benchmark_size,
        }


class NaturalnessVerifier:
    def __init__(self, constructions: ConstructionEvidenceIndex,
                 corpus: CorpusAttestationIndex, human: HumanCalibration):
        self.constructions = constructions
        self.corpus = corpus
        self.human = human

    def verify(self, surface: str, ir: dict) -> dict:
        realization = trace(surface, ir)
        morphology = morphology_verdict(surface)
        construction = self.constructions.evaluate(ir, realization, surface)
        corpus = self.corpus.evaluate(construction["normalized_signature"])
        human = self.human.evaluate()
        layers = [morphology, construction, corpus, human]
        if any(layer["verdict"] == "FAIL" for layer in layers):
            verdict = "FAIL"
        elif all(layer["verdict"] == "PASS" for layer in layers):
            verdict = "PASS"
        else:
            verdict = "UNKNOWN"
        return {
            "verdict": verdict,
            "layers": layers,
            "surface_sha256": hashlib.sha256(surface.encode()).hexdigest(),
            "canonical_hash": realization["canonical_hash"],
        }


def build_construction_index(substrate: dict, equivalence_classes: dict) -> ConstructionEvidenceIndex:
    index = ConstructionEvidenceIndex()
    class_by_family = {row["meaning_family_id"]: row for row in equivalence_classes["classes"]}
    traces = {row["surface_sha256"]: row for row in substrate["traces"]}
    for family_id, item in class_by_family.items():
        ir = item["approved_response_ir"]
        for surface in item["surfaces"]:
            realization = traces.get(surface["surface_sha256"])
            if realization is not None:
                index.add(family_id, ir, realization, surface["surface"])
    return index


def load_optional(path: Path | None) -> dict:
    if path is None or not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))
