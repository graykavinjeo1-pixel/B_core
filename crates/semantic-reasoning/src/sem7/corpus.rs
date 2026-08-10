use std::collections::BTreeMap;

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{
    lexical::canonical_request,
    model::{
        GroundingDomain, Language, LanguageEvaluatorTask, LanguageTaskCategory,
        LanguageTaskManifest, MeaningRequestIR, SemanticOperation, VisibleLanguageTask,
    },
};

pub const LANGUAGE_GENERATOR_VERSION: &str = "SEM7-LANGUAGE-GENERATOR-2.0.0";
pub const BLIND_TASK_COUNT: usize = 100;

#[derive(Debug, Clone)]
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state
    }

    fn tag(&mut self) -> String {
        format!("{:08x}", self.next() as u32)
    }
}

pub fn generate_language_tasks(seed: u64) -> Vec<LanguageEvaluatorTask> {
    let mut rng = Rng::new(seed);
    (0..BLIND_TASK_COUNT)
        .map(|index| build_task(index, &mut rng))
        .collect()
}

fn build_task(index: usize, rng: &mut Rng) -> LanguageEvaluatorTask {
    let (category, within) = match index {
        0..=19 => (LanguageTaskCategory::KoreanGrounding, index),
        20..=39 => (LanguageTaskCategory::EnglishGrounding, index - 20),
        40..=49 => (LanguageTaskCategory::ParaphraseSynonym, index - 40),
        50..=59 => (LanguageTaskCategory::AmbiguityReference, index - 50),
        60..=69 => (LanguageTaskCategory::OpaqueRelexicalization, index - 60),
        70..=79 => (LanguageTaskCategory::LanguageToForaging, index - 70),
        80..=89 => (LanguageTaskCategory::LanguageToProgram, index - 80),
        _ => (LanguageTaskCategory::LanguageToMath, index - 90),
    };
    let language = match category {
        LanguageTaskCategory::KoreanGrounding => Language::Korean,
        LanguageTaskCategory::EnglishGrounding => Language::English,
        _ if within % 2 == 0 => Language::Korean,
        _ => Language::English,
    };
    let mut parts = match category {
        LanguageTaskCategory::KoreanGrounding | LanguageTaskCategory::EnglishGrounding => {
            grounding_task(language, within)
        }
        LanguageTaskCategory::ParaphraseSynonym => paraphrase_task(language, within),
        LanguageTaskCategory::AmbiguityReference => ambiguity_reference_task(language, within),
        LanguageTaskCategory::OpaqueRelexicalization => opaque_task(language, within, rng),
        LanguageTaskCategory::LanguageToForaging => foraging_task(language, within, rng),
        LanguageTaskCategory::LanguageToProgram => {
            operation_task(language, GroundingDomain::Programming, within, within % 5)
        }
        LanguageTaskCategory::LanguageToMath => operation_task(
            language,
            GroundingDomain::Mathematics,
            within,
            5 + within % 5,
        ),
    };
    let task_id = format!("SEM7-L-{index:03}-{}", rng.tag());
    let active_text_sha256 = hash_bytes(parts.text.as_bytes());
    let visible = VisibleLanguageTask {
        task_id,
        category,
        language,
        domain: parts.domain,
        text: parts.text,
        context: parts.context,
        paraphrases: parts.paraphrases,
        near_contrast: parts.near_contrast.take(),
        introduced_alias: parts.introduced_alias,
        definition: parts.definition,
        definition_language: parts.definition_language,
        target_language: language,
        lookup_only: parts.lookup_only,
        active_text_sha256,
        answers_included: false,
        expected_goal_ir_included: false,
        target_program_included: false,
        frozen: true,
    };
    let hidden_inputs = hidden_inputs(parts.expected.operation, within);
    LanguageEvaluatorTask {
        visible,
        expected: parts.expected,
        near_contrast_expected: parts.near_contrast_expected,
        hidden_inputs,
        requires_composition: parts.requires_composition,
        requires_semantic_disambiguation: parts.requires_semantic_disambiguation,
        requires_alias_consolidation: parts.requires_alias_consolidation,
    }
}

struct TaskParts {
    domain: GroundingDomain,
    text: String,
    context: String,
    paraphrases: Vec<String>,
    near_contrast: Option<String>,
    introduced_alias: Option<String>,
    definition: Option<String>,
    definition_language: Option<Language>,
    lookup_only: bool,
    expected: MeaningRequestIR,
    near_contrast_expected: Option<MeaningRequestIR>,
    requires_composition: bool,
    requires_semantic_disambiguation: bool,
    requires_alias_consolidation: bool,
}

