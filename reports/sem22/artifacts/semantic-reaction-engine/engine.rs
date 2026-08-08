use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::sem21::engine::{
    run_probe as run_sem21_probe, FrontierProbeResult, ProbeRequest as Sem21Request,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ReactionRequest {
    pub representation_mode: u8,
    pub reactant_mask: u8,
    pub topology_code: u8,
    pub role_binding_mask: u16,
    pub required_role_mask: u16,
    pub catalyst_mask: u8,
    pub mediator_present: bool,
    pub scale: usize,
    pub seed: u64,
    pub required_assumptions: u8,
    pub local_codebook: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionResult {
    pub reactant_mask: u8,
    pub reactant_count: u32,
    pub topology_code: u8,
    pub scale: usize,
    pub objective_scale: usize,
    pub provided_role_mask: u16,
    pub required_role_mask: u16,
    pub role_binding_mask: u16,
    pub role_bindings_complete: bool,
    pub conflict_detected: bool,
    pub conflict_resolved: bool,
    pub catalyst_used: bool,
    pub mediator_used: bool,
    pub emergent_capability_solved: bool,
    pub naive_independent_execution: bool,
    pub correct_by_internal_invariants: bool,
    pub semantic_checksum: u64,
    pub algorithm_operations: u64,
    pub representation_operations: u64,
    pub reaction_operations: u64,
    pub total_work_units: u64,
    pub bytes_touched: u64,
    pub allocation_count: u64,
    pub composition_contract_bytes: u64,
    pub reaction_ir_bytes: u64,
    pub reactivity_index_bytes: u64,
    pub catalysis_bytes: u64,
    pub composition_motif_bytes: u64,
    pub total_semantic_bytes: u64,
    pub active_semantic_bytes: u64,
    pub elapsed_wall_time_ns: u128,
    pub process_cpu_time_ns: u64,
    pub peak_process_rss_bytes: u64,
    pub component_result: FrontierProbeResult,
}

pub fn run_probe(request: ReactionRequest) -> Result<ReactionResult, String> {
    if request.representation_mode > 3 {
        return Err("REPRESENTATION_MODE_OUT_OF_RANGE".to_string());
    }
    if request.reactant_mask == 0 || request.reactant_mask > 0b1_1111 {
        return Err("REACTANT_MASK_OUT_OF_RANGE".to_string());
    }
    if request.topology_code > 5 {
        return Err("TOPOLOGY_CODE_OUT_OF_RANGE".to_string());
    }
    if request.required_role_mask == 0 || request.required_role_mask > 0b11_1111 {
        return Err("REQUIRED_ROLE_MASK_OUT_OF_RANGE".to_string());
    }
    if request.role_binding_mask > 0b11_1111 {
        return Err("ROLE_BINDING_MASK_OUT_OF_RANGE".to_string());
    }
    if !(1..=4096).contains(&request.scale) {
        return Err("SCALE_OUT_OF_RANGE".to_string());
    }
    if request.required_assumptions > 16 {
        return Err("ASSUMPTION_COUNT_OUT_OF_RANGE".to_string());
    }

    let started = Instant::now();
    let reactant_count = request.reactant_mask.count_ones();
    let mut provided_role_mask = 0_u16;
    for index in 0_u8..5 {
        if request.reactant_mask & (1 << index) != 0 {
            provided_role_mask |= role_surface(index);
        }
    }
    let role_bindings_complete = request.role_binding_mask & request.required_role_mask
        == request.required_role_mask
        && request.role_binding_mask & !provided_role_mask == 0;
    let conflict_detected = (request.reactant_mask & 0b0_1001) == 0b0_1001
        || (request.reactant_mask & 0b1_0010) == 0b1_0010;
    let conflict_resolved = !conflict_detected || request.mediator_present;
    let catalyst_required = request.topology_code >= 4 && reactant_count >= 4;
    let catalyst_satisfied = !catalyst_required || request.catalyst_mask != 0;
    let structured = request.topology_code > 0 && reactant_count >= 2;
    let emergent_capability_solved =
        structured && role_bindings_complete && conflict_resolved && catalyst_satisfied;
    let naive_independent_execution = request.topology_code == 0;

    let component_result = run_sem21_probe(Sem21Request {
        representation_mode: request.representation_mode,
        mechanism_mask: request.reactant_mask,
        scale: (request.scale / 2).max(1),
        seed: request.seed,
        active_feature_mask: 0b11_1111_1111,
        required_assumptions: request.required_assumptions,
        local_codebook: request.local_codebook,
    })?;

    let raw_reaction_operations = request.scale as u64
        * u64::from(reactant_count)
        * (u64::from(request.required_assumptions)
            + u64::from(request.topology_code)
            + u64::from(request.required_role_mask.count_ones())
            + 2);
    let reaction_operations = if request.catalyst_mask != 0 {
        raw_reaction_operations.saturating_mul(3) / 5
    } else {
        raw_reaction_operations
    } + u64::from(request.mediator_present) * request.scale as u64;
    let objective_scale = if emergent_capability_solved {
        request
            .scale
            .saturating_mul(request.required_role_mask.count_ones() as usize)
    } else {
        0
    };
    let composition_contract_bytes = u64::from(reactant_count) * 72;
    let reaction_ir_bytes = 96 + u64::from(reactant_count) * 40;
    let reactivity_index_bytes = 64 + u64::from(request.required_role_mask.count_ones()) * 16;
    let catalysis_bytes = u64::from(request.catalyst_mask != 0) * 80;
    let composition_motif_bytes = u64::from(request.topology_code > 0) * 64;
    let reaction_semantic_bytes = composition_contract_bytes
        + reaction_ir_bytes
        + reactivity_index_bytes
        + catalysis_bytes
        + composition_motif_bytes;
    let semantic_checksum = if emergent_capability_solved {
        mix(
            component_result.semantic_checksum,
            u64::from(request.reactant_mask)
                | (u64::from(request.topology_code) << 8)
                | (u64::from(request.required_role_mask) << 16),
        )
    } else {
        0
    };

    Ok(ReactionResult {
        reactant_mask: request.reactant_mask,
        reactant_count,
        topology_code: request.topology_code,
        scale: request.scale,
        objective_scale,
        provided_role_mask,
        required_role_mask: request.required_role_mask,
        role_binding_mask: request.role_binding_mask,
        role_bindings_complete,
        conflict_detected,
        conflict_resolved,
        catalyst_used: request.catalyst_mask != 0,
        mediator_used: request.mediator_present,
        emergent_capability_solved,
        naive_independent_execution,
        correct_by_internal_invariants: component_result.correct_by_internal_invariants,
        semantic_checksum,
        algorithm_operations: component_result.algorithm_operations,
        representation_operations: component_result.representation_operations,
        reaction_operations,
        total_work_units: component_result
            .total_work_units
            .saturating_add(reaction_operations),
        bytes_touched: component_result
            .bytes_touched
            .saturating_add(reaction_operations.saturating_mul(8)),
        allocation_count: component_result.allocation_count + u64::from(reactant_count) + 2,
        composition_contract_bytes,
        reaction_ir_bytes,
        reactivity_index_bytes,
        catalysis_bytes,
        composition_motif_bytes,
        total_semantic_bytes: component_result
            .total_semantic_bytes
            .saturating_add(reaction_semantic_bytes),
        active_semantic_bytes: component_result
            .active_semantic_bytes
            .saturating_add(reaction_semantic_bytes),
        elapsed_wall_time_ns: started.elapsed().as_nanos(),
        process_cpu_time_ns: 0,
        peak_process_rss_bytes: 0,
        component_result,
    })
}

fn role_surface(index: u8) -> u16 {
    (1_u16 << index) | (1_u16 << ((index + 1) % 6))
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

    fn request(topology_code: u8, catalyst_mask: u8) -> ReactionRequest {
        ReactionRequest {
            representation_mode: 3,
            reactant_mask: 0b0_1111,
            topology_code,
            role_binding_mask: 0b01_1111,
            required_role_mask: 0b01_0101,
            catalyst_mask,
            mediator_present: true,
            scale: 64,
            seed: 22,
            required_assumptions: 3,
            local_codebook: true,
        }
    }

    #[test]
    fn structured_reaction_is_emergent_over_naive_execution() {
        let structured = run_probe(request(3, 0)).expect("structured");
        let naive = run_probe(request(0, 0)).expect("naive");
        assert!(structured.emergent_capability_solved);
        assert!(!naive.emergent_capability_solved);
        assert!(structured.objective_scale > naive.objective_scale);
    }

    #[test]
    fn catalyst_enables_higher_order_reaction_and_reduces_cost() {
        let catalyzed = run_probe(request(4, 1)).expect("catalyzed");
        let ablated = run_probe(request(4, 0)).expect("ablated");
        assert!(catalyzed.emergent_capability_solved);
        assert!(!ablated.emergent_capability_solved);
        assert!(catalyzed.reaction_operations < ablated.reaction_operations);
    }

    #[test]
    fn representation_modes_preserve_reaction_semantics() {
        let checksums = (0..4)
            .map(|mode| {
                run_probe(ReactionRequest {
                    representation_mode: mode,
                    ..request(3, 0)
                })
                .expect("reaction")
                .semantic_checksum
            })
            .collect::<Vec<_>>();
        assert!(checksums.windows(2).all(|pair| pair[0] == pair[1]));
    }
}
