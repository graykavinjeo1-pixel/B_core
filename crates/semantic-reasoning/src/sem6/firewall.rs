use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use crate::sem5::{
    ir::eval_scalar,
    model::{ProgramType, Value},
};

use super::model::{
    CompiledSemanticFact, ExtractionRecord, ForagingRequest, GapClass, KnowledgeGapEvent,
    QueryCategory, SourceAuthority, SourceConflict, SourceDocument, SpanClass,
    VisibleKnowledgeTask,
};

pub const REQUEST_BUDGET_PER_TASK: usize = 2;
pub const CONTENT_BUDGET_BYTES_PER_TASK: usize = 8_192;

#[derive(Debug, Clone)]
pub struct ForagingFirewall {
    known_aliases: BTreeSet<String>,
    next_gap: usize,
    next_request: usize,
}

impl ForagingFirewall {
    pub fn new(known_aliases: impl IntoIterator<Item = String>) -> Self {
        Self {
            known_aliases: known_aliases.into_iter().collect(),
            next_gap: 0,
            next_request: 0,
        }
    }

    pub fn detect_gap(&mut self, task: &VisibleKnowledgeTask) -> Option<KnowledgeGapEvent> {
        if self.known_aliases.contains(&task.unknown_symbol) {
            return None;
        }
        let gap_class = match task.domain {
            super::model::KnowledgeDomain::ProgrammingApi => GapClass::UnknownApi,
            super::model::KnowledgeDomain::MathematicalFormal => GapClass::UnknownRelation,
            super::model::KnowledgeDomain::ProtocolSpecification => GapClass::UnknownProtocol,
            super::model::KnowledgeDomain::AmbiguousConflict => GapClass::AmbiguousDefinition,
            super::model::KnowledgeDomain::AdversarialContamination => GapClass::UnknownSymbol,
        };
        let event = KnowledgeGapEvent {
            event_id: format!("GAP-{:05}", self.next_gap),
            task_id: task.task_id.clone(),
            gap_class,
            unknown: task.unknown_symbol.clone(),
            existing_concepts_insufficient_because:
                "no validated semantic fact matches the requested alias, scope, and version"
                    .to_string(),
            minimum_information_needed:
                "typed signature, normative relation, preconditions, and applicable version"
                    .to_string(),
            external_retrieval_necessary: true,
            confidence: 1.0,
        };
        self.next_gap += 1;
        Some(event)
    }

    pub fn propose_request(
        &mut self,
        task: &VisibleKnowledgeTask,
        category: QueryCategory,
    ) -> ForagingRequest {
        let query = format!(
            "official definition and contract of {} scope {} version {}",
            task.unknown_symbol, task.required_scope, task.required_version
        );
        let (exact_task_leak, near_task_similarity) = query_leakage(&query, task);
        let classification_allowed = category_allowed(category);
        let sanitized = classification_allowed
            && !exact_task_leak
            && near_task_similarity < 0.72
            && !contains_solution_language(&query);
        let rejection_reason = (!sanitized).then(|| {
            if !classification_allowed {
                "FORBIDDEN_REQUEST_CATEGORY"
            } else if exact_task_leak {
                "EXACT_ACTIVE_TASK_LEAK"
            } else if near_task_similarity >= 0.72 {
                "NEAR_ACTIVE_TASK_LEAK"
            } else {
                "SOLUTION_LANGUAGE"
            }
            .to_string()
        });
        let request = ForagingRequest {
            request_id: format!("REQ-{:05}", self.next_request),
            task_id: task.task_id.clone(),
            category,
            query,
            requested_symbol: task.unknown_symbol.clone(),
            requested_scope: task.required_scope.clone(),
            requested_version: task.required_version.clone(),
            classification_allowed,
            exact_task_leak,
            near_task_similarity,
            sanitized,
            executed: false,
            rejection_reason,
            request_budget: REQUEST_BUDGET_PER_TASK,
            content_budget_bytes: CONTENT_BUDGET_BYTES_PER_TASK,
        };
        self.next_request += 1;
        request
    }

