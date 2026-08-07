use serde::{Deserialize, Serialize};

use super::{
    integrity::hash_serializable,
    model::{
        execute_primitive_pipeline, CheckedOperator, Predicate, Reducer, Sem1ValueType, Stage,
        StageCapability, StageKind, Value,
    },
};

pub const CURRICULUM_GENERATOR_VERSION: &str = "SEM1-CURRICULUM-1.1.0";
pub const TRAIN_SEED: u64 = 20_260_807_201;
pub const BLIND_SEED: u64 = 20_260_807_929;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sem1Split {
    Discovery,
    Calibration,
    FreshBlind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Demonstration {
    pub input: Value,
    pub output: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleTask {
    pub task_id: String,
    pub input_type: Sem1ValueType,
    pub output_type: Sem1ValueType,
    pub demonstrations: Vec<Demonstration>,
    pub query_input: Value,
    pub capabilities: Vec<StageCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "expectation",
    content = "value",
    rename_all = "SCREAMING_SNAKE_CASE"
)]
pub enum ExpectedOutcome {
    Value(Value),
    SemanticInvalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationTask {
    pub visible: VisibleTask,
    pub split: Sem1Split,
    #[serde(skip_serializing)]
    pub expected_query: ExpectedOutcome,
    #[serde(skip_serializing)]
    pub hidden_stage_kinds: Vec<StageKind>,
    #[serde(skip_serializing)]
    pub hidden_program: Vec<Stage>,
    #[serde(skip_serializing)]
    pub hidden_case_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindManifest {
    pub generator_version: String,
    pub seed: u64,
    pub tasks: Vec<VisibleTask>,
    pub expected_query_outputs_included: bool,
    pub hidden_generator_metadata_included: bool,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurriculumManifest {
    pub generator_version: String,
    pub seed: u64,
    pub discovery_tasks: usize,
    pub calibration_tasks: usize,
    pub blind_tasks: usize,
    pub input_types: Vec<Sem1ValueType>,
    pub output_types: Vec<Sem1ValueType>,
    pub exposed_task_family_metadata: bool,
    pub exposed_human_abstraction_names: bool,
    pub supported_composition_arities: Vec<usize>,
    pub curriculum_sha256: String,
}

#[derive(Clone)]
struct TaskSpec {
    stages: Vec<Stage>,
    demos: Vec<Vec<i64>>,
    query: Vec<i64>,
    invalid_query: bool,
    case_code: &'static str,
}

pub type CurriculumSplits = (
    Vec<EvaluationTask>,
    Vec<EvaluationTask>,
    Vec<EvaluationTask>,
);

pub fn generate_curriculum() -> Result<CurriculumSplits, String> {
    let discovery_specs = vec![
        spec_tf(
            CheckedOperator::Add(2),
            Predicate::Positive,
            vec![-5, -1, 0, 3],
            "D001",
        ),
        spec_tf(
            CheckedOperator::Mul(3),
            Predicate::Even,
            vec![-3, 1, 4, 7],
            "D002",
        ),
        spec_tf(
            CheckedOperator::Sub(3),
            Predicate::NonZero,
            vec![0, 3, 5, 8],
            "D003",
        ),
        spec_tf(
            CheckedOperator::Add(5),
            Predicate::Negative,
            vec![-10, -6, 0, 4],
            "D004",
        ),
        spec_ta(
            CheckedOperator::Add(3),
            Reducer::Sum,
            vec![-2, 0, 5],
            "D005",
        ),
        spec_ta(
            CheckedOperator::Mul(2),
            Reducer::Product,
            vec![1, 2, 3],
            "D006",
        ),
        spec_ta(
            CheckedOperator::Sub(1),
            Reducer::Sum,
            vec![-4, 4, 9],
            "D007",
        ),
        spec_ta(
            CheckedOperator::Add(1),
            Reducer::Product,
            vec![0, 2, 4],
            "D008",
        ),
        spec_tfa(
            CheckedOperator::Add(2),
            Predicate::Positive,
            Reducer::Sum,
            vec![-5, -1, 2, 6],
            "D009",
        ),
        spec_tfa(
            CheckedOperator::Mul(3),
            Predicate::Even,
            Reducer::Product,
            vec![-2, 1, 3],
            "D010",
        ),
        spec_tfa(
            CheckedOperator::Sub(3),
            Predicate::NonZero,
            Reducer::Sum,
            vec![0, 3, 7],
            "D011",
        ),
        spec_tfa(
            CheckedOperator::Add(4),
            Predicate::Negative,
            Reducer::Product,
            vec![-9, -6, 0],
            "D012",
        ),
        spec_tt(
            CheckedOperator::Add(2),
            CheckedOperator::Mul(3),
            vec![-5, 0, 4, 11],
            "D013",
        ),
        spec_tt(
            CheckedOperator::Mul(2),
            CheckedOperator::Sub(5),
            vec![-3, 1, 7, 20],
            "D014",
        ),
        spec_tt(
            CheckedOperator::Sub(4),
            CheckedOperator::Add(9),
            vec![-8, 0, 6, 15],
            "D015",
        ),
        spec_tt(
            CheckedOperator::Add(-3),
            CheckedOperator::Mul(-2),
            vec![-10, -1, 3, 12],
            "D016",
        ),
    ];
    let calibration_specs = vec![
        spec_tf(
            CheckedOperator::Sub(2),
            Predicate::Positive,
            vec![-1, 2, 5, 9],
            "C001",
        ),
        spec_tf(
            CheckedOperator::Mul(-1),
            Predicate::Negative,
            vec![-4, 0, 3, 8],
            "C002",
        ),
        spec_ta(
            CheckedOperator::Add(4),
            Reducer::Sum,
            vec![-3, 1, 8],
            "C003",
        ),
        spec_ta(
            CheckedOperator::Sub(2),
            Reducer::Product,
            vec![3, 4, 5],
            "C004",
        ),
        spec_tfa(
            CheckedOperator::Mul(3),
            Predicate::Positive,
            Reducer::Sum,
            vec![-2, 0, 3, 5],
            "C005",
        ),
        spec_tfa(
            CheckedOperator::Add(1),
            Predicate::Even,
            Reducer::Product,
            vec![-3, 1, 5],
            "C006",
        ),
        spec_tt(
            CheckedOperator::Sub(7),
            CheckedOperator::Mul(2),
            vec![-2, 7, 19],
            "C007",
        ),
        spec_tt(
            CheckedOperator::Mul(-1),
            CheckedOperator::Add(6),
            vec![-9, 0, 8],
            "C008",
        ),
    ];
    let mut blind_specs = Vec::new();
    append_blind_group(&mut blind_specs, 0);
    append_blind_group(&mut blind_specs, 1);
    append_blind_group(&mut blind_specs, 2);
    append_blind_group(&mut blind_specs, 3);

    Ok((
        build_tasks("T2D", Sem1Split::Discovery, &discovery_specs)?,
        build_tasks("T2C", Sem1Split::Calibration, &calibration_specs)?,
        build_tasks("T2B", Sem1Split::FreshBlind, &blind_specs)?,
    ))
}

fn append_blind_group(target: &mut Vec<TaskSpec>, group: usize) {
    match group {
        0 => {
            target.push(spec_tf(
                CheckedOperator::Add(9),
                Predicate::Positive,
                vec![-31, -9, -2, 4, 37, 101],
                "B_TF_FRESH",
            ));
            target.push(spec_tf(
                CheckedOperator::Mul(-3),
                Predicate::Even,
                vec![-13, -2, 0, 7, 18, 29],
                "B_TF_SUBSTITUTE",
            ));
            target.push(spec_tf(
                CheckedOperator::AddViaSubNeg(4),
                Predicate::NonZero,
                vec![-4, -2, 0, 13, 41],
                "B_TF_EQUIVALENT",
            ));
            let mut invalid = spec_tf(
                CheckedOperator::Mul(3),
                Predicate::Positive,
                vec![i64::MAX, 2],
                "B_TF_INVALID",
            );
            invalid.invalid_query = true;
            target.push(invalid);
            target.push(spec_tf(
                CheckedOperator::Sub(13),
                Predicate::Negative,
                vec![],
                "B_TF_EMPTY",
            ));
        }
        1 => {
            target.push(spec_ta(
                CheckedOperator::Sub(6),
                Reducer::Sum,
                vec![-25, -1, 0, 22, 53],
                "B_TA_FRESH",
            ));
            target.push(spec_ta(
                CheckedOperator::Mul(-2),
                Reducer::Product,
                vec![-4, -1, 2, 6],
                "B_TA_SUBSTITUTE",
            ));
            target.push(spec_ta(
                CheckedOperator::MulViaRepeatedAdd(4),
                Reducer::Sum,
                vec![-3, 0, 5, 11],
                "B_TA_EQUIVALENT",
            ));
            let mut invalid = spec_ta(
                CheckedOperator::Add(2),
                Reducer::Sum,
                vec![i64::MAX],
                "B_TA_INVALID",
            );
            invalid.invalid_query = true;
            target.push(invalid);
            target.push(spec_ta(
                CheckedOperator::Add(10),
                Reducer::Product,
                vec![],
                "B_TA_EMPTY",
            ));
        }
        2 => {
            target.push(spec_tfa(
                CheckedOperator::Add(9),
                Predicate::Positive,
                Reducer::Sum,
                vec![-27, -9, -1, 5, 21],
                "B_TFA_FRESH",
            ));
            target.push(spec_tfa(
                CheckedOperator::Mul(-3),
                Predicate::Negative,
                Reducer::Product,
                vec![-4, 0, 1, 6],
                "B_TFA_SUBSTITUTE",
            ));
            target.push(spec_tfa(
                CheckedOperator::AddViaSubNeg(5),
                Predicate::Even,
                Reducer::Sum,
                vec![-10, -5, -1, 3, 14],
                "B_TFA_EQUIVALENT",
            ));
            let mut invalid = spec_tfa(
                CheckedOperator::Mul(2),
                Predicate::NonZero,
                Reducer::Product,
                vec![i64::MAX, 2],
                "B_TFA_INVALID",
            );
            invalid.invalid_query = true;
            target.push(invalid);
            target.push(spec_tfa(
                CheckedOperator::Sub(6),
                Predicate::Positive,
                Reducer::Sum,
                vec![],
                "B_TFA_EMPTY",
            ));
        }
        _ => {
            target.push(spec_tt(
                CheckedOperator::Add(13),
                CheckedOperator::Mul(-2),
                vec![-35, -13, 0, 18, 64],
                "B_TT_FRESH",
            ));
            target.push(spec_tt(
                CheckedOperator::Mul(4),
                CheckedOperator::Sub(7),
                vec![-11, -2, 0, 8, 27],
                "B_TT_SUBSTITUTE",
            ));
            target.push(spec_tt(
                CheckedOperator::AddViaSubNeg(6),
                CheckedOperator::MulViaRepeatedAdd(-2),
                vec![-14, -6, 2, 19],
                "B_TT_EQUIVALENT",
            ));
            let mut invalid = spec_tt(
                CheckedOperator::Add(2),
                CheckedOperator::Mul(3),
                vec![i64::MAX],
                "B_TT_INVALID",
            );
            invalid.invalid_query = true;
            target.push(invalid);
            target.push(spec_tt(
                CheckedOperator::Sub(8),
                CheckedOperator::Add(15),
                vec![],
                "B_TT_EMPTY",
            ));
        }
    }
}

fn spec_tt(
    first: CheckedOperator,
    second: CheckedOperator,
    query: Vec<i64>,
    code: &'static str,
) -> TaskSpec {
    let stages = vec![Stage::Transform(first), Stage::Transform(second)];
    TaskSpec {
        demos: diagnostic_demonstrations(),
        stages,
        query,
        invalid_query: false,
        case_code: code,
    }
}

fn spec_tf(
    operator: CheckedOperator,
    predicate: Predicate,
    query: Vec<i64>,
    code: &'static str,
) -> TaskSpec {
    let stages = vec![Stage::Transform(operator), Stage::Retain(predicate)];
    TaskSpec {
        demos: diagnostic_demonstrations(),
        stages,
        query,
        invalid_query: false,
        case_code: code,
    }
}

fn spec_ta(
    operator: CheckedOperator,
    reducer: Reducer,
    query: Vec<i64>,
    code: &'static str,
) -> TaskSpec {
    let stages = vec![Stage::Transform(operator), Stage::Aggregate(reducer)];
    TaskSpec {
        demos: diagnostic_demonstrations(),
        stages,
        query,
        invalid_query: false,
        case_code: code,
    }
}

fn spec_tfa(
    operator: CheckedOperator,
    predicate: Predicate,
    reducer: Reducer,
    query: Vec<i64>,
    code: &'static str,
) -> TaskSpec {
    let stages = vec![
        Stage::Transform(operator),
        Stage::Retain(predicate),
        Stage::Aggregate(reducer),
    ];
    TaskSpec {
        demos: diagnostic_demonstrations(),
        stages,
        query,
        invalid_query: false,
        case_code: code,
    }
}

fn diagnostic_demonstrations() -> Vec<Vec<i64>> {
    vec![
        vec![-9, -4, -1, 0, 2, 7],
        vec![-6, -2, 1, 3, 8],
        vec![-13, -5, 0, 4, 11],
    ]
}

fn build_tasks(
    prefix: &str,
    split: Sem1Split,
    specs: &[TaskSpec],
) -> Result<Vec<EvaluationTask>, String> {
    specs
        .iter()
        .enumerate()
        .map(|(index, spec)| {
            let task_id = format!("{prefix}{:06}", index + 1);
            let demonstrations = spec
                .demos
                .iter()
                .map(|input| {
                    let output = execute_primitive_pipeline(
                        &spec.stages,
                        Value::IntegerSequence(input.clone()),
                    )
                    .map_err(|error| format!("invalid generated demo {task_id}:{error:?}"))?
                    .value;
                    Ok(Demonstration {
                        input: Value::IntegerSequence(input.clone()),
                        output,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let expected_query = if spec.invalid_query {
                ExpectedOutcome::SemanticInvalid
            } else {
                ExpectedOutcome::Value(
                    execute_primitive_pipeline(
                        &spec.stages,
                        Value::IntegerSequence(spec.query.clone()),
                    )
                    .map_err(|error| format!("invalid generated query {task_id}:{error:?}"))?
                    .value,
                )
            };
            let output_type = spec
                .stages
                .last()
                .map(Stage::output_type)
                .unwrap_or(Sem1ValueType::IntegerSequence);
            Ok(EvaluationTask {
                visible: VisibleTask {
                    task_id: task_id.clone(),
                    input_type: Sem1ValueType::IntegerSequence,
                    output_type,
                    demonstrations,
                    query_input: Value::IntegerSequence(spec.query.clone()),
                    capabilities: capabilities_for(&task_id, &spec.stages),
                },
                split,
                expected_query,
                hidden_stage_kinds: spec.stages.iter().map(Stage::kind).collect(),
                hidden_program: spec.stages.clone(),
                hidden_case_code: spec.case_code.to_string(),
            })
        })
        .collect()
}

fn capabilities_for(task_id: &str, stages: &[Stage]) -> Vec<StageCapability> {
    let mut available = Vec::new();
    for (index, stage) in stages.iter().enumerate() {
        available.push(StageCapability::new(
            format!("{task_id}-O{:02}", index + 1),
            stage.clone(),
        ));
    }
    let distractors = [
        Stage::Transform(CheckedOperator::Add(13)),
        Stage::Transform(CheckedOperator::Mul(-3)),
        Stage::Retain(Predicate::Even),
        Stage::Retain(Predicate::NonZero),
        Stage::Aggregate(Reducer::Sum),
        Stage::Aggregate(Reducer::Product),
    ];
    for (index, stage) in distractors.into_iter().enumerate() {
        if !available.iter().any(|capability| capability.stage == stage) {
            available.push(StageCapability::new(
                format!("{task_id}-X{:02}", index + 1),
                stage,
            ));
        }
    }
    available
}

pub fn blind_manifest(tasks: &[EvaluationTask]) -> Result<BlindManifest, String> {
    let mut manifest = BlindManifest {
        generator_version: CURRICULUM_GENERATOR_VERSION.to_string(),
        seed: BLIND_SEED,
        tasks: tasks.iter().map(|task| task.visible.clone()).collect(),
        expected_query_outputs_included: false,
        hidden_generator_metadata_included: false,
        manifest_sha256: String::new(),
    };
    manifest.manifest_sha256 = hash_serializable(&manifest)?;
    Ok(manifest)
}

pub fn curriculum_manifest(
    discovery: &[EvaluationTask],
    calibration: &[EvaluationTask],
    blind: &[EvaluationTask],
) -> Result<CurriculumManifest, String> {
    let mut manifest = CurriculumManifest {
        generator_version: CURRICULUM_GENERATOR_VERSION.to_string(),
        seed: TRAIN_SEED,
        discovery_tasks: discovery.len(),
        calibration_tasks: calibration.len(),
        blind_tasks: blind.len(),
        input_types: vec![Sem1ValueType::IntegerSequence],
        output_types: vec![Sem1ValueType::IntegerSequence, Sem1ValueType::Integer],
        exposed_task_family_metadata: false,
        exposed_human_abstraction_names: false,
        supported_composition_arities: vec![1, 2, 3, 4],
        curriculum_sha256: String::new(),
    };
    manifest.curriculum_sha256 = hash_serializable(&manifest)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::{blind_manifest, generate_curriculum, ExpectedOutcome};

    #[test]
    fn fresh_blind_manifest_excludes_answers_and_family_metadata() {
        let (_, _, blind) = generate_curriculum().expect("curriculum");
        let manifest = blind_manifest(&blind).expect("manifest");
        let text = serde_json::to_string(&manifest).expect("json");
        assert!(!manifest.expected_query_outputs_included);
        assert!(!manifest.hidden_generator_metadata_included);
        assert!(!text.contains("hidden_stage_kinds"));
        assert!(!text.contains("\"expected_query\":"));
        assert!(!text.contains("TRANSFORM"));
        assert!(!text.contains("RETAIN"));
        assert!(!text.contains("AGGREGATE"));
        assert!(blind
            .iter()
            .any(|task| matches!(task.expected_query, ExpectedOutcome::SemanticInvalid)));
        assert_eq!(blind.len(), 20);
    }
}
