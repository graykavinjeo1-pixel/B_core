use gpt_reference_evaluator::{
    sha256_text, surface_similarity_bp, validate_final_reference_draft, BenchmarkInputSuiteIR,
    EvaluationLanguageIR, ReferenceSuiteIR, SuiteSplitIR, INPUT_SUITE_SCHEMA,
    REFERENCE_SUITE_SCHEMA,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const SUITE_ID: &str = "B_CORE_GPT_REFERENCE_V4_FINAL";
const AUTHOR_MANIFEST_SCHEMA: &str = "B_CORE_INDEPENDENT_INPUT_AUTHOR_MANIFEST_1";
const AUDIT_SCHEMA: &str = "B_CORE_INDEPENDENT_FINAL_INPUT_AUDIT_1";
const USAGE: &str = "USAGE: gpt-reference-independent-input-sealer <raw-input.json> <raw-annotation-draft.json> <author-manifest.json> <development-input.json> <v1-final-input.json> <v2-final-input.json> <v3-final-input.json> <sealed-input.json> <sealed-annotation-draft.json> <input-audit.json>";
const COMPARISON_IDENTITIES: [(&str, &str); 4] = [
    (
        "B_CORE_GPT_REFERENCE_V1_DEVELOPMENT",
        "ef2a003c6a7b4aeb1ae3143e1e8c4f0401aa3cce894d7eb953474643425f3f3e",
    ),
    (
        "B_CORE_GPT_REFERENCE_V1_FINAL",
        "c31162100ef2257a538f409fe4cd41a359b42f1244387b7fc1a3f88914f41960",
    ),
    (
        "B_CORE_GPT_REFERENCE_V2_FINAL",
        "ff80de6025b7ef07627642367d098da835f920b75ba34915979be609e0189b5d",
    ),
    (
        "B_CORE_GPT_REFERENCE_V3_FINAL",
        "77e5e6f8836bf02b972227a02a6204f8d6a220af2c65930ecdb1675ba5b6f5aa",
    ),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndependentAuthorManifestIR {
    schema: String,
    campaign_suite_id: String,
    authoring_run_id: String,
    author_model_id: String,
    authoring_date: String,
    raw_input_sha256: String,
    raw_annotation_sha256: String,
    b_core_output_consulted: bool,
    #[serde(default)]
    candidate_artifacts_consulted: Vec<String>,
    manifest_sha256: String,
}

impl IndependentAuthorManifestIR {
    fn validate(&self, raw_input: &str, raw_annotation: &str) -> Result<(), String> {
        if self.schema != AUTHOR_MANIFEST_SCHEMA
            || self.campaign_suite_id != SUITE_ID
            || self.authoring_run_id.trim().is_empty()
            || self.author_model_id.trim().is_empty()
            || self.authoring_date.trim().is_empty()
            || self.b_core_output_consulted
            || !self.candidate_artifacts_consulted.is_empty()
            || self.raw_input_sha256 != sha256_text(raw_input)
            || self.raw_annotation_sha256 != sha256_text(raw_annotation)
        {
            return Err("INDEPENDENT_AUTHOR_PROVENANCE_INVALID_OR_CONTAMINATED".to_string());
        }
        let mut unhashed = self.clone();
        let expected = unhashed.manifest_sha256.clone();
        unhashed.manifest_sha256.clear();
        let payload = serde_json::to_string(&unhashed)
            .map_err(|error| format!("AUTHOR_MANIFEST_HASH_SERIALIZATION_FAILED:{error}"))?;
        if expected != sha256_text(&payload) {
            return Err("INDEPENDENT_AUTHOR_MANIFEST_TAMPERED".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
struct OverlapRowIR {
    response_id: String,
    nearest_suite_id: String,
    nearest_response_id: String,
    similarity_bp: u16,
}

#[derive(Debug, Clone, Serialize)]
struct IndependentInputAuditIR {
    schema: &'static str,
    suite_id: String,
    sealed_input_sha256: String,
    prepared_annotation_sha256: String,
    author_manifest_sha256: String,
    authoring_run_id: String,
    author_model_id: String,
    comparison_input_sha256s: Vec<String>,
    dialogues: usize,
    responses: usize,
    category_response_counts: BTreeMap<String, usize>,
    language_response_counts: BTreeMap<EvaluationLanguageIR, usize>,
    exact_prompt_reuse: usize,
    exact_response_id_reuse: usize,
    exact_dialogue_id_reuse: usize,
    structural_skeleton_reuse: usize,
    mean_nearest_similarity_bp: u16,
    percentile_95_nearest_similarity_bp: u16,
    maximum_nearest_similarity_bp: u16,
    novelty_gate_pass: bool,
    b_core_evaluations: usize,
    candidate_outputs_accepted_as_input: bool,
    overlap_rows: Vec<OverlapRowIR>,
    audit_sha256: String,
}

fn normalized_surface(text: &str) -> String {
    text.split_whitespace()
        .flat_map(|token| token.chars())
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn resource_token(token: &str) -> bool {
    let normalized = token
        .trim_matches(|character: char| !character.is_alphanumeric())
        .to_lowercase();
    [
        "cache",
        "queue",
        "worker",
        "service",
        "log",
        "scheduler",
        "pipeline",
        "gateway",
        "relay",
        "server",
        "index",
        "report",
        "migration",
        "file",
        "document",
        "backup",
        "database",
        "endpoint",
        "캐시",
        "캐시를",
        "큐",
        "큐를",
        "워커",
        "워커를",
        "서비스",
        "서비스를",
        "로그",
        "로그를",
        "스케줄러",
        "스케줄러를",
        "파이프라인",
        "파이프라인을",
        "게이트웨이",
        "게이트웨이를",
        "서버",
        "서버를",
        "인덱스",
        "인덱스를",
        "보고서",
        "보고서를",
        "마이그레이션",
        "마이그레이션을",
        "파일",
        "파일을",
    ]
    .contains(&normalized.as_str())
}

fn structural_skeleton(text: &str) -> String {
    let mut tokens = text
        .split_whitespace()
        .map(|token| token.to_string())
        .collect::<Vec<_>>();
    for index in 1..tokens.len() {
        if resource_token(&tokens[index]) {
            tokens[index - 1] = "ENTITY".to_string();
        }
    }
    normalized_surface(&tokens.join(" "))
}

fn prepare_input(mut input: BenchmarkInputSuiteIR) -> Result<BenchmarkInputSuiteIR, String> {
    if input.schema != INPUT_SUITE_SCHEMA
        || input.suite_id != SUITE_ID
        || input.split != SuiteSplitIR::Final
        || input.frozen
        || !input.suite_payload_sha256.is_empty()
    {
        return Err("RAW_V4_INPUT_STATE_INVALID".to_string());
    }
    if input.turns.iter().any(|turn| {
        !turn.response_id.starts_with("GPTREF-V4-FINAL-")
            || !turn.dialogue_id.starts_with("GPTREF-V4-FINAL-")
    }) {
        return Err("V4_INPUT_ID_NAMESPACE_INVALID".to_string());
    }
    input.frozen = true;
    input.seal()?;
    input.validate()?;
    Ok(input)
}

fn prepare_annotation(
    input: &BenchmarkInputSuiteIR,
    mut draft: ReferenceSuiteIR,
) -> Result<ReferenceSuiteIR, String> {
    if draft.schema != REFERENCE_SUITE_SCHEMA
        || draft.suite_id != SUITE_ID
        || draft.split != SuiteSplitIR::Final
        || draft.frozen
        || !draft.suite_payload_sha256.is_empty()
    {
        return Err("RAW_V4_ANNOTATION_STATE_INVALID".to_string());
    }
    draft.input_suite_sha256 = input.suite_payload_sha256.clone();
    validate_final_reference_draft(input, &draft)?;
    Ok(draft)
}

fn audit_input(
    input: &BenchmarkInputSuiteIR,
    annotation: &ReferenceSuiteIR,
    manifest: &IndependentAuthorManifestIR,
    comparisons: &[BenchmarkInputSuiteIR],
) -> Result<IndependentInputAuditIR, String> {
    let comparison_turns = comparisons
        .iter()
        .flat_map(|suite| {
            suite
                .turns
                .iter()
                .map(move |turn| (suite.suite_id.as_str(), turn))
        })
        .collect::<Vec<_>>();
    let comparison_surfaces = comparison_turns
        .iter()
        .map(|(_, turn)| normalized_surface(&turn.raw_text))
        .collect::<BTreeSet<_>>();
    let comparison_skeletons = comparison_turns
        .iter()
        .map(|(_, turn)| structural_skeleton(&turn.raw_text))
        .collect::<BTreeSet<_>>();
    let comparison_response_ids = comparison_turns
        .iter()
        .map(|(_, turn)| turn.response_id.as_str())
        .collect::<BTreeSet<_>>();
    let comparison_dialogue_ids = comparison_turns
        .iter()
        .map(|(_, turn)| turn.dialogue_id.as_str())
        .collect::<BTreeSet<_>>();

    let mut own_surfaces = BTreeSet::new();
    let mut own_skeletons = BTreeSet::new();
    let mut exact_prompt_reuse = 0;
    let mut exact_response_id_reuse = 0;
    let mut exact_dialogue_id_reuse = 0;
    let mut structural_skeleton_reuse = 0;
    let mut overlap_rows = Vec::new();
    for turn in &input.turns {
        let surface = normalized_surface(&turn.raw_text);
        let skeleton = structural_skeleton(&turn.raw_text);
        if !own_surfaces.insert(surface.clone()) || comparison_surfaces.contains(&surface) {
            exact_prompt_reuse += 1;
        }
        if !own_skeletons.insert(skeleton.clone()) || comparison_skeletons.contains(&skeleton) {
            structural_skeleton_reuse += 1;
        }
        exact_response_id_reuse +=
            usize::from(comparison_response_ids.contains(turn.response_id.as_str()));
        exact_dialogue_id_reuse +=
            usize::from(comparison_dialogue_ids.contains(turn.dialogue_id.as_str()));
        let ((nearest_suite_id, nearest), similarity_bp) = comparison_turns
            .iter()
            .map(|(suite_id, prior)| {
                (
                    (*suite_id, *prior),
                    surface_similarity_bp(&prior.raw_text, &turn.raw_text),
                )
            })
            .max_by_key(|(_, score)| *score)
            .ok_or_else(|| "COMPARISON_INPUTS_EMPTY".to_string())?;
        overlap_rows.push(OverlapRowIR {
            response_id: turn.response_id.clone(),
            nearest_suite_id: nearest_suite_id.to_string(),
            nearest_response_id: nearest.response_id.clone(),
            similarity_bp,
        });
    }
    overlap_rows.sort_by(|left, right| {
        right
            .similarity_bp
            .cmp(&left.similarity_bp)
            .then_with(|| left.response_id.cmp(&right.response_id))
    });
    let mut scores = overlap_rows
        .iter()
        .map(|row| row.similarity_bp)
        .collect::<Vec<_>>();
    scores.sort_unstable();
    let mean = ((scores.iter().map(|score| u64::from(*score)).sum::<u64>()
        + scores.len() as u64 / 2)
        / scores.len() as u64) as u16;
    let percentile_95 = scores[((scores.len() - 1) * 95) / 100];
    let maximum = *scores.last().expect("validated input is non-empty");
    let novelty_gate_pass = exact_prompt_reuse == 0
        && exact_response_id_reuse == 0
        && exact_dialogue_id_reuse == 0
        && structural_skeleton_reuse == 0
        && mean <= 6_000
        && percentile_95 <= 8_500
        && maximum < 10_000;
    if !novelty_gate_pass {
        return Err(format!(
            "V4_INPUT_NOVELTY_GATE_FAILED:EXACT={exact_prompt_reuse}:IDS={exact_response_id_reuse}:DIALOGUES={exact_dialogue_id_reuse}:SKELETONS={structural_skeleton_reuse}:MEAN={mean}:P95={percentile_95}:MAX={maximum}"
        ));
    }

    let mut category_response_counts = BTreeMap::new();
    let mut language_response_counts = BTreeMap::new();
    for turn in &input.turns {
        *category_response_counts
            .entry(turn.category.clone())
            .or_insert(0) += 1;
        *language_response_counts.entry(turn.language).or_insert(0) += 1;
    }
    let annotation_payload = serde_json::to_string(annotation)
        .map_err(|error| format!("ANNOTATION_HASH_SERIALIZATION_FAILED:{error}"))?;
    let mut audit = IndependentInputAuditIR {
        schema: AUDIT_SCHEMA,
        suite_id: input.suite_id.clone(),
        sealed_input_sha256: input.suite_payload_sha256.clone(),
        prepared_annotation_sha256: sha256_text(&annotation_payload),
        author_manifest_sha256: manifest.manifest_sha256.clone(),
        authoring_run_id: manifest.authoring_run_id.clone(),
        author_model_id: manifest.author_model_id.clone(),
        comparison_input_sha256s: comparisons
            .iter()
            .map(|suite| suite.suite_payload_sha256.clone())
            .collect(),
        dialogues: input
            .turns
            .iter()
            .map(|turn| turn.dialogue_id.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        responses: input.turns.len(),
        category_response_counts,
        language_response_counts,
        exact_prompt_reuse,
        exact_response_id_reuse,
        exact_dialogue_id_reuse,
        structural_skeleton_reuse,
        mean_nearest_similarity_bp: mean,
        percentile_95_nearest_similarity_bp: percentile_95,
        maximum_nearest_similarity_bp: maximum,
        novelty_gate_pass,
        b_core_evaluations: 0,
        candidate_outputs_accepted_as_input: false,
        overlap_rows,
        audit_sha256: String::new(),
    };
    let unhashed = serde_json::to_string(&audit)
        .map_err(|error| format!("AUDIT_HASH_SERIALIZATION_FAILED:{error}"))?;
    audit.audit_sha256 = sha256_text(&unhashed);
    Ok(audit)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &str, label: &str) -> Result<T, String> {
    serde_json::from_slice(&fs::read(path).map_err(|error| format!("{label}_READ_FAILED:{error}"))?)
        .map_err(|error| format!("{label}_JSON_INVALID:{error}"))
}

fn workspace_root(start: &Path) -> Result<PathBuf, String> {
    for candidate in start.ancestors() {
        let manifest = candidate.join("Cargo.toml");
        if manifest.is_file()
            && fs::read_to_string(&manifest)
                .map_err(|error| format!("WORKSPACE_MANIFEST_READ_FAILED:{error}"))?
                .contains("[workspace]")
        {
            return candidate
                .canonicalize()
                .map_err(|error| format!("WORKSPACE_CANONICALIZATION_FAILED:{error}"));
        }
    }
    Err("WORKSPACE_ROOT_NOT_FOUND".to_string())
}

fn resolve_new_reports_output(workspace: &Path, output: &Path) -> Result<PathBuf, String> {
    if output.exists() {
        return Err(format!("OUTPUT_ALREADY_EXISTS:{}", output.display()));
    }
    let reports = workspace
        .join("reports")
        .canonicalize()
        .map_err(|error| format!("REPORTS_DIRECTORY_UNAVAILABLE:{error}"))?;
    let parent = output
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()
        .map_err(|error| format!("OUTPUT_PARENT_UNAVAILABLE:{error}"))?;
    if !parent.starts_with(reports) {
        return Err("OUTPUT_MUST_BE_INSIDE_WORKSPACE_REPORTS".to_string());
    }
    let file_name = output
        .file_name()
        .ok_or_else(|| "OUTPUT_FILE_NAME_MISSING".to_string())?;
    let resolved = parent.join(file_name);
    if resolved.exists() {
        return Err(format!("OUTPUT_ALREADY_EXISTS:{}", resolved.display()));
    }
    Ok(resolved)
}

fn write_new(path: &Path, payload: &str, label: &str) -> Result<(), String> {
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("{label}_CREATE_FAILED:{error}"))?;
    output
        .write_all(payload.as_bytes())
        .map_err(|error| format!("{label}_WRITE_FAILED:{error}"))
}

fn validate_comparison_identities(comparisons: &[BenchmarkInputSuiteIR]) -> Result<(), String> {
    if comparisons.len() != COMPARISON_IDENTITIES.len() {
        return Err("COMPARISON_SUITE_COUNT_INVALID".to_string());
    }
    for (comparison, (expected_id, expected_hash)) in comparisons.iter().zip(COMPARISON_IDENTITIES)
    {
        if comparison.suite_id != expected_id || comparison.suite_payload_sha256 != expected_hash {
            return Err(format!(
                "COMPARISON_SUITE_IDENTITY_INVALID:{}",
                comparison.suite_id
            ));
        }
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 10 {
        return Err(USAGE.to_string());
    }
    let raw_input_text = fs::read_to_string(&arguments[0])
        .map_err(|error| format!("RAW_INPUT_READ_FAILED:{error}"))?;
    let raw_annotation_text = fs::read_to_string(&arguments[1])
        .map_err(|error| format!("RAW_ANNOTATION_READ_FAILED:{error}"))?;
    let manifest: IndependentAuthorManifestIR = read_json(&arguments[2], "AUTHOR_MANIFEST")?;
    manifest.validate(&raw_input_text, &raw_annotation_text)?;
    let raw_input: BenchmarkInputSuiteIR = serde_json::from_str(&raw_input_text)
        .map_err(|error| format!("RAW_INPUT_JSON_INVALID:{error}"))?;
    let raw_annotation: ReferenceSuiteIR = serde_json::from_str(&raw_annotation_text)
        .map_err(|error| format!("RAW_ANNOTATION_JSON_INVALID:{error}"))?;
    let input = prepare_input(raw_input)?;
    let annotation = prepare_annotation(&input, raw_annotation)?;
    let comparisons = arguments[3..7]
        .iter()
        .map(|path| read_json::<BenchmarkInputSuiteIR>(path, "COMPARISON_INPUT"))
        .collect::<Result<Vec<_>, _>>()?;
    if comparisons.first().map(|suite| suite.split) != Some(SuiteSplitIR::Development)
        || comparisons
            .iter()
            .skip(1)
            .any(|suite| suite.split != SuiteSplitIR::Final)
    {
        return Err("COMPARISON_SUITE_ORDER_INVALID".to_string());
    }
    for comparison in &comparisons {
        comparison.validate()?;
    }
    validate_comparison_identities(&comparisons)?;
    let audit = audit_input(&input, &annotation, &manifest, &comparisons)?;
    let workspace = workspace_root(
        &env::current_dir().map_err(|error| format!("CURRENT_DIRECTORY_UNAVAILABLE:{error}"))?,
    )?;
    let requested_output_paths = arguments[7..10]
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let output_paths = requested_output_paths
        .iter()
        .map(|output| resolve_new_reports_output(&workspace, output))
        .collect::<Result<Vec<_>, _>>()?;
    if output_paths.iter().collect::<BTreeSet<_>>().len() != output_paths.len() {
        return Err("OUTPUT_PATHS_MUST_BE_DISTINCT".to_string());
    }
    let input_payload = serde_json::to_string_pretty(&input)
        .map_err(|error| format!("SEALED_INPUT_SERIALIZATION_FAILED:{error}"))?;
    let annotation_payload = serde_json::to_string_pretty(&annotation)
        .map_err(|error| format!("ANNOTATION_SERIALIZATION_FAILED:{error}"))?;
    let audit_payload = serde_json::to_string_pretty(&audit)
        .map_err(|error| format!("AUDIT_SERIALIZATION_FAILED:{error}"))?;
    write_new(
        &output_paths[0],
        &format!("{input_payload}\n"),
        "SEALED_INPUT",
    )?;
    write_new(
        &output_paths[1],
        &format!("{annotation_payload}\n"),
        "ANNOTATION",
    )?;
    write_new(&output_paths[2], &format!("{audit_payload}\n"), "AUDIT")?;
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_rejects_entity_only_prompt_renames() {
        assert_eq!(
            structural_skeleton("Inspect the Aster cache now."),
            structural_skeleton("Inspect the Birch cache now.")
        );
        assert_eq!(
            structural_skeleton("Aster 캐시를 지금 확인해."),
            structural_skeleton("Birch 캐시를 지금 확인해.")
        );
    }

    #[test]
    fn skeleton_preserves_construction_changes() {
        assert_ne!(
            structural_skeleton("Inspect the Aster cache now."),
            structural_skeleton("Before changing anything, explain what the Birch cache shows.")
        );
    }

    #[test]
    fn comparison_identities_are_pinned() {
        let mut comparison = BenchmarkInputSuiteIR {
            schema: INPUT_SUITE_SCHEMA.to_string(),
            suite_id: COMPARISON_IDENTITIES[0].0.to_string(),
            split: SuiteSplitIR::Development,
            frozen: true,
            turns: Vec::new(),
            suite_payload_sha256: COMPARISON_IDENTITIES[0].1.to_string(),
        };
        let comparisons = std::array::from_fn::<_, 4, _>(|index| {
            comparison.suite_id = COMPARISON_IDENTITIES[index].0.to_string();
            comparison.suite_payload_sha256 = COMPARISON_IDENTITIES[index].1.to_string();
            comparison.clone()
        });
        assert!(validate_comparison_identities(&comparisons).is_ok());
        let mut replaced = comparisons;
        replaced[3].suite_payload_sha256 = "0".repeat(64);
        assert_eq!(
            validate_comparison_identities(&replaced),
            Err("COMPARISON_SUITE_IDENTITY_INVALID:B_CORE_GPT_REFERENCE_V3_FINAL".to_string())
        );
    }
}