    pub fn classify_explicit_request(
        &self,
        task: &VisibleKnowledgeTask,
        category: QueryCategory,
        query: &str,
    ) -> ForagingRequest {
        let (exact_task_leak, near_task_similarity) = query_leakage(query, task);
        let classification_allowed = category_allowed(category);
        let sanitized = classification_allowed
            && !exact_task_leak
            && near_task_similarity < 0.72
            && !contains_solution_language(query);
        ForagingRequest {
            request_id: "CLASSIFICATION-PROBE".to_string(),
            task_id: task.task_id.clone(),
            category,
            query: query.to_string(),
            requested_symbol: task.unknown_symbol.clone(),
            requested_scope: task.required_scope.clone(),
            requested_version: task.required_version.clone(),
            classification_allowed,
            exact_task_leak,
            near_task_similarity,
            sanitized,
            executed: false,
            rejection_reason: (!sanitized).then(|| "REQUEST_REJECTED".to_string()),
            request_budget: REQUEST_BUDGET_PER_TASK,
            content_budget_bytes: CONTENT_BUDGET_BYTES_PER_TASK,
        }
    }
}

pub const fn category_allowed(category: QueryCategory) -> bool {
    matches!(
        category,
        QueryCategory::DefineSymbol
            | QueryCategory::DefineTerm
            | QueryCategory::GetTypeSignature
            | QueryCategory::GetApiContract
            | QueryCategory::GetPreconditions
            | QueryCategory::GetPostconditions
            | QueryCategory::GetStandardSemantics
            | QueryCategory::GetFormalRule
            | QueryCategory::GetDataFormatSpec
            | QueryCategory::GetProtocolFieldMeaning
    )
}

fn contains_solution_language(query: &str) -> bool {
    let normalized = query.to_ascii_lowercase();
    [
        "solution",
        "worked example",
        "reference implementation",
        "benchmark answer",
        "solve task",
        "target formula",
        "error plus",
    ]
    .iter()
    .any(|term| normalized.contains(term))
}

fn query_leakage(query: &str, task: &VisibleKnowledgeTask) -> (bool, f64) {
    let query_normalized = normalized_text(query);
    let task_normalized = normalized_text(&task.active_problem);
    let exact = query_normalized.contains(&task_normalized)
        || query_normalized.contains(&normalized_text(&task.task_id));
    let query_tokens = token_set(&query_normalized);
    let task_tokens = token_set(&task_normalized);
    let intersection = query_tokens.intersection(&task_tokens).count();
    let union = query_tokens.union(&task_tokens).count().max(1);
    (exact, intersection as f64 / union as f64)
}

fn normalized_text(text: &str) -> String {
    text.to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn token_set(text: &str) -> BTreeSet<&str> {
    text.split_whitespace().collect()
}

pub fn rank_sources<'a>(
    documents: impl IntoIterator<Item = &'a SourceDocument>,
) -> Vec<&'a SourceDocument> {
    let mut ranked = documents.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .authority
            .rank()
            .cmp(&left.authority.rank())
            .then_with(|| right.source_version.cmp(&left.source_version))
            .then_with(|| left.source_id.cmp(&right.source_id))
    });
    ranked
}

pub fn extract_document(
    task: &VisibleKnowledgeTask,
    document: &SourceDocument,
) -> (ExtractionRecord, Vec<CompiledSemanticFact>) {
    let mut accepted = Vec::new();
    let mut record = ExtractionRecord {
        task_id: task.task_id.clone(),
        source_id: document.source_id.clone(),
        spans_seen: document.spans.len(),
        definition_spans_accepted: 0,
        facts_extracted: 0,
        facts_accepted: 0,
        facts_rejected: 0,
        example_spans_quarantined: 0,
        implementation_spans_quarantined: 0,
        solution_like_spans_quarantined: 0,
        injection_like_spans_detected: 0,
        control_instructions_executed: 0,
        accepted_fact_ids: Vec::new(),
    };
    for span in &document.spans {
        if span.injection_like || detects_document_instruction(&span.text) {
            record.injection_like_spans_detected += 1;
        }
        match span.class {
            SpanClass::Example => record.example_spans_quarantined += 1,
            SpanClass::Implementation => record.implementation_spans_quarantined += 1,
            SpanClass::SolutionLike => record.solution_like_spans_quarantined += 1,
            _ => {}
        }
        let Some(payload) = &span.fact else {
            continue;
        };
        record.facts_extracted += 1;
        let scope_matches = payload.scope == task.required_scope;
        let version_matches = version_applies(
            &task.required_version,
            &payload.source_version,
            &payload.applicability_version_range,
        );
        let importable = span.class.importable()
            && !document.search_snippet_only
            && document.authority != SourceAuthority::Untrusted
            && payload.symbol == task.unknown_symbol
            && scope_matches
            && version_matches;
        if importable {
            record.definition_spans_accepted += 1;
            let fact_id = fact_id(&document.source_id, &span.span_id, payload);
            let validation = validate_payload(payload);
            let fact = CompiledSemanticFact {
                fact_id: fact_id.clone(),
                opaque_concept_id: format!("EXT-{}", &fact_id[..12]),
                lexical_alias: payload.symbol.clone(),
                signature_inputs: payload.signature_inputs.clone(),
                signature_output: payload.signature_output.clone(),
                formal_body: payload.formal_body.clone(),
                preconditions: payload.preconditions.clone(),
                postconditions: payload.postconditions.clone(),
                invariants: payload.invariants.clone(),
                effects: payload.effects.clone(),
                source_ids: vec![document.source_id.clone()],
                source_versions: vec![payload.source_version.clone()],
                scope: payload.scope.clone(),
                applicability_version_range: payload.applicability_version_range.clone(),
                confidence: authority_confidence(document.authority),
                agreement_count: 1,
                conflict: false,
                type_check_passed: validation.0,
                generated_probe_count: validation.1,
                generated_probes_passed: validation.2,
                normative_consistency_passed: validation.0 && validation.1 == validation.2,
                validation_passed: validation.0 && validation.1 == validation.2,
                state: "EXTERNAL_CANDIDATE".to_string(),
            };
            if fact.validation_passed {
                record.facts_accepted += 1;
                record.accepted_fact_ids.push(fact_id);
                accepted.push(fact);
            } else {
                record.facts_rejected += 1;
            }
        } else {
            record.facts_rejected += 1;
        }
    }
    (record, accepted)
}

