pub const CAMPAIGN_ID: &str = "SEM36-AUTONOMOUS-EPISTEMIC-FRONTIER-DISCOVERY-0001";
pub const BRANCH: &str = "codex/sem36-epistemic-frontier";
pub const PREDECESSOR: &str = "2b2d7b6ecc48b6b677a2fc8ac3277c41353b968f";
pub const MAX_AUTONOMOUS_RESEARCH_EPOCHS: u64 = 4096;
pub const DEVELOPMENT_SEED: u64 = 4_723_611_905_334_277_891;
pub const FINAL_WORLD_SEED: u64 = 13_607_921_844_390_117_643;
pub const NOVEL_PREDICTION_SEED: u64 = 9_841_507_331_260_774_109;
pub const DEVELOPMENT_WORLD_COUNT: usize = 18;
pub const FINAL_WORLD_COUNT: usize = 24;
pub const NOVEL_PREDICTION_WORLD_COUNT: usize = 12;
pub const REPORT_DIR: &str = "reports/sem36";
pub const CONTRACT_VERSION: &str = "SEM36_BLIND_EPISTEMIC_FRONTIER_VERIFIER_1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_ceiling_is_exactly_4096() {
        assert_eq!(MAX_AUTONOMOUS_RESEARCH_EPOCHS, 4096);
    }
}
