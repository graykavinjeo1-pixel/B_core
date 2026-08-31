use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;

use dockable_semantic_core::{
    AssessmentVerdictIR, DeliberationFactIR, DockableCore, QualityCriterionIR, SwarmDeliberationIR,
    SwarmDeliberationRequestIR, SWARM_DELIBERATION_REQUEST_SCHEMA,
};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::long_term_repair_catalog::statutory_catalog;

pub const LONG_TERM_REPAIR_PLAN_REQUEST_SCHEMA: &str = "B_CORE_LONG_TERM_REPAIR_PLAN_REQUEST_1";
pub const LONG_TERM_REPAIR_PLAN_RESPONSE_SCHEMA: &str = "B_CORE_LONG_TERM_REPAIR_PLAN_RESPONSE_1";
pub const LONG_TERM_REPAIR_KNOWLEDGE_VERSION: &str =
    "KOREA_MULTI_FAMILY_LONG_TERM_REPAIR_2026_07_30_V1";
const MAX_EVIDENCE_FILES: usize = 64;
const MAX_EVIDENCE_BYTES: u64 = 64 * 1024 * 1024;
const REQUIRED_PAGE_COUNT: usize = 50;
const REQUIRED_PLAN_YEARS: u16 = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepairMethodIR {
    PartialRepair,
    FullRepair,
    PartialReplacement,
    FullReplacement,
    FullCoating,
}

