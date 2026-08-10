pub const CAMPAIGN_ID: &str = "SEM37-R5-INTERVENTIONAL-PATH-IDENTIFICATION-0001";
pub const AUTHORITATIVE_PREDECESSOR: &str = "b33386e7a8793c5c27e2c2df3e19db0e6e04d0f4";
pub const HISTORICAL_R4_SEAL: &str = "a9d501d50b8ad82109606665b2e944db7b5e3c2f";
pub const MAX_AUTONOMOUS_RESEARCH_EPOCHS: u64 = 4096;
pub const CAMPAIGN_SEED: u64 = 3_705_202_608;
pub const DEV_SET: &str = "R5_DEV_H";
pub const FINAL_SET: &str = "R5_FINAL_I";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_budget_contract_is_frozen() {
        assert_eq!(MAX_AUTONOMOUS_RESEARCH_EPOCHS, 4096);
        assert_eq!(CAMPAIGN_SEED, 3_705_202_608);
    }
}
