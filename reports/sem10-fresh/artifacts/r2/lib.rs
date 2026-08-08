use std::mem::size_of;

const KEY_BOUND: usize = 4096;
const WORDS: usize = KEY_BOUND / 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    pub canonical_key: u64,
    pub payload: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub expansions: usize,
    pub deterministic_ops: usize,
    pub ordered_comparisons: usize,
    pub stage_writes: usize,
    pub peak_frontier: usize,
    pub estimated_peak_bytes: usize,
}

fn locate(values: &[u64], key: u64, comparisons: &mut usize) -> Result<usize, usize> {
    let mut low = 0usize;
    let mut high = values.len();
    while low < high {
        let middle = low + (high - low) / 2;
        *comparisons += 1;
        if values[middle] < key {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    if low < values.len() {
        *comparisons += 1;
        if values[low] == key {
            return Ok(low);
        }
    }
    Err(low)
}

pub fn schedule_profiled(states: &[State]) -> (Vec<u64>, Profile) {
    let mut activation = [0u64; WORDS];
    let mut overflow = Vec::new();
    let mut reachable = Vec::new();
    let mut expansions = 0usize;
    let mut ordered_comparisons = 0usize;
    let mut stage_writes = 0usize;
    let mut activation_checks = 0usize;
    for state in states {
        let key = state.canonical_key;
        let newly_activated = if key < KEY_BOUND as u64 {
            activation_checks += 1;
            let index = key as usize;
            let word = index / 64;
            let mask = 1u64 << (index % 64);
            let unseen = activation[word] & mask == 0;
            if unseen {
                activation[word] |= mask;
            }
            unseen
        } else {
            match locate(&overflow, key, &mut ordered_comparisons) {
                Ok(_) => false,
                Err(position) => {
                    overflow.insert(position, key);
                    true
                }
            }
        };
        if !newly_activated {
            continue;
        }
        reachable.push(key);
        stage_writes += 1;
        expansions += 1;
    }
    let deterministic_ops = activation_checks + ordered_comparisons + stage_writes;
    let profile = Profile {
        expansions,
        deterministic_ops,
        ordered_comparisons,
        stage_writes,
        peak_frontier: reachable.len(),
        estimated_peak_bytes: size_of::<[u64; WORDS]>()
            + (overflow.capacity() + reachable.capacity()) * size_of::<u64>(),
    };
    (reachable, profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_partition_preserves_membership_with_unbounded_fallback() {
        let states = [
            State {
                canonical_key: 4,
                payload: 40,
            },
            State {
                canonical_key: 4,
                payload: 41,
            },
            State {
                canonical_key: 9000,
                payload: 90,
            },
            State {
                canonical_key: 9000,
                payload: 91,
            },
        ];
        let (mut keys, profile) = schedule_profiled(&states);
        keys.sort_unstable();
        assert_eq!(keys, vec![4, 9000]);
        assert_eq!(profile.expansions, 2);
        assert_eq!(profile.stage_writes, 2);
    }
}
