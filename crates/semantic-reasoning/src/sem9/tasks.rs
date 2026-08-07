use std::collections::BTreeSet;

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::model::{
    CapabilityFamily, FreshBlindManifest, ReasoningState, SelfEvaluatorTask, VisibleSelfTask,
};

pub const FRESH_BLIND_TASKS: usize = 140;
pub const ADVERSARIAL_BLIND_TASKS: usize = 20;
pub const GENERATOR_VERSION: &str = "SEM9-SELF-BLIND-GENERATOR-1.0.0";

#[derive(Debug, Clone)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

pub fn generate_fresh_tasks(seed: u64) -> Vec<SelfEvaluatorTask> {
    let families = [
        CapabilityFamily::SemanticConcept,
        CapabilityFamily::AdaptiveReasoning,
        CapabilityFamily::MathematicalDerivation,
        CapabilityFamily::Programming,
        CapabilityFamily::DefinitionForaging,
        CapabilityFamily::LanguageAdapter,
        CapabilityFamily::CrossDomainTransfer,
    ];
    let mut rng = Rng(seed);
    let mut tasks = Vec::with_capacity(FRESH_BLIND_TASKS);
    for family in families {
        for within in 0..20 {
            let index = tasks.len();
            tasks.push(build_task(index, family, within, false, &mut rng));
        }
    }
    tasks
}

pub fn generate_adversarial_tasks(seed: u64) -> Vec<SelfEvaluatorTask> {
    let mut rng = Rng(seed ^ 0xa9a9_5e1f_2026_0808);
    (0..ADVERSARIAL_BLIND_TASKS)
        .map(|within| {
            build_task(
                FRESH_BLIND_TASKS + within,
                CapabilityFamily::AdaptiveReasoning,
                within,
                true,
                &mut rng,
            )
        })
        .collect()
}

fn build_task(
    index: usize,
    family: CapabilityFamily,
    within: usize,
    adversarial: bool,
    rng: &mut Rng,
) -> SelfEvaluatorTask {
    let task_id = if adversarial {
        format!("SEM9-ADV-{within:03}")
    } else {
        format!("SEM9-BLIND-{index:03}")
    };
    let unique_count = if adversarial {
        48 + within % 9
    } else {
        62 + (index * 7 + within) % 13
    };
    let duplicate_count = if adversarial {
        unique_count + 12 + within % 5
    } else {
        44 + (index * 11 + within) % 17
    };
    let salt = rng.next() | 1;
    let mut states = Vec::with_capacity(unique_count + duplicate_count);
    for ordinal in 0..unique_count {
        let key = salt
            .wrapping_add((ordinal as u64 + 1).wrapping_mul(0x9e37_79b9))
            .rotate_left((ordinal % 31) as u32);
        states.push(ReasoningState {
            canonical_key: key,
            payload: rng.next(),
        });
    }
    for ordinal in 0..duplicate_count {
        let source = ordinal % unique_count;
        let mut duplicate = states[source].clone();
        duplicate.payload = rng.next();
        states.push(duplicate);
    }
    deterministic_shuffle(&mut states, rng);
    let expected_unique_keys = states
        .iter()
        .map(|state| state.canonical_key)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let visible = VisibleSelfTask {
        task_id,
        capability_family: family,
        opaque_state_schema_sha256: hash_bytes(
            format!("opaque-state-schema:{salt}:{adversarial}").as_bytes(),
        ),
        public_contract_sha256: hash_bytes(
            b"return every semantically distinct reachable state without changing membership",
        ),
        expected_output_included: false,
        hidden_states_included: false,
        benchmark_family_label_exposed_to_patch: false,
        frozen: true,
    };
    SelfEvaluatorTask {
        visible,
        states,
        expected_unique_keys,
        adversarial,
    }
}

fn deterministic_shuffle(states: &mut [ReasoningState], rng: &mut Rng) {
    for index in (1..states.len()).rev() {
        let other = (rng.next() as usize) % (index + 1);
        states.swap(index, other);
    }
}

pub fn build_manifest(
    run_id: &str,
    seed: u64,
    fresh: &[SelfEvaluatorTask],
    adversarial: &[SelfEvaluatorTask],
) -> FreshBlindManifest {
    let fresh_tasks = fresh
        .iter()
        .map(|task| task.visible.clone())
        .collect::<Vec<_>>();
    let adversarial_tasks = adversarial
        .iter()
        .map(|task| task.visible.clone())
        .collect::<Vec<_>>();
    #[derive(Serialize)]
    struct Commitment<'a> {
        run_id: &'a str,
        generator_version: &'a str,
        seed_commitment_sha256: String,
        fresh_tasks: &'a [VisibleSelfTask],
        adversarial_tasks: &'a [VisibleSelfTask],
        self_diagnostic_tasks_included: bool,
        expected_outputs_included: bool,
        hidden_states_included: bool,
        frozen_before_candidate_evaluation: bool,
    }
    let seed_commitment_sha256 = hash_bytes(format!("SEM9-SEED:{seed}").as_bytes());
    let commitment = Commitment {
        run_id,
        generator_version: GENERATOR_VERSION,
        seed_commitment_sha256: seed_commitment_sha256.clone(),
        fresh_tasks: &fresh_tasks,
        adversarial_tasks: &adversarial_tasks,
        self_diagnostic_tasks_included: false,
        expected_outputs_included: false,
        hidden_states_included: false,
        frozen_before_candidate_evaluation: true,
    };
    let manifest_sha256 = hash_serializable(&commitment);
    FreshBlindManifest {
        run_id: run_id.to_string(),
        generator_version: GENERATOR_VERSION.to_string(),
        seed_commitment_sha256,
        fresh_tasks,
        adversarial_tasks,
        self_diagnostic_tasks_included: false,
        expected_outputs_included: false,
        hidden_states_included: false,
        frozen_before_candidate_evaluation: true,
        manifest_sha256,
    }
}

pub fn hash_serializable(value: &impl Serialize) -> String {
    hash_bytes(&serde_json::to_vec(value).expect("serialize"))
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    #[test]
    fn fresh_blind_suite_has_twenty_per_family_and_hides_evaluator_data() {
        let fresh = generate_fresh_tasks(17);
        let adversarial = generate_adversarial_tasks(17);
        let mut counts = BTreeMap::new();
        for task in &fresh {
            *counts
                .entry(task.visible.capability_family)
                .or_insert(0usize) += 1;
            assert!(!task.visible.expected_output_included);
            assert!(!task.visible.hidden_states_included);
            assert!(!task.visible.benchmark_family_label_exposed_to_patch);
        }
        assert_eq!(fresh.len(), 140);
        assert_eq!(counts.len(), 7);
        assert!(counts.values().all(|count| *count == 20));
        assert_eq!(adversarial.len(), 20);
        let manifest = build_manifest("test", 17, &fresh, &adversarial);
        assert!(!manifest.expected_outputs_included);
        assert!(!manifest.hidden_states_included);
        assert!(manifest.frozen_before_candidate_evaluation);
    }

    #[test]
    fn task_outputs_are_semantic_sets_despite_duplicate_payloads() {
        let tasks = generate_fresh_tasks(19);
        for task in tasks {
            let keys = task
                .states
                .iter()
                .map(|state| state.canonical_key)
                .collect::<BTreeSet<_>>();
            assert_eq!(keys.len(), task.expected_unique_keys.len());
            assert!(task.states.len() > keys.len());
        }
    }
}
