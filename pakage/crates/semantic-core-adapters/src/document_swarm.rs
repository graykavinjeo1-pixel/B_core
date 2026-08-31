use std::collections::BTreeSet;

use dockable_semantic_core::{
    AssessmentVerdictIR, DeliberationFactIR, DockableCore, QualityCriterionIR, SwarmCore,
    SwarmDeliberationIR, SwarmDeliberationRequestIR, SwarmError, SWARM_DELIBERATION_REQUEST_SCHEMA,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::knowledge_work::{
    CellValueIR, DocumentDesignIR, DocumentKindIR, FindingKindIR, KnowledgeDocumentIR,
    KnowledgeFindingIR, KnowledgeSourceIR, KnowledgeWorkOperationIR, KnowledgeWorkRequestIR,
};

pub const DOCUMENT_DELIBERATION_SCHEMA: &str = "B_CORE_DOCUMENT_DELIBERATION_IR_1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentDeliberationIR {
    pub schema: String,
    pub input_document_sha256: String,
    pub accepted_document_sha256: String,
    pub required_facets: Vec<String>,
    pub swarm: SwarmDeliberationIR,
    pub causally_gated: bool,
    pub render_authorized: bool,
}

pub(crate) struct DocumentDeliberationContext<'a> {
    pub request: &'a KnowledgeWorkRequestIR,
    pub operation: KnowledgeWorkOperationIR,
    pub kind: DocumentKindIR,
    pub design: &'a DocumentDesignIR,
    pub document: &'a KnowledgeDocumentIR,
    pub findings: &'a [KnowledgeFindingIR],
    pub parent_reasoning_sha256: Option<&'a str>,
}

pub(crate) fn deliberate_document(
    core: Option<&DockableCore>,
    context: DocumentDeliberationContext<'_>,
) -> Result<DocumentDeliberationIR, SwarmError> {
    let DocumentDeliberationContext {
        request,
        operation,
        kind,
        design,
        document,
        findings,
        parent_reasoning_sha256,
    } = context;
    let document_sha256 = sha256_json(document);
    let required_facets = required_facets(&request.command, kind);
    let facts = build_facts(
        request,
        operation,
        kind,
        design,
        document,
        findings,
        &required_facets,
    );
    let parent_reasoning_sha256 =
        parent_reasoning_sha256
            .map(str::to_string)
            .unwrap_or_else(|| {
                sha256_json(&(
                    &request.request_id,
                    &request.command,
                    operation,
                    kind,
                    &document_sha256,
                ))
            });
    let swarm_request = SwarmDeliberationRequestIR {
        schema: SWARM_DELIBERATION_REQUEST_SCHEMA.to_string(),
        request_id: format!("{}-DOCUMENT-DELIBERATION", request.request_id),
        subject: format!("{:?}:{:?}", operation, kind),
        parent_reasoning_sha256,
        facts,
        max_workers: 6,
        max_rounds: 2,
    };
    let swarm = if let Some(core) = core {
        core.deliberate(&swarm_request)?
    } else {
        SwarmCore.deliberate(&swarm_request)?
    };
    Ok(DocumentDeliberationIR {
        schema: DOCUMENT_DELIBERATION_SCHEMA.to_string(),
        input_document_sha256: document_sha256.clone(),
        accepted_document_sha256: document_sha256,
        required_facets,
        causally_gated: true,
        render_authorized: swarm.accepted,
        swarm,
    })
}