impl RepairMethodIR {
    fn korean_label(self) -> &'static str {
        match self {
            Self::PartialRepair => "부분수선",
            Self::FullRepair => "전면수선",
            Self::PartialReplacement => "부분교체",
            Self::FullReplacement => "전면교체",
            Self::FullCoating => "전면도장",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatutoryRepairMethodIR {
    pub method: RepairMethodIR,
    pub cycle_years: u16,
    pub repair_rate_percent: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatutoryRepairItemIR {
    pub id: String,
    pub group: String,
    pub subgroup: String,
    pub work_type: String,
    pub methods: Vec<StatutoryRepairMethodIR>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceKindIR {
    Auto,
    PlainText,
    Markdown,
    Csv,
    Json,
    Pdf,
    Hwpx,
    Hwp,
    Image,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceInputIR {
    pub evidence_id: String,
    pub path: String,
    #[serde(default = "default_evidence_kind")]
    pub kind: EvidenceKindIR,
}

fn default_evidence_kind() -> EvidenceKindIR {
    EvidenceKindIR::Auto
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceStatusIR {
    Extracted,
    ExtractorUnavailable,
    NoTextLayer,
    UnsupportedFormat,
    ReadFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceExtractionReceiptIR {
    pub evidence_id: String,
    pub path: String,
    pub detected_kind: EvidenceKindIR,
    pub status: EvidenceStatusIR,
    pub source_sha256: String,
    pub extracted_text_sha256: String,
    pub extracted_characters: usize,
    #[serde(default)]
    pub structured_block_count: usize,
    #[serde(default)]
    pub section_or_page_count: usize,
    #[serde(default)]
    pub table_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_ocr_confidence_millis: Option<u16>,
    #[serde(default)]
    pub structure_sha256: String,
    pub extractor: String,
    pub diagnostic_code: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceBlockKindIR {
    Heading,
    Paragraph,
    ListItem,
    Table,
    OcrLine,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBlockIR {
    pub block_id: String,
    pub kind: EvidenceBlockKindIR,
    pub section_or_page: usize,
    pub ordinal: usize,
    pub text: String,
    pub source_location: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence_millis: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredEvidenceIR {
    pub schema: String,
    pub evidence_id: String,
    pub blocks: Vec<EvidenceBlockIR>,
    pub structure_sha256: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApartmentProfileIR {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub complex_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_approval_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub household_count: Option<u32>,
    /// Thousandths of a square metre. Integer storage prevents hidden float drift.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_supply_area_milli_square_meters: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_reserve_won: Option<i64>,
    #[serde(default)]
    pub household_area_types: Vec<HouseholdAreaTypeIR>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HouseholdAreaTypeIR {
    pub label: String,
    pub household_count: u32,
    pub supply_area_milli_square_meters: i64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApartmentRepairRuleIR {
    pub item_id: String,
    pub method: RepairMethodIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cycle_years: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair_rate_percent: Option<u16>,
    /// Required when the apartment value differs from the frozen statutory baseline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjustment_approval_evidence_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostInputIR {
    pub item_id: String,
    pub method: RepairMethodIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applicable: Option<bool>,
    /// Thousandths of the declared unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity_milli_units: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_price_won: Option<i64>,
    /// 10_000 means a factor of 1.0000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overhead_factor_basis_points: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_scheduled_year: Option<u16>,
    #[serde(default)]
    pub overlapping_partial_repair_deduction_won: i64,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnualReserveInputIR {
    pub year: u16,
    pub contribution_won: i64,
    #[serde(default)]
    pub disposition_transfer_won: i64,
    #[serde(default)]
    pub interest_won: i64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LongTermRepairPlanRequestIR {
    pub schema: String,
    pub request_id: String,
    pub command: String,
    pub as_of_date: String,
    pub plan_start_year: u16,
    #[serde(default = "default_plan_years")]
    pub plan_years: u16,
    #[serde(default)]
    pub evidence: Vec<EvidenceInputIR>,
    #[serde(default)]
    pub profile: ApartmentProfileIR,
    #[serde(default)]
    pub apartment_rules: Vec<ApartmentRepairRuleIR>,
    #[serde(default)]
    pub cost_inputs: Vec<CostInputIR>,
    #[serde(default)]
    pub reserve_inputs: Vec<AnnualReserveInputIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_html_path: Option<String>,
    #[serde(default = "default_max_plan_steps")]
    pub max_plan_steps: usize,
}

fn default_plan_years() -> u16 {
    REQUIRED_PLAN_YEARS
}

fn default_max_plan_steps() -> usize {
    16
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuleResolutionIR {
    StatutoryBaseline,
    ApartmentMatchesBaseline,
    ApprovedApartmentAdjustment,
    ReviewRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairItemDecisionIR {
    pub item_id: String,
    pub group: String,
    pub subgroup: String,
    pub work_type: String,
    pub method: RepairMethodIR,
    pub legal_cycle_years: u16,
    pub legal_repair_rate_percent: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apartment_cycle_years: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apartment_repair_rate_percent: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_cycle_years: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_repair_rate_percent: Option<u16>,
    pub resolution: RuleResolutionIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applicable: Option<bool>,
    pub scheduled_years: Vec<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub one_time_cost_won: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_plan_cost_won: Option<i64>,
    pub evidence_refs: Vec<String>,
    pub statutory_notes: Vec<String>,
    pub status_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnualPlanIR {
    pub year: u16,
    pub scheduled_decision_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expenditure_won: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReserveYearIR {
    pub year: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening_balance_won: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contribution_won: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expenditure_won: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closing_balance_won: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonthlyAreaChargeIR {
    pub year: u16,
    pub area_type_label: String,
    pub household_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monthly_charge_per_household_won: Option<i64>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReportBlockStatusIR {
    Verified,
    Computed,
    Advisory,
    NeedsConfirmation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportBlockIR {
    pub heading: String,
    pub body: String,
    pub status: ReportBlockStatusIR,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportPageIR {
    pub page_number: usize,
    pub section_id: String,
    pub section_title: String,
    pub page_title: String,
    pub blocks: Vec<ReportBlockIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceConflictIR {
    pub conflict_id: String,
    pub older_or_secondary_claim: String,
    pub controlling_treatment: String,
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LongTermRepairFileReceiptIR {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    pub a4_page_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlanReadinessIR {
    DraftRequiresEvidence,
    ProfessionalReviewReady,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LongTermRepairPlanResponseIR {
    pub schema: String,
    pub request_id: String,
    pub knowledge_version: String,
    pub statutory_effective_date: String,
    pub statutory_item_count: usize,
    pub statutory_method_count: usize,
    pub reasoning_plan_sha256: String,
    pub extraction_receipts: Vec<EvidenceExtractionReceiptIR>,
    pub resolved_profile: ApartmentProfileIR,
    pub decisions: Vec<RepairItemDecisionIR>,
    pub annual_plan: Vec<AnnualPlanIR>,
    pub reserve_projection: Vec<ReserveYearIR>,
    pub monthly_area_charges: Vec<MonthlyAreaChargeIR>,
    pub source_conflicts: Vec<SourceConflictIR>,
    pub missing_required_inputs: Vec<String>,
    pub pages: Vec<ReportPageIR>,
    pub readiness: PlanReadinessIR,
    pub approval_ready: bool,
    pub professional_review_required: bool,
    pub deliberation: SwarmDeliberationIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_receipt: Option<LongTermRepairFileReceiptIR>,
    pub artifact_sha256: String,
    pub external_model_calls: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LongTermRepairPlanError {
    InvalidRequest,
    DuplicateInput,
    CatalogInvariant,
    ArithmeticOverflow,
    Deliberation,
    OutputWrite,
}

pub(crate) struct ExtractedEvidence {
    pub receipt: EvidenceExtractionReceiptIR,
    pub text: String,
    pub structured: StructuredEvidenceIR,
}

pub fn process_long_term_repair_plan(
    core: &DockableCore,
    request: &LongTermRepairPlanRequestIR,
    reasoning_plan_sha256: &str,
) -> Result<LongTermRepairPlanResponseIR, LongTermRepairPlanError> {
    validate_request(request, reasoning_plan_sha256)?;
    let catalog = statutory_catalog();
    validate_catalog(&catalog)?;
    validate_unique_inputs(request)?;

    let extracted = request
        .evidence
        .iter()
        .map(extract_evidence)
        .collect::<Vec<_>>();
    let extraction_receipts = extracted
        .iter()
        .map(|evidence| evidence.receipt.clone())
        .collect::<Vec<_>>();
    let resolved_profile = resolve_profile(&request.profile, &extracted);
    let evidence_status = extraction_receipts
        .iter()
        .map(|receipt| (receipt.evidence_id.as_str(), receipt.status))
        .collect::<BTreeMap<_, _>>();
    let decisions = build_decisions(request, &catalog, &evidence_status)?;
    let annual_plan = build_annual_plan(request, &decisions)?;
    let reserve_projection = build_reserve_projection(request, &resolved_profile, &annual_plan)?;
    let monthly_area_charges = build_monthly_area_charges(&resolved_profile, &reserve_projection)?;
    let source_conflicts = source_conflicts();
    let missing_required_inputs =
        missing_required_inputs(request, &resolved_profile, &extraction_receipts, &decisions);
    let pages = assemble_pages(
        request,
        &resolved_profile,
        &extraction_receipts,
        &decisions,
        &annual_plan,
        &reserve_projection,
        &monthly_area_charges,
        &source_conflicts,
        &missing_required_inputs,
    );
    if pages.len() != REQUIRED_PAGE_COUNT {
        return Err(LongTermRepairPlanError::CatalogInvariant);
    }
    let readiness = if missing_required_inputs.is_empty()
        && extraction_receipts
            .iter()
            .all(|receipt| receipt.status == EvidenceStatusIR::Extracted)
        && decisions
            .iter()
            .all(|decision| decision.resolution != RuleResolutionIR::ReviewRequired)
    {
        PlanReadinessIR::ProfessionalReviewReady
    } else {
        PlanReadinessIR::DraftRequiresEvidence
    };
    let deliberation = deliberate(
        core,
        request,
        reasoning_plan_sha256,
        &extraction_receipts,
        &decisions,
        &pages,
        readiness,
    )?;
    let artifact_sha256 = sha256_json(&(
        request,
        &extraction_receipts,
        &resolved_profile,
        &decisions,
        &annual_plan,
        &reserve_projection,
        &monthly_area_charges,
        &source_conflicts,
        &missing_required_inputs,
        &pages,
        reasoning_plan_sha256,
        &deliberation.deliberation_sha256,
    ));
    let file_receipt = if let Some(path) = &request.output_html_path {
        Some(write_html(
            Path::new(path),
            request,
            &pages,
            readiness,
            &artifact_sha256,
        )?)
    } else {
        None
    };
    Ok(LongTermRepairPlanResponseIR {
        schema: LONG_TERM_REPAIR_PLAN_RESPONSE_SCHEMA.to_string(),
        request_id: request.request_id.clone(),
        knowledge_version: LONG_TERM_REPAIR_KNOWLEDGE_VERSION.to_string(),
        statutory_effective_date: "2026-07-30".to_string(),
        statutory_item_count: catalog.len(),
        statutory_method_count: catalog.iter().map(|item| item.methods.len()).sum(),
        reasoning_plan_sha256: reasoning_plan_sha256.to_string(),
        extraction_receipts,
        resolved_profile,
        decisions,
        annual_plan,
        reserve_projection,
        monthly_area_charges,
        source_conflicts,
        missing_required_inputs,
        pages,
        readiness,
        approval_ready: readiness == PlanReadinessIR::ProfessionalReviewReady
            && deliberation.accepted,
        professional_review_required: true,
        deliberation,
        file_receipt,
        artifact_sha256,
        external_model_calls: 0,
    })
}

fn validate_request(
    request: &LongTermRepairPlanRequestIR,
    reasoning_plan_sha256: &str,
) -> Result<(), LongTermRepairPlanError> {
    let output_is_html = request.output_html_path.as_ref().is_none_or(|path| {
        Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("html"))
    });
    if request.schema != LONG_TERM_REPAIR_PLAN_REQUEST_SCHEMA
        || request.request_id.trim().is_empty()
        || request.request_id.len() > 128
        || request.command.trim().is_empty()
        || request.command.len() > 64 * 1024
        || !valid_date_shape(&request.as_of_date)
        || !(2000..=2200).contains(&request.plan_start_year)
        || request.plan_years != REQUIRED_PLAN_YEARS
        || request.evidence.len() > MAX_EVIDENCE_FILES
        || !(5..=32).contains(&request.max_plan_steps)
        || reasoning_plan_sha256.len() != 64
        || !output_is_html
        || request.profile.household_count == Some(0)
        || request
            .profile
            .total_supply_area_milli_square_meters
            .is_some_and(|value| value <= 0)
        || request
            .profile
            .current_reserve_won
            .is_some_and(|value| value < 0)
        || request
            .profile
            .household_area_types
            .iter()
            .any(|area_type| {
                area_type.label.trim().is_empty()
                    || area_type.household_count == 0
                    || area_type.supply_area_milli_square_meters <= 0
            })
        || request.apartment_rules.iter().any(|rule| {
            rule.cycle_years == Some(0)
                || rule
                    .repair_rate_percent
                    .is_some_and(|rate| !(1..=100).contains(&rate))
        })
        || request.reserve_inputs.iter().any(|input| {
            input.year < request.plan_start_year
                || input.year >= request.plan_start_year + request.plan_years
                || input.contribution_won < 0
                || input.interest_won < 0
        })
    {
        return Err(LongTermRepairPlanError::InvalidRequest);
    }
    Ok(())
}

fn valid_date_shape(value: &str) -> bool {
    value.len() == 10
        && value.as_bytes()[4] == b'-'
        && value.as_bytes()[7] == b'-'
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn validate_catalog(catalog: &[StatutoryRepairItemIR]) -> Result<(), LongTermRepairPlanError> {
    let ids = catalog
        .iter()
        .map(|item| item.id.as_str())
        .collect::<BTreeSet<_>>();
    let groups = catalog
        .iter()
        .map(|item| item.group.as_str())
        .collect::<BTreeSet<_>>();
    if catalog.len() != 69
        || ids.len() != 69
        || groups.len() != 7
        || catalog.iter().any(|item| {
            item.methods.is_empty()
                || item.methods.iter().any(|method| {
                    method.cycle_years == 0
                        || method.repair_rate_percent == 0
                        || method.repair_rate_percent > 100
                })
        })
    {
        return Err(LongTermRepairPlanError::CatalogInvariant);
    }
    Ok(())
}

fn validate_unique_inputs(
    request: &LongTermRepairPlanRequestIR,
) -> Result<(), LongTermRepairPlanError> {
    let evidence_ids = request
        .evidence
        .iter()
        .map(|input| input.evidence_id.trim())
        .collect::<BTreeSet<_>>();
    let rule_keys = request
        .apartment_rules
        .iter()
        .map(|rule| (&rule.item_id, rule.method))
        .collect::<BTreeSet<_>>();
    let cost_keys = request
        .cost_inputs
        .iter()
        .map(|input| (&input.item_id, input.method))
        .collect::<BTreeSet<_>>();
    let reserve_years = request
        .reserve_inputs
        .iter()
        .map(|input| input.year)
        .collect::<BTreeSet<_>>();
    if evidence_ids.len() != request.evidence.len()
        || evidence_ids.contains("")
        || rule_keys.len() != request.apartment_rules.len()
        || cost_keys.len() != request.cost_inputs.len()
        || reserve_years.len() != request.reserve_inputs.len()
        || request
            .cost_inputs
            .iter()
            .any(|input| input.overlapping_partial_repair_deduction_won < 0)
    {
        return Err(LongTermRepairPlanError::DuplicateInput);
    }
    Ok(())
}

pub(crate) fn extract_evidence(input: &EvidenceInputIR) -> ExtractedEvidence {
    let path = Path::new(&input.path);
    let kind = detect_kind(path, input.kind);
    let bytes = fs::read(path);
    let (source_sha256, within_limit) = match &bytes {
        Ok(bytes) => (
            sha256_bytes(bytes),
            bytes.len() as u64 <= MAX_EVIDENCE_BYTES,
        ),
        Err(_) => (String::new(), false),
    };
    if bytes.is_err() || !within_limit {
        return extracted_failure(
            input,
            kind,
            EvidenceStatusIR::ReadFailed,
            source_sha256,
            "FILE_READ_FAILED_OR_LIMIT_EXCEEDED",
        );
    }
    let result = match kind {
        EvidenceKindIR::PlainText
        | EvidenceKindIR::Markdown
        | EvidenceKindIR::Csv
        | EvidenceKindIR::Json => decode_text(bytes.as_deref().unwrap_or_default())
            .map(|text| (text, "B_CORE_TEXT_DECODER".to_string()))
            .map_err(|code| (EvidenceStatusIR::ReadFailed, code)),
        EvidenceKindIR::Pdf => extract_pdf(path),
        EvidenceKindIR::Hwpx => extract_hwpx(path),
        EvidenceKindIR::Hwp => extract_hwp(path),
        EvidenceKindIR::Image => extract_image(path),
        EvidenceKindIR::Auto => Err((
            EvidenceStatusIR::UnsupportedFormat,
            "UNRECOGNIZED_FILE_EXTENSION",
        )),
    };
    match result {
        Ok((text, extractor)) if !text.trim().is_empty() => {
            let text = normalize_extracted_text(&text);
            let structured = structure_extracted_text(&input.evidence_id, kind, &text);
            let structure_sha256 = structured.structure_sha256.clone();
            let structured_block_count = structured.blocks.len();
            let section_or_page_count = structured
                .blocks
                .iter()
                .map(|block| block.section_or_page)
                .max()
                .unwrap_or(0);
            let table_count = structured
                .blocks
                .iter()
                .filter(|block| block.kind == EvidenceBlockKindIR::Table)
                .count();
            let confidences = structured
                .blocks
                .iter()
                .filter_map(|block| block.confidence_millis)
                .collect::<Vec<_>>();
            let mean_ocr_confidence_millis = if confidences.is_empty() {
                None
            } else {
                Some(
                    (confidences
                        .iter()
                        .map(|value| u64::from(*value))
                        .sum::<u64>()
                        / confidences.len() as u64) as u16,
                )
            };
            ExtractedEvidence {
                receipt: EvidenceExtractionReceiptIR {
                    evidence_id: input.evidence_id.clone(),
                    path: input.path.clone(),
                    detected_kind: kind,
                    status: EvidenceStatusIR::Extracted,
                    source_sha256,
                    extracted_text_sha256: sha256_bytes(text.as_bytes()),
                    extracted_characters: text.chars().count(),
                    structured_block_count,
                    section_or_page_count,
                    table_count,
                    mean_ocr_confidence_millis,
                    structure_sha256,
                    extractor,
                    diagnostic_code: "TEXT_AND_STRUCTURE_EXTRACTED_AND_HASH_BOUND".to_string(),
                },
                text,
                structured,
            }
        }
        Ok((_text, extractor)) => ExtractedEvidence {
            receipt: EvidenceExtractionReceiptIR {
                evidence_id: input.evidence_id.clone(),
                path: input.path.clone(),
                detected_kind: kind,
                status: EvidenceStatusIR::NoTextLayer,
                source_sha256,
                extracted_text_sha256: String::new(),
                extracted_characters: 0,
                structured_block_count: 0,
                section_or_page_count: 0,
                table_count: 0,
                mean_ocr_confidence_millis: None,
                structure_sha256: String::new(),
                extractor,
                diagnostic_code: "NO_TEXT_LAYER_REQUIRES_OCR".to_string(),
            },
            text: String::new(),
            structured: empty_structured_evidence(&input.evidence_id),
        },
        Err((status, code)) => extracted_failure(input, kind, status, source_sha256, code),
    }
}

fn detect_kind(path: &Path, declared: EvidenceKindIR) -> EvidenceKindIR {
    if declared != EvidenceKindIR::Auto {
        return declared;
    }
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "txt" => EvidenceKindIR::PlainText,
        "md" => EvidenceKindIR::Markdown,
        "csv" | "tsv" => EvidenceKindIR::Csv,
        "json" => EvidenceKindIR::Json,
        "pdf" => EvidenceKindIR::Pdf,
        "hwpx" => EvidenceKindIR::Hwpx,
        "hwp" => EvidenceKindIR::Hwp,
        "png" | "jpg" | "jpeg" | "tif" | "tiff" | "bmp" | "webp" => EvidenceKindIR::Image,
        _ => EvidenceKindIR::Auto,
    }
}

fn decode_text(bytes: &[u8]) -> Result<String, &'static str> {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok(text.trim_start_matches('\u{feff}').to_string());
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&units).map_err(|_| "INVALID_UTF16_LE");
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&units).map_err(|_| "INVALID_UTF16_BE");
    }
    Err("UNSUPPORTED_TEXT_ENCODING")
}

fn extract_pdf(path: &Path) -> Result<(String, String), (EvidenceStatusIR, &'static str)> {
    std::panic::catch_unwind(|| pdf_extract::extract_text(path))
        .map_err(|_| {
            (
                EvidenceStatusIR::ReadFailed,
                "PDF_EXTRACTOR_PANIC_CONTAINED",
            )
        })?
        .map(|text| (text, "RUST_PDF_EXTRACT_0_12".to_string()))
        .map_err(|_| (EvidenceStatusIR::ReadFailed, "PDF_PARSE_FAILED"))
}

fn extract_hwp(path: &Path) -> Result<(String, String), (EvidenceStatusIR, &'static str)> {
    match std::panic::catch_unwind(|| extract_hwp_sections(path)) {
        Ok(Ok(text)) => Ok((text, "RUST_HWARANG_0_2_STRUCTURED_HWP5".to_string())),
        Ok(Err(_)) => extract_with_command("hwp5txt", path, &[]),
        Err(_) => Err((
            EvidenceStatusIR::ReadFailed,
            "HWP_EXTRACTOR_PANIC_CONTAINED",
        )),
    }
}

fn extract_hwp_sections(path: &Path) -> Result<String, ()> {
    let file = fs::File::open(path).map_err(|_| ())?;
    let mut compound = cfb::CompoundFile::open(file).map_err(|_| ())?;
    let header = {
        let mut stream = compound.open_stream("/FileHeader").map_err(|_| ())?;
        hwarang::hwp::header::FileHeader::from_reader(&mut stream).map_err(|_| ())?
    };
    let document_info = {
        let mut stream = compound.open_stream("/DocInfo").map_err(|_| ())?;
        let bytes = hwarang::hwp::stream::read_and_decompress(&mut stream, header.compressed)
            .map_err(|_| ())?;
        let records = hwarang::hwp::record::read_records(&bytes).map_err(|_| ())?;
        hwarang::hwp::docinfo::parse_doc_info(&records).map_err(|_| ())?
    };
    let storage = if header.distribution {
        "ViewText"
    } else {
        "BodyText"
    };
    let mut output = String::new();
    for section_index in 0..document_info.section_count {
        let stream_name = format!("/{storage}/Section{section_index}");
        let Ok(mut stream) = compound.open_stream(&stream_name) else {
            continue;
        };
        let raw = hwarang::hwp::stream::read_stream_data(&mut stream).map_err(|_| ())?;
        let bytes = if header.distribution {
            let decrypted =
                hwarang::hwp::crypto::decrypt_distribution_stream(&raw).map_err(|_| ())?;
            if header.compressed {
                hwarang::hwp::stream::decompress(&decrypted).map_err(|_| ())?
            } else {
                decrypted
            }
        } else if header.compressed {
            hwarang::hwp::stream::decompress(&raw).map_err(|_| ())?
        } else {
            raw
        };
        let records = hwarang::hwp::record::read_records(&bytes).map_err(|_| ())?;
        let mut section_text = String::new();
        hwarang::extract::extract_section_text(&records, &mut section_text);
        output.push_str(&format!("\n[[B_CORE_SECTION:{}]]\n", section_index + 1));
        output.push_str(&section_text);
    }
    Ok(output)
}

fn extract_hwpx(path: &Path) -> Result<(String, String), (EvidenceStatusIR, &'static str)> {
    let file =
        fs::File::open(path).map_err(|_| (EvidenceStatusIR::ReadFailed, "HWPX_OPEN_FAILED"))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|_| (EvidenceStatusIR::ReadFailed, "HWPX_ZIP_INVALID"))?;
    let mut names = (0..archive.len())
        .filter_map(|index| {
            archive
                .by_index(index)
                .ok()
                .map(|file| file.name().to_string())
        })
        .filter(|name| name.starts_with("Contents/section") && name.ends_with(".xml"))
        .collect::<Vec<_>>();
    names.sort();
    if names.is_empty() {
        return Err((EvidenceStatusIR::ReadFailed, "HWPX_SECTION_XML_MISSING"));
    }
    let mut output = String::new();
    for (section_index, name) in names.into_iter().enumerate() {
        let mut entry = archive
            .by_name(&name)
            .map_err(|_| (EvidenceStatusIR::ReadFailed, "HWPX_SECTION_READ_FAILED"))?;
        let mut xml = String::new();
        entry
            .read_to_string(&mut xml)
            .map_err(|_| (EvidenceStatusIR::ReadFailed, "HWPX_SECTION_NOT_UTF8"))?;
        output.push_str(&format!("\n[[B_CORE_SECTION:{}]]\n", section_index + 1));
        output.push_str(&extract_xml_text(&xml));
        output.push('\n');
    }
    Ok((output, "RUST_HWPX_ZIP_XML".to_string()))
}

fn extract_xml_text(xml: &str) -> String {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut output = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Text(text)) => {
                if let Ok(value) = text.decode() {
                    if !value.trim().is_empty() {
                        if !output.ends_with(['\n', ' ']) {
                            output.push(' ');
                        }
                        output.push_str(value.trim());
                    }
                }
            }
            Ok(Event::End(end)) => {
                let name = end.name();
                if matches!(name.as_ref(), b"hp:p" | b"p" | b"hp:tr" | b"tr") {
                    output.push('\n');
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    output
}

fn extract_with_command(
    program: &str,
    path: &Path,
    trailing_args: &[&str],
) -> Result<(String, String), (EvidenceStatusIR, &'static str)> {
    let mut command = Command::new(program);
    command.arg(path);
    command.args(trailing_args);
    match command.output() {
        Ok(output) if output.status.success() => String::from_utf8(output.stdout)
            .map(|text| (text, format!("LOCAL_{program}")))
            .map_err(|_| (EvidenceStatusIR::ReadFailed, "EXTRACTOR_OUTPUT_NOT_UTF8")),
        Ok(_) => Err((EvidenceStatusIR::ReadFailed, "LOCAL_EXTRACTOR_FAILED")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err((
            EvidenceStatusIR::ExtractorUnavailable,
            "LOCAL_EXTRACTOR_NOT_INSTALLED",
        )),
        Err(_) => Err((EvidenceStatusIR::ReadFailed, "LOCAL_EXTRACTOR_START_FAILED")),
    }
}

fn extract_image(path: &Path) -> Result<(String, String), (EvidenceStatusIR, &'static str)> {
    match extract_paddle_ocr(path) {
        Ok(result) if !result.0.trim().is_empty() => Ok(result),
        _ => match extract_with_command("tesseract", path, &["stdout", "-l", "kor+eng"]) {
            Ok(result) if !result.0.trim().is_empty() => Ok(result),
            _ => extract_windows_ocr(path),
        },
    }
}

fn extract_paddle_ocr(path: &Path) -> Result<(String, String), (EvidenceStatusIR, &'static str)> {
    const SCRIPT: &str = r#"import json, sys
try:
    from paddleocr import PaddleOCR
except Exception:
    raise SystemExit(41)
sys.stdout.reconfigure(encoding='utf-8')
ocr = PaddleOCR(lang='korean', ocr_version='PP-OCRv5', use_doc_orientation_classify=True, use_doc_unwarping=True, use_textline_orientation=True)
rows = []
for result in ocr.predict(sys.argv[1]):
    data = result.json
    if callable(data):
        data = data()
    if isinstance(data, str):
        data = json.loads(data)
    data = data.get('res', data)
    texts = data.get('rec_texts', [])
    scores = data.get('rec_scores', [])
    boxes = data.get('rec_boxes', [])
    for index, value in enumerate(texts):
        score = float(scores[index]) if index < len(scores) else 0.0
        box = boxes[index] if index < len(boxes) else []
        if hasattr(box, 'tolist'):
            box = box.tolist()
        rows.append({'text': str(value), 'score': score, 'box': box})
print('B_CORE_PADDLE_JSON=' + json.dumps(rows, ensure_ascii=False, separators=(',', ':')))
"#;
    let mut candidates = Vec::<(String, Vec<String>)>::new();
    if let Ok(program) = std::env::var("B_CORE_PADDLEOCR_PYTHON") {
        if !program.trim().is_empty() {
            candidates.push((program, Vec::new()));
        }
    }
    candidates.extend([
        ("python".to_string(), Vec::new()),
        ("py".to_string(), vec!["-3".to_string()]),
    ]);
    for (program, prefix) in candidates {
        let mut command = Command::new(&program);
        command.args(&prefix).args(["-c", SCRIPT]).arg(path);
        let output = match command.output() {
            Ok(output) => output,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err((EvidenceStatusIR::ReadFailed, "PADDLE_OCR_START_FAILED")),
        };
        if !output.status.success() {
            continue;
        }
        let stdout = String::from_utf8(output.stdout)
            .map_err(|_| (EvidenceStatusIR::ReadFailed, "PADDLE_OCR_OUTPUT_NOT_UTF8"))?;
        let Some(json) = stdout
            .lines()
            .rev()
            .find_map(|line| line.strip_prefix("B_CORE_PADDLE_JSON="))
        else {
            continue;
        };
        let rows = serde_json::from_str::<Vec<serde_json::Value>>(json)
            .map_err(|_| (EvidenceStatusIR::ReadFailed, "PADDLE_OCR_JSON_INVALID"))?;
        let mut text = String::new();
        for row in rows {
            let value = row
                .get("text")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            if value.trim().is_empty() {
                continue;
            }
            let confidence = row
                .get("score")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            let bbox = row
                .get("box")
                .map(|value| value.to_string())
                .unwrap_or_else(|| "[]".to_string());
            text.push_str(&format!(
                "[[B_CORE_OCR|{:.3}|{}|]]{}\n",
                confidence, bbox, value
            ));
        }
        if !text.trim().is_empty() {
            return Ok((text, "PADDLEOCR_PP_OCRV5_KOREAN".to_string()));
        }
    }
    Err((
        EvidenceStatusIR::ExtractorUnavailable,
        "PADDLEOCR_PP_OCRV5_NOT_INSTALLED",
    ))
}

#[cfg(target_os = "windows")]
fn extract_windows_ocr(path: &Path) -> Result<(String, String), (EvidenceStatusIR, &'static str)> {
    const SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Storage.StorageFile, Windows.Storage, ContentType=WindowsRuntime]
$null = [Windows.Storage.Streams.IRandomAccessStream, Windows.Storage.Streams, ContentType=WindowsRuntime]
$null = [Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType=WindowsRuntime]
$null = [Windows.Media.Ocr.OcrResult, Windows.Foundation, ContentType=WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder, Windows.Foundation, ContentType=WindowsRuntime]
$null = [Windows.Graphics.Imaging.SoftwareBitmap, Windows.Foundation, ContentType=WindowsRuntime]
$null = [Windows.Globalization.Language, Windows.Globalization, ContentType=WindowsRuntime]
function Await($Operation, [Type]$ResultType) {
  $method = [System.WindowsRuntimeSystemExtensions].GetMethods() |
    Where-Object { $_.Name -eq 'AsTask' -and $_.IsGenericMethod -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' } |
    Select-Object -First 1
  $task = $method.MakeGenericMethod($ResultType).Invoke($null, @($Operation))
  $task.Wait()
  $task.Result
}
$file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($env:B_CORE_OCR_INPUT_PATH)) ([Windows.Storage.StorageFile])
$stream = Await ($file.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
$decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
$bitmap = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
$language = New-Object Windows.Globalization.Language 'ko'
$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage($language)
if ($null -eq $engine) { throw 'KOREAN_OCR_LANGUAGE_NOT_INSTALLED' }
$result = Await ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
[Console]::Out.Write($result.Text)
"#;
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            SCRIPT,
        ])
        .env("B_CORE_OCR_INPUT_PATH", path)
        .output();
    match output {
        Ok(output) if output.status.success() => String::from_utf8(output.stdout)
            .map(|text| (text, "WINDOWS_MEDIA_OCR_KO".to_string()))
            .map_err(|_| (EvidenceStatusIR::ReadFailed, "WINDOWS_OCR_OUTPUT_NOT_UTF8")),
        Ok(_) => Err((EvidenceStatusIR::ReadFailed, "WINDOWS_OCR_FAILED")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err((
            EvidenceStatusIR::ExtractorUnavailable,
            "WINDOWS_OCR_POWERSHELL_UNAVAILABLE",
        )),
        Err(_) => Err((EvidenceStatusIR::ReadFailed, "WINDOWS_OCR_START_FAILED")),
    }
}

#[cfg(not(target_os = "windows"))]
fn extract_windows_ocr(_path: &Path) -> Result<(String, String), (EvidenceStatusIR, &'static str)> {
    Err((
        EvidenceStatusIR::ExtractorUnavailable,
        "LOCAL_IMAGE_OCR_NOT_INSTALLED",
    ))
}

fn extracted_failure(
    input: &EvidenceInputIR,
    kind: EvidenceKindIR,
    status: EvidenceStatusIR,
    source_sha256: String,
    code: &str,
) -> ExtractedEvidence {
    ExtractedEvidence {
        receipt: EvidenceExtractionReceiptIR {
            evidence_id: input.evidence_id.clone(),
            path: input.path.clone(),
            detected_kind: kind,
            status,
            source_sha256,
            extracted_text_sha256: String::new(),
            extracted_characters: 0,
            structured_block_count: 0,
            section_or_page_count: 0,
            table_count: 0,
            mean_ocr_confidence_millis: None,
            structure_sha256: String::new(),
            extractor: "NONE".to_string(),
            diagnostic_code: code.to_string(),
        },
        text: String::new(),
        structured: empty_structured_evidence(&input.evidence_id),
    }
}

fn empty_structured_evidence(evidence_id: &str) -> StructuredEvidenceIR {
    StructuredEvidenceIR {
        schema: "B_CORE_STRUCTURED_EVIDENCE_1".to_string(),
        evidence_id: evidence_id.to_string(),
        blocks: Vec::new(),
        structure_sha256: String::new(),
    }
}

fn structure_extracted_text(
    evidence_id: &str,
    _kind: EvidenceKindIR,
    text: &str,
) -> StructuredEvidenceIR {
    let mut blocks = Vec::new();
    let mut section_or_page = 1_usize;
    let mut table_lines = Vec::new();
    let mut table_start = 0_usize;
    let flush_table = |blocks: &mut Vec<EvidenceBlockIR>,
                       table_lines: &mut Vec<String>,
                       section_or_page: usize,
                       table_start: usize| {
        if table_lines.is_empty() {
            return;
        }
        let text = table_lines.join("\n");
        push_evidence_block(
            blocks,
            evidence_id,
            EvidenceBlockKindIR::Table,
            section_or_page,
            table_start,
            text,
            None,
            None,
        );
        table_lines.clear();
    };
    for (line_index, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if let Some(marker) = line
            .strip_prefix("[[B_CORE_SECTION:")
            .and_then(|value| value.strip_suffix("]]"))
        {
            flush_table(&mut blocks, &mut table_lines, section_or_page, table_start);
            section_or_page = marker.parse::<usize>().unwrap_or(section_or_page);
            continue;
        }
        if line == "\u{c}" {
            flush_table(&mut blocks, &mut table_lines, section_or_page, table_start);
            section_or_page += 1;
            continue;
        }
        if line.starts_with('|') && line.ends_with('|') {
            if table_lines.is_empty() {
                table_start = line_index + 1;
            }
            table_lines.push(line.to_string());
            continue;
        }
        flush_table(&mut blocks, &mut table_lines, section_or_page, table_start);
        if line.is_empty() {
            continue;
        }
        let (kind, content, confidence, geometry) = if let Some((metadata, content)) = line
            .strip_prefix("[[B_CORE_OCR|")
            .and_then(|value| value.split_once("|]]"))
        {
            let (confidence_text, geometry) = metadata.split_once('|').unwrap_or((metadata, ""));
            let confidence = Some(confidence_text)
                .and_then(|value| value.parse::<f64>().ok())
                .map(|value| (value.clamp(0.0, 1.0) * 1000.0).round() as u16);
            (
                EvidenceBlockKindIR::OcrLine,
                content.trim(),
                confidence,
                (!geometry.is_empty()).then(|| geometry.to_string()),
            )
        } else if line.starts_with('#') || looks_like_heading(line) {
            (
                EvidenceBlockKindIR::Heading,
                line.trim_start_matches('#').trim(),
                None,
                None,
            )
        } else if line.starts_with(['-', '*', '•', '·']) {
            (
                EvidenceBlockKindIR::ListItem,
                line.trim_start_matches(['-', '*', '•', '·', ' ']),
                None,
                None,
            )
        } else {
            (EvidenceBlockKindIR::Paragraph, line, None, None)
        };
        push_evidence_block(
            &mut blocks,
            evidence_id,
            kind,
            section_or_page,
            line_index + 1,
            content.to_string(),
            confidence,
            geometry,
        );
    }
    flush_table(&mut blocks, &mut table_lines, section_or_page, table_start);
    let structure_sha256 = sha256_bytes(serde_json::to_vec(&blocks).unwrap_or_default().as_slice());
    StructuredEvidenceIR {
        schema: "B_CORE_STRUCTURED_EVIDENCE_1".to_string(),
        evidence_id: evidence_id.to_string(),
        blocks,
        structure_sha256,
    }
}

#[allow(clippy::too_many_arguments)]
fn push_evidence_block(
    blocks: &mut Vec<EvidenceBlockIR>,
    evidence_id: &str,
    kind: EvidenceBlockKindIR,
    section_or_page: usize,
    source_ordinal: usize,
    text: String,
    confidence_millis: Option<u16>,
    geometry: Option<String>,
) {
    if text.trim().is_empty() {
        return;
    }
    let ordinal = blocks.len() + 1;
    let mut source_location =
        format!("{evidence_id}:section_or_page:{section_or_page}:source_ordinal:{source_ordinal}");
    if let Some(value) = &geometry {
        source_location.push_str(":bbox:");
        source_location.push_str(value);
    }
    let digest =
        sha256_bytes(format!("{evidence_id}|{section_or_page}|{source_ordinal}|{text}").as_bytes());
    blocks.push(EvidenceBlockIR {
        block_id: format!("BLOCK-{}", &digest[..16]),
        kind,
        section_or_page,
        ordinal,
        text,
        source_location,
        geometry,
        confidence_millis,
    });
}

fn looks_like_heading(line: &str) -> bool {
    let compact = line.trim();
    if compact.chars().count() > 80 || compact.ends_with(['.', '。', ',', ';']) {
        return false;
    }
    compact.starts_with("제") && compact.contains(['장', '절'])
        || compact
            .split_once('.')
            .is_some_and(|(prefix, _)| prefix.chars().all(|value| value.is_ascii_digit()))
        || compact
            .split_once(' ')
            .is_some_and(|(prefix, _)| prefix.chars().all(|value| value.is_ascii_digit()))
}

fn normalize_extracted_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn resolve_profile(
    supplied: &ApartmentProfileIR,
    extracted: &[ExtractedEvidence],
) -> ApartmentProfileIR {
    let mut profile = supplied.clone();
    for evidence in extracted
        .iter()
        .filter(|evidence| evidence.receipt.status == EvidenceStatusIR::Extracted)
    {
        let mut used = false;
        if profile.complex_name.is_none() {
            let value = extract_label_value(&evidence.text, &["단지명", "공동주택명", "아파트명"]);
            used |= value.is_some();
            profile.complex_name = value;
        }
        if profile.use_approval_date.is_none() {
            let value =
                extract_label_value(&evidence.text, &["사용검사일", "사용승인일", "준공일"]);
            used |= value.is_some();
            profile.use_approval_date = value;
        }
        if profile.household_count.is_none() {
            let value = extract_integer_after_label(&evidence.text, &["세대수", "총세대수"])
                .and_then(|value| u32::try_from(value).ok());
            used |= value.is_some();
            profile.household_count = value;
        }
        if profile.total_supply_area_milli_square_meters.is_none() {
            let value = extract_decimal_milli_after_label(
                &evidence.text,
                &["총 공급면적", "총공급면적", "공급면적 합계"],
            );
            used |= value.is_some();
            profile.total_supply_area_milli_square_meters = value;
        }
        if profile.current_reserve_won.is_none() {
            let value = extract_integer_after_label(
                &evidence.text,
                &["장기수선충당금 잔액", "충당금 잔액", "기말잔액"],
            );
            used |= value.is_some();
            profile.current_reserve_won = value;
        }
        if used {
            profile
                .evidence_refs
                .push(evidence.receipt.evidence_id.clone());
        }
    }
    profile.evidence_refs.sort();
    profile.evidence_refs.dedup();
    profile
}

fn extract_label_value(text: &str, labels: &[&str]) -> Option<String> {
    for line in text.lines() {
        for label in labels {
            if let Some(index) = line.find(label) {
                let value = line[index + label.len()..]
                    .trim_start_matches([' ', '\t', ':', '：', '-', '='])
                    .trim();
                if !value.is_empty() && value.chars().count() <= 80 {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

fn extract_integer_after_label(text: &str, labels: &[&str]) -> Option<i64> {
    extract_numeric_token(text, labels).and_then(|token| {
        token
            .chars()
            .filter(char::is_ascii_digit)
            .collect::<String>()
            .parse::<i64>()
            .ok()
    })
}

fn extract_decimal_milli_after_label(text: &str, labels: &[&str]) -> Option<i64> {
    let token = extract_numeric_token(text, labels)?.replace(',', "");
    let mut parts = token.split('.');
    let whole = parts.next()?.parse::<i64>().ok()?;
    let fraction = parts
        .next()
        .unwrap_or_default()
        .chars()
        .take(3)
        .collect::<String>();
    let fraction = format!("{fraction:0<3}").parse::<i64>().ok()?;
    whole.checked_mul(1_000)?.checked_add(fraction)
}

fn extract_numeric_token<'a>(text: &'a str, labels: &[&str]) -> Option<&'a str> {
    for line in text.lines() {
        for label in labels {
            if let Some(index) = line.find(label) {
                let tail = &line[index + label.len()..];
                let start = tail.find(|character: char| character.is_ascii_digit())?;
                let token = tail[start..]
                    .split(|character: char| {
                        !(character.is_ascii_digit() || character == ',' || character == '.')
                    })
                    .next()?;
                if !token.is_empty() {
                    return Some(token);
                }
            }
        }
    }
    None
}

fn build_decisions(
    request: &LongTermRepairPlanRequestIR,
    catalog: &[StatutoryRepairItemIR],
    evidence_status: &BTreeMap<&str, EvidenceStatusIR>,
) -> Result<Vec<RepairItemDecisionIR>, LongTermRepairPlanError> {
    let rules = request
        .apartment_rules
        .iter()
        .map(|rule| ((rule.item_id.as_str(), rule.method), rule))
        .collect::<BTreeMap<_, _>>();
    let costs = request
        .cost_inputs
        .iter()
        .map(|input| ((input.item_id.as_str(), input.method), input))
        .collect::<BTreeMap<_, _>>();
    let mut decisions = Vec::new();
    for item in catalog {
        for legal in &item.methods {
            let rule = rules.get(&(item.id.as_str(), legal.method)).copied();
            let cost = costs.get(&(item.id.as_str(), legal.method)).copied();
            let (resolution, cycle, rate) = resolve_rule(legal, rule, evidence_status);
            let applicable = cost.and_then(grounded_applicability);
            let scheduled_years = match (applicable, cycle, cost) {
                (Some(true), Some(cycle), Some(input)) => input
                    .first_scheduled_year
                    .map(|first| scheduled_years(first, cycle, request))
                    .unwrap_or_default(),
                _ => Vec::new(),
            };
            let one_time_cost_won = match (cost, rate) {
                (Some(input), Some(rate)) if applicable == Some(true) => {
                    calculate_one_time_cost(input, rate)?
                }
                _ => None,
            };
            let total_plan_cost_won = match (one_time_cost_won, cost) {
                (Some(one_time), Some(input)) => {
                    let gross = i128::from(one_time)
                        .checked_mul(scheduled_years.len() as i128)
                        .ok_or(LongTermRepairPlanError::ArithmeticOverflow)?;
                    let net = gross
                        .checked_sub(i128::from(input.overlapping_partial_repair_deduction_won))
                        .ok_or(LongTermRepairPlanError::ArithmeticOverflow)?;
                    if net < 0 {
                        return Err(LongTermRepairPlanError::InvalidRequest);
                    }
                    Some(
                        i64::try_from(net)
                            .map_err(|_| LongTermRepairPlanError::ArithmeticOverflow)?,
                    )
                }
                _ => None,
            };
            let mut evidence_refs = cost
                .map(|input| input.evidence_refs.clone())
                .unwrap_or_default();
            if let Some(rule) = rule {
                if let Some(reference) = &rule.adjustment_approval_evidence_id {
                    evidence_refs.push(reference.clone());
                }
            }
            evidence_refs.sort();
            evidence_refs.dedup();
            let status_text =
                decision_status_text(applicable, resolution, one_time_cost_won, &scheduled_years);
            decisions.push(RepairItemDecisionIR {
                item_id: item.id.clone(),
                group: item.group.clone(),
                subgroup: item.subgroup.clone(),
                work_type: item.work_type.clone(),
                method: legal.method,
                legal_cycle_years: legal.cycle_years,
                legal_repair_rate_percent: legal.repair_rate_percent,
                apartment_cycle_years: rule.and_then(|rule| rule.cycle_years),
                apartment_repair_rate_percent: rule.and_then(|rule| rule.repair_rate_percent),
                applied_cycle_years: cycle,
                applied_repair_rate_percent: rate,
                resolution,
                applicable,
                scheduled_years,
                one_time_cost_won,
                total_plan_cost_won,
                evidence_refs,
                statutory_notes: item.notes.clone(),
                status_text,
            });
        }
    }
    Ok(decisions)
}

fn resolve_rule(
    legal: &StatutoryRepairMethodIR,
    apartment: Option<&ApartmentRepairRuleIR>,
    evidence_status: &BTreeMap<&str, EvidenceStatusIR>,
) -> (RuleResolutionIR, Option<u16>, Option<u16>) {
    let Some(apartment) = apartment else {
        return (
            RuleResolutionIR::StatutoryBaseline,
            Some(legal.cycle_years),
            Some(legal.repair_rate_percent),
        );
    };
    let cycle = apartment.cycle_years.unwrap_or(legal.cycle_years);
    let rate = apartment
        .repair_rate_percent
        .unwrap_or(legal.repair_rate_percent);
    if cycle == legal.cycle_years && rate == legal.repair_rate_percent {
        return (
            RuleResolutionIR::ApartmentMatchesBaseline,
            Some(cycle),
            Some(rate),
        );
    }
    let approval_is_extracted = apartment
        .adjustment_approval_evidence_id
        .as_deref()
        .and_then(|reference| evidence_status.get(reference))
        .is_some_and(|status| *status == EvidenceStatusIR::Extracted);
    if approval_is_extracted && cycle > 0 && (1..=100).contains(&rate) {
        (
            RuleResolutionIR::ApprovedApartmentAdjustment,
            Some(cycle),
            Some(rate),
        )
    } else {
        (RuleResolutionIR::ReviewRequired, None, None)
    }
}

fn scheduled_years(first: u16, cycle: u16, request: &LongTermRepairPlanRequestIR) -> Vec<u16> {
    let end = request.plan_start_year + request.plan_years - 1;
    let mut year = first;
    while year < request.plan_start_year {
        year = match year.checked_add(cycle) {
            Some(next) => next,
            None => return Vec::new(),
        };
    }
    let mut years = Vec::new();
    while year <= end {
        years.push(year);
        year = match year.checked_add(cycle) {
            Some(next) => next,
            None => break,
        };
    }
    years
}

fn calculate_one_time_cost(
    input: &CostInputIR,
    repair_rate_percent: u16,
) -> Result<Option<i64>, LongTermRepairPlanError> {
    if input.evidence_refs.is_empty() {
        return Ok(None);
    }
    let (Some(quantity), Some(unit_price), Some(overhead)) = (
        input.quantity_milli_units,
        input.unit_price_won,
        input.overhead_factor_basis_points,
    ) else {
        return Ok(None);
    };
    if quantity < 0 || unit_price < 0 || overhead == 0 || overhead > 100_000 {
        return Err(LongTermRepairPlanError::InvalidRequest);
    }
    let numerator = i128::from(quantity)
        .checked_mul(i128::from(unit_price))
        .and_then(|value| value.checked_mul(i128::from(overhead)))
        .and_then(|value| value.checked_mul(i128::from(repair_rate_percent)))
        .ok_or(LongTermRepairPlanError::ArithmeticOverflow)?;
    let denominator = 1_000_i128 * 10_000_i128 * 100_i128;
    let rounded = numerator
        .checked_add(denominator / 2)
        .ok_or(LongTermRepairPlanError::ArithmeticOverflow)?
        / denominator;
    Ok(Some(i64::try_from(rounded).map_err(|_| {
        LongTermRepairPlanError::ArithmeticOverflow
    })?))
}

fn grounded_applicability(input: &CostInputIR) -> Option<bool> {
    if input.evidence_refs.is_empty() {
        None
    } else {
        input.applicable
    }
}

fn decision_status_text(
    applicable: Option<bool>,
    resolution: RuleResolutionIR,
    one_time_cost: Option<i64>,
    years: &[u16],
) -> String {
    if applicable == Some(false) {
        return "단지 미적용(근거 확인)".to_string();
    }
    if applicable.is_none() {
        return "적용 여부 확인 필요".to_string();
    }
    if resolution == RuleResolutionIR::ReviewRequired {
        return "조정 승인 근거 확인 필요".to_string();
    }
    if one_time_cost.is_none() {
        return "수량·단가·제경비 확인 필요".to_string();
    }
    if years.is_empty() {
        return "최초 예정연도 확인 필요".to_string();
    }
    "계산 완료".to_string()
}

fn build_annual_plan(
    request: &LongTermRepairPlanRequestIR,
    decisions: &[RepairItemDecisionIR],
) -> Result<Vec<AnnualPlanIR>, LongTermRepairPlanError> {
    let end = request.plan_start_year + request.plan_years - 1;
    (request.plan_start_year..=end)
        .map(|year| {
            let scheduled = decisions
                .iter()
                .filter(|decision| decision.scheduled_years.contains(&year))
                .collect::<Vec<_>>();
            let expenditure_won = if scheduled
                .iter()
                .all(|decision| decision.one_time_cost_won.is_some())
            {
                Some(scheduled.iter().try_fold(0_i64, |total, decision| {
                    total
                        .checked_add(decision.one_time_cost_won.unwrap_or_default())
                        .ok_or(LongTermRepairPlanError::ArithmeticOverflow)
                })?)
            } else {
                None
            };
            Ok(AnnualPlanIR {
                year,
                scheduled_decision_count: scheduled.len(),
                expenditure_won,
            })
        })
        .collect()
}

fn build_reserve_projection(
    request: &LongTermRepairPlanRequestIR,
    profile: &ApartmentProfileIR,
    annual_plan: &[AnnualPlanIR],
) -> Result<Vec<ReserveYearIR>, LongTermRepairPlanError> {
    let inputs = request
        .reserve_inputs
        .iter()
        .map(|input| (input.year, input))
        .collect::<BTreeMap<_, _>>();
    let mut prior = profile.current_reserve_won;
    let mut projection = Vec::new();
    for annual in annual_plan {
        let input = inputs
            .get(&annual.year)
            .copied()
            .filter(|input| !input.evidence_refs.is_empty());
        let contribution = input.map(|value| value.contribution_won);
        let closing = match (prior, input, annual.expenditure_won) {
            (Some(opening), Some(input), Some(expenditure)) => {
                let value = i128::from(opening)
                    + i128::from(input.contribution_won)
                    + i128::from(input.disposition_transfer_won)
                    + i128::from(input.interest_won)
                    - i128::from(expenditure);
                Some(
                    i64::try_from(value)
                        .map_err(|_| LongTermRepairPlanError::ArithmeticOverflow)?,
                )
            }
            _ => None,
        };
        projection.push(ReserveYearIR {
            year: annual.year,
            opening_balance_won: prior,
            contribution_won: contribution,
            expenditure_won: annual.expenditure_won,
            closing_balance_won: closing,
        });
        prior = closing;
    }
    Ok(projection)
}

fn build_monthly_area_charges(
    profile: &ApartmentProfileIR,
    reserve: &[ReserveYearIR],
) -> Result<Vec<MonthlyAreaChargeIR>, LongTermRepairPlanError> {
    let mut charges = Vec::new();
    for year in reserve {
        for area_type in &profile.household_area_types {
            let charge = match (
                year.contribution_won,
                profile.total_supply_area_milli_square_meters,
            ) {
                (Some(contribution), Some(total_area)) if total_area > 0 => {
                    if contribution < 0
                        || area_type.household_count == 0
                        || area_type.supply_area_milli_square_meters <= 0
                    {
                        return Err(LongTermRepairPlanError::InvalidRequest);
                    }
                    let denominator = i128::from(total_area) * 12;
                    let numerator = i128::from(contribution)
                        .checked_mul(i128::from(area_type.supply_area_milli_square_meters))
                        .ok_or(LongTermRepairPlanError::ArithmeticOverflow)?;
                    let rounded = numerator
                        .checked_add(denominator / 2)
                        .ok_or(LongTermRepairPlanError::ArithmeticOverflow)?
                        / denominator;
                    Some(
                        i64::try_from(rounded)
                            .map_err(|_| LongTermRepairPlanError::ArithmeticOverflow)?,
                    )
                }
                _ => None,
            };
            charges.push(MonthlyAreaChargeIR {
                year: year.year,
                area_type_label: area_type.label.clone(),
                household_count: area_type.household_count,
                monthly_charge_per_household_won: charge,
                evidence_refs: area_type.evidence_refs.clone(),
            });
        }
    }
    Ok(charges)
}

fn source_conflicts() -> Vec<SourceConflictIR> {
    vec![
        SourceConflictIR {
            conflict_id: "ITEM-COUNT-73-VS-69".to_string(),
            older_or_secondary_claim: "공식 안내 웹페이지의 73개 항목 표시는 개정 이력상 구기준일 수 있음"
                .to_string(),
            controlling_treatment: "2026-07-30 시행규칙 별표 1 스냅샷의 69개 공사종별을 기준으로 대사하고 생성일에 원문 재확인"
                .to_string(),
            source_refs: vec![
                "MOLIT-LH-WEB-REPAIR-GUIDE".to_string(),
                "LAW-GO-KR-ENFORCEMENT-RULE-2026-07-30-ANNEX-1".to_string(),
            ],
        },
        SourceConflictIR {
            conflict_id: "UNVERIFIED-MIN-MAX-OVERRIDE".to_string(),
            older_or_secondary_claim: "규약 주기는 무조건 더 짧게, 수선율은 무조건 더 높게 자동 보정"
                .to_string(),
            controlling_treatment: "법적 근거와 승인 증빙이 없는 차이는 자동 채택하지 않고 REVIEW_REQUIRED로 분리"
                .to_string(),
            source_refs: vec![
                "USER-SUPPLIED-REPAIR-STANDARD-RESOLVER".to_string(),
                "LAW-GO-KR-ENFORCEMENT-RULE-2026-07-30-ANNEX-1".to_string(),
            ],
        },
    ]
}

fn missing_required_inputs(
    request: &LongTermRepairPlanRequestIR,
    profile: &ApartmentProfileIR,
    receipts: &[EvidenceExtractionReceiptIR],
    decisions: &[RepairItemDecisionIR],
) -> Vec<String> {
    let mut missing = Vec::new();
    if profile.complex_name.is_none() {
        missing.push("단지명".to_string());
    }
    if profile.use_approval_date.is_none() {
        missing.push("사용검사일 또는 사용승인일".to_string());
    }
    if profile.household_count.is_none() {
        missing.push("총 세대수".to_string());
    }
    if profile.total_supply_area_milli_square_meters.is_none() {
        missing.push("총 공급면적".to_string());
    }
    if profile.household_area_types.is_empty() {
        missing.push("면적형별 공급면적과 세대수".to_string());
    } else if let Some(total_area) = profile.total_supply_area_milli_square_meters {
        let area_type_total =
            profile
                .household_area_types
                .iter()
                .try_fold(0_i128, |total, area_type| {
                    total.checked_add(
                        i128::from(area_type.supply_area_milli_square_meters)
                            * i128::from(area_type.household_count),
                    )
                });
        if area_type_total != Some(i128::from(total_area)) {
            missing.push("총 공급면적과 면적형별 공급면적 합계 대사".to_string());
        }
    }
    if profile.current_reserve_won.is_none() {
        missing.push("현재 장기수선충당금 잔액".to_string());
    }
    if profile.evidence_refs.is_empty()
        && (profile.complex_name.is_some()
            || profile.use_approval_date.is_some()
            || profile.household_count.is_some()
            || profile.total_supply_area_milli_square_meters.is_some()
            || profile.current_reserve_won.is_some())
    {
        missing.push("단지 기본정보의 근거자료".to_string());
    }
    if profile
        .household_area_types
        .iter()
        .any(|area_type| area_type.evidence_refs.is_empty())
    {
        missing.push("면적형별 공급면적·세대수의 근거자료".to_string());
    }
    if request.reserve_inputs.len() != usize::from(request.plan_years) {
        missing.push("40년 연도별 충당금 적립·이자·이월 입력".to_string());
    }
    if receipts
        .iter()
        .any(|receipt| receipt.status != EvidenceStatusIR::Extracted)
    {
        missing.push("읽지 못한 입력문서의 텍스트 또는 OCR 결과".to_string());
    }
    if decisions
        .iter()
        .any(|decision| decision.applicable.is_none())
    {
        missing.push("69개 공사종별의 단지 적용 여부".to_string());
    }
    if decisions
        .iter()
        .any(|decision| decision.applicable == Some(true) && decision.one_time_cost_won.is_none())
    {
        missing.push("적용 항목의 수량·단가·제경비 근거".to_string());
    }
    if decisions
        .iter()
        .any(|decision| decision.applicable == Some(true) && decision.scheduled_years.is_empty())
    {
        missing.push("적용 항목의 최초 예정연도".to_string());
    }
    if decisions
        .iter()
        .any(|decision| decision.resolution == RuleResolutionIR::ReviewRequired)
    {
        missing.push("법정 기준과 다른 단지 기준의 조정·승인 증빙".to_string());
    }
    missing.sort();
    missing.dedup();
    missing
}

struct SectionSpec {
    id: &'static str,
    title: &'static str,
    pages: usize,
}

const SECTIONS: &[SectionSpec] = &[
    SectionSpec {
        id: "front-matter",
        title: "표지·승인·목차",
        pages: 3,
    },
    SectionSpec {
        id: "executive-review",
        title: "종합 검토의견",
        pages: 3,
    },
    SectionSpec {
        id: "basis",
        title: "목적·작성기준·절차",
        pages: 4,
    },
    SectionSpec {
        id: "complex-profile",
        title: "단지 및 시설 현황",
        pages: 3,
    },
    SectionSpec {
        id: "diagnosis",
        title: "기존 계획·공사·충당금 진단",
        pages: 4,
    },
    SectionSpec {
        id: "item-review",
        title: "69개 공사종별 검토",
        pages: 10,
    },
    SectionSpec {
        id: "yearly-schedule",
        title: "40년 연도별 수선 실시계획",
        pages: 5,
    },
    SectionSpec {
        id: "cost-detail",
        title: "공사종별 상세 산출표",
        pages: 6,
    },
    SectionSpec {
        id: "reserve-simulation",
        title: "충당금 적립·부과 시뮬레이션",
        pages: 5,
    },
    SectionSpec {
        id: "execution-forms",
        title: "집행·결산·공개 서식",
        pages: 3,
    },
    SectionSpec {
        id: "evidence",
        title: "핵심 물량·사진·견적 부록",
        pages: 4,
    },
];

#[allow(clippy::too_many_arguments)]
fn assemble_pages(
    request: &LongTermRepairPlanRequestIR,
    profile: &ApartmentProfileIR,
    receipts: &[EvidenceExtractionReceiptIR],
    decisions: &[RepairItemDecisionIR],
    annual: &[AnnualPlanIR],
    reserve: &[ReserveYearIR],
    monthly_charges: &[MonthlyAreaChargeIR],
    conflicts: &[SourceConflictIR],
    missing: &[String],
) -> Vec<ReportPageIR> {
    let mut pages = Vec::with_capacity(REQUIRED_PAGE_COUNT);
    for section in SECTIONS {
        for local_page in 0..section.pages {
            let page_number = pages.len() + 1;
            let blocks = section_blocks(
                section.id,
                local_page,
                request,
                profile,
                receipts,
                decisions,
                annual,
                reserve,
                monthly_charges,
                conflicts,
                missing,
            );
            pages.push(ReportPageIR {
                page_number,
                section_id: section.id.to_string(),
                section_title: section.title.to_string(),
                page_title: format!("{} · {}/{}", section.title, local_page + 1, section.pages),
                blocks,
            });
        }
    }
    pages
}

#[allow(clippy::too_many_arguments)]
fn section_blocks(
    section: &str,
    local_page: usize,
    request: &LongTermRepairPlanRequestIR,
    profile: &ApartmentProfileIR,
    receipts: &[EvidenceExtractionReceiptIR],
    decisions: &[RepairItemDecisionIR],
    annual: &[AnnualPlanIR],
    reserve: &[ReserveYearIR],
    monthly_charges: &[MonthlyAreaChargeIR],
    conflicts: &[SourceConflictIR],
    missing: &[String],
) -> Vec<ReportBlockIR> {
    match section {
        "front-matter" => front_blocks(local_page, request, profile),
        "executive-review" => executive_blocks(local_page, decisions, missing),
        "basis" => basis_blocks(local_page, conflicts),
        "complex-profile" => profile_blocks(local_page, profile),
        "diagnosis" => diagnosis_blocks(local_page, receipts, decisions, missing),
        "item-review" => item_blocks(local_page, decisions),
        "yearly-schedule" => annual_blocks(local_page, annual),
        "cost-detail" => cost_blocks(local_page, decisions),
        "reserve-simulation" => reserve_blocks(local_page, reserve, monthly_charges, profile),
        "execution-forms" => execution_blocks(local_page),
        "evidence" => evidence_blocks(local_page, receipts, missing),
        _ => Vec::new(),
    }
}

fn block(
    heading: impl Into<String>,
    body: impl Into<String>,
    status: ReportBlockStatusIR,
    evidence_refs: Vec<String>,
) -> ReportBlockIR {
    ReportBlockIR {
        heading: heading.into(),
        body: body.into(),
        status,
        evidence_refs,
    }
}

fn front_blocks(
    page: usize,
    request: &LongTermRepairPlanRequestIR,
    profile: &ApartmentProfileIR,
) -> Vec<ReportBlockIR> {
    match page {
        0 => vec![block(
            profile.complex_name.clone().unwrap_or_else(|| "단지명 확인 필요".to_string()),
            format!("장기수선계획서 · 계획기간 {}~{} · 기준일 {}", request.plan_start_year, request.plan_start_year + 39, request.as_of_date),
            if profile.complex_name.is_some() { ReportBlockStatusIR::Verified } else { ReportBlockStatusIR::NeedsConfirmation },
            profile.evidence_refs.clone(),
        )],
        1 => vec![block("작성·검토·승인", "작성자, 기술검토자, 관리주체, 입주자대표회의 의결일과 버전을 기록한다. AI 산출물은 승인이나 의결을 대체하지 않는다.", ReportBlockStatusIR::Advisory, vec!["WORKFLOW-APPROVAL-CONTRACT".to_string()])],
        _ => vec![block("전체 목차", SECTIONS.iter().map(|section| format!("{} ({}쪽)", section.title, section.pages)).collect::<Vec<_>>().join("\n"), ReportBlockStatusIR::Computed, vec!["FIXED-50-PAGE-SCHEMA".to_string()])],
    }
}

fn executive_blocks(
    page: usize,
    decisions: &[RepairItemDecisionIR],
    missing: &[String],
) -> Vec<ReportBlockIR> {
    let completed = decisions
        .iter()
        .filter(|decision| decision.status_text == "계산 완료")
        .count();
    let review = decisions
        .iter()
        .filter(|decision| decision.resolution == RuleResolutionIR::ReviewRequired)
        .count();
    let content = match page {
        0 => format!("법정 69개 공사종별, {}개 수선방법을 대사했다. 계산 완료 {}건, 기준 검토 필요 {}건이다.", decisions.len(), completed, review),
        1 => format!("부족자료: {}", if missing.is_empty() { "없음".to_string() } else { missing.join(" · ") }),
        _ => "우선공사는 안전·법정검사·고장 이력·상태등급·자금흐름이 모두 근거로 연결된 뒤 확정한다. 비용이 없는 항목을 임의로 우선순위화하지 않는다.".to_string(),
    };
    vec![block(
        "핵심 진단",
        content,
        if missing.is_empty() {
            ReportBlockStatusIR::Computed
        } else {
            ReportBlockStatusIR::NeedsConfirmation
        },
        vec!["DECISION-RECONCILIATION".to_string()],
    )]
}

fn basis_blocks(page: usize, conflicts: &[SourceConflictIR]) -> Vec<ReportBlockIR> {
    let (heading, body, refs) = match page {
        0 => ("계획 목적", "공동주택 주요시설의 교체·보수를 장기간 계획하고 그 비용과 충당금 흐름을 증거 기반으로 연결한다.".to_string(), vec!["LAW-GO-KR-COMMON-HOUSING-MANAGEMENT-ACT".to_string()]),
        1 => ("작성·조정 절차", "정기 검토는 36개월 주기를 확인하고, 수시조정은 현행 법령상 의결·동의 요건과 기존 정기검토 기준일을 별도로 검토한다.".to_string(), vec!["MOLIT-LH-FAQ-THREE-YEAR-REVIEW".to_string()]),
        2 => ("근거 우선순위", "현행 법령 원문 → 공식 유권해석·가이드 → 승인된 단지 자료 → 현장 증빙 → 과거 샘플 구조 순으로 적용한다. 오래된 샘플 수치를 복사하지 않는다.".to_string(), vec!["SOURCE-AUTHORITY-LATTICE".to_string()]),
        _ => ("출처 충돌", conflicts.iter().map(|conflict| format!("{}: {}", conflict.conflict_id, conflict.controlling_treatment)).collect::<Vec<_>>().join("\n"), conflicts.iter().flat_map(|conflict| conflict.source_refs.clone()).collect()),
    };
    vec![block(
        heading,
        body,
        if page == 3 {
            ReportBlockStatusIR::NeedsConfirmation
        } else {
            ReportBlockStatusIR::Verified
        },
        refs,
    )]
}

fn profile_blocks(page: usize, profile: &ApartmentProfileIR) -> Vec<ReportBlockIR> {
    let value = match page {
        0 => format!("단지명: {}\n사용승인·검사일: {}\n세대수: {}", option_text(profile.complex_name.as_deref()), option_text(profile.use_approval_date.as_deref()), profile.household_count.map(|value| value.to_string()).unwrap_or_else(|| "확인 필요".to_string())),
        1 => format!("총 공급면적: {} ㎡", profile.total_supply_area_milli_square_meters.map(format_milli).unwrap_or_else(|| "확인 필요".to_string())),
        _ => "난방방식, 승강기, 기계식주차장, 소방·피난, 홈네트워크, 전기차충전기, 복리시설의 존재·수량·상태를 시설대장과 현장사진으로 대사한다.".to_string(),
    };
    vec![block(
        "단지·시설 개요",
        value,
        if profile.evidence_refs.is_empty() {
            ReportBlockStatusIR::NeedsConfirmation
        } else {
            ReportBlockStatusIR::Verified
        },
        profile.evidence_refs.clone(),
    )]
}

fn diagnosis_blocks(
    page: usize,
    receipts: &[EvidenceExtractionReceiptIR],
    decisions: &[RepairItemDecisionIR],
    missing: &[String],
) -> Vec<ReportBlockIR> {
    let body = match page {
        0 => format!("입력 {}건 중 텍스트 추출 성공 {}건. 실패한 자료는 추정하지 않고 OCR/변환 요청으로 남긴다.", receipts.len(), receipts.iter().filter(|receipt| receipt.status == EvidenceStatusIR::Extracted).count()),
        1 => format!("적용 여부 확인 완료 {}건 / 전체 수선방법 {}건", decisions.iter().filter(|decision| decision.applicable.is_some()).count(), decisions.len()),
        2 => "기존 공사·고장·안전점검의 일자, 범위, 금액, 사진을 항목 ID와 교차 연결해야 상태기반 조정이 가능하다.".to_string(),
        _ => format!("감사 전 확인 목록: {}", if missing.is_empty() { "입력상 누락 없음(전문가 확인 필요)".to_string() } else { missing.join(" · ") }),
    };
    vec![block(
        "진단",
        body,
        if missing.is_empty() {
            ReportBlockStatusIR::Computed
        } else {
            ReportBlockStatusIR::NeedsConfirmation
        },
        vec!["EVIDENCE-AND-DECISION-AUDIT".to_string()],
    )]
}

fn item_blocks(page: usize, decisions: &[RepairItemDecisionIR]) -> Vec<ReportBlockIR> {
    let item_ids = decisions
        .iter()
        .map(|decision| decision.item_id.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let start = page * 7;
    let selected = item_ids
        .iter()
        .skip(start)
        .take(7)
        .copied()
        .collect::<BTreeSet<_>>();
    let body = decisions
        .iter()
        .filter(|decision| selected.contains(decision.item_id.as_str()))
        .map(|decision| {
            format!(
                "{} | {} | {} | 법정 {}년/{}% | 적용 {} | {}{}",
                decision.item_id,
                decision.work_type,
                decision.method.korean_label(),
                decision.legal_cycle_years,
                decision.legal_repair_rate_percent,
                match (
                    decision.applied_cycle_years,
                    decision.applied_repair_rate_percent
                ) {
                    (Some(cycle), Some(rate)) => format!("{}년/{}%", cycle, rate),
                    _ => "확인 필요".to_string(),
                },
                decision.status_text,
                if decision.statutory_notes.is_empty() {
                    String::new()
                } else {
                    format!(" | 주의: {}", decision.statutory_notes.join("; "))
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    vec![block(
        format!("항목 검토 {}", page + 1),
        body,
        if selected.iter().all(|id| {
            decisions
                .iter()
                .filter(|decision| decision.item_id == *id)
                .all(|decision| decision.applicable.is_some())
        }) {
            ReportBlockStatusIR::Computed
        } else {
            ReportBlockStatusIR::NeedsConfirmation
        },
        selected.into_iter().map(str::to_string).collect(),
    )]
}

fn annual_blocks(page: usize, annual: &[AnnualPlanIR]) -> Vec<ReportBlockIR> {
    let rows = annual
        .iter()
        .skip(page * 8)
        .take(8)
        .map(|year| {
            format!(
                "{}년 | 예정 수선 {}건 | {}",
                year.year,
                year.scheduled_decision_count,
                year.expenditure_won
                    .map(format_won)
                    .unwrap_or_else(|| "금액 확인 필요".to_string())
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    vec![block(
        format!(
            "{}개 연도 구간",
            annual.iter().skip(page * 8).take(8).count()
        ),
        rows,
        if annual
            .iter()
            .skip(page * 8)
            .take(8)
            .all(|year| year.expenditure_won.is_some())
        {
            ReportBlockStatusIR::Computed
        } else {
            ReportBlockStatusIR::NeedsConfirmation
        },
        vec!["ACTUAL-SCHEDULED-YEAR-ENGINE".to_string()],
    )]
}

fn cost_blocks(page: usize, decisions: &[RepairItemDecisionIR]) -> Vec<ReportBlockIR> {
    let groups = decisions
        .iter()
        .map(|decision| decision.group.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let selected_groups = if page == 5 {
        groups.iter().skip(5).copied().collect::<Vec<_>>()
    } else {
        groups.get(page).copied().into_iter().collect::<Vec<_>>()
    };
    let selected = decisions
        .iter()
        .filter(|decision| selected_groups.contains(&decision.group.as_str()))
        .collect::<Vec<_>>();
    let body = selected
        .iter()
        .map(|decision| {
            format!(
                "{} {} {} | 1회 {} | 40년 {}",
                decision.item_id,
                decision.work_type,
                decision.method.korean_label(),
                decision
                    .one_time_cost_won
                    .map(format_won)
                    .unwrap_or_else(|| "확인 필요".to_string()),
                decision
                    .total_plan_cost_won
                    .map(format_won)
                    .unwrap_or_else(|| "확인 필요".to_string())
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    vec![block(
        selected_groups.join(" · "),
        body,
        if selected.iter().all(|decision| {
            decision.applicable == Some(false) || decision.total_plan_cost_won.is_some()
        }) {
            ReportBlockStatusIR::Computed
        } else {
            ReportBlockStatusIR::NeedsConfirmation
        },
        vec!["FIXED-POINT-COST-ENGINE".to_string()],
    )]
}

fn reserve_blocks(
    page: usize,
    reserve: &[ReserveYearIR],
    monthly_charges: &[MonthlyAreaChargeIR],
    profile: &ApartmentProfileIR,
) -> Vec<ReportBlockIR> {
    let rows = reserve
        .iter()
        .skip(page * 8)
        .take(8)
        .map(|year| {
            format!(
                "{}년 | 기초 {} | 적립 {} | 지출 {} | 기말 {}",
                year.year,
                year.opening_balance_won
                    .map(format_won)
                    .unwrap_or_else(|| "확인 필요".to_string()),
                year.contribution_won
                    .map(format_won)
                    .unwrap_or_else(|| "확인 필요".to_string()),
                year.expenditure_won
                    .map(format_won)
                    .unwrap_or_else(|| "확인 필요".to_string()),
                year.closing_balance_won
                    .map(format_won)
                    .unwrap_or_else(|| "확인 필요".to_string())
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let visible_years = reserve
        .iter()
        .skip(page * 8)
        .take(8)
        .map(|year| year.year)
        .collect::<BTreeSet<_>>();
    let charges = monthly_charges
        .iter()
        .filter(|charge| visible_years.contains(&charge.year))
        .map(|charge| {
            format!(
                "{}년 {} | 세대당 월 {}",
                charge.year,
                charge.area_type_label,
                charge
                    .monthly_charge_per_household_won
                    .map(format_won)
                    .unwrap_or_else(|| "확인 필요".to_string())
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    vec![
        block(
            "연도별 충당금 흐름",
            rows,
            if reserve
                .iter()
                .skip(page * 8)
                .take(8)
                .all(|year| year.closing_balance_won.is_some())
            {
                ReportBlockStatusIR::Computed
            } else {
                ReportBlockStatusIR::NeedsConfirmation
            },
            profile.evidence_refs.clone(),
        ),
        block(
            "면적형별 월 부과액",
            if charges.is_empty() {
                "면적형별 공급면적과 세대수 확인 필요".to_string()
            } else {
                charges
            },
            if monthly_charges.is_empty()
                || monthly_charges
                    .iter()
                    .filter(|charge| visible_years.contains(&charge.year))
                    .any(|charge| charge.monthly_charge_per_household_won.is_none())
            {
                ReportBlockStatusIR::NeedsConfirmation
            } else {
                ReportBlockStatusIR::Computed
            },
            profile.evidence_refs.clone(),
        ),
    ]
}

fn execution_blocks(page: usize) -> Vec<ReportBlockIR> {
    let (heading, body) = match page {
        0 => ("사용계획서", "공사명·대상 위치·설계도면·기간·방법·범위·예정금액·발주절차를 항목 ID와 연결한다."),
        1 => ("의결·발주·검수", "입주자대표회의 의결, 입찰·계약, 착공, 중간검사, 준공검사, 하자보증의 날짜와 증빙을 기록한다."),
        _ => ("결산·공개", "계획금액·계약금액·집행금액 차이와 충당금 원장을 대사하고 공개 대상·기한·매체를 체크한다."),
    };
    vec![block(
        heading,
        body,
        ReportBlockStatusIR::Advisory,
        vec!["EXECUTION-EVIDENCE-CONTRACT".to_string()],
    )]
}

fn evidence_blocks(
    page: usize,
    receipts: &[EvidenceExtractionReceiptIR],
    missing: &[String],
) -> Vec<ReportBlockIR> {
    let body = match page {
        0 => receipts
            .iter()
            .map(|receipt| {
                format!(
                    "{} | {:?} | {}자 | {}",
                    receipt.evidence_id,
                    receipt.status,
                    receipt.extracted_characters,
                    receipt.source_sha256
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        1 => "대표사진마다 시설, 위치, 촬영일, 전·중·후 유형, 연결된 공사종별 ID를 기록한다."
            .to_string(),
        2 => "도면·물량산출서·견적·가격정보의 판·쪽·셀 위치를 수량 및 단가 필드에 연결한다."
            .to_string(),
        _ => format!(
            "최종 보완 목록: {}",
            if missing.is_empty() {
                "없음".to_string()
            } else {
                missing.join(" · ")
            }
        ),
    };
    vec![block(
        "증빙 인덱스",
        body,
        if page == 0
            && receipts
                .iter()
                .all(|receipt| receipt.status == EvidenceStatusIR::Extracted)
        {
            ReportBlockStatusIR::Verified
        } else if page == 3 && missing.is_empty() {
            ReportBlockStatusIR::Computed
        } else {
            ReportBlockStatusIR::NeedsConfirmation
        },
        receipts
            .iter()
            .map(|receipt| receipt.evidence_id.clone())
            .collect(),
    )]
}

fn deliberate(
    core: &DockableCore,
    request: &LongTermRepairPlanRequestIR,
    parent: &str,
    receipts: &[EvidenceExtractionReceiptIR],
    decisions: &[RepairItemDecisionIR],
    pages: &[ReportPageIR],
    readiness: PlanReadinessIR,
) -> Result<SwarmDeliberationIR, LongTermRepairPlanError> {
    let extracted = receipts
        .iter()
        .filter(|receipt| receipt.status == EvidenceStatusIR::Extracted)
        .count();
    let calculated = decisions
        .iter()
        .filter(|decision| decision.status_text == "계산 완료")
        .count();
    let facts = vec![
        deliberation_fact(
            "REPAIR-REQUIREMENT-COVERAGE",
            QualityCriterionIR::RequirementCoverage,
            if pages.len() == 50 {
                AssessmentVerdictIR::Pass
            } else {
                AssessmentVerdictIR::Fail
            },
            "FIXED_50_PAGE_AND_69_ITEM_CONTRACT",
            vec![format!("pages:{}", pages.len()), "catalog:69".to_string()],
        ),
        deliberation_fact(
            "REPAIR-EVIDENCE-INTEGRITY",
            QualityCriterionIR::EvidenceIntegrity,
            if extracted == receipts.len() {
                AssessmentVerdictIR::Pass
            } else {
                AssessmentVerdictIR::Warning
            },
            "FILE_HASH_EXTRACTION_AND_MISSING_EVIDENCE_ARE_EXPLICIT",
            vec![format!("extracted:{extracted}/{}", receipts.len())],
        ),
        deliberation_fact(
            "REPAIR-STRUCTURE-INTEGRITY",
            QualityCriterionIR::StructureIntegrity,
            AssessmentVerdictIR::Pass,
            "PAGE_SECTION_BUDGET_RECONCILED",
            vec!["section-pages:3+3+4+3+4+10+5+6+5+3+4".to_string()],
        ),
        deliberation_fact(
            "REPAIR-AUDIENCE-USABILITY",
            QualityCriterionIR::AudienceUsability,
            AssessmentVerdictIR::Pass,
            "A4_PROFESSIONAL_DRAFT_WITH_CONFIRMATION_MARKERS",
            vec!["render:A4-html".to_string()],
        ),
        deliberation_fact(
            "REPAIR-QUANTITATIVE-INTEGRITY",
            QualityCriterionIR::QuantitativeIntegrity,
            if readiness == PlanReadinessIR::ProfessionalReviewReady {
                AssessmentVerdictIR::Pass
            } else {
                AssessmentVerdictIR::Warning
            },
            "FIXED_POINT_CALCULATION_NEVER_INVENTS_MISSING_VALUES",
            vec![format!("calculated:{calculated}/{}", decisions.len())],
        ),
        deliberation_fact(
            "REPAIR-CONTRADICTION-RESISTANCE",
            QualityCriterionIR::ContradictionResistance,
            AssessmentVerdictIR::Pass,
            "SOURCE_CONFLICTS_SURFACED_AND_UNVERIFIED_MIN_MAX_OVERRIDE_DISABLED",
            vec![
                "conflicts:ITEM-COUNT-73-VS-69".to_string(),
                "policy:UNVERIFIED-MIN-MAX-OVERRIDE".to_string(),
            ],
        ),
    ];
    core.deliberate(&SwarmDeliberationRequestIR {
        schema: SWARM_DELIBERATION_REQUEST_SCHEMA.to_string(),
        request_id: format!("{}-LONG-TERM-REPAIR-REVIEW", request.request_id),
        subject: "장기수선계획 50쪽 증거·계산·구조 검토".to_string(),
        parent_reasoning_sha256: parent.to_string(),
        facts,
        max_workers: 6,
        max_rounds: 2,
    })
    .map_err(|_| LongTermRepairPlanError::Deliberation)
}

fn deliberation_fact(
    id: &str,
    criterion: QualityCriterionIR,
    verdict: AssessmentVerdictIR,
    rationale: &str,
    evidence_refs: Vec<String>,
) -> DeliberationFactIR {
    DeliberationFactIR {
        fact_id: id.to_string(),
        criterion,
        verdict,
        rationale_code: rationale.to_string(),
        evidence_refs,
    }
}

fn write_html(
    path: &Path,
    request: &LongTermRepairPlanRequestIR,
    pages: &[ReportPageIR],
    readiness: PlanReadinessIR,
    artifact_sha256: &str,
) -> Result<LongTermRepairFileReceiptIR, LongTermRepairPlanError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| LongTermRepairPlanError::OutputWrite)?;
    }
    let html = render_html(request, pages, readiness, artifact_sha256);
    fs::write(path, html.as_bytes()).map_err(|_| LongTermRepairPlanError::OutputWrite)?;
    let metadata = fs::metadata(path).map_err(|_| LongTermRepairPlanError::OutputWrite)?;
    Ok(LongTermRepairFileReceiptIR {
        path: path.to_string_lossy().to_string(),
        bytes: metadata.len(),
        sha256: sha256_bytes(html.as_bytes()),
        a4_page_count: pages.len(),
    })
}

fn render_html(
    request: &LongTermRepairPlanRequestIR,
    pages: &[ReportPageIR],
    readiness: PlanReadinessIR,
    artifact_sha256: &str,
) -> String {
    let mut body = String::new();
    for page in pages {
        body.push_str(&format!("<section class=\"a4-page\"><header><span>{}</span><span>{:02}</span></header><h1>{}</h1>", escape_html(&page.section_title), page.page_number, escape_html(&page.page_title)));
        for block in &page.blocks {
            body.push_str(&format!("<article class=\"status-{}\"><h2>{}</h2><div>{}</div><small>근거: {}</small></article>", status_class(block.status), escape_html(&block.heading), escape_html(&block.body).replace('\n', "<br>"), escape_html(&block.evidence_refs.join(" · "))));
        }
        body.push_str(&format!(
            "<footer>B_CORE · {} · {} / 50</footer></section>",
            escape_html(&request.as_of_date),
            page.page_number
        ));
    }
    format!(
        r#"<!doctype html><html lang="ko"><head><meta charset="utf-8"><title>장기수선계획서</title><style>@page{{size:A4;margin:0}}*{{box-sizing:border-box}}body{{margin:0;background:#d8dde2;color:#17202a;font-family:"Noto Sans KR","Malgun Gothic",sans-serif}}.a4-page{{position:relative;width:210mm;height:297mm;margin:10mm auto;padding:17mm 18mm 16mm;background:white;overflow:hidden;page-break-after:always;box-shadow:0 2mm 8mm #0002}}header,footer{{display:flex;justify-content:space-between;color:#65717c;font-size:9pt;border-bottom:1px solid #bcc5cc;padding-bottom:3mm}}footer{{position:absolute;left:18mm;right:18mm;bottom:10mm;border:0;border-top:1px solid #d6dce1;padding-top:3mm}}h1{{font-size:18pt;margin:8mm 0 6mm;color:#173d5c}}article{{border-left:3px solid #5b7890;background:#f5f7f8;padding:4mm 5mm;margin:0 0 5mm}}article.status-needs{{border-color:#b97820;background:#fff9ec}}article.status-verified{{border-color:#278260}}h2{{font-size:11pt;margin:0 0 2.5mm}}article div{{white-space:normal;font-size:9.5pt;line-height:1.62}}small{{display:block;margin-top:3mm;color:#6f7982;font-size:7.5pt;word-break:break-all}}@media print{{body{{background:white}}.a4-page{{margin:0;box-shadow:none}}}}</style></head><body><div hidden data-readiness="{:?}" data-artifact-sha256="{}"></div>{}</body></html>"#,
        readiness, artifact_sha256, body
    )
}

fn status_class(status: ReportBlockStatusIR) -> &'static str {
    match status {
        ReportBlockStatusIR::Verified => "verified",
        ReportBlockStatusIR::Computed => "computed",
        ReportBlockStatusIR::Advisory => "advisory",
        ReportBlockStatusIR::NeedsConfirmation => "needs",
    }
}

fn option_text(value: Option<&str>) -> String {
    value.unwrap_or("확인 필요").to_string()
}

fn format_milli(value: i64) -> String {
    format!("{}.{:03}", value / 1_000, value.abs() % 1_000)
}

fn format_won(value: i64) -> String {
    let negative = value < 0;
    let digits = value.unsigned_abs().to_string();
    let mut output = String::new();
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            output.push(',');
        }
        output.push(character);
    }
    format!("{}{}원", if negative { "-" } else { "" }, output)
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sha256_json<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    sha256_bytes(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn request(output: Option<std::path::PathBuf>) -> LongTermRepairPlanRequestIR {
        LongTermRepairPlanRequestIR {
            schema: LONG_TERM_REPAIR_PLAN_REQUEST_SCHEMA.to_string(),
            request_id: "REPAIR-TEST-1".to_string(),
            command: "입력 근거로 50쪽 장기수선계획을 작성해".to_string(),
            as_of_date: "2026-08-13".to_string(),
            plan_start_year: 2027,
            plan_years: 40,
            evidence: Vec::new(),
            profile: ApartmentProfileIR::default(),
            apartment_rules: Vec::new(),
            cost_inputs: Vec::new(),
            reserve_inputs: Vec::new(),
            output_html_path: output.map(|path| path.to_string_lossy().to_string()),
            max_plan_steps: 16,
        }
    }

    #[test]
    fn missing_inputs_stay_explicit_and_never_become_zero() {
        let core = DockableCore::load_embedded().unwrap();
        let response =
            process_long_term_repair_plan(&core, &request(None), &"a".repeat(64)).unwrap();
        assert_eq!(response.pages.len(), 50);
        assert_eq!(response.statutory_item_count, 69);
        assert_eq!(response.readiness, PlanReadinessIR::DraftRequiresEvidence);
        assert!(!response.approval_ready);
        assert!(response
            .decisions
            .iter()
            .all(|decision| decision.one_time_cost_won.is_none()));
        assert_eq!(response.external_model_calls, 0);
        assert!(response.deliberation.accepted);
    }

    #[test]
    fn calculation_uses_actual_years_and_fixed_point_values() {
        let mut request = request(None);
        request.cost_inputs.push(CostInputIR {
            item_id: "1-가-1".to_string(),
            method: RepairMethodIR::FullRepair,
            applicable: Some(true),
            quantity_milli_units: Some(10_000),
            unit_price_won: Some(1_000),
            overhead_factor_basis_points: Some(12_000),
            first_scheduled_year: Some(2030),
            overlapping_partial_repair_deduction_won: 0,
            evidence_refs: vec!["QUANTITY-1".to_string()],
        });
        let core = DockableCore::load_embedded().unwrap();
        let response = process_long_term_repair_plan(&core, &request, &"b".repeat(64)).unwrap();
        let decision = response
            .decisions
            .iter()
            .find(|decision| decision.item_id == "1-가-1")
            .unwrap();
        assert_eq!(decision.one_time_cost_won, Some(12_000));
        assert_eq!(decision.scheduled_years, vec![2030, 2045, 2060]);
        assert_eq!(decision.total_plan_cost_won, Some(36_000));
    }

    #[test]
    fn monthly_area_charge_is_derived_from_grounded_annual_contribution() {
        let mut request = request(None);
        request.profile = ApartmentProfileIR {
            complex_name: Some("검증단지".to_string()),
            use_approval_date: Some("2020-01-01".to_string()),
            household_count: Some(100),
            total_supply_area_milli_square_meters: Some(8_500_000),
            current_reserve_won: Some(100_000_000),
            household_area_types: vec![HouseholdAreaTypeIR {
                label: "85㎡형".to_string(),
                household_count: 100,
                supply_area_milli_square_meters: 85_000,
                evidence_refs: vec!["AREA-LEDGER".to_string()],
            }],
            evidence_refs: vec!["COMPLEX-LEDGER".to_string()],
        };
        request.reserve_inputs.push(AnnualReserveInputIR {
            year: 2027,
            contribution_won: 120_000_000,
            disposition_transfer_won: 0,
            interest_won: 0,
            evidence_refs: vec!["RESERVE-PLAN-2027".to_string()],
        });
        let core = DockableCore::load_embedded().unwrap();
        let response = process_long_term_repair_plan(&core, &request, &"e".repeat(64)).unwrap();
        assert_eq!(
            response.monthly_area_charges[0].monthly_charge_per_household_won,
            Some(100_000)
        );
    }

    #[test]
    fn conflicting_apartment_rule_is_not_silently_min_maxed() {
        let mut request = request(None);
        request.apartment_rules.push(ApartmentRepairRuleIR {
            item_id: "1-가-1".to_string(),
            method: RepairMethodIR::FullRepair,
            cycle_years: Some(20),
            repair_rate_percent: Some(80),
            adjustment_approval_evidence_id: None,
        });
        let core = DockableCore::load_embedded().unwrap();
        let response = process_long_term_repair_plan(&core, &request, &"c".repeat(64)).unwrap();
        let decision = response
            .decisions
            .iter()
            .find(|decision| decision.item_id == "1-가-1")
            .unwrap();
        assert_eq!(decision.resolution, RuleResolutionIR::ReviewRequired);
        assert_eq!(decision.applied_cycle_years, None);
        assert_eq!(decision.applied_repair_rate_percent, None);
    }

    #[test]
    fn html_writer_emits_exactly_fifty_a4_pages() {
        let output =
            std::env::temp_dir().join(format!("b-core-repair-{}.html", std::process::id()));
        let core = DockableCore::load_embedded().unwrap();
        let response =
            process_long_term_repair_plan(&core, &request(Some(output.clone())), &"d".repeat(64))
                .unwrap();
        let html = fs::read_to_string(&output).unwrap();
        assert_eq!(html.matches("<section class=\"a4-page\">").count(), 50);
        assert_eq!(response.file_receipt.unwrap().a4_page_count, 50);
        fs::remove_file(output).unwrap();
    }

    #[test]
    fn hwpx_text_is_extracted_without_an_external_program() {
        let output =
            std::env::temp_dir().join(format!("b-core-repair-{}.hwpx", std::process::id()));
        let file = fs::File::create(&output).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file(
                "Contents/section0.xml",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        archive
            .write_all("<hp:p><hp:t>단지명: 테스트아파트</hp:t></hp:p>".as_bytes())
            .unwrap();
        archive.finish().unwrap();
        let extracted = extract_evidence(&EvidenceInputIR {
            evidence_id: "HWPX-1".to_string(),
            path: output.to_string_lossy().to_string(),
            kind: EvidenceKindIR::Hwpx,
        });
        assert_eq!(extracted.receipt.status, EvidenceStatusIR::Extracted);
        assert!(extracted.text.contains("테스트아파트"));
        fs::remove_file(output).unwrap();
    }

    #[test]
    fn hwpx_extraction_preserves_section_and_table_structure() {
        let output =
            std::env::temp_dir().join(format!("b-core-structured-{}.hwpx", std::process::id()));
        let file = fs::File::create(&output).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file(
                "Contents/section0.xml",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        archive
            .write_all("<hp:p><hp:t>제1장 시설 현황</hp:t></hp:p>".as_bytes())
            .unwrap();
        archive
            .start_file(
                "Contents/section1.xml",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        archive
            .write_all(
                "<hp:tbl><hp:tr><hp:tc><hp:p><hp:t>항목</hp:t></hp:p></hp:tc></hp:tr></hp:tbl>"
                    .as_bytes(),
            )
            .unwrap();
        archive.finish().unwrap();
        let extracted = extract_evidence(&EvidenceInputIR {
            evidence_id: "HWPX-STRUCTURE".to_string(),
            path: output.to_string_lossy().to_string(),
            kind: EvidenceKindIR::Hwpx,
        });
        assert_eq!(extracted.receipt.status, EvidenceStatusIR::Extracted);
        assert_eq!(extracted.receipt.section_or_page_count, 2);
        assert!(!extracted.receipt.structure_sha256.is_empty());
        assert!(extracted
            .structured
            .blocks
            .iter()
            .any(|block| block.section_or_page == 2));
        fs::remove_file(output).unwrap();
    }

    #[test]
    fn paddle_metadata_becomes_location_bound_ocr_lines() {
        let structured = structure_extracted_text(
            "IMG-1",
            EvidenceKindIR::Image,
            "[[B_CORE_OCR|0.937|[1,2,3,4]|]]장기수선계획\n",
        );
        assert_eq!(structured.blocks.len(), 1);
        assert_eq!(structured.blocks[0].kind, EvidenceBlockKindIR::OcrLine);
        assert_eq!(structured.blocks[0].confidence_millis, Some(937));
        assert_eq!(structured.blocks[0].geometry.as_deref(), Some("[1,2,3,4]"));
        assert_eq!(structured.blocks[0].text, "장기수선계획");
        assert!(structured.blocks[0]
            .source_location
            .starts_with("IMG-1:section_or_page:1"));
    }
}
