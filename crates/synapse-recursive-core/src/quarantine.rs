use serde::{Deserialize, Serialize};

/// Immutable Stage S0 status for the inherited recursive-improvement stack.
///
/// This module is intentionally descriptive only. It provides no method that
/// can propose, apply, merge, commit, push, repair, or execute a mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecursiveImprovementQuarantine {
    pub stage: &'static str,
    pub mode: &'static str,
    pub observe_enabled: bool,
    pub measure_enabled: bool,
    pub proposal_generation_enabled: bool,
    pub source_patching_enabled: bool,
    pub sandbox_apply_enabled: bool,
    pub auto_apply_enabled: bool,
    pub auto_merge_enabled: bool,
    pub auto_commit_enabled: bool,
    pub auto_push_enabled: bool,
    pub external_provider_repair_enabled: bool,
    pub recursive_benchmark_mutation_enabled: bool,
    pub network_enabled: bool,
    pub external_llm_enabled: bool,
}

pub const STAGE_S0_QUARANTINE: RecursiveImprovementQuarantine = RecursiveImprovementQuarantine {
    stage: "S0",
    mode: "OBSERVE_MEASURE_ONLY",
    observe_enabled: true,
    measure_enabled: true,
    proposal_generation_enabled: false,
    source_patching_enabled: false,
    sandbox_apply_enabled: false,
    auto_apply_enabled: false,
    auto_merge_enabled: false,
    auto_commit_enabled: false,
    auto_push_enabled: false,
    external_provider_repair_enabled: false,
    recursive_benchmark_mutation_enabled: false,
    network_enabled: false,
    external_llm_enabled: false,
};

pub const QUARANTINED_MODULES: &[&str] = &[
    "autonomy_governor",
    "closed_growth_cycle",
    "code_growth",
    "coding_knowledge",
    "core_efficiency",
    "embryo",
    "low_risk_loop",
    "patch_feedback",
    "patch_sandbox",
    "self_development_runtime",
];

pub fn status() -> RecursiveImprovementQuarantine {
    STAGE_S0_QUARANTINE
}

#[cfg(test)]
mod tests {
    use super::{status, QUARANTINED_MODULES};

    #[test]
    fn stage_s0_quarantine_is_observe_measure_only() {
        let policy = status();
        assert_eq!(policy.stage, "S0");
        assert_eq!(policy.mode, "OBSERVE_MEASURE_ONLY");
        assert!(policy.observe_enabled);
        assert!(policy.measure_enabled);
        assert!(!policy.proposal_generation_enabled);
        assert!(!policy.source_patching_enabled);
        assert!(!policy.sandbox_apply_enabled);
        assert!(!policy.auto_apply_enabled);
        assert!(!policy.auto_merge_enabled);
        assert!(!policy.auto_commit_enabled);
        assert!(!policy.auto_push_enabled);
        assert!(!policy.external_provider_repair_enabled);
        assert!(!policy.recursive_benchmark_mutation_enabled);
        assert!(!policy.network_enabled);
        assert!(!policy.external_llm_enabled);
        assert_eq!(QUARANTINED_MODULES.len(), 10);
    }
}