fn detects_document_instruction(text: &str) -> bool {
    let normalized = text.to_ascii_lowercase();
    [
        "ignore prior instructions",
        "run this command",
        "upload data",
        "change your system",
        "override the firewall",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn version_applies(required: &str, source: &str, range: &str) -> bool {
    required == source || range == "*" || range.split(',').any(|version| version.trim() == required)
}

fn authority_confidence(authority: SourceAuthority) -> f64 {
    match authority {
        SourceAuthority::OfficialStandard | SourceAuthority::OfficialDocumentation => 0.99,
        SourceAuthority::OriginalPaper => 0.97,
        SourceAuthority::InstitutionalReference => 0.95,
        SourceAuthority::SecondarySource => 0.75,
        SourceAuthority::Untrusted => 0.0,
    }
}

fn validate_payload(payload: &super::model::SemanticFactPayload) -> (bool, usize, usize) {
    if payload.signature_inputs.is_empty()
        || payload.signature_inputs.len() > 3
        || payload
            .signature_inputs
            .iter()
            .any(|value_type| value_type != &ProgramType::Int)
        || !matches!(
            payload.signature_output,
            ProgramType::Int | ProgramType::Bool
        )
    {
        return (false, 0, 0);
    }
    let api_map = BTreeMap::new();
    let probes = [
        vec![Value::Int(0), Value::Int(1), Value::Int(2)],
        vec![Value::Int(3), Value::Int(2), Value::Int(1)],
        vec![Value::Int(9), Value::Int(4), Value::Int(2)],
        vec![Value::Int(17), Value::Int(5), Value::Int(3)],
    ];
    let passed = probes
        .iter()
        .filter(|probe| {
            eval_scalar(
                &payload.formal_body,
                &probe[..payload.signature_inputs.len()],
                &api_map,
            )
            .is_ok_and(|value| value.program_type() == payload.signature_output)
        })
        .count();
    (passed == probes.len(), probes.len(), passed)
}

fn fact_id(source_id: &str, span_id: &str, payload: &super::model::SemanticFactPayload) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source_id.as_bytes());
    hasher.update(span_id.as_bytes());
    hasher.update(serde_json::to_vec(payload).expect("payload serialization"));
    format!("{:x}", hasher.finalize())
}

