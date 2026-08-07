use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::dsl::{ExecutionError, ScalarOperator};
use crate::substrate::CounterfactualCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Split {
    TrainDiscovery,
    Calibration,
    FreshBlind,
    AdversarialCounterfactual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Demonstration {
    pub input: Vec<i64>,
    pub observed_output: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleTask {
    pub task_id: String,
    pub split: Split,
    pub scalar_parameter: i64,
    pub demonstrations: Vec<Demonstration>,
    pub query_input: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct EvaluationTask {
    visible: VisibleTask,
    hidden_operator: ScalarOperator,
}

#[derive(Debug, Clone)]
pub struct CounterfactualTask {
    pub case_id: String,
    pub kind: CounterfactualCode,
    pub task: EvaluationTask,
    pub expects_precondition_rejection: bool,
}

impl CounterfactualTask {
    pub fn score(&self, result: &crate::reasoning::SolveResult) -> bool {
        if self.expects_precondition_rejection {
            return result.committed_output.is_none()
                && (result.execution_error.is_some()
                    || result.termination == "SCALAR_INFERENCE_EXHAUSTED");
        }
        self.task.score_committed(&result.committed())
    }
}

impl EvaluationTask {
    pub fn visible(&self) -> &VisibleTask {
        &self.visible
    }

    pub fn score_committed(&self, committed: &Result<Vec<i64>, ExecutionError>) -> bool {
        committed == &oracle(&self.visible.query_input, self.hidden_operator)
    }

    pub fn expected_after_commit(&self) -> Result<Vec<i64>, ExecutionError> {
        oracle(&self.visible.query_input, self.hidden_operator)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManifest {
    pub generator_version: String,
    pub seed: u64,
    pub expected_query_outputs_included: bool,
    pub hidden_generator_metadata_included: bool,
    pub tasks: Vec<VisibleTask>,
    pub manifest_sha256: String,
}

impl TaskManifest {
    pub fn new(seed: u64, tasks: &[EvaluationTask]) -> Self {
        let visible_tasks: Vec<VisibleTask> =
            tasks.iter().map(|task| task.visible.clone()).collect();
        let mut manifest = Self {
            generator_version: "SEM0_TASK_GENERATOR_V1".to_string(),
            seed,
            expected_query_outputs_included: false,
            hidden_generator_metadata_included: false,
            tasks: visible_tasks,
            manifest_sha256: String::new(),
        };
        let bytes = serde_json::to_vec(&manifest).expect("task manifest serializes");
        manifest.manifest_sha256 = format!("{:x}", Sha256::digest(bytes));
        manifest
    }
}

pub fn generate_tasks() -> (
    Vec<EvaluationTask>,
    Vec<EvaluationTask>,
    Vec<EvaluationTask>,
) {
    let train_specs = [
        (ScalarOperator::Add(2), vec![3, -1, 5]),
        (ScalarOperator::Mul(3), vec![4, 0, -2, 7]),
        (ScalarOperator::Sub(4), vec![9, -3]),
        (ScalarOperator::Add(-3), vec![8, 1, -6, 2, 0]),
        (ScalarOperator::Mul(-2), vec![-4, 5, 1]),
        (ScalarOperator::Sub(-5), vec![0, 6, -8, 3]),
    ];
    let calibration_specs = [
        (ScalarOperator::Add(11), vec![2, -5, 10]),
        (ScalarOperator::Mul(4), vec![3, -2, 0, 9]),
        (ScalarOperator::Sub(7), vec![12, 1, -9]),
    ];
    let blind_specs = [
        (ScalarOperator::Mul(5), vec![7, -3, 2, 0, 11]),
        (ScalarOperator::Add(13), vec![-8, 4, 19]),
        (ScalarOperator::Sub(9), vec![20, -2, 5, 14]),
        (ScalarOperator::Mul(-3), vec![6, -7, 1]),
        (ScalarOperator::Add(-12), vec![15, 0, -6, 8, 3, -1]),
        (ScalarOperator::Sub(-8), vec![-10, 2, 17]),
    ];

    (
        build_split("T", Split::TrainDiscovery, &train_specs),
        build_split("Q", Split::Calibration, &calibration_specs),
        build_split("B", Split::FreshBlind, &blind_specs),
    )
}

pub fn generate_counterfactual_tasks() -> Vec<CounterfactualTask> {
    let specifications = [
        (
            CounterfactualCode::EmptyInput,
            ScalarOperator::Add(5),
            vec![],
            false,
        ),
        (
            CounterfactualCode::SingletonInput,
            ScalarOperator::Mul(6),
            vec![-3],
            false,
        ),
        (
            CounterfactualCode::RepeatedValues,
            ScalarOperator::Sub(2),
            vec![4, 4, 4, 4],
            false,
        ),
        (
            CounterfactualCode::NegativeValues,
            ScalarOperator::Add(-7),
            vec![-1, -8, -13],
            false,
        ),
        (
            CounterfactualCode::ReorderedInput,
            ScalarOperator::Mul(2),
            vec![9, 1, 7, -2],
            false,
        ),
        (
            CounterfactualCode::ChangedOperator,
            ScalarOperator::Sub(5),
            vec![3, 10, -4],
            false,
        ),
        (
            CounterfactualCode::ChangedParameter,
            ScalarOperator::Add(21),
            vec![0, -9, 2],
            false,
        ),
        (
            CounterfactualCode::NumericBoundary,
            ScalarOperator::Sub(1),
            vec![i64::MIN + 1, i64::MAX],
            false,
        ),
        (
            CounterfactualCode::ArithmeticOverflow,
            ScalarOperator::Add(1),
            vec![i64::MAX],
            true,
        ),
        (
            CounterfactualCode::MissingEvidence,
            ScalarOperator::Mul(1),
            vec![2, 5],
            true,
        ),
    ];

    let mut cases = Vec::new();
    for (index, (kind, operator, query, expects_rejection)) in
        specifications.into_iter().enumerate()
    {
        let parameter = match operator {
            ScalarOperator::Add(value)
            | ScalarOperator::Sub(value)
            | ScalarOperator::Mul(value) => value,
        };
        let demonstrations = if kind == CounterfactualCode::MissingEvidence {
            Vec::new()
        } else {
            [vec![1, -2, 4], vec![0, 3]]
                .iter()
                .map(|input| Demonstration {
                    input: input.clone(),
                    observed_output: oracle(input, operator)
                        .expect("counterfactual demonstrations are valid"),
                })
                .collect()
        };
        cases.push(CounterfactualTask {
            case_id: format!("X{:06}", index + 1),
            kind,
            task: EvaluationTask {
                visible: VisibleTask {
                    task_id: format!("X{:06}", index + 1),
                    split: Split::AdversarialCounterfactual,
                    scalar_parameter: parameter,
                    demonstrations,
                    query_input: query,
                },
                hidden_operator: operator,
            },
            expects_precondition_rejection: expects_rejection,
        });
    }
    cases
}

fn build_split(
    prefix: &str,
    split: Split,
    specs: &[(ScalarOperator, Vec<i64>)],
) -> Vec<EvaluationTask> {
    let mut tasks = Vec::new();
    for (index, (operator, query)) in specs.iter().enumerate() {
        let parameter = match operator {
            ScalarOperator::Add(value)
            | ScalarOperator::Sub(value)
            | ScalarOperator::Mul(value) => *value,
        };
        let demo_inputs = [vec![1, -2, 4], vec![0, 3]];
        let demonstrations = demo_inputs
            .iter()
            .map(|input| Demonstration {
                input: input.clone(),
                observed_output: oracle(input, *operator).expect("small demonstrations are valid"),
            })
            .collect();
        tasks.push(EvaluationTask {
            visible: VisibleTask {
                task_id: format!("{prefix}{:06}", index + 1),
                split,
                scalar_parameter: parameter,
                demonstrations,
                query_input: query.clone(),
            },
            hidden_operator: *operator,
        });
    }
    tasks
}

fn oracle(input: &[i64], operator: ScalarOperator) -> Result<Vec<i64>, ExecutionError> {
    let mut output = Vec::with_capacity(input.len());
    for value in input {
        output.push(operator.apply(*value)?);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{generate_tasks, TaskManifest};

    #[test]
    fn blind_manifest_excludes_hidden_answers_and_metadata() {
        let (_, _, blind) = generate_tasks();
        let manifest = TaskManifest::new(20260807, &blind);
        let serialized = serde_json::to_string(&manifest).expect("serialize manifest");
        assert!(!manifest.expected_query_outputs_included);
        assert!(!manifest.hidden_generator_metadata_included);
        assert!(!serialized.contains("hidden_operator"));
        assert_eq!(manifest.tasks.len(), 6);
        assert_eq!(manifest.manifest_sha256.len(), 64);
    }
}
