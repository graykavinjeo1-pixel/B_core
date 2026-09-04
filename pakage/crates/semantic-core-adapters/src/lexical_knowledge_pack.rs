//! Source-attributed bilingual lexical knowledge, not promoted world facts.
//! Shared immutable data + indexed, bounded lookup. No text-to-action authority.
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const PACK_SHA256: &str = "d614048f9df99ac9104cf0206ace8cb9238f0f6a7490bfb06980d6fbf69d23c8";
pub const PACK_SCHEMA: &str = "B_CORE_BILINGUAL_LEXICAL_LOOKUP_1";
const DATA: &str = include_str!("../data/lexical-knowledge/nikl-ko-en.jsonl");

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexicalKnowledgeLookupIR {
    pub schema: String,
    pub source_sha256: String,
    pub pack_sha256: String,
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
    forms: HashMap<String, Vec<Binding>>,
}
pub fn builtin_pack() -> &'static LexicalKnowledgePack {
    static PACK: OnceLock<LexicalKnowledgePack> = OnceLock::new();
    PACK.get_or_init(|| {
        LexicalKnowledgePack::from_jsonl(DATA, PACK_SHA256).expect("sealed bilingual pack")
    })
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
            forms: HashMap::new(),
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
        Ok(pack)
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
        if matches!(e.pos.as_str(), "동사" | "보조 동사") {
            self.add_form(i, stem, "자", "KO_PROPOSITIVE");
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
                    if let Some(past) = add_final(&connected, 20) {
                        for ending in [
                            "어",
                            "어요",
                            "다",
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
                for end in start + 1..=(start + 8).min(tokens.len()) {
                    let form = tokens[start..end].join(" ");
                    probes += 1;
                    if let Some(bindings) = self.forms.get(&form) {
                        for b in bindings {
                            matched.insert((b.entry, form.clone()), b.clone());
                        }
                        covered.extend(start..end);
                    }
                    if end != start + 1 {
                        continue;
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
                                        (b.entry, form.clone()),
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
                        b.rule != "SOURCE_ENGLISH_EQUIVALENT"
                            || s.english.split(';').any(|a| normalize(a) == form)
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
        LexicalKnowledgeLookupIR{schema:PACK_SCHEMA.into(),source_sha256:digest(text),pack_sha256:self.sha256.clone(),matches,
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
fn is_connective_principal_form(form: &str) -> bool {
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
