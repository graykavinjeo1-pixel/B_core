pub const CAMPAIGN_ID: &str = "SEM37-THIRD-PARTY-DYNAMICAL-WORLD-EXTERNAL-VALIDITY-0001";
pub const BRANCH: &str = "codex/sem37-external-validity";
pub const PREDECESSOR: &str = "b33386e7a8793c5c27e2c2df3e19db0e6e04d0f4";
pub const MAX_AUTONOMOUS_RESEARCH_EPOCHS: u64 = 4096;
pub const REPORT_DIR: &str = "reports/sem37";
pub const EVALUATOR_CONTRACT_VERSION: &str = "SEM37_EXTERNAL_EVALUATOR_CONTRACT_1";
pub const SEM36_ENGINE_PATH: &str = "crates/semantic-reasoning/src/sem36/engine.rs";
pub const SEM36_ACCEPTANCE_PATH: &str = "crates/semantic-reasoning/src/sem36/acceptance.rs";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn containment_budget_is_exactly_4096() {
        assert_eq!(MAX_AUTONOMOUS_RESEARCH_EPOCHS, 4096);
    }
}
