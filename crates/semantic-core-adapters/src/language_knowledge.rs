use std::collections::{BTreeMap, BTreeSet};

use dockable_semantic_core::PlanIntentIR;
use serde::{Deserialize, Serialize};

pub const LANGUAGE_KNOWLEDGE_SCHEMA: &str = "B_CORE_LANGUAGE_KNOWLEDGE_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LanguageCodeIR {
    Korean,
    English,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LanguageKnowledgeCategoryIR {
    Grammar,
    Word,
    Idiom,
    Slang,
    InternetLanguage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LanguageRegisterIR {
    Formal,
    Neutral,
    Informal,
    Internet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PragmaticFunctionIR {
    Request,
    Proceed,
    Acknowledge,
    Reject,
    Approve,
    Laugh,
    Emphasize,
    Hedge,
    Sequence,
    Cause,
    Condition,
    Caution,
    ExactDiagnosis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageKnowledgeEntryIR {
    pub schema: String,
    pub knowledge_id: String,
    pub language: LanguageCodeIR,
    pub category: LanguageKnowledgeCategoryIR,
    pub register: LanguageRegisterIR,
    pub surface_forms: Vec<String>,
    pub canonical_concept: String,
    pub semantic_tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent_hint: Option<PlanIntentIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pragmatic_function: Option<PragmaticFunctionIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageUnderstandingIR {
    pub original_text: String,
    pub normalized_text: String,
    pub detected_language: LanguageCodeIR,
    pub detected_register: LanguageRegisterIR,
    pub intent: PlanIntentIR,
    pub subject: String,
    pub constraints: Vec<String>,
    pub desired_outcomes: Vec<String>,
    pub semantic_tags: Vec<String>,
    pub matched_knowledge_ids: Vec<String>,
    pub pragmatic_functions: Vec<PragmaticFunctionIR>,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageKnowledgeStatisticsIR {
    pub total_entries: usize,
    pub korean_entries: usize,
    pub english_entries: usize,
    pub grammar_entries: usize,
    pub word_entries: usize,
    pub idiom_entries: usize,
    pub slang_entries: usize,
    pub internet_language_entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LanguageKnowledgeError {
    InvalidSchema,
    InvalidIdentity,
    InvalidSurfaceForms,
    InvalidSemantics,
    IdentityConflict,
    EmptyInput,
}

#[derive(Debug, Clone)]
pub struct LanguageKnowledgeBase {
    entries: BTreeMap<String, LanguageKnowledgeEntryIR>,
}

impl Default for LanguageKnowledgeBase {
    fn default() -> Self {
        Self::bilingual_builtin()
    }
}

impl LanguageKnowledgeBase {
    pub fn bilingual_builtin() -> Self {
        let mut knowledge = Self {
            entries: BTreeMap::new(),
        };
        for entry in builtin_entries() {
            knowledge
                .inject(entry)
                .expect("built-in language knowledge is valid");
        }
        knowledge
    }

    pub fn inject(
        &mut self,
        entry: LanguageKnowledgeEntryIR,
    ) -> Result<bool, LanguageKnowledgeError> {
        validate_entry(&entry)?;
        if let Some(existing) = self.entries.get(&entry.knowledge_id) {
            if existing == &entry {
                return Ok(false);
            }
            return Err(LanguageKnowledgeError::IdentityConflict);
        }
        self.entries.insert(entry.knowledge_id.clone(), entry);
        Ok(true)
    }

    pub fn understand(
        &self,
        text: &str,
    ) -> Result<LanguageUnderstandingIR, LanguageKnowledgeError> {
        let normalized_text = normalize(text);
        if normalized_text.is_empty() {
            return Err(LanguageKnowledgeError::EmptyInput);
        }
        let detected_language = detect_language(&normalized_text);
        let mut matched = self
            .entries
            .values()
            .filter(|entry| {
                entry
                    .surface_forms
                    .iter()
                    .any(|form| surface_matches(&normalized_text, form, entry.language))
            })
            .cloned()
            .collect::<Vec<_>>();
        matched.sort_by(|left, right| left.knowledge_id.cmp(&right.knowledge_id));

        let mut intent_scores = BTreeMap::<PlanIntentIR, i32>::new();
        let mut semantic_tags = BTreeSet::new();
        let mut pragmatic_functions = BTreeSet::new();
        let mut internet_register = false;
        for entry in &matched {
            semantic_tags.insert(entry.canonical_concept.clone());
            semantic_tags.extend(entry.semantic_tags.iter().cloned());
            if let Some(intent) = entry.intent_hint {
                *intent_scores.entry(intent).or_default() += match entry.category {
                    LanguageKnowledgeCategoryIR::Word => 40,
                    LanguageKnowledgeCategoryIR::Idiom => 20,
                    _ => 10,
                };
            }
            if let Some(function) = entry.pragmatic_function {
                pragmatic_functions.insert(function);
            }
            internet_register |= entry.register == LanguageRegisterIR::Internet;
        }
        let intent = intent_scores
            .into_iter()
            .max_by_key(|(intent, score)| {
                (
                    score.saturating_add(intent_priority(*intent)),
                    intent_priority(*intent),
                )
            })
            .map(|(intent, _)| intent)
            .unwrap_or(PlanIntentIR::Plan);
        let detected_register = if internet_register {
            LanguageRegisterIR::Internet
        } else if normalized_text.contains("please")
            || normalized_text.contains("해주세요")
            || normalized_text.contains("하십시오")
        {
            LanguageRegisterIR::Formal
        } else if normalized_text.contains("해줘")
            || normalized_text.contains("ㄱㄱ")
            || normalized_text.contains("let's")
        {
            LanguageRegisterIR::Informal
        } else {
            LanguageRegisterIR::Neutral
        };
        semantic_tags.insert(intent_tag(intent).to_string());
        let constraints = vec![
            "preserve verified behavior outside the stated subject".to_string(),
            "bind claims and actions to observable evidence".to_string(),
        ];
        let desired_outcomes = desired_outcomes(intent, &normalized_text);
        let confidence_millis = (450_u16)
            .saturating_add(u16::try_from(matched.len().min(10) * 50).unwrap_or(500))
            .min(950);
        Ok(LanguageUnderstandingIR {
            original_text: text.to_string(),
            normalized_text: normalized_text.clone(),
            detected_language,
            detected_register,
            intent,
            subject: normalized_text,
            constraints,
            desired_outcomes,
            semantic_tags: semantic_tags.into_iter().collect(),
            matched_knowledge_ids: matched
                .into_iter()
                .map(|entry| entry.knowledge_id)
                .collect(),
            pragmatic_functions: pragmatic_functions.into_iter().collect(),
            confidence_millis,
        })
    }

    pub fn statistics(&self) -> LanguageKnowledgeStatisticsIR {
        let count_language = |language| {
            self.entries
                .values()
                .filter(|entry| entry.language == language)
                .count()
        };
        let count_category = |category| {
            self.entries
                .values()
                .filter(|entry| entry.category == category)
                .count()
        };
        LanguageKnowledgeStatisticsIR {
            total_entries: self.entries.len(),
            korean_entries: count_language(LanguageCodeIR::Korean),
            english_entries: count_language(LanguageCodeIR::English),
            grammar_entries: count_category(LanguageKnowledgeCategoryIR::Grammar),
            word_entries: count_category(LanguageKnowledgeCategoryIR::Word),
            idiom_entries: count_category(LanguageKnowledgeCategoryIR::Idiom),
            slang_entries: count_category(LanguageKnowledgeCategoryIR::Slang),
            internet_language_entries: count_category(
                LanguageKnowledgeCategoryIR::InternetLanguage,
            ),
        }
    }
}

fn validate_entry(entry: &LanguageKnowledgeEntryIR) -> Result<(), LanguageKnowledgeError> {
    if entry.schema != LANGUAGE_KNOWLEDGE_SCHEMA {
        return Err(LanguageKnowledgeError::InvalidSchema);
    }
    if entry.knowledge_id.is_empty()
        || entry.knowledge_id.len() > 128
        || !entry
            .knowledge_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(LanguageKnowledgeError::InvalidIdentity);
    }
    if entry.surface_forms.is_empty()
        || entry.surface_forms.len() > 32
        || entry
            .surface_forms
            .iter()
            .any(|form| form.trim().is_empty() || form.len() > 256)
    {
        return Err(LanguageKnowledgeError::InvalidSurfaceForms);
    }
    if entry.canonical_concept.trim().is_empty()
        || entry.canonical_concept.len() > 256
        || entry.semantic_tags.len() > 64
        || entry
            .semantic_tags
            .iter()
            .any(|tag| tag.trim().is_empty() || tag.len() > 128)
    {
        return Err(LanguageKnowledgeError::InvalidSemantics);
    }
    Ok(())
}

fn normalize(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn surface_matches(text: &str, surface_form: &str, language: LanguageCodeIR) -> bool {
    let form = normalize(surface_form);
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

fn intent_priority(intent: PlanIntentIR) -> i32 {
    match intent {
        // Concrete requested outcomes outrank supporting discourse acts. For
        // example, "inspect and repair" is a repair request whose first plan
        // phase happens to be investigation, not two competing top-level goals.
        PlanIntentIR::Repair => 30,
        PlanIntentIR::Create => 28,
        PlanIntentIR::Execute => 26,
        PlanIntentIR::Learn => 24,
        PlanIntentIR::Explain => 20,
        PlanIntentIR::Investigate => 12,
        PlanIntentIR::Communicate => 8,
        PlanIntentIR::Plan => 4,
    }
}

fn detect_language(text: &str) -> LanguageCodeIR {
    let korean = text
        .chars()
        .any(|character| matches!(character, '\u{ac00}'..='\u{d7a3}' | '\u{3131}'..='\u{318e}'));
    let english = text
        .chars()
        .any(|character| character.is_ascii_alphabetic());
    match (korean, english) {
        (true, true) => LanguageCodeIR::Mixed,
        (true, false) => LanguageCodeIR::Korean,
        (false, true) => LanguageCodeIR::English,
        (false, false) => LanguageCodeIR::Unknown,
    }
}

fn intent_tag(intent: PlanIntentIR) -> &'static str {
    match intent {
        PlanIntentIR::Plan => "plan",
        PlanIntentIR::Investigate => "investigate",
        PlanIntentIR::Repair => "repair",
        PlanIntentIR::Create => "create",
        PlanIntentIR::Learn => "learn",
        PlanIntentIR::Explain => "explain",
        PlanIntentIR::Communicate => "communicate",
        PlanIntentIR::Execute => "execute",
    }
}

fn desired_outcomes(intent: PlanIntentIR, subject: &str) -> Vec<String> {
    let outcome = match intent {
        PlanIntentIR::Repair => "the observed defect is removed without regression",
        PlanIntentIR::Investigate => "the causal bottleneck is identified with evidence",
        PlanIntentIR::Create => "the requested capability is implemented and verified",
        PlanIntentIR::Learn => "a reusable successful method is validated and retained",
        PlanIntentIR::Explain => "the subject is explained with supported claims",
        PlanIntentIR::Communicate => "the result is communicated clearly and faithfully",
        PlanIntentIR::Execute => "the requested action completes with verified effects",
        PlanIntentIR::Plan => "an executable dependency-ordered plan is produced",
    };
    vec![format!("{outcome}: {subject}")]
}

#[allow(clippy::too_many_arguments)]
fn entry(
    id: &str,
    language: LanguageCodeIR,
    category: LanguageKnowledgeCategoryIR,
    register: LanguageRegisterIR,
    forms: &[&str],
    concept: &str,
    tags: &[&str],
    intent_hint: Option<PlanIntentIR>,
    pragmatic_function: Option<PragmaticFunctionIR>,
) -> LanguageKnowledgeEntryIR {
    LanguageKnowledgeEntryIR {
        schema: LANGUAGE_KNOWLEDGE_SCHEMA.to_string(),
        knowledge_id: id.to_string(),
        language,
        category,
        register,
        surface_forms: forms.iter().map(|form| (*form).to_string()).collect(),
        canonical_concept: concept.to_string(),
        semantic_tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
        intent_hint,
        pragmatic_function,
    }
}

#[allow(clippy::too_many_lines)]
fn builtin_entries() -> Vec<LanguageKnowledgeEntryIR> {
    use LanguageCodeIR::{English as En, Korean as Ko};
    use LanguageKnowledgeCategoryIR::{Grammar, Idiom, InternetLanguage, Slang, Word};
    use LanguageRegisterIR::{Formal, Informal, Internet, Neutral};
    use PlanIntentIR::{Communicate, Create, Execute, Explain, Investigate, Learn, Plan, Repair};
    use PragmaticFunctionIR::{
        Acknowledge, Approve, Cause, Caution, Condition, Emphasize, ExactDiagnosis, Hedge, Laugh,
        Proceed, Reject, Request, Sequence,
    };
    vec![
        entry(
            "KO.WORD.PLAN",
            Ko,
            Word,
            Neutral,
            &["계획", "계획해"],
            "plan",
            &["planning"],
            Some(Plan),
            None,
        ),
        entry(
            "KO.WORD.INSPECT",
            Ko,
            Word,
            Neutral,
            &["점검", "확인", "분석"],
            "investigate",
            &["diagnosis"],
            Some(Investigate),
            None,
        ),
        entry(
            "KO.WORD.REPAIR",
            Ko,
            Word,
            Neutral,
            &["수리", "고쳐", "고치", "해결"],
            "repair",
            &["defect"],
            Some(Repair),
            None,
        ),
        entry(
            "KO.WORD.CREATE",
            Ko,
            Word,
            Neutral,
            &["만들", "구현", "추가"],
            "create",
            &["implementation"],
            Some(Create),
            None,
        ),
        entry(
            "KO.WORD.LEARN",
            Ko,
            Word,
            Neutral,
            &["학습", "배워", "기억", "흡수"],
            "learn",
            &["knowledge"],
            Some(Learn),
            None,
        ),
        entry(
            "KO.WORD.EXPLAIN",
            Ko,
            Word,
            Neutral,
            &["설명", "왜", "알려"],
            "explain",
            &["explanation"],
            Some(Explain),
            None,
        ),
        entry(
            "KO.WORD.EXECUTE",
            Ko,
            Word,
            Neutral,
            &["열어", "읽어", "저장", "실행", "보여", "계속"],
            "execute_action",
            &["execute"],
            Some(Execute),
            None,
        ),
        entry(
            "KO.WORD.COMMUNICATE",
            Ko,
            Word,
            Neutral,
            &["말해", "대화", "채팅"],
            "communicate",
            &["dialogue"],
            Some(Communicate),
            None,
        ),
        entry(
            "EN.WORD.PLAN",
            En,
            Word,
            Neutral,
            &["plan", "roadmap"],
            "plan",
            &["planning"],
            Some(Plan),
            None,
        ),
        entry(
            "EN.WORD.INSPECT",
            En,
            Word,
            Neutral,
            &["inspect", "analyze", "diagnose", "check"],
            "investigate",
            &["diagnosis"],
            Some(Investigate),
            None,
        ),
        entry(
            "EN.WORD.REPAIR",
            En,
            Word,
            Neutral,
            &["repair", "fix", "resolve"],
            "repair",
            &["defect"],
            Some(Repair),
            None,
        ),
        entry(
            "EN.WORD.CREATE",
            En,
            Word,
            Neutral,
            &["create", "implement", "build", "add"],
            "create",
            &["implementation"],
            Some(Create),
            None,
        ),
        entry(
            "EN.WORD.LEARN",
            En,
            Word,
            Neutral,
            &["learn", "remember", "absorb"],
            "learn",
            &["knowledge"],
            Some(Learn),
            None,
        ),
        entry(
            "EN.WORD.EXPLAIN",
            En,
            Word,
            Neutral,
            &["explain", "why", "describe"],
            "explain",
            &["explanation"],
            Some(Explain),
            None,
        ),
        entry(
            "EN.WORD.EXECUTE",
            En,
            Word,
            Neutral,
            &["open", "read", "save", "run", "show", "continue"],
            "execute_action",
            &["execute"],
            Some(Execute),
            None,
        ),
        entry(
            "EN.WORD.COMMUNICATE",
            En,
            Word,
            Neutral,
            &["talk", "chat", "tell"],
            "communicate",
            &["dialogue"],
            Some(Communicate),
            None,
        ),
        entry(
            "KO.GRAMMAR.REQUEST",
            Ko,
            Grammar,
            Formal,
            &["해주세요", "해 주십시오", "하십시오"],
            "polite_request",
            &["request"],
            None,
            Some(Request),
        ),
        entry(
            "KO.GRAMMAR.INFORMAL_REQUEST",
            Ko,
            Grammar,
            Informal,
            &["해줘", "해 봐", "하자"],
            "informal_request",
            &["request"],
            None,
            Some(Request),
        ),
        entry(
            "KO.GRAMMAR.SEQUENCE",
            Ko,
            Grammar,
            Neutral,
            &["한 다음", "한 뒤", "그리고", "하면서"],
            "sequence",
            &["dependency"],
            None,
            Some(Sequence),
        ),
        entry(
            "KO.GRAMMAR.CAUSE",
            Ko,
            Grammar,
            Neutral,
            &["때문에", "그래서", "원인"],
            "cause",
            &["causal"],
            Some(Investigate),
            Some(Cause),
        ),
        entry(
            "KO.GRAMMAR.CONDITION",
            Ko,
            Grammar,
            Neutral,
            &["라면", "하면", "경우"],
            "condition",
            &["conditional"],
            None,
            Some(Condition),
        ),
        entry(
            "EN.GRAMMAR.REQUEST",
            En,
            Grammar,
            Formal,
            &["please", "could you", "would you"],
            "polite_request",
            &["request"],
            None,
            Some(Request),
        ),
        entry(
            "EN.GRAMMAR.SEQUENCE",
            En,
            Grammar,
            Neutral,
            &["and then", "after that", "while"],
            "sequence",
            &["dependency"],
            None,
            Some(Sequence),
        ),
        entry(
            "EN.GRAMMAR.CAUSE",
            En,
            Grammar,
            Neutral,
            &["because", "therefore", "root cause"],
            "cause",
            &["causal"],
            Some(Investigate),
            Some(Cause),
        ),
        entry(
            "EN.GRAMMAR.CONDITION",
            En,
            Grammar,
            Neutral,
            &["if", "when", "unless"],
            "condition",
            &["conditional"],
            None,
            Some(Condition),
        ),
        entry(
            "KO.IDIOM.URGENT",
            Ko,
            Idiom,
            Informal,
            &["발등에 불이 떨어지다", "발등에 불"],
            "urgent_problem",
            &["urgent"],
            Some(Repair),
            Some(Emphasize),
        ),
        entry(
            "KO.IDIOM.CAUTION",
            Ko,
            Idiom,
            Neutral,
            &["돌다리도 두드려 보고 건너라", "돌다리도 두드려"],
            "verify_before_action",
            &["verification"],
            None,
            Some(Caution),
        ),
        entry(
            "KO.IDIOM.EXACT",
            Ko,
            Idiom,
            Neutral,
            &["정곡을 찌르다"],
            "exact_diagnosis",
            &["diagnosis"],
            Some(Investigate),
            Some(ExactDiagnosis),
        ),
        entry(
            "EN.IDIOM.EXACT",
            En,
            Idiom,
            Neutral,
            &["hit the nail on the head"],
            "exact_diagnosis",
            &["diagnosis"],
            Some(Investigate),
            Some(ExactDiagnosis),
        ),
        entry(
            "EN.IDIOM.CAUTION",
            En,
            Idiom,
            Neutral,
            &["look before you leap"],
            "verify_before_action",
            &["verification"],
            None,
            Some(Caution),
        ),
        entry(
            "EN.IDIOM.START",
            En,
            Idiom,
            Neutral,
            &["get the ball rolling"],
            "begin_action",
            &["proceed"],
            Some(Plan),
            Some(Proceed),
        ),
        entry(
            "KO.SLANG.OK",
            Ko,
            Slang,
            Informal,
            &["ㅇㅋ", "오케이"],
            "acknowledge",
            &["approval"],
            None,
            Some(Acknowledge),
        ),
        entry(
            "KO.SLANG.NO",
            Ko,
            Slang,
            Informal,
            &["ㄴㄴ", "노노"],
            "reject",
            &["rejection"],
            None,
            Some(Reject),
        ),
        entry(
            "KO.SLANG.GO",
            Ko,
            Slang,
            Internet,
            &["ㄱㄱ", "가즈아"],
            "proceed",
            &["execute"],
            None,
            Some(Proceed),
        ),
        entry(
            "KO.INTERNET.LAUGH",
            Ko,
            InternetLanguage,
            Internet,
            &["ㅋㅋ", "ㅎㅎ"],
            "laughter",
            &["affect"],
            None,
            Some(Laugh),
        ),
        entry(
            "KO.INTERNET.APPROVE",
            Ko,
            InternetLanguage,
            Internet,
            &["인정", "ㅇㅈ"],
            "approve",
            &["approval"],
            None,
            Some(Approve),
        ),
        entry(
            "EN.SLANG.HONEST",
            En,
            Slang,
            Informal,
            &["tbh", "to be honest"],
            "honest_qualifier",
            &["stance"],
            None,
            Some(Hedge),
        ),
        entry(
            "EN.SLANG.OPINION",
            En,
            Slang,
            Informal,
            &["imo", "imho"],
            "opinion_qualifier",
            &["stance"],
            None,
            Some(Hedge),
        ),
        entry(
            "EN.INTERNET.INFO",
            En,
            InternetLanguage,
            Internet,
            &["fyi"],
            "information_marker",
            &["context"],
            None,
            Some(Emphasize),
        ),
        entry(
            "EN.INTERNET.APPROVE",
            En,
            InternetLanguage,
            Internet,
            &["lgtm", "+1"],
            "approve",
            &["approval"],
            None,
            Some(Approve),
        ),
        entry(
            "EN.INTERNET.LAUGH",
            En,
            InternetLanguage,
            Internet,
            &["lol", "lmao"],
            "laughter",
            &["affect"],
            None,
            Some(Laugh),
        ),
        entry(
            "EN.INTERNET.AWAY",
            En,
            InternetLanguage,
            Internet,
            &["brb", "afk"],
            "temporary_absence",
            &["availability"],
            None,
            None,
        ),
        entry(
            "KO.TREND.DILIGENT",
            Ko,
            Slang,
            Internet,
            &["갓생"],
            "disciplined_self_improvement",
            &["learn", "persistence"],
            Some(Learn),
            Some(Approve),
        ),
        entry(
            "KO.TREND.SELF_CAUSED",
            Ko,
            Slang,
            Internet,
            &["스불재"],
            "self_caused_outcome",
            &["causal", "diagnosis"],
            Some(Investigate),
            Some(Cause),
        ),
        entry(
            "KO.TREND.AUTONOMOUS_QUALITY",
            Ko,
            Slang,
            Internet,
            &["알잘딱깔센"],
            "autonomous_context_appropriate_concise_execution",
            &["planning", "quality"],
            Some(Plan),
            Some(Proceed),
        ),
        entry(
            "KO.TREND.PERSIST",
            Ko,
            Slang,
            Internet,
            &["중꺾마"],
            "persistence_despite_difficulty",
            &["learn", "persistence"],
            Some(Learn),
            Some(Emphasize),
        ),
        entry(
            "KO.TREND.FRUSTRATION",
            Ko,
            Slang,
            Internet,
            &["킹받네", "킹받다"],
            "frustration",
            &["affect"],
            None,
            Some(Emphasize),
        ),
        entry(
            "EN.TREND.SHIP",
            En,
            Slang,
            Internet,
            &["ship it"],
            "proceed_to_verified_delivery",
            &["execute", "delivery"],
            Some(dockable_semantic_core::PlanIntentIR::Execute),
            Some(Proceed),
        ),
        entry(
            "EN.TREND.NO_CAP",
            En,
            Slang,
            Internet,
            &["no cap"],
            "sincerity_emphasis",
            &["stance"],
            None,
            Some(Emphasize),
        ),
        entry(
            "EN.TREND.BASED",
            En,
            Slang,
            Internet,
            &["based"],
            "strong_approval",
            &["approval"],
            None,
            Some(Approve),
        ),
        entry(
            "EN.TREND.LOW_KEY",
            En,
            Slang,
            Internet,
            &["low-key", "low key"],
            "softened_emphasis",
            &["stance"],
            None,
            Some(Hedge),
        ),
        entry(
            "EN.TREND.TOUCH_GRASS",
            En,
            Slang,
            Internet,
            &["touch grass"],
            "step_back_and_reassess",
            &["caution", "reassessment"],
            Some(Investigate),
            Some(Caution),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knowledge_is_typed_across_language_and_register_categories() {
        let stats = LanguageKnowledgeBase::default().statistics();
        assert!(stats.korean_entries >= 15);
        assert!(stats.english_entries >= 15);
        assert!(stats.grammar_entries >= 8);
        assert!(stats.word_entries >= 12);
        assert!(stats.idiom_entries >= 6);
        assert!(stats.slang_entries >= 5);
        assert!(stats.internet_language_entries >= 6);
    }

    #[test]
    fn korean_and_english_internet_language_affect_typed_understanding() {
        let knowledge = LanguageKnowledgeBase::default();
        let korean = knowledge
            .understand("경로 오류 원인을 점검하고 수리 계획 ㄱㄱ")
            .unwrap();
        assert_eq!(korean.detected_language, LanguageCodeIR::Korean);
        assert_eq!(korean.intent, PlanIntentIR::Repair);
        assert!(korean
            .pragmatic_functions
            .contains(&PragmaticFunctionIR::Proceed));
        let english = knowledge
            .understand("FYI, please diagnose and fix the path defect")
            .unwrap();
        assert_eq!(english.detected_language, LanguageCodeIR::English);
        assert_eq!(english.intent, PlanIntentIR::Repair);
        assert_eq!(english.detected_register, LanguageRegisterIR::Internet);
    }
}