pub fn consolidate_facts(
    task: &VisibleKnowledgeTask,
    facts: Vec<(CompiledSemanticFact, SourceAuthority)>,
) -> (Option<CompiledSemanticFact>, Option<SourceConflict>) {
    if facts.is_empty() {
        return (None, None);
    }
    let mut ranked = facts;
    ranked.sort_by(|(left, left_authority), (right, right_authority)| {
        right_authority
            .rank()
            .cmp(&left_authority.rank())
            .then_with(|| {
                let right_exact = right.source_versions.contains(&task.required_version);
                let left_exact = left.source_versions.contains(&task.required_version);
                right_exact.cmp(&left_exact)
            })
    });
    let distinct = ranked
        .iter()
        .map(|(fact, _)| serde_json::to_string(&fact.formal_body).expect("expression"))
        .collect::<BTreeSet<_>>();
    if distinct.len() == 1 {
        let mut selected = ranked[0].0.clone();
        selected.agreement_count = ranked.len();
        selected.source_ids = ranked
            .iter()
            .flat_map(|(fact, _)| fact.source_ids.clone())
            .collect();
        selected.source_versions = ranked
            .iter()
            .flat_map(|(fact, _)| fact.source_versions.clone())
            .collect();
        selected.confidence = (selected.confidence + 0.01).min(1.0);
        return (Some(selected), None);
    }
    let winner = ranked[0].0.clone();
    let top_authority = ranked[0].1.rank();
    let second_authority = ranked.get(1).map(|entry| entry.1.rank()).unwrap_or(0);
    let exact_version = winner.source_versions.contains(&task.required_version);
    let resolved = top_authority > second_authority || exact_version;
    let conflict = SourceConflict {
        conflict_id: format!("CONFLICT-{}", task.task_id),
        symbol: task.unknown_symbol.clone(),
        source_ids: ranked
            .iter()
            .flat_map(|(fact, _)| fact.source_ids.clone())
            .collect(),
        disagreement: "formal relations differ for the same lexical alias".to_string(),
        authority_compared: true,
        versions_compared: true,
        scopes_compared: true,
        resolution: if resolved {
            "selected highest-authority fact applicable to required version and scope"
        } else {
            "unresolved; all hypotheses retained and solver abstains"
        }
        .to_string(),
        resolved,
        unresolved_hypotheses_preserved: usize::from(!resolved) * ranked.len(),
    };
    if resolved {
        let mut selected = winner;
        selected.conflict = true;
        (Some(selected), Some(conflict))
    } else {
        (None, Some(conflict))
    }
}

#[cfg(test)]
mod tests {
    use crate::sem5::model::{BinaryOperator, ScalarExpression};

    use super::*;
    use crate::sem6::model::{
        ForagingEnvironment, KnowledgeDomain, SemanticFactPayload, SourceSpan,
    };

    fn task() -> VisibleKnowledgeTask {
        VisibleKnowledgeTask {
            task_id: "TASK-SECRET-183".to_string(),
            environment: ForagingEnvironment::SealedCorpusA,
            domain: KnowledgeDomain::ProgrammingApi,
            active_problem: "construct output for private benchmark parser using symbol qx"
                .to_string(),
            active_problem_sha256: "hash".to_string(),
            unknown_symbol: "qx".to_string(),
            required_version: "2".to_string(),
            required_scope: "lib-a".to_string(),
            input_types: vec![ProgramType::Int],
            output_type: ProgramType::Int,
            demonstrations: Vec::new(),
            target_solution_included: false,
            intent_frozen: true,
        }
    }

    fn payload(offset: i64, version: &str) -> SemanticFactPayload {
        SemanticFactPayload {
            symbol: "qx".to_string(),
            signature_inputs: vec![ProgramType::Int],
            signature_output: ProgramType::Int,
            formal_body: ScalarExpression::Binary {
                operator: BinaryOperator::Add,
                left: Box::new(ScalarExpression::Argument { index: 0 }),
                right: Box::new(ScalarExpression::Constant { value: offset }),
            },
            preconditions: vec!["bounded integer".to_string()],
            postconditions: vec!["formal relation".to_string()],
            invariants: Vec::new(),
            effects: vec!["PURE".to_string()],
            scope: "lib-a".to_string(),
            source_version: version.to_string(),
            applicability_version_range: version.to_string(),
        }
    }

    fn document(
        id: &str,
        authority: SourceAuthority,
        version: &str,
        offset: i64,
    ) -> SourceDocument {
        SourceDocument {
            source_id: id.to_string(),
            title: "definition".to_string(),
            source_identifier: id.to_string(),
            url: None,
            authority,
            source_version: version.to_string(),
            scope: "lib-a".to_string(),
            retrieval_time_utc: "SEALED".to_string(),
            retrieved_bytes: 100,
            content_sha256: "hash".to_string(),
            live_retrieval: false,
            search_snippet_only: false,
            spans: vec![SourceSpan {
                span_id: "s1".to_string(),
                class: SpanClass::NormativeRule,
                text: "The result is the input plus the declared offset.".to_string(),
                fact: Some(payload(offset, version)),
                injection_like: false,
            }],
        }
    }

