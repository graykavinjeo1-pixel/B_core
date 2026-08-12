use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::sem20_engine::{run_probe as run_sem20_probe, ProbeRequest as BaseRequest, ProbeResult};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ProbeRequest {
    pub representation_mode: u8,
    pub mechanism_mask: u8,
    pub scale: usize,
    pub seed: u64,
    pub active_feature_mask: u16,
    pub required_assumptions: u8,
    pub local_codebook: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontierProbeResult {
    pub mechanism_mask: u8,
    pub mechanism_count: u32,
    pub scale: usize,
    pub objective_scale: usize,
    pub semantic_checksum: u64,
    pub correct_by_internal_invariants: bool,
    pub algorithm_operations: u64,
    pub representation_operations: u64,
    pub applicability_operations: u64,
    pub total_work_units: u64,
    pub bytes_touched: u64,
    pub allocation_count: u64,
    pub semantic_payload_bytes: u64,
    pub dictionary_bytes: u64,
    pub provenance_bytes: u64,
    pub reconstruction_metadata_bytes: u64,
    pub applicability_envelope_bytes: u64,
    pub applicability_boundary_bytes: u64,
    pub total_semantic_bytes: u64,
    pub active_semantic_bytes: u64,
    pub elapsed_wall_time_ns: u128,
    pub process_cpu_time_ns: u64,
    pub peak_process_rss_bytes: u64,
    pub required_assumptions: u8,
    pub local_codebook_used: bool,
    pub components: Vec<ProbeResult>,
}

pub fn run_probe(request: ProbeRequest) -> Result<FrontierProbeResult, String> {
    if request.representation_mode > 3 {
        return Err("REPRESENTATION_MODE_OUT_OF_RANGE".to_string());
    }
    if request.mechanism_mask == 0 || request.mechanism_mask > 0b1_1111 {
        return Err("MECHANISM_MASK_OUT_OF_RANGE".to_string());
    }
    if !(1..=2048).contains(&request.scale) {
        return Err("SCALE_OUT_OF_RANGE".to_string());
    }
    if request.required_assumptions > 16 {
        return Err("ASSUMPTION_COUNT_OUT_OF_RANGE".to_string());
    }

    let started = Instant::now();
    let mut components = Vec::new();
    for family_code in 0_u8..5 {
        if request.mechanism_mask & (1 << family_code) == 0 {
            continue;
        }
        let component_scale = match family_code {
            2 => (request.scale / 4).max(1),
            3 => request.scale.saturating_mul(2).min(4096),
            _ => request.scale,
        };
        components.push(run_sem20_probe(BaseRequest {
            representation_mode: request.representation_mode,
            family_code,
            scale: component_scale,
            seed: request.seed ^ (u64::from(family_code) << 48),
            active_feature_mask: request.active_feature_mask,
            use_local_codebook: request.local_codebook,
        })?);
    }

    let mechanism_count = components.len() as u32;
    let semantic_checksum = components.iter().fold(0_u64, |acc, component| {
        acc ^ component
            .semantic_checksum
            .rotate_left(u32::from(component.family_code) * 11 + 3)
    });
    let algorithm_operations = components
        .iter()
        .map(|component| component.algorithm_operations)
        .sum::<u64>();
    let representation_operations = components
        .iter()
        .map(|component| component.representation_operations)
        .sum::<u64>();
    let applicability_operations = request.scale as u64
        * (u64::from(request.required_assumptions) * 4 + u64::from(mechanism_count) * 2 + 1);
    let semantic_payload_bytes = components
        .iter()
        .map(|component| component.semantic_payload_bytes)
        .sum::<u64>();
    let dictionary_bytes = components
        .iter()
        .map(|component| component.dictionary_bytes)
        .sum::<u64>();
    let provenance_bytes = components
        .iter()
        .map(|component| component.provenance_bytes)
        .sum::<u64>();
    let reconstruction_metadata_bytes = components
        .iter()
        .map(|component| component.reconstruction_metadata_bytes)
        .sum::<u64>();
    let applicability_envelope_bytes =
        48 + u64::from(request.required_assumptions) * 24 + u64::from(mechanism_count) * 16;
    let applicability_boundary_bytes = 32 + u64::from(mechanism_count) * 12;
    let base_semantic_bytes = components
        .iter()
        .map(|component| component.total_semantic_bytes)
        .sum::<u64>();
    let base_active_bytes = components
        .iter()
        .map(|component| component.active_semantic_bytes)
        .sum::<u64>();

    Ok(FrontierProbeResult {
        mechanism_mask: request.mechanism_mask,
        mechanism_count,
        scale: request.scale,
        objective_scale: request.scale.saturating_mul(mechanism_count as usize),
        semantic_checksum,
        correct_by_internal_invariants: components
            .iter()
            .all(|component| component.correct_by_internal_invariants),
        algorithm_operations,
        representation_operations,
        applicability_operations,
        total_work_units: algorithm_operations
            .saturating_add(representation_operations)
            .saturating_add(applicability_operations),
        bytes_touched: components
            .iter()
            .map(|component| component.bytes_touched)
            .sum::<u64>()
            .saturating_add(applicability_operations.saturating_mul(8)),
        allocation_count: components
            .iter()
            .map(|component| component.allocation_count)
            .sum::<u64>()
            + 2,
        semantic_payload_bytes,
        dictionary_bytes,
        provenance_bytes,
        reconstruction_metadata_bytes,
        applicability_envelope_bytes,
        applicability_boundary_bytes,
        total_semantic_bytes: base_semantic_bytes
            .saturating_add(applicability_envelope_bytes)
            .saturating_add(applicability_boundary_bytes),
        active_semantic_bytes: base_active_bytes
            .saturating_add(applicability_envelope_bytes)
            .saturating_add(applicability_boundary_bytes),
        elapsed_wall_time_ns: started.elapsed().as_nanos(),
        process_cpu_time_ns: 0,
        peak_process_rss_bytes: 0,
        required_assumptions: request.required_assumptions,
        local_codebook_used: request.local_codebook && request.representation_mode >= 2,
        components,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(mode: u8, assumptions: u8) -> ProbeRequest {
        ProbeRequest {
            representation_mode: mode,
            mechanism_mask: 0b1_1011,
            scale: 48,
            seed: 0x21_001,
            active_feature_mask: 0b1111_1111,
            required_assumptions: assumptions,
            local_codebook: mode >= 2,
        }
    }

    #[test]
    fn representation_modes_preserve_composed_semantics() {
        let checksums = (0..4)
            .map(|mode| {
                run_probe(request(mode, 3))
                    .expect("probe")
                    .semantic_checksum
            })
            .collect::<Vec<_>>();
        assert!(checksums.windows(2).all(|pair| pair[0] == pair[1]));
    }

    #[test]
    fn minimized_assumptions_reduce_applicability_cost() {
        let broad = run_probe(request(3, 6)).expect("broad envelope");
        let minimal = run_probe(request(3, 2)).expect("minimal envelope");
        assert_eq!(broad.semantic_checksum, minimal.semantic_checksum);
        assert!(minimal.applicability_operations < broad.applicability_operations);
        assert!(minimal.total_semantic_bytes < broad.total_semantic_bytes);
    }
}
