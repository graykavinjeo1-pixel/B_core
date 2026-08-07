use std::collections::BTreeSet;

use serde::Serialize;

use crate::sem9::{
    model::{FreshBlindManifest, SelfEvaluatorTask, VisibleSelfTask},
    tasks::{generate_adversarial_tasks, generate_fresh_tasks, hash_bytes, hash_serializable},
};

pub const RUN0002_ID: &str = "SEM9-R1-RUN-0002";
pub const RUN0002_SEED: u64 = 0x5e91_2026_0810;
pub const R1_GENERATOR_VERSION: &str = "SEM9-R1-FRESH-GENERATOR-1.0.0";

pub fn generate_run0002_tasks() -> (Vec<SelfEvaluatorTask>, Vec<SelfEvaluatorTask>) {
    let mut fresh = generate_fresh_tasks(RUN0002_SEED);
    let mut adversarial = generate_adversarial_tasks(RUN0002_SEED);
    for (index, task) in fresh.iter_mut().enumerate() {
        task.visible.task_id = format!("SEM9-R1-BLIND-{index:03}");
        task.visible.opaque_state_schema_sha256 = hash_bytes(
            format!(
                "R1-FRESH:{}:{}:{}",
                RUN0002_SEED, index, task.visible.opaque_state_schema_sha256
            )
            .as_bytes(),
        );
    }
    for (index, task) in adversarial.iter_mut().enumerate() {
        task.visible.task_id = format!("SEM9-R1-ADV-{index:03}");
        task.visible.opaque_state_schema_sha256 = hash_bytes(
            format!(
                "R1-ADVERSARIAL:{}:{}:{}",
                RUN0002_SEED, index, task.visible.opaque_state_schema_sha256
            )
            .as_bytes(),
        );
    }
    (fresh, adversarial)
}

pub fn build_run0002_manifest(
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
    let seed_commitment_sha256 = hash_bytes(format!("SEM9-R1-SEED:{RUN0002_SEED}").as_bytes());
    #[derive(Serialize)]
    struct Commitment<'a> {
        run_id: &'a str,
        generator_version: &'a str,
        seed_commitment_sha256: &'a str,
        fresh_tasks: &'a [VisibleSelfTask],
        adversarial_tasks: &'a [VisibleSelfTask],
        self_diagnostic_tasks_included: bool,
        expected_outputs_included: bool,
        hidden_states_included: bool,
        frozen_before_candidate_evaluation: bool,
    }
    let commitment = Commitment {
        run_id: RUN0002_ID,
        generator_version: R1_GENERATOR_VERSION,
        seed_commitment_sha256: &seed_commitment_sha256,
        fresh_tasks: &fresh_tasks,
        adversarial_tasks: &adversarial_tasks,
        self_diagnostic_tasks_included: false,
        expected_outputs_included: false,
        hidden_states_included: false,
        frozen_before_candidate_evaluation: true,
    };
    let manifest_sha256 = hash_serializable(&commitment);
    FreshBlindManifest {
        run_id: RUN0002_ID.to_string(),
        generator_version: R1_GENERATOR_VERSION.to_string(),
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

pub fn verify_freshness_against_run0001(
    run0001: &FreshBlindManifest,
    run0002: &FreshBlindManifest,
) -> Result<(), String> {
    let old_ids = run0001
        .fresh_tasks
        .iter()
        .chain(&run0001.adversarial_tasks)
        .map(|task| task.task_id.as_str())
        .collect::<BTreeSet<_>>();
    let old_schemas = run0001
        .fresh_tasks
        .iter()
        .chain(&run0001.adversarial_tasks)
        .map(|task| task.opaque_state_schema_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let reused_ids = run0002
        .fresh_tasks
        .iter()
        .chain(&run0002.adversarial_tasks)
        .filter(|task| old_ids.contains(task.task_id.as_str()))
        .count();
    let reused_schemas = run0002
        .fresh_tasks
        .iter()
        .chain(&run0002.adversarial_tasks)
        .filter(|task| old_schemas.contains(task.opaque_state_schema_sha256.as_str()))
        .count();
    if reused_ids != 0 || reused_schemas != 0 {
        return Err(format!(
            "RUN0002_NOT_FRESH:ids={reused_ids}:schemas={reused_schemas}"
        ));
    }
    if run0002.fresh_tasks.len() != 140 || run0002.adversarial_tasks.len() != 20 {
        return Err("RUN0002_TASK_COUNT_INVALID".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run0002_has_new_ids_and_is_answer_free() {
        let (fresh, adversarial) = generate_run0002_tasks();
        let manifest = build_run0002_manifest(&fresh, &adversarial);
        assert_eq!(manifest.fresh_tasks.len(), 140);
        assert_eq!(manifest.adversarial_tasks.len(), 20);
        assert!(manifest
            .fresh_tasks
            .iter()
            .all(|task| task.task_id.starts_with("SEM9-R1-BLIND-")));
        assert!(!manifest.expected_outputs_included);
        assert!(!manifest.hidden_states_included);
    }
}