fn grounding_task(language: Language, within: usize) -> TaskParts {
    if within < 5 {
        operation_task(language, GroundingDomain::Programming, within, within)
    } else if within < 10 {
        operation_task(language, GroundingDomain::Mathematics, within, within)
    } else {
        identify_task(language, within)
    }
}

fn operation_task(
    language: Language,
    domain: GroundingDomain,
    within: usize,
    variant: usize,
) -> TaskParts {
    let parameter = 2 + (within % 5) as i64;
    let (operation, text, paraphrase) = match (language, variant % 10) {
        (Language::Korean, 0) => (
            SemanticOperation::AddEach,
            format!("각 값에 {parameter}을 더해"),
            format!("모든 값에 {parameter}을 더해"),
        ),
        (Language::Korean, 1) => (
            SemanticOperation::MultiplyEach,
            format!("각 값에 {parameter}을 곱해"),
            format!("모든 값을 {parameter}배로 만들어"),
        ),
        (Language::Korean, 2) => (
            SemanticOperation::FilterGreater,
            format!("{parameter}보다 큰 값만 남겨"),
            format!("{parameter} 초과 값만 선택해"),
        ),
        (Language::Korean, 3) => (
            SemanticOperation::Sum,
            "모든 값의 합계를 구해".to_string(),
            "값을 모두 합해".to_string(),
        ),
        (Language::Korean, 4) => (
            SemanticOperation::FilterNotGreater,
            format!("{parameter}을 초과하지 않는 값만 남겨"),
            format!("{parameter}보다 크지 않은 값만 선택해"),
        ),
        (Language::Korean, 5) => (
            SemanticOperation::RecurrenceStep,
            format!("점화식을 {parameter}만큼 진행해"),
            format!("점화 관계의 다음 상태에 {parameter}을 적용해"),
        ),
        (Language::Korean, 6) => (
            SemanticOperation::CountGreater,
            format!("{parameter}보다 큰 값의 개수를 세어"),
            format!("{parameter} 초과 값의 개수를 구해"),
        ),
        (Language::Korean, 7) => (
            SemanticOperation::StatusClass,
            "응답 상태의 상태 등급을 구해".to_string(),
            "HTTP 상태의 응답 상태를 분류해".to_string(),
        ),
        (Language::Korean, 8) => (
            SemanticOperation::ScopedLookup,
            "버전 관계의 범위 계약을 조회해".to_string(),
            "범위 계약에서 버전 관계를 찾아".to_string(),
        ),
        (Language::Korean, _) => (
            SemanticOperation::AddEach,
            format!("각 값에 {parameter}을 더해"),
            format!("값마다 {parameter}을 증가시켜"),
        ),
        (Language::English, 0) => (
            SemanticOperation::AddEach,
            format!("add {parameter} to every value"),
            format!("increase every value by {parameter}"),
        ),
        (Language::English, 1) => (
            SemanticOperation::MultiplyEach,
            format!("multiply every value by {parameter}"),
            format!("scale every value by {parameter}"),
        ),
        (Language::English, 2) => (
            SemanticOperation::FilterGreater,
            format!("keep values greater than {parameter}"),
            format!("retain values above {parameter}"),
        ),
        (Language::English, 3) => (
            SemanticOperation::Sum,
            "sum all values".to_string(),
            "compute the total of the values".to_string(),
        ),
        (Language::English, 4) => (
            SemanticOperation::FilterNotGreater,
            format!("exclude values greater than {parameter}"),
            format!("keep values not greater than {parameter}"),
        ),
        (Language::English, 5) => (
            SemanticOperation::RecurrenceStep,
            format!("advance the recurrence by {parameter}"),
            format!("advance the recurrence relation by {parameter}"),
        ),
        (Language::English, 6) => (
            SemanticOperation::CountGreater,
            format!("count values greater than {parameter}"),
            format!("count values above {parameter}"),
        ),
        (Language::English, 7) => (
            SemanticOperation::StatusClass,
            "classify the response class".to_string(),
            "find the HTTP status class".to_string(),
        ),
        (Language::English, 8) => (
            SemanticOperation::ScopedLookup,
            "look up the versioned relation in the scoped contract".to_string(),
            "query the scoped contract for the versioned relation".to_string(),
        ),
        (Language::English, _) => (
            SemanticOperation::AddEach,
            format!("add {parameter} to every value"),
            format!("increase each value by {parameter}"),
        ),
        (Language::Opaque, _) => unreachable!(),
    };
    let expected_parameter = match operation {
        SemanticOperation::AddEach
        | SemanticOperation::MultiplyEach
        | SemanticOperation::FilterGreater
        | SemanticOperation::FilterNotGreater
        | SemanticOperation::CountGreater
        | SemanticOperation::RecurrenceStep => Some(parameter),
        _ => None,
    };
    TaskParts {
        domain,
        text,
        context: match domain {
            GroundingDomain::Programming => "bounded sequence program".to_string(),
            GroundingDomain::Mathematics => "exact mathematical derivation".to_string(),
            _ => "controlled semantic request".to_string(),
        },
        paraphrases: vec![paraphrase],
        near_contrast: None,
        introduced_alias: None,
        definition: None,
        definition_language: None,
        lookup_only: false,
        expected: canonical_request(operation, expected_parameter, false, None, None),
        near_contrast_expected: None,
        requires_composition: true,
        requires_semantic_disambiguation: false,
        requires_alias_consolidation: false,
    }
}

