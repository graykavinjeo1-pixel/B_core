use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeTier {
    AlwaysOnCore,
    OnDemandCortex,
    ToolLayer,
    ApplicationLayer,
    FrozenScaffold,
}

impl Display for RuntimeTier {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::AlwaysOnCore => "always_on_core",
            Self::OnDemandCortex => "on_demand_cortex",
            Self::ToolLayer => "tool_layer",
            Self::ApplicationLayer => "application_layer",
            Self::FrozenScaffold => "frozen_scaffold",
        };
        write!(formatter, "{value}")
    }
}

impl RuntimeTier {
    pub fn resident_by_default(self) -> bool {
        self == Self::AlwaysOnCore
    }
}