fn build_facts(
    request: &KnowledgeWorkRequestIR,
    operation: KnowledgeWorkOperationIR,
    kind: DocumentKindIR,
    design: &DocumentDesignIR,
    document: &KnowledgeDocumentIR,
    findings: &[KnowledgeFindingIR],
    required_facets: &[String],
) -> Vec<DeliberationFactIR> {
    let strict_authoring = matches!(
        operation,
        KnowledgeWorkOperationIR::Write | KnowledgeWorkOperationIR::Plan
    );
    let requirement_ok = document.kind() == kind
        && design.schema == crate::knowledge_work::DOCUMENT_DESIGN_SCHEMA
        && required_facets
            .iter()
            .all(|facet| document_has_facet(document, facet))
        && source_preservation_ok(request, operation, document);
    let structure_ok = structure_is_complete(document);
    let audience_ok = audience_contract_is_complete(document);
    let evidence_ok = !findings
        .iter()
        .any(|finding| finding.kind == FindingKindIR::MissingEvidence)
        && findings
            .iter()
            .all(|finding| !finding.evidence_locations.is_empty());
    let contradiction_ok = identifiers_and_dependencies_are_consistent(document);
    let mut facts = vec![
        fact(
            "DOCUMENT-REQUIREMENT-COVERAGE",
            QualityCriterionIR::RequirementCoverage,
            if requirement_ok {
                AssessmentVerdictIR::Pass
            } else if request.source.is_none() {
                AssessmentVerdictIR::Warning
            } else {
                gap_verdict(false, strict_authoring)
            },
            if requirement_ok {
                "REQUEST_KIND_FORMAT_AND_EXPLICIT_FACETS_COVERED"
            } else {
                "REQUEST_KIND_FORMAT_OR_EXPLICIT_FACET_MISSING"
            },
            vec![
                "request:command".to_string(),
                "request:document_kind".to_string(),
                "document:kind".to_string(),
                format!("design:page_size:{:?}", design.page_size),
            ],
        ),
        fact(
            "DOCUMENT-STRUCTURE-INTEGRITY",
            QualityCriterionIR::StructureIntegrity,
            gap_verdict(structure_ok, strict_authoring),
            if structure_ok {
                "REQUIRED_TYPED_DOCUMENT_STRUCTURE_COMPLETE"
            } else {
                "REQUIRED_TYPED_DOCUMENT_STRUCTURE_INCOMPLETE"
            },
            vec!["document:typed_structure".to_string()],
        ),
        fact(
            "DOCUMENT-AUDIENCE-USABILITY",
            QualityCriterionIR::AudienceUsability,
            gap_verdict(audience_ok, strict_authoring),
            if audience_ok {
                "AUDIENCE_GOAL_ACTION_AND_RECOVERY_SURFACES_PRESENT"
            } else {
                "AUDIENCE_GOAL_ACTION_OR_RECOVERY_SURFACE_MISSING"
            },
            vec!["document:audience_surface".to_string()],
        ),
        fact(
            "DOCUMENT-EVIDENCE-INTEGRITY",
            QualityCriterionIR::EvidenceIntegrity,
            if request.source.is_none() {
                AssessmentVerdictIR::Warning
            } else if evidence_ok {
                if request.source.is_some() {
                    AssessmentVerdictIR::Pass
                } else {
                    AssessmentVerdictIR::Warning
                }
            } else {
                gap_verdict(false, strict_authoring)
            },
            if evidence_ok && request.source.is_some() {
                "FINDINGS_BOUND_TO_OBSERVABLE_SOURCE_LOCATIONS"
            } else if evidence_ok {
                "SOURCE_FREE_AUTHORING_REQUIRES_EXPLICIT_VERIFICATION"
            } else {
                "MISSING_OR_UNBOUND_EVIDENCE"
            },
            findings
                .iter()
                .flat_map(|finding| finding.evidence_locations.iter().cloned())
                .chain(std::iter::once("findings:typed_analysis".to_string()))
                .collect(),
        ),
        fact(
            "DOCUMENT-ADVERSARIAL-CHECK",
            QualityCriterionIR::ContradictionResistance,
            gap_verdict(contradiction_ok, strict_authoring),
            if contradiction_ok {
                "IDENTIFIERS_DEPENDENCIES_AND_SOURCE_BINDINGS_CONSISTENT"
            } else {
                "DUPLICATE_IDENTIFIER_OR_BROKEN_DEPENDENCY"
            },
            vec!["document:identifier_and_dependency_graph".to_string()],
        ),
    ];
    if has_quantitative_surface(document) {
        let quantitative_ok = quantitative_sources_are_complete(document);
        facts.push(fact(
            "DOCUMENT-QUANTITATIVE-INTEGRITY",
            QualityCriterionIR::QuantitativeIntegrity,
            gap_verdict(quantitative_ok, strict_authoring),
            if quantitative_ok {
                "NUMERIC_VALUES_AND_SERIES_RETAIN_SOURCE_BINDINGS"
            } else {
                "NUMERIC_VALUE_OR_SERIES_SOURCE_BINDING_MISSING"
            },
            vec!["document:quantitative_surface".to_string()],
        ));
    }
    facts
}

fn fact(
    fact_id: &str,
    criterion: QualityCriterionIR,
    verdict: AssessmentVerdictIR,
    rationale_code: &str,
    evidence_refs: Vec<String>,
) -> DeliberationFactIR {
    DeliberationFactIR {
        fact_id: fact_id.to_string(),
        criterion,
        verdict,
        rationale_code: rationale_code.to_string(),
        evidence_refs,
    }
}

