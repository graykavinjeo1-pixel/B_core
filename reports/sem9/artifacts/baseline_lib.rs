use std::collections::BTreeSet;

pub const ENABLE_EQUIVALENCE_MERGE: bool = false;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    pub canonical_key: u64,
    pub payload: u64,
}

pub fn schedule(states: &[State]) -> (Vec<u64>, usize) {
    let mut seen = BTreeSet::new();
    let mut reachable = BTreeSet::new();
    let mut expansions = 0usize;
    for state in states {
        if ENABLE_EQUIVALENCE_MERGE && !seen.insert(state.canonical_key) {
            continue;
        }
        expansions += 1;
        reachable.insert(state.canonical_key);
    }
    (reachable.into_iter().collect(), expansions)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<State> {
        vec![
            State { canonical_key: 4, payload: 40 },
            State { canonical_key: 4, payload: 41 },
            State { canonical_key: 9, payload: 90 },
        ]
    }

    #[test]
    fn reachable_membership_is_preserved() {
        let (keys, _) = schedule(&fixture());
        assert_eq!(keys, vec![4, 9]);
    }

    #[test]
    fn equivalence_merge_changes_only_operational_cost() {
        let (_, expansions) = schedule(&fixture());
        let expected = if ENABLE_EQUIVALENCE_MERGE { 2 } else { 3 };
        assert_eq!(expansions, expected);
    }

    #[test]
    fn distinct_states_are_never_removed() {
        let states = vec![
            State { canonical_key: 1, payload: 0 },
            State { canonical_key: 2, payload: 0 },
            State { canonical_key: 3, payload: 0 },
        ];
        let (keys, expansions) = schedule(&states);
        assert_eq!(keys, vec![1, 2, 3]);
        assert_eq!(expansions, 3);
    }
}
