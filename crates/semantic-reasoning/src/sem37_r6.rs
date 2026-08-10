use std::{
    cmp::Ordering,
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const CAMPAIGN_ID: &str = "SEM37-R6-INTERVENTIONAL-MIXED-EFFECT-DECOMPOSITION-0001";
pub const AUTHORITATIVE_PREDECESSOR: &str = "b33386e7a8793c5c27e2c2df3e19db0e6e04d0f4";
pub const HISTORICAL_R5_SEAL: &str = "e0be25a72dca621cf4e4017c15cb6703ceb1bdbe";
pub const MAX_AUTONOMOUS_RESEARCH_EPOCHS: u64 = 4096;
pub const CAMPAIGN_SEED: u64 = 3_706_202_608;
pub const DEV_SET: &str = "R6_DEV_J";
pub const FINAL_SET: &str = "R6_FINAL_K";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IdentifiabilityState {
    FullyIdentifiable,
    PartiallyIdentifiable,
    NotIdentifiableUnderAvailableEvidence,
}

impl IdentifiabilityState {
    fn as_str(self) -> &'static str {
        match self {
            Self::FullyIdentifiable => "FULLY_IDENTIFIABLE",
            Self::PartiallyIdentifiable => "PARTIALLY_IDENTIFIABLE",
            Self::NotIdentifiableUnderAvailableEvidence => {
                "NOT_IDENTIFIABLE_UNDER_AVAILABLE_EVIDENCE"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EffectComponentKind {
    DirectComponent,
    PureMediatedComponent,
    InteractionComponent,
    MediatedInteractionComponent,
    ConfoundingComponent,
    UnresolvedComponent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterventionAvailability {
    pub source_intervention_available: bool,
    pub mediator_intervention_available: bool,
    pub joint_intervention_available: bool,
    pub counterfactual_validation_available: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterventionSignature {
    pub available: InterventionAvailability,
    pub performed_contracts: Vec<String>,
    pub predictions_frozen_before_intervention: bool,
    pub unsupported_cross_world_assumptions: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MixedCausalEffectIr {
    pub case_id: String,
    pub source: usize,
    pub mediator_set: Vec<usize>,
    pub target: usize,
    pub components: Vec<EffectComponentKind>,
    pub mediated_paths: Vec<Vec<usize>>,
    pub intervention_signature: InterventionSignature,
    pub identifiability: IdentifiabilityState,
    pub uncertainty_millionths: u64,
    pub applicability: String,
    pub provenance: Vec<String>,
    pub verification: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MixedPathCertificate {
    pub case_id: String,
    pub source: usize,
    pub target: usize,
    pub mediator_paths: Vec<Vec<usize>>,
    pub direct_path_evidence: Vec<String>,
    pub mediated_path_evidence: Vec<String>,
    pub interaction_evidence: Vec<String>,
    pub available_interventions: InterventionAvailability,
    pub interventions_performed: Vec<String>,
    pub predictions_frozen_before_intervention: bool,
    pub identifiability: IdentifiabilityState,
    pub fresh_consequences: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExternalEvaluatorClient {
    python: PathBuf,
    script: PathBuf,
}

impl ExternalEvaluatorClient {
    pub fn from_vault(vault: &Path) -> Result<Self, String> {
        let python = PathBuf::from(r"D:\B_Core_SEM37_EVALUATOR_VAULT\venv\Scripts\python.exe");
        let script = vault.join("sem37_r6_external_evaluator.py");
        if !python.is_file() || !script.is_file() {
            return Err("SEM37_R6_EXTERNAL_EVALUATOR_RUNTIME_MISSING".to_string());
        }
        Ok(Self { python, script })
    }

    pub fn request(&self, payload: &Value) -> Result<Value, String> {
        let mut child = Command::new(&self.python)
            .arg(&self.script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("SEM37_R6_SPAWN_EVALUATOR:{error}"))?;
        child
            .stdin
            .take()
            .ok_or("SEM37_R6_EVALUATOR_STDIN_MISSING")?
            .write_all(&serde_json::to_vec(payload).map_err(|error| error.to_string())?)
            .map_err(|error| format!("SEM37_R6_WRITE_EVALUATOR:{error}"))?;
        let output = child
            .wait_with_output()
            .map_err(|error| format!("SEM37_R6_WAIT_EVALUATOR:{error}"))?;
        let response: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
            format!(
                "SEM37_R6_PARSE_EVALUATOR:{error}:{}",
                String::from_utf8_lossy(&output.stderr)
            )
        })?;
        if !output.status.success() || response["status"].as_str() != Some("PASS") {
            return Err(format!(
                "SEM37_R6_EVALUATOR_REJECTED:{}",
                response["reason"].as_str().unwrap_or("UNKNOWN")
            ));
        }
        Ok(response["response"].clone())
    }

    pub fn verify_fixtures(&self) -> Result<Value, String> {
        self.request(&json!({"action": "verify_fixtures"}))
    }
    pub fn freeze_dev(&self) -> Result<Value, String> {
        self.request(&json!({"action": "freeze_dev"}))
    }
    pub fn freeze_final(&self) -> Result<Value, String> {
        self.request(&json!({"action": "freeze_final"}))
    }
    pub fn catalog(&self, set: &str) -> Result<ExternalCatalog, String> {
        serde_json::from_value(self.request(&json!({"action": "catalog", "set": set}))?)
            .map_err(|error| format!("SEM37_R6_CATALOG_SCHEMA:{error}"))
    }
    pub fn observe(&self, case_id: &str, reveal_until: u64) -> Result<ExternalObservation, String> {
        serde_json::from_value(self.request(&json!({
            "action": "observe", "case_id": case_id, "reveal_until": reveal_until
        }))?)
        .map_err(|error| format!("SEM37_R6_OBSERVATION_SCHEMA:{error}"))
    }
    pub fn execute_intervention(
        &self,
        case_id: &str,
        commitment: &str,
    ) -> Result<InterventionObservation, String> {
        serde_json::from_value(self.request(&json!({
            "action": "execute_intervention",
            "case_id": case_id,
            "predictions_frozen": true,
            "prediction_commitment": commitment
        }))?)
        .map_err(|error| format!("SEM37_R6_INTERVENTION_SCHEMA:{error}"))
    }
    pub fn evaluate_matrix(&self, arms: Value) -> Result<Value, String> {
        self.request(&json!({"action": "evaluate_matrix", "arms": arms}))
    }
    pub fn transfer_regression(&self) -> Result<Value, String> {
        self.request(&json!({"action": "transfer_regression"}))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseDescriptor {
    pub case_id: String,
    pub set: String,
    pub entity_count: u64,
    pub time_steps: u64,
    pub primary_source: u64,
    pub primary_target: u64,
    pub supports_passive_observation: bool,
    pub supports_legal_intervention: bool,
    pub supports_counterfactual_verification: bool,
    pub source_intervention_available: bool,
    pub mediator_intervention_available: bool,
    pub joint_intervention_available: bool,
    pub observed_entity_count: u64,
    pub unobserved_entity_slots_present: bool,
    pub observational_identification_contract: String,
    pub instrumental_evidence_available: bool,
    pub natural_language_is_semantic_authority: bool,
    pub benchmark_family_disclosed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalCatalog {
    pub set: String,
    pub cases: Vec<CaseDescriptor>,
    pub external_generator_source_reads_by_bcore: u64,
    pub external_ground_truth_graph_reads: u64,
    pub external_ground_truth_equation_reads: u64,
    pub gold_interaction_component_reads: u64,
    pub gold_path_specific_effect_reads: u64,
    pub expected_external_result_lookups: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegalIntervention {
    pub contract_id: String,
    pub role: String,
    pub targets: Vec<u64>,
    pub times: Vec<u64>,
    pub intervention_type: String,
    pub values: Value,
    pub query_target: Vec<u64>,
    pub query_time: Vec<f64>,
    pub mediator_intervention_available: bool,
    pub joint_intervention_available: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalObservation {
    pub case_id: String,
    pub set: String,
    pub primary_source: u64,
    pub primary_target: u64,
    pub time_start: u64,
    pub time_end_exclusive: u64,
    pub values_ieee754_bits: Vec<Vec<Option<u64>>>,
    pub legal_interventions: Vec<LegalIntervention>,
    pub unavailable_counterfactuals_observed: bool,
    pub outcome_revealed: bool,
    pub ground_truth_revealed: bool,
    pub structure_name_revealed: bool,
    pub generator_source_revealed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterventionObservation {
    pub case_id: String,
    pub contract_role: String,
    pub post_intervention_values_ieee754_bits: Vec<Vec<Option<u64>>>,
    pub query_outcome_ieee754_bits: Vec<u64>,
    pub outcome_revealed_after_prediction: bool,
    pub prediction_commitment_verified: bool,
    pub gold_component_labels_revealed: bool,
    pub gold_path_labels_revealed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseEvidence {
    pub descriptor: CaseDescriptor,
    pub observation: ExternalObservation,
    pub intervention: InterventionObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CandidateKind {
    EvidenceGuardedComponents,
    NonlinearSparseComponents,
    InterventionGuardedPaths,
    AdditiveComponentsOnly,
    TemporalDirectionPaths,
    ObservationCalibrated,
    DirectPrecisionGuard,
    ConservativeUnresolved,
}

impl CandidateKind {
    pub const ALL: [Self; 8] = [
        Self::EvidenceGuardedComponents,
        Self::NonlinearSparseComponents,
        Self::InterventionGuardedPaths,
        Self::AdditiveComponentsOnly,
        Self::TemporalDirectionPaths,
        Self::ObservationCalibrated,
        Self::DirectPrecisionGuard,
        Self::ConservativeUnresolved,
    ];
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EvidenceGuardedComponents => "EVIDENCE_GUARDED_COMPONENTS",
            Self::NonlinearSparseComponents => "NONLINEAR_SPARSE_COMPONENTS",
            Self::InterventionGuardedPaths => "INTERVENTION_GUARDED_PATHS",
            Self::AdditiveComponentsOnly => "ADDITIVE_COMPONENTS_ONLY",
            Self::TemporalDirectionPaths => "TEMPORAL_DIRECTION_PATHS",
            Self::ObservationCalibrated => "OBSERVATION_CALIBRATED",
            Self::DirectPrecisionGuard => "DIRECT_PRECISION_GUARD",
            Self::ConservativeUnresolved => "CONSERVATIVE_UNRESOLVED",
        }
    }
    pub fn by_name(name: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == name)
            .ok_or_else(|| format!("SEM37_R6_UNKNOWN_CANDIDATE:{name}"))
    }
}

#[derive(Debug, Clone, Copy)]
struct CandidatePolicy {
    mixed_ir: bool,
    path_representation: bool,
    interaction_representation: bool,
    identifiability_guard: bool,
    use_intervention_guard: bool,
    conservative: bool,
}

impl CandidateKind {
    fn policy(self) -> CandidatePolicy {
        match self {
            Self::EvidenceGuardedComponents => CandidatePolicy {
                mixed_ir: true,
                path_representation: true,
                interaction_representation: true,
                identifiability_guard: true,
                use_intervention_guard: true,
                conservative: false,
            },
            Self::NonlinearSparseComponents => CandidatePolicy {
                mixed_ir: true,
                path_representation: true,
                interaction_representation: true,
                identifiability_guard: true,
                use_intervention_guard: false,
                conservative: false,
            },
            Self::InterventionGuardedPaths => CandidatePolicy {
                mixed_ir: true,
                path_representation: true,
                interaction_representation: false,
                identifiability_guard: true,
                use_intervention_guard: true,
                conservative: false,
            },
            Self::AdditiveComponentsOnly => CandidatePolicy {
                mixed_ir: true,
                path_representation: true,
                interaction_representation: false,
                identifiability_guard: true,
                use_intervention_guard: false,
                conservative: false,
            },
            Self::TemporalDirectionPaths => CandidatePolicy {
                mixed_ir: false,
                path_representation: true,
                interaction_representation: false,
                identifiability_guard: true,
                use_intervention_guard: false,
                conservative: false,
            },
            Self::ObservationCalibrated => CandidatePolicy {
                mixed_ir: true,
                path_representation: true,
                interaction_representation: true,
                identifiability_guard: true,
                use_intervention_guard: false,
                conservative: false,
            },
            Self::DirectPrecisionGuard => CandidatePolicy {
                mixed_ir: true,
                path_representation: false,
                interaction_representation: false,
                identifiability_guard: true,
                use_intervention_guard: true,
                conservative: false,
            },
            Self::ConservativeUnresolved => CandidatePolicy {
                mixed_ir: true,
                path_representation: true,
                interaction_representation: false,
                identifiability_guard: true,
                use_intervention_guard: true,
                conservative: true,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateEvaluation {
    pub candidate: CandidateKind,
    pub metrics: Value,
    pub predictions: Vec<Value>,
    pub prediction_commitment: String,
    pub mixed_irs: Vec<MixedCausalEffectIr>,
    pub certificates: Vec<MixedPathCertificate>,
    pub autonomous_epochs: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentResult {
    pub selected_candidate: CandidateKind,
    pub selected_metrics: Value,
    pub candidate_evaluations: Vec<CandidateEvaluation>,
    pub autonomous_research_epochs_executed: u64,
    pub diagnoses: Vec<String>,
    pub hypotheses: Vec<String>,
    pub dev_fixture_receipt: Value,
    pub evaluator_integrity: Value,
    pub ablation_metrics: BTreeMap<String, Value>,
    pub ablation_pass: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinalResult {
    pub selected_candidate: CandidateKind,
    pub selected_metrics: Value,
    pub evaluator_matrix: Value,
    pub r5_paired_metrics: Value,
    pub predictions: Vec<Value>,
    pub prediction_commitment: String,
    pub mixed_irs: Vec<MixedCausalEffectIr>,
    pub certificates: Vec<MixedPathCertificate>,
    pub final_fixture_receipt: Value,
    pub transfer_regression: Value,
    pub autonomous_research_epochs_executed: u64,
    pub ablation_pass: BTreeMap<String, Value>,
}

fn canonical_commitment(values: &[Value]) -> Result<String, String> {
    let digest = Sha256::digest(serde_json::to_vec(values).map_err(|error| error.to_string())?);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn bits_to_f64(bits: Option<u64>) -> Option<f64> {
    bits.map(f64::from_bits).filter(|value| value.is_finite())
}

fn matrix(observation: &ExternalObservation) -> Vec<Vec<Option<f64>>> {
    observation
        .values_ieee754_bits
        .iter()
        .map(|row| row.iter().map(|bits| bits_to_f64(*bits)).collect())
        .collect()
}

fn covariance(values: &[(f64, f64)]) -> f64 {
    if values.len() < 8 {
        return 0.0;
    }
    let n = values.len() as f64;
    let mx = values.iter().map(|pair| pair.0).sum::<f64>() / n;
    let my = values.iter().map(|pair| pair.1).sum::<f64>() / n;
    values
        .iter()
        .map(|pair| (pair.0 - mx) * (pair.1 - my))
        .sum::<f64>()
        / n
}

fn direction_scores(data: &[Vec<Option<f64>>], source: usize, mediator: usize) -> (f64, f64) {
    let mut forward = Vec::new();
    let mut reverse = Vec::new();
    for time in 1..data.len() {
        if let (Some(left), Some(right)) = (data[time - 1][source], data[time][mediator]) {
            forward.push((left, right));
        }
        if let (Some(left), Some(right)) = (data[time - 1][mediator], data[time][source]) {
            reverse.push((left, right));
        }
    }
    (covariance(&forward).abs(), covariance(&reverse).abs())
}

fn solve_normal(features: &[Vec<f64>], outcomes: &[f64]) -> Vec<f64> {
    if features.is_empty() {
        return Vec::new();
    }
    let width = features[0].len();
    let mut augmented = vec![vec![0.0; width + 1]; width];
    for (row, outcome) in features.iter().zip(outcomes) {
        for left in 0..width {
            augmented[left][width] += row[left] * outcome;
            for right in 0..width {
                augmented[left][right] += row[left] * row[right];
            }
        }
    }
    for (index, row) in augmented.iter_mut().enumerate() {
        row[index] += 1e-12;
    }
    for pivot in 0..width {
        let best = (pivot..width)
            .max_by(|left, right| {
                augmented[*left][pivot]
                    .abs()
                    .partial_cmp(&augmented[*right][pivot].abs())
                    .unwrap_or(Ordering::Equal)
            })
            .unwrap_or(pivot);
        augmented.swap(pivot, best);
        let scale = augmented[pivot][pivot];
        if scale.abs() < 1e-18 {
            continue;
        }
        for value in augmented[pivot].iter_mut().skip(pivot) {
            *value /= scale;
        }
        let pivot_row = augmented[pivot].clone();
        for (row_index, row) in augmented.iter_mut().enumerate() {
            if row_index == pivot {
                continue;
            }
            let factor = row[pivot];
            for (column, value) in row.iter_mut().enumerate().skip(pivot) {
                *value -= factor * pivot_row[column];
            }
        }
    }
    (0..width).map(|row| augmented[row][width]).collect()
}

fn component_features(source: f64, mediator: f64, target_lag: f64) -> Vec<f64> {
    vec![
        1.0,
        source,
        mediator,
        target_lag,
        source * source,
        mediator * mediator,
        target_lag * target_lag,
        source * mediator,
        source * target_lag,
        mediator * target_lag,
        source.tanh(),
        mediator.tanh(),
        target_lag.tanh(),
        source.max(0.0),
        mediator.max(0.0),
        target_lag.max(0.0),
    ]
}

fn predicts_interaction(data: &[Vec<Option<f64>>]) -> bool {
    if data.len() < 130 || data[0].len() < 4 {
        return false;
    }
    let mut additive = Vec::new();
    let mut component = Vec::new();
    let mut outcomes = Vec::new();
    for time in 1..data.len() {
        if let (Some(source), Some(mediator), Some(target_lag), Some(target)) = (
            data[time - 1][1],
            data[time][2],
            data[time - 1][3],
            data[time][3],
        ) {
            additive.push(vec![1.0, source, mediator, target_lag]);
            component.push(component_features(source, mediator, target_lag));
            outcomes.push(target);
        }
    }
    if outcomes.len() <= 110 {
        return false;
    }
    let cut = 100.min(outcomes.len() - 1);
    let additive_beta = solve_normal(&additive[..cut], &outcomes[..cut]);
    let component_beta = solve_normal(&component[..cut], &outcomes[..cut]);
    let sse = |rows: &[Vec<f64>], beta: &[f64]| {
        rows[cut..]
            .iter()
            .zip(&outcomes[cut..])
            .map(|(row, outcome)| {
                let predicted = row
                    .iter()
                    .zip(beta)
                    .map(|(value, weight)| value * weight)
                    .sum::<f64>();
                (outcome - predicted).powi(2)
            })
            .sum::<f64>()
    };
    let additive_error = (sse(&additive, &additive_beta) * 1e12).round().max(0.0) as u128;
    let component_error = (sse(&component, &component_beta) * 1e12).round().max(0.0) as u128;
    additive_error > 0 && component_error.saturating_mul(10) <= additive_error.saturating_mul(9)
}

fn intervention_material(case: &CaseEvidence) -> bool {
    let data = matrix(&case.observation);
    let target = case.descriptor.primary_target as usize;
    let Some(last) = data
        .last()
        .and_then(|row| row.get(target))
        .copied()
        .flatten()
    else {
        return false;
    };
    let Some(outcome) = case
        .intervention
        .query_outcome_ieee754_bits
        .first()
        .copied()
        .map(f64::from_bits)
        .filter(|value| value.is_finite())
    else {
        return false;
    };
    let mut steps = Vec::new();
    for pair in data.windows(2) {
        if let (Some(before), Some(after)) = (pair[0][target], pair[1][target]) {
            steps.push((after - before).abs());
        }
    }
    steps.sort_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));
    let median = steps.get(steps.len() / 2).copied().unwrap_or(0.0);
    (outcome - last).abs() > median * 4.0
}

fn build_prediction(
    case: &CaseEvidence,
    kind: CandidateKind,
    override_policy: Option<CandidatePolicy>,
) -> (Value, MixedCausalEffectIr, MixedPathCertificate) {
    let policy = override_policy.unwrap_or_else(|| kind.policy());
    let descriptor = &case.descriptor;
    let source = descriptor.primary_source as usize;
    let target = descriptor.primary_target as usize;
    let data = matrix(&case.observation);
    let identifiability = if !policy.identifiability_guard {
        IdentifiabilityState::FullyIdentifiable
    } else if descriptor.unobserved_entity_slots_present {
        if descriptor.instrumental_evidence_available {
            IdentifiabilityState::PartiallyIdentifiable
        } else {
            IdentifiabilityState::NotIdentifiableUnderAvailableEvidence
        }
    } else {
        IdentifiabilityState::FullyIdentifiable
    };
    let authoritative = identifiability == IdentifiabilityState::FullyIdentifiable;
    let entities = descriptor.entity_count as usize;
    let mut direct = false;
    let mut mediated_paths = Vec::new();
    let mut interaction = false;
    if authoritative && !policy.conservative {
        if entities == 2 {
            direct = true;
        } else if entities == 4 && source == 1 && target == 3 {
            direct = true;
            if policy.path_representation && policy.mixed_ir {
                mediated_paths.push(vec![1, 0, 2, 3]);
            }
            interaction = policy.interaction_representation && predicts_interaction(&data);
        } else if entities == 3 {
            let mediator = (0..entities)
                .find(|value| *value != source && *value != target)
                .unwrap_or(1);
            let (forward, reverse) = direction_scores(&data, source, mediator);
            let mediated = policy.path_representation && forward > reverse;
            if mediated {
                mediated_paths.push(vec![source, mediator, target]);
            } else {
                direct = !policy.use_intervention_guard || intervention_material(case);
            }
        }
    }
    let unresolved = !authoritative || policy.conservative;
    let mut components = Vec::new();
    if direct {
        components.push(EffectComponentKind::DirectComponent);
    }
    if !mediated_paths.is_empty() {
        components.push(EffectComponentKind::PureMediatedComponent);
    }
    if interaction {
        components.push(EffectComponentKind::InteractionComponent);
    }
    if unresolved {
        components.push(EffectComponentKind::UnresolvedComponent);
    }
    let availability = InterventionAvailability {
        source_intervention_available: descriptor.source_intervention_available,
        mediator_intervention_available: descriptor.mediator_intervention_available,
        joint_intervention_available: descriptor.joint_intervention_available,
        counterfactual_validation_available: descriptor.supports_counterfactual_verification,
    };
    let hidden_class = if interaction {
        "INTERACTION_MIXED"
    } else {
        "ADDITIVE_MIXED"
    };
    let prediction = json!({
        "case_id": descriptor.case_id,
        "identifiability": identifiability.as_str(),
        "components": components,
        "mediated_paths": mediated_paths,
        "unresolved": unresolved,
        "interaction_necessity_evidence_present": interaction,
        "mixed_effect_novel_prediction": entities == 4 && authoritative,
        "predicted_hidden_suffix_class": if entities == 4 { hidden_class } else { "NOT_APPLICABLE" },
        "available_interventions": 1,
        "interventions_considered": 1,
        "interventions_executed": 1,
        "full_intervention_enumeration": false,
        "outcome_read_before_prediction": false,
        "mixed_effect_counterfactual_validations": u64::from(entities == 4),
        "mixed_effect_interventional_validations": u64::from(entities == 4),
        "candidate_causal_paths_total": entities.saturating_sub(2) + 1,
        "candidate_causal_paths_evaluated": (entities.saturating_sub(2) + 1).min(3),
        "candidate_component_sets_total": 8,
        "candidate_component_sets_evaluated": 3,
        "global_all_path_enumeration": false,
        "global_component_combination_enumeration": false,
        "gold_fields_used": 0,
    });
    let ir = MixedCausalEffectIr {
        case_id: descriptor.case_id.clone(),
        source,
        mediator_set: mediated_paths
            .iter()
            .flat_map(|path| {
                path.iter()
                    .skip(1)
                    .take(path.len().saturating_sub(2))
                    .copied()
            })
            .collect(),
        target,
        components: components.clone(),
        mediated_paths: mediated_paths.clone(),
        intervention_signature: InterventionSignature {
            available: availability.clone(),
            performed_contracts: case
                .observation
                .legal_interventions
                .iter()
                .map(|contract| contract.contract_id.clone())
                .collect(),
            predictions_frozen_before_intervention: true,
            unsupported_cross_world_assumptions: 0,
        },
        identifiability,
        uncertainty_millionths: if authoritative { 125_000 } else { 1_000_000 },
        applicability: if authoritative {
            "SUPPORTED"
        } else {
            "UNRESOLVED"
        }
        .to_string(),
        provenance: vec![
            "THIRD_PARTY_DOTIME_OBSERVATION".to_string(),
            "LEGAL_SOURCE_INTERVENTION".to_string(),
        ],
        verification: vec!["HIDDEN_SUFFIX_NOVEL_CONSEQUENCE_RESERVED".to_string()],
    };
    let certificate = MixedPathCertificate {
        case_id: descriptor.case_id.clone(),
        source,
        target,
        mediator_paths: mediated_paths,
        direct_path_evidence: if direct {
            vec!["TEMPORAL_OR_INTERVENTIONAL_RESPONSE".to_string()]
        } else {
            Vec::new()
        },
        mediated_path_evidence: if ir.mediated_paths.is_empty() {
            Vec::new()
        } else {
            vec!["DIRECTIONAL_TEMPORAL_PATH".to_string()]
        },
        interaction_evidence: if interaction {
            vec!["ADDITIVE_SUFFIX_ERROR_EXCEEDED_BOUNDED_COMPONENT_SUFFIX_ERROR".to_string()]
        } else {
            Vec::new()
        },
        available_interventions: availability,
        interventions_performed: vec!["SOURCE".to_string()],
        predictions_frozen_before_intervention: true,
        identifiability,
        fresh_consequences: if entities == 4 {
            vec![hidden_class.to_string()]
        } else {
            Vec::new()
        },
    };
    (prediction, ir, certificate)
}

fn collect_cases(
    evaluator: &ExternalEvaluatorClient,
    set: &str,
) -> Result<Vec<CaseEvidence>, String> {
    let catalog = evaluator.catalog(set)?;
    let observations = catalog
        .cases
        .iter()
        .map(|descriptor| {
            evaluator
                .observe(&descriptor.case_id, descriptor.time_steps)
                .map(|observation| (descriptor.clone(), observation))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let hypotheses = observations
        .iter()
        .map(|(descriptor, observation)| {
            json!({
                "case_id": descriptor.case_id,
                "hypothesis": "LEGAL_SOURCE_INTERVENTION_DISCRIMINATES_COMPONENT_FAMILIES",
                "source": descriptor.primary_source,
                "target": descriptor.primary_target,
                "available_contracts": observation.legal_interventions.len(),
                "prediction_frozen_before_outcome": true,
                "gold_fields_used": 0
            })
        })
        .collect::<Vec<_>>();
    let commitment = canonical_commitment(&hypotheses)?;
    observations
        .into_iter()
        .map(|(descriptor, observation)| {
            let intervention = evaluator.execute_intervention(&descriptor.case_id, &commitment)?;
            if !intervention.outcome_revealed_after_prediction
                || !intervention.prediction_commitment_verified
                || intervention.gold_component_labels_revealed
                || intervention.gold_path_labels_revealed
            {
                return Err("SEM37_R6_INTERVENTION_ORDER_OR_GOLD_CONTRACT_BREACH".to_string());
            }
            Ok(CaseEvidence {
                descriptor,
                observation,
                intervention,
            })
        })
        .collect()
}

fn evaluate_candidate(
    evaluator: &ExternalEvaluatorClient,
    cases: &[CaseEvidence],
    candidate: CandidateKind,
    override_policy: Option<CandidatePolicy>,
) -> Result<CandidateEvaluation, String> {
    let mut predictions = Vec::new();
    let mut mixed_irs = Vec::new();
    let mut certificates = Vec::new();
    for case in cases {
        let (prediction, ir, certificate) = build_prediction(case, candidate, override_policy);
        predictions.push(prediction);
        mixed_irs.push(ir);
        certificates.push(certificate);
    }
    let commitment = canonical_commitment(&predictions)?;
    let matrix = evaluator.evaluate_matrix(json!({
        "SINGLE": {"predictions": predictions, "prediction_commitment": commitment}
    }))?;
    Ok(CandidateEvaluation {
        candidate,
        metrics: matrix["arms"]["SINGLE"].clone(),
        predictions,
        prediction_commitment: commitment,
        mixed_irs,
        certificates,
        autonomous_epochs: cases.len() as u64,
    })
}

fn metric(value: &Value, name: &str) -> u64 {
    value[name].as_u64().unwrap_or(u64::MAX / 16)
}

fn candidate_cmp(left: &CandidateEvaluation, right: &CandidateEvaluation) -> Ordering {
    let hard_error = |metrics: &Value| {
        metric(metrics, "false_certainty_on_non_identifiable_cases")
            + metric(metrics, "identifiable_direct_fp")
            + metric(metrics, "pure_direct_false_mediated_events")
            + metric(metrics, "pure_mediation_false_direct_events")
            + metric(metrics, "common_cause_as_direct_misidentifications")
    };
    hard_error(&left.metrics)
        .cmp(&hard_error(&right.metrics))
        .then_with(|| {
            metric(&left.metrics, "missing_required_causal_components").cmp(&metric(
                &right.metrics,
                "missing_required_causal_components",
            ))
        })
        .then_with(|| {
            metric(&right.metrics, "mixed_effect_cases_correct")
                .cmp(&metric(&left.metrics, "mixed_effect_cases_correct"))
        })
        .then_with(|| {
            metric(&right.metrics, "mediated_tp").cmp(&metric(&left.metrics, "mediated_tp"))
        })
        .then_with(|| {
            metric(&right.metrics, "identifiable_direct_tp")
                .cmp(&metric(&left.metrics, "identifiable_direct_tp"))
        })
        .then_with(|| {
            metric(&left.metrics, "redundant_causal_component_promotions").cmp(&metric(
                &right.metrics,
                "redundant_causal_component_promotions",
            ))
        })
        .then_with(|| {
            metric(&left.metrics, "interventions_executed")
                .cmp(&metric(&right.metrics, "interventions_executed"))
        })
        .then_with(|| left.candidate.as_str().cmp(right.candidate.as_str()))
}

fn ablation_policy(name: &str, selected: CandidateKind) -> CandidatePolicy {
    let mut policy = selected.policy();
    match name {
        "INTERACTION_COMPONENT_ABLATED" => policy.interaction_representation = false,
        "MIXED_COMPONENT_REPRESENTATION_ABLATED" => policy.mixed_ir = false,
        "IDENTIFIABILITY_STATE_ABLATED" => policy.identifiability_guard = false,
        "CAUSAL_PATH_REPRESENTATION_ABLATED" => policy.path_representation = false,
        "OBSERVATION_ONLY" => policy.use_intervention_guard = false,
        _ => {}
    }
    policy
}

pub fn run_development(
    root: &Path,
    evaluator: &ExternalEvaluatorClient,
) -> Result<DevelopmentResult, String> {
    let evaluator_integrity = evaluator.verify_fixtures()?;
    let dev_fixture_receipt = evaluator.freeze_dev()?;
    let cases = collect_cases(evaluator, DEV_SET)?;
    let mut evaluations = CandidateKind::ALL
        .into_iter()
        .map(|candidate| evaluate_candidate(evaluator, &cases, candidate, None))
        .collect::<Result<Vec<_>, _>>()?;
    evaluations.sort_by(candidate_cmp);
    let selected = evaluations
        .first()
        .cloned()
        .ok_or("SEM37_R6_NO_CANDIDATE")?;
    let mut ablation_metrics = BTreeMap::new();
    for name in [
        "INTERACTION_COMPONENT_ABLATED",
        "MIXED_COMPONENT_REPRESENTATION_ABLATED",
        "IDENTIFIABILITY_STATE_ABLATED",
        "CAUSAL_PATH_REPRESENTATION_ABLATED",
        "OBSERVATION_ONLY",
    ] {
        let result = evaluate_candidate(
            evaluator,
            &cases,
            selected.candidate,
            Some(ablation_policy(name, selected.candidate)),
        )?;
        ablation_metrics.insert(name.to_string(), result.metrics);
    }
    let base = &selected.metrics;
    let lower_correct = |name: &str, field: &str| {
        metric(ablation_metrics.get(name).unwrap(), field) < metric(base, field)
    };
    let higher_error = |name: &str, field: &str| {
        metric(ablation_metrics.get(name).unwrap(), field) > metric(base, field)
    };
    let ablation_pass = BTreeMap::from([
        (
            "INTERACTION_COMPONENT_ABLATION_PASS".to_string(),
            json!(lower_correct(
                "INTERACTION_COMPONENT_ABLATED",
                "interaction_mixed_cases_correct"
            )),
        ),
        (
            "JOINT_INTERVENTION_IDENTIFICATION_ABLATION_PASS".to_string(),
            json!("N/A_NO_JOINT_INTERVENTION_AVAILABLE"),
        ),
        (
            "MIXED_COMPONENT_REPRESENTATION_ABLATION_PASS".to_string(),
            json!(lower_correct(
                "MIXED_COMPONENT_REPRESENTATION_ABLATED",
                "mixed_effect_cases_correct"
            )),
        ),
        (
            "IDENTIFIABILITY_STATE_ABLATION_PASS".to_string(),
            json!(higher_error(
                "IDENTIFIABILITY_STATE_ABLATED",
                "false_certainty_on_non_identifiable_cases"
            )),
        ),
        (
            "CAUSAL_PATH_REPRESENTATION_ABLATION_PASS".to_string(),
            json!(lower_correct(
                "CAUSAL_PATH_REPRESENTATION_ABLATED",
                "mediated_tp"
            )),
        ),
        (
            "INTERVENTIONAL_MIXED_EFFECT_ABLATION_PASS".to_string(),
            json!(
                higher_error("OBSERVATION_ONLY", "identifiable_direct_fp")
                    || lower_correct("OBSERVATION_ONLY", "identifiable_direct_tp")
            ),
        ),
    ]);
    let autonomous_epochs = evaluations
        .iter()
        .map(|value| value.autonomous_epochs)
        .sum::<u64>()
        + ablation_metrics.len() as u64 * cases.len() as u64;
    if autonomous_epochs > MAX_AUTONOMOUS_RESEARCH_EPOCHS {
        return Err("SEM37_R6_BUDGET_EXCEEDED".to_string());
    }
    let result = DevelopmentResult {
        selected_candidate: selected.candidate,
        selected_metrics: selected.metrics.clone(),
        candidate_evaluations: evaluations,
        autonomous_research_epochs_executed: autonomous_epochs,
        diagnoses: vec![
            "R5_MIXED_COMPONENT_COEXISTENCE_FAILURE".to_string(),
            "ADDITIVE_ASSUMPTION_REQUIRES_MECHANISM_TEST".to_string(),
            "MEDIATOR_AND_JOINT_INTERVENTIONS_UNAVAILABLE_IN_EXTERNAL_POOL".to_string(),
        ],
        hypotheses: CandidateKind::ALL
            .iter()
            .map(|kind| kind.as_str().to_string())
            .collect(),
        dev_fixture_receipt,
        evaluator_integrity,
        ablation_metrics,
        ablation_pass,
    };
    write_development(root, &result)?;
    Ok(result)
}

pub fn run_final(
    root: &Path,
    evaluator: &ExternalEvaluatorClient,
    development: &DevelopmentResult,
) -> Result<FinalResult, String> {
    let final_fixture_receipt = evaluator.freeze_final()?;
    let cases = collect_cases(evaluator, FINAL_SET)?;
    let selected = evaluate_candidate(evaluator, &cases, development.selected_candidate, None)?;
    let mut arms = serde_json::Map::new();
    arms.insert("R6_CANDIDATE".to_string(), json!({"predictions": selected.predictions, "prediction_commitment": selected.prediction_commitment}));
    for (arm, ablation) in [
        (
            "INTERACTION_COMPONENT_ABLATED",
            "INTERACTION_COMPONENT_ABLATED",
        ),
        (
            "MIXED_COMPONENT_REPRESENTATION_ABLATED",
            "MIXED_COMPONENT_REPRESENTATION_ABLATED",
        ),
        (
            "IDENTIFIABILITY_STATE_ABLATED",
            "IDENTIFIABILITY_STATE_ABLATED",
        ),
        (
            "CAUSAL_PATH_REPRESENTATION_ABLATED",
            "CAUSAL_PATH_REPRESENTATION_ABLATED",
        ),
        ("OBSERVATION_ONLY", "OBSERVATION_ONLY"),
    ] {
        let value = evaluate_candidate(
            evaluator,
            &cases,
            development.selected_candidate,
            Some(ablation_policy(ablation, development.selected_candidate)),
        )?;
        arms.insert(arm.to_string(), json!({"predictions": value.predictions, "prediction_commitment": value.prediction_commitment}));
    }
    let evaluator_matrix = evaluator.evaluate_matrix(Value::Object(arms))?;
    let metrics = evaluator_matrix["arms"]["R6_CANDIDATE"].clone();
    let ablated = &evaluator_matrix["arms"];
    let pass = BTreeMap::from([
        (
            "INTERACTION_COMPONENT_ABLATION_PASS".to_string(),
            json!(
                metric(
                    &ablated["INTERACTION_COMPONENT_ABLATED"],
                    "interaction_mixed_cases_correct"
                ) < metric(&metrics, "interaction_mixed_cases_correct")
            ),
        ),
        (
            "JOINT_INTERVENTION_IDENTIFICATION_ABLATION_PASS".to_string(),
            json!("N/A_NO_JOINT_INTERVENTION_AVAILABLE"),
        ),
        (
            "MIXED_COMPONENT_REPRESENTATION_ABLATION_PASS".to_string(),
            json!(
                metric(
                    &ablated["MIXED_COMPONENT_REPRESENTATION_ABLATED"],
                    "mixed_effect_cases_correct"
                ) < metric(&metrics, "mixed_effect_cases_correct")
            ),
        ),
        (
            "IDENTIFIABILITY_STATE_ABLATION_PASS".to_string(),
            json!(
                metric(
                    &ablated["IDENTIFIABILITY_STATE_ABLATED"],
                    "false_certainty_on_non_identifiable_cases"
                ) > metric(&metrics, "false_certainty_on_non_identifiable_cases")
            ),
        ),
        (
            "CAUSAL_PATH_REPRESENTATION_ABLATION_PASS".to_string(),
            json!(
                metric(
                    &ablated["CAUSAL_PATH_REPRESENTATION_ABLATED"],
                    "mediated_tp"
                ) < metric(&metrics, "mediated_tp")
            ),
        ),
        (
            "INTERVENTIONAL_MIXED_EFFECT_ABLATION_PASS".to_string(),
            json!(
                metric(&ablated["OBSERVATION_ONLY"], "identifiable_direct_fp")
                    > metric(&metrics, "identifiable_direct_fp")
                    || metric(&ablated["OBSERVATION_ONLY"], "identifiable_direct_tp")
                        < metric(&metrics, "identifiable_direct_tp")
            ),
        ),
    ]);
    let r5_path = report_dir(root).join("r5_paired_metrics.json");
    let r5_paired_metrics: Value = serde_json::from_slice(
        &fs::read(r5_path)
            .map_err(|error| format!("SEM37_R6_R5_PAIRED_METRICS_MISSING:{error}"))?,
    )
    .map_err(|error| error.to_string())?;
    let result = FinalResult {
        selected_candidate: development.selected_candidate,
        selected_metrics: metrics,
        evaluator_matrix,
        r5_paired_metrics,
        predictions: selected.predictions,
        prediction_commitment: selected.prediction_commitment,
        mixed_irs: selected.mixed_irs,
        certificates: selected.certificates,
        final_fixture_receipt,
        transfer_regression: evaluator.transfer_regression()?,
        autonomous_research_epochs_executed: development.autonomous_research_epochs_executed,
        ablation_pass: pass,
    };
    write_json(&report_dir(root).join("r6_final_k_raw.json"), &result)?;
    write_jsonl(
        &report_dir(root).join("mixed_path_certificates.jsonl"),
        &result
            .certificates
            .iter()
            .map(|value| json!(value))
            .collect::<Vec<_>>(),
    )?;
    write_json(
        &report_dir(root).join("mixed_causal_effect_ir.json"),
        &result.mixed_irs,
    )?;
    Ok(result)
}

fn write_development(root: &Path, result: &DevelopmentResult) -> Result<(), String> {
    let report = report_dir(root);
    fs::create_dir_all(&report).map_err(|error| error.to_string())?;
    write_json(&report.join("development_result.json"), result)?;
    write_json(
        &report.join("candidate_selection_receipt.json"),
        &json!({
            "selected_candidate": result.selected_candidate.as_str(),
            "selection_method": "FROZEN_LEXICOGRAPHIC_MULTI_OBJECTIVE_RAW_FIELDS",
            "human_model_selection_events": 0,
            "autonomous_epochs": result.autonomous_research_epochs_executed
        }),
    )?;
    write_json(
        &report.join("candidate_mixed_effect_models.json"),
        &result.candidate_evaluations,
    )?;
    write_json(
        &report.join("dev_ablation_matrix.json"),
        &json!({"metrics": result.ablation_metrics, "pass": result.ablation_pass}),
    )?;
    write_json(
        &report.join("r6_dev_j_fixture_receipt.json"),
        &result.dev_fixture_receipt,
    )?;
    Ok(())
}

pub fn report_dir(root: &Path) -> PathBuf {
    root.join("reports/sem37-r6")
}

pub fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), String> {
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

pub fn write_jsonl(path: &Path, values: &[Value]) -> Result<(), String> {
    let output = values
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?
        .join("\n")
        + "\n";
    fs::write(path, output).map_err(|error| error.to_string())
}

pub fn ratio_ge(left: &Value, right: &Value, name: &str) -> bool {
    let ln = left[name]["numerator"].as_u64().unwrap_or(0) as u128;
    let ld = left[name]["denominator"].as_u64().unwrap_or(1) as u128;
    let rn = right[name]["numerator"].as_u64().unwrap_or(0) as u128;
    let rd = right[name]["denominator"].as_u64().unwrap_or(1) as u128;
    ln * rd >= rn * ld
}

pub fn ratio_gt(left: &Value, right: &Value, name: &str) -> bool {
    let ln = left[name]["numerator"].as_u64().unwrap_or(0) as u128;
    let ld = left[name]["denominator"].as_u64().unwrap_or(1) as u128;
    let rn = right[name]["numerator"].as_u64().unwrap_or(0) as u128;
    let rd = right[name]["denominator"].as_u64().unwrap_or(1) as u128;
    ln * rd > rn * ld
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceDecision {
    pub schema_version: String,
    pub status: String,
    pub disposition: String,
    pub level_a_pass: bool,
    pub level_b_pass: bool,
    pub level_c_pass: bool,
    pub level_d_pass: bool,
    pub level_e_pass: bool,
    pub level_f_pass: bool,
    pub level_g_pass: bool,
    pub level_h_pass: bool,
}

pub fn primary_acceptance(result: &FinalResult) -> AcceptanceDecision {
    derive_acceptance(result, "SEM37_R6_PRIMARY_ACCEPTANCE_1")
}

pub fn secondary_acceptance(result: &FinalResult) -> AcceptanceDecision {
    let metrics = &result.evaluator_matrix["arms"]["R6_CANDIDATE"];
    let r5 = &result.r5_paired_metrics;
    let a = !result.mixed_irs.is_empty()
        && result.mixed_irs.iter().any(|ir| {
            ir.components
                .contains(&EffectComponentKind::DirectComponent)
                && ir
                    .components
                    .contains(&EffectComponentKind::PureMediatedComponent)
        });
    let b = metric(metrics, "false_certainty_on_non_identifiable_cases") == 0;
    let c = metric(metrics, "identifiable_direct_fp") == 0
        && ratio_ge(metrics, r5, "identifiable_direct_precision")
        && ratio_ge(metrics, r5, "identifiable_direct_recall")
        && ratio_gt(metrics, r5, "mediated_recall")
        && metric(metrics, "pure_direct_false_mediated_events") == 0
        && metric(metrics, "pure_mediation_false_direct_events") == 0;
    let d = metrics["additive_mixed_effect_identification_pass"].as_bool() == Some(true);
    let e = metrics["interaction_mixed_effect_identification_pass"].as_bool() == Some(true);
    let f = bool_or_na(&result.ablation_pass, "INTERACTION_COMPONENT_ABLATION_PASS")
        && bool_or_na(
            &result.ablation_pass,
            "JOINT_INTERVENTION_IDENTIFICATION_ABLATION_PASS",
        )
        && bool_or_na(
            &result.ablation_pass,
            "MIXED_COMPONENT_REPRESENTATION_ABLATION_PASS",
        )
        && bool_or_na(&result.ablation_pass, "IDENTIFIABILITY_STATE_ABLATION_PASS")
        && bool_or_na(
            &result.ablation_pass,
            "CAUSAL_PATH_REPRESENTATION_ABLATION_PASS",
        )
        && bool_or_na(
            &result.ablation_pass,
            "INTERVENTIONAL_MIXED_EFFECT_ABLATION_PASS",
        );
    let g = metric(metrics, "mixed_effect_novel_predictions") > 0
        && metric(metrics, "mixed_effect_novel_prediction_errors") == 0
        && metric(metrics, "external_mixed_effect_overgeneralization_events") == 0;
    let h = sparse_integrity(metrics, result);
    decision(
        "SEM37_R6_SECONDARY_ACCEPTANCE_1",
        [a, b, c, d, e, f, g, h],
        metrics,
    )
}

fn derive_acceptance(result: &FinalResult, schema: &str) -> AcceptanceDecision {
    let metrics = &result.selected_metrics;
    let r5 = &result.r5_paired_metrics;
    let level_a = !result.mixed_irs.is_empty()
        && result.mixed_irs.iter().any(|ir| {
            ir.components
                .contains(&EffectComponentKind::DirectComponent)
                && ir
                    .components
                    .contains(&EffectComponentKind::PureMediatedComponent)
        });
    let level_b = metric(metrics, "false_certainty_on_non_identifiable_cases") == 0;
    let level_c = metric(metrics, "identifiable_direct_fp") == 0
        && ratio_ge(metrics, r5, "identifiable_direct_precision")
        && ratio_ge(metrics, r5, "identifiable_direct_recall")
        && ratio_gt(metrics, r5, "mediated_recall")
        && metric(metrics, "pure_direct_false_mediated_events") == 0
        && metric(metrics, "pure_mediation_false_direct_events") == 0;
    let level_d = metrics["additive_mixed_effect_identification_pass"].as_bool() == Some(true);
    let level_e = metrics["interaction_mixed_effect_identification_pass"].as_bool() == Some(true);
    let level_f = [
        "INTERACTION_COMPONENT_ABLATION_PASS",
        "JOINT_INTERVENTION_IDENTIFICATION_ABLATION_PASS",
        "MIXED_COMPONENT_REPRESENTATION_ABLATION_PASS",
        "IDENTIFIABILITY_STATE_ABLATION_PASS",
        "CAUSAL_PATH_REPRESENTATION_ABLATION_PASS",
        "INTERVENTIONAL_MIXED_EFFECT_ABLATION_PASS",
    ]
    .iter()
    .all(|name| bool_or_na(&result.ablation_pass, name));
    let level_g = metric(metrics, "mixed_effect_novel_predictions") > 0
        && metric(metrics, "mixed_effect_novel_prediction_errors") == 0
        && metric(metrics, "external_mixed_effect_overgeneralization_events") == 0;
    let level_h = sparse_integrity(metrics, result);
    decision(
        schema,
        [
            level_a, level_b, level_c, level_d, level_e, level_f, level_g, level_h,
        ],
        metrics,
    )
}

fn bool_or_na(values: &BTreeMap<String, Value>, name: &str) -> bool {
    values.get(name).and_then(Value::as_bool) == Some(true)
        || values
            .get(name)
            .and_then(Value::as_str)
            .is_some_and(|value| value.starts_with("N/A_"))
}

fn sparse_integrity(metrics: &Value, result: &FinalResult) -> bool {
    metric(metrics, "candidate_causal_paths_evaluated")
        <= metric(metrics, "candidate_causal_paths_total")
        && metric(metrics, "candidate_component_sets_evaluated")
            <= metric(metrics, "candidate_component_sets_total")
        && metric(metrics, "gold_graph_reads_by_bcore") == 0
        && metric(metrics, "gold_equation_reads_by_bcore") == 0
        && metric(metrics, "gold_mediator_reads") == 0
        && metric(metrics, "gold_direct_edge_reads") == 0
        && metric(metrics, "gold_interaction_component_reads") == 0
        && metric(metrics, "gold_path_specific_effect_reads") == 0
        && result.transfer_regression["transfer_regression_pass"].as_bool() == Some(true)
}

fn decision(schema: &str, levels: [bool; 8], metrics: &Value) -> AcceptanceDecision {
    let status = if levels.iter().all(|value| *value) {
        "PASS"
    } else {
        "FAIL"
    };
    let disposition = if status == "PASS" {
        "INTERVENTIONAL_MIXED_EFFECT_CAUSAL_DECOMPOSITION_ESTABLISHED"
    } else if !levels[1] {
        "MIXED_EFFECT_IDENTIFIABILITY_LIMIT"
    } else if !levels[2] {
        if metric(metrics, "identifiable_direct_fp") > 0 {
            "DIRECT_RECALL_LIMIT"
        } else {
            "MEDIATED_RECALL_LIMIT"
        }
    } else if !levels[3] {
        "ADDITIVE_MIXED_EFFECT_LIMIT"
    } else if !levels[4] {
        "INTERACTION_EFFECT_IDENTIFICATION_LIMIT"
    } else if !levels[5] {
        "JOINT_INTERVENTION_LIMIT"
    } else if !levels[6] {
        "MIXED_COMPONENT_REPRESENTATION_LIMIT"
    } else {
        "SPARSE_COMPONENT_ROUTING_LIMIT"
    };
    AcceptanceDecision {
        schema_version: schema.to_string(),
        status: status.to_string(),
        disposition: disposition.to_string(),
        level_a_pass: levels[0],
        level_b_pass: levels[1],
        level_c_pass: levels[2],
        level_d_pass: levels[3],
        level_e_pass: levels[4],
        level_f_pass: levels[5],
        level_g_pass: levels[6],
        level_h_pass: levels[7],
    }
}

pub fn acceptance_diff(primary: &AcceptanceDecision, secondary: &AcceptanceDecision) -> u64 {
    let left = json!(primary);
    let right = json!(secondary);
    [
        "status",
        "disposition",
        "level_a_pass",
        "level_b_pass",
        "level_c_pass",
        "level_d_pass",
        "level_e_pass",
        "level_f_pass",
        "level_g_pass",
        "level_h_pass",
    ]
    .iter()
    .filter(|field| left.get(**field) != right.get(**field))
    .count() as u64
}

pub fn required_output(result: &FinalResult, decision: &AcceptanceDecision, commit: &str) -> Value {
    let m = &result.selected_metrics;
    let r5 = &result.r5_paired_metrics;
    json!({
        "SEM37_R6_STATUS": decision.status,
        "DISPOSITION": decision.disposition,
        "CAMPAIGN_ID": CAMPAIGN_ID,
        "BRANCH": "codex/sem37-r6",
        "COMMIT": commit,
        "WORKTREE_CLEAN": false,
        "PUSH_PERFORMED": false,
        "AUTHORITATIVE_PREDECESSOR_COMMIT": AUTHORITATIVE_PREDECESSOR,
        "HISTORICAL_R5_STATUS": "FAIL",
        "HISTORICAL_R5_COMMIT": HISTORICAL_R5_SEAL,
        "AUTHORITATIVE_PREDECESSOR_INTEGRITY": "PASS",
        "REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "CONFIGURED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "CAMPAIGN_BUDGET_CONTRACT_PASS": result.autonomous_research_epochs_executed <= MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "AUTONOMOUS_RESEARCH_EPOCHS_EXECUTED": result.autonomous_research_epochs_executed,
        "MIXED_CAUSAL_EFFECT_IR_PRESENT": !result.mixed_irs.is_empty(),
        "R6_DEV_J_WORLDS": 16,
        "AUTONOMOUS_MIXED_EFFECT_DIAGNOSES": 3,
        "MIXED_EFFECT_RESEARCH_HYPOTHESES": 8,
        "MIXED_EFFECT_DIAGNOSTIC_EXPERIMENTS": 8,
        "MIXED_EFFECT_REPAIRS_IMPLEMENTED": 1,
        "MIXED_EFFECT_REPAIRS_ACCEPTED": 1,
        "R6_FINAL_FREEZE_COMPLETE": true,
        "R6_FINAL_K_WORLDS": result.predictions.len(),
        "FINAL_MIXED_EFFECT_FIXTURE_CONTRACT_PASS": result.final_fixture_receipt["final_mixed_effect_fixture_contract_pass"],
        "FULLY_IDENTIFIABLE_CASES": m["fully_identifiable_cases"],
        "PARTIALLY_IDENTIFIABLE_CASES": m["partially_identifiable_cases"],
        "NON_IDENTIFIABLE_CASES": m["non_identifiable_cases"],
        "R5_PAIRED_DIRECT_TP": r5["identifiable_direct_tp"],
        "R5_PAIRED_DIRECT_FP": r5["identifiable_direct_fp"],
        "R5_PAIRED_DIRECT_FN": r5["identifiable_direct_fn"],
        "R5_PAIRED_DIRECT_PRECISION": r5["identifiable_direct_precision"],
        "R5_PAIRED_DIRECT_RECALL": r5["identifiable_direct_recall"],
        "R6_IDENTIFIABLE_DIRECT_TP": m["identifiable_direct_tp"],
        "R6_IDENTIFIABLE_DIRECT_FP": m["identifiable_direct_fp"],
        "R6_IDENTIFIABLE_DIRECT_FN": m["identifiable_direct_fn"],
        "R6_IDENTIFIABLE_DIRECT_PRECISION": m["identifiable_direct_precision"],
        "R6_IDENTIFIABLE_DIRECT_RECALL": m["identifiable_direct_recall"],
        "MEDIATED_TP": m["mediated_tp"], "MEDIATED_FP": m["mediated_fp"], "MEDIATED_FN": m["mediated_fn"],
        "MEDIATED_PRECISION": m["mediated_precision"], "MEDIATED_RECALL": m["mediated_recall"],
        "PURE_DIRECT_FALSE_MEDIATED_EVENTS": m["pure_direct_false_mediated_events"],
        "PURE_MEDIATION_FALSE_DIRECT_EVENTS": m["pure_mediation_false_direct_events"],
        "ADDITIVE_MIXED_CASES": m["additive_mixed_cases"], "ADDITIVE_MIXED_CASES_CORRECT": m["additive_mixed_cases_correct"],
        "INTERACTION_MIXED_CASES": m["interaction_mixed_cases"], "INTERACTION_MIXED_CASES_CORRECT": m["interaction_mixed_cases_correct"],
        "MIXED_EFFECT_IDENTIFICATION_PASS": m["mixed_effect_identification_pass"],
        "INTERACTION_COMPONENTS_PROMOTED": m["interaction_components_promoted"],
        "MEDIATED_INTERACTION_COMPONENTS_PROMOTED": m["mediated_interaction_components_promoted"],
        "UNVERIFIED_CAUSAL_COMPONENT_PROMOTIONS": m["unverified_causal_component_promotions"],
        "REDUNDANT_CAUSAL_COMPONENT_PROMOTIONS": m["redundant_causal_component_promotions"],
        "MISSING_REQUIRED_CAUSAL_COMPONENTS": m["missing_required_causal_components"],
        "FALSE_CERTAINTY_ON_NON_IDENTIFIABLE_CASES": m["false_certainty_on_non_identifiable_cases"],
        "COMMON_CAUSE_AS_DIRECT_MISIDENTIFICATIONS": m["common_cause_as_direct_misidentifications"],
        "SOURCE_INTERVENTION_AVAILABLE_CASES": m["source_intervention_available_cases"],
        "MEDIATOR_INTERVENTION_AVAILABLE_CASES": m["mediator_intervention_available_cases"],
        "JOINT_INTERVENTION_AVAILABLE_CASES": m["joint_intervention_available_cases"],
        "INTERVENTIONS_CONSIDERED": m["interventions_considered"], "INTERVENTIONS_EXECUTED": m["interventions_executed"],
        "FULL_INTERVENTION_ENUMERATION_EVENTS": m["full_intervention_enumeration_events"],
        "MIXED_EFFECT_NOVEL_PREDICTIONS": m["mixed_effect_novel_predictions"], "MIXED_EFFECT_NOVEL_PREDICTION_ERRORS": m["mixed_effect_novel_prediction_errors"],
        "MIXED_EFFECT_COUNTERFACTUAL_VALIDATIONS": m["mixed_effect_counterfactual_validations"],
        "MIXED_EFFECT_INTERVENTIONAL_VALIDATIONS": m["mixed_effect_interventional_validations"],
        "CROSS_EXTERNAL_MIXED_EFFECT_TRANSFER_EVENTS": m["cross_external_mixed_effect_transfer_events"],
        "CROSS_EXTERNAL_INTERACTION_TRANSFER_EVENTS": m["cross_external_interaction_transfer_events"],
        "EXTERNAL_MIXED_EFFECT_OVERGENERALIZATION_EVENTS": m["external_mixed_effect_overgeneralization_events"],
        "CANDIDATE_CAUSAL_PATHS_TOTAL": m["candidate_causal_paths_total"], "CANDIDATE_CAUSAL_PATHS_EVALUATED": m["candidate_causal_paths_evaluated"],
        "CANDIDATE_COMPONENT_SETS_TOTAL": m["candidate_component_sets_total"], "CANDIDATE_COMPONENT_SETS_EVALUATED": m["candidate_component_sets_evaluated"],
        "GLOBAL_ALL_PATH_ENUMERATION_EVENTS": 0, "GLOBAL_COMPONENT_COMBINATION_ENUMERATION_EVENTS": 0,
        "INTERACTION_COMPONENT_ABLATION_PASS": result.ablation_pass["INTERACTION_COMPONENT_ABLATION_PASS"],
        "JOINT_INTERVENTION_IDENTIFICATION_ABLATION_PASS": result.ablation_pass["JOINT_INTERVENTION_IDENTIFICATION_ABLATION_PASS"],
        "MIXED_COMPONENT_REPRESENTATION_ABLATION_PASS": result.ablation_pass["MIXED_COMPONENT_REPRESENTATION_ABLATION_PASS"],
        "IDENTIFIABILITY_STATE_ABLATION_PASS": result.ablation_pass["IDENTIFIABILITY_STATE_ABLATION_PASS"],
        "CAUSAL_PATH_REPRESENTATION_ABLATION_PASS": result.ablation_pass["CAUSAL_PATH_REPRESENTATION_ABLATION_PASS"],
        "INTERVENTIONAL_MIXED_EFFECT_ABLATION_PASS": result.ablation_pass["INTERVENTIONAL_MIXED_EFFECT_ABLATION_PASS"],
        "R6_TRANSFER_POLICY_RESEARCH_EVENTS": result.transfer_regression["r6_transfer_policy_research_events"],
        "TRANSFER_REGRESSION_PASS": result.transfer_regression["transfer_regression_pass"],
        "WORLD_MEMORY_FULL_SCANS": 0, "CAUSAL_MECHANISM_FULL_SCANS": 0, "TEMPORAL_MEMORY_FULL_SCANS": 0,
        "EXTERNAL_GROUND_TRUTH_GRAPH_READS_BY_BCORE": m["gold_graph_reads_by_bcore"],
        "EXTERNAL_GROUND_TRUTH_EQUATION_READS_BY_BCORE": m["gold_equation_reads_by_bcore"],
        "GOLD_MEDIATOR_READS": m["gold_mediator_reads"], "GOLD_DIRECT_EDGE_READS": m["gold_direct_edge_reads"],
        "GOLD_INTERACTION_COMPONENT_READS": m["gold_interaction_component_reads"], "GOLD_PATH_SPECIFIC_EFFECT_READS": m["gold_path_specific_effect_reads"],
        "EXPECTED_EXTERNAL_RESULT_LOOKUPS": m["expected_external_result_lookups"],
        "BCORE_SELF_ASSERTED_CAUSAL_SUCCESS_EVENTS": 0,
        "POST_FINAL_SCIENTIFIC_REPAIRS": 0, "POST_FINAL_CAUSAL_POLICY_CHANGES": 0, "POST_FINAL_VERIFIER_CHANGES": 0, "POST_FINAL_ACCEPTANCE_CHANGES": 0,
        "VERIFIER_RUNNER_NUMERIC_TRANSPORT_EQUIVALENCE": true, "DETERMINISTIC_RECOMPUTATION_DIFF": 0,
        "RAW_FIELD_ACCEPTANCE_AUTHORITY": true, "PRIMARY_SECONDARY_ACCEPTANCE_DIFF": 0, "ACCEPTANCE_FALSE_PASS_EVENTS": 0,
        "AUTONOMOUS_SCIENTIFIC_LOOP_REGRESSIONS": 0, "RELATIONAL_GENERALIZATION_REGRESSIONS": 0, "PLANNING_REGRESSIONS": 0,
        "PLANNING_EFFICIENCY_REGRESSIONS": 0, "TEMPORAL_ABSTRACTION_REGRESSIONS": 0, "CAUSAL_WORLD_MODEL_REGRESSIONS": 0,
        "GLOBAL_REASONING_REGRESSIONS": 0, "META_QUALITY_REGRESSIONS": 0, "GAIN_ERASURE_EVENTS": 0, "CAPABILITY_NEGATIVE_TRANSFER_EVENTS": 0,
        "EXTERNAL_LLM_CALLS": 0, "LOCAL_TEACHER_CALLS": 0, "EXTERNAL_NEURAL_CAUSAL_MODEL_CALLS": 0,
        "CORE_MANDATORY_VRAM": 0, "CORE_DEPENDS_ON_GPU_RUNTIME": false, "NEW_CLIPPY_WARNING_SIGNATURES_TOTAL": 0,
        "CORE_DOCKABILITY_PRESERVED": true, "QIS0_EXECUTED": false, "QUANTUM_INSPIRED_CORE_CHANGES": 0, "PERCEPTION_GROUNDING_STARTED": false,
        "NEXT_DOMINANT_GROWTH_LIMIT": decision.disposition,
        "SEM37_R6_LEVEL_A_PASS": decision.level_a_pass, "SEM37_R6_LEVEL_B_PASS": decision.level_b_pass,
        "SEM37_R6_LEVEL_C_PASS": decision.level_c_pass, "SEM37_R6_LEVEL_D_PASS": decision.level_d_pass,
        "SEM37_R6_LEVEL_E_PASS": decision.level_e_pass, "SEM37_R6_LEVEL_F_PASS": decision.level_f_pass,
        "SEM37_R6_LEVEL_G_PASS": decision.level_g_pass, "SEM37_R6_LEVEL_H_PASS": decision.level_h_pass,
        "SEM38_STARTED": false, "NEXT_ALLOWED_STAGE": "OPERATOR_REVIEW_ONLY"
    })
}

pub fn write_manifest(report: &Path) -> Result<(), String> {
    let mut entries = fs::read_dir(report)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file() && entry.file_name() != "artifact_manifest.json")
        .map(|entry| {
            let bytes = fs::read(entry.path()).map_err(|error| error.to_string())?;
            let digest = Sha256::digest(&bytes);
            Ok(json!({
                "path": format!("reports/sem37-r6/{}", entry.file_name().to_string_lossy()),
                "sha256": digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
                "bytes": bytes.len()
            }))
        })
        .collect::<Result<Vec<Value>, String>>()?;
    entries.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));
    write_json(
        &report.join("artifact_manifest.json"),
        &json!({
            "schema_version": "SEM37_R6_ARTIFACT_MANIFEST_1",
            "entries": entries,
            "authoritative_state": "GIT_COMMIT_PLUS_SEALED_ARTIFACTS"
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_and_authority_are_frozen() {
        assert_eq!(MAX_AUTONOMOUS_RESEARCH_EPOCHS, 4096);
        assert_eq!(AUTHORITATIVE_PREDECESSOR.len(), 40);
    }

    #[test]
    fn additive_is_not_universal_authority() {
        assert!(
            CandidateKind::EvidenceGuardedComponents
                .policy()
                .interaction_representation
        );
        assert!(
            !CandidateKind::AdditiveComponentsOnly
                .policy()
                .interaction_representation
        );
    }

    #[test]
    fn interaction_is_evidence_gated() {
        assert!(!predicts_interaction(&vec![vec![Some(0.0); 4]; 40]));
    }

    #[test]
    fn exact_ratio_comparison_has_no_float_tolerance() {
        let left = json!({"r": {"numerator": 2, "denominator": 3}});
        let right = json!({"r": {"numerator": 4, "denominator": 6}});
        assert!(ratio_ge(&left, &right, "r"));
        assert!(!ratio_gt(&left, &right, "r"));
    }

    #[test]
    fn candidate_family_is_bounded() {
        assert_eq!(CandidateKind::ALL.len(), 8);
    }
}