fn gap_verdict(ok: bool, authoring: bool) -> AssessmentVerdictIR {
    if ok {
        AssessmentVerdictIR::Pass
    } else if authoring {
        AssessmentVerdictIR::Fail
    } else {
        AssessmentVerdictIR::Warning
    }
}

fn required_facets(command: &str, kind: DocumentKindIR) -> Vec<String> {
    let command = command.to_lowercase();
    let mut facets = Vec::new();
    for (facet, markers) in [
        ("examples", &["예시", "example"] as &[&str]),
        ("troubleshooting", &["문제 해결", "troubleshooting"]),
        ("checklist", &["체크리스트", "checklist"]),
        ("cautions", &["주의", "caution"]),
        ("charts", &["차트", "chart", "graph"]),
        ("references", &["참고문헌", "references"]),
    ] {
        if markers.iter().any(|marker| command.contains(marker)) {
            facets.push(facet.to_string());
        }
    }
    let requests_table = command.contains("table")
        || [" 표 ", "표를", "표와", "표가", "표는", "표,"]
            .iter()
            .any(|marker| command.contains(marker));
    // In "chart from this table", the table is input material rather than an
    // additional requested output artifact.
    if requests_table && kind != DocumentKindIR::Chart {
        facets.push("tables".to_string());
    }
    facets.sort();
    facets.dedup();
    facets
}

fn document_has_facet(document: &KnowledgeDocumentIR, facet: &str) -> bool {
    match (document, facet) {
        (KnowledgeDocumentIR::UserGuide(guide), "examples") => !guide.examples.is_empty(),
        (KnowledgeDocumentIR::UserGuide(guide), "troubleshooting") => {
            !guide.troubleshooting.is_empty()
        }
        (KnowledgeDocumentIR::UserGuide(guide), "checklist") => !guide.checklist.is_empty(),
        (KnowledgeDocumentIR::UserGuide(guide), "cautions") => !guide.cautions.is_empty(),
        (KnowledgeDocumentIR::UserGuide(guide), "tables") => !guide.tables.is_empty(),
        (KnowledgeDocumentIR::UserGuide(guide), "charts") => !guide.charts.is_empty(),
        (KnowledgeDocumentIR::Paper(paper), "tables") => !paper.tables.is_empty(),
        (KnowledgeDocumentIR::Paper(paper), "charts") => !paper.charts.is_empty(),
        (KnowledgeDocumentIR::Paper(paper), "references") => !paper.references.is_empty(),
        (KnowledgeDocumentIR::BusinessPlan(value), "tables")
        | (KnowledgeDocumentIR::BusinessProposal(value), "tables") => !value.tables.is_empty(),
        (KnowledgeDocumentIR::BusinessPlan(value), "charts")
        | (KnowledgeDocumentIR::BusinessProposal(value), "charts") => !value.charts.is_empty(),
        (KnowledgeDocumentIR::Table(_), "tables") => true,
        (KnowledgeDocumentIR::Chart(_), "charts") => true,
        _ => false,
    }
}

fn source_preservation_ok(
    request: &KnowledgeWorkRequestIR,
    operation: KnowledgeWorkOperationIR,
    document: &KnowledgeDocumentIR,
) -> bool {
    match (&request.source, operation) {
        (
            Some(KnowledgeSourceIR::Structured { document: source }),
            KnowledgeWorkOperationIR::Interpret,
        )
        | (
            Some(KnowledgeSourceIR::Structured { document: source }),
            KnowledgeWorkOperationIR::Analyze,
        ) => source.as_ref() == document,
        _ => true,
    }
}

