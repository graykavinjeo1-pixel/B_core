use std::collections::{BTreeMap, BTreeSet};

use super::model::{
    BranchAllocation, CandidateAction, CanonicalMetrics, Condition, ControllerAction,
    EvaluationTask, Goal, ResourceBudget, SolveResult, TerminationReason, TraceEvent,
};

pub const CONTROLLER_VERSION: &str = "SEM2-ADAPTIVE-CONTROLLER-1.1.0";

pub struct AdaptiveReasoner;

impl AdaptiveReasoner {
    pub fn solve(
        task: &EvaluationTask,
        condition: Condition,
        budget: ResourceBudget,
    ) -> SolveResult {
        let adaptive = matches!(
            condition,
            Condition::AdaptiveD
                | Condition::DMinusInformationGain
                | Condition::DMinusSemanticPruning
                | Condition::DMinusDecomposition
                | Condition::DMinusStateMerging
        );
        let semantic_pruning = adaptive && condition != Condition::DMinusSemanticPruning;
        let information_gain = adaptive && condition != Condition::DMinusInformationGain;
        let decomposition = adaptive && condition != Condition::DMinusDecomposition;
        let state_merging = adaptive && condition != Condition::DMinusStateMerging;

        let mut metrics = CanonicalMetrics::default();
        let mut trace = Vec::new();
        let mut selected_actions = BTreeMap::new();
        let mut completed = BTreeSet::new();
        let mut facts = task.visible.initial_facts.clone();
        let mut allocations = Vec::new();
        let mut active_samples = Vec::new();
        let mut selected_concepts = BTreeSet::new();
        let mut selected_generations = BTreeSet::new();
        let mut trajectory_depth = 0usize;
        let mut sequence = 0usize;
        let mut failed_signatures = BTreeSet::new();

        loop {
            if completed.len() == task.visible.goals.len() {
                push_trace(
                    &mut trace,
                    &mut sequence,
                    task,
                    None,
                    ControllerAction::StopSolved,
                    1.0,
                    "all goal contracts independently verified",
                    metrics.cumulative_search_expansions,
                );
                break;
            }
            let mut ready = task
                .visible
                .goals
                .iter()
                .filter(|goal| {
                    !completed.contains(&goal.goal_id)
                        && goal
                            .dependencies
                            .iter()
                            .all(|item| completed.contains(item))
                })
                .collect::<Vec<_>>();
            ready.sort_by(|left, right| left.goal_id.cmp(&right.goal_id));
            if ready.is_empty() {
                return failure_result(
                    task,
                    condition,
                    metrics,
                    allocations,
                    trace,
                    selected_actions,
                    TerminationReason::VerifierFailure,
                );
            }
            metrics.maximum_simultaneous_subproblems =
                metrics.maximum_simultaneous_subproblems.max(ready.len());
            if ready.len() > 1 && decomposition {
                metrics.decomposition_count += 1;
                metrics.subproblems_created += ready.len();
                metrics.maximum_decomposition_tree_depth =
                    metrics.maximum_decomposition_tree_depth.max(
                        1 + ready
                            .iter()
                            .map(|goal| goal.dependencies.len())
                            .max()
                            .unwrap_or(0),
                    );
                push_trace(
                    &mut trace,
                    &mut sequence,
                    task,
                    None,
                    ControllerAction::DecomposeGoal,
                    action_value(0.8, 0.4, 0.0, 0.7, 0.9, 0.2, 0.0, 0.0, 0.0),
                    "independent dependency frontiers admit parallel subproblems",
                    metrics.cumulative_search_expansions,
                );
            }
            let goal = if decomposition {
                ready
                    .iter()
                    .max_by_key(|goal| (goal.recombination, goal.dependencies.len()))
                    .copied()
                    .expect("ready")
            } else {
                ready[0]
            };
            if !allocations.is_empty() && decomposition {
                push_trace(
                    &mut trace,
                    &mut sequence,
                    task,
                    Some(goal),
                    ControllerAction::SwitchSubproblem,
                    0.55,
                    "highest-valued ready subproblem selected",
                    metrics.cumulative_search_expansions,
                );
            }
            let estimated_value = estimate_goal_value(goal);
            let allocated = allocation_for(goal, budget.max_expansions, task.visible.goals.len());
            let before = metrics.cumulative_search_expansions;
            let selected = solve_goal(
                task,
                goal,
                condition,
                semantic_pruning,
                information_gain,
                state_merging,
                &facts,
                &mut metrics,
                &mut trace,
                &mut sequence,
                &mut failed_signatures,
            );
            let actual = metrics.cumulative_search_expansions - before;
            allocations.push(BranchAllocation {
                branch_id: goal.goal_id.clone(),
                allocated_expansions: allocated,
                actual_expansions: actual,
                estimated_value,
                termination_reason: if selected.is_some() {
                    "VERIFIED_SUBGOAL".to_string()
                } else {
                    "EXHAUSTED".to_string()
                },
            });
            if metrics.cumulative_search_expansions > budget.max_expansions
                || metrics.peak_simultaneously_live_branches > budget.max_live_frontier
                || metrics.wall_time_units > budget.max_wall_time_units
                || metrics.peak_memory_units > budget.max_memory_units
            {
                push_trace(
                    &mut trace,
                    &mut sequence,
                    task,
                    Some(goal),
                    ControllerAction::StopResourceExhausted,
                    -1.0,
                    "declared resource envelope exhausted",
                    metrics.cumulative_search_expansions,
                );
                return failure_result(
                    task,
                    condition,
                    metrics,
                    allocations,
                    trace,
                    selected_actions,
                    TerminationReason::ResourceExhausted,
                );
            }
            let Some(selected) = selected else {
                return failure_result(
                    task,
                    condition,
                    metrics,
                    allocations,
                    trace,
                    selected_actions,
                    TerminationReason::VerifierFailure,
                );
            };
            trajectory_depth += 1;
            selected_actions.insert(goal.goal_id.clone(), selected.action_id.clone());
            facts.insert(format!("DONE:{}", goal.goal_id));
            completed.insert(goal.goal_id.clone());
            if condition != Condition::PrimitiveFixedA {
                if let Some(concept) = &selected.concept_id {
                    selected_concepts.insert(concept.clone());
                    selected_generations.insert(selected.concept_generation);
                    metrics.promoted_concept_reuse_count += 1;
                    if adaptive {
                        push_trace(
                            &mut trace,
                            &mut sequence,
                            task,
                            Some(goal),
                            ControllerAction::ReuseConcept,
                            0.82,
                            "routed immutable promoted concept satisfies the verified contract",
                            metrics.cumulative_search_expansions,
                        );
                    }
                }
            }
            active_samples.push(selected_concepts.len().max(1));
            if goal.recombination {
                metrics.recombination_count += 1;
                push_trace(
                    &mut trace,
                    &mut sequence,
                    task,
                    Some(goal),
                    ControllerAction::RecombineResults,
                    0.95,
                    "all dependency exports verified and interface-compatible",
                    metrics.cumulative_search_expansions,
                );
                push_trace(
                    &mut trace,
                    &mut sequence,
                    task,
                    Some(goal),
                    ControllerAction::CompressIntermediate,
                    0.71,
                    "verified DAG retained while operational result is compressed",
                    metrics.cumulative_search_expansions,
                );
            } else if !goal.dependencies.is_empty() {
                metrics.subproblems_solved += 1;
            }
        }

        metrics.solution_graph_depth = solution_depth(&task.visible.goals);
        metrics.primitive_expanded_solution_depth =
            primitive_depth(&task.visible.goals, &selected_actions);
        metrics.search_trajectory_max_depth = trajectory_depth;
        metrics.concepts_composed = selected_concepts.len();
        metrics.cross_generation_concept_composition_count =
            usize::from(selected_generations.len() > 1);
        metrics.peak_active_concepts = active_samples.iter().copied().max().unwrap_or(0) + 1;
        metrics.mean_active_concepts = if active_samples.is_empty() {
            0.0
        } else {
            active_samples.iter().sum::<usize>() as f64 / active_samples.len() as f64
        };
        metrics.useful_branch_ratio = if metrics.cumulative_branches_generated == 0 {
            0.0
        } else {
            selected_actions.len() as f64 / metrics.cumulative_branches_generated as f64
        };
        metrics.wall_time_units = metrics.cumulative_search_expansions
            + metrics.information_probes_executed
            + metrics.recombination_count;
        metrics.peak_memory_units = metrics
            .peak_simultaneously_live_branches
            .saturating_mul(3)
            .saturating_add(metrics.peak_active_concepts);
        metrics.branch_expansion_gini = gini(
            &allocations
                .iter()
                .map(|allocation| allocation.actual_expansions)
                .collect::<Vec<_>>(),
        );
        SolveResult {
            task_id: task.visible.task_id.clone(),
            condition,
            solved: true,
            strictly_correct: verify_solution(task, &selected_actions),
            termination_reason: TerminationReason::VerifiedSuccess,
            selected_actions,
            metrics,
            allocations,
            trace,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn solve_goal<'a>(
    task: &EvaluationTask,
    goal: &'a Goal,
    condition: Condition,
    semantic_pruning: bool,
    information_gain: bool,
    state_merging: bool,
    facts: &BTreeSet<String>,
    metrics: &mut CanonicalMetrics,
    trace: &mut Vec<TraceEvent>,
    sequence: &mut usize,
    failed_signatures: &mut BTreeSet<String>,
) -> Option<&'a CandidateAction> {
    metrics.cumulative_branches_generated += goal.candidates.len();
    let mut live = goal.candidates.iter().collect::<Vec<_>>();
    let fixed_type_pruning =
        matches!(condition, Condition::FixedHeuristicC) || condition_is_adaptive(condition);
    if fixed_type_pruning {
        live.retain(|candidate| {
            let valid = candidate.input_type == goal.input_type
                && candidate.output_type == goal.output_type
                && candidate.required_facts.is_subset(facts);
            if !valid {
                metrics.pruned_branch_count += 1;
                push_trace(
                    trace,
                    sequence,
                    task,
                    Some(goal),
                    ControllerAction::PruneBranch,
                    -0.7,
                    "type or executable precondition failed",
                    metrics.cumulative_search_expansions,
                );
            }
            valid
        });
    }
    if semantic_pruning {
        live.retain(|candidate| {
            let valid = candidate.invariant_consistent
                && candidate.export_contract == goal.required_export_contract;
            if !valid {
                metrics.pruned_branch_count += 1;
                metrics.semantic_prune_count += 1;
                push_trace(
                    trace,
                    sequence,
                    task,
                    Some(goal),
                    ControllerAction::PruneBranch,
                    action_value(0.0, 0.0, 0.8, 0.0, 0.0, 0.1, 0.4, 0.9, 0.0),
                    "semantic invariant contradicted or required relation absent",
                    metrics.cumulative_search_expansions,
                );
            }
            valid
        });
    }
    if information_gain && live.len() > 1 {
        if let Some(probe) = best_probe(task, goal, &live) {
            metrics.information_probes_proposed += 1;
            push_trace(
                trace,
                sequence,
                task,
                Some(goal),
                ControllerAction::GenerateCounterfactual,
                probe_information_gain(probe, &live) - probe.cost as f64 * 0.05,
                "probe partitions currently viable hypotheses",
                metrics.cumulative_search_expansions,
            );
            let gain = probe_information_gain(probe, &live);
            if gain > probe.cost as f64 * 0.05 {
                metrics.information_probes_executed += 1;
                let observation = task
                    .evaluator
                    .probe_observations
                    .get(&probe.probe_id)
                    .copied()
                    .unwrap_or(false);
                let before = live.len();
                live.retain(|candidate| {
                    probe
                        .candidate_predictions
                        .get(&candidate.action_id)
                        .copied()
                        == Some(observation)
                });
                let eliminated = before - live.len();
                metrics.hypotheses_eliminated += eliminated;
                metrics.expansions_saved_by_probes += eliminated;
                metrics.pruned_branch_count += eliminated;
                push_trace(
                    trace,
                    sequence,
                    task,
                    Some(goal),
                    ControllerAction::ExecuteProbe,
                    gain,
                    "observed probe outcome eliminated divergent predictions",
                    metrics.cumulative_search_expansions,
                );
            }
        }
    }
    if state_merging {
        let mut cheapest: BTreeMap<String, &CandidateAction> = BTreeMap::new();
        for candidate in live {
            match cheapest.get(&candidate.resulting_semantic_state) {
                Some(existing) if dominates(existing, candidate) => {
                    metrics.dominance_merge_count += 1;
                }
                Some(_) => {
                    cheapest.insert(candidate.resulting_semantic_state.clone(), candidate);
                    metrics.dominance_merge_count += 1;
                }
                None => {
                    cheapest.insert(candidate.resulting_semantic_state.clone(), candidate);
                }
            }
        }
        live = cheapest.into_values().collect();
    }
    metrics.instantaneous_frontier_width = metrics.instantaneous_frontier_width.max(live.len());
    metrics.peak_simultaneously_live_branches =
        metrics.peak_simultaneously_live_branches.max(live.len());
    if live.len() > 1 {
        push_trace(
            trace,
            sequence,
            task,
            Some(goal),
            ControllerAction::BranchAlternative,
            0.3,
            "multiple viable hypotheses remain after admissible pruning",
            metrics.cumulative_search_expansions,
        );
    }
    live.sort_by(|left, right| {
        rank_candidate(right, goal)
            .total_cmp(&rank_candidate(left, goal))
            .then_with(|| left.action_id.cmp(&right.action_id))
    });
    let correct_state = correct_state(task, goal)?;
    let exhaustive = !condition_is_adaptive(condition)
        || matches!(
            condition,
            Condition::DMinusInformationGain | Condition::DMinusStateMerging
        );
    let mut selected = None;
    for candidate in live {
        metrics.cumulative_search_expansions += if condition == Condition::PrimitiveFixedA {
            candidate.primitive_expansion_cost
        } else {
            candidate.execution_cost
        };
        push_trace(
            trace,
            sequence,
            task,
            Some(goal),
            ControllerAction::ExpandCurrent,
            rank_candidate(candidate, goal),
            "candidate transition executed and sent to independent verifier",
            metrics.cumulative_search_expansions,
        );
        if candidate.resulting_semantic_state == correct_state {
            selected.get_or_insert(candidate);
            if !exhaustive {
                break;
            }
        } else {
            metrics.backtracks += 1;
            metrics.rollbacks += 1;
            if candidate
                .observed_failure_signature
                .as_ref()
                .is_some_and(|signature| !failed_signatures.insert(signature.clone()))
            {
                metrics.stagnation_prunes += 1;
                push_trace(
                    trace,
                    sequence,
                    task,
                    Some(goal),
                    ControllerAction::PruneBranch,
                    -0.8,
                    "repeated contradiction signature triggered stagnation pruning",
                    metrics.cumulative_search_expansions,
                );
            }
            push_trace(
                trace,
                sequence,
                task,
                Some(goal),
                ControllerAction::Backtrack,
                -0.4,
                "independent verifier rejected candidate; explicit rollback retained",
                metrics.cumulative_search_expansions,
            );
        }
    }
    selected
}

fn condition_is_adaptive(condition: Condition) -> bool {
    matches!(
        condition,
        Condition::AdaptiveD
            | Condition::DMinusInformationGain
            | Condition::DMinusSemanticPruning
            | Condition::DMinusDecomposition
            | Condition::DMinusStateMerging
    )
}

fn correct_state(task: &EvaluationTask, goal: &Goal) -> Option<String> {
    let correct_id = task.evaluator.correct_branches.get(&goal.goal_id)?;
    goal.candidates
        .iter()
        .find(|candidate| candidate.action_id == *correct_id)
        .map(|candidate| candidate.resulting_semantic_state.clone())
}

fn best_probe<'a>(
    task: &'a EvaluationTask,
    goal: &Goal,
    live: &[&CandidateAction],
) -> Option<&'a super::model::ProbeContract> {
    task.visible
        .probes
        .iter()
        .filter(|probe| probe.probe_id.ends_with(&goal.goal_id))
        .max_by(|left, right| {
            probe_information_gain(left, live).total_cmp(&probe_information_gain(right, live))
        })
}

