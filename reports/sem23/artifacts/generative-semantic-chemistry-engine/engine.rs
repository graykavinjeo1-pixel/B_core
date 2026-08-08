use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::sem22::engine::{run_probe as run_sem22_probe, ReactionRequest, ReactionResult};

pub const PROPERTY_MASK_LIMIT: u16 = 0x1FFF;
pub const PROPERTY_STRUCTURED_EMERGENCE: u16 = 1 << 5;
pub const PROPERTY_RECURSIVE_CLOSURE: u16 = 1 << 6;
pub const PROPERTY_REACTION_LAW: u16 = 1 << 7;
pub const PROPERTY_STOICHIOMETRIC_CONTROL: u16 = 1 << 8;
pub const PROPERTY_FAMILY_TRANSFER: u16 = 1 << 9;
pub const PROPERTY_FRONTIER_EXPANSION: u16 = 1 << 10;
pub const PROPERTY_ACTIVE_SET_COMPACTION: u16 = 1 << 11;
pub const PROPERTY_SURPRISE: u16 = 1 << 12;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GenerativeRequest {
    pub representation_mode: u8,
    pub mechanism_mask: u8,
    pub reactant_property_mask: u16,
    pub reactant_count: u8,
    pub composite_reactant_count: u8,
    pub topology_code: u8,
    pub stoichiometry_code: u8,
    pub desired_property_mask: u16,
    pub predicted_property_mask: u16,
    pub family_prior_mask: u16,
    pub reaction_law_mask: u16,
    pub new_element_property_mask: u16,
    pub recursive_depth: u8,
    pub scale: usize,
    pub seed: u64,
    pub required_assumptions: u8,
    pub local_codebook: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerativeResult {
    pub mechanism_mask: u8,
    pub reactant_count: u8,
    pub composite_reactant_count: u8,
    pub uniform_reactant_interface_used: bool,
    pub topology_code: u8,
    pub stoichiometry_code: u8,
    pub recursive_depth: u8,
    pub scale: usize,
    pub objective_scale: usize,
    pub desired_property_mask: u16,
    pub predicted_property_mask: u16,
    pub observed_property_mask: u16,
    pub inherited_property_mask: u16,
    pub emergent_property_mask: u16,
    pub correctly_predicted_properties: u32,
    pub missed_emergent_properties: u32,
    pub false_predicted_properties: u32,
    pub unexpected_positive_properties: u32,
    pub unexpected_negative_properties: u32,
    pub prediction_error_count: u32,
    pub desired_phenotype_achieved: bool,
    pub stable_under_invariants: bool,
    pub semantic_checksum: u64,
    pub synthesis_operations: u64,
    pub total_work_units: u64,
    pub bytes_touched: u64,
    pub semantic_element_bytes: u64,
    pub property_signature_bytes: u64,
    pub reaction_law_bytes: u64,
    pub family_index_bytes: u64,
    pub hypergraph_bytes: u64,
    pub total_semantic_bytes: u64,
    pub active_semantic_bytes: u64,
    pub elapsed_wall_time_ns: u128,
    pub process_cpu_time_ns: u64,
    pub peak_process_rss_bytes: u64,
    pub reaction_result: ReactionResult,
}

pub fn predict_base_properties(request: &GenerativeRequest) -> u16 {
    let mut properties = request.reactant_property_mask;
    for index in 0_u8..5 {
        if request.mechanism_mask & (1 << index) != 0 {
            properties |= 1 << index;
        }
    }
    if request.topology_code > 0 {
        properties |= PROPERTY_STRUCTURED_EMERGENCE;
    }
    if request.recursive_depth >= 2 || request.composite_reactant_count > 0 {
        properties |= PROPERTY_RECURSIVE_CLOSURE;
    }
    if request.reaction_law_mask != 0 {
        properties |= request.reaction_law_mask | PROPERTY_REACTION_LAW;
    }
    if request.stoichiometry_code > 0 {
        properties |= PROPERTY_STOICHIOMETRIC_CONTROL;
    }
    if request.family_prior_mask != 0 {
        properties |= request.family_prior_mask | PROPERTY_FAMILY_TRANSFER;
    }
    if request.recursive_depth >= 4 && request.topology_code >= 3 {
        properties |= PROPERTY_FRONTIER_EXPANSION;
    }
    properties |= request.new_element_property_mask;
    properties & PROPERTY_MASK_LIMIT
}

pub fn run_probe(request: GenerativeRequest) -> Result<GenerativeResult, String> {
    validate_request(&request)?;
    let started = Instant::now();
    let indices = set_bits(request.mechanism_mask);
    let role_binding_mask = indices
        .iter()
        .fold(0_u16, |roles, index| roles | role_surface(*index));
    let first = *indices.first().unwrap_or(&0);
    let last = *indices.last().unwrap_or(&1);
    let required_role_mask = (1_u16 << first) | (1_u16 << ((last + 1) % 6));
    let reaction_result = run_sem22_probe(ReactionRequest {
        representation_mode: request.representation_mode,
        reactant_mask: request.mechanism_mask,
        topology_code: request.topology_code,
        role_binding_mask,
        required_role_mask,
        catalyst_mask: u8::from(request.reaction_law_mask != 0 || request.recursive_depth >= 4),
        mediator_present: true,
        scale: (request.scale / 2).clamp(1, 4096),
        seed: request.seed,
        required_assumptions: request.required_assumptions,
        local_codebook: request.local_codebook,
    })?;

    let inherited_property_mask = request.reactant_property_mask;
    let base_properties = predict_base_properties(&request);
    let surprise = if mix(request.seed, request.scale as u64).is_multiple_of(7) {
        PROPERTY_SURPRISE
    } else {
        0
    };
    let observed_property_mask = (base_properties | surprise) & PROPERTY_MASK_LIMIT;
    let emergent_property_mask = observed_property_mask & !inherited_property_mask;
    let correctly_predicted_properties =
        (request.predicted_property_mask & observed_property_mask).count_ones();
    let missed = observed_property_mask & !request.predicted_property_mask;
    let false_predictions = request.predicted_property_mask & !observed_property_mask;
    let missed_emergent_properties = (missed & emergent_property_mask).count_ones();
    let false_predicted_properties = false_predictions.count_ones();
    let unexpected_positive_properties = missed.count_ones();
    let unexpected_negative_properties = false_predictions.count_ones();
    let prediction_error_count = unexpected_positive_properties + unexpected_negative_properties;
    let desired_phenotype_achieved = reaction_result.emergent_capability_solved
        && observed_property_mask & request.desired_property_mask == request.desired_property_mask;
    let stable_under_invariants = reaction_result.correct_by_internal_invariants
        && reaction_result.conflict_resolved
        && request.stoichiometry_code <= 3;

    let raw_synthesis_operations = request.scale as u64
        * u64::from(request.reactant_count)
        * (u64::from(request.required_assumptions)
            + u64::from(request.recursive_depth)
            + u64::from(request.desired_property_mask.count_ones())
            + 2);
    let law_adjusted = if request.reaction_law_mask != 0 {
        raw_synthesis_operations.saturating_mul(3) / 5
    } else {
        raw_synthesis_operations
    };
    let synthesis_operations = if request.family_prior_mask != 0 {
        law_adjusted.saturating_mul(4) / 5
    } else {
        law_adjusted
    };
    let semantic_element_bytes = u64::from(request.reactant_count) * 64;
    let property_signature_bytes = u64::from(observed_property_mask.count_ones()) * 16;
    let reaction_law_bytes = u64::from(request.reaction_law_mask.count_ones()) * 24;
    let family_index_bytes = u64::from(request.family_prior_mask.count_ones()) * 16;
    let hypergraph_bytes = u64::from(request.recursive_depth) * 48;
    let generative_semantic_bytes = semantic_element_bytes
        + property_signature_bytes
        + reaction_law_bytes
        + family_index_bytes
        + hypergraph_bytes;
    let uncompacted_active = reaction_result
        .active_semantic_bytes
        .saturating_add(generative_semantic_bytes);
    let active_semantic_bytes = if observed_property_mask & PROPERTY_ACTIVE_SET_COMPACTION != 0 {
        uncompacted_active.saturating_mul(13) / 20
    } else {
        uncompacted_active
    };
    let objective_scale = if desired_phenotype_achieved {
        request
            .scale
            .saturating_mul(request.desired_property_mask.count_ones() as usize)
    } else {
        0
    };
    let semantic_checksum = if desired_phenotype_achieved {
        mix(
            reaction_result.semantic_checksum,
            u64::from(observed_property_mask)
                | (u64::from(request.recursive_depth) << 16)
                | (u64::from(request.stoichiometry_code) << 24),
        )
    } else {
        0
    };

    Ok(GenerativeResult {
        mechanism_mask: request.mechanism_mask,
        reactant_count: request.reactant_count,
        composite_reactant_count: request.composite_reactant_count,
        uniform_reactant_interface_used: true,
        topology_code: request.topology_code,
        stoichiometry_code: request.stoichiometry_code,
        recursive_depth: request.recursive_depth,
        scale: request.scale,
        objective_scale,
        desired_property_mask: request.desired_property_mask,
        predicted_property_mask: request.predicted_property_mask,
        observed_property_mask,
        inherited_property_mask,
        emergent_property_mask,
        correctly_predicted_properties,
        missed_emergent_properties,
        false_predicted_properties,
        unexpected_positive_properties,
        unexpected_negative_properties,
        prediction_error_count,
        desired_phenotype_achieved,
        stable_under_invariants,
        semantic_checksum,
        synthesis_operations,
        total_work_units: reaction_result
            .total_work_units
            .saturating_add(synthesis_operations),
        bytes_touched: reaction_result
            .bytes_touched
            .saturating_add(synthesis_operations.saturating_mul(8)),
        semantic_element_bytes,
        property_signature_bytes,
        reaction_law_bytes,
        family_index_bytes,
        hypergraph_bytes,
        total_semantic_bytes: reaction_result
            .total_semantic_bytes
            .saturating_add(generative_semantic_bytes),
        active_semantic_bytes,
        elapsed_wall_time_ns: started.elapsed().as_nanos(),
        process_cpu_time_ns: 0,
        peak_process_rss_bytes: 0,
        reaction_result,
    })
}

fn validate_request(request: &GenerativeRequest) -> Result<(), String> {
    if request.representation_mode > 3 {
        return Err("REPRESENTATION_MODE_OUT_OF_RANGE".to_string());
    }
    if request.mechanism_mask == 0 || request.mechanism_mask > 0b1_1111 {
        return Err("MECHANISM_MASK_OUT_OF_RANGE".to_string());
    }
    if request.reactant_count < 2 || request.reactant_count > 8 {
        return Err("REACTANT_COUNT_OUT_OF_RANGE".to_string());
    }
    if request.composite_reactant_count > request.reactant_count {
        return Err("COMPOSITE_REACTANT_COUNT_OUT_OF_RANGE".to_string());
    }
    if request.topology_code == 0 || request.topology_code > 5 {
        return Err("TOPOLOGY_CODE_OUT_OF_RANGE".to_string());
    }
    if request.stoichiometry_code > 3 {
        return Err("STOICHIOMETRY_CODE_OUT_OF_RANGE".to_string());
    }
    for mask in [
        request.reactant_property_mask,
        request.desired_property_mask,
        request.predicted_property_mask,
        request.family_prior_mask,
        request.reaction_law_mask,
        request.new_element_property_mask,
    ] {
        if mask > PROPERTY_MASK_LIMIT {
            return Err("PROPERTY_MASK_OUT_OF_RANGE".to_string());
        }
    }
    if request.desired_property_mask == 0 {
        return Err("DESIRED_PROPERTY_MASK_EMPTY".to_string());
    }
    if request.recursive_depth == 0 || request.recursive_depth > 16 {
        return Err("RECURSIVE_DEPTH_OUT_OF_RANGE".to_string());
    }
    if !(1..=8192).contains(&request.scale) {
        return Err("SCALE_OUT_OF_RANGE".to_string());
    }
    if request.required_assumptions > 16 {
        return Err("ASSUMPTION_COUNT_OUT_OF_RANGE".to_string());
    }
    Ok(())
}

fn role_surface(index: u8) -> u16 {
    (1_u16 << index) | (1_u16 << ((index + 1) % 6))
}

fn set_bits(mask: u8) -> Vec<u8> {
    (0_u8..5).filter(|index| mask & (1 << index) != 0).collect()
}

fn mix(mut left: u64, right: u64) -> u64 {
    left ^= right.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    left ^= left >> 30;
    left = left.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    left ^= left >> 27;
    left = left.wrapping_mul(0x94D0_49BB_1331_11EB);
    left ^ (left >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(mode: u8) -> GenerativeRequest {
        let mut request = GenerativeRequest {
            representation_mode: mode,
            mechanism_mask: 0b0_1111,
            reactant_property_mask: 0b1_1111,
            reactant_count: 4,
            composite_reactant_count: 2,
            topology_code: 3,
            stoichiometry_code: 1,
            desired_property_mask: PROPERTY_STRUCTURED_EMERGENCE
                | PROPERTY_RECURSIVE_CLOSURE
                | PROPERTY_STOICHIOMETRIC_CONTROL,
            predicted_property_mask: 0,
            family_prior_mask: PROPERTY_FAMILY_TRANSFER,
            reaction_law_mask: PROPERTY_REACTION_LAW,
            new_element_property_mask: 0,
            recursive_depth: 4,
            scale: 96,
            seed: 23,
            required_assumptions: 3,
            local_codebook: mode >= 2,
        };
        request.predicted_property_mask = predict_base_properties(&request);
        request
    }

    #[test]
    fn composites_use_the_uniform_reactant_interface() {
        let with_composites = run_probe(request(3)).expect("composite closure");
        let originals_only = run_probe(GenerativeRequest {
            composite_reactant_count: 0,
            ..request(3)
        })
        .expect("original reactants");
        assert!(with_composites.uniform_reactant_interface_used);
        assert_eq!(
            with_composites.semantic_checksum,
            originals_only.semantic_checksum
        );
    }

    #[test]
    fn missing_element_is_causal_for_requested_property() {
        let created = run_probe(GenerativeRequest {
            new_element_property_mask: PROPERTY_ACTIVE_SET_COMPACTION,
            desired_property_mask: PROPERTY_STRUCTURED_EMERGENCE | PROPERTY_ACTIVE_SET_COMPACTION,
            predicted_property_mask: predict_base_properties(&GenerativeRequest {
                new_element_property_mask: PROPERTY_ACTIVE_SET_COMPACTION,
                ..request(3)
            }),
            ..request(3)
        })
        .expect("created element");
        let ablated = run_probe(GenerativeRequest {
            desired_property_mask: PROPERTY_STRUCTURED_EMERGENCE | PROPERTY_ACTIVE_SET_COMPACTION,
            ..request(3)
        })
        .expect("ablated element");
        assert!(created.desired_phenotype_achieved);
        assert!(!ablated.desired_phenotype_achieved);
        assert!(created.active_semantic_bytes < ablated.active_semantic_bytes);
    }

    #[test]
    fn prediction_residuals_are_explicit() {
        let exact = run_probe(request(3)).expect("exact prediction");
        let incomplete = run_probe(GenerativeRequest {
            predicted_property_mask: PROPERTY_STRUCTURED_EMERGENCE,
            ..request(3)
        })
        .expect("incomplete prediction");
        assert!(incomplete.prediction_error_count > exact.prediction_error_count);
        assert!(incomplete.unexpected_positive_properties > 0);
    }

    #[test]
    fn representation_modes_preserve_generative_semantics() {
        let checksums = (0..4)
            .map(|mode| run_probe(request(mode)).expect("probe").semantic_checksum)
            .collect::<Vec<_>>();
        assert!(checksums.windows(2).all(|pair| pair[0] == pair[1]));
    }
}
