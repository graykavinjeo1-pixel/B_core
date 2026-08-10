pub const CAMPAIGN_ID: &str = "SEM37-R1-SHIFT-AWARE-EXTERNAL-MECHANISM-TRANSFER-0001";
pub const BRANCH: &str = "codex/sem37-r1-shift-aware-transfer";
pub const CAPABILITY_PREDECESSOR: &str = "b33386e7a8793c5c27e2c2df3e19db0e6e04d0f4";
pub const HISTORICAL_SEM37_COMMIT: &str = "4ab8fb474725b22fe0ef53dba60df2c53f5e6511";
pub const MAX_AUTONOMOUS_RESEARCH_EPOCHS: u64 = 4096;
pub const REPORT_DIR: &str = "reports/sem37-r1";
pub const SEM36_ENGINE_PATH: &str = "crates/semantic-reasoning/src/sem36/engine.rs";
pub const SEM36_ACCEPTANCE_PATH: &str = "crates/semantic-reasoning/src/sem36/acceptance.rs";
pub const SEM36_ENGINE_SHA256: &str =
    "a14e5065c1ce830c78bf16937110aab5612f820344d87e3eca67b00db1ba6fcf";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_containment_budget_is_preserved() {
        assert_eq!(MAX_AUTONOMOUS_RESEARCH_EPOCHS, 4096);
    }
}