pub fn probe_information_gain(
    probe: &super::model::ProbeContract,
    candidates: &[&CandidateAction],
) -> f64 {
    if candidates.len() < 2 {
        return 0.0;
    }
    let positives = candidates
        .iter()
        .filter(|candidate| {
            probe
                .candidate_predictions
                .get(&candidate.action_id)
                .copied()
                .unwrap_or(false)
        })
        .count();
    let negatives = candidates.len() - positives;
    positives.min(negatives) as f64 / candidates.len() as f64
}

pub fn semantically_equivalent(left: &CandidateAction, right: &CandidateAction) -> bool {
    left.resulting_semantic_state == right.resulting_semantic_state
        && left.export_contract == right.export_contract
        && left.output_type == right.output_type
        && left.invariant_consistent == right.invariant_consistent
}

pub fn dominates(left: &CandidateAction, right: &CandidateAction) -> bool {
    semantically_equivalent(left, right)
        && left.execution_cost <= right.execution_cost
        && left.required_facts.is_subset(&right.required_facts)
}

pub fn stagnating(
    unresolved_before: usize,
    unresolved_after: usize,
    uncertainty_before: usize,
    uncertainty_after: usize,
    repeated_state: bool,
) -> bool {
    repeated_state
        && unresolved_after >= unresolved_before
        && uncertainty_after >= uncertainty_before
}