fn identify_task(language: Language, within: usize) -> TaskParts {
    let concepts = [
        "C000008", "C000009", "C000010", "C000006", "C000011", "C000012",
    ];
    let concept_id = concepts[within % concepts.len()];
    let surface = match (language, concept_id) {
        (Language::Korean, "C000008") => "경계 순회",
        (Language::Korean, "C000009") => "상태 누적",
        (Language::Korean, "C000010") => "단계 합성",
        (Language::Korean, "C000006") => "점화 관계",
        (Language::Korean, "C000011") => "범위 계약",
        (Language::Korean, _) => "상태 등급",
        (Language::English, "C000008") => "guarded traversal",
        (Language::English, "C000009") => "guarded state transition",
        (Language::English, "C000010") => "staged composition",
        (Language::English, "C000006") => "recurrence relation",
        (Language::English, "C000011") => "scoped contract",
        (Language::English, _) => "status class",
        (Language::Opaque, _) => unreachable!(),
    };
    let text = if language == Language::Korean {
        format!("{surface} 개념을 식별해")
    } else {
        format!("identify {surface}")
    };
    let mut expected = canonical_request(SemanticOperation::Identify, None, false, None, None);
    expected.target_concept_id = concept_id.to_string();
    expected.requested_relations = vec![concept_id.to_string()];
    TaskParts {
        domain: GroundingDomain::PriorSemantic,
        text,
        context: "single semantic concept lookup".to_string(),
        paraphrases: Vec::new(),
        near_contrast: None,
        introduced_alias: None,
        definition: None,
        definition_language: None,
        lookup_only: true,
        expected,
        near_contrast_expected: None,
        requires_composition: false,
        requires_semantic_disambiguation: false,
        requires_alias_consolidation: false,
    }
}

fn paraphrase_task(language: Language, within: usize) -> TaskParts {
    let mut task = operation_task(language, GroundingDomain::PriorSemantic, within, within % 5);
    let parameter = 2 + (within % 5) as i64;
    let contrast_parameter = parameter + 1;
    task.near_contrast = Some(if language == Language::Korean {
        match task.expected.operation {
            SemanticOperation::AddEach => format!("각 값에 {contrast_parameter}을 더해"),
            SemanticOperation::MultiplyEach => format!("각 값에 {contrast_parameter}을 곱해"),
            SemanticOperation::FilterGreater => format!("{contrast_parameter}보다 큰 값만 남겨"),
            SemanticOperation::FilterNotGreater => {
                format!("{contrast_parameter}보다 큰 값은 제외해")
            }
            _ => format!("각 값에 {contrast_parameter}을 더해"),
        }
    } else {
        match task.expected.operation {
            SemanticOperation::AddEach => format!("add {contrast_parameter} to every value"),
            SemanticOperation::MultiplyEach => {
                format!("multiply every value by {contrast_parameter}")
            }
            SemanticOperation::FilterGreater => {
                format!("keep values greater than {contrast_parameter}")
            }
            SemanticOperation::FilterNotGreater => {
                format!("exclude values greater than {contrast_parameter}")
            }
            _ => format!("add {contrast_parameter} to every value"),
        }
    });
    if task.expected.parameter.is_some() {
        task.near_contrast_expected = Some(canonical_request(
            task.expected.operation,
            Some(contrast_parameter),
            false,
            None,
            None,
        ));
    }
    task
}