fn structure_is_complete(document: &KnowledgeDocumentIR) -> bool {
    match document {
        KnowledgeDocumentIR::Paper(value) => {
            !value.title.trim().is_empty()
                && !value.abstract_text.trim().is_empty()
                && !value.sections.is_empty()
                && value.sections.iter().all(|section| {
                    !section.section_id.trim().is_empty()
                        && !section.heading.trim().is_empty()
                        && !section.body.trim().is_empty()
                })
        }
        KnowledgeDocumentIR::BusinessPlan(value) | KnowledgeDocumentIR::BusinessProposal(value) => {
            !value.title.trim().is_empty()
                && !value.executive_summary.trim().is_empty()
                && !value.sections.is_empty()
                && !value.execution_plan.tasks.is_empty()
                && !value.next_action.trim().is_empty()
        }
        KnowledgeDocumentIR::UserGuide(value) => {
            !value.title.trim().is_empty()
                && !value.introduction.trim().is_empty()
                && !value.sections.is_empty()
                && value.sections.iter().all(|section| {
                    !section.section_id.trim().is_empty()
                        && !section.heading.trim().is_empty()
                        && !section.body.trim().is_empty()
                })
        }
        KnowledgeDocumentIR::Table(value) => {
            !value.title.trim().is_empty()
                && !value.columns.is_empty()
                && !value.rows.is_empty()
                && value
                    .rows
                    .iter()
                    .all(|row| row.len() == value.columns.len())
        }
        KnowledgeDocumentIR::Chart(value) => {
            !value.title.trim().is_empty()
                && !value.series.is_empty()
                && value.series.iter().all(|series| !series.points.is_empty())
        }
        KnowledgeDocumentIR::FinancialStatement(value) => {
            !value.entity.trim().is_empty()
                && !value.periods.is_empty()
                && !value.line_items.is_empty()
        }
        KnowledgeDocumentIR::PlanProposal(value) => {
            !value.title.trim().is_empty()
                && !value.objective.trim().is_empty()
                && !value.tasks.is_empty()
        }
    }
}

fn audience_contract_is_complete(document: &KnowledgeDocumentIR) -> bool {
    match document {
        KnowledgeDocumentIR::UserGuide(value) => {
            !value.audience.trim().is_empty()
                && !value.examples.is_empty()
                && !value.troubleshooting.is_empty()
                && !value.checklist.is_empty()
        }
        KnowledgeDocumentIR::Paper(value) => {
            !value.abstract_text.trim().is_empty() && !value.sections.is_empty()
        }
        KnowledgeDocumentIR::BusinessPlan(value) | KnowledgeDocumentIR::BusinessProposal(value) => {
            !value.audience.trim().is_empty()
                && !value.executive_summary.trim().is_empty()
                && !value.next_action.trim().is_empty()
        }
        KnowledgeDocumentIR::Table(value) => !value.columns.is_empty(),
        KnowledgeDocumentIR::Chart(value) => {
            !value.category_axis.trim().is_empty() && !value.value_axis.trim().is_empty()
        }
        KnowledgeDocumentIR::FinancialStatement(value) => {
            !value.currency.trim().is_empty() && !value.display_unit.trim().is_empty()
        }
        KnowledgeDocumentIR::PlanProposal(value) => value
            .tasks
            .iter()
            .all(|task| !task.description.trim().is_empty()),
    }
}

fn identifiers_and_dependencies_are_consistent(document: &KnowledgeDocumentIR) -> bool {
    fn unique<'a>(mut values: impl Iterator<Item = &'a str>) -> bool {
        let mut seen = BTreeSet::new();
        values.all(|value| !value.trim().is_empty() && seen.insert(value))
    }
    match document {
        KnowledgeDocumentIR::Paper(value) => {
            unique(
                value
                    .sections
                    .iter()
                    .map(|section| section.section_id.as_str()),
            ) && unique(value.claims.iter().map(|claim| claim.claim_id.as_str()))
                && unique(
                    value
                        .references
                        .iter()
                        .map(|reference| reference.reference_id.as_str()),
                )
        }
        KnowledgeDocumentIR::BusinessPlan(value) | KnowledgeDocumentIR::BusinessProposal(value) => {
            unique(
                value
                    .sections
                    .iter()
                    .map(|section| section.section_id.as_str()),
            ) && plan_dependencies_are_consistent(&value.execution_plan)
        }
        KnowledgeDocumentIR::UserGuide(value) => unique(
            value
                .sections
                .iter()
                .map(|section| section.section_id.as_str()),
        ),
        KnowledgeDocumentIR::PlanProposal(value) => plan_dependencies_are_consistent(value),
        _ => true,
    }
}

fn plan_dependencies_are_consistent(plan: &crate::knowledge_work::PlanProposalIR) -> bool {
    let ids = plan
        .tasks
        .iter()
        .map(|task| task.task_id.as_str())
        .collect::<BTreeSet<_>>();
    ids.len() == plan.tasks.len()
        && !ids.contains("")
        && plan.tasks.iter().all(|task| {
            task.dependencies
                .iter()
                .all(|dependency| ids.contains(dependency.as_str()) && dependency != &task.task_id)
        })
}

