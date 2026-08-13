use std::collections::{BTreeMap, BTreeSet};

use dockable_semantic_core::PlanIntentIR;
use serde::{Deserialize, Serialize};

use crate::language_knowledge::LanguageCodeIR;

pub const LEXEME_SCHEMA: &str = "B_CORE_LEXEME_IR_1";
pub const LEXEME_SNAPSHOT_SCHEMA: &str = "B_CORE_LEXEME_SNAPSHOT_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PartOfSpeechIR {
    Noun,
    Verb,
    Adjective,
    Adverb,
    Pronoun,
    Determiner,
    Particle,
    Conjunction,
    Preposition,
    Interjection,
    Phrase,
    Symbol,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GrammaticalRoleIR {
    Subject,
    Object,
    Predicate,
    Modifier,
    Complement,
    Connector,
    Topic,
    CaseMarker,
    Command,
    DiscourseMarker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticRelationKindIR {
    Synonym,
    Antonym,
    Hypernym,
    Hyponym,
    Entails,
    Related,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticRelationIR {
    pub relation: SemanticRelationKindIR,
    pub target_sense_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SenseIR {
    pub sense_id: String,
    pub canonical_concept: String,
    pub gloss: String,
    #[serde(default)]
    pub semantic_tags: Vec<String>,
    #[serde(default)]
    pub context_selectors: Vec<String>,
    #[serde(default)]
    pub relations: Vec<SemanticRelationIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent_hint: Option<PlanIntentIR>,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexemeIR {
    pub schema: String,
    pub lexeme_id: String,
    pub language: LanguageCodeIR,
    pub lemma: String,
    #[serde(default)]
    pub inflected_forms: Vec<String>,
    pub part_of_speech: PartOfSpeechIR,
    #[serde(default)]
    pub grammatical_roles: Vec<GrammaticalRoleIR>,
    pub senses: Vec<SenseIR>,
    #[serde(default)]
    pub collocations: Vec<String>,
    #[serde(default)]
    pub domains: Vec<String>,
    pub source: String,
    pub confidence_millis: u16,
    #[serde(default)]
    pub frequency_prior: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LexemeUsageIR {
    pub encounter_count: u64,
    pub last_observed_sequence: u64,
    #[serde(default)]
    pub sense_usage: BTreeMap<String, SenseUsageIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SenseUsageIR {
    pub activation_count: u64,
    pub verified_success_count: u64,
    pub rejected_activation_count: u64,
    pub last_observed_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivatedSenseIR {
    pub lexeme_id: String,
    pub sense_id: String,
    pub matched_form: String,
    pub canonical_concept: String,
    pub semantic_tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent_hint: Option<PlanIntentIR>,
    pub activation_millis: u32,
    pub activation_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexicalOutcomeIR {
    pub activation_keys: Vec<String>,
    pub verified_success: bool,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexemeSnapshotEntryIR {
    pub lexeme: LexemeIR,
    pub usage: LexemeUsageIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexemeSnapshotIR {
    pub schema: String,
    pub sequence: u64,
    pub entries: Vec<LexemeSnapshotEntryIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexicalMemoryStatisticsIR {
    pub lexeme_count: usize,
    pub sense_count: usize,
    pub total_encounters: u64,
    pub verified_successes: u64,
    pub rejected_activations: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LexicalMemoryError {
    InvalidSchema,
    InvalidIdentity,
    InvalidLexeme,
    InvalidSense,
    IdentityConflict,
    UnknownActivation,
    MissingEvidence,
    SnapshotConflict,
}

#[derive(Debug, Clone)]
pub struct LexicalMemory {
    entries: BTreeMap<String, (LexemeIR, LexemeUsageIR)>,
    sequence: u64,
}

impl Default for LexicalMemory {
    fn default() -> Self {
        let mut memory = Self {
            entries: BTreeMap::new(),
            sequence: 0,
        };
        for lexeme in builtin_lexemes() {
            memory.inject(lexeme).expect("valid built-in lexeme");
        }
        memory
    }
}

impl LexicalMemory {
    pub fn inject(&mut self, lexeme: LexemeIR) -> Result<bool, LexicalMemoryError> {
        validate_lexeme(&lexeme)?;
        if let Some((existing, _)) = self.entries.get(&lexeme.lexeme_id) {
            return if existing == &lexeme {
                Ok(false)
            } else {
                Err(LexicalMemoryError::IdentityConflict)
            };
        }
        self.entries
            .insert(lexeme.lexeme_id.clone(), (lexeme, LexemeUsageIR::default()));
        Ok(true)
    }

    /// Activates meanings from surface form, context, collocation, frequency and
    /// verified-use history. Encounter frequency is a prior, never sole authority.
    pub fn activate(&mut self, text: &str, context_tags: &[String]) -> Vec<ActivatedSenseIR> {
        self.sequence = self.sequence.saturating_add(1);
        let normalized = normalize(text);
        let context = context_tags
            .iter()
            .map(|value| normalize(value))
            .collect::<BTreeSet<_>>();
        let mut activated = Vec::new();
        for (lexeme_id, (lexeme, usage)) in &mut self.entries {
            let matched_form = std::iter::once(&lexeme.lemma)
                .chain(&lexeme.inflected_forms)
                .filter(|form| surface_matches(&normalized, form, lexeme.language))
                .max_by_key(|form| form.chars().count())
                .cloned();
            let Some(matched_form) = matched_form else {
                continue;
            };
            usage.encounter_count = usage.encounter_count.saturating_add(1);
            usage.last_observed_sequence = self.sequence;
            for sense in &lexeme.senses {
                let sense_usage = usage.sense_usage.entry(sense.sense_id.clone()).or_default();
                sense_usage.activation_count = sense_usage.activation_count.saturating_add(1);
                sense_usage.last_observed_sequence = self.sequence;
                let mut reasons = vec!["surface_form".to_string()];
                let selector_overlap = sense
                    .context_selectors
                    .iter()
                    .filter(|selector| {
                        let selector = normalize(selector);
                        context.contains(&selector) || normalized.contains(&selector)
                    })
                    .count();
                let domain_overlap = lexeme
                    .domains
                    .iter()
                    .filter(|domain| {
                        let domain = normalize(domain);
                        context.contains(&domain) || normalized.contains(&domain)
                    })
                    .count();
                let collocation_hits = lexeme
                    .collocations
                    .iter()
                    .filter(|collocation| normalized.contains(&normalize(collocation)))
                    .count();
                if selector_overlap > 0 {
                    reasons.push(format!("context_selector:{selector_overlap}"));
                }
                if domain_overlap > 0 {
                    reasons.push(format!("domain:{domain_overlap}"));
                }
                if collocation_hits > 0 {
                    reasons.push(format!("collocation:{collocation_hits}"));
                }
                if sense_usage.verified_success_count > 0 {
                    reasons.push(format!(
                        "verified_success:{}",
                        sense_usage.verified_success_count
                    ));
                }
                let frequency = log2_floor(
                    usage
                        .encounter_count
                        .saturating_add(u64::from(lexeme.frequency_prior))
                        .saturating_add(1),
                );
                let ambiguity_penalty = lexeme.senses.len().saturating_sub(1) as u32 * 35;
                let rejection_penalty =
                    u32::try_from(sense_usage.rejected_activation_count.min(20)).unwrap_or(20) * 20;
                let score = 180_u32
                    .saturating_add(frequency.saturating_mul(45))
                    .saturating_add(u32::try_from(selector_overlap).unwrap_or(0) * 180)
                    .saturating_add(u32::try_from(domain_overlap).unwrap_or(0) * 120)
                    .saturating_add(u32::try_from(collocation_hits).unwrap_or(0) * 100)
                    .saturating_add(
                        u32::try_from(sense_usage.verified_success_count.min(20)).unwrap_or(20)
                            * 30,
                    )
                    .saturating_add(u32::from(sense.confidence_millis) / 5)
                    .saturating_add(u32::from(lexeme.confidence_millis) / 10)
                    .saturating_sub(ambiguity_penalty)
                    .saturating_sub(rejection_penalty)
                    .min(4_000);
                activated.push(ActivatedSenseIR {
                    lexeme_id: lexeme_id.clone(),
                    sense_id: sense.sense_id.clone(),
                    matched_form: matched_form.clone(),
                    canonical_concept: sense.canonical_concept.clone(),
                    semantic_tags: sense.semantic_tags.clone(),
                    intent_hint: sense.intent_hint,
                    activation_millis: score,
                    activation_reasons: reasons,
                });
            }
        }
        activated = self.spread_semantic_relations(activated);
        activated.sort_by(|left, right| {
            right
                .activation_millis
                .cmp(&left.activation_millis)
                .then_with(|| left.lexeme_id.cmp(&right.lexeme_id))
                .then_with(|| left.sense_id.cmp(&right.sense_id))
        });
        activated.truncate(64);
        activated
    }

    fn spread_semantic_relations(&self, direct: Vec<ActivatedSenseIR>) -> Vec<ActivatedSenseIR> {
        let sense_index = self
            .entries
            .iter()
            .flat_map(|(lexeme_id, (lexeme, _))| {
                lexeme
                    .senses
                    .iter()
                    .map(move |sense| (sense.sense_id.clone(), (lexeme_id, sense)))
            })
            .collect::<BTreeMap<_, _>>();
        let mut best = direct
            .iter()
            .cloned()
            .map(|activation| {
                (
                    (activation.lexeme_id.clone(), activation.sense_id.clone()),
                    activation,
                )
            })
            .collect::<BTreeMap<_, _>>();
        for source in &direct {
            let Some((_, (lexeme, _))) = self.entries.get_key_value(&source.lexeme_id) else {
                continue;
            };
            let Some(source_sense) = lexeme
                .senses
                .iter()
                .find(|sense| sense.sense_id == source.sense_id)
            else {
                continue;
            };
            for relation in &source_sense.relations {
                let Some((target_lexeme_id, target_sense)) =
                    sense_index.get(&relation.target_sense_id)
                else {
                    continue;
                };
                let factor = relation_factor(relation.relation);
                let activation_millis = source.activation_millis.saturating_mul(factor) / 100;
                let candidate = ActivatedSenseIR {
                    lexeme_id: (*target_lexeme_id).clone(),
                    sense_id: target_sense.sense_id.clone(),
                    matched_form: "<semantic-spread>".to_string(),
                    canonical_concept: target_sense.canonical_concept.clone(),
                    semantic_tags: target_sense.semantic_tags.clone(),
                    intent_hint: target_sense.intent_hint,
                    activation_millis,
                    activation_reasons: vec![format!(
                        "semantic_relation:{:?}:{}",
                        relation.relation, source.sense_id
                    )],
                };
                let key = (candidate.lexeme_id.clone(), candidate.sense_id.clone());
                if best
                    .get(&key)
                    .is_none_or(|existing| candidate.activation_millis > existing.activation_millis)
                {
                    best.insert(key, candidate);
                }
            }
        }
        best.into_values().collect()
    }

    pub fn record_outcome(&mut self, outcome: &LexicalOutcomeIR) -> Result<(), LexicalMemoryError> {
        if outcome.evidence.is_empty()
            || outcome
                .evidence
                .iter()
                .any(|item| item.trim().is_empty() || item.len() > 4_096)
        {
            return Err(LexicalMemoryError::MissingEvidence);
        }
        let mut activation_ids = BTreeSet::new();
        for key in &outcome.activation_keys {
            let Some((lexeme_id, sense_id)) = key.split_once('/') else {
                return Err(LexicalMemoryError::UnknownActivation);
            };
            let Some((lexeme, _)) = self.entries.get(lexeme_id) else {
                return Err(LexicalMemoryError::UnknownActivation);
            };
            if !lexeme.senses.iter().any(|sense| sense.sense_id == sense_id) {
                return Err(LexicalMemoryError::UnknownActivation);
            }
            activation_ids.insert((lexeme_id.to_string(), sense_id.to_string()));
        }
        for (lexeme_id, sense_id) in activation_ids {
            let (_, usage) = self
                .entries
                .get_mut(&lexeme_id)
                .ok_or(LexicalMemoryError::UnknownActivation)?;
            let sense_usage = usage.sense_usage.entry(sense_id).or_default();
            if outcome.verified_success {
                sense_usage.verified_success_count =
                    sense_usage.verified_success_count.saturating_add(1);
            } else {
                sense_usage.rejected_activation_count =
                    sense_usage.rejected_activation_count.saturating_add(1);
            }
        }
        Ok(())
    }

    pub fn snapshot(&self) -> LexemeSnapshotIR {
        LexemeSnapshotIR {
            schema: LEXEME_SNAPSHOT_SCHEMA.to_string(),
            sequence: self.sequence,
            entries: self
                .entries
                .values()
                .map(|(lexeme, usage)| LexemeSnapshotEntryIR {
                    lexeme: lexeme.clone(),
                    usage: usage.clone(),
                })
                .collect(),
        }
    }

    pub fn import_snapshot(
        &mut self,
        snapshot: &LexemeSnapshotIR,
    ) -> Result<(), LexicalMemoryError> {
        if snapshot.schema != LEXEME_SNAPSHOT_SCHEMA {
            return Err(LexicalMemoryError::InvalidSchema);
        }
        let mut candidate = BTreeMap::new();
        for entry in &snapshot.entries {
            validate_lexeme(&entry.lexeme)?;
            if candidate
                .insert(
                    entry.lexeme.lexeme_id.clone(),
                    (entry.lexeme.clone(), entry.usage.clone()),
                )
                .is_some()
            {
                return Err(LexicalMemoryError::SnapshotConflict);
            }
        }
        self.entries = candidate;
        self.sequence = snapshot.sequence;
        Ok(())
    }

    pub fn statistics(&self) -> LexicalMemoryStatisticsIR {
        LexicalMemoryStatisticsIR {
            lexeme_count: self.entries.len(),
            sense_count: self
                .entries
                .values()
                .map(|(lexeme, _)| lexeme.senses.len())
                .sum(),
            total_encounters: self
                .entries
                .values()
                .map(|(_, usage)| usage.encounter_count)
                .sum(),
            verified_successes: self
                .entries
                .values()
                .flat_map(|(_, usage)| usage.sense_usage.values())
                .map(|usage| usage.verified_success_count)
                .sum(),
            rejected_activations: self
                .entries
                .values()
                .flat_map(|(_, usage)| usage.sense_usage.values())
                .map(|usage| usage.rejected_activation_count)
                .sum(),
        }
    }
}

fn validate_lexeme(lexeme: &LexemeIR) -> Result<(), LexicalMemoryError> {
    if lexeme.schema != LEXEME_SCHEMA {
        return Err(LexicalMemoryError::InvalidSchema);
    }
    if !valid_id(&lexeme.lexeme_id) {
        return Err(LexicalMemoryError::InvalidIdentity);
    }
    if lexeme.lemma.trim().is_empty()
        || lexeme.lemma.len() > 256
        || lexeme.inflected_forms.len() > 128
        || lexeme.collocations.len() > 128
        || lexeme.domains.len() > 64
        || lexeme.source.trim().is_empty()
        || lexeme.source.len() > 2_048
        || lexeme.confidence_millis > 1_000
        || lexeme.senses.is_empty()
        || lexeme.senses.len() > 32
    {
        return Err(LexicalMemoryError::InvalidLexeme);
    }
    let mut sense_ids = BTreeSet::new();
    for sense in &lexeme.senses {
        if !valid_id(&sense.sense_id)
            || !sense_ids.insert(sense.sense_id.clone())
            || sense.canonical_concept.trim().is_empty()
            || sense.gloss.trim().is_empty()
            || sense.confidence_millis > 1_000
            || sense.semantic_tags.len() > 64
            || sense.context_selectors.len() > 64
            || sense.relations.len() > 128
        {
            return Err(LexicalMemoryError::InvalidSense);
        }
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn surface_matches(text: &str, surface_form: &str, language: LanguageCodeIR) -> bool {
    let form = normalize(surface_form);
    if form.is_empty() {
        return false;
    }
    if language == LanguageCodeIR::English
        && form
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        text.split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .any(|token| token == form)
    } else {
        text.contains(&form)
    }
}

fn log2_floor(value: u64) -> u32 {
    u64::BITS - 1 - value.max(1).leading_zeros()
}

fn relation_factor(relation: SemanticRelationKindIR) -> u32 {
    match relation {
        SemanticRelationKindIR::Synonym => 75,
        SemanticRelationKindIR::Entails => 65,
        SemanticRelationKindIR::Hypernym | SemanticRelationKindIR::Hyponym => 50,
        SemanticRelationKindIR::Related => 40,
        SemanticRelationKindIR::Antonym => 30,
    }
}

#[allow(clippy::too_many_arguments)]
fn lexeme(
    id: &str,
    language: LanguageCodeIR,
    lemma: &str,
    forms: &[&str],
    part_of_speech: PartOfSpeechIR,
    concept: &str,
    gloss: &str,
    tags: &[&str],
    selectors: &[&str],
    domains: &[&str],
    intent_hint: Option<PlanIntentIR>,
) -> LexemeIR {
    LexemeIR {
        schema: LEXEME_SCHEMA.to_string(),
        lexeme_id: id.to_string(),
        language,
        lemma: lemma.to_string(),
        inflected_forms: forms.iter().map(|value| (*value).to_string()).collect(),
        part_of_speech,
        grammatical_roles: vec![GrammaticalRoleIR::Command],
        senses: vec![SenseIR {
            sense_id: format!("{id}.S1"),
            canonical_concept: concept.to_string(),
            gloss: gloss.to_string(),
            semantic_tags: tags.iter().map(|value| (*value).to_string()).collect(),
            context_selectors: selectors.iter().map(|value| (*value).to_string()).collect(),
            relations: Vec::new(),
            intent_hint,
            confidence_millis: 950,
        }],
        collocations: Vec::new(),
        domains: domains.iter().map(|value| (*value).to_string()).collect(),
        source: "B_Core bilingual cognitive bootstrap".to_string(),
        confidence_millis: 950,
        frequency_prior: 8,
    }
}

fn builtin_lexemes() -> Vec<LexemeIR> {
    use LanguageCodeIR::{English, Korean};
    use PartOfSpeechIR::{Noun, Verb};
    use PlanIntentIR::{Create, Investigate, Plan};
    vec![
        lexeme(
            "KO.PAPER",
            Korean,
            "논문",
            &["연구논문"],
            Noun,
            "academic_paper",
            "학술적 주장과 근거를 구조화한 문서",
            &["paper", "research"],
            &[],
            &["academic"],
            Some(Investigate),
        ),
        lexeme(
            "EN.PAPER",
            English,
            "paper",
            &["research paper", "article"],
            Noun,
            "academic_paper",
            "a structured academic argument with evidence",
            &["paper", "research"],
            &[],
            &["academic"],
            Some(Investigate),
        ),
        lexeme(
            "KO.BUSINESS_PLAN",
            Korean,
            "사업계획서",
            &["사업 계획서", "창업계획서", "창업 계획서"],
            Noun,
            "business_plan",
            "사업 모델, 시장, 실행, 재무 계획을 근거와 함께 구조화한 문서",
            &["business", "plan", "strategy", "execution"],
            &["시장", "재무", "실행", "사업"],
            &["business", "strategy"],
            Some(Plan),
        ),
        lexeme(
            "EN.BUSINESS_PLAN",
            English,
            "business plan",
            &["venture plan", "startup plan"],
            Noun,
            "business_plan",
            "an evidence-grounded business model, market, execution, and financial plan",
            &["business", "plan", "strategy", "execution"],
            &["market", "financial", "execution", "business"],
            &["business", "strategy"],
            Some(Plan),
        ),
        lexeme(
            "KO.BUSINESS_PROPOSAL",
            Korean,
            "사업제안서",
            &["사업 제안서", "제안서", "사업 제안"],
            Noun,
            "business_proposal",
            "고객 문제, 제안 가치, 범위, 일정, 비용과 다음 행동을 설득적으로 구조화한 문서",
            &["business", "proposal", "value", "decision"],
            &["고객", "제안", "범위", "일정", "비용"],
            &["business", "sales"],
            Some(Create),
        ),
        lexeme(
            "EN.BUSINESS_PROPOSAL",
            English,
            "business proposal",
            &["commercial proposal", "client proposal"],
            Noun,
            "business_proposal",
            "a persuasive decision document covering client need, value, scope, timing, and cost",
            &["business", "proposal", "value", "decision"],
            &["client", "proposal", "scope", "timeline", "cost"],
            &["business", "sales"],
            Some(Create),
        ),
        lexeme(
            "KO.TABLE",
            Korean,
            "표",
            &["테이블"],
            Noun,
            "data_table",
            "행과 열로 구성된 구조 데이터",
            &["table", "structured_data"],
            &["행", "열", "데이터"],
            &["data"],
            Some(Investigate),
        ),
        lexeme(
            "EN.TABLE",
            English,
            "table",
            &["data table"],
            Noun,
            "data_table",
            "data organized in rows and columns",
            &["table", "structured_data"],
            &["row", "column"],
            &["data"],
            Some(Investigate),
        ),
        lexeme(
            "KO.CHART",
            Korean,
            "차트",
            &["도표", "그래프"],
            Noun,
            "data_chart",
            "수치 관계를 시각 부호로 표현한 구조",
            &["chart", "visualization"],
            &["축", "범례", "추세"],
            &["data"],
            Some(Investigate),
        ),
        lexeme(
            "EN.CHART",
            English,
            "chart",
            &["graph", "plot"],
            Noun,
            "data_chart",
            "a visual encoding of numeric relations",
            &["chart", "visualization"],
            &["axis", "legend", "trend"],
            &["data"],
            Some(Investigate),
        ),
        lexeme(
            "KO.FINANCIAL",
            Korean,
            "재무제표",
            &["손익계산서", "대차대조표", "현금흐름표"],
            Noun,
            "financial_statement",
            "기간별 기업 재무 상태와 흐름을 나타내는 문서",
            &["finance", "accounting"],
            &["자산", "부채", "자본", "매출"],
            &["finance"],
            Some(Investigate),
        ),
        lexeme(
            "EN.FINANCIAL",
            English,
            "financial statement",
            &["balance sheet", "income statement", "cash flow statement"],
            Noun,
            "financial_statement",
            "a period-bound accounting representation",
            &["finance", "accounting"],
            &["asset", "liability", "equity", "revenue"],
            &["finance"],
            Some(Investigate),
        ),
        lexeme(
            "KO.ANALYZE",
            Korean,
            "분석하다",
            &["분석", "해석", "검토"],
            Verb,
            "analyze",
            "구조와 근거에서 의미를 도출하다",
            &["analyze"],
            &[],
            &[],
            Some(Investigate),
        ),
        lexeme(
            "EN.ANALYZE",
            English,
            "analyze",
            &["interpret", "inspect"],
            Verb,
            "analyze",
            "derive supported meaning from structure and evidence",
            &["analyze"],
            &[],
            &[],
            Some(Investigate),
        ),
        lexeme(
            "KO.WRITE",
            Korean,
            "작성하다",
            &["작성", "써줘", "만들어"],
            Verb,
            "author",
            "근거와 구조를 갖춘 결과물을 생성하다",
            &["write", "create"],
            &[],
            &[],
            Some(Create),
        ),
        lexeme(
            "EN.WRITE",
            English,
            "write",
            &["author", "draft", "create"],
            Verb,
            "author",
            "create a structured evidence-grounded artifact",
            &["write", "create"],
            &[],
            &[],
            Some(Create),
        ),
        lexeme(
            "KO.REVISE",
            Korean,
            "수정하다",
            &["수정", "고쳐", "개정"],
            Verb,
            "revise",
            "기존 구조를 보존하며 지정된 내용을 변경하다",
            &["revise", "edit"],
            &[],
            &[],
            Some(Create),
        ),
        lexeme(
            "EN.REVISE",
            English,
            "revise",
            &["edit", "modify", "rewrite"],
            Verb,
            "revise",
            "change specified content while preserving unrelated structure",
            &["revise", "edit"],
            &[],
            &[],
            Some(Create),
        ),
        lexeme(
            "KO.PLAN",
            Korean,
            "계획하다",
            &["계획", "계획안"],
            Verb,
            "plan",
            "목표를 의존관계가 있는 실행 단계로 바꾸다",
            &["plan"],
            &[],
            &[],
            Some(Plan),
        ),
        lexeme(
            "EN.PLAN",
            English,
            "plan",
            &["roadmap", "proposal"],
            Verb,
            "plan",
            "transform a goal into dependency-ordered actions",
            &["plan"],
            &[],
            &[],
            Some(Plan),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_disambiguation_and_frequency_are_bounded_priors() {
        let mut memory = LexicalMemory::default();
        let first = memory.activate("재무제표 표를 분석해", &["data".to_string()]);
        let table = first
            .iter()
            .find(|sense| sense.lexeme_id == "KO.TABLE")
            .unwrap();
        assert!(table.activation_reasons.iter().any(|reason| {
            reason.starts_with("context_selector") || reason.starts_with("domain")
        }));
        let key = format!("{}/{}", table.lexeme_id, table.sense_id);
        memory
            .record_outcome(&LexicalOutcomeIR {
                activation_keys: vec![key],
                verified_success: true,
                evidence: vec!["table interpretation verified".to_string()],
            })
            .unwrap();
        let second = memory.activate("표를 분석해", &["data".to_string()]);
        let table = second
            .iter()
            .find(|sense| sense.lexeme_id == "KO.TABLE")
            .unwrap();
        assert!(table
            .activation_reasons
            .iter()
            .any(|reason| reason.starts_with("verified_success")));
    }

    #[test]
    fn snapshot_round_trip_preserves_usage() {
        let mut source = LexicalMemory::default();
        source.activate("write a paper", &[]);
        let snapshot = source.snapshot();
        let mut destination = LexicalMemory::default();
        destination.import_snapshot(&snapshot).unwrap();
        assert_eq!(source.statistics(), destination.statistics());
    }

    #[test]
    fn polysemous_word_selects_a_sense_by_context_and_credits_only_that_sense() {
        let mut memory = LexicalMemory::default();
        memory
            .inject(LexemeIR {
                schema: LEXEME_SCHEMA.to_string(),
                lexeme_id: "KO.POLY.BAE".to_string(),
                language: LanguageCodeIR::Korean,
                lemma: "배".to_string(),
                inflected_forms: Vec::new(),
                part_of_speech: PartOfSpeechIR::Noun,
                grammatical_roles: vec![GrammaticalRoleIR::Object],
                senses: vec![
                    SenseIR {
                        sense_id: "KO.POLY.BAE.FRUIT".to_string(),
                        canonical_concept: "pear".to_string(),
                        gloss: "먹는 과일".to_string(),
                        semantic_tags: vec!["food".to_string()],
                        context_selectors: vec!["과일".to_string(), "먹다".to_string()],
                        relations: Vec::new(),
                        intent_hint: None,
                        confidence_millis: 950,
                    },
                    SenseIR {
                        sense_id: "KO.POLY.BAE.SHIP".to_string(),
                        canonical_concept: "ship".to_string(),
                        gloss: "물을 건너는 선박".to_string(),
                        semantic_tags: vec!["transport".to_string()],
                        context_selectors: vec!["바다".to_string(), "항구".to_string()],
                        relations: Vec::new(),
                        intent_hint: None,
                        confidence_millis: 950,
                    },
                ],
                collocations: Vec::new(),
                domains: Vec::new(),
                source: "test evidence".to_string(),
                confidence_millis: 950,
                frequency_prior: 100,
            })
            .unwrap();
        let activated = memory.activate("배가 항구에 도착했다", &["바다".to_string()]);
        let selected = activated
            .iter()
            .find(|activation| activation.lexeme_id == "KO.POLY.BAE")
            .unwrap();
        assert_eq!(selected.sense_id, "KO.POLY.BAE.SHIP");
        memory
            .record_outcome(&LexicalOutcomeIR {
                activation_keys: vec![format!("{}/{}", selected.lexeme_id, selected.sense_id)],
                verified_success: true,
                evidence: vec!["context refers to harbor arrival".to_string()],
            })
            .unwrap();
        let snapshot = memory.snapshot();
        let usage = &snapshot
            .entries
            .iter()
            .find(|entry| entry.lexeme.lexeme_id == "KO.POLY.BAE")
            .unwrap()
            .usage
            .sense_usage;
        assert_eq!(usage["KO.POLY.BAE.SHIP"].verified_success_count, 1);
        assert_eq!(usage["KO.POLY.BAE.FRUIT"].verified_success_count, 0);
    }

    #[test]
    fn typed_semantic_relations_spread_bounded_activation() {
        let mut memory = LexicalMemory::default();
        memory
            .inject(LexemeIR {
                schema: LEXEME_SCHEMA.to_string(),
                lexeme_id: "EN.REL.INCREASE".to_string(),
                language: LanguageCodeIR::English,
                lemma: "increase".to_string(),
                inflected_forms: vec!["increased".to_string()],
                part_of_speech: PartOfSpeechIR::Verb,
                grammatical_roles: vec![GrammaticalRoleIR::Predicate],
                senses: vec![SenseIR {
                    sense_id: "EN.REL.INCREASE.S1".to_string(),
                    canonical_concept: "increase".to_string(),
                    gloss: "become greater".to_string(),
                    semantic_tags: vec!["change".to_string()],
                    context_selectors: Vec::new(),
                    relations: vec![SemanticRelationIR {
                        relation: SemanticRelationKindIR::Synonym,
                        target_sense_id: "EN.REL.RISE.S1".to_string(),
                    }],
                    intent_hint: None,
                    confidence_millis: 950,
                }],
                collocations: Vec::new(),
                domains: Vec::new(),
                source: "relation test".to_string(),
                confidence_millis: 950,
                frequency_prior: 1,
            })
            .unwrap();
        memory
            .inject(LexemeIR {
                schema: LEXEME_SCHEMA.to_string(),
                lexeme_id: "EN.REL.RISE".to_string(),
                language: LanguageCodeIR::English,
                lemma: "rise".to_string(),
                inflected_forms: Vec::new(),
                part_of_speech: PartOfSpeechIR::Verb,
                grammatical_roles: vec![GrammaticalRoleIR::Predicate],
                senses: vec![SenseIR {
                    sense_id: "EN.REL.RISE.S1".to_string(),
                    canonical_concept: "rise".to_string(),
                    gloss: "move upward".to_string(),
                    semantic_tags: vec!["change".to_string()],
                    context_selectors: Vec::new(),
                    relations: Vec::new(),
                    intent_hint: None,
                    confidence_millis: 950,
                }],
                collocations: Vec::new(),
                domains: Vec::new(),
                source: "relation test".to_string(),
                confidence_millis: 950,
                frequency_prior: 1,
            })
            .unwrap();
        let activated = memory.activate("values increased", &[]);
        let direct = activated
            .iter()
            .find(|activation| activation.sense_id == "EN.REL.INCREASE.S1")
            .unwrap();
        let spread = activated
            .iter()
            .find(|activation| activation.sense_id == "EN.REL.RISE.S1")
            .unwrap();
        assert!(spread.activation_millis < direct.activation_millis);
        assert!(spread
            .activation_reasons
            .iter()
            .any(|reason| reason.starts_with("semantic_relation:Synonym")));
    }
}
