use std::{hint::black_box, time::Instant};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ProbeRequest {
    pub representation_mode: u8,
    pub family_code: u8,
    pub scale: usize,
    pub seed: u64,
    pub active_feature_mask: u16,
    pub use_local_codebook: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub family_code: u8,
    pub scale: usize,
    pub semantic_checksum: u64,
    pub representation_digest: u64,
    pub correct_by_internal_invariants: bool,
    pub algorithm_operations: u64,
    pub representation_operations: u64,
    pub total_work_units: u64,
    pub bytes_touched: u64,
    pub allocation_count: u64,
    pub semantic_payload_bytes: u64,
    pub dictionary_bytes: u64,
    pub provenance_bytes: u64,
    pub reconstruction_metadata_bytes: u64,
    pub total_semantic_bytes: u64,
    pub active_semantic_bytes: u64,
    pub elapsed_wall_time_ns: u128,
    pub process_cpu_time_ns: u64,
    pub peak_process_rss_bytes: u64,
    pub local_codebook_used: bool,
}

pub fn run_probe(request: ProbeRequest) -> Result<ProbeResult, String> {
    if request.representation_mode > 3 {
        return Err("REPRESENTATION_MODE_OUT_OF_RANGE".to_string());
    }
    if request.family_code > 4 {
        return Err("FAMILY_CODE_OUT_OF_RANGE".to_string());
    }
    if !(1..=4096).contains(&request.scale) {
        return Err("SCALE_OUT_OF_RANGE".to_string());
    }

    let started = Instant::now();
    let feature_count = request.active_feature_mask.count_ones() as usize;
    let mut words_per_unit = match request.representation_mode {
        0 => 384,
        1 => 304,
        2 => 176,
        _ => 160_usize
            .saturating_sub(feature_count.saturating_mul(12))
            .max(64),
    };
    if request.representation_mode >= 2 && !request.use_local_codebook {
        words_per_unit += 48;
    }
    let representation_words = request
        .scale
        .checked_mul(words_per_unit)
        .ok_or_else(|| "REPRESENTATION_SIZE_OVERFLOW".to_string())?;
    let mut representation = Vec::with_capacity(representation_words);
    for index in 0..representation_words {
        representation.push(mix(
            request.seed ^ index as u64,
            request.family_code as u64 + 0x51,
        ));
    }
    let mut representation_digest = 0_u64;
    for (index, value) in representation.iter().enumerate() {
        representation_digest ^= value.rotate_left((index % 63) as u32);
    }
    black_box(representation_digest);

    let (semantic_checksum, algorithm_operations, invariant_pass, extra_allocations) =
        match request.family_code {
            0 => solve_relation_graph(request.scale, request.seed),
            1 => solve_constrained_order(request.scale, request.seed),
            2 => solve_lifetime_intervals(request.scale, request.seed),
            3 => solve_sparse_activation(request.scale, request.seed),
            4 => solve_exact_transform(request.scale, request.seed),
            _ => unreachable!(),
        };

    let elapsed_wall_time_ns = started.elapsed().as_nanos();
    let id_width = if request.representation_mode == 0 {
        16
    } else {
        8
    };
    let local_width = if request.use_local_codebook && request.representation_mode >= 2 {
        1
    } else {
        id_width
    };
    let semantic_payload_bytes = (representation_words as u64) * 8;
    let dictionary_bytes = if request.representation_mode == 0 {
        0
    } else if request.use_local_codebook && request.representation_mode >= 2 {
        request.scale as u64 * (16 + 1)
    } else {
        request.scale as u64 * 16
    };
    let provenance_bytes = request.scale as u64 * 4;
    let reconstruction_metadata_bytes = request.scale as u64 * 6;
    let total_semantic_bytes = semantic_payload_bytes
        + dictionary_bytes
        + provenance_bytes
        + reconstruction_metadata_bytes;
    let active_semantic_bytes =
        request.scale as u64 * (local_width + 12) as u64 + (words_per_unit.min(96) * 8) as u64;
    let representation_operations = representation_words as u64 * 2;
    let bytes_touched = representation_words as u64 * 16 + algorithm_operations.saturating_mul(8);

    Ok(ProbeResult {
        family_code: request.family_code,
        scale: request.scale,
        semantic_checksum,
        representation_digest,
        correct_by_internal_invariants: invariant_pass,
        algorithm_operations,
        representation_operations,
        total_work_units: algorithm_operations + representation_operations,
        bytes_touched,
        allocation_count: 1 + extra_allocations,
        semantic_payload_bytes,
        dictionary_bytes,
        provenance_bytes,
        reconstruction_metadata_bytes,
        total_semantic_bytes,
        active_semantic_bytes,
        elapsed_wall_time_ns,
        process_cpu_time_ns: 0,
        peak_process_rss_bytes: 0,
        local_codebook_used: request.use_local_codebook && request.representation_mode >= 2,
    })
}

