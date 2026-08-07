use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

use super::efficiency_budget::EfficiencyBudget;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceProfile {
    RaspberryPi8Gb,
    Android8Gb,
    WindowsPcDevelopment,
}

impl Display for DeviceProfile {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::RaspberryPi8Gb => "raspberry_pi_8gb",
            Self::Android8Gb => "android_8gb",
            Self::WindowsPcDevelopment => "windows_pc_development",
        };
        write!(formatter, "{value}")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceProfileReport {
    pub profile: DeviceProfile,
    pub priority_rank: u8,
    pub budget: EfficiencyBudget,
    pub teacher_enabled: bool,
    pub vram_required_mb: f32,
    pub note: String,
}

impl DeviceProfile {
    pub fn parse(input: &str) -> Self {
        match input {
            "android-8gb" | "android_8gb" => Self::Android8Gb,
            "windows" | "windows-pc-development" | "windows_pc_development" => {
                Self::WindowsPcDevelopment
            }
            _ => Self::RaspberryPi8Gb,
        }
    }

    pub fn report(self) -> DeviceProfileReport {
        let mut budget = EfficiencyBudget::default();
        let (priority_rank, note) = match self {
            Self::RaspberryPi8Gb => {
                budget.target_device = "RaspberryPi8Gb".to_string();
                budget.max_idle_ram_mb = 224.0;
                budget.max_active_ram_mb = 896.0;
                (1, "primary low-power 8GB edge target")
            }
            Self::Android8Gb => {
                budget.target_device = "Android8Gb".to_string();
                budget.max_idle_ram_mb = 256.0;
                budget.max_active_ram_mb = 1024.0;
                (2, "mobile teacherless low-power target")
            }
            Self::WindowsPcDevelopment => {
                budget.target_device = "WindowsPcDevelopment".to_string();
                budget.max_idle_ram_mb = 512.0;
                budget.max_active_ram_mb = 2048.0;
                budget.max_peak_ram_mb = 4096.0;
                (3, "development host, not target deployment hardware")
            }
        };
        DeviceProfileReport {
            profile: self,
            priority_rank,
            teacher_enabled: budget.teacher_call_allowed,
            vram_required_mb: budget.vram_required_mb,
            budget,
            note: note.to_string(),
        }
    }
}