    #[test]
    fn gap_detection_queries_only_unknown_aliases() {
        let task = task();
        let mut firewall = ForagingFirewall::new(Vec::new());
        assert!(firewall.detect_gap(&task).is_some());
        let mut known = ForagingFirewall::new(vec!["qx".to_string()]);
        assert!(known.detect_gap(&task).is_none());
    }

    #[test]
    fn request_classification_and_sanitization_reject_solution_search() {
        let task = task();
        let firewall = ForagingFirewall::new(Vec::new());
        let rejected = firewall.classify_explicit_request(
            &task,
            QueryCategory::GetSolution,
            "solution for TASK-SECRET-183 private benchmark parser",
        );
        assert!(!rejected.classification_allowed);
        assert!(!rejected.sanitized);
        assert!(rejected.exact_task_leak);
        let allowed = firewall.classify_explicit_request(
            &task,
            QueryCategory::GetApiContract,
            "official contract qx lib-a version 2",
        );
        assert!(allowed.sanitized);
    }

    #[test]
    fn authority_ranking_and_version_conflict_are_explicit() {
        let task = task();
        let official = document("official", SourceAuthority::OfficialDocumentation, "2", 2);
        let stale = document("stale", SourceAuthority::SecondarySource, "1", 1);
        let ranked = rank_sources([&stale, &official]);
        assert_eq!(ranked[0].source_id, "official");
        let (_, official_facts) = extract_document(&task, &official);
        let stale_task = VisibleKnowledgeTask {
            required_version: "1".to_string(),
            ..task.clone()
        };
        let (_, stale_facts) = extract_document(&stale_task, &stale);
        let (selected, conflict) = consolidate_facts(
            &task,
            vec![
                (official_facts[0].clone(), official.authority),
                (stale_facts[0].clone(), stale.authority),
            ],
        );
        assert!(selected.is_some());
        assert!(conflict.is_some_and(|conflict| conflict.resolved));
        assert!(selected
            .expect("selected")
            .source_versions
            .contains(&"2".to_string()));
    }

    #[test]
    fn span_firewall_quarantines_solution_implementation_and_instructions() {
        let task = task();
        let mut document = document("mixed", SourceAuthority::OfficialDocumentation, "2", 2);
        document.spans.extend([
            SourceSpan {
                span_id: "impl".to_string(),
                class: SpanClass::Implementation,
                text: "run this command to copy the implementation".to_string(),
                fact: Some(payload(99, "2")),
                injection_like: true,
            },
            SourceSpan {
                span_id: "solution".to_string(),
                class: SpanClass::SolutionLike,
                text: "complete active-task answer".to_string(),
                fact: Some(payload(88, "2")),
                injection_like: false,
            },
            SourceSpan {
                span_id: "example".to_string(),
                class: SpanClass::Example,
                text: "non-authoritative example".to_string(),
                fact: None,
                injection_like: false,
            },
        ]);
        let (record, facts) = extract_document(&task, &document);
        assert_eq!(facts.len(), 1);
        assert_eq!(record.implementation_spans_quarantined, 1);
        assert_eq!(record.solution_like_spans_quarantined, 1);
        assert_eq!(record.example_spans_quarantined, 1);
        assert_eq!(record.injection_like_spans_detected, 1);
        assert_eq!(record.control_instructions_executed, 0);
    }

    #[test]
    fn semantic_compilation_validates_types_probes_and_provenance() {
        let task = task();
        let document = document("official", SourceAuthority::OfficialDocumentation, "2", 2);
        let (record, facts) = extract_document(&task, &document);
        assert_eq!(record.facts_accepted, 1);
        let fact = &facts[0];
        assert!(fact.type_check_passed);
        assert_eq!(fact.generated_probe_count, fact.generated_probes_passed);
        assert!(fact.validation_passed);
        assert_eq!(fact.source_ids, vec!["official"]);
        assert_eq!(fact.state, "EXTERNAL_CANDIDATE");
    }

    #[test]
    fn request_and_content_budgets_are_finite_and_read_only_by_design() {
        assert_eq!(REQUEST_BUDGET_PER_TASK, 2);
        assert_eq!(CONTENT_BUDGET_BYTES_PER_TASK, 8_192);
        assert!(!category_allowed(QueryCategory::GetBenchmarkPatch));
        assert!(!category_allowed(QueryCategory::GetReferenceImplementation));
    }
}