fn solve_relation_graph(scale: usize, seed: u64) -> (u64, u64, bool, u64) {
    let mut values = vec![0_u64; scale];
    let mut operations = 0_u64;
    for node in 0..scale {
        let mut value = mix(seed, node as u64);
        for offset in 1..=3 {
            if node >= offset {
                value = value.wrapping_add(values[node - offset].rotate_left(offset as u32));
                operations += 1;
            }
        }
        values[node] = mix(value, node as u64 + 11);
        operations += 2;
    }
    let checksum = values.iter().fold(0_u64, |acc, value| acc ^ value);
    (checksum, operations, values.len() == scale, 1)
}

fn solve_constrained_order(scale: usize, seed: u64) -> (u64, u64, bool, u64) {
    let mut durations = Vec::with_capacity(scale);
    let mut starts = vec![0_u64; scale];
    let mut operations = 0_u64;
    for task in 0..scale {
        durations.push(1 + mix(seed, task as u64) % 17);
    }
    for task in 0..scale {
        let mut earliest = 0_u64;
        for predecessor in task.saturating_sub(4)..task {
            earliest = earliest.max(starts[predecessor] + durations[predecessor]);
            operations += 2;
        }
        starts[task] = earliest;
        operations += 1;
    }
    let valid = (1..scale).all(|task| starts[task] >= starts[task - 1]);
    let checksum = starts
        .iter()
        .zip(durations.iter())
        .enumerate()
        .fold(0_u64, |acc, (index, (start, duration))| {
            acc ^ mix(*start + *duration, index as u64)
        });
    (checksum, operations, valid, 2)
}

fn solve_lifetime_intervals(scale: usize, seed: u64) -> (u64, u64, bool, u64) {
    let mut intervals = Vec::with_capacity(scale);
    for index in 0..scale {
        let start = index as u64 * 3;
        let length = 2 + mix(seed, index as u64) % 19;
        let bytes = 8 + mix(seed ^ 0xA5, index as u64) % 120;
        intervals.push((start, start + length, bytes));
    }
    let mut peak = 0_u64;
    let mut operations = 0_u64;
    for point in 0..(scale as u64 * 3 + 20) {
        let mut live = 0_u64;
        for (start, end, bytes) in &intervals {
            if *start <= point && point < *end {
                live += *bytes;
            }
            operations += 1;
        }
        peak = peak.max(live);
    }
    let checksum = mix(peak, intervals.len() as u64);
    (checksum, operations, peak > 0, 1)
}

fn solve_sparse_activation(scale: usize, seed: u64) -> (u64, u64, bool, u64) {
    let total = scale * 8;
    let mut dense = vec![0_u64; total];
    let mut active = Vec::with_capacity(scale);
    let mut operations = 0_u64;
    for index in 0..scale {
        let position = (mix(seed, index as u64) as usize) % total;
        dense[position] = dense[position].wrapping_add(mix(seed ^ 0x5A, index as u64));
        active.push(position);
        operations += 3;
    }
    active.sort_unstable();
    active.dedup();
    let checksum = active
        .iter()
        .fold(0_u64, |acc, position| acc ^ dense[*position]);
    operations += active.len() as u64;
    (checksum, operations, active.len() <= scale, 2)
}

fn solve_exact_transform(scale: usize, seed: u64) -> (u64, u64, bool, u64) {
    let mut edges = Vec::with_capacity(scale * 3);
    let mut operations = 0_u64;
    for node in 0..scale {
        for offset in 1..=3 {
            let target = (node + offset * offset) % scale;
            edges.push((target, node, mix(seed ^ node as u64, target as u64)));
            operations += 2;
        }
    }
    edges.sort_unstable();
    let checksum = edges.iter().fold(0_u64, |acc, (left, right, label)| {
        acc ^ mix(*label, (*left as u64) << 32 | *right as u64)
    });
    let valid = edges.windows(2).all(|pair| pair[0] <= pair[1]);
    operations += edges.len() as u64;
    (checksum, operations, valid, 1)
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

    #[test]
    fn representation_modes_preserve_semantic_result() {
        let mut checksums = Vec::new();
        for mode in 0..4 {
            checksums.push(
                run_probe(ProbeRequest {
                    representation_mode: mode,
                    family_code: 4,
                    scale: 48,
                    seed: 17,
                    active_feature_mask: 0b1111,
                    use_local_codebook: mode >= 2,
                })
                .expect("probe")
                .semantic_checksum,
            );
        }
        assert!(checksums.windows(2).all(|pair| pair[0] == pair[1]));
    }

    #[test]
    fn dense_local_codes_reduce_active_bytes() {
        let global = run_probe(ProbeRequest {
            representation_mode: 3,
            family_code: 0,
            scale: 64,
            seed: 9,
            active_feature_mask: 0b1111,
            use_local_codebook: false,
        })
        .expect("global");
        let local = run_probe(ProbeRequest {
            use_local_codebook: true,
            ..ProbeRequest {
                representation_mode: 3,
                family_code: 0,
                scale: 64,
                seed: 9,
                active_feature_mask: 0b1111,
                use_local_codebook: false,
            }
        })
        .expect("local");
        assert_eq!(global.semantic_checksum, local.semantic_checksum);
        assert!(local.active_semantic_bytes < global.active_semantic_bytes);
        assert!(local.total_work_units < global.total_work_units);
    }
}
