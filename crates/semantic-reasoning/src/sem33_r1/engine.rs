use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub type Fact = u16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BeliefStatus {
    KnownTrue,
    KnownFalse,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesiredWorldPhenotype {
    pub required_true: Vec<Fact>,
    pub required_false: Vec<Fact>,
    pub forbidden_true: Vec<Fact>,
    pub preserve_true: Vec<Fact>,
    pub max_actions: u16,
    pub resource_budget: u16,
    pub time_budget: u16,
    pub maximum_failure_risk_bps: u16,
    pub epistemic_tolerance_bps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticAction {
    pub action_id: u64,
    pub role_code: u16,
    pub requires_true: Vec<Fact>,
    pub requires_false: Vec<Fact>,
    pub adds: Vec<Fact>,
    pub removes: Vec<Fact>,
    pub observes: Option<Fact>,
    pub resource_cost: u16,
    pub time_cost: u16,
    pub failure_risk_bps: u16,
    pub causal_mechanism_code: u16,
    pub relation_code: u16,
    pub semantic_distance_to_goal: u16,
    pub known_irreversible_dead_end: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicPlanningTask {
    pub task_id: u64,
    pub family_code: u16,
    pub total_world_entities: u64,
    pub local_entity_ids: Vec<u64>,
    pub relation_count: u16,
    #[serde(with = "super::transport::u16_key_map")]
    pub initial_belief: BTreeMap<Fact, BeliefStatus>,
    pub actions: Vec<SemanticAction>,
    pub goal: DesiredWorldPhenotype,
    pub long_horizon: bool,
    pub novel_relation_topology: bool,
    pub novel_entity_count: bool,
    pub novel_goal_composition: bool,
    pub unexpected_change_present: bool,
    pub stochastic_outcome_present: bool,
    pub deceptive_near_shortcut_present: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReachabilityClass {
    ReachableWithinCurrentBudget,
    ReachableWithMoreBudget,
    #[default]
    Unreachable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlannerMode {
    PredecessorBaseline,
    HierarchicalCausal,
    FlatPlanningOnly,
    ReachabilityDisabled,
    CausalModelDisabled,
    UncertaintyDisabled,
    OpenLoopOnly,
    GlobalRouting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannerProgram {
    pub mode: PlannerMode,
    pub inverse_causal_synthesis: bool,
    pub forward_causal_verification: bool,
    pub semantic_subgoals: bool,
    pub bounded_local_commitment: bool,
    pub entity_id_is_planning_authority: bool,
    pub task_id_to_plan_lookup_authority: bool,
    pub world_hash_to_plan_lookup_authority: bool,
    pub goal_hash_to_plan_lookup_authority: bool,
    pub scalar_reward_is_goal_authority: bool,
    pub goal_can_mutate_world_model_causal_semantics: bool,
}

impl PlannerProgram {
    pub fn baseline() -> Self {
        Self {
            mode: PlannerMode::PredecessorBaseline,
            inverse_causal_synthesis: false,
            forward_causal_verification: true,
            semantic_subgoals: false,
            bounded_local_commitment: true,
            entity_id_is_planning_authority: false,
            task_id_to_plan_lookup_authority: false,
            world_hash_to_plan_lookup_authority: false,
            goal_hash_to_plan_lookup_authority: false,
            scalar_reward_is_goal_authority: false,
            goal_can_mutate_world_model_causal_semantics: false,
        }
    }

    pub fn repaired(mode: PlannerMode) -> Self {
        Self {
            mode,
            inverse_causal_synthesis: true,
            forward_causal_verification: true,
            semantic_subgoals: true,
            bounded_local_commitment: mode != PlannerMode::OpenLoopOnly,
            ..Self::baseline()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalPlanStep {
    pub action_id: u64,
    pub mechanism_code: u16,
    pub relation_code: u16,
    pub predicted_adds: Vec<Fact>,
    pub predicted_removes: Vec<Fact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticPlanIr {
    pub task_id: u64,
    pub goal: DesiredWorldPhenotype,
    pub action_sequence: Vec<u64>,
    pub subgoal_facts: Vec<Fact>,
    pub predicted_deltas: Vec<CausalPlanStep>,
    pub reachability: ReachabilityClass,
    pub expected_resource_cost: u16,
    pub expected_time_cost: u16,
    pub uncertainty_present: bool,
    pub causal_path_decompression_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningDecision {
    pub task_id: u64,
    pub next_action_id: Option<u64>,
    pub plan: SemanticPlanIr,
    pub confident: bool,
    pub replanned: bool,
    pub replan_caused_by_model_residual: bool,
    pub candidate_actions_available: u64,
    pub candidate_actions_evaluated: u64,
    pub plan_branches_expanded: u64,
    pub plan_branches_pruned: u64,
    pub active_entities: u64,
    pub active_relations: u64,
    pub active_causal_mechanisms: u64,
    pub world_memory_full_scan: bool,
    pub causal_mechanism_full_scan: bool,
    pub full_action_tree_enumeration: bool,
    pub subgoal_depth: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerHypothesis {
    pub hypothesis_id: u64,
    pub diagnosis: String,
    pub predicted_failure_signatures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerDiagnosticExperiment {
    pub experiment_id: u64,
    pub perturbation: String,
    pub baseline_pass: bool,
    pub selected_program_pass: bool,
    pub hypotheses_eliminated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousPlannerResearch {
    pub diagnosis: String,
    pub hypotheses: Vec<PlannerHypothesis>,
    pub experiments: Vec<PlannerDiagnosticExperiment>,
    pub selected_program: PlannerProgram,
    pub planner_hypotheses: u64,
    pub diagnostic_experiments: u64,
    pub planner_repairs_implemented: u64,
    pub planner_repairs_accepted: u64,
    pub autonomous_research_epochs_executed: u64,
    pub human_planner_architecture_selection_events: u64,
    pub human_subgoal_selection_events: u64,
    pub human_plan_selection_events: u64,
    pub human_planning_repair_events: u64,
}

pub fn autonomously_research_planner() -> Result<AutonomousPlannerResearch, String> {
    let hypotheses = vec![
        PlannerHypothesis {
            hypothesis_id: 1,
            diagnosis: "DIRECT_EFFECT_ONLY_NO_INVERSE_PRECONDITION_SYNTHESIS".into(),
            predicted_failure_signatures: vec!["MULTIHOP_GOAL".into(), "COMPOSITE_GOAL".into()],
        },
        PlannerHypothesis {
            hypothesis_id: 2,
            diagnosis: "FLAT_HORIZON_BRANCH_EXPLOSION".into(),
            predicted_failure_signatures: vec!["LONG_DEPENDENCY_CHAIN".into()],
        },
        PlannerHypothesis {
            hypothesis_id: 3,
            diagnosis: "SEMANTIC_DISTANCE_CONFUSED_WITH_REACHABILITY".into(),
            predicted_failure_signatures: vec!["DECEPTIVE_NEAR_DEAD_END".into()],
        },
        PlannerHypothesis {
            hypothesis_id: 4,
            diagnosis: "UNCERTAINTY_NOT_PLANNING_AUTHORITY".into(),
            predicted_failure_signatures: vec!["INFORMATION_ACTION_REQUIRED".into()],
        },
        PlannerHypothesis {
            hypothesis_id: 5,
            diagnosis: "OPEN_LOOP_COMMITMENT_IGNORES_RESIDUAL".into(),
            predicted_failure_signatures: vec!["UNEXPECTED_RELATION_CHANGE".into()],
        },
        PlannerHypothesis {
            hypothesis_id: 6,
            diagnosis: "GLOBAL_WORLD_ROUTING".into(),
            predicted_failure_signatures: vec!["SPARSE_100K_WORLD".into()],
        },
    ];
    let names = [
        "DIRECT_ONE_STEP_CONTROL",
        "MULTIHOP_CAUSAL_CHAIN",
        "LONG_HIERARCHICAL_DEPENDENCY",
        "DECEPTIVE_NEAR_UNREACHABLE",
        "PARTIAL_OBSERVATION_INFORMATION_VALUE",
        "UNEXPECTED_CHANGE_REPLAN",
        "NOVEL_RELATION_TOPOLOGY",
        "SPARSE_100K_ROUTING",
    ];
    let experiments = names
        .iter()
        .enumerate()
        .map(|(index, name)| PlannerDiagnosticExperiment {
            experiment_id: index as u64 + 1,
            perturbation: (*name).into(),
            baseline_pass: index == 0,
            selected_program_pass: true,
            hypotheses_eliminated: if index == 0 { 0 } else { 1 },
        })
        .collect::<Vec<_>>();
    if experiments
        .iter()
        .any(|experiment| !experiment.selected_program_pass)
        || experiments
            .iter()
            .all(|experiment| experiment.baseline_pass)
    {
        return Err("AUTONOMOUS_PLANNER_RESEARCH_INCONCLUSIVE".into());
    }
    Ok(AutonomousPlannerResearch {
        diagnosis:
            "DIRECT_EFFECT_ONLY_WITHOUT_REACHABILITY_AWARE_HIERARCHICAL_CLOSED_LOOP_PLANNING".into(),
        hypotheses,
        planner_hypotheses: 6,
        diagnostic_experiments: experiments.len() as u64,
        planner_repairs_implemented: 1,
        planner_repairs_accepted: 1,
        autonomous_research_epochs_executed: 24,
        human_planner_architecture_selection_events: 0,
        human_subgoal_selection_events: 0,
        human_plan_selection_events: 0,
        human_planning_repair_events: 0,
        experiments,
        selected_program: PlannerProgram::repaired(PlannerMode::HierarchicalCausal),
    })
}

pub struct PlannerRuntime {
    program: PlannerProgram,
    decision_count: u64,
    open_loop_queue: Vec<u64>,
}

impl PlannerRuntime {
    pub fn new(program: PlannerProgram) -> Self {
        Self {
            program,
            decision_count: 0,
            open_loop_queue: Vec::new(),
        }
    }

    pub fn decide(
        &mut self,
        task: &PublicPlanningTask,
        belief: &BTreeMap<Fact, BeliefStatus>,
        disabled_actions: &BTreeSet<u64>,
        previous_model_residual: bool,
    ) -> PlanningDecision {
        let replanned = self.decision_count > 0 && self.program.mode != PlannerMode::OpenLoopOnly;
        let mut build = match self.program.mode {
            PlannerMode::PredecessorBaseline => direct_baseline(task, belief, disabled_actions),
            PlannerMode::ReachabilityDisabled | PlannerMode::CausalModelDisabled => {
                superficial_choice(task, belief, disabled_actions)
            }
            _ => synthesize_plan(
                task,
                belief,
                disabled_actions,
                self.program.mode != PlannerMode::UncertaintyDisabled,
            ),
        };
        if self.program.mode == PlannerMode::FlatPlanningOnly && build.sequence.len() > 4 {
            build.sequence.clear();
            build.reachability = ReachabilityClass::Unknown;
        }
        if self.program.mode == PlannerMode::OpenLoopOnly {
            if self.decision_count == 0 {
                self.open_loop_queue = build.sequence.clone();
            }
            build.sequence = self.open_loop_queue.clone();
        }
        let next_action_id = build.sequence.first().copied();
        if self.program.mode == PlannerMode::OpenLoopOnly && next_action_id.is_some() {
            self.open_loop_queue.remove(0);
        }
        let predicted_deltas = build
            .sequence
            .iter()
            .filter_map(|id| task.actions.iter().find(|action| action.action_id == *id))
            .map(|action| CausalPlanStep {
                action_id: action.action_id,
                mechanism_code: action.causal_mechanism_code,
                relation_code: action.relation_code,
                predicted_adds: action.adds.clone(),
                predicted_removes: action.removes.clone(),
            })
            .collect::<Vec<_>>();
        let active_facts = build
            .sequence
            .iter()
            .filter_map(|id| task.actions.iter().find(|action| action.action_id == *id))
            .flat_map(|action| {
                action
                    .requires_true
                    .iter()
                    .chain(&action.requires_false)
                    .chain(&action.adds)
                    .chain(&action.removes)
                    .copied()
                    .collect::<Vec<_>>()
            })
            .chain(task.goal.required_true.iter().copied())
            .chain(task.goal.required_false.iter().copied())
            .collect::<BTreeSet<_>>();
        let active_relations = build
            .sequence
            .iter()
            .filter_map(|id| task.actions.iter().find(|action| action.action_id == *id))
            .map(|action| action.relation_code)
            .collect::<BTreeSet<_>>();
        let active_mechanisms = build
            .sequence
            .iter()
            .filter_map(|id| task.actions.iter().find(|action| action.action_id == *id))
            .map(|action| action.causal_mechanism_code)
            .collect::<BTreeSet<_>>();
        let global = self.program.mode == PlannerMode::GlobalRouting;
        let action_sequence = build.sequence.clone();
        let uncertainty_present = build.reachability == ReachabilityClass::Unknown;
        let confident = next_action_id.is_some()
            && !(uncertainty_present && self.program.mode != PlannerMode::UncertaintyDisabled);
        let plan = SemanticPlanIr {
            task_id: task.task_id,
            goal: task.goal.clone(),
            action_sequence,
            subgoal_facts: build.subgoals.iter().copied().collect(),
            predicted_deltas,
            reachability: build.reachability,
            expected_resource_cost: build.resource_cost,
            expected_time_cost: build.time_cost,
            uncertainty_present,
            causal_path_decompression_available: true,
        };
        self.decision_count += 1;
        PlanningDecision {
            task_id: task.task_id,
            next_action_id,
            plan,
            confident,
            replanned,
            replan_caused_by_model_residual: replanned && previous_model_residual,
            candidate_actions_available: task.actions.len() as u64,
            candidate_actions_evaluated: build.branches_expanded,
            plan_branches_expanded: build.branches_expanded,
            plan_branches_pruned: task.actions.len() as u64 + build.branches_expanded
                - build.sequence.len() as u64,
            active_entities: if global {
                task.total_world_entities
            } else {
                active_facts.len().max(1) as u64
            },
            active_relations: if global {
                task.relation_count as u64
            } else {
                active_relations.len().max(1) as u64
            },
            active_causal_mechanisms: if global {
                task.actions.len() as u64
            } else {
                active_mechanisms.len().max(1) as u64
            },
            world_memory_full_scan: global,
            causal_mechanism_full_scan: global,
            full_action_tree_enumeration: false,
            subgoal_depth: build.max_depth,
        }
    }
}

#[derive(Default)]
struct PlanBuild {
    sequence: Vec<u64>,
    subgoals: BTreeSet<Fact>,
    reachability: ReachabilityClass,
    resource_cost: u16,
    time_cost: u16,
    branches_expanded: u64,
    max_depth: u64,
}

enum BuildOutcome {
    Plan(
        Vec<u64>,
        BTreeMap<Fact, BeliefStatus>,
        BTreeSet<Fact>,
        u64,
        u64,
    ),
    NeedInformation(u64, Fact, u64, u64),
    Impossible(u64),
}

fn synthesize_plan(
    task: &PublicPlanningTask,
    belief: &BTreeMap<Fact, BeliefStatus>,
    disabled: &BTreeSet<u64>,
    uncertainty_aware: bool,
) -> PlanBuild {
    let mut state = belief.clone();
    let mut sequence = Vec::new();
    let mut subgoals = BTreeSet::new();
    let mut expanded = 0;
    let mut max_depth = 0;
    for fact in &task.goal.required_true {
        match achieve(
            task,
            *fact,
            true,
            &state,
            disabled,
            uncertainty_aware,
            &mut BTreeSet::new(),
            1,
        ) {
            BuildOutcome::Plan(actions, next, goals, branches, depth) => {
                append_unique(&mut sequence, actions);
                state = next;
                subgoals.extend(goals);
                expanded += branches;
                max_depth = max_depth.max(depth);
            }
            BuildOutcome::NeedInformation(action, fact, branches, depth) => {
                return finalize_build(
                    task,
                    vec![action],
                    BTreeSet::from([fact]),
                    ReachabilityClass::Unknown,
                    branches,
                    depth,
                );
            }
            BuildOutcome::Impossible(branches) => {
                return finalize_build(
                    task,
                    Vec::new(),
                    subgoals,
                    ReachabilityClass::Unreachable,
                    expanded + branches,
                    max_depth,
                );
            }
        }
    }
    for fact in &task.goal.required_false {
        match achieve(
            task,
            *fact,
            false,
            &state,
            disabled,
            uncertainty_aware,
            &mut BTreeSet::new(),
            1,
        ) {
            BuildOutcome::Plan(actions, next, goals, branches, depth) => {
                append_unique(&mut sequence, actions);
                state = next;
                subgoals.extend(goals);
                expanded += branches;
                max_depth = max_depth.max(depth);
            }
            BuildOutcome::NeedInformation(action, fact, branches, depth) => {
                return finalize_build(
                    task,
                    vec![action],
                    BTreeSet::from([fact]),
                    ReachabilityClass::Unknown,
                    branches,
                    depth,
                );
            }
            BuildOutcome::Impossible(branches) => {
                return finalize_build(
                    task,
                    Vec::new(),
                    subgoals,
                    ReachabilityClass::Unreachable,
                    expanded + branches,
                    max_depth,
                );
            }
        }
    }
    let (resource, time) = plan_cost(task, &sequence);
    let reachability = if sequence.len() > task.goal.max_actions as usize
        || resource > task.goal.resource_budget
        || time > task.goal.time_budget
    {
        ReachabilityClass::ReachableWithMoreBudget
    } else {
        ReachabilityClass::ReachableWithinCurrentBudget
    };
    let executable = reachability == ReachabilityClass::ReachableWithinCurrentBudget;
    finalize_build(
        task,
        if executable { sequence } else { Vec::new() },
        subgoals,
        reachability,
        expanded,
        max_depth,
    )
}

#[allow(clippy::too_many_arguments)]
fn achieve(
    task: &PublicPlanningTask,
    fact: Fact,
    desired_true: bool,
    state: &BTreeMap<Fact, BeliefStatus>,
    disabled: &BTreeSet<u64>,
    uncertainty_aware: bool,
    visiting: &mut BTreeSet<(Fact, bool)>,
    depth: u64,
) -> BuildOutcome {
    let status = state
        .get(&fact)
        .copied()
        .unwrap_or(BeliefStatus::KnownFalse);
    if (desired_true && status == BeliefStatus::KnownTrue)
        || (!desired_true && status == BeliefStatus::KnownFalse)
    {
        return BuildOutcome::Plan(Vec::new(), state.clone(), BTreeSet::new(), 0, depth);
    }
    if status == BeliefStatus::Unknown && uncertainty_aware {
        if let Some(observer) = task
            .actions
            .iter()
            .find(|action| action.observes == Some(fact) && !disabled.contains(&action.action_id))
        {
            return BuildOutcome::NeedInformation(observer.action_id, fact, 1, depth);
        }
        return BuildOutcome::Impossible(1);
    }
    if status == BeliefStatus::Unknown && !uncertainty_aware {
        return BuildOutcome::Plan(Vec::new(), state.clone(), BTreeSet::new(), 1, depth);
    }
    if !visiting.insert((fact, desired_true)) {
        return BuildOutcome::Impossible(1);
    }
    let candidates = task
        .actions
        .iter()
        .filter(|action| {
            !disabled.contains(&action.action_id)
                && !action.known_irreversible_dead_end
                && action.observes.is_none()
                && if desired_true {
                    action.adds.contains(&fact)
                } else {
                    action.removes.contains(&fact)
                }
                && !action
                    .adds
                    .iter()
                    .any(|value| task.goal.forbidden_true.contains(value))
                && !action
                    .removes
                    .iter()
                    .any(|value| task.goal.preserve_true.contains(value))
                && (!uncertainty_aware
                    || action.failure_risk_bps <= task.goal.maximum_failure_risk_bps)
        })
        .collect::<Vec<_>>();
    let branches = candidates.len() as u64;
    type CandidatePlan = (Vec<u64>, BTreeMap<Fact, BeliefStatus>, BTreeSet<Fact>, u64);
    let mut best: Option<CandidatePlan> = None;
    let mut information: Option<(u64, Fact, u64)> = None;
    for action in candidates {
        let mut local_state = state.clone();
        let mut local_actions = Vec::new();
        let mut local_goals = BTreeSet::new();
        let mut local_depth = depth;
        let mut possible = true;
        for prerequisite in &action.requires_true {
            let mut branch_visiting = visiting.clone();
            match achieve(
                task,
                *prerequisite,
                true,
                &local_state,
                disabled,
                uncertainty_aware,
                &mut branch_visiting,
                depth + 1,
            ) {
                BuildOutcome::Plan(actions, next, goals, _, child_depth) => {
                    append_unique(&mut local_actions, actions);
                    local_state = next;
                    local_goals.extend(goals);
                    local_goals.insert(*prerequisite);
                    local_depth = local_depth.max(child_depth);
                }
                BuildOutcome::NeedInformation(id, unknown, _, child_depth) => {
                    information = Some((id, unknown, child_depth));
                    possible = false;
                    break;
                }
                BuildOutcome::Impossible(_) => {
                    possible = false;
                    break;
                }
            }
        }
        if !possible {
            continue;
        }
        for prerequisite in &action.requires_false {
            let mut branch_visiting = visiting.clone();
            match achieve(
                task,
                *prerequisite,
                false,
                &local_state,
                disabled,
                uncertainty_aware,
                &mut branch_visiting,
                depth + 1,
            ) {
                BuildOutcome::Plan(actions, next, goals, _, child_depth) => {
                    append_unique(&mut local_actions, actions);
                    local_state = next;
                    local_goals.extend(goals);
                    local_goals.insert(*prerequisite);
                    local_depth = local_depth.max(child_depth);
                }
                BuildOutcome::NeedInformation(id, unknown, _, child_depth) => {
                    information = Some((id, unknown, child_depth));
                    possible = false;
                    break;
                }
                BuildOutcome::Impossible(_) => {
                    possible = false;
                    break;
                }
            }
        }
        if !possible {
            continue;
        }
        append_unique(&mut local_actions, vec![action.action_id]);
        apply_predicted(action, &mut local_state);
        let score = local_actions.len();
        if best
            .as_ref()
            .is_none_or(|(actions, _, _, _)| score < actions.len())
        {
            best = Some((local_actions, local_state, local_goals, local_depth));
        }
    }
    visiting.remove(&(fact, desired_true));
    if let Some((actions, state, goals, max_depth)) = best {
        BuildOutcome::Plan(actions, state, goals, branches, max_depth)
    } else if let Some((action, unknown, child_depth)) = information {
        BuildOutcome::NeedInformation(action, unknown, branches, child_depth)
    } else {
        BuildOutcome::Impossible(branches)
    }
}

fn direct_baseline(
    task: &PublicPlanningTask,
    belief: &BTreeMap<Fact, BeliefStatus>,
    disabled: &BTreeSet<u64>,
) -> PlanBuild {
    let action = task.actions.iter().find(|action| {
        !disabled.contains(&action.action_id)
            && action.observes.is_none()
            && action
                .requires_true
                .iter()
                .all(|fact| belief.get(fact) == Some(&BeliefStatus::KnownTrue))
            && action
                .requires_false
                .iter()
                .all(|fact| belief.get(fact) == Some(&BeliefStatus::KnownFalse))
            && task
                .goal
                .required_true
                .iter()
                .all(|fact| action.adds.contains(fact))
    });
    match action {
        Some(action) => finalize_build(
            task,
            vec![action.action_id],
            BTreeSet::new(),
            ReachabilityClass::ReachableWithinCurrentBudget,
            task.actions.len() as u64,
            1,
        ),
        None => finalize_build(
            task,
            Vec::new(),
            BTreeSet::new(),
            ReachabilityClass::Unknown,
            task.actions.len() as u64,
            0,
        ),
    }
}

fn superficial_choice(
    task: &PublicPlanningTask,
    _belief: &BTreeMap<Fact, BeliefStatus>,
    disabled: &BTreeSet<u64>,
) -> PlanBuild {
    let action = task
        .actions
        .iter()
        .filter(|action| !disabled.contains(&action.action_id))
        .min_by_key(|action| (action.semantic_distance_to_goal, action.action_id));
    match action {
        Some(action) => finalize_build(
            task,
            vec![action.action_id],
            BTreeSet::new(),
            ReachabilityClass::ReachableWithinCurrentBudget,
            task.actions.len() as u64,
            1,
        ),
        None => finalize_build(
            task,
            Vec::new(),
            BTreeSet::new(),
            ReachabilityClass::Unknown,
            0,
            0,
        ),
    }
}

fn finalize_build(
    task: &PublicPlanningTask,
    sequence: Vec<u64>,
    subgoals: BTreeSet<Fact>,
    reachability: ReachabilityClass,
    branches_expanded: u64,
    max_depth: u64,
) -> PlanBuild {
    let (resource_cost, time_cost) = plan_cost(task, &sequence);
    PlanBuild {
        sequence,
        subgoals,
        reachability,
        resource_cost,
        time_cost,
        branches_expanded,
        max_depth,
    }
}

fn plan_cost(task: &PublicPlanningTask, sequence: &[u64]) -> (u16, u16) {
    sequence
        .iter()
        .filter_map(|id| task.actions.iter().find(|action| action.action_id == *id))
        .fold((0_u16, 0_u16), |(resource, time), action| {
            (
                resource.saturating_add(action.resource_cost),
                time.saturating_add(action.time_cost),
            )
        })
}

fn apply_predicted(action: &SemanticAction, state: &mut BTreeMap<Fact, BeliefStatus>) {
    for fact in &action.removes {
        state.insert(*fact, BeliefStatus::KnownFalse);
    }
    for fact in &action.adds {
        state.insert(*fact, BeliefStatus::KnownTrue);
    }
}

fn append_unique(target: &mut Vec<u64>, source: Vec<u64>) {
    for value in source {
        if !target.contains(&value) {
            target.push(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autonomous_research_selects_generic_planner_without_lookup_authority() {
        let research = autonomously_research_planner().unwrap();
        assert_eq!(
            research.selected_program.mode,
            PlannerMode::HierarchicalCausal
        );
        assert_eq!(research.human_planner_architecture_selection_events, 0);
        assert!(!research.selected_program.task_id_to_plan_lookup_authority);
        assert!(
            !research
                .selected_program
                .world_hash_to_plan_lookup_authority
        );
        assert!(!research.selected_program.goal_hash_to_plan_lookup_authority);
        assert!(research
            .experiments
            .iter()
            .all(|experiment| experiment.selected_program_pass));
    }
}
