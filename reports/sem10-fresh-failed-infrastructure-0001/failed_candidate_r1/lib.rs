use std::mem::size_of;

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
    let mut reachable = Vec::new();
    let mut expansions = 0usize;
    let mut ordered_comparisons = 0usize;
    let mut stage_writes = 0usize;
    for state in states {
        let Err(position) = locate(&reachable, state.canonical_key, &mut ordered_comparisons)
        else {
            continue;
        };
        reachable.insert(position, state.canonical_key);
        stage_writes += 1;
        expansions += 1;
    }
    let deterministic_ops = ordered_comparisons + stage_writes;
    let profile = Profile {
        expansions,
        deterministic_ops,
        ordered_comparisons,
        stage_writes,
        peak_frontier: reachable.len(),
        estimated_peak_bytes: reachable.capacity() * size_of::<u64>(),
    };
    (reachable, profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composes_membership_stages_without_semantic_change() {
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
                canonical_key: 9,
                payload: 90,
            },
        ];
        let (keys, profile) = schedule_profiled(&states);
        assert_eq!(keys, vec![4, 9]);
        assert_eq!(profile.expansions, 2);
        assert_eq!(profile.stage_writes, 2);
    }
}
