use std::collections::{BTreeMap, BTreeSet};

use super::{
    concepts::{hash_serializable, ConceptRegistry},
    model::{
        GroundingCondition, Language, LanguageTaskCategory, LexicalAlias, MeaningRequestIR,
        Quantifier, RealizationStyle, SemanticOperation, VisibleLanguageTask,
    },
};

#[derive(Debug, Clone)]
pub struct LexicalStore {
    aliases: BTreeMap<String, LexicalAlias>,
    next_id: usize,
    semantic_duplicates_avoided: usize,
}

impl LexicalStore {
    pub fn canonical() -> Self {
        let mut store = Self {
            aliases: BTreeMap::new(),
            next_id: 0,
            semantic_duplicates_avoided: 0,
        };
        for (surface, language, concept, sense, scope) in default_aliases() {
            store.attach(
                surface,
                language,
                Some(concept),
                sense,
                scope,
                vec!["SEM7_CONTROLLED_LEXICON".to_string()],
                true,
            );
        }
        store.semantic_duplicates_avoided = 0;
        store
    }

    #[allow(clippy::too_many_arguments)]
    pub fn attach(
        &mut self,
        surface: &str,
        language: Language,
        concept_id: Option<&str>,
        sense_id: &str,
        scope: &str,
        provenance: Vec<String>,
        semantic_grounding_complete: bool,
    ) -> String {
        if let Some(existing) = self.aliases.values().find(|alias| {
            normalize(&alias.surface_form) == normalize(surface)
                && alias.language == language
                && alias.concept_id.as_deref() == concept_id
                && alias.sense_id == sense_id
        }) {
            return existing.alias_id.clone();
        }
        let alias_id = format!("LEX-{:05}", self.next_id);
        self.next_id += 1;
        let mut morphology = BTreeMap::new();
        morphology.insert("normalization".to_string(), "unicode-lowercase".to_string());
        self.aliases.insert(
            alias_id.clone(),
            LexicalAlias {
                alias_id: alias_id.clone(),
                surface_form: surface.to_string(),
                language,
                concept_id: concept_id.map(str::to_string),
                sense_id: sense_id.to_string(),
                morphological_features: morphology,
                syntactic_role: "semantic predicate or concept noun".to_string(),
                scope: scope.to_string(),
                confidence: if semantic_grounding_complete {
                    0.99
                } else {
                    0.4
                },
                provenance,
                version: "SEM7-LEXICON-1".to_string(),
                semantic_grounding_complete,
            },
        );
        alias_id
    }

    pub fn ground_definition(
        &mut self,
        surface: &str,
        language: Language,
        definition: &str,
        registry: &ConceptRegistry,
    ) -> DefinitionGrounding {
        let Some(concept) = registry.semantically_equivalent_signature(definition) else {
            let alias_id = self.attach(
                surface,
                language,
                None,
                "UNRESOLVED",
                "definition-incomplete",
                vec!["CONTROLLED_DEFINITION".to_string()],
                false,
            );
            return DefinitionGrounding {
                alias_id,
                concept_id: None,
                semantic_duplicate_avoided: false,
                grounding_complete: false,
            };
        };
        let sense = sense_from_definition(definition, &concept.concept_id);
        let concept_id = concept.concept_id.clone();
        let alias_id = self.attach(
            surface,
            language,
            Some(&concept_id),
            &sense,
            "definition-grounded",
            vec![
                "CONTROLLED_DEFINITION".to_string(),
                format!("SEMANTIC_EQUIVALENCE_MATCH:{concept_id}"),
            ],
            true,
        );
        self.semantic_duplicates_avoided += 1;
        DefinitionGrounding {
            alias_id,
            concept_id: Some(concept_id),
            semantic_duplicate_avoided: true,
            grounding_complete: true,
        }
    }

    pub fn rename(&mut self, alias_id: &str, new_surface: &str) -> Result<(), String> {
        let alias = self
            .aliases
            .get_mut(alias_id)
            .ok_or_else(|| format!("UNKNOWN_ALIAS:{alias_id}"))?;
        alias.surface_form = new_surface.to_string();
        alias.version = "SEM7-LEXICON-2".to_string();
        Ok(())
    }

    pub fn remove_alias(&mut self, alias_id: &str) -> Option<LexicalAlias> {
        self.aliases.remove(alias_id)
    }

    pub fn remove_concept_aliases(&mut self, concept_id: &str) -> Vec<LexicalAlias> {
        let ids = self
            .aliases
            .values()
            .filter(|alias| alias.concept_id.as_deref() == Some(concept_id))
            .map(|alias| alias.alias_id.clone())
            .collect::<Vec<_>>();
        ids.into_iter()
            .filter_map(|alias_id| self.aliases.remove(&alias_id))
            .collect()
    }

