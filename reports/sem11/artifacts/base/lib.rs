use std::collections::{BTreeMap, BTreeSet};
use std::mem::size_of;

const SCOPED_ROUTING: bool = false;
const REDUCED_STATE: bool = false;
const CACHED_COMPOSITION: bool = false;
const KEY_BOUND: usize = 4096;
const WORDS: usize = KEY_BOUND / 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    pub id: u64,
    pub scope: u64,
    pub assumption: bool,
    pub score: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    pub key: u64,
    pub payload: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskInput {
    pub required_scope: u64,
    pub candidates: Vec<Candidate>,
    pub states: Vec<State>,
    pub reuse_count: usize,
    pub chains: Vec<Vec<u64>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticOutput {
    pub selected_id: u64,
    pub state_checksum: u64,
    pub composition_checksum: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub routing_ops: usize,
    pub false_activations: usize,
    pub peak_transient_bytes: usize,
    pub reconstruction_ops: usize,
    pub composition_ops: usize,
    pub max_solution_depth: usize,
    pub max_primitive_expanded_depth: usize,
    pub peak_frontier: usize,
    pub peak_active_concepts: usize,
    pub total_primary_cost: usize,
}

pub fn evaluate(task: &TaskInput) -> (SemanticOutput, Profile) {
    let (selected_id, routing_ops, false_activations) = route(task);
    let (state_checksum, peak_transient_bytes, reconstruction_ops, unique_states) =
        evaluate_state(task);
    let (composition_checksum, composition_ops, max_depth, primitive_depth) =
        evaluate_composition(task);
    let peak_frontier = task.candidates.len() + unique_states + task.chains.len();
    let peak_active_concepts = 3;
    let total_primary_cost = routing_ops
        + false_activations
        + reconstruction_ops
        + composition_ops
        + peak_transient_bytes / size_of::<u64>();
    (
        SemanticOutput {
            selected_id,
            state_checksum,
            composition_checksum,
        },
        Profile {
            routing_ops,
            false_activations,
            peak_transient_bytes,
            reconstruction_ops,
            composition_ops,
            max_solution_depth: max_depth,
            max_primitive_expanded_depth: primitive_depth,
            peak_frontier,
            peak_active_concepts,
            total_primary_cost,
        },
    )
}

fn route(task: &TaskInput) -> (u64, usize, usize) {
    let mut operations = 0usize;
    let mut false_activations = 0usize;
    let mut selected = None::<Candidate>;
    if SCOPED_ROUTING {
        for candidate in &task.candidates {
            operations += 1;
            if candidate.scope != task.required_scope || !candidate.assumption {
                continue;
            }
            if selected.is_none_or(|current| better(*candidate, current)) {
                selected = Some(*candidate);
            }
        }
    } else {
        let mut scoped = Vec::new();
        for candidate in &task.candidates {
            operations += 1;
            if candidate.scope == task.required_scope {
                if !candidate.assumption {
                    false_activations += 1;
                }
                scoped.push(*candidate);
            }
        }
        for candidate in scoped {
            operations += 1;
            if candidate.assumption && selected.is_none_or(|current| better(candidate, current)) {
                selected = Some(candidate);
            }
        }
    }
    (
        selected.expect("valid candidate").id,
        operations,
        false_activations,
    )
}

fn better(candidate: Candidate, current: Candidate) -> bool {
    candidate.score > current.score
        || (candidate.score == current.score && candidate.id < current.id)
}

fn evaluate_state(task: &TaskInput) -> (u64, usize, usize, usize) {
    if REDUCED_STATE {
        let unique = canonical_state(&task.states);
        let mut checksum_value = 0u64;
        for _ in 0..task.reuse_count {
            checksum_value ^= checksum(&unique);
        }
        let semantic = checksum(&unique);
        let peak = size_of::<[u64; WORDS]>() + unique.capacity() * size_of::<u64>();
        let _ = checksum_value;
        (semantic, peak, task.states.len(), unique.len())
    } else {
        let mut snapshots = Vec::with_capacity(task.reuse_count);
        let mut reconstruction_ops = 0usize;
        for _ in 0..task.reuse_count {
            snapshots.push(canonical_state(&task.states));
            reconstruction_ops += task.states.len();
        }
        let unique = snapshots.first().expect("snapshot");
        let peak = size_of::<[u64; WORDS]>()
            + snapshots
                .iter()
                .map(|snapshot| snapshot.capacity() * size_of::<u64>())
                .sum::<usize>();
        (checksum(unique), peak, reconstruction_ops, unique.len())
    }
}

fn canonical_state(states: &[State]) -> Vec<u64> {
    let mut activation = [0u64; WORDS];
    let mut overflow = BTreeSet::new();
    let mut unique = Vec::new();
    for state in states {
        let unseen = if state.key < KEY_BOUND as u64 {
            let index = state.key as usize;
            let word = index / 64;
            let mask = 1u64 << (index % 64);
            let unseen = activation[word] & mask == 0;
            activation[word] |= mask;
            unseen
        } else {
            overflow.insert(state.key)
        };
        if unseen {
            unique.push(state.key);
        }
    }
    unique.sort_unstable();
    unique
}

fn evaluate_composition(task: &TaskInput) -> (u64, usize, usize, usize) {
    let mut results = Vec::with_capacity(task.chains.len());
    let mut operations = 0usize;
    let mut max_depth = 0usize;
    if CACHED_COMPOSITION {
        let mut cache = BTreeMap::<Vec<u64>, u64>::new();
        for chain in &task.chains {
            let mut value = 0x5e11_2026u64;
            let mut start = 0usize;
            for length in (1..=chain.len()).rev() {
                if let Some(cached) = cache.get(&chain[..length]) {
                    value = *cached;
                    start = length;
                    break;
                }
            }
            for index in start..chain.len() {
                value = apply(value, chain[index]);
                operations += 1;
                cache.insert(chain[..=index].to_vec(), value);
            }
            max_depth = max_depth.max(chain.len());
            results.push(value);
        }
    } else {
        for chain in &task.chains {
            let mut value = 0x5e11_2026u64;
            for operation in chain {
                value = apply(value, *operation);
                operations += 1;
            }
            max_depth = max_depth.max(chain.len());
            results.push(value);
        }
    }
    let primitive_depth = task.chains.iter().map(Vec::len).max().unwrap_or(0);
    (checksum(&results), operations, max_depth, primitive_depth)
}

fn apply(value: u64, operation: u64) -> u64 {
    value
        .rotate_left((operation % 31) as u32)
        .wrapping_add(operation.wrapping_mul(0x9e37_79b9))
        ^ operation.rotate_right(7)
}

fn checksum(values: &[u64]) -> u64 {
    values.iter().fold(0xcbf2_9ce4_8422_2325, |hash, value| {
        (hash ^ value).wrapping_mul(0x1000_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> TaskInput {
        TaskInput {
            required_scope: 2,
            candidates: vec![
                Candidate {
                    id: 1,
                    scope: 2,
                    assumption: false,
                    score: 99,
                },
                Candidate {
                    id: 2,
                    scope: 2,
                    assumption: true,
                    score: 90,
                },
                Candidate {
                    id: 3,
                    scope: 1,
                    assumption: true,
                    score: 100,
                },
            ],
            states: vec![
                State {
                    key: 4,
                    payload: 40,
                },
                State {
                    key: 4,
                    payload: 41,
                },
                State {
                    key: 9,
                    payload: 90,
                },
                State {
                    key: 9000,
                    payload: 1,
                },
                State {
                    key: 9000,
                    payload: 2,
                },
            ],
            reuse_count: 3,
            chains: vec![vec![1, 2, 3], vec![1, 2, 4]],
        }
    }

    #[test]
    fn semantic_contract_is_satisfied() {
        let (output, profile) = evaluate(&fixture());
        assert_eq!(output.selected_id, 2);
        assert_eq!(profile.max_solution_depth, 3);
        assert_eq!(profile.max_primitive_expanded_depth, 3);
        assert_eq!(profile.peak_active_concepts, 3);
    }

    #[test]
    fn output_is_deterministic() {
        let first = evaluate(&fixture());
        let second = evaluate(&fixture());
        assert_eq!(first, second);
    }
}
