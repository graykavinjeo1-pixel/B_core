use serde::{Deserialize, Serialize};

use super::active_field_budget::ActiveFieldBudgetReport;
use super::cold_storage_policy::ColdStorageReport;
use super::device_profile::DeviceProfileReport;
use super::efficiency_budget::EfficiencyBudget;
use super::memory_scale_audit::MemoryScaleAuditReport;
use super::residency_registry::ResidencySummary;
use super::wake_sleep_controller::WakeSleepReport;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoreEfficiencyReport {
    pub budget: EfficiencyBudget,
    pub residency: ResidencySummary,
    pub active_field_budget: ActiveFieldBudgetReport,
    pub memory_scale_audit: MemoryScaleAuditReport,
    pub device_profile: DeviceProfileReport,
    pub wake_report: WakeSleepReport,
    pub sleep_report: WakeSleepReport,
    pub cold_storage: ColdStorageReport,
    pub teacher_call_allowed: bool,
    pub vram_required_mb: f32,
    pub full_scan_used: bool,
    pub core_efficiency_score: f32,
}