    pub fn candidates<'a>(&'a self, text: &str) -> Vec<&'a LexicalAlias> {
        let normalized = normalize(text);
        self.aliases
            .values()
            .filter(|alias| {
                alias.semantic_grounding_complete
                    && normalized.contains(&normalize(&alias.surface_form))
            })
            .collect()
    }

    pub fn aliases(&self) -> impl Iterator<Item = &LexicalAlias> {
        self.aliases.values()
    }

    pub fn len(&self) -> usize {
        self.aliases.len()
    }

    pub fn is_empty(&self) -> bool {
        self.aliases.is_empty()
    }

    pub fn semantic_duplicates_avoided(&self) -> usize {
        self.semantic_duplicates_avoided
    }

    pub fn hash(&self) -> String {
        hash_serializable(&self.aliases)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionGrounding {
    pub alias_id: String,
    pub concept_id: Option<String>,
    pub semantic_duplicate_avoided: bool,
    pub grounding_complete: bool,
}

#[derive(Debug, Clone, Copy)]
struct AdapterCapabilities {
    compositional: bool,
    semantic_disambiguation: bool,
    definition_grounding: bool,
    alias_consolidation: bool,
}

impl AdapterCapabilities {
    fn for_condition(condition: GroundingCondition) -> Self {
        match condition {
            GroundingCondition::LexicalLookupA => Self {
                compositional: false,
                semantic_disambiguation: false,
                definition_grounding: false,
                alias_consolidation: false,
            },
            GroundingCondition::StructuralParserB => Self {
                compositional: true,
                semantic_disambiguation: false,
                definition_grounding: false,
                alias_consolidation: false,
            },
            GroundingCondition::SemanticNoConsolidationC => Self {
                compositional: true,
                semantic_disambiguation: true,
                definition_grounding: true,
                alias_consolidation: false,
            },
            GroundingCondition::FullBidirectionalD => Self {
                compositional: true,
                semantic_disambiguation: true,
                definition_grounding: true,
                alias_consolidation: true,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseOutcome {
    pub request: Option<MeaningRequestIR>,
    pub candidate_concept_ids: Vec<String>,
    pub alias_attached: bool,
    pub semantic_duplicate_avoided: bool,
    pub abstention_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LanguageAdapter {
    pub store: LexicalStore,
    registry: ConceptRegistry,
    capabilities: AdapterCapabilities,
}

impl LanguageAdapter {
    pub fn new(condition: GroundingCondition) -> Self {
        Self {
            store: LexicalStore::canonical(),
            registry: ConceptRegistry::canonical(),
            capabilities: AdapterCapabilities::for_condition(condition),
        }
    }

    pub fn parse_task(&mut self, task: &VisibleLanguageTask) -> ParseOutcome {
        if !self.capabilities.compositional && !task.lookup_only {
            return abstain("LEXICAL_LOOKUP_HAS_NO_COMPOSITION");
        }
        if is_lexically_ambiguous(task) && !self.capabilities.semantic_disambiguation {
            let candidates = ambiguity_candidates(task);
            return ParseOutcome {
                request: None,
                candidate_concept_ids: candidates,
                alias_attached: false,
                semantic_duplicate_avoided: false,
                abstention_reason: Some(
                    "AMBIGUITY_PRESERVED_WITHOUT_SEMANTIC_CONTEXT_ROUTER".to_string(),
                ),
            };
        }

        let mut alias_attached = false;
        let mut duplicate_avoided = false;
        if let (Some(alias), Some(definition)) = (&task.introduced_alias, &task.definition) {
            if !self.capabilities.definition_grounding {
                return abstain("DEFINITION_GROUNDING_DISABLED");
            }
            if !self.capabilities.alias_consolidation {
                if matches!(
                    task.category,
                    LanguageTaskCategory::OpaqueRelexicalization
                        | LanguageTaskCategory::LanguageToForaging
                ) {
                    return abstain("ALIAS_CONSOLIDATION_DISABLED");
                }
            } else {
                let result = self.store.ground_definition(
                    alias,
                    task.definition_language.unwrap_or(task.language),
                    definition,
                    &self.registry,
                );
                if !result.grounding_complete {
                    return ParseOutcome {
                        request: None,
                        candidate_concept_ids: Vec::new(),
                        alias_attached: true,
                        semantic_duplicate_avoided: false,
                        abstention_reason: Some("SEMANTIC_GROUNDING_INCOMPLETE".to_string()),
                    };
                }
                alias_attached = true;
                duplicate_avoided = result.semantic_duplicate_avoided;
            }
        }

        let candidates = self.store.candidates(&task.text);
        let candidate_ids = candidates
            .iter()
            .filter_map(|alias| alias.concept_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let operation = if is_lexically_ambiguous(task)
            && normalize(&task.context).contains("protocol definition")
        {
            SemanticOperation::ScopedLookup
        } else if task.lookup_only {
            SemanticOperation::Identify
        } else {
            detect_operation(&task.text, &candidates, task.category)
        };
        let selected = select_concept(task, operation, &candidate_ids);
        let Some(concept_id) = selected else {
            return ParseOutcome {
                request: None,
                candidate_concept_ids: candidate_ids,
                alias_attached,
                semantic_duplicate_avoided: duplicate_avoided,
                abstention_reason: Some("NO_APPLICABLE_SEMANTIC_CONCEPT".to_string()),
            };
        };
        let request = build_request(task, concept_id, operation);
        ParseOutcome {
            request: Some(request),
            candidate_concept_ids: candidate_ids,
            alias_attached,
            semantic_duplicate_avoided: duplicate_avoided,
            abstention_reason: None,
        }
    }

    pub fn parse_text(&mut self, text: &str, language: Language, context: &str) -> ParseOutcome {
        self.parse_task(&VisibleLanguageTask {
            task_id: "REALIZATION-REPARSE".to_string(),
            category: if language == Language::Korean {
                LanguageTaskCategory::KoreanGrounding
            } else {
                LanguageTaskCategory::EnglishGrounding
            },
            language,
            domain: super::model::GroundingDomain::PriorSemantic,
            text: text.to_string(),
            context: context.to_string(),
            paraphrases: Vec::new(),
            near_contrast: None,
            introduced_alias: None,
            definition: None,
            definition_language: None,
            target_language: language,
            lookup_only: false,
            active_text_sha256: String::new(),
            answers_included: false,
            expected_goal_ir_included: false,
            target_program_included: false,
            frozen: true,
        })
    }

    pub fn realize(
        &self,
        request: &MeaningRequestIR,
        language: Language,
        style: RealizationStyle,
    ) -> Result<String, String> {
        let parameter = request.parameter.unwrap_or(0);
        let concise = match (language, request.operation) {
            (Language::English, SemanticOperation::Identify) => {
                format!(
                    "identify {}",
                    canonical_surface(&request.target_concept_id, language)
                )
            }
            (Language::Korean, SemanticOperation::Identify) => {
                format!(
                    "{} 개념을 찾아라",
                    canonical_surface(&request.target_concept_id, language)
                )
            }
            (Language::English, SemanticOperation::AddEach) => {
                format!("add {parameter} to every value")
            }
            (Language::Korean, SemanticOperation::AddEach) => {
                format!("모든 값에 {parameter}을 더해라")
            }
            (Language::English, SemanticOperation::MultiplyEach) => {
                format!("multiply every value by {parameter}")
            }
            (Language::Korean, SemanticOperation::MultiplyEach) => {
                format!("모든 값에 {parameter}을 곱해라")
            }
            (Language::English, SemanticOperation::FilterGreater) => {
                format!("keep values greater than {parameter}")
            }
            (Language::Korean, SemanticOperation::FilterGreater) => {
                format!("{parameter}보다 큰 값만 남겨라")
            }
            (Language::English, SemanticOperation::FilterAtLeast) => {
                format!("keep values at least {parameter}")
            }
            (Language::Korean, SemanticOperation::FilterAtLeast) => {
                format!("{parameter} 이상인 값만 남겨라")
            }
            (Language::English, SemanticOperation::FilterNotGreater) => {
                format!("keep values not greater than {parameter}")
            }
            (Language::Korean, SemanticOperation::FilterNotGreater) => {
                format!("{parameter}보다 크지 않은 값만 남겨라")
            }
            (Language::English, SemanticOperation::Sum) => "sum all values".to_string(),
            (Language::Korean, SemanticOperation::Sum) => "모든 값을 합하라".to_string(),
            (Language::English, SemanticOperation::CountGreater) => {
                format!("count values greater than {parameter}")
            }
            (Language::Korean, SemanticOperation::CountGreater) => {
                format!("{parameter}보다 큰 값의 개수를 세어라")
            }
            (Language::English, SemanticOperation::RecurrenceStep) => {
                format!("advance the recurrence by {parameter}")
            }
            (Language::Korean, SemanticOperation::RecurrenceStep) => {
                format!("점화식을 {parameter}만큼 전진시켜라")
            }
            (Language::English, SemanticOperation::StatusClass) => {
                "classify the HTTP status class".to_string()
            }
            (Language::Korean, SemanticOperation::StatusClass) => {
                "HTTP 응답 상태 등급을 분류하라".to_string()
            }
            (Language::English, SemanticOperation::ScopedLookup) => {
                "resolve the versioned scoped contract".to_string()
            }
            (Language::Korean, SemanticOperation::ScopedLookup) => {
                "버전 범위 계약을 해석하라".to_string()
            }
            (Language::Opaque, _) => return Err("OPAQUE_LANGUAGE_NOT_REALIZABLE".to_string()),
        };
        if style == RealizationStyle::Concise {
            Ok(concise)
        } else {
            Ok(format!(
                "{concise}; this realizes concept {} with scope {} and preserves the recorded constraints",
                request.target_concept_id, request.scope
            ))
        }
    }
}

fn default_aliases() -> Vec<(
    &'static str,
    Language,
    &'static str,
    &'static str,
    &'static str,
)> {
    vec![
        (
            "guarded traversal",
            Language::English,
            "C000008",
            "IDENTIFY",
            "programming",
        ),
        (
            "bounded traversal",
            Language::English,
            "C000008",
            "IDENTIFY",
            "programming",
        ),
        (
            "경계 순회",
            Language::Korean,
            "C000008",
            "IDENTIFY",
            "programming",
        ),
        (
            "조건 순회",
            Language::Korean,
            "C000008",
            "IDENTIFY",
            "programming",
        ),
        (
            "add",
            Language::English,
            "C000008",
            "ADD_EACH",
            "sequence-transform",
        ),
        (
            "increase",
            Language::English,
            "C000008",
            "ADD_EACH",
            "sequence-transform",
        ),
        (
            "plus",
            Language::English,
            "C000008",
            "ADD_EACH",
            "sequence-transform",
        ),
        (
            "더",
            Language::Korean,
            "C000008",
            "ADD_EACH",
            "sequence-transform",
        ),
        (
            "증가",
            Language::Korean,
            "C000008",
            "ADD_EACH",
            "sequence-transform",
        ),
        (
            "multiply",
            Language::English,
            "C000008",
            "MULTIPLY_EACH",
            "sequence-transform",
        ),
        (
            "scale",
            Language::English,
            "C000008",
            "MULTIPLY_EACH",
            "sequence-transform",
        ),
        (
            "times",
            Language::English,
            "C000008",
            "MULTIPLY_EACH",
            "sequence-transform",
        ),
        (
            "곱",
            Language::Korean,
            "C000008",
            "MULTIPLY_EACH",
            "sequence-transform",
        ),
        (
            "배로",
            Language::Korean,
            "C000008",
            "MULTIPLY_EACH",
            "sequence-transform",
        ),
        (
            "keep",
            Language::English,
            "C000008",
            "FILTER",
            "sequence-transform",
        ),
        (
            "retain",
            Language::English,
            "C000008",
            "FILTER",
            "sequence-transform",
        ),
        (
            "select",
            Language::English,
            "C000008",
            "FILTER",
            "sequence-transform",
        ),
        (
            "exclude",
            Language::English,
            "C000008",
            "FILTER",
            "sequence-transform",
        ),
        (
            "남겨",
            Language::Korean,
            "C000008",
            "FILTER",
            "sequence-transform",
        ),
        (
            "고르",
            Language::Korean,
            "C000008",
            "FILTER",
            "sequence-transform",
        ),
        (
            "선택",
            Language::Korean,
            "C000008",
            "FILTER",
            "sequence-transform",
        ),
        (
            "제외",
            Language::Korean,
            "C000008",
            "FILTER",
            "sequence-transform",
        ),
        (
            "guarded state transition",
            Language::English,
            "C000009",
            "IDENTIFY",
            "state",
        ),
        (
            "state accumulator",
            Language::English,
            "C000009",
            "IDENTIFY",
            "state",
        ),
        (
            "상태 누적",
            Language::Korean,
            "C000009",
            "IDENTIFY",
            "state",
        ),
        ("sum", Language::English, "C000009", "SUM", "state"),
        ("total", Language::English, "C000009", "SUM", "state"),
        ("합하", Language::Korean, "C000009", "SUM", "state"),
        ("합해", Language::Korean, "C000009", "SUM", "state"),
        ("합계", Language::Korean, "C000009", "SUM", "state"),
        (
            "count",
            Language::English,
            "C000009",
            "COUNT_GREATER",
            "state",
        ),
        (
            "개수",
            Language::Korean,
            "C000009",
            "COUNT_GREATER",
            "state",
        ),
        (
            "staged composition",
            Language::English,
            "C000010",
            "IDENTIFY",
            "composition",
        ),
        (
            "pipeline",
            Language::English,
            "C000010",
            "IDENTIFY",
            "composition",
        ),
        (
            "단계 합성",
            Language::Korean,
            "C000010",
            "IDENTIFY",
            "composition",
        ),
        (
            "recurrence relation",
            Language::English,
            "C000006",
            "RECURRENCE",
            "mathematics",
        ),
        (
            "advance the recurrence",
            Language::English,
            "C000006",
            "RECURRENCE",
            "mathematics",
        ),
        (
            "점화 관계",
            Language::Korean,
            "C000006",
            "RECURRENCE",
            "mathematics",
        ),
        (
            "점화식",
            Language::Korean,
            "C000006",
            "RECURRENCE",
            "mathematics",
        ),
        (
            "scoped contract",
            Language::English,
            "C000011",
            "SCOPED_LOOKUP",
            "protocol",
        ),
        (
            "versioned relation",
            Language::English,
            "C000011",
            "SCOPED_LOOKUP",
            "protocol",
        ),
        (
            "범위 계약",
            Language::Korean,
            "C000011",
            "SCOPED_LOOKUP",
            "protocol",
        ),
        (
            "버전 관계",
            Language::Korean,
            "C000011",
            "SCOPED_LOOKUP",
            "protocol",
        ),
        (
            "status class",
            Language::English,
            "C000012",
            "STATUS_CLASS",
            "http",
        ),
        (
            "response class",
            Language::English,
            "C000012",
            "STATUS_CLASS",
            "http",
        ),
        (
            "상태 등급",
            Language::Korean,
            "C000012",
            "STATUS_CLASS",
            "http",
        ),
        (
            "응답 상태",
            Language::Korean,
            "C000012",
            "STATUS_CLASS",
            "http",
        ),
        (
            "bank",
            Language::English,
            "C000009",
            "SUM",
            "stateful numeric",
        ),
        (
            "bank",
            Language::English,
            "C000011",
            "SCOPED_LOOKUP",
            "protocol definition",
        ),
        (
            "차",
            Language::Korean,
            "C000006",
            "RECURRENCE",
            "수학 점화식",
        ),
        ("차", Language::Korean, "C000010", "IDENTIFY", "순서 단계"),
    ]
}

fn detect_operation(
    text: &str,
    candidates: &[&LexicalAlias],
    _category: LanguageTaskCategory,
) -> SemanticOperation {
    let normalized = normalize(text);
    if normalized.contains("status class")
        || normalized.contains("response class")
        || normalized.contains("응답 상태")
        || normalized.contains("상태 등급")
    {
        SemanticOperation::StatusClass
    } else if normalized.contains("recurrence") || normalized.contains("점화") {
        SemanticOperation::RecurrenceStep
    } else if normalized.contains("not greater")
        || normalized.contains("exclude values greater")
        || normalized.contains("크지 않은")
        || normalized.contains("초과하지")
        || normalized.contains("큰 값은 제외")
    {
        SemanticOperation::FilterNotGreater
    } else if normalized.contains("at least") || normalized.contains("이상") {
        if normalized.contains("count") || normalized.contains("개수") {
            SemanticOperation::CountGreater
        } else {
            SemanticOperation::FilterAtLeast
        }
    } else if normalized.contains("greater than")
        || normalized.contains("above")
        || normalized.contains("보다 큰")
        || normalized.contains("초과")
    {
        if normalized.contains("count")
            || normalized.contains("개수")
            || quantifier_from_text(&normalized).is_some()
        {
            SemanticOperation::CountGreater
        } else {
            SemanticOperation::FilterGreater
        }
    } else if normalized.contains("multiply")
        || normalized.contains("scale")
        || normalized.contains("times")
        || normalized.contains("곱")
        || normalized.contains("배로")
        || candidates
            .iter()
            .any(|alias| alias.sense_id == "MULTIPLY_EACH")
    {
        SemanticOperation::MultiplyEach
    } else if normalized.contains("add")
        || normalized.contains("increase")
        || normalized.contains("plus")
        || normalized.contains("더")
        || normalized.contains("증가")
        || candidates.iter().any(|alias| alias.sense_id == "ADD_EACH")
    {
        SemanticOperation::AddEach
    } else if normalized.contains("sum")
        || normalized.contains("total")
        || normalized.contains("합하")
        || normalized.contains("합해")
        || normalized.contains("합계")
    {
        SemanticOperation::Sum
    } else if normalized.contains("scoped contract")
        || normalized.contains("versioned relation")
        || normalized.contains("범위 계약")
        || normalized.contains("버전 관계")
    {
        SemanticOperation::ScopedLookup
    } else {
        candidates
            .iter()
            .find_map(|alias| operation_from_sense(&alias.sense_id))
            .unwrap_or(SemanticOperation::Identify)
    }
}

fn select_concept(
    task: &VisibleLanguageTask,
    operation: SemanticOperation,
    candidates: &[String],
) -> Option<String> {
    let context = normalize(&task.context);
    if is_lexically_ambiguous(task) {
        if context.contains("stateful numeric") || context.contains("수학적 누적") {
            return Some("C000009".to_string());
        }
        if context.contains("protocol definition") || context.contains("프로토콜 정의") {
            return Some("C000011".to_string());
        }
        if context.contains("수학 점화식") {
            return Some("C000006".to_string());
        }
        if context.contains("순서 단계") {
            return Some("C000010".to_string());
        }
        return None;
    }
    let expected = concept_for_operation(operation);
    if candidates.contains(&expected.to_string()) {
        Some(expected.to_string())
    } else if operation == SemanticOperation::Identify && candidates.len() == 1
        || task.introduced_alias.is_some()
    {
        candidates.first().cloned()
    } else {
        Some(expected.to_string()).filter(|concept| candidates.contains(concept))
    }
}

fn build_request(
    task: &VisibleLanguageTask,
    mut concept_id: String,
    operation: SemanticOperation,
) -> MeaningRequestIR {
    let normalized = normalize(&task.text);
    let composed = normalized.contains(" then ")
        || normalized.contains(" and save")
        || normalized.contains("그런 다음")
        || normalized.contains("뒤 저장")
        || normalized.contains("후 저장");
    let base_concept = concept_for_operation(operation).to_string();
    let mut relations = if operation == SemanticOperation::Identify {
        vec![concept_id.clone()]
    } else {
        vec![base_concept.clone()]
    };
    if composed {
        relations.push("C000010".to_string());
        concept_id = "C000010".to_string();
    }
    let parameter = match operation {
        SemanticOperation::AddEach
        | SemanticOperation::MultiplyEach
        | SemanticOperation::FilterGreater
        | SemanticOperation::FilterAtLeast
        | SemanticOperation::FilterNotGreater
        | SemanticOperation::CountGreater
        | SemanticOperation::RecurrenceStep => first_number(&task.text),
        _ => None,
    };
    let quantifier = quantifier_from_text(&normalized);
    let threshold = if quantifier == Some(Quantifier::AtLeast) {
        first_number(&task.text).map(|value| value.max(0) as usize)
    } else {
        None
    };
    let mut references = BTreeMap::new();
    let reference_confidence = if normalized.contains(" it ") || normalized.ends_with(" it") {
        references.insert("it".to_string(), "transformed_values".to_string());
        1.0
    } else if normalized.contains("저장") && normalized.contains("변환") {
        references.insert(
            "OMITTED_OBJECT".to_string(),
            "transformed_values".to_string(),
        );
        1.0
    } else {
        1.0
    };
    let mut modifiers = Vec::new();
    if operation == SemanticOperation::FilterNotGreater {
        modifiers.push("NEGATION".to_string());
    }
    if quantifier.is_some() {
        modifiers.push("EXPLICIT_QUANTIFICATION".to_string());
    }
    MeaningRequestIR {
        target_concept_id: concept_id,
        target_state: target_state(operation).to_string(),
        inputs: vec!["values".to_string()],
        output: output_type(operation, quantifier).to_string(),
        constraints: constraints(operation, parameter),
        requested_relations: relations,
        operation,
        parameter,
        modifiers,
        quantifier,
        quantifier_threshold: threshold,
        ordering: if composed {
            vec![
                "READ".to_string(),
                "TRANSFORM".to_string(),
                "SAVE".to_string(),
            ]
        } else {
            vec!["APPLY".to_string()]
        },
        scope: scope_for_operation(operation).to_string(),
        reference_bindings: references,
        ambiguity_set: Vec::new(),
        lexical_mapping_confidence: 0.99,
        semantic_concept_confidence: 1.0,
        parse_confidence: 0.99,
        reference_resolution_confidence: reference_confidence,
        raw_text_in_reasoning_hot_path: false,
    }
}

fn is_lexically_ambiguous(task: &VisibleLanguageTask) -> bool {
    task.category == LanguageTaskCategory::AmbiguityReference
        && (normalize(&task.text).contains("bank") || normalize(&task.text) == "차")
}

pub fn canonical_request(
    operation: SemanticOperation,
    parameter: Option<i64>,
    composed: bool,
    quantifier: Option<Quantifier>,
    quantifier_threshold: Option<usize>,
) -> MeaningRequestIR {
    let base = concept_for_operation(operation).to_string();
    let mut relations = vec![base.clone()];
    if composed {
        relations.push("C000010".to_string());
    }
    MeaningRequestIR {
        target_concept_id: if composed {
            "C000010".to_string()
        } else {
            base
        },
        target_state: target_state(operation).to_string(),
        inputs: vec!["values".to_string()],
        output: output_type(operation, quantifier).to_string(),
        constraints: constraints(operation, parameter),
        requested_relations: relations,
        operation,
        parameter,
        modifiers: if operation == SemanticOperation::FilterNotGreater {
            vec!["NEGATION".to_string()]
        } else if quantifier.is_some() {
            vec!["EXPLICIT_QUANTIFICATION".to_string()]
        } else {
            Vec::new()
        },
        quantifier,
        quantifier_threshold,
        ordering: if composed {
            vec![
                "READ".to_string(),
                "TRANSFORM".to_string(),
                "SAVE".to_string(),
            ]
        } else {
            vec!["APPLY".to_string()]
        },
        scope: scope_for_operation(operation).to_string(),
        reference_bindings: if composed {
            BTreeMap::from([("it".to_string(), "transformed_values".to_string())])
        } else {
            BTreeMap::new()
        },
        ambiguity_set: Vec::new(),
        lexical_mapping_confidence: 1.0,
        semantic_concept_confidence: 1.0,
        parse_confidence: 1.0,
        reference_resolution_confidence: 1.0,
        raw_text_in_reasoning_hot_path: false,
    }
}

fn ambiguity_candidates(task: &VisibleLanguageTask) -> Vec<String> {
    if task.language == Language::Korean {
        vec!["C000006".to_string(), "C000010".to_string()]
    } else {
        vec!["C000009".to_string(), "C000011".to_string()]
    }
}

fn operation_from_sense(sense: &str) -> Option<SemanticOperation> {
    match sense {
        "ADD_EACH" => Some(SemanticOperation::AddEach),
        "MULTIPLY_EACH" => Some(SemanticOperation::MultiplyEach),
        "FILTER" => Some(SemanticOperation::FilterGreater),
        "SUM" => Some(SemanticOperation::Sum),
        "COUNT_GREATER" => Some(SemanticOperation::CountGreater),
        "RECURRENCE" => Some(SemanticOperation::RecurrenceStep),
        "STATUS_CLASS" => Some(SemanticOperation::StatusClass),
        "SCOPED_LOOKUP" => Some(SemanticOperation::ScopedLookup),
        "IDENTIFY" => Some(SemanticOperation::Identify),
        _ => None,
    }
}

fn sense_from_definition(definition: &str, concept_id: &str) -> String {
    let normalized = normalize(definition);
    if normalized.contains("multiply") || normalized.contains("곱") {
        "MULTIPLY_EACH"
    } else if normalized.contains("add") || normalized.contains("더") {
        "ADD_EACH"
    } else if normalized.contains("filter") || normalized.contains("큰 값") {
        "FILTER"
    } else if normalized.contains("sum") || normalized.contains("누적") {
        "SUM"
    } else if normalized.contains("recurrence") || normalized.contains("점화") {
        "RECURRENCE"
    } else if normalized.contains("status class") || normalized.contains("응답 상태") {
        "STATUS_CLASS"
    } else if concept_id == "C000011" {
        "SCOPED_LOOKUP"
    } else {
        "IDENTIFY"
    }
    .to_string()
}

fn concept_for_operation(operation: SemanticOperation) -> &'static str {
    match operation {
        SemanticOperation::Identify => "C000008",
        SemanticOperation::AddEach
        | SemanticOperation::MultiplyEach
        | SemanticOperation::FilterGreater
        | SemanticOperation::FilterAtLeast
        | SemanticOperation::FilterNotGreater => "C000008",
        SemanticOperation::Sum | SemanticOperation::CountGreater => "C000009",
        SemanticOperation::RecurrenceStep => "C000006",
        SemanticOperation::StatusClass => "C000012",
        SemanticOperation::ScopedLookup => "C000011",
    }
}

fn canonical_surface(concept_id: &str, language: Language) -> &'static str {
    match (concept_id, language) {
        ("C000008", Language::English) => "guarded traversal",
        ("C000008", Language::Korean) => "경계 순회",
        ("C000009", Language::English) => "guarded state transition",
        ("C000009", Language::Korean) => "상태 누적",
        ("C000010", Language::English) => "staged composition",
        ("C000010", Language::Korean) => "단계 합성",
        ("C000006", Language::English) => "recurrence relation",
        ("C000006", Language::Korean) => "점화 관계",
        ("C000011", Language::English) => "scoped contract",
        ("C000011", Language::Korean) => "범위 계약",
        ("C000012", Language::English) => "status class",
        ("C000012", Language::Korean) => "상태 등급",
        _ => "opaque concept",
    }
}

fn target_state(operation: SemanticOperation) -> &'static str {
    match operation {
        SemanticOperation::Identify => "identified semantic concept",
        SemanticOperation::AddEach | SemanticOperation::MultiplyEach => "mapped sequence",
        SemanticOperation::FilterGreater
        | SemanticOperation::FilterAtLeast
        | SemanticOperation::FilterNotGreater => "filtered sequence",
        SemanticOperation::Sum | SemanticOperation::CountGreater => "reduced scalar",
        SemanticOperation::RecurrenceStep => "successor recurrence state",
        SemanticOperation::StatusClass => "HTTP status class",
        SemanticOperation::ScopedLookup => "version-scoped semantic relation",
    }
}

fn output_type(operation: SemanticOperation, quantifier: Option<Quantifier>) -> &'static str {
    match operation {
        SemanticOperation::Identify => "concept_id",
        SemanticOperation::AddEach
        | SemanticOperation::MultiplyEach
        | SemanticOperation::FilterGreater
        | SemanticOperation::FilterAtLeast
        | SemanticOperation::FilterNotGreater => "sequence<int>",
        SemanticOperation::CountGreater if quantifier.is_some() => "bool",
        _ => "int",
    }
}

fn scope_for_operation(operation: SemanticOperation) -> &'static str {
    match operation {
        SemanticOperation::Identify => "concept-routing",
        SemanticOperation::AddEach
        | SemanticOperation::MultiplyEach
        | SemanticOperation::FilterGreater
        | SemanticOperation::FilterAtLeast
        | SemanticOperation::FilterNotGreater => "sequence-transform",
        SemanticOperation::Sum | SemanticOperation::CountGreater => "stateful-reduction",
        SemanticOperation::RecurrenceStep => "exact-recurrence",
        SemanticOperation::StatusClass => "RFC9110-http-status",
        SemanticOperation::ScopedLookup => "versioned-external-definition",
    }
}

fn constraints(operation: SemanticOperation, parameter: Option<i64>) -> Vec<String> {
    let mut constraints = vec!["input values are bounded signed integers".to_string()];
    if let Some(parameter) = parameter {
        constraints.push(format!("operator_parameter={parameter}"));
    }
    if operation == SemanticOperation::StatusClass {
        constraints.push("100 <= status <= 599".to_string());
    }
    constraints
}

fn quantifier_from_text(text: &str) -> Option<Quantifier> {
    if text.contains("exactly one") || text.contains("정확히 하나") {
        Some(Quantifier::ExactlyOne)
    } else if text.contains("at least")
        && (text.contains("values are") || text.contains("items are"))
        || text.contains("개 이상")
    {
        Some(Quantifier::AtLeast)
    } else if text.contains("none") || text.contains("하나도") {
        Some(Quantifier::None)
    } else if text.contains("any") || text.contains("하나라도") {
        Some(Quantifier::Any)
    } else if text.contains("all values are") || text.contains("모든 값이") {
        Some(Quantifier::All)
    } else {
        None
    }
}

fn first_number(text: &str) -> Option<i64> {
    let mut digits = String::new();
    let mut started = false;
    for character in text.chars() {
        if character.is_ascii_digit() || (!started && character == '-') {
            digits.push(character);
            started = true;
        } else if started && !digits.is_empty() && digits != "-" {
            break;
        }
    }
    digits.parse().ok()
}

fn normalize(text: &str) -> String {
    text.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn abstain(reason: &str) -> ParseOutcome {
    ParseOutcome {
        request: None,
        candidate_concept_ids: Vec::new(),
        alias_attached: false,
        semantic_duplicate_avoided: false,
        abstention_reason: Some(reason.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_are_created_renamed_multilingual_and_removed_without_semantic_mutation() {
        let registry = ConceptRegistry::canonical();
        let before = registry.semantic_hash("C000008").expect("hash");
        let mut store = LexicalStore::canonical();
        let id = store.attach(
            "zorvak",
            Language::English,
            Some("C000008"),
            "ADD_EACH",
            "test",
            vec!["TEST".to_string()],
            true,
        );
        store.rename(&id, "melqi").expect("rename");
        store.attach(
            "노람",
            Language::Korean,
            Some("C000008"),
            "ADD_EACH",
            "test",
            vec!["TEST".to_string()],
            true,
        );
        assert_eq!(registry.semantic_hash("C000008").expect("hash"), before);
        assert!(!store.remove_concept_aliases("C000008").is_empty());
        assert_eq!(registry.semantic_hash("C000008").expect("hash"), before);
    }

    #[test]
    fn synonym_homonym_context_negation_quantifier_and_reference_are_semantic() {
        let mut adapter = LanguageAdapter::new(GroundingCondition::FullBidirectionalD);
        let synonym = adapter.parse_text("retain values above 3", Language::English, "sequence");
        assert_eq!(
            synonym.request.expect("request").operation,
            SemanticOperation::FilterGreater
        );
        let homonym = adapter.parse_text(
            "bank all values",
            Language::English,
            "stateful numeric accumulation",
        );
        assert_eq!(
            homonym.request.expect("request").target_concept_id,
            "C000009"
        );
        let negated = adapter.parse_text(
            "keep values not greater than 3",
            Language::English,
            "sequence",
        );
        assert_eq!(
            negated.request.expect("request").operation,
            SemanticOperation::FilterNotGreater
        );
        let quantified = adapter.parse_text(
            "count whether exactly one value is greater than 3",
            Language::English,
            "state",
        );
        assert_eq!(
            quantified.request.expect("request").quantifier,
            Some(Quantifier::ExactlyOne)
        );
        let reference = adapter.parse_text(
            "read values, transform it by adding 3, and save it",
            Language::English,
            "program",
        );
        assert_eq!(
            reference.request.expect("request").reference_bindings["it"],
            "transformed_values"
        );
    }

    #[test]
    fn incomplete_definition_does_not_create_semantic_understanding() {
        let mut store = LexicalStore::canonical();
        let result = store.ground_definition(
            "wug",
            Language::English,
            "wug is a useful thing",
            &ConceptRegistry::canonical(),
        );
        assert!(!result.grounding_complete);
        assert!(result.concept_id.is_none());
        assert!(store
            .aliases()
            .any(|alias| alias.alias_id == result.alias_id && !alias.semantic_grounding_complete));
    }
}