fn has_quantitative_surface(document: &KnowledgeDocumentIR) -> bool {
    match document {
        KnowledgeDocumentIR::Paper(value) => !value.tables.is_empty() || !value.charts.is_empty(),
        KnowledgeDocumentIR::BusinessPlan(value) | KnowledgeDocumentIR::BusinessProposal(value) => {
            !value.key_metrics.is_empty()
                || !value.tables.is_empty()
                || !value.charts.is_empty()
                || !value.financial_statements.is_empty()
        }
        KnowledgeDocumentIR::UserGuide(value) => {
            !value.tables.is_empty() || !value.charts.is_empty()
        }
        KnowledgeDocumentIR::Table(value) => value
            .rows
            .iter()
            .flatten()
            .any(|cell| matches!(cell.value, CellValueIR::Number(_))),
        KnowledgeDocumentIR::Chart(_) | KnowledgeDocumentIR::FinancialStatement(_) => true,
        KnowledgeDocumentIR::PlanProposal(_) => false,
    }
}

fn quantitative_sources_are_complete(document: &KnowledgeDocumentIR) -> bool {
    fn table_ok(table: &crate::knowledge_work::TableIR) -> bool {
        table.rows.iter().flatten().all(|cell| {
            !matches!(cell.value, CellValueIR::Number(_)) || !cell.source_location.trim().is_empty()
        })
    }
    fn chart_ok(chart: &crate::knowledge_work::ChartIR) -> bool {
        chart
            .series
            .iter()
            .flat_map(|series| &series.points)
            .all(|point| point.value.is_none() || !point.source_location.trim().is_empty())
    }
    fn financial_ok(statement: &crate::knowledge_work::FinancialStatementIR) -> bool {
        statement
            .line_items
            .iter()
            .all(|item| !item.source_location.trim().is_empty())
    }
    match document {
        KnowledgeDocumentIR::Paper(value) => {
            value.tables.iter().all(table_ok) && value.charts.iter().all(chart_ok)
        }
        KnowledgeDocumentIR::BusinessPlan(value) | KnowledgeDocumentIR::BusinessProposal(value) => {
            value
                .key_metrics
                .iter()
                .all(|metric| !metric.evidence_location.trim().is_empty())
                && value.tables.iter().all(table_ok)
                && value.charts.iter().all(chart_ok)
                && value.financial_statements.iter().all(financial_ok)
        }
        KnowledgeDocumentIR::UserGuide(value) => {
            value.tables.iter().all(table_ok) && value.charts.iter().all(chart_ok)
        }
        KnowledgeDocumentIR::Table(value) => table_ok(value),
        KnowledgeDocumentIR::Chart(value) => chart_ok(value),
        KnowledgeDocumentIR::FinancialStatement(value) => financial_ok(value),
        KnowledgeDocumentIR::PlanProposal(_) => true,
    }
}

