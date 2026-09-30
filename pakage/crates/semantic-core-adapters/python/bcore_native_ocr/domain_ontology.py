"""Conservative vertical-ontology mapping for OCR evidence.

The ontology never replaces the source transcription.  It emits a separate
canonical semantic value and a receipt explaining whether the value was an
exact visible candidate or a unique near-match suggestion.  Domain terms are
loaded from data; no apartment item or Korean phrase is hard-coded here.
"""

from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
import re
import unicodedata
from typing import Iterable


LEADING_ORDINAL = re.compile(r"^\s*(?:\d+\s*[.)]|[가-힣]\s*[.)])\s*")


def normalized_term(value: str) -> str:
    value = unicodedata.normalize("NFKC", str(value or ""))
    value = LEADING_ORDINAL.sub("", value)
    return "".join(character for character in value.casefold() if character.isalnum())


def edit_distance(left: str, right: str) -> int:
    previous = list(range(len(right) + 1))
    for left_index, left_character in enumerate(left, start=1):
        current = [left_index]
        for right_index, right_character in enumerate(right, start=1):
            current.append(
                min(
                    current[-1] + 1,
                    previous[right_index] + 1,
                    previous[right_index - 1] + (left_character != right_character),
                )
            )
        previous = current
    return previous[-1]


@dataclass(frozen=True)
class OntologyEntry:
    canonical: str
    kind: str
    identifier: str | None = None
    aliases: tuple[str, ...] = ()
    provenance: str | None = None


class DomainOntology:
    def __init__(self, entries: Iterable[OntologyEntry]):
        self.entries = tuple(entries)
        index: dict[str, list[OntologyEntry]] = {}
        for entry in self.entries:
            for surface in (entry.canonical, *entry.aliases):
                key = normalized_term(surface)
                if key:
                    index.setdefault(key, []).append(entry)
        self.index = {key: tuple(values) for key, values in index.items()}

    def resolve(
        self,
        observed: str,
        *,
        visual_candidates: Iterable[str] = (),
        allowed_kinds: set[str] | None = None,
    ) -> dict:
        surfaces = [("observed", observed), *[("visual_candidate", item) for item in visual_candidates]]
        exact = []
        for source, surface in surfaces:
            for entry in self.index.get(normalized_term(surface), ()):
                if allowed_kinds is None or entry.kind in allowed_kinds:
                    exact.append((source, surface, entry))
        unique_exact = {
            (entry.canonical, entry.kind, entry.identifier): (source, surface, entry)
            for source, surface, entry in exact
        }
        if len(unique_exact) == 1:
            source, surface, entry = next(iter(unique_exact.values()))
            return self._receipt(
                observed,
                entry,
                status="resolved",
                authority="visual_candidate" if source == "visual_candidate" else "exact_surface",
                matched_surface=surface,
                distance=0,
            )

        observed_key = normalized_term(observed)
        ranked = []
        for surface_key, entries in self.index.items():
            distance = edit_distance(observed_key, surface_key)
            allowed_distance = max(1, int(max(len(observed_key), len(surface_key)) * 0.12))
            if distance > allowed_distance:
                continue
            for entry in entries:
                if allowed_kinds is not None and entry.kind not in allowed_kinds:
                    continue
                ranked.append((distance, abs(len(observed_key) - len(surface_key)), entry.canonical, entry, surface_key))
        ranked.sort(key=lambda item: item[:3])
        if not ranked:
            return {
                "schema": "B_CORE_NATIVE_OCR_ONTOLOGY_RESOLUTION_1",
                "status": "unresolved",
                "observed": observed,
                "rawPreserved": True,
                "externalOcrInvocations": 0,
            }
        best_distance = ranked[0][0]
        best_entries = {
            (item[3].canonical, item[3].kind, item[3].identifier): item
            for item in ranked
            if item[0] == best_distance
        }
        if len(best_entries) != 1:
            return {
                "schema": "B_CORE_NATIVE_OCR_ONTOLOGY_RESOLUTION_1",
                "status": "ambiguous",
                "observed": observed,
                "rawPreserved": True,
                "externalOcrInvocations": 0,
                "candidateCanonicals": sorted({item[3].canonical for item in best_entries.values()}),
                "distance": best_distance,
            }
        item = next(iter(best_entries.values()))
        return self._receipt(
            observed,
            item[3],
            status="suggested",
            authority="unique_ontology_near_match",
            matched_surface=item[4],
            distance=item[0],
        )

    @staticmethod
    def _receipt(
        observed: str,
        entry: OntologyEntry,
        *,
        status: str,
        authority: str,
        matched_surface: str,
        distance: int,
    ) -> dict:
        return {
            "schema": "B_CORE_NATIVE_OCR_ONTOLOGY_RESOLUTION_1",
            "status": status,
            "observed": observed,
            "canonical": entry.canonical,
            "kind": entry.kind,
            "identifier": entry.identifier,
            "authority": authority,
            "matchedSurface": matched_surface,
            "editDistance": distance,
            "provenance": entry.provenance,
            "rawPreserved": True,
            "externalOcrInvocations": 0,
        }