fn ambiguity_reference_task(language: Language, within: usize) -> TaskParts {
    if within < 6 {
        let (text, context, concept, operation) = if language == Language::English {
            if within % 4 == 1 {
                (
                    "bank",
                    "protocol definition",
                    "C000011",
                    SemanticOperation::ScopedLookup,
                )
            } else {
                (
                    "bank all values",
                    "stateful numeric accumulation",
                    "C000009",
                    SemanticOperation::Sum,
                )
            }
        } else if within.is_multiple_of(4) {
            (
                "차",
                "수학 점화식",
                "C000006",
                SemanticOperation::RecurrenceStep,
            )
        } else {
            (
                "차",
                "순서 단계",
                "C000010",
                SemanticOperation::RecurrenceStep,
            )
        };
        let mut expected = canonical_request(operation, None, false, None, None);
        expected.target_concept_id = concept.to_string();
        TaskParts {
            domain: GroundingDomain::PriorSemantic,
            text: text.to_string(),
            context: context.to_string(),
            paraphrases: Vec::new(),
            near_contrast: None,
            introduced_alias: None,
            definition: None,
            definition_language: None,
            lookup_only: false,
            expected,
            near_contrast_expected: None,
            requires_composition: true,
            requires_semantic_disambiguation: true,
            requires_alias_consolidation: false,
        }
    } else {
        let parameter = 2 + (within % 4) as i64;
        let (text, mut expected) = if language == Language::English {
            (
                format!("read values, transform it by adding {parameter}, and save it"),
                canonical_request(
                    SemanticOperation::AddEach,
                    Some(parameter),
                    true,
                    None,
                    None,
                ),
            )
        } else {
            let mut expected = canonical_request(
                SemanticOperation::AddEach,
                Some(parameter),
                true,
                None,
                None,
            );
            expected.reference_bindings = BTreeMap::from([(
                "OMITTED_OBJECT".to_string(),
                "transformed_values".to_string(),
            )]);
            (
                format!("값을 읽고 {parameter}을 더해 변환한 뒤 저장해"),
                expected,
            )
        };
        expected.target_concept_id = "C000010".to_string();
        TaskParts {
            domain: GroundingDomain::PriorSemantic,
            text,
            context: "ordered file-like sequence pipeline with one semantic object".to_string(),
            paraphrases: Vec::new(),
            near_contrast: None,
            introduced_alias: None,
            definition: None,
            definition_language: None,
            lookup_only: false,
            expected,
            near_contrast_expected: None,
            requires_composition: true,
            requires_semantic_disambiguation: true,
            requires_alias_consolidation: false,
        }
    }
}

fn opaque_task(language: Language, within: usize, rng: &mut Rng) -> TaskParts {
    let alias = if within == 0 {
        "무루".to_string()
    } else {
        format!(
            "{}{}",
            if language == Language::Korean {
                "누"
            } else {
                "zu"
            },
            rng.tag()
        )
    };
    let parameter = 2 + (within % 5) as i64;
    let definition = if language == Language::Korean {
        "모든 정수에 같은 값을 더하는 연산".to_string()
    } else {
        "an operation that can add to every integer".to_string()
    };
    TaskParts {
        domain: GroundingDomain::PriorSemantic,
        text: format!("{parameter} {alias}"),
        context: "new alias defined before the request".to_string(),
        paraphrases: Vec::new(),
        near_contrast: None,
        introduced_alias: Some(alias),
        definition: Some(definition),
        definition_language: Some(language),
        lookup_only: false,
        expected: canonical_request(
            SemanticOperation::AddEach,
            Some(parameter),
            false,
            None,
            None,
        ),
        near_contrast_expected: None,
        requires_composition: true,
        requires_semantic_disambiguation: true,
        requires_alias_consolidation: true,
    }
}

fn foraging_task(language: Language, _within: usize, rng: &mut Rng) -> TaskParts {
    let alias = format!(
        "{}{}",
        if language == Language::Korean {
            "새등급"
        } else {
            "newclass"
        },
        rng.tag()
    );
    let definition = if language == Language::Korean {
        "HTTP 응답 상태 코드를 상태 등급으로 분류한다".to_string()
    } else {
        "classifies an HTTP response status into a status class".to_string()
    };
    TaskParts {
        domain: GroundingDomain::ExternalForaged,
        text: alias.clone(),
        context: "definition-only SEM-6 replay; active solution unavailable".to_string(),
        paraphrases: Vec::new(),
        near_contrast: None,
        introduced_alias: Some(alias),
        definition: Some(definition),
        definition_language: Some(language),
        lookup_only: false,
        expected: canonical_request(SemanticOperation::StatusClass, None, false, None, None),
        near_contrast_expected: None,
        requires_composition: true,
        requires_semantic_disambiguation: true,
        requires_alias_consolidation: true,
    }
}