fn sha256_json<T: Serialize>(value: &T) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).unwrap_or_default())
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge_work::{
        DocumentThemeIR, GuideExampleIR, GuideSectionIR, OutputDirectiveIR, OutputFormatIR,
        OutputModeIR, PageSizeIR, TroubleshootingItemIR, UserGuideIR, DOCUMENT_DESIGN_SCHEMA,
        KNOWLEDGE_WORK_REQUEST_SCHEMA, USER_GUIDE_SCHEMA,
    };

    fn request(command: &str, document: UserGuideIR) -> KnowledgeWorkRequestIR {
        KnowledgeWorkRequestIR {
            schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
            request_id: "DOC-SWARM-TEST".to_string(),
            command: command.to_string(),
            source: Some(KnowledgeSourceIR::Structured {
                document: Box::new(KnowledgeDocumentIR::UserGuide(document)),
            }),
            document_kind: Some(DocumentKindIR::UserGuide),
            output_language: None,
            design: None,
            output: OutputDirectiveIR {
                mode: OutputModeIR::Text,
                format: OutputFormatIR::Html,
                path: None,
                overwrite: false,
            },
            context_tags: Vec::new(),
            max_plan_steps: 8,
        }
    }

    fn complete_guide() -> UserGuideIR {
        UserGuideIR {
            schema: USER_GUIDE_SCHEMA.to_string(),
            document_id: "GUIDE".to_string(),
            title: "전문 설명서".to_string(),
            audience: "실무자".to_string(),
            introduction: "목적과 범위를 설명한다.".to_string(),
            sections: vec![GuideSectionIR {
                section_id: "S1".to_string(),
                heading: "절차".to_string(),
                body: "검증 가능한 절차".to_string(),
                steps: vec!["실행".to_string()],
            }],
            examples: vec![GuideExampleIR {
                title: "예시".to_string(),
                input: "입력".to_string(),
                expected_result: "결과".to_string(),
            }],
            cautions: vec!["주의".to_string()],
            troubleshooting: vec![TroubleshootingItemIR {
                symptom: "증상".to_string(),
                resolution: "조치".to_string(),
            }],
            checklist: vec!["확인".to_string()],
            tables: Vec::new(),
            charts: Vec::new(),
        }
    }

    #[test]
    fn explicit_missing_facet_is_rejected_before_rendering() {
        let mut guide = complete_guide();
        guide.troubleshooting.clear();
        let request = request("문제 해결을 포함한 사용 설명서를 작성해", guide.clone());
        let document = KnowledgeDocumentIR::UserGuide(guide);
        let deliberation = deliberate_document(
            None,
            DocumentDeliberationContext {
                request: &request,
                operation: KnowledgeWorkOperationIR::Write,
                kind: DocumentKindIR::UserGuide,
                design: &DocumentDesignIR {
                    schema: DOCUMENT_DESIGN_SCHEMA.to_string(),
                    theme: DocumentThemeIR::GuideIndigo,
                    page_size: PageSizeIR::A4,
                    brand_name: None,
                    accent_color: None,
                    compact: false,
                    show_table_of_contents: true,
                    show_page_furniture: true,
                },
                document: &document,
                findings: &[],
                parent_reasoning_sha256: Some("plan"),
            },
        )
        .expect("deliberation");
        assert!(!deliberation.render_authorized);
        assert!(deliberation
            .required_facets
            .contains(&"troubleshooting".to_string()));
    }

    #[test]
    fn rejected_deliberation_cannot_write_a_file() {
        let root =
            std::env::temp_dir().join(format!("b-core-deliberation-gate-{}", std::process::id()));
        let path = root.join("must-not-exist.html");
        let mut guide = complete_guide();
        guide.troubleshooting.clear();
        let mut request = request("문제 해결을 포함한 사용 설명서를 작성해", guide);
        request.output = OutputDirectiveIR {
            mode: OutputModeIR::File,
            format: OutputFormatIR::Html,
            path: Some(path.to_string_lossy().to_string()),
            overwrite: true,
        };
        let result = crate::knowledge_work::execute_document_work(&request);
        assert_eq!(
            result,
            Err(crate::knowledge_work::KnowledgeWorkError::DeliberationRejected)
        );
        assert!(!path.exists());
        if root.exists() {
            std::fs::remove_dir_all(root).expect("cleanup");
        }
    }

    #[test]
    fn complete_document_is_reviewed_by_multiple_internal_workers() {
        let guide = complete_guide();
        let request = request(
            "예시와 체크리스트가 있는 사용 설명서를 작성해",
            guide.clone(),
        );
        let document = KnowledgeDocumentIR::UserGuide(guide);
        let deliberation = deliberate_document(
            None,
            DocumentDeliberationContext {
                request: &request,
                operation: KnowledgeWorkOperationIR::Write,
                kind: DocumentKindIR::UserGuide,
                design: &DocumentDesignIR::for_kind(DocumentKindIR::UserGuide),
                document: &document,
                findings: &[],
                parent_reasoning_sha256: Some("plan"),
            },
        )
        .expect("deliberation");
        assert!(deliberation.render_authorized);
        assert!(deliberation.swarm.worker_spawn_count >= 5);
        assert!(!deliberation.swarm.peer_messages.is_empty());
        assert_eq!(deliberation.swarm.external_model_calls, 0);
    }

    #[test]
    fn source_free_professional_guide_is_allowed_with_explicit_evidence_warning() {
        let request = KnowledgeWorkRequestIR {
            schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
            request_id: "GUIDE-SOURCE-FREE".to_string(),
            command: "GPT 사용 설명서를 전문 A4 문서로 작성해. 확인되지 않은 기능은 확인 필요라고 표시해."
                .to_string(),
            source: None,
            document_kind: Some(DocumentKindIR::UserGuide),
            output_language: None,
            design: None,
            output: OutputDirectiveIR {
                mode: OutputModeIR::Text,
                format: OutputFormatIR::Html,
                path: None,
                overwrite: false,
            },
            context_tags: vec!["professional_document".to_string()],
            max_plan_steps: 16,
        };
        let product = crate::knowledge_work::execute_document_work(&request)
            .expect("source-free guide with explicit verification language");
        assert!(product.deliberation.render_authorized);
        assert!(product
            .deliberation
            .swarm
            .decisions
            .iter()
            .any(|decision| decision.verdict == AssessmentVerdictIR::Warning));
    }
}
