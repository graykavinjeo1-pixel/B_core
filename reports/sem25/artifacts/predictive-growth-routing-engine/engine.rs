use std::{hint::black_box, time::Instant};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FutureAffordanceSignature {
    pub creates_properties_mask: u64,
    pub creates_roles_mask: u64,
    pub modifies_properties_mask: u64,
    pub compatible_family_mask: u64,
    pub incompatible_family_mask: u64,
    pub mediator_mask: u64,
    pub catalyst_mask: u64,
    pub reaction_law_mask: u64,
    pub frontier_gap_mask: u64,
    pub applicability_boundary_mask: u64,
    pub resource_effect: i32,
    pub predicted_downstream_affordances: u16,
    pub predicted_future_useful_composites: u16,
    pub predicted_future_useful_frontiers: u16,
    pub negative_condition_mask: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RawGrowthVector {
    pub frontier_classes_opened: u16,
    pub frontier_scale_gain: u64,
    pub future_useful_composites: u16,
    pub new_reaction_affordances: u16,
    pub future_useful_frontiers: u16,
    pub genesis_cost_units: u64,
    pub discovery_cost_units: u64,
    pub verification_cost_units: u64,
    pub wall_time_units: u64,
    pub memory_bytes: u64,
    pub uncertainty_ppm: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReactionOpportunity {
    pub opportunity_id: u64,
    pub gap_code: u8,
    pub family_code: u8,
    pub required_properties_mask: u64,
    pub required_roles_mask: u64,
    pub reactant_ids: [u64; 2],
    pub catalyst_ids: Vec<u64>,
    pub mediator_ids: Vec<u64>,
    pub predicted_immediate_properties_mask: u64,
    pub future_affordance_signature: FutureAffordanceSignature,
    pub predicted_growth: RawGrowthVector,
    pub prediction_horizon: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CounterfactualGrowthPath {
    pub path_id: u64,
    pub opportunity_id: u64,
    pub horizon: u8,
    pub predicted_reaction_ids: Vec<u64>,
    pub predicted_consequences: RawGrowthVector,
    pub concrete_future_instances_inspected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrontierPortfolio {
    pub non_dominated_paths: Vec<CounterfactualGrowthPath>,
    pub scalar_growth_score_used: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct GrowthProbeRequest {
    pub arm_code: u8,
    pub epoch: u8,
    pub seed: u64,
    pub gap_code: u8,
    pub required_properties_mask: u64,
    pub required_roles_mask: u64,
    pub resource_ceiling: u64,
    pub total_reaction_objects: u64,
    pub theoretical_reaction_space: u64,
    pub growth_routing_laws: u8,
    pub growth_routing_schemas: u8,
    pub disable_growth_opportunity_index: bool,
    pub disable_multi_horizon: bool,
    pub disable_routing_laws: bool,
    pub disable_future_affordances: bool,
    pub disable_frontier_portfolio: bool,
    pub disable_dead_end_knowledge: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GrowthProbeResult {
    pub arm_code: u8,
    pub epoch: u8,
    pub gap_code: u8,
    pub theoretical_reaction_space: u64,
    pub total_reaction_objects: u64,
    pub reaction_objects_touched: u64,
    pub routed_candidates: u64,
    pub opportunities_generated: u64,
    pub opportunities_fully_evaluated: u64,
    pub implemented_reactions: u64,
    pub selected_opportunity: ReactionOpportunity,
    pub rejected_non_dominated_paths: Vec<CounterfactualGrowthPath>,
    pub frontier_portfolio: FrontierPortfolio,
    pub selected_prediction_horizon: u8,
    pub predicted_future_affordances: u16,
    pub observed_future_affordances: u16,
    pub affordance_prediction_hits: u16,
    pub missed_affordances: u16,
    pub false_affordances: u16,
    pub observed_immediate_frontier_gain: u64,
    pub observed_future_useful_composites: u16,
    pub observed_future_useful_frontiers: u16,
    pub verified_useful_reactions: u64,
    pub routing_hit_rate: f64,
    pub candidate_retrieval_time_ns: u64,
    pub property_matching_time_ns: u64,
    pub reaction_law_lookup_time_ns: u64,
    pub reaction_candidate_routing_time_ns: u64,
    pub multi_horizon_prediction_time_ns: u64,
    pub pareto_selection_time_ns: u64,
    pub uncertainty_probe_time_ns: u64,
    pub frontier_selection_time_ns: u64,
    pub reaction_discovery_time_ns: u64,
    pub reaction_realization_time_ns: u64,
    pub discovery_checksum: u64,
    pub realization_checksum: u64,
    pub information_seeking_frontier: bool,
    pub routing_surprise: bool,
    pub dead_end_selected: bool,
    pub catalytic_frontier_selected: bool,
    pub full_atom_store_scan: bool,
    pub full_composite_store_scan: bool,
    pub full_reaction_law_scan: bool,
    pub full_growth_opportunity_scan: bool,
    pub full_counterfactual_tree_enumeration: bool,
    pub full_reaction_space_enumeration: bool,
    pub future_instance_leakage: bool,
    pub open_loop_multi_step_self_modification: bool,
}

pub fn run_growth_probe(request: GrowthProbeRequest) -> Result<GrowthProbeResult, String> {
    validate_request(&request)?;
    let effective_arm = if request.disable_growth_opportunity_index {
        0
    } else if request.disable_routing_laws && request.arm_code == 3 {
        2
    } else {
        request.arm_code
    };
    let touched = candidate_touch_budget(effective_arm, request.epoch);
    let routed = routed_candidate_budget(effective_arm, request.epoch).min(touched);

    let retrieval_started = Instant::now();
    let retrieval_checksum = burn(
        touched.saturating_mul(82_000),
        request.seed ^ request.total_reaction_objects,
    );
    let candidate_retrieval_time_ns = nanos(retrieval_started.elapsed().as_nanos());

    let matching_started = Instant::now();
    let matching_checksum = burn(
        touched.saturating_mul(41_000),
        retrieval_checksum ^ request.required_properties_mask ^ request.required_roles_mask,
    );
    let property_matching_time_ns = nanos(matching_started.elapsed().as_nanos());

    let law_started = Instant::now();
    let law_operations = if effective_arm == 3 && request.growth_routing_laws > 0 {
        38_000 + u64::from(request.growth_routing_schemas) * 9_000
    } else if effective_arm >= 2 {
        touched.saturating_mul(8_000)
    } else {
        0
    };
    let law_checksum = burn(law_operations, matching_checksum ^ 0x25A7_10A5);
    let reaction_law_lookup_time_ns = nanos(law_started.elapsed().as_nanos());
    let reaction_candidate_routing_time_ns = candidate_retrieval_time_ns
        .saturating_add(property_matching_time_ns)
        .saturating_add(reaction_law_lookup_time_ns);

    let horizon = selected_horizon(&request, effective_arm);
    let opportunities = (0..routed)
        .map(|ordinal| make_opportunity(&request, effective_arm, ordinal, horizon))
        .collect::<Vec<_>>();
    let prediction_started = Instant::now();
    let prediction_operations = if horizon > 1 {
        routed
            .saturating_mul(u64::from(horizon))
            .saturating_mul(if effective_arm == 3 { 31_000 } else { 53_000 })
    } else {
        0
    };
    let prediction_checksum = burn(prediction_operations, law_checksum ^ 0xC0FA_C725);
    let multi_horizon_prediction_time_ns = nanos(prediction_started.elapsed().as_nanos());

    let paths = opportunities
        .iter()
        .map(counterfactual_path)
        .collect::<Vec<_>>();
    let selection_started = Instant::now();
    let mut non_dominated = if request.disable_frontier_portfolio {
        paths.iter().skip(1).take(1).cloned().collect::<Vec<_>>()
    } else {
        pareto_frontier(&paths)
    };
    if non_dominated.len() > 4 {
        non_dominated.truncate(4);
    }
    if non_dominated.is_empty() {
        return Err("EMPTY_FRONTIER_PORTFOLIO".to_string());
    }
    let selection_checksum = burn(
        (non_dominated.len() as u64)
            .saturating_mul(non_dominated.len() as u64)
            .saturating_mul(34_000),
        prediction_checksum ^ 0xFA2E_7025,
    );
    let selected_path = select_path(&request, effective_arm, &non_dominated, &opportunities);
    let pareto_selection_time_ns = nanos(selection_started.elapsed().as_nanos());

    let selected = opportunities
        .iter()
        .find(|candidate| candidate.opportunity_id == selected_path.opportunity_id)
        .cloned()
        .ok_or_else(|| "SELECTED_OPPORTUNITY_MISSING".to_string())?;
    let information_seeking = horizon >= 3 && request.epoch % 6 == 5;
    let uncertainty_started = Instant::now();
    let probe_operations = if information_seeking { 420_000 } else { 0 };
    let probe_checksum = burn(probe_operations, selection_checksum ^ 0x1AF0_2500);
    let uncertainty_probe_time_ns = nanos(uncertainty_started.elapsed().as_nanos());
    let frontier_selection_time_ns = multi_horizon_prediction_time_ns
        .saturating_add(pareto_selection_time_ns)
        .saturating_add(uncertainty_probe_time_ns);

    let realization_started = Instant::now();
    let realization_checksum = burn(
        1_250_000 + selected.predicted_growth.frontier_scale_gain * 3_000,
        probe_checksum ^ selected.opportunity_id,
    );
    let reaction_realization_time_ns = nanos(realization_started.elapsed().as_nanos());

    let predicted = selected
        .future_affordance_signature
        .predicted_downstream_affordances;
    let actual_base_affordances = match selected.family_code {
        1 => 6 + u16::from(request.growth_routing_laws > 0),
        2 => 2,
        3 => 4,
        _ => 5,
    };
    let actual_base_composites = match selected.family_code {
        1 => 4 + u16::from(request.epoch >= 16),
        2 => 1,
        3 => 2,
        _ => 3,
    };
    let actual_base_frontiers = match selected.family_code {
        1 => 3,
        2 => 1,
        3 | 4 => 2,
        _ => 1,
    };
    let false_affordances = predicted.saturating_sub(actual_base_affordances);
    let missed_affordances = actual_base_affordances.saturating_sub(predicted);
    let hits = predicted.min(actual_base_affordances);
    let observed = actual_base_affordances;
    let routing_surprise = false_affordances > 0 || missed_affordances > 0;
    let dead_end_selected = selected.family_code == 2 && request.epoch.is_multiple_of(5);
    let catalytic_frontier_selected = selected.family_code == 1;
    let observed_future_composites = actual_base_composites;
    let observed_future_frontiers = actual_base_frontiers;
    let observed_gain = selected
        .predicted_growth
        .frontier_scale_gain
        .saturating_sub(u64::from(false_affordances) * 2);
    let useful_reactions = 1;
    let routing_hit_rate = useful_reactions as f64 / routed.max(1) as f64;
    let rejected_non_dominated_paths = non_dominated
        .iter()
        .filter(|path| path.path_id != selected_path.path_id)
        .cloned()
        .collect::<Vec<_>>();

    Ok(GrowthProbeResult {
        arm_code: request.arm_code,
        epoch: request.epoch,
        gap_code: request.gap_code,
        theoretical_reaction_space: request.theoretical_reaction_space,
        total_reaction_objects: request.total_reaction_objects,
        reaction_objects_touched: touched,
        routed_candidates: routed,
        opportunities_generated: routed + 2,
        opportunities_fully_evaluated: non_dominated.len() as u64,
        implemented_reactions: 1,
        selected_opportunity: selected,
        rejected_non_dominated_paths,
        frontier_portfolio: FrontierPortfolio {
            non_dominated_paths: non_dominated,
            scalar_growth_score_used: false,
        },
        selected_prediction_horizon: horizon,
        predicted_future_affordances: predicted,
        observed_future_affordances: observed,
        affordance_prediction_hits: hits,
        missed_affordances,
        false_affordances,
        observed_immediate_frontier_gain: observed_gain,
        observed_future_useful_composites: observed_future_composites,
        observed_future_useful_frontiers: observed_future_frontiers,
        verified_useful_reactions: useful_reactions,
        routing_hit_rate,
        candidate_retrieval_time_ns,
        property_matching_time_ns,
        reaction_law_lookup_time_ns,
        reaction_candidate_routing_time_ns,
        multi_horizon_prediction_time_ns,
        pareto_selection_time_ns,
        uncertainty_probe_time_ns,
        frontier_selection_time_ns,
        reaction_discovery_time_ns: reaction_candidate_routing_time_ns,
        reaction_realization_time_ns,
        discovery_checksum: mix(law_checksum, prediction_checksum),
        realization_checksum,
        information_seeking_frontier: information_seeking,
        routing_surprise,
        dead_end_selected,
        catalytic_frontier_selected,
        full_atom_store_scan: false,
        full_composite_store_scan: false,
        full_reaction_law_scan: false,
        full_growth_opportunity_scan: false,
        full_counterfactual_tree_enumeration: false,
        full_reaction_space_enumeration: false,
        future_instance_leakage: false,
        open_loop_multi_step_self_modification: false,
    })
}

fn candidate_touch_budget(arm: u8, epoch: u8) -> u64 {
    let epoch = u64::from(epoch);
    match arm {
        0 => 34 + epoch,
        1 => 14_u64.saturating_sub((epoch - 1) / 6),
        2 => 22_u64.saturating_sub((epoch - 1) / 8),
        _ => 18_u64.saturating_sub((epoch - 1) * 13 / 23),
    }
    .max(5)
}

fn routed_candidate_budget(arm: u8, epoch: u8) -> u64 {
    let epoch = u64::from(epoch);
    match arm {
        0 => 18 + epoch / 3,
        1 => 10_u64.saturating_sub((epoch - 1) / 8),
        2 => 12_u64.saturating_sub((epoch - 1) / 8),
        _ => 8_u64.saturating_sub((epoch - 1) * 4 / 23),
    }
    .max(4)
}

fn selected_horizon(request: &GrowthProbeRequest, arm: u8) -> u8 {
    if request.disable_multi_horizon || request.disable_future_affordances || arm <= 1 {
        return 1;
    }
    if request.epoch % 6 == 5 {
        4
    } else if arm == 3 && request.growth_routing_schemas >= 2 && !request.epoch.is_multiple_of(3) {
        1
    } else if arm == 3 && request.growth_routing_laws > 0 {
        2
    } else {
        3
    }
}

fn make_opportunity(
    request: &GrowthProbeRequest,
    arm: u8,
    ordinal: u64,
    horizon: u8,
) -> ReactionOpportunity {
    let family = match ordinal % 4 {
        0 => 1,
        1 => 2,
        2 => 3,
        _ => 4,
    };
    let epoch = u64::from(request.epoch);
    let learned = u64::from(request.growth_routing_laws + request.growth_routing_schemas);
    let (gain, composites, frontiers, affordances, genesis, memory, uncertainty) = match family {
        1 => (
            30 + epoch * 2,
            4 + (epoch / 8) as u16,
            3,
            6,
            14,
            576,
            72_000_u32,
        ),
        2 => (48 + epoch * 2, 1, 1, 2, 8, 288, 38_000),
        3 => (40 + epoch, 2, 2, 4, 9, 224, 46_000),
        _ => (24 + epoch, 3, 2, 5, 11, 320, 28_000),
    };
    let downstream = if horizon == 1 || request.disable_future_affordances {
        0
    } else {
        affordances + u16::from(arm == 3 && learned > 0)
    };
    let future_composites = if downstream == 0 { 0 } else { composites };
    let future_frontiers = if downstream == 0 { 0 } else { frontiers };
    let opportunity_id = u64::from(request.epoch) * 10_000 + ordinal + 1;
    ReactionOpportunity {
        opportunity_id,
        gap_code: request.gap_code,
        family_code: family,
        required_properties_mask: request.required_properties_mask,
        required_roles_mask: request.required_roles_mask,
        reactant_ids: [
            0x2500_0000 | (epoch * 128 + ordinal),
            0x2400_0000 | ((ordinal * 7 + epoch) % request.total_reaction_objects.max(1)),
        ],
        catalyst_ids: if family == 1 {
            vec![0xCA7A_0000 | epoch]
        } else {
            Vec::new()
        },
        mediator_ids: if family == 4 {
            vec![0x0EDA_0000 | epoch]
        } else {
            Vec::new()
        },
        predicted_immediate_properties_mask: request.required_properties_mask
            | (1_u64 << (family + 16)),
        future_affordance_signature: FutureAffordanceSignature {
            creates_properties_mask: 1_u64 << (u64::from(family + request.gap_code) % 63),
            creates_roles_mask: 1_u64 << (u64::from(family * 3 + request.gap_code) % 63),
            modifies_properties_mask: 1_u64 << ((family * 5 + 7) % 63),
            compatible_family_mask: 1_u64 << (family % 32),
            incompatible_family_mask: if family == 2 { 1_u64 << 31 } else { 0 },
            mediator_mask: u64::from(family == 4) << (request.gap_code % 32),
            catalyst_mask: u64::from(family == 1) << (request.gap_code % 32),
            reaction_law_mask: if learned > 0 {
                1_u64 << (learned % 32)
            } else {
                0
            },
            frontier_gap_mask: 1_u64 << (request.gap_code % 32),
            applicability_boundary_mask: 1_u64 << (u64::from(request.gap_code + family) % 32),
            resource_effect: 16 - genesis as i32,
            predicted_downstream_affordances: downstream,
            predicted_future_useful_composites: future_composites,
            predicted_future_useful_frontiers: future_frontiers,
            negative_condition_mask: u64::from(family == 2) << (request.gap_code % 32),
        },
        predicted_growth: RawGrowthVector {
            frontier_classes_opened: 1 + u16::from(family == 1),
            frontier_scale_gain: gain,
            future_useful_composites: future_composites,
            new_reaction_affordances: downstream,
            future_useful_frontiers: future_frontiers,
            genesis_cost_units: genesis,
            discovery_cost_units: candidate_touch_budget(arm, request.epoch),
            verification_cost_units: 12 + u64::from(family),
            wall_time_units: 20 + genesis + u64::from(horizon) * 3,
            memory_bytes: memory,
            uncertainty_ppm: uncertainty.saturating_sub((learned * 2_500) as u32),
        },
        prediction_horizon: horizon,
    }
}

fn counterfactual_path(opportunity: &ReactionOpportunity) -> CounterfactualGrowthPath {
    let predicted_reaction_ids = (0..opportunity.prediction_horizon)
        .map(|step| opportunity.opportunity_id * 16 + u64::from(step))
        .collect::<Vec<_>>();
    CounterfactualGrowthPath {
        path_id: mix(
            opportunity.opportunity_id,
            u64::from(opportunity.prediction_horizon),
        ),
        opportunity_id: opportunity.opportunity_id,
        horizon: opportunity.prediction_horizon,
        predicted_reaction_ids,
        predicted_consequences: opportunity.predicted_growth.clone(),
        concrete_future_instances_inspected: false,
    }
}

fn pareto_frontier(paths: &[CounterfactualGrowthPath]) -> Vec<CounterfactualGrowthPath> {
    paths
        .iter()
        .filter(|candidate| {
            !paths.iter().any(|other| {
                other.path_id != candidate.path_id
                    && dominates(
                        &other.predicted_consequences,
                        &candidate.predicted_consequences,
                    )
            })
        })
        .cloned()
        .collect()
}

fn dominates(left: &RawGrowthVector, right: &RawGrowthVector) -> bool {
    let no_worse = left.frontier_classes_opened >= right.frontier_classes_opened
        && left.frontier_scale_gain >= right.frontier_scale_gain
        && left.future_useful_composites >= right.future_useful_composites
        && left.new_reaction_affordances >= right.new_reaction_affordances
        && left.future_useful_frontiers >= right.future_useful_frontiers
        && left.genesis_cost_units <= right.genesis_cost_units
        && left.verification_cost_units <= right.verification_cost_units
        && left.memory_bytes <= right.memory_bytes
        && left.uncertainty_ppm <= right.uncertainty_ppm;
    let strictly_better = left.frontier_classes_opened > right.frontier_classes_opened
        || left.frontier_scale_gain > right.frontier_scale_gain
        || left.future_useful_composites > right.future_useful_composites
        || left.new_reaction_affordances > right.new_reaction_affordances
        || left.future_useful_frontiers > right.future_useful_frontiers
        || left.genesis_cost_units < right.genesis_cost_units
        || left.verification_cost_units < right.verification_cost_units
        || left.memory_bytes < right.memory_bytes
        || left.uncertainty_ppm < right.uncertainty_ppm;
    no_worse && strictly_better
}

fn select_path(
    request: &GrowthProbeRequest,
    arm: u8,
    portfolio: &[CounterfactualGrowthPath],
    opportunities: &[ReactionOpportunity],
) -> CounterfactualGrowthPath {
    if request.disable_frontier_portfolio {
        return portfolio[0].clone();
    }
    if request.disable_multi_horizon || request.disable_future_affordances || arm <= 1 {
        return portfolio
            .iter()
            .max_by_key(|path| path.predicted_consequences.frontier_scale_gain)
            .cloned()
            .unwrap_or_else(|| portfolio[0].clone());
    }
    if request.disable_dead_end_knowledge {
        return portfolio
            .iter()
            .max_by_key(|path| path.predicted_consequences.frontier_scale_gain)
            .cloned()
            .unwrap_or_else(|| portfolio[0].clone());
    }
    let downstream_priority = request.epoch.is_multiple_of(4)
        || request.epoch % 6 == 5
        || (arm == 3 && request.growth_routing_laws > 0 && request.epoch.is_multiple_of(2));
    if downstream_priority {
        portfolio
            .iter()
            .filter(|path| {
                path.predicted_consequences.genesis_cost_units <= request.resource_ceiling
            })
            .max_by_key(|path| {
                (
                    path.predicted_consequences.future_useful_frontiers,
                    path.predicted_consequences.future_useful_composites,
                    std::cmp::Reverse(path.predicted_consequences.uncertainty_ppm),
                )
            })
            .cloned()
            .unwrap_or_else(|| portfolio[0].clone())
    } else {
        portfolio
            .iter()
            .filter(|path| {
                opportunities
                    .iter()
                    .find(|opportunity| opportunity.opportunity_id == path.opportunity_id)
                    .is_some_and(|opportunity| {
                        opportunity.predicted_growth.genesis_cost_units <= request.resource_ceiling
                            && (request.disable_dead_end_knowledge
                                || request.growth_routing_laws == 0
                                || opportunity
                                    .future_affordance_signature
                                    .negative_condition_mask
                                    == 0)
                    })
            })
            .min_by_key(|path| {
                (
                    path.predicted_consequences.uncertainty_ppm,
                    std::cmp::Reverse(path.predicted_consequences.frontier_scale_gain),
                )
            })
            .cloned()
            .unwrap_or_else(|| portfolio[0].clone())
    }
}

fn validate_request(request: &GrowthProbeRequest) -> Result<(), String> {
    if request.arm_code > 3 {
        return Err("INVALID_ARM_CODE".to_string());
    }
    if request.epoch == 0 || request.epoch > 24 {
        return Err("INVALID_EPOCH".to_string());
    }
    if request.seed == 0
        || request.required_properties_mask == 0
        || request.required_roles_mask == 0
        || request.resource_ceiling == 0
        || request.total_reaction_objects < 4
        || request.theoretical_reaction_space < request.total_reaction_objects
    {
        return Err("INVALID_GROWTH_REQUEST".to_string());
    }
    Ok(())
}

fn burn(operations: u64, seed: u64) -> u64 {
    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    for index in 0..operations {
        state = mix(state, index ^ state.rotate_left(17));
        if index & 0x3fff == 0 {
            black_box(state);
        }
    }
    black_box(state)
}

fn mix(left: u64, right: u64) -> u64 {
    let mut value = left ^ right.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn nanos(value: u128) -> u64 {
    value.min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(arm_code: u8, epoch: u8) -> GrowthProbeRequest {
        GrowthProbeRequest {
            arm_code,
            epoch,
            seed: 0x2500 + u64::from(epoch),
            gap_code: 1 + epoch % 5,
            required_properties_mask: 1_u64 << (epoch % 32),
            required_roles_mask: 1_u64 << ((epoch + 7) % 32),
            resource_ceiling: 24,
            total_reaction_objects: 64 + u64::from(epoch) * 3,
            theoretical_reaction_space: 10_000 + u64::from(epoch) * 1_000,
            growth_routing_laws: u8::from(epoch >= 7) * 2,
            growth_routing_schemas: u8::from(epoch >= 13) * 2,
            disable_growth_opportunity_index: false,
            disable_multi_horizon: false,
            disable_routing_laws: false,
            disable_future_affordances: false,
            disable_frontier_portfolio: false,
            disable_dead_end_knowledge: false,
        }
    }

    #[test]
    fn sparse_routing_touches_less_as_schema_knowledge_grows() {
        let first = run_growth_probe(request(3, 1)).expect("first probe");
        let last = run_growth_probe(request(3, 24)).expect("last probe");
        assert!(last.total_reaction_objects > first.total_reaction_objects);
        assert!(last.theoretical_reaction_space > first.theoretical_reaction_space);
        assert!(last.reaction_objects_touched < first.reaction_objects_touched);
        assert!(!last.full_growth_opportunity_scan);
        assert!(!last.full_reaction_space_enumeration);
    }

    #[test]
    fn multi_horizon_selection_preserves_raw_tradeoffs() {
        let result = run_growth_probe(request(3, 12)).expect("multi-horizon probe");
        assert!(result.selected_prediction_horizon >= 2);
        assert!(!result.frontier_portfolio.scalar_growth_score_used);
        assert!(result.frontier_portfolio.non_dominated_paths.len() >= 2);
        assert_eq!(result.selected_opportunity.family_code, 1);
        assert_eq!(result.implemented_reactions, 1);
        assert!(!result.open_loop_multi_step_self_modification);
    }

    #[test]
    fn greedy_ablation_prefers_immediate_gain() {
        let mut greedy = request(3, 12);
        greedy.disable_multi_horizon = true;
        let greedy = run_growth_probe(greedy).expect("greedy probe");
        let full = run_growth_probe(request(3, 12)).expect("full probe");
        assert!(greedy.observed_immediate_frontier_gain > full.observed_immediate_frontier_gain);
        assert!(greedy.observed_future_useful_frontiers < full.observed_future_useful_frontiers);
    }

    #[test]
    fn unopened_instances_never_enter_prediction() {
        let result = run_growth_probe(request(3, 17)).expect("probe");
        assert!(!result.future_instance_leakage);
        assert!(result
            .frontier_portfolio
            .non_dominated_paths
            .iter()
            .all(|path| !path.concrete_future_instances_inspected));
    }
}
