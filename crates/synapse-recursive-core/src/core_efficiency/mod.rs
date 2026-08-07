pub mod active_field_budget;
pub mod cold_storage_policy;
pub mod core_efficiency_benchmark;
pub mod core_efficiency_report;
pub mod device_profile;
pub mod efficiency_budget;
pub mod memory_scale_audit;
pub mod module_residency;
pub mod residency_registry;
pub mod runtime_tier;
pub mod wake_sleep_controller;

pub use active_field_budget::{ActiveFieldBudget, ActiveFieldBudgetReport};
pub use cold_storage_policy::{ColdStoragePolicy, ColdStorageReport};
pub use core_efficiency_benchmark::CoreEfficiencyBenchmark;
pub use core_efficiency_report::CoreEfficiencyReport;
pub use device_profile::{DeviceProfile, DeviceProfileReport};
pub use efficiency_budget::EfficiencyBudget;
pub use memory_scale_audit::{MemoryScaleAudit, MemoryScaleAuditReport, MemoryScaleAuditRow};
pub use module_residency::ModuleResidency;
pub use residency_registry::{ResidencyRegistry, ResidencySummary};
pub use runtime_tier::RuntimeTier;
pub use wake_sleep_controller::{WakeSleepController, WakeSleepReport};

#[derive(Debug, Clone)]
pub struct CoreEfficiencyEngine {
    registry: ResidencyRegistry,
    budget: EfficiencyBudget,
}

impl Default for CoreEfficiencyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreEfficiencyEngine {
    pub fn new() -> Self {
        Self {
            registry: ResidencyRegistry::new(),
            budget: EfficiencyBudget::default(),
        }
    }

    pub fn registry(&self) -> &ResidencyRegistry {
        &self.registry
    }

    pub fn budget(&self) -> &EfficiencyBudget {
        &self.budget
    }

    pub fn status(&self) -> CoreEfficiencyReport {
        self.audit_for_device(DeviceProfile::RaspberryPi8Gb)
    }

    pub fn tiers(&self) -> Vec<(RuntimeTier, Vec<ModuleResidency>)> {
        [
            RuntimeTier::AlwaysOnCore,
            RuntimeTier::OnDemandCortex,
            RuntimeTier::ToolLayer,
            RuntimeTier::ApplicationLayer,
            RuntimeTier::FrozenScaffold,
        ]
        .into_iter()
        .map(|tier| {
            (
                tier,
                self.registry.tier(tier).into_iter().cloned().collect(),
            )
        })
        .collect()
    }

    pub fn residency(&self) -> ResidencySummary {
        self.registry.summary()
    }

    pub fn active_field_report(&self) -> ActiveFieldBudgetReport {
        ActiveFieldBudget::evaluate(32, 128, false, &self.budget)
    }

    pub fn memory_scale(&self) -> MemoryScaleAuditReport {
        MemoryScaleAudit::run(&self.budget)
    }

    pub fn wake_test(&self, module_name: &str) -> WakeSleepReport {
        WakeSleepController::wake(
            module_name,
            "explicit self-development goal with test failure",
            &self.registry,
        )
    }

    pub fn sleep_test(&self, module_name: &str) -> WakeSleepReport {
        WakeSleepController::sleep_after_task(module_name)
    }

    pub fn device(&self, profile: DeviceProfile) -> DeviceProfileReport {
        profile.report()
    }

    pub fn audit_for_device(&self, profile: DeviceProfile) -> CoreEfficiencyReport {
        let device_profile = profile.report();
        let residency = self.registry.summary();
        let active_field_budget =
            ActiveFieldBudget::evaluate(32, 128, false, &device_profile.budget);
        let memory_scale_audit = MemoryScaleAudit::run(&device_profile.budget);
        let wake_report = self.wake_test("code-growth");
        let sleep_report = self.sleep_test("code-growth");
        let cold_storage = ColdStoragePolicy::compress_development_artifacts();
        let core_efficiency_score = score_core_efficiency(
            &residency,
            &active_field_budget,
            &memory_scale_audit,
            &cold_storage,
            &device_profile,
        );

        CoreEfficiencyReport {
            budget: device_profile.budget.clone(),
            residency,
            active_field_budget,
            memory_scale_audit,
            device_profile,
            wake_report,
            sleep_report,
            cold_storage,
            teacher_call_allowed: self.budget.teacher_call_allowed,
            vram_required_mb: self.budget.vram_required_mb,
            full_scan_used: false,
            core_efficiency_score,
        }
    }

