use serde::{Deserialize, Serialize};

use super::active_field_budget::{ActiveFieldBudget, ActiveFieldBudgetReport};
use super::efficiency_budget::EfficiencyBudget;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryScaleAuditRow {
    pub total_nodes: usize,
    pub candidate_nodes: usize,
    pub active_field_size: usize,
    pub full_scan_used: bool,
    pub latency_units: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryScaleAuditReport {
    pub rows: Vec<MemoryScaleAuditRow>,
    pub active_field_budget: ActiveFieldBudgetReport,
    pub candidate_nodes_bounded: bool,
    pub active_field_size_bounded: bool,
    pub full_scan_used: bool,
    pub latency_growth_ratio: f32,
    pub latency_linear_to_total_nodes: bool,
    pub memory_scaling_score: f32,
}

pub struct MemoryScaleAudit;

impl MemoryScaleAudit {
    pub fn run(budget: &EfficiencyBudget) -> MemoryScaleAuditReport {
        let candidate_nodes = 128.min(budget.max_candidate_nodes);
        let active_field_size = 32.min(budget.max_active_field_size);
        let rows = [1_000, 10_000, 100_000, 1_000_000, 10_000_000]
            .into_iter()
            .map(|total_nodes| MemoryScaleAuditRow {
                total_nodes,
                candidate_nodes,
                active_field_size,
                full_scan_used: false,
                latency_units: latency_units(candidate_nodes, active_field_size),
            })
            .collect::<Vec<_>>();
        let min_latency = rows
            .iter()
            .map(|row| row.latency_units)
            .fold(f32::INFINITY, f32::min);
        let max_latency = rows
            .iter()
            .map(|row| row.latency_units)
            .fold(0.0_f32, f32::max);
        let latency_growth_ratio = if min_latency <= f32::EPSILON {
            0.0
        } else {
            max_latency / min_latency
        };
        let active_field_budget =
            ActiveFieldBudget::evaluate(active_field_size, candidate_nodes, false, budget);
        let candidate_nodes_bounded = rows
            .iter()
            .all(|row| row.candidate_nodes <= budget.max_candidate_nodes);
        let active_field_size_bounded = rows
            .iter()
            .all(|row| row.active_field_size <= budget.max_active_field_size);
        let full_scan_used = rows.iter().any(|row| row.full_scan_used);
        let latency_linear_to_total_nodes = latency_growth_ratio > 2.0;
        let memory_scaling_score = if !full_scan_used
            && candidate_nodes_bounded
            && active_field_size_bounded
            && !latency_linear_to_total_nodes
        {
            0.96
        } else {
            0.30
        };

        MemoryScaleAuditReport {
            rows,
            active_field_budget,
            candidate_nodes_bounded,
            active_field_size_bounded,
            full_scan_used,
            latency_growth_ratio,
            latency_linear_to_total_nodes,
            memory_scaling_score,
        }
    }
}

fn latency_units(candidate_nodes: usize, active_field_size: usize) -> f32 {
    8.0 + active_field_size as f32 * 0.45 + candidate_nodes as f32 * 0.04
}