def load_long_term_repair_ontology(
    legal_items_path: Path,
    language_terms_path: Path | None = None,
    aliases_path: Path | None = None,
) -> DomainOntology:
    legal_payload = json.loads(legal_items_path.read_text(encoding="utf-8"))
    items = legal_payload.get("items") if isinstance(legal_payload, dict) else legal_payload
    entries: list[OntologyEntry] = []
    for item in items or []:
        identifier = str(item.get("id") or item.get("number") or "") or None
        provenance = str(legal_items_path)
        for kind, key in (("group", "group"), ("category", "category"), ("work_item", "item")):
            value = str(item.get(key) or "").strip()
            if value:
                entries.append(OntologyEntry(value, kind, identifier, provenance=provenance))
        for method in item.get("methods") or []:
            value = str(method.get("method") or "").strip()
            if value:
                entries.append(OntologyEntry(value, "repair_method", identifier, provenance=provenance))
    if language_terms_path:
        language_payload = json.loads(language_terms_path.read_text(encoding="utf-8"))
        for term in language_payload.get("terms") or []:
            value = str(term).strip()
            if value:
                entries.append(
                    OntologyEntry(
                        value,
                        "domain_term",
                        provenance=str(language_terms_path),
                    )
                )
    aliases_by_key: dict[tuple[str, str], set[str]] = {}
    alias_provenance: dict[tuple[str, str], str] = {}
    if aliases_path:
        aliases_payload = json.loads(aliases_path.read_text(encoding="utf-8"))
        for item in aliases_payload.get("entries") or []:
            canonical = str(item.get("canonical") or "").strip()
            kind = str(item.get("kind") or "work_item").strip()
            aliases = {
                str(alias).strip()
                for alias in item.get("aliases") or []
                if str(alias).strip()
            }
            if canonical and aliases:
                aliases_by_key.setdefault((canonical, kind), set()).update(aliases)
                alias_provenance[(canonical, kind)] = str(aliases_path)
    # Multiple legal items may share a method or category.  Canonical surface
    # identity is enough for OCR normalization, so collapse exact duplicates.
    unique = {
        (entry.canonical, entry.kind): OntologyEntry(
            entry.canonical,
            entry.kind,
            entry.identifier,
            tuple(
                sorted(
                    set(entry.aliases)
                    | aliases_by_key.get((entry.canonical, entry.kind), set())
                )
            ),
            (
                f"{entry.provenance};{alias_provenance[(entry.canonical, entry.kind)]}"
                if (entry.canonical, entry.kind) in alias_provenance
                else entry.provenance
            ),
        )
        for entry in entries
    }
    return DomainOntology(unique.values())