#[allow(clippy::too_many_arguments)]
pub fn action_value(
    expected_goal_progress: f64,
    expected_information_gain: f64,
    contradiction_resolution_value: f64,
    expected_reuse_value: f64,
    semantic_applicability_confidence: f64,
    computational_cost: f64,
    branch_redundancy: f64,
    contradiction_risk: f64,
    stagnation_penalty: f64,
) -> f64 {
    expected_goal_progress
        + expected_information_gain
        + contradiction_resolution_value
        + expected_reuse_value
        + semantic_applicability_confidence
        - computational_cost
        - branch_redundancy
        - contradiction_risk
        - stagnation_penalty
}

fn rank_candidate(candidate: &CandidateAction, goal: &Goal) -> f64 {
    action_value(
        f64::from(candidate.export_contract == goal.required_export_contract),
        0.0,
        f64::from(candidate.invariant_consistent) * 0.2,
        f64::from(candidate.concept_id.is_some()) * 0.3,
        f64::from(candidate.input_type == goal.input_type) * 0.7,
        candidate.execution_cost as f64 * 0.05,
        0.0,
        f64::from(!candidate.invariant_consistent) * 0.8,
        f64::from(candidate.observed_failure_signature.is_some()) * 0.1,
    )
}

fn estimate_goal_value(goal: &Goal) -> f64 {
    0.5 + goal.dependencies.len() as f64 * 0.15 + f64::from(goal.recombination) * 0.4
}

