use serde::{Deserialize, Serialize};

pub const REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS: u64 = 4096;
pub const CONFIGURED_MAX_AUTONOMOUS_RESEARCH_EPOCHS: u64 = 4096;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignConfig {
    pub requested_max_autonomous_research_epochs: u64,
    pub configured_max_autonomous_research_epochs: u64,
    pub budget_is_research_semantic_input: bool,
}

impl CampaignConfig {
    pub fn frozen() -> Self {
        Self {
            requested_max_autonomous_research_epochs: REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS,
            configured_max_autonomous_research_epochs: CONFIGURED_MAX_AUTONOMOUS_RESEARCH_EPOCHS,
            budget_is_research_semantic_input: false,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.requested_max_autonomous_research_epochs != REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS
            || self.configured_max_autonomous_research_epochs
                != REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS
            || self.budget_is_research_semantic_input
        {
            return Err("CAMPAIGN_CONFIG_INVALID");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_budget_contract_passes() {
        assert_eq!(CampaignConfig::frozen().validate(), Ok(()));
    }

    #[test]
    fn any_ceiling_drift_fails_closed() {
        let mut config = CampaignConfig::frozen();
        config.configured_max_autonomous_research_epochs = 512;
        assert_eq!(config.validate(), Err("CAMPAIGN_CONFIG_INVALID"));
    }
}
