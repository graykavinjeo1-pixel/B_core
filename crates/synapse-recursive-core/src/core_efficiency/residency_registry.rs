use serde::{Deserialize, Serialize};

use super::module_residency::ModuleResidency;
use super::runtime_tier::RuntimeTier;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResidencySummary {
    pub total_modules: usize,
    pub resident_module_count: usize,
    pub on_demand_module_count: usize,
    pub always_on_memory_mb: f32,
    pub active_memory_mb_if_code_growth_wakes: f32,
    pub peak_memory_mb: f32,
    pub always_on_modules: Vec<String>,
    pub sleeping_recursive_stack: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResidencyRegistry {
    pub modules: Vec<ModuleResidency>,
}

impl Default for ResidencyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ResidencyRegistry {
    pub fn new() -> Self {
        let mut modules = Vec::new();
        for (name, memory, field) in [
            ("Genome", 18.0, 4),
            ("Core Purpose", 4.0, 1),
            ("Core Needs", 5.0, 2),
            ("Safety Boundary", 12.0, 4),
            ("Active Field", 16.0, 16),
            ("Concept Index", 24.0, 24),
            ("Memory Cue Index", 22.0, 20),
            ("Need / Gap Detector", 8.0, 4),
            ("Prediction Error Detector", 8.0, 4),
            ("Reward Signal", 6.0, 2),
            ("Compression Trigger", 7.0, 2),
            ("Minimal Self State", 8.0, 2),
            ("Minimal Body State", 5.0, 1),
        ] {
            modules.push(ModuleResidency::new(
                name,
                RuntimeTier::AlwaysOnCore,
                memory,
                0.0,
                field,
                None,
                None,
            ));
        }

        for (name, memory, wake, field) in [
            ("Dream Engine", 96.0, 28.0, 32),
            ("Creativity Engine", 88.0, 24.0, 28),
            ("Social Scaffold", 64.0, 14.0, 16),
            ("Profession Scaffold", 72.0, 18.0, 20),
            ("Project Execution", 86.0, 20.0, 24),
            ("Embryo Growth Expansion", 64.0, 14.0, 16),
            ("Code Growth Loop", 118.0, 32.0, 32),
            ("Patch Feedback Loop", 90.0, 24.0, 24),
            ("Coding Training Arena", 128.0, 36.0, 36),
            ("Self-Development Runtime", 72.0, 18.0, 18),
            ("Patch Sandbox", 160.0, 42.0, 40),
        ] {
            modules.push(ModuleResidency::new(
                name,
                RuntimeTier::OnDemandCortex,
                memory,
                wake,
                field,
                Some("explicit task, regression, or approved session"),
                Some("task complete; retain compressed summaries only"),
            ));
        }

        for (name, memory, wake) in [
            ("Blender", 512.0, 450.0),
            ("Voice Synthesis", 384.0, 380.0),
            ("OBS / Broadcast", 256.0, 260.0),
            ("File Tool", 48.0, 20.0),
            ("External App Tool", 128.0, 90.0),
            ("Windows Controlled Action", 96.0, 70.0),
        ] {
            modules.push(ModuleResidency::new(
                name,
                RuntimeTier::ToolLayer,
                memory,
                wake,
                0,
                Some("explicit tool-capability request with permission"),
                Some("tool task complete or permission revoked"),
            ));
        }

        for (name, memory, wake) in [
            ("Avatar", 192.0, 180.0),
            ("Voice Presence", 96.0, 90.0),
            ("Broadcast Presence", 160.0, 160.0),
            ("Robot Body", 220.0, 220.0),
            ("Content Creator Mode", 256.0, 240.0),
        ] {
            modules.push(ModuleResidency::new(
                name,
                RuntimeTier::ApplicationLayer,
                memory,
                wake,
                0,
                Some("explicit product/presence mode"),
                Some("application session complete"),
            ));
        }

        for name in [
            "Coding Training Problem Hint",
            "Historical PatchProposal Full Text",
            "Historical Test Log Full Text",
            "Old Sandbox Record",
            "Long Benchmark Raw Log",
        ] {
            modules.push(ModuleResidency::new(
                name,
                RuntimeTier::FrozenScaffold,
                0.0,
                8.0,
                0,
                Some("reference lookup only"),
                Some("drop raw data; keep summary reference"),
            ));
        }

        Self { modules }
    }

    pub fn tier(&self, tier: RuntimeTier) -> Vec<&ModuleResidency> {
        self.modules
            .iter()
            .filter(|module| module.tier == tier)
            .collect()
    }

    pub fn find(&self, module_name: &str) -> Option<&ModuleResidency> {
        let needle = normalize(module_name);
        self.modules.iter().find(|module| {
            let normalized = normalize(&module.module_name);
            normalized == needle || module_alias_matches(&needle, &normalized)
        })
    }

    pub fn recursive_development_stack(&self) -> Vec<&ModuleResidency> {
        self.modules
            .iter()
            .filter(|module| {
                matches!(
                    module.module_name.as_str(),
                    "Code Growth Loop"
                        | "Patch Feedback Loop"
                        | "Coding Training Arena"
                        | "Self-Development Runtime"
                        | "Patch Sandbox"
                )
            })
            .collect()
    }

    pub fn summary(&self) -> ResidencySummary {
        let resident_module_count = self
            .modules
            .iter()
            .filter(|module| module.resident_by_default)
            .count();
        let always_on_memory_mb = self
            .modules
            .iter()
            .filter(|module| module.resident_by_default)
            .map(|module| module.estimated_memory_cost_mb)
            .sum::<f32>();
        let code_growth_wake_mb = self
            .find("Code Growth Loop")
            .map(|module| module.estimated_memory_cost_mb)
            .unwrap_or(0.0);
        let patch_sandbox_wake_mb = self
            .find("Patch Sandbox")
            .map(|module| module.estimated_memory_cost_mb)
            .unwrap_or(0.0);
        let always_on_modules = self
            .tier(RuntimeTier::AlwaysOnCore)
            .iter()
            .map(|module| module.module_name.clone())
            .collect();
        let sleeping_recursive_stack = self
            .recursive_development_stack()
            .iter()
            .filter(|module| !module.resident_by_default)
            .map(|module| module.module_name.clone())
            .collect();

        ResidencySummary {
            total_modules: self.modules.len(),
            resident_module_count,
            on_demand_module_count: self.modules.len().saturating_sub(resident_module_count),
            always_on_memory_mb,
            active_memory_mb_if_code_growth_wakes: always_on_memory_mb + code_growth_wake_mb,
            peak_memory_mb: always_on_memory_mb + code_growth_wake_mb + patch_sandbox_wake_mb,
            always_on_modules,
            sleeping_recursive_stack,
        }
    }
}

fn normalize(input: &str) -> String {
    input
        .to_lowercase()
        .replace([' ', '_'], "-")
        .replace("--", "-")
}

fn module_alias_matches(needle: &str, normalized: &str) -> bool {
    matches!(
        (needle, normalized),
        ("code-growth", "code-growth-loop")
            | ("patch-feedback", "patch-feedback-loop")
            | ("coding-training", "coding-training-arena")
            | ("self-dev", "self-development-runtime")
            | ("patch-sandbox", "patch-sandbox")
    )
}
