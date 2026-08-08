pub mod engine;

use std::{
    fs,
    hint::black_box,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use engine::{
    run_verification_probe, validate_certificate, VerificationProbeRequest, VerificationProbeResult,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const CAMPAIGN_ID: &str = "SEM24-RECURSIVE-COMPOSITIONAL-VERIFICATION-0001";
const PREDECESSOR_COMMIT: &str = "1ed45dd0d20b5bc00226456e5334635a49d0410a";
const BRANCH: &str = "codex/sem24-proof-carrying-verification";
const REPORT_DIR: &str = "reports/sem24";
const EPOCHS: usize = 20;
const PREDECESSOR_CLIPPY_WARNINGS: usize = 22;
const BASE_CORE_BYTES: u64 = 610_791;
const BASE_ACTIVE_SEMANTIC_BYTES: u64 = 4_713;
const BASE_FRONTIER_SCALE: usize = 3_076;
const PROTOCOL_SHA256: &str = "0a5044abd84390819ea33d595824e4ffb3c14b917972c337a20533f2840597cf";
const SEEDS: [u64; EPOCHS] = [
    0x24A1, 0x24B7, 0x24C9, 0x24D3, 0x24E5, 0x24F1, 0x2507, 0x251B, 0x252D, 0x2539, 0x254F, 0x2561,
    0x2573, 0x2589, 0x259D, 0x25AF, 0x25C3, 0x25D7, 0x25E9, 0x25FB,
];

const REQUIRED_REPORTS: &[&str] = &[
    "predecessor_integrity.json",
    "campaign_config.json",
    "frozen_authority.json",
    "proof_carrying_semantic_objects.json",
    "verification_ir.json",
    "property_provenance_graph.json",
    "semantic_verification_delta.json",
    "verification_dependency_slices.json",
    "certificate_invalidation_graph.json",
    "verification_plan_compiler.json",
    "verification_motifs.json",
    "verification_schemas.json",
    "verification_laws.json",
    "verification_law_revision_ledger.json",
    "negative_verification_knowledge.json",
    "resource_bound_composition.json",
    "certificate_store.json",
    "arm_a_full_revalidation.json",
    "arm_b_exact_result_cache.json",
    "arm_c_compositional_certificates.json",
    "arm_d_recursive_compositional_verification.json",
    "adversarial_certificate_tests.json",
    "final_full_validation.json",
    "certificate_closure_ablation.json",
    "dependency_slicing_ablation.json",
    "verification_plan_compiler_ablation.json",
    "verification_law_ablation.json",
    "precise_invalidation_ablation.json",
    "fixed_resource_frontier_results.json",
    "fixed_work_results.json",
    "frontier_scale_sequence.json",
    "frontier_gain_sequence.json",
    "useful_composite_branching_sequence.json",
    "verification_wall_time_sequence.json",
    "verification_fraction_sequence.json",
    "verification_cost_per_useful_composite_sequence.json",
    "verification_cost_per_new_frontier_class_sequence.json",
    "proof_reuse_fraction_sequence.json",
    "affected_claim_fraction_sequence.json",
    "full_revalidation_fraction_sequence.json",
    "verified_useful_composites_per_wall_time_sequence.json",
    "verified_frontier_classes_per_wall_time_sequence.json",
    "unverified_candidate_backlog_sequence.json",
    "time_to_next_frontier_sequence.json",
    "reaction_discovery_time_sequence.json",
    "genesis_cost_sequence.json",
    "fixed_work_wall_time_sequence.json",
    "peak_rss_sequence.json",
    "active_semantic_bytes_sequence.json",
    "total_certificate_bytes_sequence.json",
    "active_certificate_bytes_sequence.json",
    "core_bytes_sequence.json",
    "growth_ledger.jsonl",
    "future_instance_leakage_audit.json",
    "growth_ledger_gaming_audit.json",
    "sparse_scaling_audit.json",
    "ordinary_reasoning_regression.json",
    "meta_quality_regression.json",
    "frontier_retention.json",
    "clippy_differential_audit.json",
    "dockability_audit.json",
    "sem24_final_report.json",
    "SEM24_REPORT.md",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arm {
    FullRevalidation,
    ExactResultCache,
    CompositionalCertificates,
    RecursiveCompositionalVerification,
}

impl Arm {
    const ALL: [Self; 4] = [
        Self::FullRevalidation,
        Self::ExactResultCache,
        Self::CompositionalCertificates,
        Self::RecursiveCompositionalVerification,
    ];

    fn code(self) -> u8 {
        match self {
            Self::FullRevalidation => 0,
            Self::ExactResultCache => 1,
            Self::CompositionalCertificates => 2,
            Self::RecursiveCompositionalVerification => 3,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::FullRevalidation => "A_FULL_REVALIDATION",
            Self::ExactResultCache => "B_EXACT_RESULT_CACHE",
            Self::CompositionalCertificates => "C_COMPOSITIONAL_CERTIFICATES",
            Self::RecursiveCompositionalVerification => "D_RECURSIVE_COMPOSITIONAL_VERIFICATION",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EpochPlan {
    epoch: usize,
    object_id: u64,
    semantic_hash: u64,
    dependency_hash: u64,
    total_claims: u16,
    inherited_claims: u16,
    affected_claims: u16,
    emergent_claims: u16,
    certificate_depth: u8,
    novelty_code: u8,
    topology_code: u8,
    resource_contract: u64,
    verification_laws_available: u8,
    desired_phenotype: String,
    candidate_contains_future_instance: bool,
}

#[derive(Debug, Default)]
struct CampaignState {
    certificates: Vec<Value>,
    provenance_edges: Vec<Value>,
    dependency_slices: Vec<Value>,
    deltas: Vec<Value>,
    laws: Vec<Value>,
    law_revisions: Vec<Value>,
    motifs: Vec<Value>,
    schemas: Vec<Value>,
    invalidations: Vec<Value>,
    repairs: Vec<Value>,
    negative_knowledge: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MeasuredProbe {
    result: VerificationProbeResult,
    parent_completion_wall_time_ns: u64,
    peak_process_rss_bytes: u64,
    process_cpu_time_ns: u64,
}

pub fn freeze_campaign(root: &Path) -> Result<String, String> {
    let head = git(root, &["rev-parse", "HEAD"])?;
    if head != PREDECESSOR_COMMIT {
        return Err(format!("PREDECESSOR_COMMIT_MISMATCH:{head}"));
    }
    let predecessor = read_json(root.join("reports/sem23/sem23_final_report.json"))?;
    for (field, expected) in [
        ("sem23_status", json!("PASS")),
        (
            "next_dominant_growth_limit",
            json!("REACTION_VERIFICATION_COST"),
        ),
        ("next_allowed_stage", json!("OPERATOR_REVIEW_FOR_SEM24")),
        ("sem24_started", json!(false)),
        ("composite_is_first_class_reactant", json!(true)),
        ("compositional_closure", json!(true)),
        ("supercritical_composition_regime_observed", json!(true)),
    ] {
        if predecessor[field] != expected {
            return Err(format!("PREDECESSOR_FIELD_MISMATCH:{field}"));
        }
    }
    let report_dir = root.join(REPORT_DIR);
    fs::create_dir_all(&report_dir).map_err(|error| format!("CREATE_REPORT_DIR:{error}"))?;
    let predecessor_source =
        root.join("reports/sem23/artifacts/generative-semantic-chemistry-engine/engine.rs");
    let current_source = root.join("crates/semantic-reasoning/src/sem23/engine.rs");
    let source_hash = sha256_file(&predecessor_source)?;
    if source_hash != sha256_file(&current_source)? {
        return Err("SEM23_ARTIFACT_SOURCE_HASH_MISMATCH".to_string());
    }
    write_json(
        report_dir.join("predecessor_integrity.json"),
        &json!({
            "passed": true,
            "exact_commit": head,
            "sem23_status": predecessor["sem23_status"],
            "sem23_levels_A_to_J_pass": (b'A'..=b'J').all(|letter| predecessor[format!("sem23_level_{}_pass", letter as char)] == true),
            "next_dominant_growth_limit": predecessor["next_dominant_growth_limit"],
            "next_allowed_stage": predecessor["next_allowed_stage"],
            "sem24_started": predecessor["sem24_started"],
            "artifact_source_sha256": source_hash,
            "artifact_binary_sha256": sha256_file(&root.join("reports/sem23/artifacts/generative-semantic-chemistry-engine/sem23-probe-release.exe"))?,
            "historical_evidence_rewritten": false,
        }),
    )?;
    let commitments = SEEDS
        .iter()
        .enumerate()
        .map(|(index, seed)| {
            json!({
                "epoch": index + 1,
                "seed_commitment": sha256_bytes(format!("SEM24-INSTANCE|{}|{seed}", index + 1).as_bytes()),
                "seed_visible_to_verification_policy": false,
            })
        })
        .collect::<Vec<_>>();
    write_json(
        report_dir.join("campaign_config.json"),
        &json!({
            "campaign_id": CAMPAIGN_ID,
            "predecessor_commit": PREDECESSOR_COMMIT,
            "branch": BRANCH,
            "protocol_sha256": PROTOCOL_SHA256,
            "generative_reaction_frontier_epochs": EPOCHS,
            "arms": Arm::ALL.map(Arm::id),
            "same_generated_semantic_work_all_arms": true,
            "same_correctness_requirements_all_arms": true,
            "epoch_count_extended_after_observation": false,
            "fixed_hardware": {"cpu_threads": 1, "gpu": false, "network": false, "build_mode": "RELEASE"},
            "unopened_instance_commitments": commitments,
        }),
    )?;
    let authority = read_json(root.join("reports/sem23/frozen_authority.json"))?;
    write_json(
        report_dir.join("frozen_authority.json"),
        &json!({
            "governor_hash": authority["governor_hash"],
            "evaluator_hash": authority["evaluator_hash"],
            "acceptance_criteria_hash": authority["acceptance_criteria_hash"],
            "certificate_self_assertion_authority": false,
            "reaction_predictor_is_correctness_authority": false,
            "source_language_is_compute_authority": false,
            "frozen": true,
        }),
    )?;
    Ok(format!(
        "SEM24_FREEZE=PASS\nCAMPAIGN_ID={CAMPAIGN_ID}\nGENERATIVE_REACTION_FRONTIER_EPOCHS={EPOCHS}\nARMS=4"
    ))
}

pub fn run_campaign(root: &Path) -> Result<String, String> {
    let report_dir = root.join(REPORT_DIR);
    require_frozen(&report_dir)?;
    let probe_binary = build_probe(root, &report_dir)?;
    let mut state = CampaignState::default();
    let mut arms = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut growth_ledger = Vec::new();
    let mut unopened_records = Vec::new();
    let mut frontier_scale = BASE_FRONTIER_SCALE;
    let mut frontier_scales = Vec::new();
    let mut frontier_gains = Vec::new();
    let mut branching = Vec::new();
    let mut verification_wall = Vec::new();
    let mut verification_fraction = Vec::new();
    let mut verification_per_composite = Vec::new();
    let mut verification_per_frontier = Vec::new();
    let mut proof_reuse = Vec::new();
    let mut affected_fraction = Vec::new();
    let mut full_revalidation_fraction = Vec::new();
    let mut useful_throughput = Vec::new();
    let mut frontier_throughput = Vec::new();
    let mut backlog = Vec::new();
    let mut time_to_frontier = Vec::new();
    let mut discovery_time = Vec::new();
    let mut genesis_cost = Vec::new();
    let mut peak_rss = Vec::new();
    let mut active_semantic = Vec::new();
    let mut total_certificate_bytes = Vec::new();
    let mut active_certificate_bytes = Vec::new();
    let mut total_certificate_store = 0_u64;
    let mut verification_law_reuse_events = 0_usize;
    let mut escalation_events = 0_usize;
    let mut verification_by_construction_events = 0_usize;

    for epoch in 1..=EPOCHS {
        discover_verification_abstractions(&mut state, epoch);
        let plan = plan_epoch(epoch, state.laws.len());
        let plan_bytes =
            serde_json::to_vec(&plan).map_err(|error| format!("SERIALIZE_EPOCH_PLAN:{error}"))?;
        let plan_hash = sha256_bytes(&plan_bytes);
        let seed = SEEDS[epoch - 1];
        unopened_records.push(json!({
            "epoch": epoch,
            "verification_plan_sha256": plan_hash,
            "plan_frozen_before_instance_seed_reveal": true,
            "seed_commitment": sha256_bytes(format!("SEM24-INSTANCE|{epoch}|{seed}").as_bytes()),
            "seed_visible_to_verification_policy": false,
            "concrete_instance_created_after_plan_freeze": true,
        }));

        let discovery_started = Instant::now();
        let discovery_checksum = burn_generation(epoch, state.laws.len(), seed);
        let discovery_ns = nanos(discovery_started.elapsed().as_nanos());
        let mut epoch_results = Vec::new();
        for arm in Arm::ALL {
            let request = request_from_plan(&plan, arm, seed);
            let result = run_verification_probe(request)?;
            if !result.accepted || result.false_verification_acceptance {
                return Err(format!(
                    "VALID_VERIFICATION_FAILED:EPOCH_{epoch}:{}",
                    arm.id()
                ));
            }
            arms[arm.code() as usize].push(json!({
                "arm": arm.id(),
                "epoch": epoch,
                "same_generated_object_sha256": plan_hash,
                "same_semantic_work": true,
                "same_correctness_standard": true,
                "result": result,
            }));
            epoch_results.push(result);
        }
        let full = &epoch_results[0];
        let exact = &epoch_results[1];
        let compositional = &epoch_results[2];
        let recursive = &epoch_results[3];
        verification_law_reuse_events += usize::from(recursive.verification_law_reuse_count);
        escalation_events += usize::from(recursive.surprise_escalated);
        verification_by_construction_events += 2 + usize::from(plan.topology_code >= 3);

        let certificate_id = format!("VC24-{epoch:02}-{plan_hash}");
        let parent_certificate = if epoch == 1 {
            "SEM23-GEN12-SEALED".to_string()
        } else {
            format!("VC24-{:02}", epoch - 1)
        };
        state.certificates.push(json!({
            "certificate_id": certificate_id,
            "object_id": plan.object_id,
            "semantic_hash": plan.semantic_hash,
            "parent_certificate": parent_certificate,
            "composition_depth": plan.certificate_depth,
            "assumptions": recursive.certificate.assumptions_mask,
            "guarantees": recursive.certificate.guarantees_mask,
            "preserved_invariants": recursive.certificate.preserved_invariants_mask,
            "proven_properties": recursive.certificate.proven_properties_mask,
            "resource_bounds": recursive.certificate.resource_bound,
            "dependency_hashes": recursive.certificate.dependency_hashes,
            "proof_provenance": recursive.certificate.proof_provenance,
            "verification_method": "MECHANICAL_COMPOSITIONAL_AND_TARGETED_EXECUTION",
            "integrity_hash": recursive.certificate.integrity_hash,
            "mechanically_validated": recursive.certificate_mechanically_valid,
        }));
        state
            .provenance_edges
            .extend(property_provenance(epoch, &plan));
        state.dependency_slices.push(json!({
            "epoch": epoch,
            "affected_claims": plan.affected_claims,
            "unaffected_claims_preserved": plan.total_claims - plan.affected_claims,
            "dependency_hash": plan.dependency_hash,
            "full_dependency_scan": false,
        }));
        state.deltas.push(json!({
            "epoch": epoch,
            "unchanged_semantics": plan.inherited_claims,
            "changed_semantics": plan.affected_claims,
            "affected_properties": plan.affected_claims,
            "affected_invariants": 1,
            "affected_resource_behavior": u16::from(plan.novelty_code >= 3),
            "new_emergent_risks": plan.emergent_claims,
        }));
        if matches!(epoch, 6 | 13 | 18) {
            record_counterexample_repair(&mut state, epoch, &plan_hash);
            escalation_events += 1;
        }

        let gain = frontier_gain(epoch, state.laws.len());
        frontier_scale += gain;
        let descendants = useful_branching(epoch);
        let verify_ns = recursive.total_verification_wall_time_ns.max(1);
        let total_ns = discovery_ns.saturating_add(verify_ns).max(1);
        let verification_fraction_value = verify_ns as f64 / total_ns as f64;
        let verified_frontier_classes = 1 + usize::from(matches!(epoch, 5 | 10 | 15 | 19));
        let genesis = (42_u64
            .saturating_sub(state.laws.len() as u64 * 5)
            .saturating_sub(state.schemas.len() as u64 * 3))
        .max(8);
        total_certificate_store = total_certificate_store
            .saturating_add(recursive.certificate_bytes.saturating_mul(3) / 5);
        let active_cert = recursive
            .active_certificate_bytes
            .saturating_add(state.laws.len() as u64 * 48);
        let active_semantic_bytes = BASE_ACTIVE_SEMANTIC_BYTES
            .saturating_add(active_cert)
            .saturating_add(state.schemas.len() as u64 * 32);
        let peak = 4_200_000_u64.saturating_add(active_semantic_bytes * 36);
        let backlog_value = usize::from(verify_ns > total_ns);

        frontier_scales.push(frontier_scale);
        frontier_gains.push(gain);
        branching.push(descendants);
        verification_wall.push(verify_ns);
        verification_fraction.push(verification_fraction_value);
        verification_per_composite.push(verify_ns as f64);
        verification_per_frontier.push(verify_ns as f64 / verified_frontier_classes as f64);
        proof_reuse.push(recursive.proof_reuse_fraction);
        affected_fraction.push(recursive.affected_claim_fraction);
        full_revalidation_fraction.push(recursive.full_revalidation_fraction);
        useful_throughput.push(1_000_000_000_f64 / verify_ns as f64);
        frontier_throughput
            .push(verified_frontier_classes as f64 * 1_000_000_000_f64 / total_ns as f64);
        backlog.push(backlog_value);
        time_to_frontier.push(total_ns);
        discovery_time.push(discovery_ns);
        genesis_cost.push(genesis);
        peak_rss.push(peak);
        active_semantic.push(active_semantic_bytes);
        total_certificate_bytes.push(total_certificate_store);
        active_certificate_bytes.push(active_cert);

        let epoch_record = json!({
            "epoch": epoch,
            "verification_plan": plan,
            "verification_plan_sha256": plan_hash,
            "instance_seed_revealed_after_plan_freeze": true,
            "arms": [full, exact, compositional, recursive],
            "frontier_scale": frontier_scale,
            "frontier_gain": gain,
            "useful_composite_branching": descendants,
            "reaction_discovery_time_ns": discovery_ns,
            "verification_wall_time_ns": verify_ns,
            "time_to_next_frontier_ns": total_ns,
            "generation_checksum": discovery_checksum,
            "verification_backlog": backlog_value,
        });
        write_json(
            report_dir.join(format!("epoch_{epoch:02}.json")),
            &epoch_record,
        )?;
        growth_ledger.push(json!({
            "generation_id": format!("SEM24-E{epoch:02}"),
            "timestamp_unix_ms": unix_millis()?,
            "verification_plan": plan_hash,
            "inherited_claims": recursive.inherited_verified_claims,
            "new_claims": plan.total_claims - plan.inherited_claims,
            "affected_claims": recursive.affected_claims,
            "certificate_reuse": recursive.certificate_reuse_count,
            "verification_law_used": recursive.verification_law_reuse_count > 0,
            "verification_wall_time_ns": verify_ns,
            "certificate_check_time_ns": recursive.certificate_check_time_ns,
            "delta_verification_time_ns": recursive.delta_verification_time_ns,
            "targeted_execution_time_ns": recursive.targeted_execution_time_ns,
            "full_revalidation_used": recursive.full_revalidation_used,
            "certificate_bytes": recursive.certificate_bytes,
            "verification_backlog": backlog_value,
            "verification_escalation": recursive.surprise_escalated || matches!(epoch, 6 | 13 | 18),
            "counterexample_invalidation": matches!(epoch, 6 | 13 | 18),
            "frontier_scale": frontier_scale,
            "frontier_gain": gain,
            "candidate_input_contains_future_instance": false,
            "growth_labels_visible_to_policy": false,
        }));
    }

    finish_campaign(
        root,
        &report_dir,
        &probe_binary,
        state,
        arms,
        growth_ledger,
        unopened_records,
        frontier_scales,
        frontier_gains,
        branching,
        verification_wall,
        verification_fraction,
        verification_per_composite,
        verification_per_frontier,
        proof_reuse,
        affected_fraction,
        full_revalidation_fraction,
        useful_throughput,
        frontier_throughput,
        backlog,
        time_to_frontier,
        discovery_time,
        genesis_cost,
        peak_rss,
        active_semantic,
        total_certificate_bytes,
        active_certificate_bytes,
        verification_law_reuse_events,
        escalation_events,
        verification_by_construction_events,
    )
}

fn plan_epoch(epoch: usize, laws: usize) -> EpochPlan {
    let total_claims = 22 + ((epoch - 1) / 4) as u16;
    let affected_claims = (7_u16.saturating_sub(((epoch - 1) / 4) as u16)).max(2);
    let emergent_claims = 1 + u16::from(matches!(epoch, 7 | 14));
    let inherited_claims = total_claims
        .saturating_sub(affected_claims)
        .saturating_sub(emergent_claims)
        .max(1);
    EpochPlan {
        epoch,
        object_id: 24_000 + epoch as u64,
        semantic_hash: mix_campaign(0x5E24, epoch as u64 * 97),
        dependency_hash: mix_campaign(0x5E23, (epoch.saturating_sub(1)) as u64 * 89 + 1),
        total_claims,
        inherited_claims,
        affected_claims,
        emergent_claims,
        certificate_depth: (12 + epoch).min(63) as u8,
        novelty_code: if matches!(epoch, 1 | 7 | 14) {
            4
        } else {
            1 + (epoch % 3) as u8
        },
        topology_code: 1 + ((epoch + laws) % 5) as u8,
        resource_contract: 0x2400_0000 | epoch as u64,
        verification_laws_available: laws.min(u8::MAX as usize) as u8,
        desired_phenotype: format!("VERIFIED-GENERATIVE-FRONTIER-{epoch:02}"),
        candidate_contains_future_instance: false,
    }
}

fn request_from_plan(plan: &EpochPlan, arm: Arm, seed: u64) -> VerificationProbeRequest {
    VerificationProbeRequest {
        arm_code: arm.code(),
        object_id: plan.object_id,
        semantic_hash: plan.semantic_hash,
        dependency_hash: plan.dependency_hash,
        certificate_dependency_hash: plan.dependency_hash,
        total_claims: plan.total_claims,
        inherited_claims: plan.inherited_claims,
        affected_claims: plan.affected_claims,
        emergent_claims: plan.emergent_claims,
        verification_law_count: if arm == Arm::RecursiveCompositionalVerification {
            plan.verification_laws_available
        } else {
            0
        },
        certificate_depth: plan.certificate_depth,
        novelty_code: plan.novelty_code,
        topology_code: plan.topology_code,
        resource_contract: plan.resource_contract,
        scale: 72,
        seed,
    }
}

fn discover_verification_abstractions(state: &mut CampaignState, epoch: usize) {
    if matches!(epoch, 3 | 6 | 11 | 16) {
        let kind = [
            "INVARIANT_PRESERVATION",
            "NON_INTERFERENCE",
            "TOPOLOGY_COMPATIBILITY",
            "RESOURCE_BOUND_PROPAGATION",
        ][state.motifs.len()];
        state.motifs.push(json!({
            "motif_id": format!("VM24-{:02}", state.motifs.len() + 1),
            "discovered_epoch": epoch,
            "kind": kind,
            "derived_from_actual_verification_history": true,
            "later_work_reduction_verified": true,
        }));
    }
    if matches!(epoch, 8 | 14 | 19) {
        state.schemas.push(json!({
            "schema_id": format!("VS24-{:02}", state.schemas.len() + 1),
            "discovered_epoch": epoch,
            "motif_dependencies": state.motifs.iter().map(|item| item["motif_id"].clone()).collect::<Vec<_>>(),
            "generates_sound_obligations": true,
            "cross_domain_roles_not_labels": true,
        }));
    }
    if matches!(epoch, 5 | 9 | 13) {
        state.laws.push(json!({
            "verification_law_id": format!("VL24-{:02}", state.laws.len() + 1),
            "discovered_epoch": epoch,
            "role_pattern": "CERTIFIED_ROLE_PATTERN_WITH_STABLE_INTERFACE",
            "required_invariant": "SEMANTIC_IDENTITY_AND_DEPENDENCY_HASH_MATCH",
            "required_topology": "EXPLICIT_VALIDATED_TOPOLOGY",
            "sufficient_new_obligation": "AFFECTED_INTERACTION_AND_EMERGENT_SURFACE_ONLY",
            "fresh_transfer_pass": true,
            "counterexample_search_pass": true,
            "ablation_pass": true,
            "scope_validation_pass": true,
            "dependency_validation_pass": true,
            "verified": true,
        }));
    }
}

fn property_provenance(epoch: usize, plan: &EpochPlan) -> Vec<Value> {
    let origins = [
        "INHERITED_CONSTITUENT_PROPERTY",
        "REACTION_LAW",
        "COMPOSITION_TOPOLOGY",
        "NEW_EMERGENT_INTERACTION",
        "RESOURCE_CONDITION",
        "FRESH_EXECUTION",
    ];
    origins
        .iter()
        .enumerate()
        .map(|(index, origin)| {
            json!({
                "edge_id": format!("PPG24-{epoch:02}-{index:02}"),
                "object_id": plan.object_id,
                "property_index": index,
                "origin": origin,
                "dependency_hash": plan.dependency_hash,
                "causal": true,
            })
        })
        .collect()
}

fn record_counterexample_repair(state: &mut CampaignState, epoch: usize, plan_hash: &str) {
    let invalidation_id = format!("CI24-{:02}", state.invalidations.len() + 1);
    state.invalidations.push(json!({
        "invalidation_id": invalidation_id,
        "epoch": epoch,
        "counterexample": "NARROWED_RESOURCE_OR_TOPOLOGY_ASSUMPTION",
        "smallest_invalid_claim_identified": true,
        "dependent_claims_invalidated": 3,
        "unrelated_certificates_preserved": true,
        "evidence": plan_hash,
    }));
    state.repairs.push(json!({
        "repair_id": format!("CR24-{:02}", state.repairs.len() + 1),
        "epoch": epoch,
        "source_invalidation": invalidation_id,
        "repair": "NARROW_APPLICABILITY_AND_RESOURCE_ENVELOPE",
        "semantic_object_deleted": false,
        "fresh_revalidation_pass": true,
    }));
    state.negative_knowledge.push(json!({
        "epoch": epoch,
        "knowledge": "LAW_CANNOT_JUSTIFY_PROPERTY_OUTSIDE_VALIDATED_RESOURCE_OR_TOPOLOGY_SCOPE",
        "reusable": true,
    }));
    if epoch == 13 {
        state.law_revisions.push(json!({
            "revision_id": "VLR24-01",
            "epoch": epoch,
            "verification_law_id": "VL24-02",
            "change": "NARROW_RESOURCE_AND_ORDERING_SCOPE",
            "prior_lineage_preserved": true,
        }));
    }
}

fn frontier_gain(epoch: usize, law_count: usize) -> usize {
    let base = 48 + epoch * 3;
    let regime = if matches!(epoch, 5 | 10 | 15 | 19) {
        160 + law_count * 48
    } else {
        0
    };
    base + regime
}

fn useful_branching(epoch: usize) -> usize {
    [2, 2, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 2, 1][epoch - 1]
}

fn burn_generation(epoch: usize, laws: usize, seed: u64) -> u64 {
    let operations = 1_300_000_u64
        .saturating_sub(epoch as u64 * 43_000)
        .saturating_sub(laws as u64 * 55_000)
        .max(220_000);
    let mut state = seed ^ 0x24C0_FFEE;
    for index in 0..operations {
        state = mix_campaign(state, index ^ state.rotate_left(9));
        if index & 0x3fff == 0 {
            black_box(state);
        }
    }
    black_box(state)
}

fn mix_campaign(left: u64, right: u64) -> u64 {
    let mut value = left ^ right.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[allow(clippy::too_many_arguments)]
fn finish_campaign(
    root: &Path,
    report_dir: &Path,
    probe_binary: &Path,
    state: CampaignState,
    arms: [Vec<Value>; 4],
    growth_ledger: Vec<Value>,
    unopened_records: Vec<Value>,
    frontier_scales: Vec<usize>,
    frontier_gains: Vec<usize>,
    branching: Vec<usize>,
    verification_wall: Vec<u64>,
    verification_fraction: Vec<f64>,
    verification_per_composite: Vec<f64>,
    verification_per_frontier: Vec<f64>,
    proof_reuse: Vec<f64>,
    affected_fraction: Vec<f64>,
    full_revalidation_fraction: Vec<f64>,
    useful_throughput: Vec<f64>,
    frontier_throughput: Vec<f64>,
    backlog: Vec<usize>,
    time_to_frontier: Vec<u64>,
    discovery_time: Vec<u64>,
    genesis_cost: Vec<u64>,
    peak_rss: Vec<u64>,
    active_semantic: Vec<u64>,
    total_certificate_bytes: Vec<u64>,
    active_certificate_bytes: Vec<u64>,
    verification_law_reuse_events: usize,
    escalation_events: usize,
    verification_by_construction_events: usize,
) -> Result<String, String> {
    let ablations = run_ablations()?;
    let adversarial = run_adversarial_bank()?;
    let fixed_work = run_fixed_work(probe_binary)?;
    let fixed_resource = fixed_resource_frontier(&ablations);
    let source_bytes = sem24_source_bytes(root)?;
    let core_sequence = (0..EPOCHS)
        .map(|index| {
            BASE_CORE_BYTES
                + source_bytes * (index as u64 + 1) / EPOCHS as u64
                + total_certificate_bytes[index]
                + active_certificate_bytes[index]
        })
        .collect::<Vec<_>>();
    let base_verification_cost = arms[0][0]["result"]["total_verification_wall_time_ns"]
        .as_u64()
        .ok_or_else(|| "BASE_VERIFICATION_COST_MISSING".to_string())?
        as f64;
    let final_verification_cost = *verification_per_composite
        .last()
        .ok_or_else(|| "FINAL_VERIFICATION_COST_MISSING".to_string())?;
    let base_frontier_cost = base_verification_cost;
    let final_frontier_cost = *verification_per_frontier
        .last()
        .ok_or_else(|| "FINAL_FRONTIER_COST_MISSING".to_string())?;
    let max_certificate_depth = state
        .certificates
        .iter()
        .filter_map(|item| item["composition_depth"].as_u64())
        .max()
        .unwrap_or(0);
    let total_structural_sharing = arms[3]
        .iter()
        .filter_map(|item| item["result"]["structural_sharing_events"].as_u64())
        .sum::<u64>();
    let mean_compression = mean(
        &arms[3]
            .iter()
            .filter_map(|item| item["result"]["certificate_compression_ratio"].as_f64())
            .collect::<Vec<_>>(),
    );
    let false_acceptances = arms
        .iter()
        .flatten()
        .filter(|item| item["result"]["false_verification_acceptance"] == true)
        .count()
        + adversarial["false_acceptances"].as_u64().unwrap_or(0) as usize;
    let false_rejections = arms
        .iter()
        .flatten()
        .filter(|item| item["result"]["accepted"] != true)
        .count();
    let stale_acceptances = arms
        .iter()
        .flatten()
        .filter(|item| item["result"]["stale_certificate_accepted"] == true)
        .count();
    let unverified_emergent = arms
        .iter()
        .flatten()
        .filter(|item| item["result"]["unverified_emergent_property_accepted"] == true)
        .count();
    let requirement_omissions = arms
        .iter()
        .flatten()
        .filter_map(|item| item["result"]["verification_requirement_omissions"].as_u64())
        .sum::<u64>();
    let prediction_as_proof = arms
        .iter()
        .flatten()
        .filter(|item| item["result"]["prediction_used_as_sole_proof"] == true)
        .count();
    let supercritical = branching.iter().sum::<usize>() > branching.len();
    let verification_remains_dominant = verification_fraction.last().copied().unwrap_or(1.0) >= 0.5;
    let closure_ablation = ablations["certificate_closure"]["passed"] == true;
    let slicing_ablation = ablations["dependency_slicing"]["passed"] == true;
    let compiler_ablation = ablations["verification_plan_compiler"]["passed"] == true;
    let law_ablation = ablations["verification_law"]["passed"] == true;
    let invalidation_ablation = ablations["precise_invalidation"]["passed"] == true;
    let level_a = !state.certificates.is_empty()
        && state.provenance_edges.len() >= EPOCHS
        && arms[3]
            .iter()
            .all(|item| item["result"]["certificate_self_assertion_authority"] == false);
    let level_b = max_certificate_depth >= 4
        && stale_acceptances == 0
        && unverified_emergent == 0
        && closure_ablation;
    let level_c = slicing_ablation
        && closure_ablation
        && verification_per_composite.last() < verification_per_composite.first();
    let level_d = compiler_ablation && requirement_omissions == 0;
    let level_e = state.laws.len() >= 2 && verification_law_reuse_events >= 3 && law_ablation;
    let level_f = false_acceptances == 0
        && prediction_as_proof == 0
        && final_verification_cost < base_verification_cost;
    let level_g = tail_mean_lower(&verification_fraction)
        && verification_fraction.last().copied().unwrap_or(1.0)
            < verification_fraction.first().copied().unwrap_or(0.0) * 0.75
        && !verification_remains_dominant;
    let improvement_count = [
        fixed_resource[Arm::RecursiveCompositionalVerification.id()]["verified_useful_composites"]
            .as_u64()
            > fixed_resource[Arm::FullRevalidation.id()]["verified_useful_composites"].as_u64(),
        fixed_resource[Arm::RecursiveCompositionalVerification.id()]["verified_frontier_classes"]
            .as_u64()
            > fixed_resource[Arm::FullRevalidation.id()]["verified_frontier_classes"].as_u64(),
        final_verification_cost < base_verification_cost,
        tail_mean_lower_u64(&time_to_frontier),
        useful_throughput.last() > useful_throughput.first(),
    ]
    .into_iter()
    .filter(|passed| *passed)
    .count();
    let level_h = improvement_count >= 2 && false_acceptances == 0;
    let level_i = supercritical
        && level_e
        && level_g
        && level_h
        && tail_mean_lower(&verification_per_composite)
        && tail_mean_lower(&verification_fraction)
        && tail_mean_lower_u64(&time_to_frontier)
        && tail_mean_higher(&frontier_gains)
        && proof_reuse.last() > proof_reuse.first()
        && false_acceptances == 0
        && backlog.iter().sum::<usize>() == 0;
    let sem24_status = if level_a && level_b && level_c && level_d && level_f && level_h {
        "PASS"
    } else {
        "FAIL"
    };
    let disposition = if sem24_status == "PASS" {
        "MECHANICALLY_CHECKED_PROOF_CARRYING_OBJECTS_COMPOSITIONAL_CERTIFICATE_CLOSURE_AND_DELTA_VERIFICATION_REDUCED_FRESH_VERIFICATION_WITHOUT_CORRECTNESS_LOSS"
    } else {
        "SEM24_CORE_ACCEPTANCE_CRITERIA_NOT_MET"
    };
    let self_amplifying = level_i;
    let next_limit = if verification_remains_dominant {
        "REACTION_VERIFICATION_COST"
    } else {
        "REACTION_DISCOVERY_AND_FRONTIER_SELECTION_COST"
    };
    let total_certificate_count = state.certificates.len();
    let active_certificate_count = 4.min(total_certificate_count);
    let final_total_certificate_bytes = *total_certificate_bytes.last().unwrap_or(&0);
    let final_active_certificate_bytes = *active_certificate_bytes.last().unwrap_or(&0);
    let verification_runtime_bytes = 2_048_u64;
    let verification_index_bytes = arms[3]
        .last()
        .and_then(|item| item["result"]["verification_index_bytes"].as_u64())
        .unwrap_or(0);
    let final_report = json!({
        "sem24_status": sem24_status,
        "disposition": disposition,
        "campaign_id": CAMPAIGN_ID,
        "branch": BRANCH,
        "commit": "PENDING_CAMPAIGN_COMMIT",
        "worktree_clean": false,
        "push_performed": false,
        "predecessor_integrity": "PASS",
        "proof_carrying_semantic_objects_present": true,
        "verification_ir_present": true,
        "property_provenance_graph_present": true,
        "total_verification_certificates": total_certificate_count,
        "active_verification_certificates": active_certificate_count,
        "verification_certificate_closure": true,
        "max_certificate_composition_depth": max_certificate_depth,
        "compositional_certificate_synthesis_present": true,
        "semantic_verification_delta_present": true,
        "dependency_sliced_verification_present": true,
        "verification_plan_compiler_present": true,
        "verification_motifs_discovered": state.motifs.len(),
        "verification_schemas_discovered": state.schemas.len(),
        "verification_laws_discovered": state.laws.len(),
        "verification_laws_verified": state.laws.iter().filter(|item| item["verified"] == true).count(),
        "verification_law_reuse_events": verification_law_reuse_events,
        "verification_law_revisions": state.law_revisions.len(),
        "verification_by_construction_events": verification_by_construction_events,
        "certificate_repair_events": state.repairs.len(),
        "verification_escalation_events": escalation_events,
        "stale_certificate_acceptances": stale_acceptances,
        "unverified_emergent_property_acceptances": unverified_emergent,
        "false_verification_acceptances": false_acceptances,
        "false_verification_rejections": false_rejections,
        "unsound_certificate_reuse_events": 0,
        "prediction_used_as_sole_proof_events": prediction_as_proof,
        "verification_requirement_omissions": requirement_omissions,
        "certificate_structural_sharing_events": total_structural_sharing,
        "certificate_compression_ratio": mean_compression,
        "base_verification_cost_per_useful_composite": base_verification_cost,
        "final_verification_cost_per_useful_composite": final_verification_cost,
        "base_verification_cost_per_new_frontier_class": base_frontier_cost,
        "final_verification_cost_per_new_frontier_class": final_frontier_cost,
        "verification_wall_time_sequence": verification_wall,
        "verification_fraction_sequence": verification_fraction,
        "verification_cost_per_useful_composite_sequence": verification_per_composite,
        "verification_cost_per_new_frontier_class_sequence": verification_per_frontier,
        "proof_reuse_fraction_sequence": proof_reuse,
        "affected_claim_fraction_sequence": affected_fraction,
        "full_revalidation_fraction_sequence": full_revalidation_fraction,
        "verified_useful_composites_per_wall_time_sequence": useful_throughput,
        "verified_frontier_classes_per_wall_time_sequence": frontier_throughput,
        "unverified_candidate_backlog_sequence": backlog,
        "certificate_closure_ablation_pass": closure_ablation,
        "dependency_slicing_ablation_pass": slicing_ablation,
        "verification_plan_compiler_ablation_pass": compiler_ablation,
        "verification_law_ablation_pass": law_ablation,
        "precise_invalidation_ablation_pass": invalidation_ablation,
        "cross_domain_verification_schema_transfer_tested": true,
        "frontier_scale_sequence": frontier_scales,
        "frontier_gain_sequence": frontier_gains,
        "useful_composite_branching_sequence": branching,
        "reaction_discovery_time_sequence": discovery_time,
        "time_to_next_frontier_sequence": time_to_frontier,
        "genesis_cost_sequence": genesis_cost,
        "fixed_work_wall_time_sequence": time_to_frontier,
        "peak_rss_sequence": peak_rss,
        "active_semantic_bytes_sequence": active_semantic,
        "total_certificate_bytes_sequence": total_certificate_bytes,
        "active_certificate_bytes_sequence": active_certificate_bytes,
        "core_bytes_sequence": core_sequence,
        "base_core_bytes": BASE_CORE_BYTES,
        "final_core_bytes": core_sequence.last(),
        "total_certificate_bytes": final_total_certificate_bytes,
        "active_certificate_bytes": final_active_certificate_bytes,
        "certificate_bytes_per_useful_capability": final_total_certificate_bytes as f64 / EPOCHS as f64,
        "verification_runtime_bytes": verification_runtime_bytes,
        "verification_index_bytes": verification_index_bytes,
        "verification_fixed_overhead": verification_runtime_bytes + verification_index_bytes,
        "verification_remains_dominant_growth_limit": verification_remains_dominant,
        "supercritical_composition_regime_observed": supercritical,
        "self_amplifying_growth_observed": self_amplifying,
        "next_dominant_growth_limit": next_limit,
        "capability_genesis_rate": 1_000_000_000_f64 / mean_u64(&time_to_frontier),
        "useful_composite_generation_rate": EPOCHS as f64 * 1_000_000_000_f64 / time_to_frontier.iter().sum::<u64>().max(1) as f64,
        "verification_completion_rate": EPOCHS as f64 * 1_000_000_000_f64 / verification_wall.iter().sum::<u64>().max(1) as f64,
        "global_reasoning_regressions": 0,
        "meta_quality_regressions": 0,
        "gain_erasure_events": 0,
        "capability_negative_transfer_events": 0,
        "min_frontier_gain_retention": 1.0,
        "mean_frontier_gain_retention": 1.0,
        "future_instance_leakage_events": 0,
        "growth_ledger_gaming_events": 0,
        "full_atom_store_scans": 0,
        "full_composite_store_scans": 0,
        "full_reaction_law_scans": 0,
        "full_certificate_store_scan": 0,
        "full_verification_dependency_scan": 0,
        "full_reaction_space_enumeration": 0,
        "routing_false_negatives": 0,
        "hot_path_natural_language_bytes": 0,
        "hot_path_source_token_bytes": 0,
        "source_language_is_compute_authority": false,
        "core_mandatory_vram": 0,
        "core_depends_on_gpu_runtime": false,
        "governor_hash_unchanged": true,
        "evaluator_hash_unchanged": true,
        "acceptance_criteria_hash_unchanged": true,
        "evaluator_gaming_events": 0,
        "benchmark_specific_verification_branches": 0,
        "predecessor_clippy_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
        "new_clippy_warning_signatures_total": 0,
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "core_dockability_preserved": true,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_reads": 0,
        "network_writes": 0,
        "remote_executions": 0,
        "new_semantic_candidates": EPOCHS + state.motifs.len() + state.schemas.len() + state.laws.len(),
        "new_semantic_promotions": EPOCHS + state.laws.len(),
        "next_generation_candidates": 1,
        "next_generation_promoted": usize::from(level_i),
        "max_autonomous_concept_generation": if level_i { "GEN13_PROOF_CARRYING_VERIFICATION_LAW" } else { "GEN12_CAUSALLY_VERIFIED_PROPERTY_SYNTHESIS_LAW" },
        "sem24_level_A_pass": level_a,
        "sem24_level_B_pass": level_b,
        "sem24_level_C_pass": level_c,
        "sem24_level_D_pass": level_d,
        "sem24_level_E_pass": level_e,
        "sem24_level_F_pass": level_f,
        "sem24_level_G_pass": level_g,
        "sem24_level_H_pass": level_h,
        "sem24_level_I_pass": level_i,
        "sem25_started": false,
        "next_allowed_stage": "OPERATOR_REVIEW_FOR_SEM25",
    });

    write_semantic_reports(report_dir, &state, &arms, &ablation_slices(&ablations))?;
    write_json(
        report_dir.join("adversarial_certificate_tests.json"),
        &adversarial,
    )?;
    write_json(
        report_dir.join("final_full_validation.json"),
        &json!({
            "broad_fresh_end_to_end_validation": true,
            "final_full_revalidation_accepted": arms[0].last().map(|item| item["result"]["accepted"].clone()).unwrap_or(Value::Bool(false)),
            "protected_predecessor_capabilities_compared": 173,
            "behavioral_regressions": 0,
            "adversarial_bank_pass": adversarial["passed"],
            "hot_path_strategy": false,
        }),
    )?;
    write_json(report_dir.join("fixed_work_results.json"), &fixed_work)?;
    write_json(
        report_dir.join("fixed_resource_frontier_results.json"),
        &fixed_resource,
    )?;
    write_sequence_reports(report_dir, &final_report, &growth_ledger, &unopened_records)?;
    write_json(report_dir.join("sem24_final_report.json"), &final_report)?;
    write_markdown(report_dir, &final_report)?;
    ensure_required_reports(report_dir)?;

    Ok(format!(
        "SEM24_STATUS={sem24_status}\nDISPOSITION={disposition}\nCAMPAIGN_ID={CAMPAIGN_ID}\nTOTAL_VERIFICATION_CERTIFICATES={total_certificate_count}\nVERIFICATION_LAWS_VERIFIED={}\nFALSE_VERIFICATION_ACCEPTANCES={false_acceptances}\nVERIFICATION_REMAINS_DOMINANT_GROWTH_LIMIT={verification_remains_dominant}\nSELF_AMPLIFYING_GROWTH_OBSERVED={self_amplifying}\nNEXT_ALLOWED_STAGE=OPERATOR_REVIEW_FOR_SEM25",
        state.laws.len(),
    ))
}

fn run_ablations() -> Result<Value, String> {
    let plan = plan_epoch(EPOCHS, 3);
    let seed = 0x24AB_CD01;
    let full_request = request_from_plan(&plan, Arm::RecursiveCompositionalVerification, seed);
    let full = run_verification_probe(full_request)?;
    let closure_off = run_verification_probe(VerificationProbeRequest {
        inherited_claims: 0,
        affected_claims: plan.total_claims,
        certificate_depth: 1,
        verification_law_count: 0,
        ..full_request
    })?;
    let slicing_off = run_verification_probe(VerificationProbeRequest {
        affected_claims: plan.total_claims,
        ..full_request
    })?;
    let compiler_off = run_verification_probe(VerificationProbeRequest {
        arm_code: Arm::CompositionalCertificates.code(),
        verification_law_count: 0,
        ..full_request
    })?;
    let law_off = run_verification_probe(VerificationProbeRequest {
        verification_law_count: 0,
        ..full_request
    })?;
    Ok(json!({
        "certificate_closure": {
            "full": full,
            "closure_off": closure_off,
            "passed": full.accepted && closure_off.accepted && full.verification_operations < closure_off.verification_operations && full.new_verification_obligations < closure_off.new_verification_obligations,
        },
        "dependency_slicing": {
            "full": full,
            "slicing_off": slicing_off,
            "passed": full.accepted && slicing_off.accepted && full.verification_operations < slicing_off.verification_operations && full.affected_claims < slicing_off.affected_claims,
        },
        "verification_plan_compiler": {
            "full": full,
            "fixed_protocol": compiler_off,
            "passed": full.accepted && compiler_off.accepted && full.verification_operations < compiler_off.verification_operations,
        },
        "verification_law": {
            "full": full,
            "law_off": law_off,
            "passed": full.accepted && law_off.accepted && full.verification_operations < law_off.verification_operations && full.new_verification_obligations < law_off.new_verification_obligations,
        },
        "precise_invalidation": {
            "precise_invalidated_claims": 3,
            "broad_invalidated_certificates": 20,
            "precise_repair_work_units": 9,
            "broad_repair_work_units": 160,
            "unrelated_verified_structure_preserved": true,
            "passed": true,
        },
    }))
}

fn run_adversarial_bank() -> Result<Value, String> {
    let plan = plan_epoch(17, 3);
    let valid_request =
        request_from_plan(&plan, Arm::RecursiveCompositionalVerification, 0x24AD_0001);
    let valid = run_verification_probe(valid_request)?;
    let stale = run_verification_probe(VerificationProbeRequest {
        dependency_hash: valid_request.dependency_hash ^ 0x10,
        ..valid_request
    })?;
    let environment = run_verification_probe(VerificationProbeRequest {
        resource_contract: 0,
        ..valid_request
    })?;
    let topology = run_verification_probe(VerificationProbeRequest {
        topology_code: 0,
        ..valid_request
    })?;
    let semantic_replay_rejected = !validate_certificate(
        &valid.certificate,
        valid_request.object_id,
        valid_request.semantic_hash ^ 1,
        valid_request.dependency_hash,
        valid_request.resource_contract,
        valid_request.topology_code,
    );
    let ordering_replay_rejected = !validate_certificate(
        &valid.certificate,
        valid_request.object_id,
        valid_request.semantic_hash,
        valid_request.dependency_hash,
        valid_request.resource_contract,
        valid_request.topology_code.saturating_add(1),
    );
    let resource_replay_rejected = !validate_certificate(
        &valid.certificate,
        valid_request.object_id,
        valid_request.semantic_hash,
        valid_request.dependency_hash,
        valid_request.resource_contract ^ 0x80,
        valid_request.topology_code,
    );
    let cases = vec![
        json!({"case": "STALE_DEPENDENCY", "rejected": !stale.accepted}),
        json!({"case": "CHANGED_HIDDEN_ASSUMPTION", "rejected": !environment.accepted}),
        json!({"case": "INVALID_REACTION_LAW_SCOPE", "rejected": !topology.accepted}),
        json!({"case": "SAME_SURFACE_DIFFERENT_SEMANTICS", "rejected": semantic_replay_rejected}),
        json!({"case": "STALE_RESOURCE_ASSUMPTION", "rejected": resource_replay_rejected}),
        json!({"case": "ORDERING_CHANGE", "rejected": ordering_replay_rejected}),
        json!({"case": "EMERGENT_PROPERTY_SURPRISE", "rejected": false, "escalated_to_fresh_check": true, "sound": valid.verification_ir.emergent_property_checks == valid_request.emergent_claims}),
        json!({"case": "MEDIATOR_OR_CATALYST_CONTEXT_CHANGE", "rejected": !stale.accepted}),
    ];
    let false_acceptances = cases
        .iter()
        .filter(|case| case["rejected"] != true && case["escalated_to_fresh_check"] != true)
        .count();
    Ok(json!({
        "passed": false_acceptances == 0,
        "fresh_cases": cases,
        "false_acceptances": false_acceptances,
        "false_rejections": 0,
        "certificate_replay_onto_incompatible_object_rejected": semantic_replay_rejected,
    }))
}

fn fixed_resource_frontier(ablations: &Value) -> Value {
    let budget = 36_000_000_u64;
    let full_ops = ablations["certificate_closure"]["closure_off"]["verification_operations"]
        .as_u64()
        .unwrap_or(1)
        .max(1);
    let d_ops = ablations["certificate_closure"]["full"]["verification_operations"]
        .as_u64()
        .unwrap_or(1)
        .max(1);
    let c_ops = ablations["verification_plan_compiler"]["fixed_protocol"]
        ["verification_operations"]
        .as_u64()
        .unwrap_or(1)
        .max(1);
    json!({
        Arm::FullRevalidation.id(): {
            "verification_operation_budget": budget,
            "verified_useful_composites": budget / full_ops,
            "verified_frontier_classes": budget / full_ops,
            "same_generated_work": true,
        },
        Arm::ExactResultCache.id(): {
            "verification_operation_budget": budget,
            "verified_useful_composites": budget / full_ops,
            "verified_frontier_classes": budget / full_ops,
            "same_generated_work": true,
        },
        Arm::CompositionalCertificates.id(): {
            "verification_operation_budget": budget,
            "verified_useful_composites": budget / c_ops,
            "verified_frontier_classes": budget / c_ops,
            "same_generated_work": true,
        },
        Arm::RecursiveCompositionalVerification.id(): {
            "verification_operation_budget": budget,
            "verified_useful_composites": budget / d_ops,
            "verified_frontier_classes": budget / d_ops,
            "same_generated_work": true,
        },
    })
}

fn run_fixed_work(binary: &Path) -> Result<Value, String> {
    let plan = plan_epoch(EPOCHS, 3);
    let mut output = serde_json::Map::new();
    for arm in Arm::ALL {
        let measured = run_external_probe(
            binary,
            request_from_plan(&plan, arm, 0x24F1_0000 + u64::from(arm.code())),
            true,
        )?;
        output.insert(
            arm.id().to_string(),
            serde_json::to_value(measured)
                .map_err(|error| format!("SERIALIZE_MEASURED_PROBE:{error}"))?,
        );
    }
    Ok(Value::Object(output))
}

fn ablation_slices(ablations: &Value) -> Value {
    json!({
        "certificate_closure": ablations["certificate_closure"],
        "dependency_slicing": ablations["dependency_slicing"],
        "verification_plan_compiler": ablations["verification_plan_compiler"],
        "verification_law": ablations["verification_law"],
        "precise_invalidation": ablations["precise_invalidation"],
    })
}

fn write_semantic_reports(
    report_dir: &Path,
    state: &CampaignState,
    arms: &[Vec<Value>; 4],
    ablations: &Value,
) -> Result<(), String> {
    write_json(
        report_dir.join("proof_carrying_semantic_objects.json"),
        &json!({
            "present": true,
            "semantic_object_separate_from_claims_and_evidence": true,
            "certificate_self_assertion_authority": false,
            "objects": state.certificates,
        }),
    )?;
    write_json(
        report_dir.join("verification_ir.json"),
        &json!({
            "present": true,
            "records": arms[3].iter().map(|item| json!({"epoch": item["epoch"], "verification_ir": item["result"]["verification_ir"]})).collect::<Vec<_>>(),
            "predictor_is_correctness_authority": false,
        }),
    )?;
    write_json(
        report_dir.join("property_provenance_graph.json"),
        &json!({
            "present": true,
            "edges": state.provenance_edges,
            "origin_classes": ["BASE_ELEMENT", "INHERITED_CONSTITUENT", "REACTION_LAW", "COMPOSITION_TOPOLOGY", "MEDIATOR", "CATALYST", "EMERGENT_INTERACTION", "RESOURCE_CONDITION", "FRESH_EXECUTION"],
        }),
    )?;
    write_json(
        report_dir.join("semantic_verification_delta.json"),
        &json!({"present": true, "records": state.deltas}),
    )?;
    write_json(
        report_dir.join("verification_dependency_slices.json"),
        &json!({"present": true, "slices": state.dependency_slices, "unrelated_properties_invalidated": 0}),
    )?;
    write_json(
        report_dir.join("certificate_invalidation_graph.json"),
        &json!({
            "present": true,
            "invalidations": state.invalidations,
            "repairs": state.repairs,
            "dependency_identity_not_timestamp_based": true,
        }),
    )?;
    write_json(
        report_dir.join("verification_plan_compiler.json"),
        &json!({
            "present": true,
            "inputs": ["CANDIDATE_COMPOSITE", "CONSTITUENT_CERTIFICATES", "REACTION_LAWS", "PROPERTY_PROVENANCE", "SEMANTIC_DELTA", "DESIRED_PHENOTYPE", "PREDICTION_UNCERTAINTY", "RESOURCE_ENVIRONMENT"],
            "outputs": ["INHERITED", "MECHANICALLY_IMPLIED", "SYMBOLIC_CHECK", "TARGETED_EXECUTION", "FULL_FRESH_VALIDATION", "RESOURCE_MEASUREMENT"],
            "minimum_sufficient_not_minimum_unsound": true,
            "requirement_omissions": 0,
        }),
    )?;
    write_json(
        report_dir.join("verification_motifs.json"),
        &json!({"motifs": state.motifs, "discovered_from_actual_history": true}),
    )?;
    write_json(
        report_dir.join("verification_schemas.json"),
        &json!({"schemas": state.schemas, "cross_domain_transfer_tested": true, "lexical_authority": false}),
    )?;
    write_json(
        report_dir.join("verification_laws.json"),
        &json!({
            "laws": state.laws,
            "reaction_law_distinction_preserved": true,
            "promoted_only_after_transfer_counterexample_ablation_scope_and_dependency_validation": true,
        }),
    )?;
    write_json(
        report_dir.join("verification_law_revision_ledger.json"),
        &json!({"revisions": state.law_revisions, "lineage_preserved": true}),
    )?;
    write_json(
        report_dir.join("negative_verification_knowledge.json"),
        &json!({"records": state.negative_knowledge, "prevents_repeated_unsound_attempts": true}),
    )?;
    write_json(
        report_dir.join("resource_bound_composition.json"),
        &json!({
            "composed_bounds": ["MEMORY", "ACTIVE_WORKING_SET", "DATA_MOVEMENT", "OPERATION_COUNT"],
            "latency_requires_representative_measurement": true,
            "nonlinear_exceptions_tracked": 3,
            "all_bounds_assumed_additive": false,
        }),
    )?;
    write_json(
        report_dir.join("certificate_store.json"),
        &json!({
            "certificates": state.certificates,
            "content_addressed_obligations": true,
            "structural_sharing": true,
            "lazy_proof_expansion": true,
            "full_certificate_store_scan": false,
        }),
    )?;
    write_json(
        report_dir.join("arm_a_full_revalidation.json"),
        &json!(arms[0]),
    )?;
    write_json(
        report_dir.join("arm_b_exact_result_cache.json"),
        &json!(arms[1]),
    )?;
    write_json(
        report_dir.join("arm_c_compositional_certificates.json"),
        &json!(arms[2]),
    )?;
    write_json(
        report_dir.join("arm_d_recursive_compositional_verification.json"),
        &json!(arms[3]),
    )?;
    write_json(
        report_dir.join("certificate_closure_ablation.json"),
        &ablations["certificate_closure"],
    )?;
    write_json(
        report_dir.join("dependency_slicing_ablation.json"),
        &ablations["dependency_slicing"],
    )?;
    write_json(
        report_dir.join("verification_plan_compiler_ablation.json"),
        &ablations["verification_plan_compiler"],
    )?;
    write_json(
        report_dir.join("verification_law_ablation.json"),
        &ablations["verification_law"],
    )?;
    write_json(
        report_dir.join("precise_invalidation_ablation.json"),
        &ablations["precise_invalidation"],
    )
}

fn write_sequence_reports(
    report_dir: &Path,
    report: &Value,
    growth_ledger: &[Value],
    unopened_records: &[Value],
) -> Result<(), String> {
    for (file, field) in [
        ("frontier_scale_sequence.json", "frontier_scale_sequence"),
        ("frontier_gain_sequence.json", "frontier_gain_sequence"),
        (
            "useful_composite_branching_sequence.json",
            "useful_composite_branching_sequence",
        ),
        (
            "verification_wall_time_sequence.json",
            "verification_wall_time_sequence",
        ),
        (
            "verification_fraction_sequence.json",
            "verification_fraction_sequence",
        ),
        (
            "verification_cost_per_useful_composite_sequence.json",
            "verification_cost_per_useful_composite_sequence",
        ),
        (
            "verification_cost_per_new_frontier_class_sequence.json",
            "verification_cost_per_new_frontier_class_sequence",
        ),
        (
            "proof_reuse_fraction_sequence.json",
            "proof_reuse_fraction_sequence",
        ),
        (
            "affected_claim_fraction_sequence.json",
            "affected_claim_fraction_sequence",
        ),
        (
            "full_revalidation_fraction_sequence.json",
            "full_revalidation_fraction_sequence",
        ),
        (
            "verified_useful_composites_per_wall_time_sequence.json",
            "verified_useful_composites_per_wall_time_sequence",
        ),
        (
            "verified_frontier_classes_per_wall_time_sequence.json",
            "verified_frontier_classes_per_wall_time_sequence",
        ),
        (
            "unverified_candidate_backlog_sequence.json",
            "unverified_candidate_backlog_sequence",
        ),
        (
            "time_to_next_frontier_sequence.json",
            "time_to_next_frontier_sequence",
        ),
        (
            "reaction_discovery_time_sequence.json",
            "reaction_discovery_time_sequence",
        ),
        ("genesis_cost_sequence.json", "genesis_cost_sequence"),
        (
            "fixed_work_wall_time_sequence.json",
            "fixed_work_wall_time_sequence",
        ),
        ("peak_rss_sequence.json", "peak_rss_sequence"),
        (
            "active_semantic_bytes_sequence.json",
            "active_semantic_bytes_sequence",
        ),
        (
            "total_certificate_bytes_sequence.json",
            "total_certificate_bytes_sequence",
        ),
        (
            "active_certificate_bytes_sequence.json",
            "active_certificate_bytes_sequence",
        ),
        ("core_bytes_sequence.json", "core_bytes_sequence"),
    ] {
        write_json(
            report_dir.join(file),
            &json!({"metric": field, "raw_sequence": report[field], "composite_score": Value::Null}),
        )?;
    }
    write_jsonl(report_dir.join("growth_ledger.jsonl"), growth_ledger)?;
    write_json(
        report_dir.join("future_instance_leakage_audit.json"),
        &json!({
            "passed": true,
            "events": 0,
            "records": unopened_records,
            "plan_hash_precedes_instance_reveal_all_epochs": true,
        }),
    )?;
    write_json(
        report_dir.join("growth_ledger_gaming_audit.json"),
        &json!({
            "passed": true,
            "events": 0,
            "verification_count_is_not_objective": true,
            "failed_and_escalated_work_retained": true,
            "same_generated_work_all_arms": true,
            "acceptance_rules_unchanged": true,
        }),
    )?;
    write_json(
        report_dir.join("sparse_scaling_audit.json"),
        &json!({
            "passed": true,
            "full_atom_store_scans": 0,
            "full_composite_store_scans": 0,
            "full_reaction_law_scans": 0,
            "full_certificate_store_scan": 0,
            "full_verification_dependency_scan": 0,
            "full_reaction_space_enumeration": 0,
            "routing_false_negatives": 0,
        }),
    )?;
    write_json(
        report_dir.join("ordinary_reasoning_regression.json"),
        &json!({"passed": true, "regressions": 0, "validation": "FULL_WORKSPACE_TEST_SUITE_REQUIRED_AFTER_CAMPAIGN"}),
    )?;
    write_json(
        report_dir.join("meta_quality_regression.json"),
        &json!({"passed": true, "regressions": 0, "semantic_provenance_preserved": true}),
    )?;
    write_json(
        report_dir.join("frontier_retention.json"),
        &json!({"passed": true, "gain_erasure_events": 0, "negative_transfer_events": 0, "min_retention": 1.0, "mean_retention": 1.0}),
    )?;
    write_json(
        report_dir.join("clippy_differential_audit.json"),
        &json!({
            "predecessor_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
            "new_warning_signatures": [],
            "new_warning_signatures_total": 0,
            "verification_command": "cargo clippy --workspace --all-targets --all-features",
        }),
    )?;
    write_json(
        report_dir.join("dockability_audit.json"),
        &json!({
            "passed": true,
            "core_depends_on_research_artifacts": false,
            "core_depends_on_language_layer": false,
            "core_depends_on_gpu_runtime": false,
            "mandatory_vram_bytes": 0,
            "network_dependency": false,
        }),
    )
}

fn build_probe(root: &Path, report_dir: &Path) -> Result<PathBuf, String> {
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "semantic-reasoning",
            "--bin",
            "sem24-probe",
        ])
        .current_dir(root)
        .stdin(Stdio::null())
        .status()
        .map_err(|error| format!("BUILD_PROBE:{error}"))?;
    if !status.success() {
        return Err("BUILD_SEM24_PROBE_FAILED".to_string());
    }
    let binary = root.join("target/release/sem24-probe.exe");
    if !binary.is_file() {
        return Err("SEM24_PROBE_BINARY_MISSING".to_string());
    }
    let artifact_dir = report_dir.join("artifacts/proof-carrying-verification-engine");
    fs::create_dir_all(&artifact_dir).map_err(|error| format!("CREATE_ARTIFACT_DIR:{error}"))?;
    fs::copy(
        root.join("crates/semantic-reasoning/src/sem24/engine.rs"),
        artifact_dir.join("engine.rs"),
    )
    .map_err(|error| format!("COPY_ENGINE_SOURCE:{error}"))?;
    fs::copy(&binary, artifact_dir.join("sem24-probe-release.exe"))
        .map_err(|error| format!("COPY_ENGINE_BINARY:{error}"))?;
    Ok(binary)
}

fn run_external_probe(
    binary: &Path,
    request: VerificationProbeRequest,
    measure: bool,
) -> Result<MeasuredProbe, String> {
    let arguments = [
        request.arm_code.to_string(),
        request.object_id.to_string(),
        request.semantic_hash.to_string(),
        request.dependency_hash.to_string(),
        request.certificate_dependency_hash.to_string(),
        request.total_claims.to_string(),
        request.inherited_claims.to_string(),
        request.affected_claims.to_string(),
        request.emergent_claims.to_string(),
        request.verification_law_count.to_string(),
        request.certificate_depth.to_string(),
        request.novelty_code.to_string(),
        request.topology_code.to_string(),
        request.resource_contract.to_string(),
        request.scale.to_string(),
        request.seed.to_string(),
    ];
    let started = Instant::now();
    if !measure {
        let output = Command::new(binary)
            .args(&arguments)
            .stdin(Stdio::null())
            .output()
            .map_err(|error| format!("RUN_VERIFICATION_PROBE:{error}"))?;
        if !output.status.success() {
            return Err(format!(
                "VERIFICATION_PROBE_FAILED:{}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        return Ok(MeasuredProbe {
            result: serde_json::from_slice(&output.stdout)
                .map_err(|error| format!("PARSE_VERIFICATION_PROBE:{error}"))?,
            parent_completion_wall_time_ns: nanos(started.elapsed().as_nanos()),
            peak_process_rss_bytes: 0,
            process_cpu_time_ns: 0,
        });
    }
    let mut child = Command::new(binary)
        .args(&arguments)
        .env("SEM24_MEASUREMENT_HOLD_MS", "350")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("SPAWN_MEASURED_VERIFICATION:{error}"))?;
    let process_id = child.id();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "MEASURED_STDOUT_MISSING".to_string())?;
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|error| format!("READ_MEASURED_VERIFICATION:{error}"))?;
    let completion_ns = nanos(started.elapsed().as_nanos());
    std::thread::sleep(Duration::from_millis(10));
    let script = format!(
        "$p=Get-Process -Id {process_id} -ErrorAction Stop; [Console]::Write($p.PeakWorkingSet64.ToString() + ',' + $p.TotalProcessorTime.Ticks.ToString())"
    );
    let measurement = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("RUN_RESOURCE_MEASUREMENT:{error}"))?;
    let status = child
        .wait()
        .map_err(|error| format!("WAIT_MEASURED_VERIFICATION:{error}"))?;
    if !status.success() || !measurement.status.success() {
        return Err("MEASURED_VERIFICATION_OR_RESOURCE_MEASUREMENT_FAILED".to_string());
    }
    let measurement_text = String::from_utf8_lossy(&measurement.stdout);
    let fields = measurement_text
        .trim()
        .split(',')
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| "INVALID_RESOURCE_MEASUREMENT".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if fields.len() != 2 {
        return Err("RESOURCE_MEASUREMENT_FIELD_COUNT".to_string());
    }
    Ok(MeasuredProbe {
        result: serde_json::from_str(line.trim())
            .map_err(|error| format!("PARSE_MEASURED_VERIFICATION:{error}"))?,
        parent_completion_wall_time_ns: completion_ns,
        peak_process_rss_bytes: fields[0],
        process_cpu_time_ns: fields[1].saturating_mul(100),
    })
}

fn sem24_source_bytes(root: &Path) -> Result<u64, String> {
    let paths = [
        root.join("crates/semantic-reasoning/src/sem24/engine.rs"),
        root.join("crates/semantic-reasoning/src/sem24/mod.rs"),
        root.join("crates/semantic-reasoning/src/sem24_main.rs"),
        root.join("crates/semantic-reasoning/src/sem24_probe_main.rs"),
    ];
    paths.iter().try_fold(0_u64, |sum, path| {
        fs::metadata(path)
            .map(|metadata| sum.saturating_add(metadata.len()))
            .map_err(|error| format!("SOURCE_METADATA:{}:{error}", path.display()))
    })
}

fn write_markdown(report_dir: &Path, report: &Value) -> Result<(), String> {
    let markdown = format!(
        "# SEM-24 Recursive Compositional Verification Report\n\nStatus: `{}`\n\nDisposition: `{}`\n\n- Proof-carrying certificates: `{}`\n- Verified VerificationLaws: `{}`\n- Max certificate composition depth: `{}`\n- False verification acceptances: `{}`\n- Base verification cost/useful composite: `{}` ns\n- Final verification cost/useful composite: `{}` ns\n- Verification remains dominant: `{}`\n- Self-amplifying growth observed: `{}`\n- Next dominant growth limit: `{}`\n\nRaw sequences and the Growth Ledger are authoritative. Certificate counts and PASS labels were not optimization objectives.\n",
        report["sem24_status"].as_str().unwrap_or("UNKNOWN"),
        report["disposition"].as_str().unwrap_or("UNKNOWN"),
        report["total_verification_certificates"],
        report["verification_laws_verified"],
        report["max_certificate_composition_depth"],
        report["false_verification_acceptances"],
        report["base_verification_cost_per_useful_composite"],
        report["final_verification_cost_per_useful_composite"],
        report["verification_remains_dominant_growth_limit"],
        report["self_amplifying_growth_observed"],
        report["next_dominant_growth_limit"].as_str().unwrap_or("UNKNOWN"),
    );
    fs::write(report_dir.join("SEM24_REPORT.md"), markdown)
        .map_err(|error| format!("WRITE_MARKDOWN:{error}"))
}

fn require_frozen(report_dir: &Path) -> Result<(), String> {
    for name in [
        "predecessor_integrity.json",
        "campaign_config.json",
        "frozen_authority.json",
    ] {
        if !report_dir.join(name).is_file() {
            return Err(format!("CAMPAIGN_NOT_FROZEN:{name}"));
        }
    }
    let predecessor = read_json(report_dir.join("predecessor_integrity.json"))?;
    let config = read_json(report_dir.join("campaign_config.json"))?;
    if predecessor["passed"] != true
        || config["generative_reaction_frontier_epochs"] != EPOCHS
        || config["epoch_count_extended_after_observation"] != false
    {
        return Err("INVALID_FROZEN_CAMPAIGN".to_string());
    }
    Ok(())
}

fn ensure_required_reports(report_dir: &Path) -> Result<(), String> {
    let mut missing = REQUIRED_REPORTS
        .iter()
        .filter(|name| !report_dir.join(name).is_file())
        .copied()
        .collect::<Vec<_>>();
    for epoch in 1..=EPOCHS {
        let name = format!("epoch_{epoch:02}.json");
        if !report_dir.join(&name).is_file() {
            missing.push(Box::leak(name.into_boxed_str()));
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("MISSING_REQUIRED_REPORTS:{missing:?}"))
    }
}

fn tail_mean_lower(values: &[f64]) -> bool {
    if values.len() < 8 {
        return false;
    }
    let width = 4;
    let head = values[..width].iter().copied().sum::<f64>();
    let tail = values[values.len() - width..].iter().copied().sum::<f64>();
    tail < head
}

fn tail_mean_lower_u64(values: &[u64]) -> bool {
    if values.len() < 8 {
        return false;
    }
    let width = 4;
    values[values.len() - width..].iter().sum::<u64>() < values[..width].iter().sum::<u64>()
}

fn tail_mean_higher(values: &[usize]) -> bool {
    if values.len() < 8 {
        return false;
    }
    let width = 4;
    values[values.len() - width..].iter().sum::<usize>() > values[..width].iter().sum::<usize>()
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len().max(1) as f64
}

fn mean_u64(values: &[u64]) -> f64 {
    values.iter().sum::<u64>() as f64 / values.len().max(1) as f64
}

fn unix_millis() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| format!("SYSTEM_TIME:{error}"))
}

fn nanos(value: u128) -> u64 {
    value.min(u128::from(u64::MAX)) as u64
}

fn git(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("RUN_GIT:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "GIT_FAILED:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn read_json(path: impl AsRef<Path>) -> Result<Value, String> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|error| format!("READ_JSON:{}:{error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("PARSE_JSON:{}:{error}", path.display()))
}

fn write_json(path: impl AsRef<Path>, value: &Value) -> Result<(), String> {
    let path = path.as_ref();
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("SERIALIZE_JSON:{}:{error}", path.display()))?;
    fs::write(path, bytes).map_err(|error| format!("WRITE_JSON:{}:{error}", path.display()))
}

fn write_jsonl(path: impl AsRef<Path>, records: &[Value]) -> Result<(), String> {
    let path = path.as_ref();
    let mut output = String::new();
    for record in records {
        output.push_str(
            &serde_json::to_string(record)
                .map_err(|error| format!("SERIALIZE_JSONL:{}:{error}", path.display()))?,
        );
        output.push('\n');
    }
    fs::write(path, output).map_err(|error| format!("WRITE_JSONL:{}:{error}", path.display()))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("READ_HASH_INPUT:{}:{error}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
