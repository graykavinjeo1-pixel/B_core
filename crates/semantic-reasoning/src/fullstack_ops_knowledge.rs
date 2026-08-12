//! Compact frontend, backend, and operations knowledge absorbed from the
//! corrected Synapse coding-knowledge generation.
//!
//! The source universe remains an evidence store, not runtime authority. This
//! module retains only content-addressed source-family receipts, normalized
//! semantic atoms, and typed composition recipes. It copies no source code,
//! documentation body, benchmark answer, or repository-specific patch.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::code_graft::{file_sha256, write_json, Element};
use crate::self_healing_pipeline::{
    validate_composition_lesson, CompositionEdgeIR, RepairCompositionLessonIR, RepairPrimitiveIR,
};
use crate::self_repair_contract::sha256;

pub const CAMPAIGN_ID: &str = "B_CORE-CODE-GRAFT-04";
pub const SOURCE_UNIVERSE_SHA256: &str =
    "BF9F10C31F6504473050028280C1DCB22AC54CFD29DFC9215EC29DDD42BBE52C";
pub const SOURCE_PREDECESSOR_UNIVERSE_SHA256: &str =
    "CBAF8D5548446D1D3165E4712A450F56A529AABB929AE757EAAC59596E51140C";
pub const SOURCE_MANIFEST_SHA256: &str =
    "FDE633C7BF219D34C29DCE5BA3C4D763840762BF36950D65692605F3ED2E5F2B";
pub const SOURCE_RECORDS_SHA256: &str =
    "6BDE8E5722B284181521959995F86FEA2B0EAEBAC9B011A896A52360FFDA15F8";
