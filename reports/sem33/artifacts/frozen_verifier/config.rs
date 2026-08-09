use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignConfig {
    pub requested_max_autonomous_research_epochs: u64,
    pub configured_max_autonomous_research_epochs: u64,
    pub hard_ceiling: u64,
    pub budget_is_research_semantic_input: bool,
}

impl CampaignConfig {
    pub fn frozen() -> Self {
        Self {
            requested_max_autonomous_research_epochs: 4096,
            configured_max_autonomous_research_epochs: 4096,
            hard_ceiling: 4096,
            budget_is_research_semantic_input: false,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.requested_max_autonomous_research_epochs != 4096
            || self.configured_max_autonomous_research_epochs != 4096
            || self.hard_ceiling != 4096
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
    fn exact_4096_ceiling_is_required() {
        assert!(CampaignConfig::frozen().validate().is_ok());
        let mut invalid = CampaignConfig::frozen();
        invalid.configured_max_autonomous_research_epochs = 4095;
        assert_eq!(invalid.validate(), Err("CAMPAIGN_CONFIG_INVALID"));
    }
}