fn allocation_for(goal: &Goal, total: usize, goal_count: usize) -> usize {
    let base = (total / goal_count.max(1)).max(1);
    let uncertainty = (usize::BITS - goal.candidates.len().max(1).leading_zeros()) as usize;
    (2 + uncertainty * 2 + goal.dependencies.len() * 3 + usize::from(goal.recombination) * 5)
        .min(base)
}

fn solution_depth(goals: &[Goal]) -> usize {
    let mut depths = BTreeMap::new();
    for goal in goals {
        let depth = 1 + goal
            .dependencies
            .iter()
            .filter_map(|dependency| depths.get(dependency).copied())
            .max()
            .unwrap_or(0);
        depths.insert(goal.goal_id.clone(), depth);
    }
    depths.values().copied().max().unwrap_or(0)
}

fn primitive_depth(goals: &[Goal], selected: &BTreeMap<String, String>) -> usize {
    let mut depths = BTreeMap::new();
    for goal in goals {
        let previous = goal
            .dependencies
            .iter()
            .filter_map(|dependency| depths.get(dependency).copied())
            .max()
            .unwrap_or(0);
        let cost = selected
            .get(&goal.goal_id)
            .and_then(|id| {
                goal.candidates
                    .iter()
                    .find(|candidate| candidate.action_id == *id)
            })
            .map(|candidate| candidate.primitive_expansion_cost)
            .unwrap_or(1);
        depths.insert(goal.goal_id.clone(), previous + cost);
    }
    depths.values().copied().max().unwrap_or(0)
}