    pub fn benchmark() -> CoreEfficiencyBenchmark {
        let engine = Self::new();
        let budget = engine.budget();
        let summary = engine.residency();
        let audit = engine.memory_scale();
        let field = engine.active_field_report();
        let code_growth = engine.registry.find("Code Growth Loop");
        let coding_training = engine.registry.find("Coding Training Arena");
        let patch_sandbox = engine.registry.find("Patch Sandbox");
        let tool_layers = engine.registry.tier(RuntimeTier::ToolLayer);
        let app_layers = engine.registry.tier(RuntimeTier::ApplicationLayer);
        let wake_dev = WakeSleepController::wake(
            "code-growth",
            "explicit self-development goal",
            engine.registry(),
        );
        let wake_idle = WakeSleepController::wake("code-growth", "idle tick", engine.registry());
        let sleep = WakeSleepController::sleep_after_task("code-growth");
        let cold = ColdStoragePolicy::compress_development_artifacts();
        let raspberry = DeviceProfile::RaspberryPi8Gb.report();

        let off_resident_module_count = engine.registry.modules.len();
        let on_resident_module_count = summary.resident_module_count;
        let off_always_on_memory_mb = engine
            .registry
            .modules
            .iter()
            .map(|module| module.estimated_memory_cost_mb)
            .sum::<f32>();
        let on_always_on_memory_mb = summary.always_on_memory_mb;
        let off_active_memory_mb = off_always_on_memory_mb;
        let on_active_memory_mb = summary.active_memory_mb_if_code_growth_wakes;
        let off_peak_memory_mb = off_always_on_memory_mb + 512.0;
        let on_peak_memory_mb = summary.peak_memory_mb;
        let off_active_field_size = 2048;
        let on_active_field_size = field.active_field_size;
        let off_candidate_nodes = 8192;
        let on_candidate_nodes = field.candidate_nodes;
        let recursive_stack_total = engine.registry.recursive_development_stack().len() as f32;
        let recursive_stack_sleeping = engine
            .registry
            .recursive_development_stack()
            .into_iter()
            .filter(|module| !module.resident_by_default)
            .count() as f32;
        let recursive_stack_residency_reduction = if recursive_stack_total <= f32::EPSILON {
            0.0
        } else {
            recursive_stack_sleeping / recursive_stack_total
        };

        CoreEfficiencyBenchmark {
            efficiency_budget_targets_8gb_edge_device: budget.target_device == "8GB Edge Device"
                && budget.max_idle_ram_mb <= 256.0
                && budget.vram_required_mb == 0.0
                && !budget.teacher_call_allowed,
            runtime_tiers_classify_always_on_and_on_demand_modules: !engine
                .registry
                .tier(RuntimeTier::AlwaysOnCore)
                .is_empty()
                && !engine.registry.tier(RuntimeTier::OnDemandCortex).is_empty(),
            always_on_core_contains_only_minimal_kernel: summary.always_on_modules.len() <= 13
                && summary
                    .always_on_modules
                    .iter()
                    .all(|name| minimal_kernel_names().contains(&name.as_str())),
            recursive_development_stack_is_on_demand_not_resident: engine
                .registry
                .recursive_development_stack()
                .iter()
                .all(|module| {
                    module.tier == RuntimeTier::OnDemandCortex && !module.resident_by_default
                }),
            coding_training_arena_is_on_demand_not_resident: coding_training.is_some_and(
                |module| module.tier == RuntimeTier::OnDemandCortex && !module.resident_by_default,
            ),
            patch_sandbox_is_on_demand_not_resident: patch_sandbox.is_some_and(|module| {
                module.tier == RuntimeTier::OnDemandCortex && !module.resident_by_default
            }),
            tool_layers_are_not_resident_by_default: tool_layers
                .iter()
                .all(|module| !module.resident_by_default),
            application_layers_are_not_resident_by_default: app_layers
                .iter()
                .all(|module| !module.resident_by_default),
            active_field_budget_limits_active_field_size: field.active_field_within_budget,
            active_field_budget_limits_candidate_nodes: field.candidate_nodes_within_budget,
            memory_scale_audit_rejects_full_scan: !audit.full_scan_used
                && !audit.active_field_budget.full_scan_used,
            memory_scale_audit_keeps_latency_non_linear_to_total_nodes: !audit
                .latency_linear_to_total_nodes
                && audit.latency_growth_ratio <= 1.05,
            wake_controller_wakes_code_growth_only_on_development_goal: wake_dev.woke
                && !wake_idle.woke,
            sleep_controller_compresses_development_memory_after_task: sleep.slept
                && !sleep.retained_summaries.is_empty()
                && !sleep.dropped_resident_payloads.is_empty(),
            cold_storage_policy_drops_large_raw_logs_from_resident_memory: !cold.raw_logs_resident
                && cold
                    .moved_to_cold_storage
                    .iter()
                    .any(|item| item == "large raw logs"),
            device_profile_registers_raspberry_pi_8gb: raspberry.profile
                == DeviceProfile::RaspberryPi8Gb
                && raspberry.priority_rank == 1,
            core_efficiency_benchmark_reduces_resident_memory_without_breaking_growth:
                on_resident_module_count < off_resident_module_count
                    && on_always_on_memory_mb < off_always_on_memory_mb
                    && code_growth.is_some_and(|module| !module.resident_by_default)
                    && wake_dev.woke,
            off_always_on_memory_mb,
            on_always_on_memory_mb,
            off_active_memory_mb,
            on_active_memory_mb,
            off_peak_memory_mb,
            on_peak_memory_mb,
            off_resident_module_count,
            on_resident_module_count,
            off_active_field_size,
            on_active_field_size,
            off_candidate_nodes,
            on_candidate_nodes,
            full_scan_used: audit.full_scan_used,
            wake_cost_ms: wake_dev.estimated_wake_cost_ms,
            sleep_compression_score: if cold.resident_items_after < cold.resident_items_before {
                0.92
            } else {
                0.30
            },
            memory_scaling_score: audit.memory_scaling_score,
            latency_growth_ratio: audit.latency_growth_ratio,
            core_efficiency_score: engine.status().core_efficiency_score,
            recursive_stack_residency_reduction,
        }
    }
}

