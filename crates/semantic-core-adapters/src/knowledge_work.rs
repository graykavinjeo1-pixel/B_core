use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::language_knowledge::LanguageCodeIR;

pub const KNOWLEDGE_WORK_REQUEST_SCHEMA: &str = "B_CORE_KNOWLEDGE_WORK_REQUEST_IR_1";
pub const KNOWLEDGE_WORK_RESPONSE_SCHEMA: &str = "B_CORE_KNOWLEDGE_WORK_RESPONSE_IR_1";
pub const PAPER_SCHEMA: &str = "B_CORE_PAPER_IR_1";
pub const TABLE_SCHEMA: &str = "B_CORE_TABLE_IR_1";
pub const CHART_SCHEMA: &str = "B_CORE_CHART_IR_1";
pub const FINANCIAL_STATEMENT_SCHEMA: &str = "B_CORE_FINANCIAL_STATEMENT_IR_1";
pub const PLAN_PROPOSAL_SCHEMA: &str = "B_CORE_PLAN_PROPOSAL_IR_1";
pub const BUSINESS_DOCUMENT_SCHEMA: &str = "B_CORE_BUSINESS_DOCUMENT_IR_1";
pub const USER_GUIDE_SCHEMA: &str = "B_CORE_USER_GUIDE_IR_1";
pub const DOCUMENT_DESIGN_SCHEMA: &str = "B_CORE_DOCUMENT_DESIGN_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentKindIR {
    Paper,
    BusinessPlan,
    BusinessProposal,
    UserGuide,
    Table,
    Chart,
    FinancialStatement,
    PlanProposal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KnowledgeWorkOperationIR {
    Interpret,
    Analyze,
    Write,
    Plan,
    Revise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceTextFormatIR {
    PlainText,
    Markdown,
    Csv,
    Tsv,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KnowledgeSourceIR {
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        format: Option<SourceTextFormatIR>,
    },
    File {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        format: Option<SourceTextFormatIR>,
    },
    Structured {
        document: Box<KnowledgeDocumentIR>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutputModeIR {
    Text,
    File,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutputFormatIR {
    Markdown,
    Html,
    Json,
    Csv,
    Svg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentThemeIR {
    AcademicEditorial,
    ExecutiveNavy,
    ProposalCobalt,
    GuideIndigo,
    MinimalMonochrome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PageSizeIR {
    A4,
    Letter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentDesignIR {
    pub schema: String,
    pub theme: DocumentThemeIR,
    pub page_size: PageSizeIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brand_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<String>,
    #[serde(default)]
    pub compact: bool,
    #[serde(default = "default_true")]
    pub show_table_of_contents: bool,
    #[serde(default = "default_true")]
    pub show_page_furniture: bool,
}

impl DocumentDesignIR {
    pub fn for_kind(kind: DocumentKindIR) -> Self {
        let theme = match kind {
            DocumentKindIR::Paper => DocumentThemeIR::AcademicEditorial,
            DocumentKindIR::BusinessPlan => DocumentThemeIR::ExecutiveNavy,
            DocumentKindIR::BusinessProposal => DocumentThemeIR::ProposalCobalt,
            DocumentKindIR::UserGuide => DocumentThemeIR::GuideIndigo,
            _ => DocumentThemeIR::MinimalMonochrome,
        };
        Self {
            schema: DOCUMENT_DESIGN_SCHEMA.to_string(),
            theme,
            page_size: PageSizeIR::A4,
            brand_name: None,
            accent_color: None,
            compact: false,
            show_table_of_contents: true,
            show_page_furniture: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputDirectiveIR {
    pub mode: OutputModeIR,
    pub format: OutputFormatIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeWorkRequestIR {
    pub schema: String,
    pub request_id: String,
    /// Natural-language authority for operation, artifact kind and revision.
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<KnowledgeSourceIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_kind: Option<DocumentKindIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_language: Option<LanguageCodeIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design: Option<DocumentDesignIR>,
    pub output: OutputDirectiveIR,
    #[serde(default)]
    pub context_tags: Vec<String>,
    pub max_plan_steps: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NumericValueIR {
    pub coefficient: i64,
    pub scale: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub original: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CellValueIR {
    Text(String),
    Number(NumericValueIR),
    Boolean(bool),
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableCellIR {
    pub value: CellValueIR,
    pub raw: String,
    pub source_location: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableIR {
    pub schema: String,
    pub document_id: String,
    pub title: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<TableCellIR>>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaperSectionIR {
    pub section_id: String,
    pub heading: String,
    pub body: String,
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaperClaimIR {
    pub claim_id: String,
    pub statement: String,
    pub evidence_locations: Vec<String>,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaperReferenceIR {
    pub reference_id: String,
    pub citation_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaperIR {
    pub schema: String,
    pub document_id: String,
    pub title: String,
    #[serde(default)]
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub sections: Vec<PaperSectionIR>,
    #[serde(default)]
    pub claims: Vec<PaperClaimIR>,
    #[serde(default)]
    pub references: Vec<PaperReferenceIR>,
    #[serde(default)]
    pub tables: Vec<TableIR>,
    #[serde(default)]
    pub charts: Vec<ChartIR>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChartTypeIR {
    Line,
    Bar,
    Scatter,
    Pie,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChartPointIR {
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<NumericValueIR>,
    pub source_location: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChartSeriesIR {
    pub name: String,
    pub points: Vec<ChartPointIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChartIR {
    pub schema: String,
    pub document_id: String,
    pub title: String,
    pub chart_type: ChartTypeIR,
    pub category_axis: String,
    pub value_axis: String,
    pub series: Vec<ChartSeriesIR>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FinancialStatementTypeIR {
    BalanceSheet,
    IncomeStatement,
    CashFlowStatement,
    ChangesInEquity,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FinancialLineClassIR {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
    CashFlow,
    Total,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinancialLineItemIR {
    pub label: String,
    pub normalized_concept: String,
    pub class: FinancialLineClassIR,
    pub values_by_period: BTreeMap<String, NumericValueIR>,
    pub source_location: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinancialStatementIR {
    pub schema: String,
    pub document_id: String,
    pub entity: String,
    pub statement_type: FinancialStatementTypeIR,
    pub periods: Vec<String>,
    pub currency: String,
    pub display_unit: String,
    pub line_items: Vec<FinancialLineItemIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanTaskIR {
    pub task_id: String,
    pub description: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_condition: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanProposalIR {
    pub schema: String,
    pub document_id: String,
    pub title: String,
    pub objective: String,
    pub tasks: Vec<PlanTaskIR>,
    #[serde(default)]
    pub risks: Vec<String>,
    #[serde(default)]
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BusinessDocumentTypeIR {
    BusinessPlan,
    BusinessProposal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusinessMetricIR {
    pub label: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<String>,
    pub evidence_location: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusinessSectionIR {
    pub section_id: String,
    pub heading: String,
    pub body: String,
    #[serde(default)]
    pub highlights: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusinessDocumentIR {
    pub schema: String,
    pub document_id: String,
    pub document_type: BusinessDocumentTypeIR,
    pub title: String,
    pub organization: String,
    pub audience: String,
    pub executive_summary: String,
    pub sections: Vec<BusinessSectionIR>,
    #[serde(default)]
    pub key_metrics: Vec<BusinessMetricIR>,
    pub execution_plan: PlanProposalIR,
    #[serde(default)]
    pub tables: Vec<TableIR>,
    #[serde(default)]
    pub charts: Vec<ChartIR>,
    #[serde(default)]
    pub financial_statements: Vec<FinancialStatementIR>,
    #[serde(default)]
    pub risks: Vec<String>,
    pub next_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuideSectionIR {
    pub section_id: String,
    pub heading: String,
    pub body: String,
    #[serde(default)]
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuideExampleIR {
    pub title: String,
    pub input: String,
    pub expected_result: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TroubleshootingItemIR {
    pub symptom: String,
    pub resolution: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserGuideIR {
    pub schema: String,
    pub document_id: String,
    pub title: String,
    pub audience: String,
    pub introduction: String,
    pub sections: Vec<GuideSectionIR>,
    #[serde(default)]
    pub examples: Vec<GuideExampleIR>,
    #[serde(default)]
    pub cautions: Vec<String>,
    #[serde(default)]
    pub troubleshooting: Vec<TroubleshootingItemIR>,
    #[serde(default)]
    pub checklist: Vec<String>,
    #[serde(default)]
    pub tables: Vec<TableIR>,
    #[serde(default)]
    pub charts: Vec<ChartIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "content", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KnowledgeDocumentIR {
    Paper(PaperIR),
    BusinessPlan(BusinessDocumentIR),
    BusinessProposal(BusinessDocumentIR),
    UserGuide(UserGuideIR),
    Table(TableIR),
    Chart(ChartIR),
    FinancialStatement(FinancialStatementIR),
    PlanProposal(PlanProposalIR),
}

impl KnowledgeDocumentIR {
    pub fn kind(&self) -> DocumentKindIR {
        match self {
            Self::Paper(_) => DocumentKindIR::Paper,
            Self::BusinessPlan(_) => DocumentKindIR::BusinessPlan,
            Self::BusinessProposal(_) => DocumentKindIR::BusinessProposal,
            Self::UserGuide(_) => DocumentKindIR::UserGuide,
            Self::Table(_) => DocumentKindIR::Table,
            Self::Chart(_) => DocumentKindIR::Chart,
            Self::FinancialStatement(_) => DocumentKindIR::FinancialStatement,
            Self::PlanProposal(_) => DocumentKindIR::PlanProposal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FindingKindIR {
    Summary,
    Trend,
    Maximum,
    Minimum,
    MissingEvidence,
    StructuralGap,
    AccountingCheck,
    Risk,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeFindingIR {
    pub finding_id: String,
    pub kind: FindingKindIR,
    pub statement: String,
    pub evidence_locations: Vec<String>,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileOutputReceiptIR {
    pub path: String,
    pub format: OutputFormatIR,
    pub bytes_written: u64,
    pub sha256: String,
    pub overwritten: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeWorkProductIR {
    pub operation: KnowledgeWorkOperationIR,
    pub output_language: LanguageCodeIR,
    pub design: DocumentDesignIR,
    pub document: KnowledgeDocumentIR,
    pub findings: Vec<KnowledgeFindingIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_output: Option<FileOutputReceiptIR>,
    pub content_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KnowledgeWorkError {
    InvalidSchema,
    InvalidRequest,
    MissingSource,
    UnsupportedSource,
    ParseFailure,
    UnsupportedOutput,
    InvalidOutputPath,
    OutputExists,
    FileRead,
    FileWrite,
    RevisionNotGrounded,
    NumericOverflow,
    Json,
}

fn default_true() -> bool {
    true
}

pub fn infer_document_design(command: &str, kind: DocumentKindIR) -> DocumentDesignIR {
    let command = normalize(command);
    let mut design = DocumentDesignIR::for_kind(kind);
    if contains_any(
        &command,
        &["흑백", "모노크롬", "monochrome", "black and white"],
    ) {
        design.theme = DocumentThemeIR::MinimalMonochrome;
    } else if contains_any(
        &command,
        &["학술", "저널", "academic", "journal", "editorial"],
    ) {
        design.theme = DocumentThemeIR::AcademicEditorial;
    } else if contains_any(
        &command,
        &["투자", "이사회", "executive", "investor", "board"],
    ) {
        design.theme = DocumentThemeIR::ExecutiveNavy;
    } else if contains_any(
        &command,
        &[
            "고객 제안",
            "영업 제안",
            "client proposal",
            "sales proposal",
        ],
    ) {
        design.theme = DocumentThemeIR::ProposalCobalt;
    }
    design.compact = contains_any(&command, &["간결", "압축", "compact", "dense"]);
    design.page_size = if contains_any(&command, &["letter size", "us letter"]) {
        PageSizeIR::Letter
    } else {
        PageSizeIR::A4
    };
    design
}

fn valid_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn infer_operation(command: &str) -> KnowledgeWorkOperationIR {
    let command = normalize(command);
    if contains_any(
        &command,
        &[
            "수정", "고쳐", "개정", "revise", "edit", "modify", "rewrite",
        ],
    ) {
        KnowledgeWorkOperationIR::Revise
    } else if contains_any(&command, &["계획", "계획안", "plan", "roadmap"]) {
        KnowledgeWorkOperationIR::Plan
    } else if contains_any(
        &command,
        &[
            "작성",
            "써줘",
            "만들어",
            "write",
            "author",
            "draft",
            "create",
        ],
    ) {
        KnowledgeWorkOperationIR::Write
    } else if contains_any(&command, &["분석", "검토", "analyze", "inspect", "compare"]) {
        KnowledgeWorkOperationIR::Analyze
    } else {
        KnowledgeWorkOperationIR::Interpret
    }
}

pub fn infer_document_kind(command: &str, source_text: Option<&str>) -> DocumentKindIR {
    let command = normalize(command);
    if contains_any(
        &command,
        &["사업계획서", "사업 계획서", "business plan", "venture plan"],
    ) {
        return DocumentKindIR::BusinessPlan;
    }
    if contains_any(
        &command,
        &[
            "사업제안서",
            "사업 제안서",
            "제안서",
            "business proposal",
            "commercial proposal",
        ],
    ) {
        return DocumentKindIR::BusinessProposal;
    }
    if contains_any(
        &command,
        &[
            "사용 설명서",
            "사용설명서",
            "사용자 가이드",
            "설명서",
            "매뉴얼",
            "안내서",
            "user guide",
            "user manual",
            "manual",
            "how-to guide",
        ],
    ) {
        return DocumentKindIR::UserGuide;
    }
    if contains_any(
        &command,
        &[
            "재무제표",
            "재무재표",
            "손익계산서",
            "대차대조표",
            "현금흐름표",
            "financial statement",
            "balance sheet",
            "income statement",
            "cash flow statement",
        ],
    ) {
        return DocumentKindIR::FinancialStatement;
    }
    if contains_any(
        &command,
        &["차트", "그래프", "도표", "chart", "graph", "plot"],
    ) {
        return DocumentKindIR::Chart;
    }
    if contains_any(
        &command,
        &["논문", "연구", "paper", "article", "manuscript"],
    ) {
        return DocumentKindIR::Paper;
    }
    if contains_any(
        &command,
        &["계획안", "실행계획", "proposal", "roadmap", "project plan"],
    ) {
        return DocumentKindIR::PlanProposal;
    }
    if contains_surface_any(&command, &["표", "테이블", "table", "csv", "tsv"]) {
        return DocumentKindIR::Table;
    }
    let source = source_text.unwrap_or_default();
    if source
        .lines()
        .any(|line| line.trim_start().starts_with('#'))
    {
        DocumentKindIR::Paper
    } else if source.lines().any(|line| line.matches('|').count() >= 2)
        || source.lines().any(|line| line.matches(',').count() >= 2)
    {
        DocumentKindIR::Table
    } else {
        DocumentKindIR::PlanProposal
    }
}

pub fn validate_request(request: &KnowledgeWorkRequestIR) -> Result<(), KnowledgeWorkError> {
    if request.schema != KNOWLEDGE_WORK_REQUEST_SCHEMA {
        return Err(KnowledgeWorkError::InvalidSchema);
    }
    if request.request_id.trim().is_empty()
        || request.request_id.len() > 128
        || request.command.trim().is_empty()
        || request.command.len() > 64 * 1024
        || !(5..=32).contains(&request.max_plan_steps)
        || request.context_tags.len() > 64
        || request
            .context_tags
            .iter()
            .any(|tag| tag.trim().is_empty() || tag.len() > 128)
    {
        return Err(KnowledgeWorkError::InvalidRequest);
    }
    if let Some(design) = &request.design {
        if design.schema != DOCUMENT_DESIGN_SCHEMA
            || design
                .brand_name
                .as_ref()
                .is_some_and(|value| value.trim().is_empty() || value.len() > 256)
            || design
                .accent_color
                .as_ref()
                .is_some_and(|value| !valid_hex_color(value))
        {
            return Err(KnowledgeWorkError::InvalidRequest);
        }
    }
    match request.output.mode {
        OutputModeIR::Text => {
            if request.output.path.is_some() {
                return Err(KnowledgeWorkError::InvalidOutputPath);
            }
        }
        OutputModeIR::File | OutputModeIR::Both => {
            let Some(path) = request.output.path.as_deref() else {
                return Err(KnowledgeWorkError::InvalidOutputPath);
            };
            if path.trim().is_empty() || path.len() > 32_768 {
                return Err(KnowledgeWorkError::InvalidOutputPath);
            }
        }
    }
    Ok(())
}

pub fn execute_document_work(
    request: &KnowledgeWorkRequestIR,
) -> Result<KnowledgeWorkProductIR, KnowledgeWorkError> {
    execute_document_work_as(request, infer_operation(&request.command), None)
}

pub fn execute_document_work_as(
    request: &KnowledgeWorkRequestIR,
    operation: KnowledgeWorkOperationIR,
    inferred_kind: Option<DocumentKindIR>,
) -> Result<KnowledgeWorkProductIR, KnowledgeWorkError> {
    validate_request(request)?;
    let (source_document, source_text) = load_source(request.source.as_ref(), &request.request_id)?;
    let output_language = request
        .output_language
        .filter(|language| matches!(language, LanguageCodeIR::Korean | LanguageCodeIR::English))
        .unwrap_or_else(|| detect_output_language(&request.command));
    let kind = request
        .document_kind
        .or_else(|| source_document.as_ref().map(KnowledgeDocumentIR::kind))
        .or(inferred_kind)
        .unwrap_or_else(|| infer_document_kind(&request.command, source_text.as_deref()));
    let design = request
        .design
        .clone()
        .unwrap_or_else(|| infer_document_design(&request.command, kind));
    let mut document = if let Some(document) = source_document {
        document
    } else if let Some(text) = source_text.as_deref() {
        parse_document(kind, &request.request_id, text)?
    } else if matches!(
        operation,
        KnowledgeWorkOperationIR::Write | KnowledgeWorkOperationIR::Plan
    ) {
        create_document(kind, &request.request_id, &request.command, output_language)
    } else {
        return Err(KnowledgeWorkError::MissingSource);
    };
    if operation == KnowledgeWorkOperationIR::Revise {
        revise_document(&mut document, &request.command)?;
    } else if operation == KnowledgeWorkOperationIR::Write {
        if let KnowledgeDocumentIR::Chart(chart) = &mut document {
            chart.chart_type = infer_chart_type(&request.command);
        }
    }
    let findings = analyze_document_in_language(&document, output_language);
    let rendered = render_result(
        &document,
        &findings,
        operation,
        request.output.format,
        output_language,
        &design,
    )?;
    let content_sha256 = sha256(rendered.as_bytes());
    let file_output = if matches!(request.output.mode, OutputModeIR::File | OutputModeIR::Both) {
        Some(write_output(
            request
                .output
                .path
                .as_deref()
                .ok_or(KnowledgeWorkError::InvalidOutputPath)?,
            request.output.format,
            rendered.as_bytes(),
            request.output.overwrite,
        )?)
    } else {
        None
    };
    let text_output = if matches!(request.output.mode, OutputModeIR::Text | OutputModeIR::Both) {
        Some(rendered)
    } else {
        None
    };
    Ok(KnowledgeWorkProductIR {
        operation,
        output_language,
        design,
        document,
        findings,
        text_output,
        file_output,
        content_sha256,
    })
}

fn load_source(
    source: Option<&KnowledgeSourceIR>,
    _request_id: &str,
) -> Result<(Option<KnowledgeDocumentIR>, Option<String>), KnowledgeWorkError> {
    match source {
        None => Ok((None, None)),
        Some(KnowledgeSourceIR::Text { text, format }) => {
            if text.is_empty() || text.len() > 16 * 1024 * 1024 {
                return Err(KnowledgeWorkError::UnsupportedSource);
            }
            if *format == Some(SourceTextFormatIR::Json) {
                let document = serde_json::from_str::<KnowledgeDocumentIR>(text)
                    .map_err(|_| KnowledgeWorkError::ParseFailure)?;
                Ok((Some(document), None))
            } else {
                Ok((None, Some(text.clone())))
            }
        }
        Some(KnowledgeSourceIR::File { path, format }) => {
            if path.trim().is_empty() || path.len() > 32_768 {
                return Err(KnowledgeWorkError::UnsupportedSource);
            }
            let metadata = fs::metadata(path).map_err(|_| KnowledgeWorkError::FileRead)?;
            if !metadata.is_file() || metadata.len() > 16 * 1024 * 1024 {
                return Err(KnowledgeWorkError::UnsupportedSource);
            }
            let text = fs::read_to_string(path).map_err(|_| KnowledgeWorkError::FileRead)?;
            let is_json = *format == Some(SourceTextFormatIR::Json)
                || Path::new(path)
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
            if is_json {
                let document = serde_json::from_str::<KnowledgeDocumentIR>(&text)
                    .map_err(|_| KnowledgeWorkError::ParseFailure)?;
                Ok((Some(document), None))
            } else {
                Ok((None, Some(text)))
            }
        }
        Some(KnowledgeSourceIR::Structured { document }) => Ok((Some((**document).clone()), None)),
    }
}

fn parse_document(
    kind: DocumentKindIR,
    document_id: &str,
    text: &str,
) -> Result<KnowledgeDocumentIR, KnowledgeWorkError> {
    match kind {
        DocumentKindIR::Paper => Ok(KnowledgeDocumentIR::Paper(parse_paper(document_id, text))),
        DocumentKindIR::BusinessPlan => Ok(KnowledgeDocumentIR::BusinessPlan(parse_business(
            document_id,
            text,
            BusinessDocumentTypeIR::BusinessPlan,
        ))),
        DocumentKindIR::BusinessProposal => Ok(KnowledgeDocumentIR::BusinessProposal(
            parse_business(document_id, text, BusinessDocumentTypeIR::BusinessProposal),
        )),
        DocumentKindIR::UserGuide => Ok(KnowledgeDocumentIR::UserGuide(parse_user_guide(
            document_id,
            text,
        ))),
        DocumentKindIR::Table => Ok(KnowledgeDocumentIR::Table(parse_table(document_id, text)?)),
        DocumentKindIR::Chart => {
            let table = parse_table(document_id, text)?;
            Ok(KnowledgeDocumentIR::Chart(chart_from_table(
                document_id,
                &table,
            )?))
        }
        DocumentKindIR::FinancialStatement => {
            let table = parse_table(document_id, text)?;
            Ok(KnowledgeDocumentIR::FinancialStatement(
                financial_from_table(document_id, &table)?,
            ))
        }
        DocumentKindIR::PlanProposal => Ok(KnowledgeDocumentIR::PlanProposal(parse_plan(
            document_id,
            text,
        ))),
    }
}

fn create_document(
    kind: DocumentKindIR,
    document_id: &str,
    command: &str,
    output_language: LanguageCodeIR,
) -> KnowledgeDocumentIR {
    let subject = command_subject(command);
    let korean = output_language == LanguageCodeIR::Korean;
    match kind {
        DocumentKindIR::Paper => KnowledgeDocumentIR::Paper(PaperIR {
            schema: PAPER_SCHEMA.to_string(),
            document_id: document_id.to_string(),
            title: subject.clone(),
            authors: Vec::new(),
            abstract_text: if korean {
                format!(
                    "{}에 관한 연구 목적, 방법, 결과와 한계를 검증 가능한 근거로 정리한다.",
                    subject
                )
            } else {
                format!(
                    "This draft organizes the objective, method, results, and limitations of {subject} around verifiable evidence."
                )
            },
            sections: if korean {
                vec!["서론", "방법", "결과", "논의", "결론"]
            } else {
                vec![
                    "Introduction",
                    "Methods",
                    "Results",
                    "Discussion",
                    "Conclusion",
                ]
            }
            .into_iter()
            .enumerate()
            .map(|(index, heading)| PaperSectionIR {
                section_id: format!("SEC-{}", index + 1),
                heading: heading.to_string(),
                body: if korean {
                    format!("{subject}: {heading}의 검증된 내용을 작성한다.")
                } else {
                    format!("Write the evidence-grounded {heading} content for {subject}.")
                },
                level: 2,
            })
            .collect(),
            claims: Vec::new(),
            references: Vec::new(),
            tables: Vec::new(),
            charts: Vec::new(),
        }),
        DocumentKindIR::BusinessPlan => {
            KnowledgeDocumentIR::BusinessPlan(create_business_document(
                document_id,
                &subject,
                BusinessDocumentTypeIR::BusinessPlan,
                korean,
            ))
        }
        DocumentKindIR::BusinessProposal => {
            KnowledgeDocumentIR::BusinessProposal(create_business_document(
                document_id,
                &subject,
                BusinessDocumentTypeIR::BusinessProposal,
                korean,
            ))
        }
        DocumentKindIR::UserGuide => {
            KnowledgeDocumentIR::UserGuide(create_user_guide(document_id, command, korean))
        }
        DocumentKindIR::Table => KnowledgeDocumentIR::Table(TableIR {
            schema: TABLE_SCHEMA.to_string(),
            document_id: document_id.to_string(),
            title: subject,
            columns: if korean {
                vec!["항목".to_string(), "값".to_string(), "근거".to_string()]
            } else {
                vec![
                    "item".to_string(),
                    "value".to_string(),
                    "evidence".to_string(),
                ]
            },
            rows: Vec::new(),
            notes: vec![if korean {
                "자연어 명령에서 생성된 빈 구조; 값은 검증 후 추가".to_string()
            } else {
                "Empty structure created from the command; add values only after verification"
                    .to_string()
            }],
        }),
        DocumentKindIR::Chart => KnowledgeDocumentIR::Chart(ChartIR {
            schema: CHART_SCHEMA.to_string(),
            document_id: document_id.to_string(),
            title: subject,
            chart_type: infer_chart_type(command),
            category_axis: "category".to_string(),
            value_axis: "value".to_string(),
            series: Vec::new(),
        }),
        DocumentKindIR::FinancialStatement => {
            KnowledgeDocumentIR::FinancialStatement(FinancialStatementIR {
                schema: FINANCIAL_STATEMENT_SCHEMA.to_string(),
                document_id: document_id.to_string(),
                entity: subject,
                statement_type: infer_statement_type(command),
                periods: Vec::new(),
                currency: "UNSPECIFIED".to_string(),
                display_unit: "1".to_string(),
                line_items: Vec::new(),
            })
        }
        DocumentKindIR::PlanProposal => KnowledgeDocumentIR::PlanProposal(PlanProposalIR {
            schema: PLAN_PROPOSAL_SCHEMA.to_string(),
            document_id: document_id.to_string(),
            title: subject.clone(),
            objective: subject,
            tasks: if korean {
                vec![
                    task("TASK-1", "현재 상태와 근거를 확인", &[]),
                    task("TASK-2", "완료 조건과 제약을 구조화", &["TASK-1"]),
                    task("TASK-3", "후보 실행안을 작성하고 검증", &["TASK-2"]),
                    task("TASK-4", "검증된 결과를 반영", &["TASK-3"]),
                ]
            } else {
                vec![
                    task("TASK-1", "Observe current state and evidence", &[]),
                    task(
                        "TASK-2",
                        "Structure completion conditions and constraints",
                        &["TASK-1"],
                    ),
                    task(
                        "TASK-3",
                        "Draft and validate candidate actions",
                        &["TASK-2"],
                    ),
                    task("TASK-4", "Apply the verified result", &["TASK-3"]),
                ]
            },
            risks: vec![if korean {
                "관찰되지 않은 사실을 확정하지 않음".to_string()
            } else {
                "Do not assert unobserved facts".to_string()
            }],
            assumptions: Vec::new(),
        }),
    }
}

fn create_user_guide(document_id: &str, command: &str, korean: bool) -> UserGuideIR {
    let title = guide_title(command, korean);
    let sections = if korean {
        vec![
            GuideSectionIR {
                section_id: "GUIDE-START".to_string(),
                heading: "빠른 시작".to_string(),
                body: "목표와 필요한 결과를 먼저 정한 뒤, 관련 자료와 제약을 함께 제공한다. 제품별 화면이나 기능의 존재 여부는 연결된 공식 자료로 확인해야 한다.".to_string(),
                steps: vec![
                    "하려는 일을 한 문장으로 정한다.".to_string(),
                    "판단에 필요한 배경과 자료를 제공한다.".to_string(),
                    "분량, 형식, 독자와 금지 조건을 지정한다.".to_string(),
                    "결과의 사실·수치·출처를 검토하고 필요한 부분을 다시 요청한다.".to_string(),
                ],
            },
            GuideSectionIR {
                section_id: "GUIDE-WORK".to_string(),
                heading: "주요 사용 방식".to_string(),
                body: "제공된 자료의 요약·분류·비교, 초안 작성, 설명, 아이디어 구조화, 표와 체크리스트 변환 같은 대화형 작업을 요청할 수 있다. 실제 계정에서 사용할 수 있는 제품 기능과 제한은 확인 필요이다.".to_string(),
                steps: Vec::new(),
            },
            GuideSectionIR {
                section_id: "GUIDE-PROMPT".to_string(),
                heading: "좋은 질문 작성법".to_string(),
                body: "역할이나 관점, 목표, 배경, 제약, 출력 형식, 검증 조건을 한 요청 안에 명시한다. 모르는 사실을 추측하지 말고 불확실성을 구분하라는 조건을 덧붙인다.".to_string(),
                steps: vec![
                    "목표: 무엇을 완료해야 하는가?".to_string(),
                    "맥락: 어떤 자료와 상황을 알아야 하는가?".to_string(),
                    "제약: 분량, 독자, 금지사항은 무엇인가?".to_string(),
                    "형식: 표, 목록, 문서 등 어떤 형태가 필요한가?".to_string(),
                    "검증: 어떤 근거로 완료를 확인할 것인가?".to_string(),
                ],
            },
            GuideSectionIR {
                section_id: "GUIDE-ITERATE".to_string(),
                heading: "단계별 사용 예시".to_string(),
                body: "첫 응답을 최종본으로 간주하지 않는다. 초안 생성, 누락 검토, 근거 확인, 형식 정리의 순서로 반복하면 결과의 통제 가능성이 높아진다.".to_string(),
                steps: vec![
                    "초안을 요청한다.".to_string(),
                    "주장과 근거를 분리해 달라고 요청한다.".to_string(),
                    "누락·모순·불확실성을 점검한다.".to_string(),
                    "검증된 내용만 원하는 형식으로 다시 작성한다.".to_string(),
                ],
            },
        ]
    } else {
        vec![
            GuideSectionIR {
                section_id: "GUIDE-START".to_string(),
                heading: "Quick start".to_string(),
                body: "Define the outcome first, then provide relevant material and constraints. Product-specific interface features must be confirmed against connected official documentation.".to_string(),
                steps: vec![
                    "State the task in one sentence.".to_string(),
                    "Provide the background and source material.".to_string(),
                    "Specify audience, length, format, and prohibitions.".to_string(),
                    "Review facts, figures, and sources before reuse.".to_string(),
                ],
            },
            GuideSectionIR {
                section_id: "GUIDE-WORK".to_string(),
                heading: "Common workflows".to_string(),
                body: "You can request source-grounded summarization, classification, comparison, explanation, drafting, and conversion into tables or checklists. Product availability and account limits require confirmation.".to_string(),
                steps: Vec::new(),
            },
            GuideSectionIR {
                section_id: "GUIDE-PROMPT".to_string(),
                heading: "Write an effective request".to_string(),
                body: "Specify the perspective, goal, context, constraints, output form, and verification condition. Ask for uncertainty to be separated from observed facts.".to_string(),
                steps: vec![
                    "Goal: what must be completed?".to_string(),
                    "Context: what material and situation matter?".to_string(),
                    "Constraints: audience, length, and prohibitions?".to_string(),
                    "Format: table, list, or document?".to_string(),
                    "Verification: what evidence proves completion?".to_string(),
                ],
            },
            GuideSectionIR {
                section_id: "GUIDE-ITERATE".to_string(),
                heading: "Iterate in stages".to_string(),
                body: "Treat the first response as a draft. Generate, inspect omissions, verify evidence, and only then format the final result.".to_string(),
                steps: vec![
                    "Request a draft.".to_string(),
                    "Separate claims from evidence.".to_string(),
                    "Inspect omissions, conflicts, and uncertainty.".to_string(),
                    "Rewrite only verified content in the target format.".to_string(),
                ],
            },
        ]
    };
    UserGuideIR {
        schema: USER_GUIDE_SCHEMA.to_string(),
        document_id: document_id.to_string(),
        title,
        audience: if korean {
            "처음 사용하는 사용자"
        } else {
            "First-time users"
        }
        .to_string(),
        introduction: if korean {
            "이 안내서는 대화형 도구에 작업을 명확히 요청하고 결과를 검증하는 기본 절차를 설명한다. 현재 제품의 구체적인 화면, 요금, 모델, 한도와 기능은 공식 자료가 제공되지 않았으므로 확인 필요이다."
        } else {
            "This guide explains how to request work clearly and verify the result. Specific current product screens, pricing, models, limits, and features require official source material."
        }
        .to_string(),
        sections,
        examples: vec![GuideExampleIR {
            title: if korean { "자료 요약 요청" } else { "Source summary request" }.to_string(),
            input: if korean {
                "다음 자료를 핵심 주장, 근거, 불확실성으로 나눠 요약해. 자료 밖의 사실은 추가하지 마."
            } else {
                "Summarize this material as claims, evidence, and uncertainty. Do not add facts outside the source."
            }
            .to_string(),
            expected_result: if korean {
                "주장과 근거가 분리되고 확인되지 않은 내용이 표시된 요약"
            } else {
                "A summary separating claims, evidence, and unverified content"
            }
            .to_string(),
        }],
        cautions: if korean {
            vec![
                "중요한 사실·수치·인용은 원문 또는 신뢰할 수 있는 출처로 다시 확인한다.".to_string(),
                "민감하거나 비공개인 자료는 적용되는 정책과 권한을 확인한 뒤 사용한다.".to_string(),
                "관찰된 사실, 추론, 제안을 서로 구분한다.".to_string(),
            ]
        } else {
            vec![
                "Recheck important facts, figures, and quotations against a reliable source.".to_string(),
                "Confirm applicable policy and authority before providing sensitive material.".to_string(),
                "Keep observations, inferences, and proposals separate.".to_string(),
            ]
        },
        troubleshooting: if korean {
            vec![
                TroubleshootingItemIR { symptom: "답변이 너무 일반적임".to_string(), resolution: "독자, 목적, 사용 자료, 제외할 내용과 출력 예시를 추가한다.".to_string() },
                TroubleshootingItemIR { symptom: "원하는 형식과 다름".to_string(), resolution: "열 이름, 목차, 길이 또는 JSON 같은 목표 형식을 명시한다.".to_string() },
                TroubleshootingItemIR { symptom: "근거 없는 내용이 섞임".to_string(), resolution: "각 주장에 출처 위치를 붙이고 근거 없는 항목은 확인 필요로 분리하도록 요청한다.".to_string() },
            ]
        } else {
            vec![
                TroubleshootingItemIR { symptom: "The response is too generic".to_string(), resolution: "Add the audience, objective, source material, exclusions, and an output example.".to_string() },
                TroubleshootingItemIR { symptom: "The format is wrong".to_string(), resolution: "Specify column names, outline, length, or a target schema such as JSON.".to_string() },
                TroubleshootingItemIR { symptom: "Unsupported claims appear".to_string(), resolution: "Require source locations for each claim and move unsupported items to needs confirmation.".to_string() },
            ]
        },
        checklist: if korean {
            vec![
                "목표와 대상 독자를 적었는가?".to_string(),
                "필요한 자료와 맥락을 제공했는가?".to_string(),
                "제약과 출력 형식을 지정했는가?".to_string(),
                "사실·수치·출처를 검토했는가?".to_string(),
                "확인 필요 항목을 분리했는가?".to_string(),
            ]
        } else {
            vec![
                "Did I state the goal and audience?".to_string(),
                "Did I provide the required source material and context?".to_string(),
                "Did I specify constraints and output format?".to_string(),
                "Did I verify facts, figures, and sources?".to_string(),
                "Did I separate items that need confirmation?".to_string(),
            ]
        },
        tables: Vec::new(),
        charts: Vec::new(),
    }
}

fn guide_title(command: &str, korean: bool) -> String {
    let markers: &[&str] = if korean {
        &[
            "사용 설명서",
            "사용설명서",
            "사용자 가이드",
            "설명서",
            "매뉴얼",
            "안내서",
        ]
    } else {
        &["user guide", "user manual", "manual", "how-to guide"]
    };
    let lowered = command.to_lowercase();
    if let Some((index, marker)) = markers
        .iter()
        .filter_map(|marker| lowered.find(marker).map(|index| (index, *marker)))
        .min_by_key(|(index, _)| *index)
    {
        let end = index + marker.len();
        let mut title = command[..end].trim().to_string();
        if !korean {
            for prefix in [
                "create a ",
                "create an ",
                "write a ",
                "write an ",
                "draft a ",
            ] {
                if title.to_lowercase().starts_with(prefix) {
                    title = title[prefix.len()..].trim().to_string();
                    break;
                }
            }
        }
        if !title.is_empty() {
            return title;
        }
    }
    let first_sentence = command
        .split(['.', '!', '?', '\n'])
        .next()
        .unwrap_or(command)
        .trim();
    let endings = if korean {
        ["작성해", "작성", "만들어", "써줘"]
    } else {
        ["write", "create", "author", "draft"]
    };
    let cleaned = endings
        .iter()
        .fold(first_sentence.to_string(), |value, ending| {
            value.trim_end_matches(ending).trim().to_string()
        });
    if cleaned.is_empty() {
        if korean {
            "사용 설명서".to_string()
        } else {
            "User guide".to_string()
        }
    } else {
        cleaned
    }
}

fn create_business_document(
    document_id: &str,
    subject: &str,
    document_type: BusinessDocumentTypeIR,
    korean: bool,
) -> BusinessDocumentIR {
    let (headings, summary, next_action) = match (document_type, korean) {
        (BusinessDocumentTypeIR::BusinessPlan, true) => (
            vec!["사업 기회", "고객과 시장", "제품·서비스", "수익 모델", "실행 전략", "재무 계획"],
            format!("{subject}의 시장 근거, 실행 모델, 재무 가정과 검증 조건을 하나의 사업계획으로 구조화한다."),
            "검증 가능한 시장·재무 자료를 연결하고 실행 승인 여부를 결정한다.".to_string(),
        ),
        (BusinessDocumentTypeIR::BusinessProposal, true) => (
            vec!["제안 배경", "고객 과제", "제안 솔루션", "제공 범위", "추진 일정", "투자·계약 조건"],
            format!("{subject}에 대한 고객 가치, 제공 범위, 일정과 의사결정 조건을 명확히 제안한다."),
            "제안 범위와 조건을 확인하고 다음 협의 일정을 확정한다.".to_string(),
        ),
        (BusinessDocumentTypeIR::BusinessPlan, false) => (
            vec!["Opportunity", "Customer & Market", "Product or Service", "Business Model", "Execution Strategy", "Financial Plan"],
            format!("This business plan structures the market evidence, operating model, financial assumptions, and verification conditions for {subject}."),
            "Connect verified market and financial evidence, then decide whether to approve execution.".to_string(),
        ),
        (BusinessDocumentTypeIR::BusinessProposal, false) => (
            vec!["Proposal Context", "Client Challenge", "Proposed Solution", "Scope", "Delivery Roadmap", "Commercial Terms"],
            format!("This proposal presents the customer value, delivery scope, schedule, and decision conditions for {subject}."),
            "Confirm the proposed scope and terms, then schedule the next decision meeting.".to_string(),
        ),
    };
    let sections = headings
        .into_iter()
        .enumerate()
        .map(|(index, heading)| BusinessSectionIR {
            section_id: format!("BSEC-{}", index + 1),
            heading: heading.to_string(),
            body: if korean {
                format!("{heading}에 관한 검증된 자료와 판단 근거를 배치한다.")
            } else {
                format!("Place verified evidence and decision rationale for {heading} here.")
            },
            highlights: Vec::new(),
        })
        .collect::<Vec<_>>();
    BusinessDocumentIR {
        schema: BUSINESS_DOCUMENT_SCHEMA.to_string(),
        document_id: document_id.to_string(),
        document_type,
        title: subject.to_string(),
        organization: if korean {
            "조직명 미지정"
        } else {
            "Organization not specified"
        }
        .to_string(),
        audience: if korean {
            "의사결정자"
        } else {
            "Decision makers"
        }
        .to_string(),
        executive_summary: summary,
        sections,
        key_metrics: Vec::new(),
        execution_plan: PlanProposalIR {
            schema: PLAN_PROPOSAL_SCHEMA.to_string(),
            document_id: format!("{document_id}-EXECUTION"),
            title: if korean {
                "실행 로드맵"
            } else {
                "Execution roadmap"
            }
            .to_string(),
            objective: subject.to_string(),
            tasks: if korean {
                vec![
                    task("PHASE-1", "근거와 요구사항 검증", &[]),
                    task("PHASE-2", "핵심 가설의 제한된 검증", &["PHASE-1"]),
                    task("PHASE-3", "실행·측정·조정", &["PHASE-2"]),
                ]
            } else {
                vec![
                    task("PHASE-1", "Validate evidence and requirements", &[]),
                    task(
                        "PHASE-2",
                        "Run a bounded validation of key assumptions",
                        &["PHASE-1"],
                    ),
                    task("PHASE-3", "Execute, measure, and adjust", &["PHASE-2"]),
                ]
            },
            risks: Vec::new(),
            assumptions: Vec::new(),
        },
        tables: Vec::new(),
        charts: Vec::new(),
        financial_statements: Vec::new(),
        risks: vec![if korean {
            "자료로 검증되지 않은 수치와 주장은 확정하지 않는다."
        } else {
            "Do not treat figures or claims as final until they are evidence-backed."
        }
        .to_string()],
        next_action,
    }
}

fn parse_business(
    document_id: &str,
    text: &str,
    document_type: BusinessDocumentTypeIR,
) -> BusinessDocumentIR {
    let paper = parse_paper(document_id, text);
    let korean = detect_output_language(text) == LanguageCodeIR::Korean;
    let mut metrics = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let Some(raw) = strip_prefix_any(line.trim(), &["KPI:", "지표:", "Metric:"]) else {
            continue;
        };
        let parts = raw.split('|').map(str::trim).collect::<Vec<_>>();
        if parts.len() >= 2 {
            metrics.push(BusinessMetricIR {
                label: parts[0].to_string(),
                value: parts[1].to_string(),
                change: parts
                    .get(2)
                    .filter(|value| !value.is_empty())
                    .map(|value| (*value).to_string()),
                evidence_location: format!("line:{}", line_index + 1),
            });
        }
    }
    let sections = paper
        .sections
        .iter()
        .filter(|section| {
            !matches_any(
                &normalize(&section.heading),
                &["abstract", "초록", "요약", "executive summary", "핵심 요약"],
            )
        })
        .map(|section| BusinessSectionIR {
            section_id: section.section_id.replace("SEC", "BSEC"),
            heading: section.heading.clone(),
            body: section.body.clone(),
            highlights: section
                .body
                .lines()
                .filter_map(|line| line.trim().strip_prefix("- ").map(str::to_string))
                .take(8)
                .collect(),
        })
        .collect::<Vec<_>>();
    let executive_summary = paper
        .sections
        .iter()
        .find(|section| {
            matches_any(
                &normalize(&section.heading),
                &["abstract", "초록", "요약", "executive summary", "핵심 요약"],
            )
        })
        .map(|section| section.body.clone())
        .filter(|body| !body.is_empty())
        .unwrap_or_else(|| paper.abstract_text.clone());
    BusinessDocumentIR {
        schema: BUSINESS_DOCUMENT_SCHEMA.to_string(),
        document_id: document_id.to_string(),
        document_type,
        title: paper.title,
        organization: if korean {
            "조직명 미지정"
        } else {
            "Organization not specified"
        }
        .to_string(),
        audience: if korean {
            "의사결정자"
        } else {
            "Decision makers"
        }
        .to_string(),
        executive_summary,
        sections,
        key_metrics: metrics,
        execution_plan: parse_plan(&format!("{document_id}-EXECUTION"), text),
        tables: Vec::new(),
        charts: Vec::new(),
        financial_statements: Vec::new(),
        risks: Vec::new(),
        next_action: if korean {
            "다음 의사결정 조건을 확인한다."
        } else {
            "Confirm the next decision condition."
        }
        .to_string(),
    }
}

fn parse_user_guide(document_id: &str, text: &str) -> UserGuideIR {
    let paper = parse_paper(document_id, text);
    let korean = detect_output_language(text) == LanguageCodeIR::Korean;
    let mut sections = Vec::new();
    let mut cautions = Vec::new();
    let mut troubleshooting = Vec::new();
    let mut checklist = Vec::new();
    for section in &paper.sections {
        let heading = normalize(&section.heading);
        let list_items = section
            .body
            .lines()
            .filter_map(|line| {
                line.trim()
                    .strip_prefix("- ")
                    .or_else(|| line.trim().strip_prefix("* "))
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_string)
            })
            .collect::<Vec<_>>();
        if matches_any(&heading, &["주의사항", "주의", "cautions", "warnings"]) {
            cautions.extend(list_items);
        } else if matches_any(
            &heading,
            &[
                "빠른 확인 목록",
                "체크리스트",
                "checklist",
                "quick checklist",
            ],
        ) {
            checklist.extend(list_items);
        } else if matches_any(
            &heading,
            &["문제 해결", "문제해결", "troubleshooting", "troubleshoot"],
        ) {
            troubleshooting.extend(list_items.into_iter().map(|item| {
                let (symptom, resolution) = item
                    .split_once('|')
                    .or_else(|| item.split_once(':'))
                    .unwrap_or((
                        item.as_str(),
                        if korean {
                            "확인 필요"
                        } else {
                            "Needs confirmation"
                        },
                    ));
                TroubleshootingItemIR {
                    symptom: symptom.trim().to_string(),
                    resolution: resolution.trim().to_string(),
                }
            }));
        } else {
            sections.push(GuideSectionIR {
                section_id: section.section_id.replace("SEC", "GUIDE"),
                heading: section.heading.clone(),
                body: section
                    .body
                    .lines()
                    .filter(|line| {
                        let trimmed = line.trim();
                        !trimmed.starts_with("- ") && !trimmed.starts_with("* ")
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
                    .trim()
                    .to_string(),
                steps: list_items,
            });
        }
    }
    UserGuideIR {
        schema: USER_GUIDE_SCHEMA.to_string(),
        document_id: document_id.to_string(),
        title: paper.title,
        audience: if korean { "사용자" } else { "Users" }.to_string(),
        introduction: paper.abstract_text,
        sections,
        examples: Vec::new(),
        cautions,
        troubleshooting,
        checklist,
        tables: paper.tables,
        charts: paper.charts,
    }
}

fn parse_paper(document_id: &str, text: &str) -> PaperIR {
    let mut title = String::new();
    let mut authors = Vec::new();
    let mut abstract_text = String::new();
    let mut sections = Vec::new();
    let mut current_heading = "본문".to_string();
    let mut current_level = 2_u8;
    let mut body = Vec::new();
    let flush =
        |sections: &mut Vec<PaperSectionIR>, heading: &str, level: u8, body: &mut Vec<String>| {
            if !body.is_empty() {
                sections.push(PaperSectionIR {
                    section_id: format!("SEC-{}", sections.len() + 1),
                    heading: heading.to_string(),
                    body: body.join("\n").trim().to_string(),
                    level,
                });
                body.clear();
            }
        };
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            if title.is_empty() {
                title = heading.trim().to_string();
            } else {
                flush(&mut sections, &current_heading, current_level, &mut body);
                current_heading = heading.trim().to_string();
                current_level = 1;
            }
        } else if trimmed.starts_with('#') {
            flush(&mut sections, &current_heading, current_level, &mut body);
            let level = trimmed
                .chars()
                .take_while(|character| *character == '#')
                .count();
            current_level = u8::try_from(level.min(6)).unwrap_or(6);
            current_heading = trimmed[level..].trim().to_string();
        } else if let Some(value) = strip_prefix_any(trimmed, &["Authors:", "Author:", "저자:"]) {
            authors = value
                .split(',')
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect();
        } else if let Some(value) = strip_prefix_any(trimmed, &["Abstract:", "초록:"]) {
            abstract_text = value.trim().to_string();
        } else {
            body.push(line.to_string());
        }
    }
    flush(&mut sections, &current_heading, current_level, &mut body);
    if title.is_empty() {
        title = text
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("Untitled paper")
            .trim()
            .to_string();
    }
    if abstract_text.is_empty() {
        abstract_text = sections
            .iter()
            .find(|section| {
                matches_any(&normalize(&section.heading), &["abstract", "초록", "요약"])
            })
            .map(|section| section.body.clone())
            .unwrap_or_default();
    }
    let claims = extract_claims(&sections);
    let references = extract_references(&sections);
    PaperIR {
        schema: PAPER_SCHEMA.to_string(),
        document_id: document_id.to_string(),
        title,
        authors,
        abstract_text,
        sections,
        claims,
        references,
        tables: Vec::new(),
        charts: Vec::new(),
    }
}

fn parse_table(document_id: &str, text: &str) -> Result<TableIR, KnowledgeWorkError> {
    let lines = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Err(KnowledgeWorkError::ParseFailure);
    }
    let markdown = lines.iter().any(|line| line.matches('|').count() >= 2);
    let delimiter = if markdown {
        '|'
    } else if lines[0].contains('\t') {
        '\t'
    } else {
        ','
    };
    let split = |line: &str| {
        let mut cells = split_delimited(line, delimiter);
        if delimiter == '|' && cells.first().is_some_and(String::is_empty) {
            cells.remove(0);
        }
        if delimiter == '|' && cells.last().is_some_and(String::is_empty) {
            cells.pop();
        }
        cells
    };
    let columns = split(lines[0]);
    if columns.is_empty() || columns.len() > 256 {
        return Err(KnowledgeWorkError::ParseFailure);
    }
    let mut rows = Vec::new();
    for (line_index, line) in lines.iter().enumerate().skip(1) {
        let values = split(line);
        if delimiter == '|'
            && values.len() == columns.len()
            && values.iter().all(|value| {
                value
                    .chars()
                    .all(|character| matches!(character, '-' | ':' | ' '))
            })
        {
            continue;
        }
        if values.len() != columns.len() || rows.len() >= 100_000 {
            return Err(KnowledgeWorkError::ParseFailure);
        }
        rows.push(
            values
                .into_iter()
                .enumerate()
                .map(|(column_index, raw)| TableCellIR {
                    value: parse_cell(&raw),
                    raw,
                    source_location: format!("row:{}:column:{}", line_index + 1, column_index + 1),
                })
                .collect(),
        );
    }
    Ok(TableIR {
        schema: TABLE_SCHEMA.to_string(),
        document_id: document_id.to_string(),
        title: "Structured table".to_string(),
        columns,
        rows,
        notes: Vec::new(),
    })
}

fn chart_from_table(document_id: &str, table: &TableIR) -> Result<ChartIR, KnowledgeWorkError> {
    if table.columns.len() < 2 {
        return Err(KnowledgeWorkError::ParseFailure);
    }
    let mut series = Vec::new();
    for column in 1..table.columns.len() {
        let points = table
            .rows
            .iter()
            .map(|row| ChartPointIR {
                category: row.first().map(|cell| cell.raw.clone()).unwrap_or_default(),
                value: row.get(column).and_then(|cell| match &cell.value {
                    CellValueIR::Number(value) => Some(value.clone()),
                    _ => None,
                }),
                source_location: row
                    .get(column)
                    .map(|cell| cell.source_location.clone())
                    .unwrap_or_default(),
            })
            .collect::<Vec<_>>();
        if points.iter().any(|point| point.value.is_some()) {
            series.push(ChartSeriesIR {
                name: table.columns[column].clone(),
                points,
            });
        }
    }
    if series.is_empty() {
        return Err(KnowledgeWorkError::ParseFailure);
    }
    Ok(ChartIR {
        schema: CHART_SCHEMA.to_string(),
        document_id: document_id.to_string(),
        title: table.title.clone(),
        chart_type: ChartTypeIR::Line,
        category_axis: table.columns[0].clone(),
        value_axis: "value".to_string(),
        series,
    })
}

fn financial_from_table(
    document_id: &str,
    table: &TableIR,
) -> Result<FinancialStatementIR, KnowledgeWorkError> {
    if table.columns.len() < 2 {
        return Err(KnowledgeWorkError::ParseFailure);
    }
    let periods = table.columns[1..].to_vec();
    let mut items = Vec::new();
    for (row_index, row) in table.rows.iter().enumerate() {
        let label = row
            .first()
            .map(|cell| cell.raw.trim().to_string())
            .unwrap_or_default();
        if label.is_empty() {
            continue;
        }
        let mut values_by_period = BTreeMap::new();
        for (column_index, period) in periods.iter().enumerate() {
            if let Some(CellValueIR::Number(value)) =
                row.get(column_index + 1).map(|cell| &cell.value)
            {
                values_by_period.insert(period.clone(), value.clone());
            }
        }
        items.push(FinancialLineItemIR {
            normalized_concept: normalize_financial_concept(&label),
            class: classify_financial_line(&label),
            label,
            values_by_period,
            source_location: format!("row:{}", row_index + 2),
        });
    }
    let statement_evidence = table
        .rows
        .iter()
        .filter_map(|row| row.first())
        .map(|cell| cell.raw.as_str())
        .chain(table.columns.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    Ok(FinancialStatementIR {
        schema: FINANCIAL_STATEMENT_SCHEMA.to_string(),
        document_id: document_id.to_string(),
        entity: table.title.clone(),
        statement_type: infer_statement_type(&statement_evidence),
        periods,
        currency: infer_currency(table),
        display_unit: "1".to_string(),
        line_items: items,
    })
}

fn parse_plan(document_id: &str, text: &str) -> PlanProposalIR {
    let mut title = "Plan proposal".to_string();
    let mut objective = String::new();
    let mut tasks = Vec::new();
    let mut risks = Vec::new();
    let mut assumptions = Vec::new();
    let mut section = "tasks";
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("# ") {
            title = value.trim().to_string();
        } else if let Some(value) = strip_prefix_any(trimmed, &["목표:", "Objective:"]) {
            objective = value.trim().to_string();
        } else if trimmed.starts_with('#') {
            let heading = normalize(trimmed.trim_start_matches('#').trim());
            section = if contains_any(&heading, &["risk", "위험", "리스크"]) {
                "risks"
            } else if contains_any(&heading, &["assumption", "가정"]) {
                "assumptions"
            } else {
                "tasks"
            };
        } else if let Some(item) = trimmed.strip_prefix("- ") {
            match section {
                "risks" => risks.push(item.to_string()),
                "assumptions" => assumptions.push(item.to_string()),
                _ => {
                    let task_id = format!("TASK-{}", tasks.len() + 1);
                    let dependencies = tasks
                        .last()
                        .map(|previous: &PlanTaskIR| vec![previous.task_id.clone()])
                        .unwrap_or_default();
                    tasks.push(PlanTaskIR {
                        task_id,
                        description: item.to_string(),
                        dependencies,
                        owner: None,
                        completion_condition: None,
                    });
                }
            }
        }
    }
    if objective.is_empty() {
        objective = title.clone();
    }
    PlanProposalIR {
        schema: PLAN_PROPOSAL_SCHEMA.to_string(),
        document_id: document_id.to_string(),
        title,
        objective,
        tasks,
        risks,
        assumptions,
    }
}

fn revise_document(
    document: &mut KnowledgeDocumentIR,
    command: &str,
) -> Result<(), KnowledgeWorkError> {
    let title = value_after_marker(command, &["제목:", "title:"]);
    let mut changed = false;
    if let Some(title) = title.filter(|value| !value.is_empty()) {
        match document {
            KnowledgeDocumentIR::Paper(value) => value.title = title,
            KnowledgeDocumentIR::BusinessPlan(value)
            | KnowledgeDocumentIR::BusinessProposal(value) => value.title = title,
            KnowledgeDocumentIR::UserGuide(value) => value.title = title,
            KnowledgeDocumentIR::Table(value) => value.title = title,
            KnowledgeDocumentIR::Chart(value) => value.title = title,
            KnowledgeDocumentIR::FinancialStatement(value) => value.entity = title,
            KnowledgeDocumentIR::PlanProposal(value) => value.title = title,
        }
        changed = true;
    }
    match document {
        KnowledgeDocumentIR::Paper(paper) => {
            if let Some(value) = value_after_marker(command, &["초록:", "abstract:"]) {
                paper.abstract_text = value;
                changed = true;
            }
            if let Some(value) = value_after_marker(command, &["섹션 추가:", "add section:"]) {
                let (heading, body) = value.split_once('|').unwrap_or((value.as_str(), ""));
                paper.sections.push(PaperSectionIR {
                    section_id: format!("SEC-{}", paper.sections.len() + 1),
                    heading: heading.trim().to_string(),
                    body: body.trim().to_string(),
                    level: 2,
                });
                changed = true;
            }
        }
        KnowledgeDocumentIR::BusinessPlan(business)
        | KnowledgeDocumentIR::BusinessProposal(business) => {
            if let Some(value) =
                value_after_marker(command, &["핵심 요약:", "요약:", "executive summary:"])
            {
                business.executive_summary = value;
                changed = true;
            }
            if let Some(value) = value_after_marker(command, &["조직:", "organization:"]) {
                business.organization = value;
                changed = true;
            }
            if let Some(value) = value_after_marker(command, &["대상:", "audience:"]) {
                business.audience = value;
                changed = true;
            }
            if let Some(value) = value_after_marker(command, &["섹션 추가:", "add section:"]) {
                let (heading, body) = value.split_once('|').unwrap_or((value.as_str(), ""));
                business.sections.push(BusinessSectionIR {
                    section_id: format!("BSEC-{}", business.sections.len() + 1),
                    heading: heading.trim().to_string(),
                    body: body.trim().to_string(),
                    highlights: Vec::new(),
                });
                changed = true;
            }
            if let Some(value) = value_after_marker(command, &["지표 추가:", "add metric:"]) {
                let parts = value.split('|').map(str::trim).collect::<Vec<_>>();
                if parts.len() < 2 {
                    return Err(KnowledgeWorkError::RevisionNotGrounded);
                }
                business.key_metrics.push(BusinessMetricIR {
                    label: parts[0].to_string(),
                    value: parts[1].to_string(),
                    change: parts
                        .get(2)
                        .filter(|value| !value.is_empty())
                        .map(|value| (*value).to_string()),
                    evidence_location: "revision:operator".to_string(),
                });
                changed = true;
            }
            if let Some(value) = value_after_marker(command, &["다음 단계:", "next action:"]) {
                business.next_action = value;
                changed = true;
            }
            if let Some(value) = value_after_marker(command, &["위험 추가:", "add risk:"]) {
                business.risks.push(value);
                changed = true;
            }
        }
        KnowledgeDocumentIR::UserGuide(guide) => {
            if let Some(value) = value_after_marker(command, &["소개:", "introduction:"]) {
                guide.introduction = value;
                changed = true;
            }
            if let Some(value) = value_after_marker(command, &["대상:", "audience:"]) {
                guide.audience = value;
                changed = true;
            }
            if let Some(value) = value_after_marker(command, &["섹션 추가:", "add section:"]) {
                let (heading, body) = value.split_once('|').unwrap_or((value.as_str(), ""));
                guide.sections.push(GuideSectionIR {
                    section_id: format!("GUIDE-{}", guide.sections.len() + 1),
                    heading: heading.trim().to_string(),
                    body: body.trim().to_string(),
                    steps: Vec::new(),
                });
                changed = true;
            }
            if let Some(value) = value_after_marker(command, &["주의 추가:", "add caution:"]) {
                guide.cautions.push(value);
                changed = true;
            }
            if let Some(value) = value_after_marker(command, &["확인 추가:", "add checklist:"])
            {
                guide.checklist.push(value);
                changed = true;
            }
        }
        KnowledgeDocumentIR::Table(table) => {
            if let Some(value) = value_after_marker(command, &["행 추가:", "add row:"]) {
                let cells = split_delimited(&value, ',');
                if cells.len() != table.columns.len() {
                    return Err(KnowledgeWorkError::RevisionNotGrounded);
                }
                let row_index = table.rows.len() + 2;
                table.rows.push(
                    cells
                        .into_iter()
                        .enumerate()
                        .map(|(column_index, raw)| TableCellIR {
                            value: parse_cell(&raw),
                            raw,
                            source_location: format!(
                                "revision:row:{row_index}:column:{}",
                                column_index + 1
                            ),
                        })
                        .collect(),
                );
                changed = true;
            }
        }
        KnowledgeDocumentIR::Chart(chart) => {
            if contains_any(&normalize(command), &["막대", "bar chart"]) {
                chart.chart_type = ChartTypeIR::Bar;
                changed = true;
            }
            if contains_any(&normalize(command), &["선형", "line chart"]) {
                chart.chart_type = ChartTypeIR::Line;
                changed = true;
            }
        }
        KnowledgeDocumentIR::FinancialStatement(statement) => {
            if let Some(value) = value_after_marker(command, &["통화:", "currency:"]) {
                statement.currency = value;
                changed = true;
            }
            if let Some(value) = value_after_marker(command, &["단위:", "unit:"]) {
                statement.display_unit = value;
                changed = true;
            }
        }
        KnowledgeDocumentIR::PlanProposal(plan) => {
            if let Some(value) = value_after_marker(command, &["작업 추가:", "add task:"]) {
                let task_id = format!("TASK-{}", plan.tasks.len() + 1);
                let dependencies = plan
                    .tasks
                    .last()
                    .map(|task| vec![task.task_id.clone()])
                    .unwrap_or_default();
                plan.tasks.push(PlanTaskIR {
                    task_id,
                    description: value,
                    dependencies,
                    owner: None,
                    completion_condition: None,
                });
                changed = true;
            }
            if let Some(value) = value_after_marker(command, &["위험 추가:", "add risk:"]) {
                plan.risks.push(value);
                changed = true;
            }
        }
    }
    if changed {
        Ok(())
    } else {
        Err(KnowledgeWorkError::RevisionNotGrounded)
    }
}

pub fn analyze_document(document: &KnowledgeDocumentIR) -> Vec<KnowledgeFindingIR> {
    analyze_document_in_language(document, LanguageCodeIR::Korean)
}

pub fn analyze_document_in_language(
    document: &KnowledgeDocumentIR,
    language: LanguageCodeIR,
) -> Vec<KnowledgeFindingIR> {
    let korean = language == LanguageCodeIR::Korean;
    let mut findings = match document {
        KnowledgeDocumentIR::Paper(paper) => analyze_paper(paper, korean),
        KnowledgeDocumentIR::BusinessPlan(business)
        | KnowledgeDocumentIR::BusinessProposal(business) => analyze_business(business, korean),
        KnowledgeDocumentIR::UserGuide(guide) => analyze_user_guide(guide, korean),
        KnowledgeDocumentIR::Table(table) => analyze_table(table, korean),
        KnowledgeDocumentIR::Chart(chart) => analyze_chart(chart, korean),
        KnowledgeDocumentIR::FinancialStatement(statement) => analyze_financial(statement, korean),
        KnowledgeDocumentIR::PlanProposal(plan) => analyze_plan(plan, korean),
    };
    for (index, finding) in findings.iter_mut().enumerate() {
        finding.finding_id = format!("FINDING-{}", index + 1);
    }
    findings
}

fn analyze_user_guide(guide: &UserGuideIR, korean: bool) -> Vec<KnowledgeFindingIR> {
    let mut findings = vec![finding(
        FindingKindIR::Summary,
        if korean {
            format!(
                "설명서는 {}개 본문 절, {}개 예시, {}개 문제 해결 항목, {}개 확인 항목으로 구성됩니다.",
                guide.sections.len(),
                guide.examples.len(),
                guide.troubleshooting.len(),
                guide.checklist.len()
            )
        } else {
            format!(
                "The guide contains {} section(s), {} example(s), {} troubleshooting item(s), and {} checklist item(s).",
                guide.sections.len(),
                guide.examples.len(),
                guide.troubleshooting.len(),
                guide.checklist.len()
            )
        },
        vec!["user_guide".to_string()],
        1_000,
    )];
    for (location, empty, statement_ko, statement_en) in [
        (
            "introduction",
            guide.introduction.trim().is_empty(),
            "소개가 비어 있습니다.",
            "The introduction is empty.",
        ),
        (
            "sections",
            guide.sections.is_empty(),
            "사용 절차가 비어 있습니다.",
            "The usage procedure is empty.",
        ),
        (
            "troubleshooting",
            guide.troubleshooting.is_empty(),
            "문제 해결 항목이 비어 있습니다.",
            "Troubleshooting is empty.",
        ),
        (
            "checklist",
            guide.checklist.is_empty(),
            "빠른 확인 목록이 비어 있습니다.",
            "The quick checklist is empty.",
        ),
    ] {
        if empty {
            findings.push(finding(
                FindingKindIR::StructuralGap,
                if korean { statement_ko } else { statement_en }.to_string(),
                vec![location.to_string()],
                1_000,
            ));
        }
    }
    findings
}

fn analyze_paper(paper: &PaperIR, korean: bool) -> Vec<KnowledgeFindingIR> {
    let mut findings = vec![finding(
        FindingKindIR::Summary,
        if korean {
            format!(
                "논문은 {}개 절, {}개 추출 주장, {}개 참고문헌으로 구조화되었습니다.",
                paper.sections.len(),
                paper.claims.len(),
                paper.references.len()
            )
        } else {
            format!(
                "The paper contains {} section(s), {} extracted claim(s), and {} reference(s).",
                paper.sections.len(),
                paper.claims.len(),
                paper.references.len()
            )
        },
        vec!["document".to_string()],
        1_000,
    )];
    if paper.abstract_text.trim().is_empty() {
        findings.push(finding(
            FindingKindIR::StructuralGap,
            if korean {
                "초록이 식별되지 않았습니다."
            } else {
                "No abstract was identified."
            }
            .to_string(),
            vec!["abstract".to_string()],
            1_000,
        ));
    }
    if paper.references.is_empty() {
        findings.push(finding(
            FindingKindIR::MissingEvidence,
            if korean {
                "참고문헌 절 또는 인용 항목이 식별되지 않았습니다."
            } else {
                "No reference section or citation entry was identified."
            }
            .to_string(),
            vec!["references".to_string()],
            900,
        ));
    }
    findings
}

fn analyze_business(business: &BusinessDocumentIR, korean: bool) -> Vec<KnowledgeFindingIR> {
    let mut findings = vec![finding(
        FindingKindIR::Summary,
        if korean {
            format!(
                "문서는 {}개 전략 절, {}개 핵심 지표, {}개 실행 단계로 구성됩니다.",
                business.sections.len(),
                business.key_metrics.len(),
                business.execution_plan.tasks.len()
            )
        } else {
            format!(
                "The document contains {} strategy section(s), {} key metric(s), and {} execution stage(s).",
                business.sections.len(),
                business.key_metrics.len(),
                business.execution_plan.tasks.len()
            )
        },
        vec!["business_document".to_string()],
        1_000,
    )];
    if business.executive_summary.trim().is_empty() {
        findings.push(finding(
            FindingKindIR::StructuralGap,
            if korean {
                "핵심 요약이 비어 있습니다."
            } else {
                "The executive summary is empty."
            }
            .to_string(),
            vec!["executive_summary".to_string()],
            1_000,
        ));
    }
    for metric in &business.key_metrics {
        if metric.evidence_location.trim().is_empty() {
            findings.push(finding(
                FindingKindIR::MissingEvidence,
                if korean {
                    format!("'{}' 지표에 근거 위치가 없습니다.", metric.label)
                } else {
                    format!("Metric '{}' has no evidence location.", metric.label)
                },
                vec![format!("metric:{}", metric.label)],
                1_000,
            ));
        }
    }
    findings.extend(analyze_plan(&business.execution_plan, korean));
    findings
}

fn analyze_table(table: &TableIR, korean: bool) -> Vec<KnowledgeFindingIR> {
    let mut findings = vec![finding(
        FindingKindIR::Summary,
        if korean {
            format!(
                "표는 {}개 열과 {}개 데이터 행을 가집니다.",
                table.columns.len(),
                table.rows.len()
            )
        } else {
            format!(
                "The table contains {} column(s) and {} data row(s).",
                table.columns.len(),
                table.rows.len()
            )
        },
        vec!["table".to_string()],
        1_000,
    )];
    for (column_index, column) in table.columns.iter().enumerate() {
        let values = table
            .rows
            .iter()
            .filter_map(|row| row.get(column_index))
            .filter_map(|cell| {
                if let CellValueIR::Number(number) = &cell.value {
                    Some((number, cell.source_location.clone()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if values.is_empty() {
            continue;
        }
        if let Some((number, location)) = values
            .iter()
            .max_by(|(left, _), (right, _)| numeric_cmp(left, right))
        {
            findings.push(finding(
                FindingKindIR::Maximum,
                if korean {
                    format!("'{column}' 열의 관측 최대값은 {}입니다.", number.original)
                } else {
                    format!(
                        "The observed maximum in column '{column}' is {}.",
                        number.original
                    )
                },
                vec![location.clone()],
                1_000,
            ));
        }
        if let Some((number, location)) = values
            .iter()
            .min_by(|(left, _), (right, _)| numeric_cmp(left, right))
        {
            findings.push(finding(
                FindingKindIR::Minimum,
                if korean {
                    format!("'{column}' 열의 관측 최소값은 {}입니다.", number.original)
                } else {
                    format!(
                        "The observed minimum in column '{column}' is {}.",
                        number.original
                    )
                },
                vec![location.clone()],
                1_000,
            ));
        }
    }
    findings
}

fn analyze_chart(chart: &ChartIR, korean: bool) -> Vec<KnowledgeFindingIR> {
    let mut findings = vec![finding(
        FindingKindIR::Summary,
        if korean {
            format!(
                "차트는 {:?} 유형이며 {}개 계열을 포함합니다.",
                chart.chart_type,
                chart.series.len()
            )
        } else {
            format!(
                "The chart is {:?} and contains {} series.",
                chart.chart_type,
                chart.series.len()
            )
        },
        vec!["chart".to_string()],
        1_000,
    )];
    for series in &chart.series {
        let observed = series
            .points
            .iter()
            .filter_map(|point| {
                point
                    .value
                    .as_ref()
                    .map(|value| (value, &point.source_location))
            })
            .collect::<Vec<_>>();
        if let (Some((first, first_location)), Some((last, last_location))) =
            (observed.first(), observed.last())
        {
            let direction = if korean {
                match numeric_cmp(first, last) {
                    std::cmp::Ordering::Less => "상승",
                    std::cmp::Ordering::Greater => "하락",
                    std::cmp::Ordering::Equal => "변화 없음",
                }
            } else {
                match numeric_cmp(first, last) {
                    std::cmp::Ordering::Less => "increased",
                    std::cmp::Ordering::Greater => "decreased",
                    std::cmp::Ordering::Equal => "did not change",
                }
            };
            findings.push(finding(
                FindingKindIR::Trend,
                if korean { format!("'{}' 계열은 첫 관측값 {}에서 마지막 관측값 {}으로 {}했습니다.", series.name, first.original, last.original, direction) } else { format!("Series '{}' {} from the first observed value {} to the last observed value {}.", series.name, direction, first.original, last.original) },
                vec![(*first_location).clone(), (*last_location).clone()],
                1_000,
            ));
        }
    }
    findings
}

fn analyze_financial(statement: &FinancialStatementIR, korean: bool) -> Vec<KnowledgeFindingIR> {
    let mut findings = vec![finding(
        FindingKindIR::Summary,
        if korean {
            format!(
                "재무제표는 {}개 기간과 {}개 계정 항목을 포함합니다.",
                statement.periods.len(),
                statement.line_items.len()
            )
        } else {
            format!(
                "The financial statement contains {} period(s) and {} line item(s).",
                statement.periods.len(),
                statement.line_items.len()
            )
        },
        vec!["statement".to_string()],
        1_000,
    )];
    if statement.statement_type == FinancialStatementTypeIR::BalanceSheet {
        for period in &statement.periods {
            let assets = find_financial_value(statement, &["total_assets", "assets"], period);
            let liabilities =
                find_financial_value(statement, &["total_liabilities", "liabilities"], period);
            let equity = find_financial_value(statement, &["total_equity", "equity"], period);
            if let (Some(assets), Some(liabilities), Some(equity)) = (assets, liabilities, equity) {
                let balanced = exact_sum_equal(assets, liabilities, equity);
                findings.push(finding(
                    FindingKindIR::AccountingCheck,
                    if korean {
                        format!(
                            "{period} 자산 = 부채 + 자본 검사는 {}입니다.",
                            if balanced { "일치" } else { "불일치" }
                        )
                    } else {
                        format!(
                            "For {period}, the assets = liabilities + equity check {}.",
                            if balanced {
                                "balances"
                            } else {
                                "does not balance"
                            }
                        )
                    },
                    vec![
                        "total_assets".to_string(),
                        "total_liabilities".to_string(),
                        "total_equity".to_string(),
                    ],
                    1_000,
                ));
            }
        }
    }
    findings
}

fn analyze_plan(plan: &PlanProposalIR, korean: bool) -> Vec<KnowledgeFindingIR> {
    let mut findings = vec![finding(
        FindingKindIR::Summary,
        if korean {
            format!(
                "계획안은 {}개 작업과 {}개 명시적 위험을 포함합니다.",
                plan.tasks.len(),
                plan.risks.len()
            )
        } else {
            format!(
                "The plan contains {} task(s) and {} explicit risk(s).",
                plan.tasks.len(),
                plan.risks.len()
            )
        },
        vec!["plan".to_string()],
        1_000,
    )];
    let known = plan
        .tasks
        .iter()
        .map(|task| task.task_id.as_str())
        .collect::<BTreeSet<_>>();
    if known.len() != plan.tasks.len() {
        findings.push(finding(
            FindingKindIR::StructuralGap,
            if korean {
                "계획안에 중복된 작업 ID가 있습니다."
            } else {
                "The plan contains duplicate task identifiers."
            }
            .to_string(),
            vec!["tasks".to_string()],
            1_000,
        ));
    }
    for task in &plan.tasks {
        if task
            .dependencies
            .iter()
            .any(|dependency| !known.contains(dependency.as_str()))
        {
            findings.push(finding(
                FindingKindIR::StructuralGap,
                if korean {
                    format!("{} 작업에 존재하지 않는 의존성이 있습니다.", task.task_id)
                } else {
                    format!(
                        "Task {} has a dependency that does not exist.",
                        task.task_id
                    )
                },
                vec![task.task_id.clone()],
                1_000,
            ));
        }
        if task.completion_condition.is_none() {
            findings.push(finding(
                FindingKindIR::Risk,
                if korean {
                    format!("{} 작업에 완료 조건이 없습니다.", task.task_id)
                } else {
                    format!("Task {} has no completion condition.", task.task_id)
                },
                vec![task.task_id.clone()],
                900,
            ));
        }
    }
    if let Some(cycle) = plan_dependency_cycle(plan) {
        findings.push(finding(
            FindingKindIR::StructuralGap,
            if korean {
                format!("계획안 의존성에 순환이 있습니다: {}", cycle.join(" -> "))
            } else {
                format!(
                    "The plan dependency graph contains a cycle: {}",
                    cycle.join(" -> ")
                )
            },
            cycle,
            1_000,
        ));
    }
    findings
}

fn plan_dependency_cycle(plan: &PlanProposalIR) -> Option<Vec<String>> {
    let mut graph = BTreeMap::<String, Vec<String>>::new();
    for task in &plan.tasks {
        graph
            .entry(task.task_id.clone())
            .or_default()
            .extend(task.dependencies.iter().cloned());
    }
    for start in graph.keys() {
        let mut frontier = graph.get(start).cloned().unwrap_or_default();
        let mut visited = BTreeSet::new();
        while let Some(current) = frontier.pop() {
            if &current == start {
                let mut cycle = visited.into_iter().collect::<Vec<_>>();
                cycle.push(start.clone());
                return Some(cycle);
            }
            if visited.insert(current.clone()) {
                frontier.extend(graph.get(&current).cloned().unwrap_or_default());
            }
        }
    }
    None
}

fn render_document(
    document: &KnowledgeDocumentIR,
    format: OutputFormatIR,
    language: LanguageCodeIR,
) -> Result<String, KnowledgeWorkError> {
    match format {
        OutputFormatIR::Json => {
            serde_json::to_string_pretty(document).map_err(|_| KnowledgeWorkError::Json)
        }
        OutputFormatIR::Markdown => Ok(render_markdown(
            document,
            language == LanguageCodeIR::Korean,
        )),
        OutputFormatIR::Html => Ok(crate::document_design::render_print_ready_html(
            document,
            &[],
            language,
            &DocumentDesignIR::for_kind(document.kind()),
        )),
        OutputFormatIR::Csv => match document {
            KnowledgeDocumentIR::Table(table) => Ok(render_table_csv(table)),
            KnowledgeDocumentIR::FinancialStatement(statement) => {
                Ok(render_financial_csv(statement))
            }
            _ => Err(KnowledgeWorkError::UnsupportedOutput),
        },
        OutputFormatIR::Svg => match document {
            KnowledgeDocumentIR::Chart(chart) => render_chart_svg(chart),
            _ => Err(KnowledgeWorkError::UnsupportedOutput),
        },
    }
}

#[derive(Serialize)]
struct JsonKnowledgeResult<'a> {
    document: &'a KnowledgeDocumentIR,
    findings: &'a [KnowledgeFindingIR],
}

fn render_result(
    document: &KnowledgeDocumentIR,
    findings: &[KnowledgeFindingIR],
    operation: KnowledgeWorkOperationIR,
    format: OutputFormatIR,
    language: LanguageCodeIR,
    design: &DocumentDesignIR,
) -> Result<String, KnowledgeWorkError> {
    if format == OutputFormatIR::Json {
        return serde_json::to_string_pretty(&JsonKnowledgeResult { document, findings })
            .map_err(|_| KnowledgeWorkError::Json);
    }
    if format == OutputFormatIR::Html {
        let rendered_findings = if matches!(
            operation,
            KnowledgeWorkOperationIR::Interpret | KnowledgeWorkOperationIR::Analyze
        ) {
            findings
        } else {
            &[]
        };
        return Ok(crate::document_design::render_print_ready_html(
            document,
            rendered_findings,
            language,
            design,
        ));
    }
    let korean = language == LanguageCodeIR::Korean;
    let mut rendered = render_document(document, format, language)?;
    if format == OutputFormatIR::Markdown
        && matches!(
            operation,
            KnowledgeWorkOperationIR::Interpret | KnowledgeWorkOperationIR::Analyze
        )
    {
        rendered.push_str(if korean {
            "\n## 분석 결과\n\n"
        } else {
            "\n## Analysis findings\n\n"
        });
        for finding in findings {
            rendered.push_str(&format!(
                "- **{:?}** {} ({}: {})\n",
                finding.kind,
                finding.statement,
                if korean { "근거" } else { "evidence" },
                finding.evidence_locations.join(", ")
            ));
        }
    }
    Ok(rendered)
}

fn render_markdown(document: &KnowledgeDocumentIR, korean: bool) -> String {
    match document {
        KnowledgeDocumentIR::Paper(paper) => {
            let mut output = format!("# {}\n\n", paper.title);
            if !paper.authors.is_empty() {
                output.push_str(&format!(
                    "**{}:** {}\n\n",
                    if korean { "저자" } else { "Authors" },
                    paper.authors.join(", ")
                ));
            }
            if !paper.abstract_text.is_empty() {
                output.push_str(&format!(
                    "## {}\n\n{}\n\n",
                    if korean { "초록" } else { "Abstract" },
                    paper.abstract_text
                ));
            }
            for section in &paper.sections {
                output.push_str(&format!(
                    "{} {}\n\n{}\n\n",
                    "#".repeat(usize::from(section.level.max(2))),
                    section.heading,
                    section.body
                ));
            }
            if !paper.references.is_empty() {
                output.push_str(if korean {
                    "## 참고문헌\n\n"
                } else {
                    "## References\n\n"
                });
                for reference in &paper.references {
                    output.push_str(&format!("- {}\n", reference.citation_text));
                }
            }
            output
        }
        KnowledgeDocumentIR::BusinessPlan(business)
        | KnowledgeDocumentIR::BusinessProposal(business) => {
            render_business_markdown(business, korean)
        }
        KnowledgeDocumentIR::UserGuide(guide) => render_user_guide_markdown(guide, korean),
        KnowledgeDocumentIR::Table(table) => render_table_markdown(table),
        KnowledgeDocumentIR::Chart(chart) => {
            let mut output = format!(
                "# {}\n\n- 유형: {:?}\n- 범주축: {}\n- 값축: {}\n\n",
                chart.title, chart.chart_type, chart.category_axis, chart.value_axis
            );
            for series in &chart.series {
                output.push_str(&format!("## {}\n\n", series.name));
                for point in &series.points {
                    output.push_str(&format!(
                        "- {}: {}\n",
                        point.category,
                        point
                            .value
                            .as_ref()
                            .map(|value| value.original.as_str())
                            .unwrap_or("N/A")
                    ));
                }
            }
            output
        }
        KnowledgeDocumentIR::FinancialStatement(statement) => {
            let table = table_from_financial(statement);
            format!(
                "# {} — {:?}\n\n{}: {} / {}: {}\n\n{}",
                statement.entity,
                statement.statement_type,
                if korean { "통화" } else { "Currency" },
                statement.currency,
                if korean { "단위" } else { "Unit" },
                statement.display_unit,
                render_table_markdown(&table)
            )
        }
        KnowledgeDocumentIR::PlanProposal(plan) => {
            let mut output = format!(
                "# {}\n\n**{}:** {}\n\n## {}\n\n",
                plan.title,
                if korean { "목표" } else { "Objective" },
                plan.objective,
                if korean { "작업" } else { "Tasks" }
            );
            for task in &plan.tasks {
                output.push_str(&format!("- **{}** {}", task.task_id, task.description));
                if !task.dependencies.is_empty() {
                    output.push_str(&format!(
                        " ({}: {})",
                        if korean { "의존" } else { "depends on" },
                        task.dependencies.join(", ")
                    ));
                }
                output.push('\n');
            }
            if !plan.risks.is_empty() {
                output.push_str(if korean {
                    "\n## 위험\n\n"
                } else {
                    "\n## Risks\n\n"
                });
                for risk in &plan.risks {
                    output.push_str(&format!("- {risk}\n"));
                }
            }
            if !plan.assumptions.is_empty() {
                output.push_str(if korean {
                    "\n## 가정\n\n"
                } else {
                    "\n## Assumptions\n\n"
                });
                for assumption in &plan.assumptions {
                    output.push_str(&format!("- {assumption}\n"));
                }
            }
            output
        }
    }
}

fn render_user_guide_markdown(guide: &UserGuideIR, korean: bool) -> String {
    let mut output = format!(
        "# {}\n\n**{}:** {}\n\n{}\n\n",
        guide.title,
        if korean { "대상" } else { "Audience" },
        guide.audience,
        guide.introduction
    );
    for section in &guide.sections {
        output.push_str(&format!("## {}\n\n{}\n\n", section.heading, section.body));
        for (index, step) in section.steps.iter().enumerate() {
            output.push_str(&format!("{}. {}\n", index + 1, step));
        }
        if !section.steps.is_empty() {
            output.push('\n');
        }
    }
    if !guide.examples.is_empty() {
        output.push_str(if korean {
            "## 사용 예시\n\n"
        } else {
            "## Examples\n\n"
        });
        for example in &guide.examples {
            output.push_str(&format!(
                "### {}\n\n- **{}:** {}\n- **{}:** {}\n\n",
                example.title,
                if korean { "입력" } else { "Input" },
                example.input,
                if korean {
                    "기대 결과"
                } else {
                    "Expected result"
                },
                example.expected_result
            ));
        }
    }
    if !guide.cautions.is_empty() {
        output.push_str(if korean {
            "## 주의사항\n\n"
        } else {
            "## Cautions\n\n"
        });
        for caution in &guide.cautions {
            output.push_str(&format!("- {caution}\n"));
        }
        output.push('\n');
    }
    if !guide.troubleshooting.is_empty() {
        output.push_str(if korean {
            "## 문제 해결\n\n"
        } else {
            "## Troubleshooting\n\n"
        });
        for item in &guide.troubleshooting {
            output.push_str(&format!("- **{}** — {}\n", item.symptom, item.resolution));
        }
        output.push('\n');
    }
    if !guide.checklist.is_empty() {
        output.push_str(if korean {
            "## 빠른 확인 목록\n\n"
        } else {
            "## Quick checklist\n\n"
        });
        for item in &guide.checklist {
            output.push_str(&format!("- [ ] {item}\n"));
        }
    }
    for table in &guide.tables {
        output.push_str(&format!("\n{}", render_table_markdown(table)));
    }
    output
}

fn render_business_markdown(business: &BusinessDocumentIR, korean: bool) -> String {
    let mut output = format!(
        "# {}\n\n**{}:** {}  \n**{}:** {}\n\n## {}\n\n{}\n\n",
        business.title,
        if korean { "조직" } else { "Organization" },
        business.organization,
        if korean { "대상" } else { "Audience" },
        business.audience,
        if korean {
            "핵심 요약"
        } else {
            "Executive summary"
        },
        business.executive_summary,
    );
    if !business.key_metrics.is_empty() {
        output.push_str(&format!(
            "## {}\n\n",
            if korean {
                "핵심 지표"
            } else {
                "Key metrics"
            }
        ));
        for metric in &business.key_metrics {
            output.push_str(&format!(
                "- **{}:** {}{} ({})\n",
                metric.label,
                metric.value,
                metric
                    .change
                    .as_ref()
                    .map(|change| format!(" · {change}"))
                    .unwrap_or_default(),
                metric.evidence_location,
            ));
        }
        output.push('\n');
    }
    for section in &business.sections {
        output.push_str(&format!("## {}\n\n{}\n\n", section.heading, section.body));
        for highlight in &section.highlights {
            output.push_str(&format!("- {highlight}\n"));
        }
        if !section.highlights.is_empty() {
            output.push('\n');
        }
    }
    output.push_str(&format!(
        "## {}\n\n{}\n\n",
        if korean {
            "실행 계획"
        } else {
            "Execution plan"
        },
        render_plan_markdown(&business.execution_plan, korean)
    ));
    for table in &business.tables {
        output.push_str(&render_table_markdown(table));
    }
    if !business.risks.is_empty() {
        output.push_str(&format!(
            "\n## {}\n\n",
            if korean { "위험" } else { "Risks" }
        ));
        for risk in &business.risks {
            output.push_str(&format!("- {risk}\n"));
        }
    }
    output.push_str(&format!(
        "\n## {}\n\n{}\n",
        if korean {
            "다음 단계"
        } else {
            "Next action"
        },
        business.next_action
    ));
    output
}

fn render_plan_markdown(plan: &PlanProposalIR, korean: bool) -> String {
    plan.tasks
        .iter()
        .map(|task| {
            format!(
                "- **{}** {}{}",
                task.task_id,
                task.description,
                if task.dependencies.is_empty() {
                    String::new()
                } else {
                    format!(
                        " ({}: {})",
                        if korean { "의존" } else { "depends on" },
                        task.dependencies.join(", ")
                    )
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_table_markdown(table: &TableIR) -> String {
    let mut output = format!("## {}\n\n", table.title);
    output.push_str(&format!(
        "| {} |\n",
        table
            .columns
            .iter()
            .map(|value| escape_markdown_cell(value))
            .collect::<Vec<_>>()
            .join(" | ")
    ));
    output.push_str(&format!(
        "| {} |\n",
        table
            .columns
            .iter()
            .map(|_| "---")
            .collect::<Vec<_>>()
            .join(" | ")
    ));
    for row in &table.rows {
        output.push_str(&format!(
            "| {} |\n",
            row.iter()
                .map(|cell| escape_markdown_cell(&cell.raw))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
    }
    output
}

fn render_table_csv(table: &TableIR) -> String {
    let mut lines = vec![table
        .columns
        .iter()
        .map(|value| csv_escape(value))
        .collect::<Vec<_>>()
        .join(",")];
    lines.extend(table.rows.iter().map(|row| {
        row.iter()
            .map(|cell| csv_escape(&cell.raw))
            .collect::<Vec<_>>()
            .join(",")
    }));
    lines.join("\r\n") + "\r\n"
}

fn render_financial_csv(statement: &FinancialStatementIR) -> String {
    render_table_csv(&table_from_financial(statement))
}

pub(crate) fn render_chart_svg(chart: &ChartIR) -> Result<String, KnowledgeWorkError> {
    let values = chart
        .series
        .iter()
        .flat_map(|series| &series.points)
        .filter_map(|point| point.value.as_ref())
        .map(numeric_f64)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Err(KnowledgeWorkError::UnsupportedOutput);
    }
    if chart.chart_type == ChartTypeIR::Pie {
        return render_pie_svg(chart);
    }
    let min = values
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min)
        .min(0.0);
    let max = values
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        .max(0.0);
    let span = (max - min).abs().max(1.0);
    let width = 960.0;
    let height = 540.0;
    let left = 96.0;
    let top = 72.0;
    let plot_width = 804.0;
    let plot_height = 352.0;
    let colors = ["#39e6b0", "#55c7ff", "#ffb454", "#b394ff", "#ff6f91"];
    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" role=\"img\" aria-label=\"{}\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\"><title>{}</title><rect width=\"100%\" height=\"100%\" fill=\"#0b1114\"/><text x=\"{left}\" y=\"34\" fill=\"#e8f1f2\" font-family=\"sans-serif\" font-size=\"21\" font-weight=\"700\">{}</text><text x=\"{left}\" y=\"55\" fill=\"#607078\" font-family=\"sans-serif\" font-size=\"12\">{} · {}</text>", xml_escape(&chart.title), xml_escape(&chart.title), xml_escape(&chart.title), xml_escape(&chart.category_axis), xml_escape(&chart.value_axis));
    for tick in 0..=4 {
        let ratio = tick as f64 / 4.0;
        let y = top + ratio * plot_height;
        let value = max - ratio * span;
        svg.push_str(&format!("<line x1=\"{left}\" y1=\"{y:.2}\" x2=\"{}\" y2=\"{y:.2}\" stroke=\"#607078\" stroke-opacity=\"0.35\"/><text x=\"{}\" y=\"{:.2}\" text-anchor=\"end\" fill=\"#607078\" font-family=\"sans-serif\" font-size=\"11\">{}</text>", left + plot_width, left - 12.0, y + 4.0, xml_escape(&format_chart_value(value))));
    }
    svg.push_str(&format!("<line x1=\"{left}\" y1=\"{top}\" x2=\"{left}\" y2=\"{}\" stroke=\"#607078\"/><line x1=\"{left}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#607078\"/>", top + plot_height, top + plot_height, left + plot_width, top + plot_height));
    if let Some(category_series) = chart.series.first() {
        let category_count = category_series.points.len();
        let denominator = category_count.saturating_sub(1).max(1) as f64;
        let label_step = category_count.div_ceil(8).max(1);
        for (index, point) in category_series.points.iter().enumerate() {
            if index % label_step != 0 && index + 1 != category_count {
                continue;
            }
            let x = if chart.chart_type == ChartTypeIR::Bar {
                left + (index as f64 + 0.5) / category_count.max(1) as f64 * plot_width
            } else {
                left + index as f64 / denominator * plot_width
            };
            svg.push_str(&format!("<text x=\"{x:.2}\" y=\"{}\" text-anchor=\"middle\" fill=\"#607078\" font-family=\"sans-serif\" font-size=\"11\">{}</text>", top + plot_height + 24.0, xml_escape(&point.category)));
        }
    }
    for (series_index, series) in chart.series.iter().enumerate() {
        let color = colors[series_index % colors.len()];
        let observed = series
            .points
            .iter()
            .enumerate()
            .filter_map(|(index, point)| {
                point
                    .value
                    .as_ref()
                    .map(|value| (index, numeric_f64(value)))
            })
            .collect::<Vec<_>>();
        if observed.is_empty() {
            continue;
        }
        let denominator = series.points.len().saturating_sub(1).max(1) as f64;
        if chart.chart_type == ChartTypeIR::Bar {
            let slot_width = plot_width / series.points.len().max(1) as f64;
            let group_width = slot_width * 0.72;
            let bar_width = (group_width / chart.series.len().max(1) as f64).max(2.0);
            let baseline_y = top + max / span * plot_height;
            for (index, value) in observed {
                let x = left
                    + index as f64 * slot_width
                    + (slot_width - group_width) / 2.0
                    + series_index as f64 * bar_width;
                let value_y = top + (max - value) / span * plot_height;
                let y = value_y.min(baseline_y);
                let bar_height = (value_y - baseline_y).abs();
                svg.push_str(&format!("<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{bar_width:.2}\" height=\"{bar_height:.2}\" rx=\"3\" fill=\"{color}\"><title>{}: {}</title></rect>", xml_escape(&series.points[index].category), xml_escape(&format_chart_value(value))));
            }
        } else if chart.chart_type == ChartTypeIR::Scatter {
            for (index, value) in observed {
                let x = left + index as f64 / denominator * plot_width;
                let y = top + (max - value) / span * plot_height;
                svg.push_str(&format!("<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"5\" fill=\"{color}\"><title>{}: {}</title></circle>", xml_escape(&series.points[index].category), xml_escape(&format_chart_value(value))));
            }
        } else {
            let points = observed
                .iter()
                .map(|(index, value)| {
                    format!(
                        "{:.2},{:.2}",
                        left + *index as f64 / denominator * plot_width,
                        top + (max - *value) / span * plot_height
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            svg.push_str(&format!("<polyline fill=\"none\" stroke=\"{color}\" stroke-width=\"3\" points=\"{points}\"/>"));
            for (index, value) in observed {
                let x = left + index as f64 / denominator * plot_width;
                let y = top + (max - value) / span * plot_height;
                svg.push_str(&format!("<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"4.5\" fill=\"#0b1114\" stroke=\"{color}\" stroke-width=\"3\"><title>{}: {}</title></circle>", xml_escape(&series.points[index].category), xml_escape(&format_chart_value(value))));
            }
        }
        let legend_x = left + series_index as f64 * 180.0;
        svg.push_str(&format!("<rect x=\"{legend_x}\" y=\"{}\" width=\"18\" height=\"4\" rx=\"2\" fill=\"{color}\"/><text x=\"{}\" y=\"{}\" fill=\"#e8f1f2\" font-family=\"sans-serif\" font-size=\"12\">{}</text>", height - 26.0, legend_x + 26.0, height - 20.0, xml_escape(&series.name)));
    }
    svg.push_str("</svg>");
    Ok(svg)
}

fn format_chart_value(value: f64) -> String {
    if value.abs() >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value.abs() >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else if (value - value.round()).abs() < 0.000_001 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn render_pie_svg(chart: &ChartIR) -> Result<String, KnowledgeWorkError> {
    if chart.series.len() != 1 {
        return Err(KnowledgeWorkError::UnsupportedOutput);
    }
    let points = chart.series[0]
        .points
        .iter()
        .filter_map(|point| {
            point
                .value
                .as_ref()
                .map(|value| (point.category.as_str(), numeric_f64(value)))
        })
        .collect::<Vec<_>>();
    if points.is_empty() || points.iter().any(|(_, value)| *value < 0.0) {
        return Err(KnowledgeWorkError::UnsupportedOutput);
    }
    let total = points.iter().map(|(_, value)| *value).sum::<f64>();
    if total <= 0.0 {
        return Err(KnowledgeWorkError::UnsupportedOutput);
    }
    let colors = [
        "#39e6b0", "#55c7ff", "#ffb454", "#b394ff", "#ff6f91", "#87d068",
    ];
    let (center_x, center_y, radius) = (360.0_f64, 270.0_f64, 190.0_f64);
    let mut angle = -std::f64::consts::FRAC_PI_2;
    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"960\" height=\"540\" viewBox=\"0 0 960 540\"><rect width=\"100%\" height=\"100%\" fill=\"#0b1114\"/><text x=\"60\" y=\"32\" fill=\"#e8f1f2\" font-family=\"sans-serif\" font-size=\"20\">{}</text>", xml_escape(&chart.title));
    if points.len() == 1 {
        svg.push_str(&format!("<circle cx=\"{center_x}\" cy=\"{center_y}\" r=\"{radius}\" fill=\"{}\"/><rect x=\"610\" y=\"80\" width=\"14\" height=\"14\" fill=\"{}\"/><text x=\"632\" y=\"92\" fill=\"#e8f1f2\" font-family=\"sans-serif\" font-size=\"14\">{} (100.0%)</text></svg>", colors[0], colors[0], xml_escape(points[0].0)));
        return Ok(svg);
    }
    for (index, (category, value)) in points.iter().enumerate() {
        let sweep = value / total * std::f64::consts::TAU;
        let end = angle + sweep;
        let (start_x, start_y) = (
            center_x + radius * angle.cos(),
            center_y + radius * angle.sin(),
        );
        let (end_x, end_y) = (center_x + radius * end.cos(), center_y + radius * end.sin());
        let large_arc = i32::from(sweep > std::f64::consts::PI);
        let color = colors[index % colors.len()];
        svg.push_str(&format!("<path d=\"M {center_x:.2} {center_y:.2} L {start_x:.2} {start_y:.2} A {radius:.2} {radius:.2} 0 {large_arc} 1 {end_x:.2} {end_y:.2} Z\" fill=\"{color}\"/>"));
        svg.push_str(&format!("<rect x=\"610\" y=\"{}\" width=\"14\" height=\"14\" fill=\"{color}\"/><text x=\"632\" y=\"{}\" fill=\"#e8f1f2\" font-family=\"sans-serif\" font-size=\"14\">{} ({:.1}%)</text>", 80 + index * 28, 92 + index * 28, xml_escape(category), value / total * 100.0));
        angle = end;
    }
    svg.push_str("</svg>");
    Ok(svg)
}

fn write_output(
    path: &str,
    format: OutputFormatIR,
    bytes: &[u8],
    overwrite: bool,
) -> Result<FileOutputReceiptIR, KnowledgeWorkError> {
    let target = PathBuf::from(path);
    if target.file_name().is_none() || target.is_dir() {
        return Err(KnowledgeWorkError::InvalidOutputPath);
    }
    if !extension_matches(&target, format) {
        return Err(KnowledgeWorkError::InvalidOutputPath);
    }
    let existed = target.exists();
    if existed && !overwrite {
        return Err(KnowledgeWorkError::OutputExists);
    }
    if let Some(parent) = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|_| KnowledgeWorkError::FileWrite)?;
    }
    let stage = staging_path(&target);
    let mut stage_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&stage)
        .map_err(|_| KnowledgeWorkError::FileWrite)?;
    let result = (|| {
        stage_file
            .write_all(bytes)
            .map_err(|_| KnowledgeWorkError::FileWrite)?;
        stage_file
            .sync_all()
            .map_err(|_| KnowledgeWorkError::FileWrite)?;
        if existed {
            let mut target_file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&target)
                .map_err(|_| KnowledgeWorkError::FileWrite)?;
            target_file
                .write_all(bytes)
                .map_err(|_| KnowledgeWorkError::FileWrite)?;
            target_file
                .sync_all()
                .map_err(|_| KnowledgeWorkError::FileWrite)?;
            fs::remove_file(&stage).map_err(|_| KnowledgeWorkError::FileWrite)?;
        } else {
            fs::rename(&stage, &target).map_err(|_| KnowledgeWorkError::FileWrite)?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&stage);
    }
    result?;
    Ok(FileOutputReceiptIR {
        path: target.to_string_lossy().to_string(),
        format,
        bytes_written: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        sha256: sha256(bytes),
        overwritten: existed,
    })
}

fn parse_cell(raw: &str) -> CellValueIR {
    let value = raw.trim();
    if value.is_empty()
        || matches!(
            normalize(value).as_str(),
            "n/a" | "na" | "null" | "없음" | "-"
        )
    {
        CellValueIR::Missing
    } else if value.eq_ignore_ascii_case("true") {
        CellValueIR::Boolean(true)
    } else if value.eq_ignore_ascii_case("false") {
        CellValueIR::Boolean(false)
    } else if let Some(number) = parse_number(value) {
        CellValueIR::Number(number)
    } else {
        CellValueIR::Text(value.to_string())
    }
}

fn parse_number(raw: &str) -> Option<NumericValueIR> {
    let trimmed = raw.trim();
    let negative_parentheses = trimmed.starts_with('(') && trimmed.ends_with(')');
    let mut cleaned = trimmed
        .trim_matches(|character| matches!(character, '(' | ')' | '$' | '€' | '£' | '₩' | '¥'))
        .replace([',', ' '], "");
    let percent = cleaned.ends_with('%');
    if percent {
        cleaned.pop();
    }
    let negative = negative_parentheses || cleaned.starts_with('-');
    cleaned = cleaned.trim_start_matches(['+', '-']).to_string();
    let (whole, fraction) = cleaned.split_once('.').unwrap_or((cleaned.as_str(), ""));
    if whole.is_empty()
        || !whole.chars().all(|character| character.is_ascii_digit())
        || !fraction.chars().all(|character| character.is_ascii_digit())
        || fraction.len() > 9
    {
        return None;
    }
    let scale = u8::try_from(fraction.len()).ok()?;
    let digits = format!("{whole}{fraction}");
    let mut coefficient = digits.parse::<i64>().ok()?;
    if negative {
        coefficient = coefficient.checked_neg()?;
    }
    let unit = if percent {
        Some("percent".to_string())
    } else if trimmed.contains('₩') {
        Some("KRW".to_string())
    } else if trimmed.contains('$') {
        Some("USD".to_string())
    } else if trimmed.contains('€') {
        Some("EUR".to_string())
    } else {
        None
    };
    Some(NumericValueIR {
        coefficient,
        scale,
        unit,
        original: raw.to_string(),
    })
}

fn split_delimited(line: &str, delimiter: char) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut characters = line.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '"' {
            if quoted && characters.peek() == Some(&'"') {
                current.push('"');
                characters.next();
            } else {
                quoted = !quoted;
            }
        } else if character == delimiter && !quoted {
            result.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(character);
        }
    }
    result.push(current.trim().to_string());
    result
}

fn extract_claims(sections: &[PaperSectionIR]) -> Vec<PaperClaimIR> {
    let mut claims = Vec::new();
    for section in sections {
        if matches_any(&normalize(&section.heading), &["references", "참고문헌"]) {
            continue;
        }
        for sentence in section
            .body
            .split(['.', '!', '?', '。'])
            .map(str::trim)
            .filter(|sentence| sentence.chars().count() >= 20)
            .take(64)
        {
            claims.push(PaperClaimIR {
                claim_id: format!("CLAIM-{}", claims.len() + 1),
                statement: sentence.to_string(),
                evidence_locations: vec![section.section_id.clone()],
                confidence_millis: if sentence.contains('[') || sentence.contains('(') {
                    750
                } else {
                    500
                },
            });
        }
    }
    claims
}

fn extract_references(sections: &[PaperSectionIR]) -> Vec<PaperReferenceIR> {
    sections
        .iter()
        .filter(|section| {
            matches_any(
                &normalize(&section.heading),
                &["references", "bibliography", "참고문헌"],
            )
        })
        .flat_map(|section| section.body.lines())
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .enumerate()
        .map(|(index, line)| PaperReferenceIR {
            reference_id: format!("REF-{}", index + 1),
            citation_text: line.trim_start_matches(['-', '*']).trim().to_string(),
        })
        .collect()
}

fn table_from_financial(statement: &FinancialStatementIR) -> TableIR {
    let mut columns = vec!["항목".to_string()];
    columns.extend(statement.periods.clone());
    let rows = statement
        .line_items
        .iter()
        .enumerate()
        .map(|(row_index, item)| {
            let mut row = vec![TableCellIR {
                value: CellValueIR::Text(item.label.clone()),
                raw: item.label.clone(),
                source_location: item.source_location.clone(),
            }];
            for (column_index, period) in statement.periods.iter().enumerate() {
                let value = item.values_by_period.get(period);
                row.push(TableCellIR {
                    value: value
                        .cloned()
                        .map(CellValueIR::Number)
                        .unwrap_or(CellValueIR::Missing),
                    raw: value
                        .map(|number| number.original.clone())
                        .unwrap_or_default(),
                    source_location: format!("row:{}:column:{}", row_index + 2, column_index + 2),
                });
            }
            row
        })
        .collect();
    TableIR {
        schema: TABLE_SCHEMA.to_string(),
        document_id: statement.document_id.clone(),
        title: statement.entity.clone(),
        columns,
        rows,
        notes: Vec::new(),
    }
}

fn find_financial_value<'a>(
    statement: &'a FinancialStatementIR,
    concepts: &[&str],
    period: &str,
) -> Option<&'a NumericValueIR> {
    statement
        .line_items
        .iter()
        .find(|item| concepts.contains(&item.normalized_concept.as_str()))
        .and_then(|item| item.values_by_period.get(period))
}

fn exact_sum_equal(left: &NumericValueIR, first: &NumericValueIR, second: &NumericValueIR) -> bool {
    let scale = left.scale.max(first.scale).max(second.scale);
    let align = |value: &NumericValueIR| {
        i128::from(value.coefficient).checked_mul(10_i128.pow(u32::from(scale - value.scale)))
    };
    matches!((align(left), align(first), align(second)), (Some(left), Some(first), Some(second)) if first.checked_add(second) == Some(left))
}

fn normalize_financial_concept(label: &str) -> String {
    let normalized = normalize(label).replace([' ', '_', '-'], "");
    if contains_any(&normalized, &["totalassets", "총자산", "자산총계"]) {
        "total_assets"
    } else if contains_any(&normalized, &["totalliabilities", "총부채", "부채총계"]) {
        "total_liabilities"
    } else if contains_any(&normalized, &["totalequity", "총자본", "자본총계"]) {
        "total_equity"
    } else if contains_any(&normalized, &["assets", "자산"]) {
        "assets"
    } else if contains_any(&normalized, &["liabilities", "부채"]) {
        "liabilities"
    } else if contains_any(&normalized, &["equity", "자본"]) {
        "equity"
    } else if contains_any(&normalized, &["revenue", "sales", "매출"]) {
        "revenue"
    } else if contains_any(&normalized, &["netincome", "당기순이익", "순이익"]) {
        "net_income"
    } else {
        return normalized;
    }
    .to_string()
}

fn classify_financial_line(label: &str) -> FinancialLineClassIR {
    let concept = normalize_financial_concept(label);
    if concept.contains("asset") {
        FinancialLineClassIR::Asset
    } else if concept.contains("liabilit") {
        FinancialLineClassIR::Liability
    } else if concept.contains("equity") {
        FinancialLineClassIR::Equity
    } else if concept.contains("revenue") || concept.contains("income") {
        FinancialLineClassIR::Revenue
    } else if contains_any(&normalize(label), &["expense", "cost", "비용", "원가"]) {
        FinancialLineClassIR::Expense
    } else if contains_any(&normalize(label), &["cash flow", "현금흐름"]) {
        FinancialLineClassIR::CashFlow
    } else if contains_any(&normalize(label), &["total", "총계", "합계"]) {
        FinancialLineClassIR::Total
    } else {
        FinancialLineClassIR::Other
    }
}

fn infer_statement_type(text: &str) -> FinancialStatementTypeIR {
    let text = normalize(text);
    if contains_any(
        &text,
        &[
            "balance sheet",
            "대차대조표",
            "재무상태표",
            "자산",
            "부채",
            "자본",
        ],
    ) {
        FinancialStatementTypeIR::BalanceSheet
    } else if contains_any(&text, &["income statement", "손익계산서", "매출", "순이익"]) {
        FinancialStatementTypeIR::IncomeStatement
    } else if contains_any(&text, &["cash flow", "현금흐름"]) {
        FinancialStatementTypeIR::CashFlowStatement
    } else if contains_any(&text, &["changes in equity", "자본변동"]) {
        FinancialStatementTypeIR::ChangesInEquity
    } else {
        FinancialStatementTypeIR::Unknown
    }
}

fn infer_chart_type(command: &str) -> ChartTypeIR {
    let command = normalize(command);
    if contains_any(&command, &["bar", "막대"]) {
        ChartTypeIR::Bar
    } else if contains_any(&command, &["scatter", "산점"]) {
        ChartTypeIR::Scatter
    } else if contains_any(&command, &["pie", "원형"]) {
        ChartTypeIR::Pie
    } else {
        ChartTypeIR::Line
    }
}
fn infer_currency(table: &TableIR) -> String {
    for row in &table.rows {
        for cell in row {
            if let CellValueIR::Number(number) = &cell.value {
                if let Some(unit) = &number.unit {
                    return unit.clone();
                }
            }
        }
    }
    "UNSPECIFIED".to_string()
}
fn numeric_f64(value: &NumericValueIR) -> f64 {
    value.coefficient as f64 / 10_f64.powi(i32::from(value.scale))
}
fn numeric_cmp(left: &NumericValueIR, right: &NumericValueIR) -> std::cmp::Ordering {
    numeric_f64(left)
        .partial_cmp(&numeric_f64(right))
        .unwrap_or(std::cmp::Ordering::Equal)
}
fn finding(
    kind: FindingKindIR,
    statement: String,
    evidence_locations: Vec<String>,
    confidence_millis: u16,
) -> KnowledgeFindingIR {
    KnowledgeFindingIR {
        finding_id: String::new(),
        kind,
        statement,
        evidence_locations,
        confidence_millis,
    }
}
fn task(id: &str, description: &str, dependencies: &[&str]) -> PlanTaskIR {
    PlanTaskIR {
        task_id: id.to_string(),
        description: description.to_string(),
        dependencies: dependencies
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        owner: None,
        completion_condition: Some(format!("verified:{description}")),
    }
}
fn detect_output_language(command: &str) -> LanguageCodeIR {
    if command
        .chars()
        .any(|character| matches!(character, '\u{ac00}'..='\u{d7a3}' | '\u{3131}'..='\u{318e}'))
    {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    }
}
fn command_subject(command: &str) -> String {
    command.trim().trim_end_matches(['.', '!', '?']).to_string()
}
fn value_after_marker(command: &str, markers: &[&str]) -> Option<String> {
    let normalized = command.to_lowercase();
    markers.iter().find_map(|marker| {
        normalized.find(&marker.to_lowercase()).map(|index| {
            command[index + marker.len()..]
                .lines()
                .next()
                .unwrap_or_default()
                .trim()
                .to_string()
        })
    })
}
fn strip_prefix_any<'a>(value: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    prefixes.iter().find_map(|prefix| {
        value
            .get(..prefix.len())
            .filter(|head| head.eq_ignore_ascii_case(prefix))
            .map(|_| &value[prefix.len()..])
    })
}
fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles
        .iter()
        .any(|needle| value.contains(&needle.to_lowercase()))
}

fn contains_surface_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| {
        let needle = needle.to_lowercase();
        if needle.chars().count() == 1 && !needle.is_ascii() {
            korean_short_term_matches(value, &needle)
        } else {
            value.contains(&needle)
        }
    })
}

fn korean_short_term_matches(value: &str, term: &str) -> bool {
    value.match_indices(term).any(|(index, matched)| {
        let before = value[..index].chars().next_back();
        if before.is_some_and(|character| character.is_alphanumeric() || character == '_') {
            return false;
        }
        let after = &value[index + matched.len()..];
        if after.is_empty()
            || after
                .chars()
                .next()
                .is_some_and(|character| !character.is_alphanumeric() && character != '_')
        {
            return true;
        }
        const PARTICLES: [&str; 14] = [
            "은", "는", "이", "가", "을", "를", "와", "과", "에", "에서", "로", "도", "만", "의",
        ];
        PARTICLES.iter().any(|particle| {
            after.strip_prefix(particle).is_some_and(|remainder| {
                remainder.is_empty()
                    || remainder
                        .chars()
                        .next()
                        .is_some_and(|character| !character.is_alphanumeric() && character != '_')
            })
        })
    })
}
fn matches_any(value: &str, candidates: &[&str]) -> bool {
    candidates.contains(&value)
}
fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
fn escape_markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', "<br>")
}
fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn extension_matches(path: &Path, format: OutputFormatIR) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(
        (format, extension.as_str()),
        (OutputFormatIR::Markdown, "md" | "markdown" | "txt")
            | (OutputFormatIR::Html, "html" | "htm")
            | (OutputFormatIR::Json, "json")
            | (OutputFormatIR::Csv, "csv")
            | (OutputFormatIR::Svg, "svg")
    )
}
fn staging_path(target: &Path) -> PathBuf {
    let mut stage = target.as_os_str().to_os_string();
    stage.push(format!(".{}.tmp", std::process::id()));
    PathBuf::from(stage)
}
fn sha256(bytes: &[u8]) -> String {
    format!("{:X}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_is_structured_and_analyzed_from_markdown() {
        let paper = parse_document(DocumentKindIR::Paper, "P-1", "# 인과 추론\nAuthors: A, B\nAbstract: 실험 요약\n## 결과\n처치는 관측값을 유의미하게 증가시켰다 [1].\n## 참고문헌\n- [1] Source").unwrap();
        let KnowledgeDocumentIR::Paper(paper) = paper else {
            panic!()
        };
        assert_eq!(paper.title, "인과 추론");
        assert_eq!(paper.authors, vec!["A", "B"]);
        assert_eq!(paper.references.len(), 1);
        assert!(!analyze_paper(&paper, true).is_empty());
    }

    #[test]
    fn table_chart_and_financial_analysis_preserve_exact_observations() {
        let table = parse_table(
            "T-1",
            "항목,2025,2026\n총자산,100,120\n총부채,40,50\n총자본,60,70",
        )
        .unwrap();
        assert!(analyze_table(&table, true)
            .iter()
            .any(|finding| finding.kind == FindingKindIR::Maximum));
        let chart = chart_from_table("C-1", &table).unwrap();
        assert!(analyze_chart(&chart, true)
            .iter()
            .any(|finding| finding.kind == FindingKindIR::Trend));
        let statement = financial_from_table("F-1", &table).unwrap();
        let accounting = analyze_financial(&statement, true)
            .into_iter()
            .filter(|finding| finding.kind == FindingKindIR::AccountingCheck)
            .collect::<Vec<_>>();
        assert_eq!(accounting.len(), 2);
        assert!(accounting
            .iter()
            .all(|finding| finding.statement.contains("일치")));
    }

    #[test]
    fn natural_language_revision_changes_only_grounded_field() {
        let mut document = create_document(
            DocumentKindIR::Paper,
            "P-2",
            "양자화 논문 작성",
            LanguageCodeIR::Korean,
        );
        revise_document(
            &mut document,
            "제목: 희소 양자화\n섹션 추가: 한계|표본 수가 작다",
        )
        .unwrap();
        let KnowledgeDocumentIR::Paper(paper) = document else {
            panic!()
        };
        assert_eq!(paper.title, "희소 양자화");
        assert!(paper
            .sections
            .iter()
            .any(|section| section.heading == "한계"));
    }

    #[test]
    fn output_file_and_text_modes_are_distinct() {
        let root =
            std::env::temp_dir().join(format!("b-core-knowledge-work-{}", std::process::id()));
        let path = root.join("table.csv");
        let request = KnowledgeWorkRequestIR {
            schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
            request_id: "KW-1".to_string(),
            command: "표를 분석해".to_string(),
            source: Some(KnowledgeSourceIR::Text {
                text: "name,value\na,1\nb,2".to_string(),
                format: Some(SourceTextFormatIR::Csv),
            }),
            document_kind: Some(DocumentKindIR::Table),
            output_language: Some(LanguageCodeIR::Korean),
            design: None,
            output: OutputDirectiveIR {
                mode: OutputModeIR::Both,
                format: OutputFormatIR::Csv,
                path: Some(path.to_string_lossy().to_string()),
                overwrite: true,
            },
            context_tags: Vec::new(),
            max_plan_steps: 12,
        };
        let product = execute_document_work(&request).unwrap();
        assert!(product.text_output.is_some());
        assert!(product.file_output.is_some());
        assert!(path.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn english_paper_and_plan_outputs_are_not_korean_templates() {
        let paper = execute_document_work(&KnowledgeWorkRequestIR {
            schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
            request_id: "PAPER-EN".to_string(),
            command: "Write an evidence-grounded paper about sparse reasoning".to_string(),
            source: None,
            document_kind: Some(DocumentKindIR::Paper),
            output_language: Some(LanguageCodeIR::English),
            design: None,
            output: OutputDirectiveIR {
                mode: OutputModeIR::Text,
                format: OutputFormatIR::Markdown,
                path: None,
                overwrite: false,
            },
            context_tags: vec!["research".to_string()],
            max_plan_steps: 12,
        })
        .unwrap();
        assert_eq!(paper.output_language, LanguageCodeIR::English);
        let paper_text = paper.text_output.unwrap();
        assert!(paper_text.contains("## Abstract"));
        assert!(paper_text.contains("## Introduction"));
        assert!(!paper_text.contains("## 초록"));
        assert!(paper
            .findings
            .iter()
            .all(|finding| !finding.statement.contains("논문은")));

        let plan = execute_document_work(&KnowledgeWorkRequestIR {
            schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
            request_id: "PLAN-EN".to_string(),
            command: "Create a project plan for verified document analysis".to_string(),
            source: None,
            document_kind: Some(DocumentKindIR::PlanProposal),
            output_language: Some(LanguageCodeIR::English),
            design: None,
            output: OutputDirectiveIR {
                mode: OutputModeIR::Text,
                format: OutputFormatIR::Markdown,
                path: None,
                overwrite: false,
            },
            context_tags: Vec::new(),
            max_plan_steps: 12,
        })
        .unwrap();
        let KnowledgeDocumentIR::PlanProposal(plan_document) = &plan.document else {
            panic!("plan document")
        };
        assert_eq!(plan_document.tasks.len(), 4);
        assert!(plan_document
            .tasks
            .iter()
            .all(|task| task.completion_condition.is_some()));
        let plan_text = plan.text_output.unwrap();
        assert!(plan_text.contains("**Objective:**"));
        assert!(plan_text.contains("## Tasks"));
    }

    #[test]
    fn plan_interpretation_detects_duplicate_unknown_and_cyclic_dependencies() {
        let plan = PlanProposalIR {
            schema: PLAN_PROPOSAL_SCHEMA.to_string(),
            document_id: "PLAN-BROKEN".to_string(),
            title: "Broken plan".to_string(),
            objective: "Exercise dependency analysis".to_string(),
            tasks: vec![
                PlanTaskIR {
                    task_id: "A".to_string(),
                    description: "A".to_string(),
                    dependencies: vec!["B".to_string()],
                    owner: None,
                    completion_condition: Some("done A".to_string()),
                },
                PlanTaskIR {
                    task_id: "B".to_string(),
                    description: "B".to_string(),
                    dependencies: vec!["A".to_string(), "MISSING".to_string()],
                    owner: None,
                    completion_condition: Some("done B".to_string()),
                },
                PlanTaskIR {
                    task_id: "B".to_string(),
                    description: "duplicate B".to_string(),
                    dependencies: Vec::new(),
                    owner: None,
                    completion_condition: Some("done duplicate".to_string()),
                },
            ],
            risks: Vec::new(),
            assumptions: Vec::new(),
        };
        let findings = analyze_document_in_language(
            &KnowledgeDocumentIR::PlanProposal(plan),
            LanguageCodeIR::English,
        );
        assert!(findings
            .iter()
            .any(|finding| finding.statement.contains("duplicate")));
        assert!(findings
            .iter()
            .any(|finding| finding.statement.contains("does not exist")));
        assert!(findings
            .iter()
            .any(|finding| finding.statement.contains("cycle")));
    }

    #[test]
    fn chart_writer_honors_bar_scatter_and_pie_semantics() {
        let table = parse_table("CHART-TYPES", "category,value\nA,10\nB,20\nC,30").unwrap();
        let mut chart = chart_from_table("CHART-TYPES", &table).unwrap();
        chart.chart_type = ChartTypeIR::Bar;
        assert!(render_chart_svg(&chart).unwrap().contains("<rect x="));
        chart.chart_type = ChartTypeIR::Scatter;
        assert!(render_chart_svg(&chart).unwrap().contains("<circle cx="));
        chart.chart_type = ChartTypeIR::Pie;
        let pie = render_chart_svg(&chart).unwrap();
        assert!(pie.contains("<path d="));
        assert!(pie.contains("50.0%"));
    }

    #[test]
    fn chart_natural_language_write_command_controls_the_materialized_type() {
        let product = execute_document_work(&KnowledgeWorkRequestIR {
            schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
            request_id: "BAR-COMMAND".to_string(),
            command: "Write a bar chart from this table".to_string(),
            source: Some(KnowledgeSourceIR::Text {
                text: "category,value\nA,10\nB,20".to_string(),
                format: Some(SourceTextFormatIR::Csv),
            }),
            document_kind: Some(DocumentKindIR::Chart),
            output_language: Some(LanguageCodeIR::English),
            design: None,
            output: OutputDirectiveIR {
                mode: OutputModeIR::Text,
                format: OutputFormatIR::Svg,
                path: None,
                overwrite: false,
            },
            context_tags: vec!["data".to_string()],
            max_plan_steps: 12,
        })
        .unwrap();
        let KnowledgeDocumentIR::Chart(chart) = product.document else {
            panic!("chart")
        };
        assert_eq!(chart.chart_type, ChartTypeIR::Bar);
        assert!(product.text_output.unwrap().contains("<rect x="));
    }

    #[test]
    fn business_genres_outrank_embedded_chart_terms_and_select_distinct_themes() {
        assert_eq!(
            infer_document_kind(
                "시장 차트와 재무 표를 포함한 투자자용 사업계획서를 작성해",
                None
            ),
            DocumentKindIR::BusinessPlan
        );
        assert_eq!(
            infer_document_kind("고객 그래프를 포함한 사업제안서를 디자인해", None),
            DocumentKindIR::BusinessProposal
        );
        assert_eq!(
            infer_document_design("투자위원회용 사업계획서", DocumentKindIR::BusinessPlan).theme,
            DocumentThemeIR::ExecutiveNavy
        );
        assert_eq!(
            infer_document_design("고객 제안용 사업제안서", DocumentKindIR::BusinessProposal).theme,
            DocumentThemeIR::ProposalCobalt
        );
    }

    #[test]
    fn guide_genre_outranks_incidental_table_syllables_and_materializes_a_real_manual() {
        let command =
            "GPT 사용 설명서를 디자인 좋게 작성해. 확인되지 않은 기능은 확인 필요라고 표시해.";
        assert_eq!(
            infer_document_kind(command, None),
            DocumentKindIR::UserGuide
        );
        assert_ne!(infer_document_kind(command, None), DocumentKindIR::Table);
        let product = execute_document_work(&KnowledgeWorkRequestIR {
            schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
            request_id: "GUIDE-KO-1".to_string(),
            command: command.to_string(),
            source: None,
            document_kind: None,
            output_language: Some(LanguageCodeIR::Korean),
            design: None,
            output: OutputDirectiveIR {
                mode: OutputModeIR::Text,
                format: OutputFormatIR::Html,
                path: None,
                overwrite: false,
            },
            context_tags: vec!["manual".to_string(), "gpt".to_string()],
            max_plan_steps: 16,
        })
        .unwrap();
        assert_eq!(product.document.kind(), DocumentKindIR::UserGuide);
        assert_eq!(product.design.theme, DocumentThemeIR::GuideIndigo);
        let KnowledgeDocumentIR::UserGuide(guide) = &product.document else {
            panic!("user guide")
        };
        assert_eq!(guide.title, "GPT 사용 설명서");
        assert!(guide
            .sections
            .iter()
            .any(|section| section.heading == "빠른 시작"));
        assert!(guide
            .sections
            .iter()
            .any(|section| section.heading == "좋은 질문 작성법"));
        assert!(!guide.examples.is_empty());
        assert!(!guide.cautions.is_empty());
        assert!(!guide.troubleshooting.is_empty());
        assert!(!guide.checklist.is_empty());
        let html = product.text_output.unwrap();
        for required in [
            "GPT 사용 설명서",
            "빠른 시작",
            "좋은 질문 작성법",
            "바로 쓰는 예시",
            "주의사항",
            "문제 해결",
            "빠른 확인 목록",
        ] {
            assert!(html.contains(required), "missing {required}");
        }
        assert!(html.contains("class=\"theme-guide\""));
        assert_eq!(html.matches("class=\"sheet").count(), 4);
        assert!(html.contains("--page-width:210mm"));
        assert!(html.contains("counter(sheet)"));
        assert!(!html.contains("EVIDENCE REVIEW"));
        assert!(!html.contains("DATA TABLE"));
    }

    #[test]
    fn html_business_plan_is_print_ready_and_reports_the_applied_design() {
        let root =
            std::env::temp_dir().join(format!("b-core-business-html-{}", std::process::id()));
        let path = root.join("plan.html");
        let product = execute_document_work(&KnowledgeWorkRequestIR {
            schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
            request_id: "BUSINESS-HTML-1".to_string(),
            command: "투자위원회용 사업계획서를 디자인 좋게 작성해".to_string(),
            source: None,
            document_kind: None,
            output_language: Some(LanguageCodeIR::Korean),
            design: Some(DocumentDesignIR {
                schema: DOCUMENT_DESIGN_SCHEMA.to_string(),
                theme: DocumentThemeIR::ExecutiveNavy,
                page_size: PageSizeIR::A4,
                brand_name: Some("B_CORE LAB".to_string()),
                accent_color: Some("#087F6B".to_string()),
                compact: false,
                show_table_of_contents: true,
                show_page_furniture: true,
            }),
            output: OutputDirectiveIR {
                mode: OutputModeIR::Both,
                format: OutputFormatIR::Html,
                path: Some(path.to_string_lossy().to_string()),
                overwrite: true,
            },
            context_tags: vec!["business".to_string(), "design".to_string()],
            max_plan_steps: 12,
        })
        .unwrap();
        assert_eq!(product.document.kind(), DocumentKindIR::BusinessPlan);
        assert_eq!(product.design.theme, DocumentThemeIR::ExecutiveNavy);
        assert_eq!(product.design.accent_color.as_deref(), Some("#087F6B"));
        let html = product.text_output.as_deref().unwrap();
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("@page { size: A4"));
        assert!(html.contains("class=\"sheet cover\""));
        assert!(html.contains("--page-width:210mm"));
        assert!(html.contains("--page-height:297mm"));
        assert!(html.contains("class=\"toc\""));
        assert!(html.contains("class=\"timeline\""));
        assert!(html.contains("--accent:#087F6B"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), html);
        assert!(product.file_output.is_some());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_brand_color_and_wrong_html_extension_fail_closed() {
        let mut request = KnowledgeWorkRequestIR {
            schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
            request_id: "BUSINESS-HTML-INVALID".to_string(),
            command: "Write a business proposal".to_string(),
            source: None,
            document_kind: Some(DocumentKindIR::BusinessProposal),
            output_language: Some(LanguageCodeIR::English),
            design: Some(DocumentDesignIR {
                schema: DOCUMENT_DESIGN_SCHEMA.to_string(),
                theme: DocumentThemeIR::ProposalCobalt,
                page_size: PageSizeIR::Letter,
                brand_name: Some("Acme".to_string()),
                accent_color: Some("blue".to_string()),
                compact: false,
                show_table_of_contents: true,
                show_page_furniture: true,
            }),
            output: OutputDirectiveIR {
                mode: OutputModeIR::Text,
                format: OutputFormatIR::Html,
                path: None,
                overwrite: false,
            },
            context_tags: Vec::new(),
            max_plan_steps: 12,
        };
        assert_eq!(
            execute_document_work(&request).unwrap_err(),
            KnowledgeWorkError::InvalidRequest
        );
        request.design.as_mut().unwrap().accent_color = Some("#2457D6".to_string());
        request.output.mode = OutputModeIR::File;
        request.output.path = Some("proposal.md".to_string());
        assert_eq!(
            execute_document_work(&request).unwrap_err(),
            KnowledgeWorkError::InvalidOutputPath
        );
    }
}