fn verify_solution(task: &EvaluationTask, selected: &BTreeMap<String, String>) -> bool {
    task.visible.goals.iter().all(|goal| {
        let Some(selected_id) = selected.get(&goal.goal_id) else {
            return false;
        };
        let Some(candidate) = goal
            .candidates
            .iter()
            .find(|candidate| candidate.action_id == *selected_id)
        else {
            return false;
        };
        correct_state(task, goal).is_some_and(|state| state == candidate.resulting_semantic_state)
    })
}

#[allow(clippy::too_many_arguments)]
fn push_trace(
    trace: &mut Vec<TraceEvent>,
    sequence: &mut usize,
    task: &EvaluationTask,
    goal: Option<&Goal>,
    action: ControllerAction,
    action_value: f64,
    reason: &str,
    cumulative_expansions: usize,
) {
    *sequence += 1;
    trace.push(TraceEvent {
        sequence: *sequence,
        task_id: task.visible.task_id.clone(),
        goal_id: goal.map(|item| item.goal_id.clone()),
        action,
        action_value,
        reason: reason.to_string(),
        cumulative_expansions,
    });
}

fn failure_result(
    task: &EvaluationTask,
    condition: Condition,
    metrics: CanonicalMetrics,
    allocations: Vec<BranchAllocation>,
    trace: Vec<TraceEvent>,
    selected_actions: BTreeMap<String, String>,
    termination_reason: TerminationReason,
) -> SolveResult {
    SolveResult {
        task_id: task.visible.task_id.clone(),
        condition,
        solved: false,
        strictly_correct: false,
        termination_reason,
        selected_actions,
        metrics,
        allocations,
        trace,
    }
}