pub const MAX_ACTIVE_ATOMS: usize = 8;
pub const SOURCE_FAMILY_CLASSIFIER_VERSION: &str = "FULLSTACK_SOURCE_FAMILY_V1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CodingLayer {
    Frontend,
    Backend,
    Operations,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Capability {
    UserEventNormalization,
    StateProjection,
    AsyncTransport,
    ProtocolValidation,
    Authorization,
    TransactionalExecution,
    ErrorMapping,
    ResponseReconciliation,
    ReleaseContract,
    CanaryDeployment,
    HealthGating,
    Rollback,
    TelemetryCorrelation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFamilyReceipt {
    pub layer: CodingLayer,
    pub classifier_version: String,
    pub source_count: usize,
    pub source_set_sha256: String,
    pub expected_source_count: usize,
    pub expected_source_set_sha256: String,
    pub matches_expected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeAtom {
    pub atom_id: String,
    pub layer: CodingLayer,
    pub capabilities: Vec<Capability>,
    pub mechanism_ids: Vec<String>,
    pub elemental_operations: Vec<Element>,
    pub input_contract: String,
    pub output_contract: String,
    pub semantic_role: String,
    pub source_family_sha256: String,
    pub source_universe_sha256: String,
    pub exact_source_fragment_present: bool,
    pub natural_language_is_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionRecipe {
    pub recipe_id: String,
    pub required_layers: Vec<CodingLayer>,
    pub capabilities: Vec<Capability>,
    pub ordered_atom_ids: Vec<String>,
    pub invariants: Vec<String>,
    pub abstain_when: Vec<String>,
    pub verification_obligations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullStackKnowledgeBundle {
    pub schema: String,
    pub campaign_id: String,
    pub source_universe_sha256: String,
    pub source_predecessor_universe_sha256: String,
    pub source_manifest_projection_correction: bool,
    pub source_semantics_mutated: bool,
    pub source_family_receipts: Vec<SourceFamilyReceipt>,
    pub atoms: Vec<KnowledgeAtom>,
    pub recipes: Vec<CompositionRecipe>,
    pub max_active_atoms: usize,
    pub sparse_activation_required: bool,
    pub raw_source_copied: bool,
    pub external_llm_calls: usize,
    pub codex_runtime_calls: usize,
    pub network_reads: usize,
    pub network_writes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeQuery {
    pub required_layers: Vec<CodingLayer>,
    pub required_capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeActivation {
    pub selected_recipe_ids: Vec<String>,
    pub active_atom_ids: Vec<String>,
    pub active_atom_count: usize,
    pub max_active_atoms: usize,
    pub full_knowledge_scans: usize,
    pub task_identity_routing_events: usize,
    pub repository_identity_routing_events: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullStackBehavioralExecution {
    pub recipe_id: String,
    pub input_contract: String,
    pub output_contract: String,
    pub input_payload_sha256: String,
    pub output_payload_sha256: String,
    pub executed_atom_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullStackBehavioralCanaryReceipt {
    pub schema: String,
    pub recipe_id: String,
    pub behavioral_artifact_sha256: String,
    pub cases_executed: usize,
    pub cases_passed: usize,
    pub exact_pipeline_observed: bool,
    pub wrong_input_contract_rejected: bool,
    pub reordered_pipeline_rejected: bool,
    pub execution: FullStackBehavioralExecution,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsorptionReport {
    pub schema: String,
    pub campaign_id: String,
    pub status: String,
    pub source_universe_sha256: String,
    pub predecessor_universe_sha256: String,
    pub manifest_projection_correction_absorbed: bool,
    pub source_semantics_changed: bool,
    pub source_family_count: usize,
    pub frontend_source_count: usize,
    pub backend_source_count: usize,
    pub operations_source_count: usize,
    pub promoted_knowledge_atoms: usize,
    pub promoted_composition_recipes: usize,
    pub three_layer_recipe_count: usize,
    pub activation_canaries_passed: usize,
    pub raw_training_data_files_copied: usize,
    pub raw_source_fragments_promoted: usize,
    pub external_llm_calls: usize,
    pub codex_runtime_calls: usize,
    pub network_reads: usize,
    pub network_writes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnowledgeValidationError {
    SourceFamilyMismatch(CodingLayer),
    MissingLayer(CodingLayer),
    DuplicateAtom,
    EmptyMechanismSet,
    EmptyElementalOperations,
    UnknownRecipeAtom,
    DuplicateRecipe,
    LayerDeclarationMismatch,
    CompositionInvalid,
    ActivationBoundExceeded,
    SourceFragmentPresent,
    NaturalLanguageAuthority,
    ExternalDependencyPresent,
}

#[derive(Clone, Copy)]
struct SourceFamilySpec {
    layer: CodingLayer,
    patterns: &'static [&'static str],
    expected_count: usize,
    expected_sha256: &'static str,
}

const SOURCE_FAMILY_SPECS: [SourceFamilySpec; 3] = [
    SourceFamilySpec {
        layer: CodingLayer::Frontend,
        patterns: &[
            "typescript official",
            "javascript",
            "svelte",
            "wasm-bindgen",
            "web-sys",
            "winit",
            "egui",
            "gtk",
            "iced",
            "leptos",
            "yew",
            "dioxus",
            "tauri",
        ],
        expected_count: 11,
        expected_sha256: "9D5BC43E560CB4C9AA991A9ACDC529DF3308A6550F66B01F5DD5F59155261254",
    },
    SourceFamilySpec {
        layer: CodingLayer::Backend,
        patterns: &[
            "aspnet",
            "jsonrpsee",
            "tonic",
            "axum",
            "actix",
            "crates.io hyper ",
            "grpc",
            "database",
            "sqlx",
            "postgres",
            "mysql",
            "redis",
            "django",
            "spring",
            "http11",
            "http-body",
        ],
        expected_count: 26,
        expected_sha256: "E24056EACD52ED3561C41ED4392148F97C0D476C05AE9ECAD9B36EC28CBA8CBD",
    },
    SourceFamilySpec {
        layer: CodingLayer::Operations,
        patterns: &[
            "opentelemetry",
            "prometheus",
            "grafana",
            "terraform",
            "logging",
            "metrics",
            "alertmanager",
            "collector",
            "telemetry",
            "tracing",
        ],
        expected_count: 68,
        expected_sha256: "362EDC8BD51D8075C59BFDDDAF7A343F82428C675EF7C60D6C14887ECACAD84A",
    },
];

/// Reconstructs the compact promoted bundle from sealed family receipts only.
/// No source corpus or report directory is needed at runtime.
pub fn promoted_bundle() -> FullStackKnowledgeBundle {
    build_bundle(
        SOURCE_FAMILY_SPECS
            .iter()
            .map(|spec| SourceFamilyReceipt {
                layer: spec.layer,
                classifier_version: SOURCE_FAMILY_CLASSIFIER_VERSION.to_string(),
                source_count: spec.expected_count,
                source_set_sha256: spec.expected_sha256.to_ascii_lowercase(),
                expected_source_count: spec.expected_count,
                expected_source_set_sha256: spec.expected_sha256.to_string(),
                matches_expected: true,
            })
            .collect(),
    )
}

fn source_family_hashes(source_records_path: &Path) -> Result<Vec<SourceFamilyReceipt>, String> {
    let actual_hash = file_sha256(source_records_path)?;
    if !actual_hash.eq_ignore_ascii_case(SOURCE_RECORDS_SHA256) {
        return Err(format!("SOURCE_RECORDS_HASH_MISMATCH:{actual_hash}"));
    }
    let input = fs::read_to_string(source_records_path)
        .map_err(|error| format!("SOURCE_RECORDS_READ:{error}"))?;
    let mut family_hashes = SOURCE_FAMILY_SPECS
        .iter()
        .map(|spec| (spec.layer, BTreeSet::<String>::new()))
        .collect::<BTreeMap<_, _>>();
    for (index, line) in input.lines().enumerate() {
        let value: Value = serde_json::from_str(line)
            .map_err(|error| format!("SOURCE_RECORD_JSON:{}:{error}", index + 1))?;
        let source_sha = value["canonical_source_sha256"]
            .as_str()
            .ok_or_else(|| format!("SOURCE_RECORD_SHA_MISSING:{}", index + 1))?;
        let title = value["title"].as_str().unwrap_or_default();
        let url = value["url"].as_str().unwrap_or_default();
        let searchable = format!("{title} {url}").to_ascii_lowercase();
        for spec in SOURCE_FAMILY_SPECS {
            if spec
                .patterns
                .iter()
                .any(|pattern| searchable.contains(pattern))
            {
                family_hashes
                    .get_mut(&spec.layer)
                    .expect("all source families initialized")
                    .insert(source_sha.to_string());
            }
        }
    }

    Ok(SOURCE_FAMILY_SPECS
        .iter()
        .map(|spec| {
            let hashes = &family_hashes[&spec.layer];
            let joined = hashes.iter().cloned().collect::<Vec<_>>().join("\n");
            let digest = sha256(joined.as_bytes());
            SourceFamilyReceipt {
                layer: spec.layer,
                classifier_version: SOURCE_FAMILY_CLASSIFIER_VERSION.to_string(),
                source_count: hashes.len(),
                source_set_sha256: digest.clone(),
                expected_source_count: spec.expected_count,
                expected_source_set_sha256: spec.expected_sha256.to_string(),
                matches_expected: hashes.len() == spec.expected_count
                    && digest.eq_ignore_ascii_case(spec.expected_sha256),
            }
        })
        .collect())
}

fn source_family(receipts: &[SourceFamilyReceipt], layer: CodingLayer) -> String {
    receipts
        .iter()
        .find(|receipt| receipt.layer == layer)
        .expect("validated layer receipt")
        .source_set_sha256
        .clone()
}

#[allow(clippy::too_many_arguments)]
fn atom(
    receipts: &[SourceFamilyReceipt],
    atom_id: &str,
    layer: CodingLayer,
    capabilities: &[Capability],
    mechanism_ids: &[&str],
    operations: &[Element],
    input_contract: &str,
    output_contract: &str,
    semantic_role: &str,
) -> KnowledgeAtom {
    KnowledgeAtom {
        atom_id: atom_id.to_string(),
        layer,
        capabilities: capabilities.to_vec(),
        mechanism_ids: mechanism_ids
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        elemental_operations: operations.to_vec(),
        input_contract: input_contract.to_string(),
        output_contract: output_contract.to_string(),
        semantic_role: semantic_role.to_string(),
        source_family_sha256: source_family(receipts, layer),
        source_universe_sha256: SOURCE_UNIVERSE_SHA256.to_string(),
        exact_source_fragment_present: false,
        natural_language_is_authority: false,
    }
}

pub fn build_bundle(receipts: Vec<SourceFamilyReceipt>) -> FullStackKnowledgeBundle {
    use Capability::*;
    use CodingLayer::*;
    use Element::*;

    let atoms = vec![
        atom(
            &receipts,
            "FSK-FE-EVENT-NORMALIZATION",
            Frontend,
            &[UserEventNormalization],
            &["event_loop", "pattern_matching"],
            &[Read, Parse, Branch, TypeCheck],
            "RAW_USER_EVENT",
            "VALIDATED_INTERACTION",
            "boundary_validation",
        ),
        atom(
            &receipts,
            "FSK-FE-STATE-PROJECTION",
            Frontend,
            &[StateProjection],
            &["algebraic_data_type", "type_inference"],
            &[Read, Branch, TypeCheck, Serialize],
            "VALIDATED_INTERACTION",
            "CLIENT_REQUEST",
            "state_projection",
        ),
        atom(
            &receipts,
            "FSK-FE-ASYNC-TRANSPORT",
            Frontend,
            &[AsyncTransport],
            &["async_await", "result_value"],
            &[Call, Suspend, Resume, PropagateError],
            "CLIENT_REQUEST",
            "TRANSPORT_REQUEST",
            "async_failure_boundary",
        ),
        atom(
            &receipts,
            "FSK-BE-PROTOCOL-GATE",
            Backend,
            &[ProtocolValidation, Authorization],
            &["interface_protocol", "static_typing"],
            &[Parse, TypeCheck, Verify, Branch],
            "TRANSPORT_REQUEST",
            "AUTHORIZED_COMMAND",
            "contract_enforcement",
        ),
        atom(
            &receipts,
            "FSK-BE-TRANSACTION",
            Backend,
            &[TransactionalExecution],
            &["result_value", "relational_query"],
            &[Read, Write, Verify, Recover],
            "AUTHORIZED_COMMAND",
            "DOMAIN_RESULT",
            "transactional_execution",
        ),
        atom(
            &receipts,
            "FSK-BE-ERROR-MAP",
            Backend,
            &[ErrorMapping],
            &["result_value", "exception"],
            &[Branch, PropagateError, Recover, Serialize],
            "DOMAIN_RESULT",
            "SERVICE_RESPONSE",
            "error_contract_mapping",
        ),
        atom(
            &receipts,
            "FSK-FE-RESPONSE-RECONCILE",
            Frontend,
            &[ResponseReconciliation],
            &["pattern_matching", "algebraic_data_type"],
            &[Parse, Branch, Write, Verify],
            "SERVICE_RESPONSE",
            "STABLE_VIEW_STATE",
            "response_reconciliation",
        ),
        atom(
            &receipts,
            "FSK-OPS-REQUEST-TELEMETRY",
            Operations,
            &[TelemetryCorrelation],
            &["interface_protocol", "message_passing"],
            &[Serialize, Dispatch, Verify, Write],
            "SERVICE_RESPONSE",
            "OBSERVED_OUTCOME",
            "telemetry_correlation",
        ),
        atom(
            &receipts,
            "FSK-OPS-CLIENT-TELEMETRY",
            Operations,
            &[TelemetryCorrelation],
            &["event_loop", "message_passing"],
            &[Read, Serialize, Dispatch, Verify],
            "STABLE_VIEW_STATE",
            "OBSERVED_OUTCOME",
            "client_outcome_correlation",
        ),
        atom(
            &receipts,
            "FSK-FE-RELEASE-CONTRACT",
            Frontend,
            &[ReleaseContract],
            &["build_tool", "package_manager"],
            &[Parse, Compile, Link, Verify],
            "RELEASE_CANDIDATE",
            "RELEASE_CANDIDATE",
            "frontend_release_contract",
        ),
        atom(
            &receipts,
            "FSK-BE-RELEASE-CONTRACT",
            Backend,
            &[ReleaseContract],
            &["build_tool", "interface_protocol"],
            &[Parse, Compile, Verify, Serialize],
            "RELEASE_CANDIDATE",
            "RELEASE_CANDIDATE",
            "backend_release_contract",
        ),
        atom(
            &receipts,
            "FSK-OPS-CANARY",
            Operations,
            &[CanaryDeployment],
            &["package_manager", "shell_pipeline"],
            &[Verify, Dispatch, Synchronize, Read],
            "RELEASE_CANDIDATE",
            "CANARY_OBSERVATION",
            "bounded_canary",
        ),
        atom(
            &receipts,
            "FSK-OPS-HEALTH-GATE",
            Operations,
            &[HealthGating],
            &["result_value", "debugger_tooling"],
            &[Read, Verify, Branch, PropagateError],
            "CANARY_OBSERVATION",
            "RELEASE_DECISION",
            "health_gate",
        ),
        atom(
            &receipts,
            "FSK-OPS-ROLLBACK",
            Operations,
            &[Rollback],
            &["result_value", "package_manager"],
            &[Branch, Recover, Verify, Synchronize],
            "RELEASE_DECISION",
            "STABLE_RELEASE",
            "rollback_guard",
        ),
    ];

    let recipes = vec![
        CompositionRecipe {
            recipe_id: "FULLSTACK-REQUEST-TO-OBSERVABILITY-V1".to_string(),
            required_layers: vec![Frontend, Backend, Operations],
            capabilities: vec![
                UserEventNormalization,
                StateProjection,
                AsyncTransport,
                ProtocolValidation,
                Authorization,
                TransactionalExecution,
                ErrorMapping,
                TelemetryCorrelation,
            ],
            ordered_atom_ids: vec![
                "FSK-FE-EVENT-NORMALIZATION",
                "FSK-FE-STATE-PROJECTION",
                "FSK-FE-ASYNC-TRANSPORT",
                "FSK-BE-PROTOCOL-GATE",
                "FSK-BE-TRANSACTION",
                "FSK-BE-ERROR-MAP",
                "FSK-OPS-REQUEST-TELEMETRY",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            invariants: vec![
                "validate before authority-bearing execution".to_string(),
                "preserve one typed error contract across the boundary".to_string(),
                "correlate outcome without making telemetry business authority".to_string(),
            ],
            abstain_when: vec![
                "the public protocol contract is unavailable".to_string(),
                "authorization or data-ownership policy is unfrozen".to_string(),
            ],
            verification_obligations: vec![
                "frontend state and protocol contract tests".to_string(),
                "backend transaction and error-mapping tests".to_string(),
                "trace-correlation and failure-path tests".to_string(),
            ],
        },
        CompositionRecipe {
            recipe_id: "FULLSTACK-RESPONSE-CONSISTENCY-V1".to_string(),
            required_layers: vec![Frontend, Backend, Operations],
            capabilities: vec![
                TransactionalExecution,
                ErrorMapping,
                ResponseReconciliation,
                TelemetryCorrelation,
            ],
            ordered_atom_ids: vec![
                "FSK-BE-TRANSACTION",
                "FSK-BE-ERROR-MAP",
                "FSK-FE-RESPONSE-RECONCILE",
                "FSK-OPS-CLIENT-TELEMETRY",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            invariants: vec![
                "one service result maps to one explicit response variant".to_string(),
                "client reconciliation is deterministic and idempotent".to_string(),
            ],
            abstain_when: vec!["response variant ownership is ambiguous".to_string()],
            verification_obligations: vec![
                "fresh success and failure response variants".to_string(),
                "duplicate delivery reconciliation".to_string(),
                "client-visible outcome correlation".to_string(),
            ],
        },
        CompositionRecipe {
            recipe_id: "FULLSTACK-SAFE-DELIVERY-V1".to_string(),
            required_layers: vec![Frontend, Backend, Operations],
            capabilities: vec![ReleaseContract, CanaryDeployment, HealthGating, Rollback],
            ordered_atom_ids: vec![
                "FSK-FE-RELEASE-CONTRACT",
                "FSK-BE-RELEASE-CONTRACT",
                "FSK-OPS-CANARY",
                "FSK-OPS-HEALTH-GATE",
                "FSK-OPS-ROLLBACK",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            invariants: vec![
                "frontend and backend artifacts share one versioned release contract".to_string(),
                "promotion requires bounded canary evidence".to_string(),
                "rollback remains possible until health closure".to_string(),
            ],
            abstain_when: vec![
                "schema migration reversibility is unknown".to_string(),
                "health signals are not causally tied to the release".to_string(),
            ],
            verification_obligations: vec![
                "artifact and schema compatibility".to_string(),
                "canary health threshold replay".to_string(),
                "rollback restoration test".to_string(),
            ],
        },
    ];

    FullStackKnowledgeBundle {
        schema: "b_core_fullstack_ops_knowledge_bundle_v1".to_string(),
        campaign_id: CAMPAIGN_ID.to_string(),
        source_universe_sha256: SOURCE_UNIVERSE_SHA256.to_string(),
        source_predecessor_universe_sha256: SOURCE_PREDECESSOR_UNIVERSE_SHA256.to_string(),
        source_manifest_projection_correction: true,
        source_semantics_mutated: false,
        source_family_receipts: receipts,
        atoms,
        recipes,
        max_active_atoms: MAX_ACTIVE_ATOMS,
        sparse_activation_required: true,
        raw_source_copied: false,
        external_llm_calls: 0,
        codex_runtime_calls: 0,
        network_reads: 0,
        network_writes: 0,
    }
}

pub fn recipe_as_composition_lesson(
    bundle: &FullStackKnowledgeBundle,
    recipe_id: &str,
) -> Result<RepairCompositionLessonIR, String> {
    let recipe = bundle
        .recipes
        .iter()
        .find(|candidate| candidate.recipe_id == recipe_id)
        .ok_or_else(|| format!("RECIPE_NOT_FOUND:{recipe_id}"))?;
    let by_id = bundle
        .atoms
        .iter()
        .map(|atom| (atom.atom_id.as_str(), atom))
        .collect::<BTreeMap<_, _>>();
    let mut primitives = Vec::new();
    for atom_id in &recipe.ordered_atom_ids {
        let atom = by_id
            .get(atom_id.as_str())
            .ok_or_else(|| format!("RECIPE_ATOM_NOT_FOUND:{atom_id}"))?;
        primitives.push(RepairPrimitiveIR {
            primitive_id: atom.atom_id.clone(),
            implementation_anchor: format!("bcore://fullstack-knowledge/{}", atom.atom_id),
            input_type: atom.input_contract.clone(),
            output_type: atom.output_contract.clone(),
            semantic_role: atom.semantic_role.clone(),
        });
    }
    let edges = primitives
        .windows(2)
        .map(|pair| CompositionEdgeIR {
            from_primitive_id: pair[0].primitive_id.clone(),
            to_primitive_id: pair[1].primitive_id.clone(),
            transported_type: pair[0].output_type.clone(),
        })
        .collect::<Vec<_>>();
    Ok(RepairCompositionLessonIR {
        composition_id: recipe.recipe_id.clone(),
        required_semantic_roles: primitives
            .iter()
            .map(|primitive| primitive.semantic_role.clone())
            .collect(),
        execution_order: recipe.ordered_atom_ids.clone(),
        primitives,
        edges,
        applicability: recipe
            .capabilities
            .iter()
            .map(|capability| format!("{capability:?}"))
            .collect(),
        non_applicability: recipe.abstain_when.clone(),
        exact_source_fragment_present: false,
    })
}

fn execute_fullstack_atom_sequence(
    bundle: &FullStackKnowledgeBundle,
    recipe_id: &str,
    ordered_atom_ids: &[String],
    input_contract: &str,
    input_payload_sha256: &str,
) -> Result<FullStackBehavioralExecution, String> {
    if input_payload_sha256.len() != 64
        || !input_payload_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("FULLSTACK_BEHAVIOR_INPUT_PAYLOAD_INVALID".to_string());
    }
    let by_id = bundle
        .atoms
        .iter()
        .map(|atom| (atom.atom_id.as_str(), atom))
        .collect::<BTreeMap<_, _>>();
    let mut current_contract = input_contract.to_string();
    let mut current_payload_sha256 = input_payload_sha256.to_ascii_lowercase();
    let mut executed_atom_ids = Vec::with_capacity(ordered_atom_ids.len());
    for atom_id in ordered_atom_ids {
        let atom = by_id
            .get(atom_id.as_str())
            .ok_or_else(|| format!("FULLSTACK_BEHAVIOR_ATOM_MISSING:{atom_id}"))?;
        if atom.input_contract != current_contract {
            return Err(format!(
                "FULLSTACK_BEHAVIOR_CONTRACT_MISMATCH:{}:{}:{}",
                atom.atom_id, current_contract, atom.input_contract
            ));
        }
        let atom_bytes = serde_json::to_vec(atom)
            .map_err(|error| format!("FULLSTACK_BEHAVIOR_ATOM_SERIALIZE:{error}"))?;
        current_payload_sha256 = sha256(
            format!(
                "{}:{}:{}:{}",
                current_payload_sha256,
                atom.atom_id,
                atom.output_contract,
                sha256(&atom_bytes)
            )
            .as_bytes(),
        );
        current_contract = atom.output_contract.clone();
        executed_atom_ids.push(atom.atom_id.clone());
    }
    Ok(FullStackBehavioralExecution {
        recipe_id: recipe_id.to_string(),
        input_contract: input_contract.to_string(),
        output_contract: current_contract,
        input_payload_sha256: input_payload_sha256.to_ascii_lowercase(),
        output_payload_sha256: current_payload_sha256,
        executed_atom_ids,
    })
}

/// Executes a promoted full-stack recipe as a typed dataflow program and
/// falsifies it with a wrong input contract and an atom-order counterexample.
/// This is deliberately repository-independent: the capability artifact is
/// the executable cross-layer transition law, not a copied framework patch.
pub fn execute_fullstack_recipe_behavioral_canary(
    bundle: &FullStackKnowledgeBundle,
    recipe_id: &str,
) -> Result<FullStackBehavioralCanaryReceipt, String> {
    validate_bundle(bundle).map_err(|error| format!("FULLSTACK_BEHAVIOR_BUNDLE:{error:?}"))?;
    let recipe = bundle
        .recipes
        .iter()
        .find(|recipe| recipe.recipe_id == recipe_id)
        .ok_or_else(|| format!("FULLSTACK_BEHAVIOR_RECIPE_MISSING:{recipe_id}"))?;
    let lesson = recipe_as_composition_lesson(bundle, recipe_id)?;
    validate_composition_lesson(&lesson)
        .map_err(|error| format!("FULLSTACK_BEHAVIOR_LESSON:{error:?}"))?;
    let first_atom = bundle
        .atoms
        .iter()
        .find(|atom| recipe.ordered_atom_ids.first() == Some(&atom.atom_id))
        .ok_or_else(|| "FULLSTACK_BEHAVIOR_FIRST_ATOM_MISSING".to_string())?;
    let last_atom = bundle
        .atoms
        .iter()
        .find(|atom| recipe.ordered_atom_ids.last() == Some(&atom.atom_id))
        .ok_or_else(|| "FULLSTACK_BEHAVIOR_LAST_ATOM_MISSING".to_string())?;
    let input_payload_sha256 = sha256(format!("{recipe_id}:FRESH_INPUT").as_bytes());
    let execution = execute_fullstack_atom_sequence(
        bundle,
        recipe_id,
        &recipe.ordered_atom_ids,
        &first_atom.input_contract,
        &input_payload_sha256,
    )?;
    let exact_pipeline_observed = execution.executed_atom_ids == recipe.ordered_atom_ids
        && execution.output_contract == last_atom.output_contract
        && execution.output_payload_sha256 != execution.input_payload_sha256;
    let wrong_input_contract_rejected = execute_fullstack_atom_sequence(
        bundle,
        recipe_id,
        &recipe.ordered_atom_ids,
        "UNRELATED_CONTRACT",
        &input_payload_sha256,
    )
    .is_err();
    let mut reordered = recipe.ordered_atom_ids.clone();
    if reordered.len() >= 2 {
        // Rotate rather than merely swap adjacent atoms: some independent
        // release-contract atoms intentionally share one input/output type,
        // while moving the head behind the terminal transition must violate
        // the recipe's transported contract.
        reordered.rotate_left(1);
    }
    let reordered_pipeline_rejected = execute_fullstack_atom_sequence(
        bundle,
        recipe_id,
        &reordered,
        &first_atom.input_contract,
        &input_payload_sha256,
    )
    .is_err();
    let cases_executed = 3;
    let cases_passed = [
        exact_pipeline_observed,
        wrong_input_contract_rejected,
        reordered_pipeline_rejected,
    ]
    .into_iter()
    .filter(|passed| *passed)
    .count();
    let behavioral_artifact_sha256 = sha256(
        &serde_json::to_vec(&(
            &lesson,
            &execution.output_contract,
            &execution.executed_atom_ids,
        ))
        .map_err(|error| format!("FULLSTACK_BEHAVIOR_ARTIFACT_SERIALIZE:{error}"))?,
    );
    let mut receipt = FullStackBehavioralCanaryReceipt {
        schema: "B_CORE_FULLSTACK_BEHAVIORAL_CANARY_1".to_string(),
        recipe_id: recipe_id.to_string(),
        behavioral_artifact_sha256,
        cases_executed,
        cases_passed,
        exact_pipeline_observed,
        wrong_input_contract_rejected,
        reordered_pipeline_rejected,
        execution,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = sha256(
        &serde_json::to_vec(&receipt)
            .map_err(|error| format!("FULLSTACK_BEHAVIOR_RECEIPT_SERIALIZE:{error}"))?,
    );
    Ok(receipt)
}

pub fn validate_bundle(bundle: &FullStackKnowledgeBundle) -> Result<(), KnowledgeValidationError> {
    if bundle.raw_source_copied
        || bundle
            .atoms
            .iter()
            .any(|atom| atom.exact_source_fragment_present)
    {
        return Err(KnowledgeValidationError::SourceFragmentPresent);
    }
    if bundle
        .atoms
        .iter()
        .any(|atom| atom.natural_language_is_authority)
    {
        return Err(KnowledgeValidationError::NaturalLanguageAuthority);
    }
    if bundle.external_llm_calls != 0
        || bundle.codex_runtime_calls != 0
        || bundle.network_reads != 0
        || bundle.network_writes != 0
    {
        return Err(KnowledgeValidationError::ExternalDependencyPresent);
    }
    for layer in [
        CodingLayer::Frontend,
        CodingLayer::Backend,
        CodingLayer::Operations,
    ] {
        let receipt = bundle
            .source_family_receipts
            .iter()
            .find(|receipt| receipt.layer == layer)
            .ok_or(KnowledgeValidationError::MissingLayer(layer))?;
        let specification = SOURCE_FAMILY_SPECS
            .iter()
            .find(|specification| specification.layer == layer)
            .expect("all promoted layers have a sealed source-family specification");
        if !receipt.matches_expected
            || receipt.classifier_version != SOURCE_FAMILY_CLASSIFIER_VERSION
            || receipt.source_count != specification.expected_count
            || receipt.expected_source_count != specification.expected_count
            || !receipt
                .source_set_sha256
                .eq_ignore_ascii_case(specification.expected_sha256)
            || !receipt
                .expected_source_set_sha256
                .eq_ignore_ascii_case(specification.expected_sha256)
        {
            return Err(KnowledgeValidationError::SourceFamilyMismatch(layer));
        }
    }
    let mut atom_ids = BTreeSet::new();
    for atom in &bundle.atoms {
        if !atom_ids.insert(atom.atom_id.as_str()) {
            return Err(KnowledgeValidationError::DuplicateAtom);
        }
        if atom.mechanism_ids.is_empty() {
            return Err(KnowledgeValidationError::EmptyMechanismSet);
        }
        if atom.elemental_operations.is_empty() {
            return Err(KnowledgeValidationError::EmptyElementalOperations);
        }
    }
    let mut recipe_ids = BTreeSet::new();
    for recipe in &bundle.recipes {
        if !recipe_ids.insert(recipe.recipe_id.as_str()) {
            return Err(KnowledgeValidationError::DuplicateRecipe);
        }
        if recipe
            .ordered_atom_ids
            .iter()
            .any(|atom_id| !atom_ids.contains(atom_id.as_str()))
        {
            return Err(KnowledgeValidationError::UnknownRecipeAtom);
        }
        let actual_layers = recipe
            .ordered_atom_ids
            .iter()
            .filter_map(|atom_id| {
                bundle
                    .atoms
                    .iter()
                    .find(|atom| atom.atom_id == *atom_id)
                    .map(|atom| atom.layer)
            })
            .collect::<BTreeSet<_>>();
        let declared_layers = recipe
            .required_layers
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if actual_layers != declared_layers {
            return Err(KnowledgeValidationError::LayerDeclarationMismatch);
        }
        let lesson = recipe_as_composition_lesson(bundle, &recipe.recipe_id)
            .map_err(|_| KnowledgeValidationError::CompositionInvalid)?;
        validate_composition_lesson(&lesson)
            .map_err(|_| KnowledgeValidationError::CompositionInvalid)?;
    }
    Ok(())
}

pub fn activate(
    bundle: &FullStackKnowledgeBundle,
    query: &KnowledgeQuery,
) -> Result<KnowledgeActivation, KnowledgeValidationError> {
    validate_bundle(bundle)?;
    let required_layers = query
        .required_layers
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let required_capabilities = query
        .required_capabilities
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut selected = bundle
        .recipes
        .iter()
        .filter(|recipe| {
            let recipe_layers = recipe
                .required_layers
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            let recipe_capabilities = recipe.capabilities.iter().copied().collect::<BTreeSet<_>>();
            required_layers.is_subset(&recipe_layers)
                && required_capabilities.is_subset(&recipe_capabilities)
        })
        .collect::<Vec<_>>();
    selected.sort_by(|left, right| {
        left.ordered_atom_ids
            .len()
            .cmp(&right.ordered_atom_ids.len())
            .then_with(|| left.recipe_id.cmp(&right.recipe_id))
    });
    let selected = selected.into_iter().take(1).collect::<Vec<_>>();
    let active_atom_ids = selected
        .iter()
        .flat_map(|recipe| recipe.ordered_atom_ids.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if active_atom_ids.len() > bundle.max_active_atoms {
        return Err(KnowledgeValidationError::ActivationBoundExceeded);
    }
    Ok(KnowledgeActivation {
        selected_recipe_ids: selected
            .iter()
            .map(|recipe| recipe.recipe_id.clone())
            .collect(),
        active_atom_count: active_atom_ids.len(),
        active_atom_ids,
        max_active_atoms: bundle.max_active_atoms,
        full_knowledge_scans: 0,
        task_identity_routing_events: 0,
        repository_identity_routing_events: 0,
    })
}

fn validate_source_manifest(source_root: &Path) -> Result<Value, String> {
    let path = source_root.join("manifest.json");
    let actual_hash = file_sha256(&path)?;
    if !actual_hash.eq_ignore_ascii_case(SOURCE_MANIFEST_SHA256) {
        return Err(format!("SOURCE_MANIFEST_HASH_MISMATCH:{actual_hash}"));
    }
    let bytes = fs::read(&path).map_err(|error| format!("SOURCE_MANIFEST_READ:{error}"))?;
    let manifest: Value =
        serde_json::from_slice(&bytes).map_err(|error| format!("SOURCE_MANIFEST_JSON:{error}"))?;
    if manifest["knowledge_universe_sha256"].as_str() != Some(SOURCE_UNIVERSE_SHA256)
        || manifest["manifest_predecessor_universe_sha256"].as_str()
            != Some(SOURCE_PREDECESSOR_UNIVERSE_SHA256)
        || manifest["manifest_projection_correction"].as_u64() != Some(1)
        || manifest["manifest_projection_semantics_mutated"].as_u64() != Some(0)
        || manifest["canonical_programming_mechanism_count"].as_u64() != Some(61)
    {
        return Err("SOURCE_MANIFEST_CONTRACT_MISMATCH".to_string());
    }
    Ok(manifest)
}

fn activation_canaries(bundle: &FullStackKnowledgeBundle) -> Result<Vec<Value>, String> {
    use Capability::*;
    use CodingLayer::*;
    let queries = [
        KnowledgeQuery {
            required_layers: vec![Frontend, Backend, Operations],
            required_capabilities: vec![ProtocolValidation, TelemetryCorrelation],
        },
        KnowledgeQuery {
            required_layers: vec![Frontend, Backend, Operations],
            required_capabilities: vec![ResponseReconciliation],
        },
        KnowledgeQuery {
            required_layers: vec![Frontend, Backend, Operations],
            required_capabilities: vec![CanaryDeployment, Rollback],
        },
    ];
    queries
        .iter()
        .map(|query| {
            let activation =
                activate(bundle, query).map_err(|error| format!("ACTIVATION_CANARY:{error:?}"))?;
            if activation.selected_recipe_ids.len() != 1 || activation.active_atom_count < 3 {
                return Err("ACTIVATION_CANARY_EMPTY".to_string());
            }
            Ok(json!({
                "query": query,
                "activation": activation,
                "pass": true,
            }))
        })
        .collect()
}

pub fn run_absorption(report_dir: &Path, source_root: &Path) -> Result<AbsorptionReport, String> {
    fs::create_dir_all(report_dir).map_err(|error| format!("REPORT_DIR_CREATE:{error}"))?;
    let manifest = validate_source_manifest(source_root)?;
    let receipts = source_family_hashes(&source_root.join("sources.jsonl"))?;
    let bundle = build_bundle(receipts.clone());
    validate_bundle(&bundle).map_err(|error| format!("BUNDLE_VALIDATION:{error:?}"))?;
    let canaries = activation_canaries(&bundle)?;
    let frontend_source_count = receipts
        .iter()
        .find(|receipt| receipt.layer == CodingLayer::Frontend)
        .map(|receipt| receipt.source_count)
        .unwrap_or_default();
    let backend_source_count = receipts
        .iter()
        .find(|receipt| receipt.layer == CodingLayer::Backend)
        .map(|receipt| receipt.source_count)
        .unwrap_or_default();
    let operations_source_count = receipts
        .iter()
        .find(|receipt| receipt.layer == CodingLayer::Operations)
        .map(|receipt| receipt.source_count)
        .unwrap_or_default();
    let report = AbsorptionReport {
        schema: "b_core_fullstack_ops_absorption_report_v1".to_string(),
        campaign_id: CAMPAIGN_ID.to_string(),
        status: "PASS".to_string(),
        source_universe_sha256: SOURCE_UNIVERSE_SHA256.to_string(),
        predecessor_universe_sha256: SOURCE_PREDECESSOR_UNIVERSE_SHA256.to_string(),
        manifest_projection_correction_absorbed: true,
        source_semantics_changed: false,
        source_family_count: receipts.len(),
        frontend_source_count,
        backend_source_count,
        operations_source_count,
        promoted_knowledge_atoms: bundle.atoms.len(),
        promoted_composition_recipes: bundle.recipes.len(),
        three_layer_recipe_count: bundle
            .recipes
            .iter()
            .filter(|recipe| recipe.required_layers.len() == 3)
            .count(),
        activation_canaries_passed: canaries.len(),
        raw_training_data_files_copied: 0,
        raw_source_fragments_promoted: 0,
        external_llm_calls: 0,
        codex_runtime_calls: 0,
        network_reads: 0,
        network_writes: 0,
    };
    write_json(
        report_dir.join("source_generation_receipt.json"),
        &json!({
            "schema": "b_core_synapse_generation_receipt_v1",
            "source_universe_sha256": SOURCE_UNIVERSE_SHA256,
            "source_predecessor_universe_sha256": SOURCE_PREDECESSOR_UNIVERSE_SHA256,
            "source_manifest_sha256": SOURCE_MANIFEST_SHA256,
            "source_records_sha256": SOURCE_RECORDS_SHA256,
            "canonical_node_count": manifest["canonical_node_count"],
            "canonical_relation_count": manifest["canonical_relation_count"],
            "canonical_programming_mechanism_count": manifest["canonical_programming_mechanism_count"],
            "manifest_projection_correction": manifest["manifest_projection_correction"],
            "manifest_projection_semantics_mutated": manifest["manifest_projection_semantics_mutated"],
            "raw_knowledge_files_copied": 0,
        }),
    )?;
    write_json(report_dir.join("source_family_receipts.json"), &receipts)?;
    write_json(
        report_dir.join("fullstack_ops_knowledge_bundle.json"),
        &bundle,
    )?;
    write_json(report_dir.join("activation_canaries.json"), &canaries)?;
    write_json(report_dir.join("absorption_report.json"), &report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipts() -> Vec<SourceFamilyReceipt> {
        SOURCE_FAMILY_SPECS
            .iter()
            .map(|spec| SourceFamilyReceipt {
                layer: spec.layer,
                classifier_version: SOURCE_FAMILY_CLASSIFIER_VERSION.to_string(),
                source_count: spec.expected_count,
                source_set_sha256: spec.expected_sha256.to_string(),
                expected_source_count: spec.expected_count,
                expected_source_set_sha256: spec.expected_sha256.to_string(),
                matches_expected: true,
            })
            .collect()
    }

    #[test]
    fn bundle_is_compact_local_and_three_layer_composable() {
        let bundle = promoted_bundle();
        assert_eq!(validate_bundle(&bundle), Ok(()));
        assert_eq!(bundle.source_family_receipts.len(), 3);
        assert_eq!(bundle.recipes.len(), 3);
        assert!(bundle
            .recipes
            .iter()
            .all(|recipe| recipe.required_layers.len() == 3));
        assert!(!bundle.raw_source_copied);
        assert_eq!(bundle.external_llm_calls, 0);
        assert_eq!(bundle.codex_runtime_calls, 0);
    }

    #[test]
    fn request_flow_activates_sparse_cross_layer_recipe() {
        let bundle = build_bundle(receipts());
        let activation = activate(
            &bundle,
            &KnowledgeQuery {
                required_layers: vec![
                    CodingLayer::Frontend,
                    CodingLayer::Backend,
                    CodingLayer::Operations,
                ],
                required_capabilities: vec![
                    Capability::ProtocolValidation,
                    Capability::TelemetryCorrelation,
                ],
            },
        )
        .expect("activate request recipe");
        assert_eq!(
            activation.selected_recipe_ids,
            vec!["FULLSTACK-REQUEST-TO-OBSERVABILITY-V1"]
        );
        assert!(activation.active_atom_count <= MAX_ACTIVE_ATOMS);
        assert_eq!(activation.full_knowledge_scans, 0);
    }

    #[test]
    fn every_recipe_transports_types_as_a_repair_composition_lesson() {
        let bundle = build_bundle(receipts());
        for recipe in &bundle.recipes {
            let lesson =
                recipe_as_composition_lesson(&bundle, &recipe.recipe_id).expect("convert recipe");
            assert_eq!(validate_composition_lesson(&lesson), Ok(()));
            assert!(!lesson.exact_source_fragment_present);
        }
    }

    #[test]
    fn every_recipe_executes_and_rejects_contract_counterexamples() {
        let bundle = build_bundle(receipts());
        let mut artifacts = BTreeSet::new();
        for recipe in &bundle.recipes {
            let receipt =
                execute_fullstack_recipe_behavioral_canary(&bundle, &recipe.recipe_id).unwrap();
            assert_eq!(receipt.cases_executed, 3);
            assert_eq!(receipt.cases_passed, 3);
            assert!(receipt.exact_pipeline_observed);
            assert!(receipt.wrong_input_contract_rejected);
            assert!(receipt.reordered_pipeline_rejected);
            assert_eq!(receipt.execution.executed_atom_ids, recipe.ordered_atom_ids);
            assert!(artifacts.insert(receipt.behavioral_artifact_sha256));
        }
        assert_eq!(artifacts.len(), bundle.recipes.len());
    }

    #[test]
    fn invalid_source_family_receipt_blocks_promotion() {
        let mut invalid = receipts();
        invalid[0].matches_expected = false;
        assert_eq!(
            validate_bundle(&build_bundle(invalid)),
            Err(KnowledgeValidationError::SourceFamilyMismatch(
                CodingLayer::Frontend
            ))
        );
    }

    #[test]
    fn matching_flag_cannot_hide_a_forged_source_hash() {
        let mut invalid = receipts();
        invalid[0].source_set_sha256 = "0".repeat(64);
        invalid[0].matches_expected = true;
        assert_eq!(
            validate_bundle(&build_bundle(invalid)),
            Err(KnowledgeValidationError::SourceFamilyMismatch(
                CodingLayer::Frontend
            ))
        );
    }

    #[test]
    fn activation_never_exceeds_predeclared_bound() {
        let bundle = build_bundle(receipts());
        for recipe in &bundle.recipes {
            let activation = activate(
                &bundle,
                &KnowledgeQuery {
                    required_layers: recipe.required_layers.clone(),
                    required_capabilities: recipe.capabilities.clone(),
                },
            )
            .expect("activate exact recipe");
            assert!(activation.active_atom_count <= MAX_ACTIVE_ATOMS);
            assert_eq!(activation.task_identity_routing_events, 0);
            assert_eq!(activation.repository_identity_routing_events, 0);
        }
    }
}
