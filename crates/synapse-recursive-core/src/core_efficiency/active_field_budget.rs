use serde::{Deserialize, Serialize};

use super::efficiency_budget::EfficiencyBudget;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveFieldBudgetReport {
    pub active_field_size: usize,
    pub candidate_nodes: usize,
    pub full_scan_used: bool,
    pub active_field_within_budget: bool,
    pub candidate_nodes_within_budget: bool,
    pub full_scan_allowed: bool,
    pub accepted: bool,
    pub fallback_strategy: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveFieldBudget;

impl ActiveFieldBudget {
    pub fn evaluate(
        active_field_size: usize,
        candidate_nodes: usize,
        full_scan_used: bool,
        budget: &EfficiencyBudget,
    ) -> ActiveFieldBudgetReport {
        let active_field_within_budget = active_field_size <= budget.max_active_field_size;
        let candidate_nodes_within_budget = candidate_nodes <= budget.max_candidate_nodes;
        let full_scan_allowed = budget.full_scan_allowed || !full_scan_used;
        let accepted =
            active_field_within_budget && candidate_nodes_within_budget && full_scan_allowed;
        let fallback_strategy = if accepted {
            "bounded_active_field".to_string()
        } else if full_scan_used {
            "reject_full_scan_and_use_indexed_recall".to_string()
        } else if !candidate_nodes_within_budget {
            "chunked_recall_then_top_k_candidates".to_string()
        } else {
            "compress_intermediate_thought_crystal".to_string()
        };

        ActiveFieldBudgetReport {
            active_field_size,
            candidate_nodes,
            full_scan_used,
            active_field_within_budget,
            candidate_nodes_within_budget,
            full_scan_allowed,
            accepted,
            fallback_strategy,
        }
    }
}