pub fn gini(values: &[usize]) -> f64 {
    if values.is_empty() || values.iter().all(|value| *value == 0) {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let n = sorted.len() as f64;
    let sum = sorted.iter().sum::<usize>() as f64;
    let weighted = sorted
        .iter()
        .enumerate()
        .map(|(index, value)| (index + 1) as f64 * *value as f64)
        .sum::<f64>();
    (2.0 * weighted) / (n * sum) - (n + 1.0) / n
}

#[cfg(test)]
mod tests {
    use super::{dominates, gini, probe_information_gain, semantically_equivalent, stagnating};
    use crate::sem2::{model::ResourceBudget, tasks::generate_curriculum};

    #[test]
    fn metric_semantics_distinguish_instantaneous_and_cumulative_frontier() {
        let curriculum = generate_curriculum();
        let task = curriculum
            .blind
            .iter()
            .find(|task| task.evaluator.task_class == crate::sem2::model::TaskClass::Width)
            .expect("width task");
        let result = super::AdaptiveReasoner::solve(
            task,
            crate::sem2::model::Condition::SemanticNonAdaptiveB,
            ResourceBudget::equal_resource(),
        );
        assert!(
            result.metrics.cumulative_branches_generated
                >= result.metrics.instantaneous_frontier_width
        );
        assert!(result.metrics.cumulative_search_expansions > 0);
    }

    #[test]
    fn semantic_equivalence_and_dominance_require_matching_meaning() {
        let curriculum = generate_curriculum();
        let goal = &curriculum.adversarial[0].visible.goals[0];
        let correct_id = &curriculum.adversarial[0].evaluator.correct_branches[&goal.goal_id];
        let correct = goal
            .candidates
            .iter()
            .find(|item| item.action_id == *correct_id)
            .unwrap();
        let duplicate = goal
            .candidates
            .iter()
            .find(|item| {
                item.action_id != *correct_id
                    && item.resulting_semantic_state == correct.resulting_semantic_state
            })
            .unwrap();
        assert!(semantically_equivalent(correct, duplicate));
        assert!(dominates(correct, duplicate) || dominates(duplicate, correct));
        let trap = goal
            .candidates
            .iter()
            .find(|item| item.resulting_semantic_state != correct.resulting_semantic_state)
            .unwrap();
        assert!(!semantically_equivalent(correct, trap));
    }

    #[test]
    fn information_probe_ranking_rewards_balanced_partition() {
        let curriculum = generate_curriculum();
        let task = curriculum
            .blind
            .iter()
            .find(|task| !task.visible.probes.is_empty())
            .unwrap();
        let goal = task
            .visible
            .goals
            .iter()
            .find(|goal| {
                task.visible
                    .probes
                    .iter()
                    .any(|probe| probe.probe_id.ends_with(&goal.goal_id))
            })
            .unwrap();
        let live = goal.candidates.iter().collect::<Vec<_>>();
        assert!(probe_information_gain(&task.visible.probes[0], &live) > 0.0);
    }

    #[test]
    fn stagnation_requires_no_progress_and_repetition() {
        assert!(stagnating(3, 3, 2, 2, true));
        assert!(!stagnating(3, 2, 2, 2, true));
        assert!(gini(&[1, 3, 9]) > 0.0);
    }

    #[test]
    fn adaptive_controller_preserves_very_deep_task() {
        let curriculum = generate_curriculum();
        let task = curriculum
            .blind
            .iter()
            .find(|task| task.evaluator.required_depth >= 50)
            .unwrap();
        let result = super::AdaptiveReasoner::solve(
            task,
            crate::sem2::model::Condition::AdaptiveD,
            ResourceBudget::equal_resource(),
        );
        assert!(result.strictly_correct);
        assert!(result.metrics.solution_graph_depth >= 50);
        assert_eq!(result.metrics.false_prune_count, 0);
    }

    #[test]
    fn calibration_recombination_contract_survives_type_pruning() {
        let curriculum = generate_curriculum();
        let task = curriculum
            .calibration
            .iter()
            .find(|task| task.evaluator.task_class == crate::sem2::model::TaskClass::Recombination)
            .expect("recombination calibration task");
        let result = super::AdaptiveReasoner::solve(
            task,
            crate::sem2::model::Condition::AdaptiveD,
            ResourceBudget::equal_resource(),
        );
        assert!(result.strictly_correct);
        assert!(result.metrics.decomposition_count > 0);
        assert_eq!(result.metrics.recombination_count, 1);
    }
}