fn score_core_efficiency(
    residency: &ResidencySummary,
    active_field_budget: &ActiveFieldBudgetReport,
    memory_scale_audit: &MemoryScaleAuditReport,
    cold_storage: &ColdStorageReport,
    device_profile: &DeviceProfileReport,
) -> f32 {
    let memory_score = if residency.always_on_memory_mb <= device_profile.budget.max_idle_ram_mb {
        0.25
    } else {
        0.05
    };
    let active_score = if active_field_budget.accepted {
        0.25
    } else {
        0.05
    };
    let scale_score = memory_scale_audit.memory_scaling_score * 0.25;
    let cold_score = if !cold_storage.raw_logs_resident {
        0.25
    } else {
        0.05
    };
    (memory_score + active_score + scale_score + cold_score).clamp(0.0, 1.0)
}

fn minimal_kernel_names() -> Vec<&'static str> {
    vec![
        "Genome",
        "Core Purpose",
        "Core Needs",
        "Safety Boundary",
        "Active Field",
        "Concept Index",
        "Memory Cue Index",
        "Need / Gap Detector",
        "Prediction Error Detector",
        "Reward Signal",
        "Compression Trigger",
        "Minimal Self State",
        "Minimal Body State",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn efficiency_budget_targets_8gb_edge_device() {
        let budget = EfficiencyBudget::default();
        assert_eq!(budget.target_device, "8GB Edge Device");
        assert_eq!(budget.max_idle_ram_mb, 256.0);
        assert_eq!(budget.vram_required_mb, 0.0);
        assert!(!budget.teacher_call_allowed);
        assert!(!budget.full_scan_allowed);
    }

    #[test]
    fn runtime_tiers_classify_always_on_and_on_demand_modules() {
        let registry = ResidencyRegistry::new();
        assert!(!registry.tier(RuntimeTier::AlwaysOnCore).is_empty());
        assert!(!registry.tier(RuntimeTier::OnDemandCortex).is_empty());
    }

    #[test]
    fn always_on_core_contains_only_minimal_kernel() {
        let benchmark = CoreEfficiencyEngine::benchmark();
        assert!(benchmark.always_on_core_contains_only_minimal_kernel);
    }

    #[test]
    fn recursive_development_stack_is_on_demand_not_resident() {
        let benchmark = CoreEfficiencyEngine::benchmark();
        assert!(benchmark.recursive_development_stack_is_on_demand_not_resident);
    }

    #[test]
    fn coding_training_arena_is_on_demand_not_resident() {
        let benchmark = CoreEfficiencyEngine::benchmark();
        assert!(benchmark.coding_training_arena_is_on_demand_not_resident);
    }

    #[test]
    fn patch_sandbox_is_on_demand_not_resident() {
        let benchmark = CoreEfficiencyEngine::benchmark();
        assert!(benchmark.patch_sandbox_is_on_demand_not_resident);
    }

    #[test]
    fn tool_layers_are_not_resident_by_default() {
        let benchmark = CoreEfficiencyEngine::benchmark();
        assert!(benchmark.tool_layers_are_not_resident_by_default);
    }

    #[test]
    fn application_layers_are_not_resident_by_default() {
        let benchmark = CoreEfficiencyEngine::benchmark();
        assert!(benchmark.application_layers_are_not_resident_by_default);
    }

    #[test]
    fn active_field_budget_limits_active_field_size() {
        let benchmark = CoreEfficiencyEngine::benchmark();
        assert!(benchmark.active_field_budget_limits_active_field_size);
        assert!(benchmark.on_active_field_size <= 128);
    }

    #[test]
    fn active_field_budget_limits_candidate_nodes() {
        let benchmark = CoreEfficiencyEngine::benchmark();
        assert!(benchmark.active_field_budget_limits_candidate_nodes);
        assert!(benchmark.on_candidate_nodes <= 512);
    }

    #[test]
    fn memory_scale_audit_rejects_full_scan() {
        let benchmark = CoreEfficiencyEngine::benchmark();
        assert!(benchmark.memory_scale_audit_rejects_full_scan);
        assert!(!benchmark.full_scan_used);
    }

    #[test]
    fn memory_scale_audit_keeps_latency_non_linear_to_total_nodes() {
        let benchmark = CoreEfficiencyEngine::benchmark();
        assert!(benchmark.memory_scale_audit_keeps_latency_non_linear_to_total_nodes);
        assert!(benchmark.latency_growth_ratio <= 1.05);
    }

    #[test]
    fn wake_controller_wakes_code_growth_only_on_development_goal() {
        let benchmark = CoreEfficiencyEngine::benchmark();
        assert!(benchmark.wake_controller_wakes_code_growth_only_on_development_goal);
    }

    #[test]
    fn sleep_controller_compresses_development_memory_after_task() {
        let benchmark = CoreEfficiencyEngine::benchmark();
        assert!(benchmark.sleep_controller_compresses_development_memory_after_task);
    }

    #[test]
    fn cold_storage_policy_drops_large_raw_logs_from_resident_memory() {
        let benchmark = CoreEfficiencyEngine::benchmark();
        assert!(benchmark.cold_storage_policy_drops_large_raw_logs_from_resident_memory);
    }

    #[test]
    fn device_profile_registers_raspberry_pi_8gb() {
        let report = DeviceProfile::RaspberryPi8Gb.report();
        assert_eq!(report.priority_rank, 1);
        assert_eq!(report.profile, DeviceProfile::RaspberryPi8Gb);
        assert_eq!(report.vram_required_mb, 0.0);
    }

    #[test]
    fn core_efficiency_benchmark_reduces_resident_memory_without_breaking_growth() {
        let benchmark = CoreEfficiencyEngine::benchmark();
        assert!(
            benchmark.core_efficiency_benchmark_reduces_resident_memory_without_breaking_growth
        );
        assert!(benchmark.on_resident_module_count < benchmark.off_resident_module_count);
        assert!(benchmark.on_always_on_memory_mb < benchmark.off_always_on_memory_mb);
        assert!(benchmark.wake_cost_ms > 0.0);
    }
}
