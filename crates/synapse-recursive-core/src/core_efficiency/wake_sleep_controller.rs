use serde::{Deserialize, Serialize};

use super::residency_registry::ResidencyRegistry;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WakeSleepReport {
    pub module_name: String,
    pub requested_event: String,
    pub woke: bool,
    pub slept: bool,
    pub estimated_wake_cost_ms: f32,
    pub resident_after: bool,
    pub retained_summaries: Vec<String>,
    pub dropped_resident_payloads: Vec<String>,
    pub reason: String,
}

pub struct WakeSleepController;

impl WakeSleepController {
    pub fn wake(module_name: &str, event: &str, registry: &ResidencyRegistry) -> WakeSleepReport {
        let module = registry.find(module_name);
        let event_lower = event.to_lowercase();
        let development_event = [
            "self-development goal",
            "development goal",
            "test failure",
            "benchmark regression",
            "capability gap",
            "user-approved development session",
        ]
        .iter()
        .any(|needle| event_lower.contains(needle));
        let woke = module.is_some()
            && if normalize(module_name).contains("code-growth") {
                development_event
            } else {
                !event_lower.contains("idle")
            };
        WakeSleepReport {
            module_name: module_name.to_string(),
            requested_event: event.to_string(),
            woke,
            slept: false,
            estimated_wake_cost_ms: module
                .map(|module| {
                    if woke {
                        module.estimated_wake_cost_ms
                    } else {
                        0.0
                    }
                })
                .unwrap_or(0.0),
            resident_after: module
                .map(|module| module.resident_by_default || woke)
                .unwrap_or(false),
            retained_summaries: Vec::new(),
            dropped_resident_payloads: Vec::new(),
            reason: if woke {
                "wake_condition_satisfied".to_string()
            } else {
                "wake_condition_not_satisfied".to_string()
            },
        }
    }

    pub fn sleep_after_task(module_name: &str) -> WakeSleepReport {
        WakeSleepReport {
            module_name: module_name.to_string(),
            requested_event: "task_complete".to_string(),
            woke: false,
            slept: true,
            estimated_wake_cost_ms: 0.0,
            resident_after: false,
            retained_summaries: vec![
                "CodingLesson summary".to_string(),
                "DevelopmentMemory summary".to_string(),
                "Patch lineage summary".to_string(),
                "Risk memory".to_string(),
                "Compressed growth principle".to_string(),
            ],
            dropped_resident_payloads: vec![
                "full patch logs".to_string(),
                "large diff history".to_string(),
                "full test output".to_string(),
                "temporary sandbox files".to_string(),
                "large training task state".to_string(),
            ],
            reason: "compressed_summary_retained_and_runtime_slept".to_string(),
        }
    }
}

fn normalize(input: &str) -> String {
    input.to_lowercase().replace([' ', '_'], "-")
}
