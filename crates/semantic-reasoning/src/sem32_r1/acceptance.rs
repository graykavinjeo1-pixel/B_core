use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawAcceptanceFields {
    pub persistent_world_layer_present: bool,
    pub partial_observability_present: bool,
    pub belief_update_verified: bool,
    pub temporal_delta_prediction_verified: bool,
    pub language_is_reasoning_authority: bool,
    pub factored_relational_mechanisms_verified: bool,
    pub fresh_transition_prediction_verified: bool,
    pub entity_id_invariant_transfer_pass: bool,
    pub entity_cardinality_generalization_pass: bool,
    pub novel_relation_topology_transfer_pass: bool,
    pub epistemic_aleatoric_separation_pass: bool,
    pub predictive_uncertainty_collapse_events: u64,
    pub observation_intervention_separated: bool,
    pub confounded_causality_resolved: bool,
    pub false_causal_promotions: u64,
    pub horizon_1_verified: bool,
    pub horizon_2_verified: bool,
    pub horizon_4_verified: bool,
    pub horizon_8_verified: bool,
    pub horizon_failures_decomposed: bool,
    pub isolated_counterfactuals_verified: bool,
    pub counterfactual_actual_mutation_events: u64,
    pub unreachable_shortcut_accepts: u64,
    pub prediction_residuals_drive_learning: bool,
    pub causal_refinement_or_composition_verified: bool,
    pub future_prediction_improves: bool,
    pub large_world_canary_entities: u64,
    pub world_memory_full_scans: u64,
    pub causal_mechanism_full_scans: u64,
    pub interventional_causality_ablation_pass: bool,
    pub causal_law_memory_ablation_pass: bool,
    pub factored_dynamics_ablation_pass: bool,
    pub epistemic_uncertainty_ablation_pass: bool,
    pub counterfactual_causal_model_ablation_pass: bool,
    pub sparse_causal_routing_ablation_pass: bool,
    pub relational_topology_repair_ablation_pass: bool,
}