fn hidden_inputs(operation: SemanticOperation, within: usize) -> Vec<Vec<i64>> {
    if operation == SemanticOperation::StatusClass {
        vec![vec![100], vec![204], vec![404], vec![599]]
    } else {
        vec![
            vec![1, 2, 3, 4],
            vec![-2, 0, 2, 6],
            vec![100 + (within as i64 % 5) * 100],
            vec![5, 5, 1, 9],
        ]
    }
}

pub fn build_manifest(
    run_id: &str,
    seed: u64,
    tasks: &[LanguageEvaluatorTask],
) -> LanguageTaskManifest {
    let visible = tasks
        .iter()
        .map(|task| task.visible.clone())
        .collect::<Vec<_>>();
    #[derive(Serialize)]
    struct Commitment<'a> {
        run_id: &'a str,
        generator_version: &'a str,
        seed_commitment_sha256: String,
        tasks: &'a [VisibleLanguageTask],
        expected_goal_ir_included: bool,
        hidden_inputs_included: bool,
        target_answers_included: bool,
        target_programs_included: bool,
        frozen_before_evaluation: bool,
    }
    let seed_hash = hash_bytes(&seed.to_le_bytes());
    let commitment = Commitment {
        run_id,
        generator_version: LANGUAGE_GENERATOR_VERSION,
        seed_commitment_sha256: seed_hash.clone(),
        tasks: &visible,
        expected_goal_ir_included: false,
        hidden_inputs_included: false,
        target_answers_included: false,
        target_programs_included: false,
        frozen_before_evaluation: true,
    };
    let manifest_sha256 = hash_bytes(&serde_json::to_vec(&commitment).expect("commitment"));
    drop(commitment);
    LanguageTaskManifest {
        run_id: run_id.to_string(),
        generator_version: LANGUAGE_GENERATOR_VERSION.to_string(),
        seed_commitment_sha256: seed_hash,
        tasks: visible,
        expected_goal_ir_included: false,
        hidden_inputs_included: false,
        target_answers_included: false,
        target_programs_included: false,
        frozen_before_evaluation: true,
        manifest_sha256,
    }
}

pub fn category_counts(tasks: &[LanguageEvaluatorTask]) -> BTreeMap<LanguageTaskCategory, usize> {
    tasks.iter().fold(BTreeMap::new(), |mut counts, task| {
        *counts.entry(task.visible.category).or_insert(0) += 1;
        counts
    })
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_benchmark_is_compact_balanced_and_answer_free() {
        let tasks = generate_language_tasks(71);
        assert_eq!(tasks.len(), BLIND_TASK_COUNT);
        assert_eq!(category_counts(&tasks).values().sum::<usize>(), 100);
        assert_eq!(
            category_counts(&tasks)[&LanguageTaskCategory::KoreanGrounding],
            20
        );
        assert_eq!(
            category_counts(&tasks)[&LanguageTaskCategory::EnglishGrounding],
            20
        );
        assert!(category_counts(&tasks)
            .iter()
            .filter(|(category, _)| {
                !matches!(
                    category,
                    LanguageTaskCategory::KoreanGrounding | LanguageTaskCategory::EnglishGrounding
                )
            })
            .all(|(_, count)| *count == 10));
        let korean = tasks
            .iter()
            .filter(|task| task.visible.language == Language::Korean)
            .count();
        let english = tasks
            .iter()
            .filter(|task| task.visible.language == Language::English)
            .count();
        assert_eq!((korean, english), (50, 50));
        assert_eq!(
            tasks
                .iter()
                .filter(|task| task.visible.domain == GroundingDomain::Programming)
                .count(),
            20
        );
        assert_eq!(
            tasks
                .iter()
                .filter(|task| task.visible.domain == GroundingDomain::Mathematics)
                .count(),
            20
        );
        assert_eq!(
            tasks
                .iter()
                .filter(|task| task.visible.domain == GroundingDomain::ExternalForaged)
                .count(),
            10
        );
        let manifest = build_manifest("test", 71, &tasks);
        let encoded = serde_json::to_string(&manifest).expect("json");
        assert!(!encoded.contains("expected_goal_ir\":{"));
        assert!(manifest.tasks.iter().all(|task| !task.answers_included));
    }
}
