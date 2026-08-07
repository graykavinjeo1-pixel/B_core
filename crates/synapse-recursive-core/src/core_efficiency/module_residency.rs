use serde::{Deserialize, Serialize};

use super::runtime_tier::RuntimeTier;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleResidency {
    pub module_name: String,
    pub tier: RuntimeTier,
    pub resident_by_default: bool,
    pub wake_condition: Option<String>,
    pub sleep_condition: Option<String>,
    pub estimated_memory_cost_mb: f32,
    pub estimated_wake_cost_ms: f32,
    pub active_field_cost: usize,
}

impl ModuleResidency {
    pub fn new(
        module_name: impl Into<String>,
        tier: RuntimeTier,
        estimated_memory_cost_mb: f32,
        estimated_wake_cost_ms: f32,
        active_field_cost: usize,
        wake_condition: Option<&str>,
        sleep_condition: Option<&str>,
    ) -> Self {
        Self {
            module_name: module_name.into(),
            tier,
            resident_by_default: tier.resident_by_default(),
            wake_condition: wake_condition.map(str::to_string),
            sleep_condition: sleep_condition.map(str::to_string),
            estimated_memory_cost_mb,
            estimated_wake_cost_ms,
            active_field_cost,
        }
    }
}
