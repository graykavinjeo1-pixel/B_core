//! Source-attributed bilingual lexical knowledge, not promoted world facts.
//! Shared immutable data + indexed, bounded lookup. No text-to-action authority.
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const PACK_SHA256: &str = "d614048f9df99ac9104cf0206ace8cb9238f0f6a7490bfb06980d6fbf69d23c8";
pub const PACK_SCHEMA: &str = "B_CORE_BILINGUAL_LEXICAL_LOOKUP_5";
const DATA: &str = include_str!("../data/lexical-knowledge/nikl-ko-en.jsonl");
const ENGLISH_STRESS_DATA: &str =
    include_str!("../data/lexical-knowledge/english-final-stress.json");
pub const ENGLISH_STRESS_SHA256: &str =
    "34ff6891bb70f9bbf66d2ea05fd7e6df8dd6e9aaf721866d9b2b5ff7c3ef066c";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BilingualSenseIR {
    pub source_sense_id: String,
    pub english: String,
    pub definition_ko: String,
    pub definition_en: String,
    pub grammar: BTreeMap<String, String>,
    #[serde(default)]
    pub frames: Vec<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BilingualLexicalEntryIR {
    pub source_entry_id: String,
    pub lemma: String,
    pub pos: String,
    pub level: String,
    pub domain: String,
    pub attributes: BTreeMap<String, String>,
    pub forms: Vec<BTreeMap<String, String>>,
    pub senses: Vec<BilingualSenseIR>,
    pub selection_evidence: Vec<String>,
}

impl BilingualLexicalEntryIR {
    pub fn working_lexemes(&self) -> Vec<crate::lexical_memory::LexemeIR> {
        use crate::language_knowledge::LanguageCodeIR;
        use crate::lexical_memory::{LexemeIR, PartOfSpeechIR, SenseIR, LEXEME_SCHEMA};
        let pos = match self.pos.as_str() {
            "동사" | "보조 동사" => PartOfSpeechIR::Verb,
            "형용사" | "보조 형용사" => PartOfSpeechIR::Adjective,
            "부사" => PartOfSpeechIR::Adverb,
            "대명사" => PartOfSpeechIR::Pronoun,
            "조사" => PartOfSpeechIR::Particle,
            "감탄사" => PartOfSpeechIR::Interjection,
            "관형사" => PartOfSpeechIR::Determiner,
            "품사 없음" | "어미" | "접사" => PartOfSpeechIR::Phrase,
            _ => PartOfSpeechIR::Noun,
        };
        self.senses
            .iter()
            .flat_map(|sense| {
                [LanguageCodeIR::Korean, LanguageCodeIR::English]
                    .into_iter()
                    .map(move |language| {
                        let korean = language == LanguageCodeIR::Korean;
                        let aliases = sense
                            .english
                            .split(';')
                            .map(str::trim)
                            .filter(|a| !a.is_empty())
                            .map(str::to_string)
                            .collect::<Vec<_>>();
                        LexemeIR {
                            schema: LEXEME_SCHEMA.into(),
                            lexeme_id: format!(
                                "NIKL.{}.{}.{}",
                                if korean { "ko" } else { "en" },
                                self.source_entry_id,
                                sense.source_sense_id
                            ),
                            language,
                            lemma: if korean {
                                self.lemma.clone()
                            } else {
                                aliases[0].clone()
                            },
                            inflected_forms: if korean {
                                self.forms
                                    .iter()
                                    .filter_map(|f| f.get("writtenForm").cloned())
                                    .take(128)
                                    .collect()
                            } else {
                                aliases.into_iter().skip(1).take(128).collect()
                            },
                            part_of_speech: pos,
                            grammatical_roles: vec![],
                            senses: vec![SenseIR {
                                sense_id: format!(
                                    "NIKL.{}.{}",
                                    self.source_entry_id, sense.source_sense_id
                                ),
                                canonical_concept: self.concept_id(sense),
                                gloss: if korean {
                                    sense.definition_ko.clone()
                                } else {
                                    sense.definition_en.clone()
                                },
                                semantic_tags: vec!["LEXICAL_DEFINITION_ONLY".into()],
                                context_selectors: vec![],
                                relations: vec![],
                                intent_hint: None,
                                confidence_millis: 650,
                            }],
                            collocations: vec![],
                            domains: vec![self.domain.clone()],
                            source: format!(
                                "{} | 국립국어원 | CC BY-SA 2.0 KR | {}",
                                self.source_url(),
                                PACK_SHA256
                            ),
                            confidence_millis: 650,
                            frequency_prior: 0,
                        }
                    })
            })
            .collect()
    }
    pub fn concept_id(&self, sense: &BilingualSenseIR) -> String {
        format!(
            "C_LEX_NIKL_{}_{}",
            self.source_entry_id, sense.source_sense_id
        )
    }
    pub fn source_url(&self) -> String {
        format!(
            "https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo={}",
            self.source_entry_id
        )
    }
    pub fn validate(&self) -> bool {
        !self.source_entry_id.is_empty()
            && self.source_entry_id.bytes().all(|c| c.is_ascii_digit())
            && !self.lemma.trim().is_empty()
            && self.lemma.len() <= 160
            && !self.senses.is_empty()
            && self.senses.len() <= 128
            && matches!(
                self.domain.as_str(),
                "GENERAL" | "LAW_ECONOMICS_RELATED" | "GRAMMAR"
            )
            && self.senses.iter().all(|s| {
                !s.source_sense_id.is_empty()
                    && s.source_sense_id.bytes().all(|c| c.is_ascii_digit())
                    && !s.english.trim().is_empty()
                    && !s.definition_ko.trim().is_empty()
                    && !s.definition_en.trim().is_empty()
                    && !s.english.to_lowercase().contains("no equivalent")
            })
            && self
                .senses
                .iter()
                .map(|s| &s.source_sense_id)
                .collect::<BTreeSet<_>>()
                .len()
                == self.senses.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexicalMorphologyIR {
    pub base: String,
    pub ending: String,
    pub grammar_rule: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexicalKnowledgeMatchIR {
    pub entry: BilingualLexicalEntryIR,
    pub matched_form: String,
    pub morphology: LexicalMorphologyIR,
    /// Polysemy remains a set of candidates; frequency does not select truth.
    pub concept_ids: Vec<String>,
}

/// A bounded reading of an attributed dictionary definition, not world truth.
/// Topic categories (e.g. housing) are deliberately not entity types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NominalReferentKindIR {
    Person,
    Place,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NominalReferentEvidenceIR {
    pub pack_sha256: String,
    pub surface: String,
    pub kind: NominalReferentKindIR,
    pub definition_heads: BTreeMap<String, String>,
}

fn definition_referent_head(definition: &str) -> Option<(&str, NominalReferentKindIR)> {
    // Korean nominal dictionary definitions end in their genus. A noun
    // occurring inside a modifier is not evidence for the referent's type.
    let head = definition
        .trim()
        .trim_end_matches('.')
        .split_whitespace()
        .next_back()?;
    let kind = match head {
        "사람" => NominalReferentKindIR::Person,
        "곳" | "장소" | "공간" => NominalReferentKindIR::Place,
        _ => return None,
    };
    Some((head, kind))
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexicalKnowledgeLookupIR {
    pub schema: String,
    pub source_sha256: String,
    pub pack_sha256: String,
    /// Auxiliary pronunciation/inflection knowledge, not semantic concept data.
    #[serde(default)]
    pub english_stress_lexicon_sha256: String,
    pub matches: Vec<LexicalKnowledgeMatchIR>,
    pub unmatched_tokens: Vec<String>,
    pub truncated: bool,
    pub index_probes: usize,
    pub full_catalog_scans: usize,
    pub semantic_authority: bool,
    pub execution_authority: bool,
    pub attribution: String,
}
impl LexicalKnowledgeLookupIR {
    pub fn validate_source(&self, source: &str) -> bool {
        self == &builtin_pack().lookup(source)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexicalKnowledgePackStatisticsIR {
    pub general_unique_lemmas: usize,
    pub law_economics_related_unique_lemmas: usize,
    pub supplementary_grammar_entries: usize,
    pub source_entries: usize,
    pub bilingual_senses: usize,
    pub indexed_forms: usize,
    pub pack_sha256: String,
}

#[derive(Debug, Clone)]
struct Binding {
    entry: usize,
    base: String,
    ending: String,
    rule: &'static str,
}
pub struct LexicalKnowledgePack {
    sha256: String,
    entries: Vec<BilingualLexicalEntryIR>,
    entry_index: HashMap<String, usize>,
    /// Checked once while loading the immutable source pack. Request-local
    /// lexical memories may reuse these facets without revalidating identical
    /// generated records on every shard.
    sealed_working_lexemes: HashMap<String, Vec<crate::lexical_memory::LexemeIR>>,
    forms: HashMap<String, Vec<Binding>>,
    /// The current lookup contract probes at most eight whitespace tokens.
    /// This index preserves that exact bound while avoiding impossible phrase
    /// probes for source forms that cannot start at a given token.
    form_max_tokens_by_first: HashMap<String, usize>,
}
pub fn builtin_pack() -> &'static LexicalKnowledgePack {
    static PACK: OnceLock<LexicalKnowledgePack> = OnceLock::new();
    PACK.get_or_init(|| {
        LexicalKnowledgePack::from_jsonl(DATA, PACK_SHA256).expect("sealed bilingual pack")
    })
}

/// Validate and load the immutable lexical pack before the host accepts work.
///
/// The pack is sealed and read-only. Preloading moves one-time JSON parsing and
/// form-index construction out of a user's first-turn latency without changing
/// lookup semantics or admitting unverified lexical data.
pub fn preload_builtin_pack() {
    let _ = builtin_pack();
}
impl LexicalKnowledgePack {
    pub fn from_jsonl(data: &str, expected_sha256: &str) -> Result<Self, String> {
        if digest(data) != expected_sha256 {
            return Err("LEXICAL_PACK_HASH_MISMATCH".into());
        }
        let mut pack = Self {
            sha256: expected_sha256.into(),
            entries: vec![],
            entry_index: HashMap::new(),
            sealed_working_lexemes: HashMap::new(),
            forms: HashMap::new(),
            form_max_tokens_by_first: HashMap::new(),
        };
        for line in data.lines().filter(|l| !l.trim().is_empty()) {
            let entry: BilingualLexicalEntryIR =
                serde_json::from_str(line).map_err(|_| "LEXICAL_PACK_JSON")?;
            if !entry.validate() || pack.entry_index.contains_key(&entry.source_entry_id) {
                return Err(format!("LEXICAL_ENTRY_INVALID:{}", entry.source_entry_id));
            }
            pack.entry_index
                .insert(entry.source_entry_id.clone(), pack.entries.len());
            pack.entries.push(entry);
        }
        for i in 0..pack.entries.len() {
            pack.index_entry(i);
        }
        for entry in &pack.entries {
            let facets = entry.working_lexemes();
            if facets
                .iter()
                .any(|facet| crate::lexical_memory::validate_lexeme_structure(facet).is_err())
            {
                return Err(format!(
                    "LEXICAL_WORKING_FACET_INVALID:{}",
                    entry.source_entry_id
                ));
            }
            pack.sealed_working_lexemes
                .insert(entry.source_entry_id.clone(), facets);
        }
        Ok(pack)
    }
    pub(crate) fn sealed_working_lexemes(
        &self,
        entry_id: &str,
    ) -> Option<&[crate::lexical_memory::LexemeIR]> {
        self.sealed_working_lexemes.get(entry_id).map(Vec::as_slice)
    }
    pub fn entry(&self, id: &str) -> Option<&BilingualLexicalEntryIR> {
        self.entry_index.get(id).map(|i| &self.entries[*i])
    }
    pub fn entry_ids(&self) -> impl Iterator<Item = &str> {
        self.entry_index.keys().map(String::as_str)
    }
    /// A Korean lookup obtains the already source-linked English expressions in
    /// the SAME atomic entry. No guessed translation or monolingual promotion.
    pub fn bilingual_entry(&self, id: &str) -> Option<BilingualLexicalEntryIR> {
        self.entry(id).cloned()
    }
    fn add_form(&mut self, i: usize, base: &str, ending: &str, rule: &'static str) {
        let form = normalize(&format!("{base}{ending}"));
        if form.is_empty() || form.len() > 256 {
            return;
        }
        let phrase_tokens = form
            .split(|c: char| !(c.is_alphanumeric() || matches!(c, '-' | '\'')))
            .filter(|token| !token.is_empty())
            .collect::<Vec<_>>();
        let first_phrase_token = phrase_tokens.first().map(|token| (*token).to_string());
        let phrase_token_count = phrase_tokens.len().min(8);
        let bindings = self.forms.entry(form).or_default();
        if bindings.iter().any(|b| b.entry == i) {
            return;
        }
        bindings.push(Binding {
            entry: i,
            base: base.into(),
            ending: ending.into(),
            rule,
        });
        if let Some(first) = first_phrase_token {
            self.form_max_tokens_by_first
                .entry(first)
                .and_modify(|existing| *existing = (*existing).max(phrase_token_count))
                .or_insert(phrase_token_count);
        }
    }
    fn index_entry(&mut self, i: usize) {
        let e = self.entries[i].clone();
        self.add_form(i, &e.lemma, "", "LEXICAL_LEMMA");
        for sense in &e.senses {
            for alias in sense
                .english
                .split(';')
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                self.add_form(i, alias, "", "SOURCE_ENGLISH_EQUIVALENT");
                // Finite irregular forms are lexical grammar, not event effects.
                // Keep the source lemma/senses; do not create new concepts.
                if e.pos == "동사" {
                    if let Some(past) = english_irregular_past(alias) {
                        self.add_form(i, past, "", "EN_IRREGULAR_PAST");
                    }
                    if let Some(participle) = english_participle(alias) {
                        self.add_form(i, participle, "", "EN_PAST_PARTICIPLE");
                    }
                    if let Some(past) = english_regular_past(alias) {
                        self.add_form(i, &past, "", "EN_REGULAR_PAST");
                    }
                }
            }
        }
        for form in &e.forms {
            if let Some(form) = form.get("writtenForm") {
                self.add_form(i, form, "", "SOURCE_KOREAN_PRINCIPAL_FORM");
            }
        }
        if !matches!(
            e.pos.as_str(),
            "동사" | "형용사" | "보조 동사" | "보조 형용사"
        ) {
            return;
        }
        let Some(stem) = e.lemma.strip_suffix('다') else {
            return;
        };
        for ending in [
            "다",
            "고",
            "지만",
            "지",
            "긴",
            "기",
            "네",
            "네요",
            "거든",
            "거든요",
            "잖아",
            "잖아요",
            "더라고",
            "더라고요",
        ] {
            self.add_form(i, stem, ending, "KO_STEM_SENTENTIAL_ENDING");
        }
        if matches!(e.pos.as_str(), "형용사" | "보조 형용사") {
            self.add_form(i, stem, "다고", "KO_STATIVE_QUOTATIVE");
        }
        if matches!(e.pos.as_str(), "동사" | "보조 동사") {
            self.add_form(i, stem, "자", "KO_PROPOSITIVE");
            self.add_form(i, stem, "는", "KO_ADNOMINAL_PRESENT");
            if let Some(adnominal) = add_final(stem, 4) {
                self.add_form(i, &adnominal, "", "KO_ADNOMINAL_PAST");
            } else {
                self.add_form(i, stem, "은", "KO_ADNOMINAL_PAST");
            }
        }
        for form in e.forms.iter().filter_map(|f| f.get("writtenForm")) {
            let mut connected = vec![form.clone()];
            if let Some(prefix) = form.strip_suffix("하여") {
                connected.push(format!("{prefix}해"));
            }
            if let Some(contracted) = contract_connective(form, stem) {
                connected.push(contracted);
            }
            for connected in connected {
                if is_connective_principal_form(&connected) {
                    self.add_form(i, &connected, "", "KO_CONNECTIVE_CONTRACTION");
                    self.add_form(i, &connected, "요", "KO_CONNECTIVE_POLITE");
                    self.add_form(i, &connected, "서", "KO_CONNECTIVE_CAUSE_OR_SEQUENCE");
                    if let Some(past) = add_final(&connected, 20) {
                        for ending in [
                            "어",
                            "어요",
                            "다",
                            "지",
                            "죠",
                            "을까",
                            "을까요",
                            "네",
                            "네요",
                            "거든",
                            "거든요",
                            "잖아",
                            "잖아요",
                            "더라고",
                            "더라고요",
                            "는데",
                            "지만",
                            "고",
                        ] {
                            self.add_form(i, &past, ending, "KO_PRINCIPAL_FORM_PAST_ENDING");
                        }
                    }
                }
            }
            if let Some(prospective) = form.strip_suffix('니') {
                // The dictionary supplies irregular -(으) bases (듣다 -> 들으니).
                for ending in ["려나", "려나요", "려고", "려면", "면"] {
                    self.add_form(i, prospective, ending, "KO_PRINCIPAL_EU_MODAL_ENDING");
                }
                let modal = if prospective.ends_with('으') {
                    Some(format!("{}을", prospective.strip_suffix('으').unwrap()))
                } else {
                    add_final(prospective, 8)
                };
                if let Some(modal) = modal {
                    for ending in ["래", "래요", "까", "까요", "지"] {
                        self.add_form(i, &modal, ending, "KO_PROSPECTIVE_ENDING");
                    }
                }
            }
        }
    }
    /// Use the same sense IDs for Korean lemmas and English aliases. Every
    /// admissible nominal sense must agree; unknown/polysemous types abstain.
    #[cfg(test)]
    pub(crate) fn nominal_referent_evidence(
        &self,
        surface: &str,
    ) -> Option<NominalReferentEvidenceIR> {
        self.nominal_referent_evidence_with_constraint(surface, None)
    }

    /// A typed question/reference supplies a sense constraint, not a new
    /// lexical definition. Only existing compatible senses may satisfy it.
    pub(crate) fn nominal_referent_evidence_for(
        &self,
        surface: &str,
        required: NominalReferentKindIR,
    ) -> Option<NominalReferentEvidenceIR> {
        self.nominal_referent_evidence_with_constraint(surface, Some(required))
    }

    fn nominal_referent_evidence_with_constraint(
        &self,
        surface: &str,
        required: Option<NominalReferentKindIR>,
    ) -> Option<NominalReferentEvidenceIR> {
        let surface = normalize(surface);
        let lookup = self.lookup(&surface);
        if lookup.truncated || !lookup.unmatched_tokens.is_empty() {
            return None;
        }
        let mut kind = None;
        let mut definition_heads = BTreeMap::new();
        for m in lookup.matches.iter().filter(|m| {
            m.matched_form == surface
                && m.entry.pos == "명사"
                && matches!(
                    m.morphology.grammar_rule.as_str(),
                    "LEXICAL_LEMMA" | "SOURCE_ENGLISH_EQUIVALENT"
                )
        }) {
            for sense in m
                .entry
                .senses
                .iter()
                .filter(|s| m.concept_ids.contains(&m.entry.concept_id(s)))
            {
                let Some((head, candidate)) = definition_referent_head(&sense.definition_ko) else {
                    if required.is_some() {
                        continue;
                    }
                    return None;
                };
                if required.is_some_and(|r| r != candidate) {
                    continue;
                }
                if kind.is_some_and(|prior| prior != candidate) {
                    return None;
                }
                kind = Some(candidate);
                definition_heads.insert(m.entry.concept_id(sense), head.to_string());
            }
        }
        Some(NominalReferentEvidenceIR {
            pack_sha256: self.sha256.clone(),
            surface,
            kind: kind?,
            definition_heads,
        })
    }

    pub fn lookup(&self, text: &str) -> LexicalKnowledgeLookupIR {
        let normalized = normalize(text);
        let mut tokens = normalized
            .split(|c: char| !(c.is_alphanumeric() || matches!(c, '-' | '\'')))
            .filter(|s| !s.is_empty())
            .take(129)
            .collect::<Vec<_>>();
        let mut matched = BTreeMap::<(usize, String), Binding>::new();
        let mut covered = BTreeSet::new();
        let mut probes = 0;
        let oversized = normalized.chars().count() > 8192;
        let mut truncated = oversized || tokens.len() > 128;
        tokens.truncate(128);
        if !oversized {
            for start in 0..tokens.len() {
                // An exact phrase can only start with a token that exists in
                // the immutable form index. Single-token probing remains for
                // productive morphology and unmatched-token evidence.
                let phrase_limit = self
                    .form_max_tokens_by_first
                    .get(tokens[start])
                    .copied()
                    .unwrap_or(1);
                for end in start + 1..=(start + phrase_limit).min(tokens.len()) {
                    // Most Korean and English source forms are one token. Keep
                    // that slice borrowed; allocate only for an actual phrase.
                    let form = if end == start + 1 {
                        Cow::Borrowed(tokens[start])
                    } else {
                        Cow::Owned(tokens[start..end].join(" "))
                    };
                    probes += 1;
                    let exact_bindings = self.forms.get(form.as_ref());
                    if let Some(bindings) = exact_bindings {
                        for b in bindings {
                            matched.insert((b.entry, form.to_string()), b.clone());
                        }
                        covered.extend(start..end);
                    }
                    if end != start + 1 {
                        continue;
                    }
                    // Recover productive finite morphology against the sparse
                    // lemma index when source principal forms omit it. A suffix
                    // alone never establishes a lexeme or its world semantics.
                    if exact_bindings.is_none() {
                        for (lemma, ending, rule) in finite_lemma_candidates(form.as_ref()) {
                            probes += 1;
                            if let Some(bindings) = self.forms.get(&lemma) {
                                for b in bindings {
                                    let entry = &self.entries[b.entry];
                                    if normalize(&entry.lemma) == lemma
                                        && matches!(
                                            entry.pos.as_str(),
                                            "동사" | "형용사" | "보조 동사" | "보조 형용사"
                                        )
                                    {
                                        matched.insert(
                                            (b.entry, form.to_string()),
                                            Binding {
                                                entry: b.entry,
                                                base: entry.lemma.trim_end_matches('다').into(),
                                                ending: ending.into(),
                                                rule,
                                            },
                                        );
                                        covered.insert(start);
                                    }
                                }
                            }
                        }
                    }
                    for (cut, _) in form.char_indices().skip(1) {
                        let (base, ending) = form.split_at(cut);
                        if !matches!(
                            ending,
                            "은" | "는"
                                | "이"
                                | "가"
                                | "을"
                                | "를"
                                | "의"
                                | "에"
                                | "에서"
                                | "에게"
                                | "한테"
                                | "으로"
                                | "로"
                                | "도"
                                | "만"
                                | "부터"
                                | "까지"
                                | "와"
                                | "과"
                                | "하고"
                                | "이랑"
                                | "랑"
                                | "에는"
                                | "에서는"
                                | "에게는"
                                | "으로는"
                                | "이라고"
                                | "라고"
                        ) {
                            continue;
                        }
                        probes += 1;
                        if let Some(bindings) = self.forms.get(base) {
                            for b in bindings {
                                if matches!(
                                    self.entries[b.entry].pos.as_str(),
                                    "명사" | "대명사" | "수사" | "의존 명사" | "품사 없음"
                                ) {
                                    matched.insert(
                                        (b.entry, form.to_string()),
                                        Binding {
                                            entry: b.entry,
                                            base: base.into(),
                                            ending: ending.into(),
                                            rule: "KO_NOMINAL_PARTICLE",
                                        },
                                    );
                                    covered.insert(start);
                                }
                            }
                        }
                    }
                }
            }
        }
        truncated |= matched.len() > 32;
        let matches = matched
            .into_iter()
            .take(32)
            .map(|((i, form), b)| {
                let entry = self.entries[i].clone();
                let concept_ids = entry
                    .senses
                    .iter()
                    .filter(|s| {
                        !matches!(
                            b.rule,
                            "SOURCE_ENGLISH_EQUIVALENT"
                                | "EN_IRREGULAR_PAST"
                                | "EN_PAST_PARTICIPLE"
                                | "EN_REGULAR_PAST"
                        ) || s.english.split(';').any(|a| {
                            normalize(a) == form
                                || b.rule == "EN_IRREGULAR_PAST"
                                    && english_irregular_past(a.trim()) == Some(form.as_str())
                                || b.rule == "EN_PAST_PARTICIPLE"
                                    && english_participle(a.trim()) == Some(form.as_str())
                                || b.rule == "EN_REGULAR_PAST"
                                    && english_regular_past(a.trim()).as_deref()
                                        == Some(form.as_str())
                        })
                    })
                    .map(|s| entry.concept_id(s))
                    .collect();
                LexicalKnowledgeMatchIR {
                    entry,
                    matched_form: form,
                    morphology: LexicalMorphologyIR {
                        base: b.base,
                        ending: b.ending,
                        grammar_rule: b.rule.into(),
                    },
                    concept_ids,
                }
            })
            .collect();
        LexicalKnowledgeLookupIR{schema:PACK_SCHEMA.into(),source_sha256:digest(text),pack_sha256:self.sha256.clone(),english_stress_lexicon_sha256:ENGLISH_STRESS_SHA256.into(),matches,
            unmatched_tokens:tokens.iter().enumerate().filter(|(i,_)|!covered.contains(i)).map(|(_,t)|(*t).into()).collect(),truncated,index_probes:probes,full_catalog_scans:0,
            semantic_authority:false,execution_authority:false,attribution:"국립국어원 한국어기초사전, 2026-08-19; CC BY-SA 2.0 KR; definitions/lexical grammar only".into()}
    }
    pub fn statistics(&self) -> LexicalKnowledgePackStatisticsIR {
        let lemmas = |domain: &str| {
            self.entries
                .iter()
                .filter(|e| e.domain == domain)
                .map(|e| &e.lemma)
                .collect::<BTreeSet<_>>()
                .len()
        };
        LexicalKnowledgePackStatisticsIR {
            general_unique_lemmas: lemmas("GENERAL"),
            law_economics_related_unique_lemmas: lemmas("LAW_ECONOMICS_RELATED"),
            supplementary_grammar_entries: self
                .entries
                .iter()
                .filter(|e| e.domain == "GRAMMAR")
                .count(),
            source_entries: self.entries.len(),
            bilingual_senses: self.entries.iter().map(|e| e.senses.len()).sum(),
            indexed_forms: self.forms.len(),
            pack_sha256: self.sha256.clone(),
        }
    }
}

#[derive(Deserialize)]
struct EnglishStressLexicon {
    schema: String,
    base_dictionary_sha256: String,
    entry_count: usize,
    entries: BTreeMap<String, EnglishStressEntry>,
}

#[derive(Deserialize)]
struct EnglishStressEntry {
    pronunciations: Vec<String>,
    attested_forms: Vec<String>,
}

fn stress_licensed_past(lemma: &str) -> Option<String> {
    static LEXICON: OnceLock<EnglishStressLexicon> = OnceLock::new();
    let lexicon = LEXICON.get_or_init(|| {
        assert_eq!(digest(ENGLISH_STRESS_DATA), ENGLISH_STRESS_SHA256);
        let data: EnglishStressLexicon =
            serde_json::from_str(ENGLISH_STRESS_DATA).expect("sealed English stress metadata");
        assert_eq!(data.schema, "B_CORE_ENGLISH_FINAL_STRESS_LEXICON_1");
        assert_eq!(data.base_dictionary_sha256, PACK_SHA256);
        assert_eq!(data.entry_count, data.entries.len());
        assert!(data.entry_count <= 512);
        data
    });
    let entry = lexicon.entries.get(lemma)?;
    let final_stress = |pronunciation: &str| {
        pronunciation
            .split_whitespace()
            .filter_map(|phone| match phone.as_bytes().last()? {
                b'0' => Some(false),
                b'1' | b'2' => Some(true),
                _ => None,
            })
            .next_back()
    };
    let stressed = final_stress(entry.pronunciations.first()?)?;
    if !entry
        .pronunciations
        .iter()
        .all(|p| final_stress(p) == Some(stressed))
    {
        return None;
    }
    let form = if stressed {
        format!("{lemma}{}ed", lemma.chars().last()?)
    } else {
        format!("{lemma}ed")
    };
    // Korean POS does not prove that an English equivalent is a base verb.
    // Require both the stress-derived spelling AND source form attestation.
    entry.attested_forms.contains(&form).then_some(form)
}

fn english_regular_past(lemma: &str) -> Option<String> {
    if lemma.is_empty()
        || !lemma.bytes().all(|b| b.is_ascii_lowercase())
        || english_irregular_past(lemma).is_some()
        || english_participle(lemma).is_some()
        // Unimplemented irregular paradigms are not permission to accept a
        // fabricated regular form (e.g. haved/teached). These are lexical
        // exceptions, not sentence or answer patterns.
        || matches!(lemma,
            "be" | "am" | "is" | "are" | "do" | "have" | "say" | "make" | "get"
            | "know" | "think" | "find" | "feel" | "leave" | "keep" | "sell" | "tell"
            | "meet" | "stand" | "sit" | "fall" | "hold" | "hear" | "lose" | "win"
            | "pay" | "teach" | "catch" | "choose" | "break" | "begin" | "become"
            | "arise" | "awake" | "bear" | "beat" | "bend" | "bet" | "bid" | "bind"
            | "bite" | "bleed" | "blow" | "breed" | "build" | "burst" | "cast"
            | "cling" | "cost" | "creep" | "cut" | "deal" | "dig" | "draw" | "drive"
            | "feed" | "fight" | "flee" | "fling" | "fly" | "forbid" | "forget"
            | "forgive" | "freeze" | "grow" | "hang" | "hide" | "hit" | "hurt"
            | "lay" | "lead" | "lie" | "light" | "mean" | "put" | "quit" | "ride"
            | "ring" | "rise" | "seek" | "set" | "shake" | "shine" | "shoot"
            | "shrink" | "shut" | "sing" | "sink" | "slide" | "slit" | "spend"
            | "spin" | "spit" | "split" | "spread" | "spring" | "steal" | "stick"
            | "sting" | "stink" | "stride" | "strike" | "string" | "strive"
            | "swear" | "sweep" | "swim" | "swing" | "tear" | "throw" | "understand"
            | "wear" | "weave" | "weep" | "wind" | "withdraw" | "wring")
    {
        return None;
    }
    let vowel = |b: u8| matches!(b, b'a' | b'e' | b'i' | b'o' | b'u');
    let bytes = lemma.as_bytes();
    if lemma.ends_with('e') {
        Some(format!("{lemma}d"))
    } else if bytes.len() > 1 && lemma.ends_with('y') && !vowel(bytes[bytes.len() - 2]) {
        Some(format!("{}ied", &lemma[..lemma.len() - 1]))
    } else if bytes.len() >= 3
        && !vowel(bytes[bytes.len() - 3])
        && vowel(bytes[bytes.len() - 2])
        && !vowel(bytes[bytes.len() - 1])
        && !matches!(bytes[bytes.len() - 1], b'w' | b'x' | b'y')
    {
        // Final stress cannot be inferred from spelling for a multi-syllable
        // CVC stem. Do not manufacture a spelling without lexical evidence.
        if bytes.iter().filter(|b| vowel(**b)).count() != 1 {
            return stress_licensed_past(lemma);
        }
        Some(format!("{lemma}{}ed", bytes[bytes.len() - 1] as char))
    } else {
        Some(format!("{lemma}ed"))
    }
}

fn english_irregular_past(lemma: &str) -> Option<&'static str> {
    Some(match lemma {
        "lend" => "lent",
        "send" => "sent",
        "sleep" => "slept",
        "give" => "gave",
        "take" => "took",
        "write" => "wrote",
        "eat" => "ate",
        "buy" => "bought",
        "bring" => "brought",
        "go" => "went",
        "come" => "came",
        "see" => "saw",
        "speak" => "spoke",
        "drink" => "drank",
        "run" => "ran",
        _ => return None,
    })
}

// Lexical inflection data, not a semantic consequence or solved-event table.
pub(crate) fn english_participle(lemma: &str) -> Option<&'static str> {
    Some(match lemma {
        "read" => "read",
        "lend" => "lent",
        "send" => "sent",
        "give" => "given",
        "take" => "taken",
        "write" => "written",
        "eat" => "eaten",
        "buy" => "bought",
        "bring" => "brought",
        "see" => "seen",
        "speak" => "spoken",
        "drink" => "drunk",
        _ => return None,
    })
}
pub(crate) fn has_productive_finite_shape(form: &str) -> bool {
    !finite_lemma_candidates(form).is_empty()
}

fn finite_lemma_candidates(form: &str) -> Vec<(String, &'static str, &'static str)> {
    let mut candidates = Vec::new();
    for ending in [
        "겠어",
        "겠어요",
        "겠다",
        "겠네",
        "겠네요",
        "겠지",
        "겠죠",
        "겠지만",
        "겠습니다",
    ] {
        if let Some(stem) = form.strip_suffix(ending).filter(|s| !s.is_empty()) {
            candidates.push((format!("{stem}다"), ending, "KO_STEM_MODAL_FINITE"));
        }
    }
    for ending in [
        "해",
        "해요",
        "했어",
        "했어요",
        "했다",
        "했네",
        "했네요",
        "했지만",
        "했는데",
    ] {
        if let Some(base) = form.strip_suffix(ending) {
            candidates.push((format!("{base}하다"), ending, "KO_HADA_FINITE_CONTRACTION"));
        }
    }
    candidates
}

pub(crate) fn is_connective_principal_form(form: &str) -> bool {
    if ["아라", "어라", "너라", "거라"]
        .iter()
        .any(|ending| form.ends_with(ending))
    {
        return false;
    }
    form.chars()
        .next_back()
        .and_then(|c| (c as u32).checked_sub(0xac00))
        .is_some_and(|o| {
            o < 11172
                && o % 28 == 0
                && matches!((o / 28) % 21, 0 | 1 | 4 | 5 | 6 | 9 | 10 | 11 | 14 | 15)
        })
}
fn contract_connective(form: &str, stem: &str) -> Option<String> {
    let (suffix_start, ending) = form.char_indices().next_back()?;
    // Do not turn an irregular consonant-deleted form (짓다 -> 지어) into
    // a vowel-stem contraction: its source prefix is not the dictionary stem.
    if &form[..suffix_start] != stem {
        return None;
    }
    let (start, last) = stem.char_indices().next_back()?;
    let offset = (last as u32).checked_sub(0xac00)?;
    if offset >= 11172 || offset % 28 != 0 {
        return None;
    }
    let vowel = (offset / 28) % 21;
    let contracted = match (vowel, ending) {
        (8, '아') => 9,   // ㅗ + ㅏ -> ㅘ
        (13, '어') => 14, // ㅜ + ㅓ -> ㅝ
        (20, '어') => 6,  // ㅣ + ㅓ -> ㅕ
        (11, '어') => 10, // ㅚ + ㅓ -> ㅙ
        _ => return None,
    };
    Some(format!(
        "{}{}",
        &stem[..start],
        char::from_u32(0xac00 + (offset / 588) * 588 + contracted * 28)?
    ))
}
fn add_final(word: &str, final_index: u32) -> Option<String> {
    let (index, last) = word.char_indices().next_back()?;
    let offset = (last as u32).checked_sub(0xac00)?;
    if offset >= 11172 || offset % 28 != 0 {
        return None;
    }
    Some(format!(
        "{}{}",
        &word[..index],
        char::from_u32(last as u32 + final_index)?
    ))
}
fn normalize(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn sealed_working_facet_cache_exactly_matches_every_source_entry() {
        let pack = builtin_pack();
        for entry_id in pack.entry_ids().collect::<Vec<_>>() {
            let entry = pack.entry(entry_id).expect("entry id is indexed");
            assert_eq!(
                pack.sealed_working_lexemes(entry_id),
                Some(entry.working_lexemes().as_slice()),
                "{entry_id}"
            );
        }
    }

    #[test]
    fn stress_and_attestation_license_regular_inflection_not_arbitrary_suffixes() {
        use super::*;
        for (lemma, past) in [
            ("open", "opened"),
            ("happen", "happened"),
            ("visit", "visited"),
            ("limit", "limited"),
            ("prefer", "preferred"),
            ("admit", "admitted"),
            ("permit", "permitted"),
            ("occur", "occurred"),
        ] {
            assert_eq!(english_regular_past(lemma).as_deref(), Some(past));
        }
        for lemma in [
            "transfer", "closed", "stupid", "arson", "traffic", "qzorpen", "begin", "forget",
        ] {
            assert!(english_regular_past(lemma).is_none(), "{lemma}");
        }
        for word in [
            "openned",
            "happenned",
            "visitted",
            "limitted",
            "prefered",
            "haved",
            "teached",
        ] {
            assert!(builtin_pack().lookup(word).matches.is_empty(), "{word}");
        }
    }

    #[test]
    fn inflection_keeps_bilingual_concepts_and_records_auxiliary_provenance() {
        use super::*;
        for (lemma, past) in [
            ("open", "opened"),
            ("visit", "visited"),
            ("admit", "admitted"),
        ] {
            let base = builtin_pack().lookup(lemma);
            let inflected = builtin_pack().lookup(past);
            assert!(!base.truncated && !inflected.truncated);
            assert!(!inflected.matches.is_empty());
            assert_eq!(inflected.full_catalog_scans, 0);
            assert_eq!(
                inflected.english_stress_lexicon_sha256,
                ENGLISH_STRESS_SHA256
            );
            for form in &inflected.matches {
                assert!(base
                    .matches
                    .iter()
                    .any(|b| b.entry == form.entry && b.concept_ids == form.concept_ids));
            }
            assert!(inflected.validate_source(past));
            let mut corrupted = inflected;
            corrupted.english_stress_lexicon_sha256.clear();
            assert!(!corrupted.validate_source(past));
        }
    }

    #[test]
    fn nominal_types_read_definition_heads_not_topic_categories_or_substrings() {
        use super::*;
        let pack = builtin_pack();
        for (surface, expected) in [
            ("창고", NominalReferentKindIR::Place),
            ("학생", NominalReferentKindIR::Person),
        ] {
            let evidence = pack.nominal_referent_evidence(surface).expect(surface);
            assert_eq!(evidence.kind, expected);
            assert_eq!(evidence.pack_sha256, PACK_SHA256);
            assert!(!evidence.definition_heads.is_empty());
        }
        for surface in ["시멘트", "가방", "교사", "창고의 시멘트", "unknown-noun"] {
            assert!(
                pack.nominal_referent_evidence(surface).is_none(),
                "{surface}"
            );
        }
        let korean = pack.nominal_referent_evidence("창고").unwrap();
        let english = pack.nominal_referent_evidence("warehouse").unwrap();
        assert_eq!(korean.kind, english.kind);
        assert_eq!(korean.definition_heads, english.definition_heads);
        assert_eq!(pack.lookup("warehouse").full_catalog_scans, 0);
        assert_eq!(
            definition_referent_head("사람이 물건을 보관하는 곳."),
            Some(("곳", NominalReferentKindIR::Place))
        );
        assert!(definition_referent_head("장소를 찾아가는 행위.").is_none());
        assert!(definition_referent_head("사람이 사용하는 물건.").is_none());
    }
    #[test]
    fn past_recollection_endings_share_source_lemma_and_tense() {
        for (lemma, past) in [
            ("잃어버리다", "잃어버렸"),
            ("쓰다", "썼"),
            ("사다", "샀"),
            ("오다", "왔"),
            ("보다", "봤"),
            ("먹다", "먹었"),
        ] {
            for ending in ["지", "죠", "을까", "을까요"] {
                let form = format!("{past}{ending}");
                let lookup = super::builtin_pack().lookup(&form);
                assert!(!lookup.truncated);
                assert_eq!(lookup.full_catalog_scans, 0);
                assert!(
                    lookup.matches.iter().any(|m| m.entry.lemma == lemma
                        && m.matched_form == form
                        && m.morphology.grammar_rule == "KO_PRINCIPAL_FORM_PAST_ENDING"),
                    "{form}: {:?}",
                    lookup.matches
                );
            }
        }
    }

    #[test]
    fn regular_past_inflection_preserves_lexical_senses() {
        use super::*;
        for (lemma, past) in [
            ("borrow", "borrowed"),
            ("play", "played"),
            ("study", "studied"),
            ("move", "moved"),
            ("stop", "stopped"),
            ("fix", "fixed"),
        ] {
            assert_eq!(english_regular_past(lemma).as_deref(), Some(past));
        }
        assert!(english_regular_past("read").is_none());
        for lemma in ["have", "teach", "do", "make", "put"] {
            assert!(english_regular_past(lemma).is_none());
        }
        assert_eq!(english_regular_past("prefer").as_deref(), Some("preferred"));
        assert!(english_regular_past("transfer").is_none()); // competing stress readings
        assert!(english_regular_past("take away").is_none());
        let lookup = builtin_pack().lookup("borrowed");
        assert!(!lookup.truncated);
        assert_eq!(lookup.full_catalog_scans, 0);
        assert!(lookup
            .matches
            .iter()
            .any(|m| m.entry.source_entry_id == "17824"
                && m.morphology.grammar_rule == "EN_REGULAR_PAST"
                && m.concept_ids.len() == 1));
    }
    use super::*;
    #[test]
    fn pack_has_real_disjoint_counts_and_bilingual_senses() {
        let pack = builtin_pack();
        let s = pack.statistics();
        assert_eq!(s.general_unique_lemmas, 10_000);
        assert_eq!(s.law_economics_related_unique_lemmas, 5_000);
        assert_eq!(s.bilingual_senses, 29_766);
        assert!(pack.entries.iter().all(BilingualLexicalEntryIR::validate));
        let general = pack
            .entries
            .iter()
            .filter(|e| e.domain == "GENERAL")
            .map(|e| &e.lemma)
            .collect::<BTreeSet<_>>();
        assert!(pack
            .entries
            .iter()
            .filter(|e| e.domain == "LAW_ECONOMICS_RELATED")
            .all(|e| !general.contains(&e.lemma)));
    }
    #[test]
    fn korean_agglutinative_forms_recover_lemma_not_whole_sentence() {
        for form in [
            "먹었어",
            "먹을래",
            "먹자",
            "먹었거든",
            "먹긴 했는데",
            "먹으려나",
        ] {
            let result = builtin_pack().lookup(form);
            assert!(
                result.matches.iter().any(|m| m.entry.lemma == "먹다"),
                "{form}"
            );
            assert_eq!(result.full_catalog_scans, 0);
            assert!(result.validate_source(form));
        }
        for text in ["계약서를", "보험료는", "재산권에서"] {
            let result = builtin_pack().lookup(text);
            assert!(
                result
                    .matches
                    .iter()
                    .any(|m| m.morphology.grammar_rule == "KO_NOMINAL_PARTICLE"),
                "{text}"
            );
        }
    }
    #[test]
    fn source_linked_english_and_polysemy_do_not_duplicate_meaning() {
        let ko = builtin_pack().lookup("계약");
        let en = builtin_pack().lookup("contract");
        let ko_ids = ko
            .matches
            .iter()
            .flat_map(|m| m.concept_ids.iter())
            .collect::<BTreeSet<_>>();
        assert!(en
            .matches
            .iter()
            .flat_map(|m| &m.concept_ids)
            .any(|id| ko_ids.contains(id)));
        assert!(!ko.semantic_authority && !ko.execution_authority);
        let mut tampered = ko.clone();
        tampered.matches[0].entry.senses[0].english = "invented translation".into();
        assert!(!tampered.validate_source("계약"));
        assert!(LexicalKnowledgePack::from_jsonl(DATA, "bad hash").is_err());
    }
    #[test]
    fn productive_finite_lookup_uses_lemma_evidence_without_catalog_growth() {
        let pack = builtin_pack();
        let before = pack.statistics();
        for (surface, lemma) in [
            ("취소해", "취소하다"),
            ("취소했어요", "취소하다"),
            ("모르겠어", "모르다"),
            ("읽겠어요", "읽다"),
            ("확인했네요", "확인하다"),
            ("노력해요", "노력하다"),
        ] {
            let found = pack.lookup(surface);
            let base = pack.lookup(lemma);
            assert!(
                found.matches.iter().any(|m| m.entry.lemma == lemma),
                "{surface}"
            );
            for m in found.matches.iter().filter(|m| m.entry.lemma == lemma) {
                assert!(base
                    .matches
                    .iter()
                    .any(|b| b.entry == m.entry && b.concept_ids == m.concept_ids));
            }
            assert_eq!(found.full_catalog_scans, 0);
            assert!(found.index_probes < 20);
            assert!(found.validate_source(surface));
            assert!(!found.semantic_authority && !found.execution_authority);
        }
        for unknown in ["zzqvnonce해", "zzqvnonce겠어요"] {
            assert!(pack.lookup(unknown).matches.is_empty());
        }
        assert_eq!(before, pack.statistics());
    }

    #[test]
    fn morphology_transfers_across_roots_and_keeps_bounds_explicit() {
        for (surface, lemma) in [
            ("갔어요", "가다"),
            ("왔거든", "오다"),
            ("봤잖아", "보다"),
            ("했는데", "하다"),
            ("읽었어", "읽다"),
            ("마셨어요", "마시다"),
            ("들었어", "듣다"),
            ("걸었어요", "걷다"),
            ("도왔거든", "돕다"),
            ("지었어", "짓다"),
            ("몰랐어요", "모르다"),
            ("썼는데", "쓰다"),
            ("들을래", "듣다"),
            ("걸으려나", "걷다"),
            ("도우려나", "돕다"),
            ("마실래", "마시다"),
            ("읽자", "읽다"),
            ("먹지", "먹다"),
            ("피곤했어", "피곤하다"),
            ("답답하잖아", "답답하다"),
            ("애매하네요", "애매하다"),
            ("솔직히", "솔직히"),
            ("글쎄", "글쎄"),
            ("어쩐지", "어쩐지"),
        ] {
            let found = builtin_pack().lookup(surface);
            assert!(
                found.matches.iter().any(|m| m.entry.lemma == lemma),
                "{surface} -> {lemma}"
            );
            assert!(!found.semantic_authority && !found.execution_authority);
        }
        let unknown = builtin_pack().lookup("zzqvnoncezz");
        assert!(unknown.matches.is_empty());
        assert_eq!(unknown.unmatched_tokens, ["zzqvnoncezz"]);
        assert!(
            builtin_pack()
                .lookup(&vec!["먹다"; 129].join(" "))
                .truncated
        );
        let oversized = builtin_pack().lookup(&"가".repeat(8193));
        assert!(oversized.truncated && oversized.matches.is_empty());
        assert_eq!(oversized.index_probes, 0);
    }
    #[test]
    fn phrase_prefix_index_covers_every_legacy_lookup_window() {
        let pack = builtin_pack();
        for form in pack.forms.keys() {
            let tokens = form
                .split(|c: char| !(c.is_alphanumeric() || matches!(c, '-' | '\'')))
                .filter(|token| !token.is_empty())
                .collect::<Vec<_>>();
            let Some(first) = tokens.first() else {
                continue;
            };
            let legacy_window = tokens.len().min(8);
            assert!(
                pack.form_max_tokens_by_first
                    .get(*first)
                    .is_some_and(|limit| *limit >= legacy_window),
                "{form}"
            );
        }
    }

    #[test]
    fn conversation_and_lookup_api_use_the_real_pack_without_action_authority() {
        use crate::cognitive::{CognitiveApi, CognitiveApiCommandIR, CognitiveApiPayloadIR};
        use crate::conversation::{
            ConversationInputModalityIR, ConversationTurnRequestIR,
            CONVERSATION_TURN_REQUEST_SCHEMA,
        };
        use crate::language_knowledge::LanguageCodeIR;
        let mut api = CognitiveApi::new_embedded().unwrap();
        let queried = api.execute_command(CognitiveApiCommandIR::LookupLexicalKnowledge {
            text: "계약서를".into(),
        });
        assert!(
            matches!(queried.payload,Some(CognitiveApiPayloadIR::LexicalKnowledgeLookup(ref q)) if q.validate_source("계약서를") && !q.matches.is_empty())
        );
        for (i, text) in ["음, 나 피곤해.", "먹었어?", "계약서는?"]
            .into_iter()
            .enumerate()
        {
            let request = ConversationTurnRequestIR {
                schema: CONVERSATION_TURN_REQUEST_SCHEMA.into(),
                conversation_id: "PACK-API".into(),
                turn_index: i as u64 + 1,
                request_id: format!("PACK-API-{i}"),
                modality: ConversationInputModalityIR::Text,
                raw_text: text.into(),
                input_confidence_millis: 1000,
                alternatives: vec![],
                output_language: Some(LanguageCodeIR::Korean),
                context_tags: vec![],
                max_plan_steps: 16,
            };
            let response = api.process_conversation_turn(&request).unwrap();
            assert!(response.validate_against(&request));
            assert!(!response.lexical_knowledge.matches.is_empty(), "{text}");
            assert!(
                !response.lexical_knowledge.semantic_authority
                    && !response.lexical_knowledge.execution_authority
            );
            assert!(response
                .conversation_state
                .action_state_ledger
                .records
                .is_empty());
            let mut tampered = response.clone();
            tampered.lexical_knowledge.matches.clear();
            assert!(!tampered.validate_against(&request));
        }
    }
}
