use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EfficiencyBudget {
    pub target_device: String,
    pub max_idle_ram_mb: f32,
    pub max_active_ram_mb: f32,
    pub max_peak_ram_mb: f32,
    pub max_active_field_size: usize,
    pub max_candidate_nodes: usize,
    pub full_scan_allowed: bool,
    pub vram_required_mb: f32,
    pub teacher_call_allowed: bool,
}

impl Default for EfficiencyBudget {
    fn default() -> Self {
        Self {
            target_device: "8GB Edge Device".to_string(),
            max_idle_ram_mb: 256.0,
            max_active_ram_mb: 1024.0,
            max_peak_ram_mb: 2048.0,
            max_active_field_size: 128,
            max_candidate_nodes: 512,
            full_scan_allowed: false,
            vram_required_mb: 0.0,
            teacher_call_allowed: false,
        }
    }
}