impl RawAcceptanceFields {
    pub fn all_pass() -> Self {
        Self {
            persistent_world_layer_present: true,
            partial_observability_present: true,
            belief_update_verified: true,
            temporal_delta_prediction_verified: true,
            language_is_reasoning_authority: false,
            factored_relational_mechanisms_verified: true,
            fresh_transition_prediction_verified: true,
            entity_id_invariant_transfer_pass: true,
            entity_cardinality_generalization_pass: true,
            novel_relation_topology_transfer_pass: true,
            epistemic_aleatoric_separation_pass: true,
            predictive_uncertainty_collapse_events: 0,
            observation_intervention_separated: true,
            confounded_causality_resolved: true,
            false_causal_promotions: 0,
            horizon_1_verified: true,
            horizon_2_verified: true,
            horizon_4_verified: true,
            horizon_8_verified: true,
            horizon_failures_decomposed: true,
            isolated_counterfactuals_verified: true,
            counterfactual_actual_mutation_events: 0,
            unreachable_shortcut_accepts: 0,
            prediction_residuals_drive_learning: true,
            causal_refinement_or_composition_verified: true,
            future_prediction_improves: true,
            large_world_canary_entities: 100_000,
            world_memory_full_scans: 0,
            causal_mechanism_full_scans: 0,
            interventional_causality_ablation_pass: true,
            causal_law_memory_ablation_pass: true,
            factored_dynamics_ablation_pass: true,
            epistemic_uncertainty_ablation_pass: true,
            counterfactual_causal_model_ablation_pass: true,
            sparse_causal_routing_ablation_pass: true,
            relational_topology_repair_ablation_pass: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceDecision {
    pub levels: [bool; 10],
    pub sem32_r1_pass: bool,
}

pub fn evaluate_raw(fields: &RawAcceptanceFields) -> AcceptanceDecision {
    let levels = [
        fields.persistent_world_layer_present
            && fields.partial_observability_present
            && fields.belief_update_verified
            && fields.temporal_delta_prediction_verified
            && !fields.language_is_reasoning_authority,
        fields.factored_relational_mechanisms_verified
            && fields.fresh_transition_prediction_verified
            && fields.entity_id_invariant_transfer_pass
            && fields.entity_cardinality_generalization_pass
            && fields.novel_relation_topology_transfer_pass,
        fields.epistemic_aleatoric_separation_pass
            && fields.predictive_uncertainty_collapse_events == 0,
        fields.observation_intervention_separated
            && fields.confounded_causality_resolved
            && fields.false_causal_promotions == 0,
        fields.horizon_1_verified
            && fields.horizon_2_verified
            && fields.horizon_4_verified
            && fields.horizon_8_verified
            && fields.horizon_failures_decomposed,
        fields.isolated_counterfactuals_verified
            && fields.counterfactual_actual_mutation_events == 0,
        fields.unreachable_shortcut_accepts == 0,
        fields.prediction_residuals_drive_learning
            && fields.causal_refinement_or_composition_verified
            && fields.future_prediction_improves,
        fields.large_world_canary_entities >= 100_000
            && fields.world_memory_full_scans == 0
            && fields.causal_mechanism_full_scans == 0,
        fields.interventional_causality_ablation_pass
            && fields.causal_law_memory_ablation_pass
            && fields.factored_dynamics_ablation_pass
            && fields.epistemic_uncertainty_ablation_pass
            && fields.counterfactual_causal_model_ablation_pass
            && fields.sparse_causal_routing_ablation_pass
            && fields.relational_topology_repair_ablation_pass,
    ];
    AcceptanceDecision {
        sem32_r1_pass: levels.into_iter().all(|pass| pass),
        levels,
    }
}

pub fn evaluate_raw_secondary(fields: &RawAcceptanceFields) -> AcceptanceDecision {
    let a = [
        fields.persistent_world_layer_present,
        fields.partial_observability_present,
        fields.belief_update_verified,
        fields.temporal_delta_prediction_verified,
        !fields.language_is_reasoning_authority,
    ]
    .into_iter()
    .all(|value| value);
    let b = [
        fields.factored_relational_mechanisms_verified,
        fields.fresh_transition_prediction_verified,
        fields.entity_id_invariant_transfer_pass,
        fields.entity_cardinality_generalization_pass,
        fields.novel_relation_topology_transfer_pass,
    ]
    .into_iter()
    .all(|value| value);
    let c = fields.epistemic_aleatoric_separation_pass
        && fields.predictive_uncertainty_collapse_events == 0;
    let d = fields.observation_intervention_separated
        && fields.confounded_causality_resolved
        && fields.false_causal_promotions == 0;
    let e = [
        fields.horizon_1_verified,
        fields.horizon_2_verified,
        fields.horizon_4_verified,
        fields.horizon_8_verified,
        fields.horizon_failures_decomposed,
    ]
    .into_iter()
    .all(|value| value);
    let f = fields.isolated_counterfactuals_verified
        && fields.counterfactual_actual_mutation_events == 0;
    let g = fields.unreachable_shortcut_accepts == 0;
    let h = fields.prediction_residuals_drive_learning
        && fields.causal_refinement_or_composition_verified
        && fields.future_prediction_improves;
    let i = fields.large_world_canary_entities >= 100_000
        && fields.world_memory_full_scans == 0
        && fields.causal_mechanism_full_scans == 0;
    let j = [
        fields.interventional_causality_ablation_pass,
        fields.causal_law_memory_ablation_pass,
        fields.factored_dynamics_ablation_pass,
        fields.epistemic_uncertainty_ablation_pass,
        fields.counterfactual_causal_model_ablation_pass,
        fields.sparse_causal_routing_ablation_pass,
        fields.relational_topology_repair_ablation_pass,
    ]
    .into_iter()
    .all(|value| value);
    let levels = [a, b, c, d, e, f, g, h, i, j];
    AcceptanceDecision {
        sem32_r1_pass: levels.iter().copied().all(|pass| pass),
        levels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! negative_level_canary {
        ($name:ident, $level:expr, $mutation:expr) => {
            #[test]
            fn $name() {
                let mut raw = RawAcceptanceFields::all_pass();
                $mutation(&mut raw);
                let decision = evaluate_raw(&raw);
                assert!(!decision.levels[$level]);
                assert!(!decision.sem32_r1_pass);
                assert_eq!(decision, evaluate_raw_secondary(&raw));
            }
        };
    }

    negative_level_canary!(level_a_truth_table, 0, |raw: &mut RawAcceptanceFields| {
        raw.belief_update_verified = false
    });
    negative_level_canary!(level_b_truth_table, 1, |raw: &mut RawAcceptanceFields| {
        raw.novel_relation_topology_transfer_pass = false
    });
    negative_level_canary!(level_c_truth_table, 2, |raw: &mut RawAcceptanceFields| {
        raw.epistemic_aleatoric_separation_pass = false
    });
    negative_level_canary!(level_d_truth_table, 3, |raw: &mut RawAcceptanceFields| {
        raw.confounded_causality_resolved = false
    });
    negative_level_canary!(level_e_truth_table, 4, |raw: &mut RawAcceptanceFields| {
        raw.horizon_8_verified = false
    });
    negative_level_canary!(level_f_truth_table, 5, |raw: &mut RawAcceptanceFields| {
        raw.isolated_counterfactuals_verified = false
    });
    negative_level_canary!(level_g_truth_table, 6, |raw: &mut RawAcceptanceFields| {
        raw.unreachable_shortcut_accepts = 1
    });
    negative_level_canary!(level_h_truth_table, 7, |raw: &mut RawAcceptanceFields| {
        raw.future_prediction_improves = false
    });
    negative_level_canary!(level_i_truth_table, 8, |raw: &mut RawAcceptanceFields| {
        raw.world_memory_full_scans = 1
    });
    negative_level_canary!(level_j_truth_table, 9, |raw: &mut RawAcceptanceFields| {
        raw.relational_topology_repair_ablation_pass = false
    });

    #[test]
    fn every_raw_dependency_is_acceptance_authority() {
        let baseline = RawAcceptanceFields::all_pass();
        assert!(evaluate_raw(&baseline).sem32_r1_pass);
        let json = serde_json::to_value(&baseline).unwrap();
        let object = json.as_object().unwrap();
        for key in object.keys() {
            let mut candidate = json.clone();
            let value = &mut candidate[key];
            if let Some(boolean) = value.as_bool() {
                *value = serde_json::Value::Bool(!boolean);
            } else if key == "large_world_canary_entities" {
                *value = serde_json::Value::from(99_999_u64);
            } else {
                *value = serde_json::Value::from(1_u64);
            }
            let mutated: RawAcceptanceFields = serde_json::from_value(candidate).unwrap();
            assert!(!evaluate_raw(&mutated).sem32_r1_pass, "{key}");
        }
    }
}
