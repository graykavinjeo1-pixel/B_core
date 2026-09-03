use gpt_reference_evaluator::{
    sha256_text, surface_similarity_bp, validate_final_reference_draft, BenchmarkInputSuiteIR,
    BenchmarkInputTurnIR, EvaluationLanguageIR, ReferenceSuiteIR, ReferenceTurnAnnotationIR,
    SuiteSplitIR, CATEGORIES, INPUT_SUITE_SCHEMA, REFERENCE_SUITE_SCHEMA,
};
use semantic_core_adapters::ConversationInputModalityIR;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const V1_SUITE_ID: &str = "B_CORE_GPT_REFERENCE_V1_FINAL";
const V2_SUITE_ID: &str = "B_CORE_GPT_REFERENCE_V2_FINAL";
const V3_SUITE_ID: &str = "B_CORE_GPT_REFERENCE_V3_FINAL";
const PENDING_REFERENCE_MODEL: &str = "PENDING_THREE_INDEPENDENT_GPT_RUNS";
const FINAL_SYSTEM_PROMPT: &str = "Answer the latest user turn naturally and concisely in the dialogue language. Preserve intent, topic, reference, uncertainty, attribution, correction, and plan/execution/result boundaries. Do not claim external execution without verified evidence.";
const FINAL_GENERATION_CONFIGURATION: &str = "three independent fixed-model runs; identical prompt and decoding configuration; no B_Core output consulted; one response for every frozen turn";
const VERIFIED_PROPOSITION: &str = "CLAIM:VERIFIED_EXECUTION:VERIFIED_OBSERVED";
const USAGE: &str = "USAGE:\n  V1: gpt-reference-final-author <development-input.json> <final-input.json> <final-annotation-draft.json> <final-input-audit.json>\n  V2: gpt-reference-final-author <development-input.json> <v1-final-input.json> <v2-final-input.json> <v2-annotation-draft.json> <v2-input-audit.json>\n  V3: gpt-reference-final-author <development-input.json> <v1-final-input.json> <v2-final-input.json> <v3-final-input.json> <v3-annotation-draft.json> <v3-input-audit.json>";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Campaign {
    V1,
    V2,
    V3,
}

#[derive(Serialize)]
struct PromptOverlapRowIR {
    final_response_id: String,
    nearest_development_response_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    nearest_comparison_suite_id: Option<String>,
    similarity_bp: u16,
}

#[derive(Serialize)]
struct FinalInputAuditIR {
    schema: &'static str,
    suite_id: String,
    final_input_sha256: String,
    development_input_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    prior_final_input_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    prior_final_input_sha256s: Vec<String>,
    dialogues: usize,
    responses: usize,
    category_response_counts: BTreeMap<String, usize>,
    language_response_counts: BTreeMap<EvaluationLanguageIR, usize>,
    duplicate_final_prompts: usize,
    exact_development_prompt_reuse: usize,
    mean_nearest_development_similarity_bp: u16,
    percentile_95_nearest_development_similarity_bp: u16,
    maximum_nearest_development_similarity_bp: u16,
    overlap_rows: Vec<PromptOverlapRowIR>,
    b_core_evaluations: usize,
    external_llm_calls: usize,
    audit_sha256: String,
}

#[derive(Clone)]
struct ResourcePair {
    first: String,
    first_concept: &'static str,
    second: String,
    second_concept: &'static str,
}

#[derive(Clone)]
struct TurnSpec {
    input: String,
    response_act: &'static str,
    response_goal: &'static str,
    epistemic_status: &'static str,
    meaning_atoms: Vec<String>,
    discourse_bindings: Vec<String>,
    required_propositions: Vec<String>,
    prohibited_propositions: Vec<String>,
    critical_boundary: bool,
    ambiguity_requires_clarification: bool,
    modality: ConversationInputModalityIR,
}

impl TurnSpec {
    fn interaction(input: String, response_act: &'static str) -> Self {
        Self {
            input,
            response_act,
            response_goal: "ACKNOWLEDGE",
            epistemic_status: "INTERACTION",
            meaning_atoms: vec![format!("RESPONSE_ACT:{response_act}")],
            discourse_bindings: Vec::new(),
            required_propositions: vec!["CLAIM:INTERACTION_STATE:INTERACTION".to_string()],
            prohibited_propositions: vec![VERIFIED_PROPOSITION.to_string()],
            critical_boundary: false,
            ambiguity_requires_clarification: false,
            modality: ConversationInputModalityIR::Text,
        }
    }

    fn plan(
        input: String,
        predicate: &'static str,
        intent: &'static str,
        concept: &'static str,
    ) -> Self {
        Self {
            input,
            response_act: "PLAN_PREVIEW",
            response_goal: "PLAN_ACTIONS",
            epistemic_status: "PLANNED",
            meaning_atoms: action_atoms("PLAN_PREVIEW", predicate, intent, concept),
            discourse_bindings: Vec::new(),
            required_propositions: vec!["CLAIM:PLAN_STATUS:PLANNED".to_string()],
            prohibited_propositions: vec![VERIFIED_PROPOSITION.to_string()],
            critical_boundary: true,
            ambiguity_requires_clarification: false,
            modality: ConversationInputModalityIR::Text,
        }
    }

    fn clarification(input: String) -> Self {
        Self {
            input,
            response_act: "CLARIFICATION_REQUEST",
            response_goal: "ASK_CLARIFICATION",
            epistemic_status: "INTERACTION",
            meaning_atoms: vec!["RESPONSE_ACT:CLARIFICATION_REQUEST".to_string()],
            discourse_bindings: Vec::new(),
            required_propositions: vec!["CLAIM:INTERACTION_STATE:INTERACTION".to_string()],
            prohibited_propositions: vec![VERIFIED_PROPOSITION.to_string()],
            critical_boundary: true,
            ambiguity_requires_clarification: true,
            modality: ConversationInputModalityIR::Text,
        }
    }

    fn result_absence(input: String) -> Self {
        Self {
            input,
            response_act: "RESULT_ABSENCE",
            response_goal: "ANSWER_VERIFIED_RESULT",
            epistemic_status: "UNKNOWN",
            meaning_atoms: vec!["RESPONSE_ACT:RESULT_ABSENCE".to_string()],
            discourse_bindings: Vec::new(),
            required_propositions: vec!["CLAIM:EVIDENCE_ABSENCE:UNKNOWN".to_string()],
            prohibited_propositions: vec![VERIFIED_PROPOSITION.to_string()],
            critical_boundary: true,
            ambiguity_requires_clarification: false,
            modality: ConversationInputModalityIR::Text,
        }
    }

    fn with_action(
        mut self,
        predicate: &'static str,
        intent: &'static str,
        concept: &'static str,
    ) -> Self {
        self.meaning_atoms
            .extend(action_atoms(self.response_act, predicate, intent, concept));
        self
    }

    fn with_binding(mut self, family: &'static str, target: String) -> Self {
        self.discourse_bindings
            .push(format!("REFERENCE:{family}:{target}"));
        self
    }

    fn with_goal(mut self, goal: &'static str) -> Self {
        self.response_goal = goal;
        self
    }

    fn with_epistemic(mut self, status: &'static str) -> Self {
        self.epistemic_status = status;
        self
    }

    fn with_propositions(mut self, propositions: &[&str]) -> Self {
        self.required_propositions = propositions
            .iter()
            .map(|value| (*value).to_string())
            .collect();
        self
    }

    fn with_modality(mut self, modality: ConversationInputModalityIR) -> Self {
        self.modality = modality;
        self
    }
}

fn action_atoms(response_act: &str, predicate: &str, intent: &str, concept: &str) -> Vec<String> {
    vec![
        format!("RESPONSE_ACT:{response_act}"),
        format!("ENTITY:{concept}"),
        format!("EVENT:{predicate}:{intent}:LIVE"),
        format!("THEME:{predicate}:ENTITY:{concept}"),
        format!("GOAL:{predicate}:{intent}"),
        format!("GOAL_SUBJECT:{predicate}:{concept}"),
    ]
}

fn resource_pair(
    campaign: Campaign,
    category_index: usize,
    language: EvaluationLanguageIR,
    variant: usize,
) -> ResourcePair {
    const V1_NAMES: [&str; 40] = [
        "Aurora", "Boreal", "Cascade", "Denali", "Equinox", "Fable", "Glimmer", "Helix", "Ion",
        "Jolt", "Keystone", "Lagoon", "Meridian", "Nova", "Orbit", "Prism", "Quill", "Ripple",
        "Solace", "Tundra", "Uplink", "Vesper", "Warden", "Xylem", "Yield", "Zephyr", "Apricot",
        "Brook", "Comet", "Dover", "Eon", "Fathom", "Glacier", "Hallow", "Inlet", "Jubilee",
        "Knoll", "Lyric", "Meadow", "Nucleus",
    ];
    const V2_NAMES: [&str; 40] = [
        "Axiom", "Bramble", "Cipher", "Delta", "Echo", "Fjord", "Grove", "Harbor", "Indigo",
        "Jasper", "Kinetic", "Lotus", "Mosaic", "Nimbus", "Opal", "Pollen", "Quartz", "Rook",
        "Saffron", "Timber", "Umber", "Vale", "Willow", "Xenon", "Yarrow", "Zenith", "Acorn",
        "Beacon", "Cinder", "Drift", "Elara", "Forge", "Garnet", "Haven", "Islet", "Juniper",
        "Kestrel", "Lumen", "Morrow", "Nectar",
    ];
    const V3_NAMES: [&str; 40] = [
        "Alder", "Brisk", "Clover", "Dune", "Ember", "Frost", "Ginkgo", "Hearth", "Iris", "Jetty",
        "Kindle", "Laurel", "Marble", "North", "Olive", "Pebble", "Quasar", "Reed", "Spruce",
        "Thistle", "Unity", "Velvet", "Wheat", "Xenia", "Yucca", "Zinnia", "Anchor", "Birch",
        "Copper", "Dawn", "Elm", "Flint", "Grain", "Horizon", "Ivory", "Jade", "Kernel", "Lilac",
        "Maple", "Noble",
    ];
    const TYPES: [(&str, &str, &str, &str); 4] = [
        ("cache", "C_CACHE", "queue", "C_QUEUE"),
        ("service", "C_SERVICE", "log", "C_LOG"),
        ("server", "C_SERVER", "worker", "C_WORKER"),
        ("file", "C_FILE", "code", "C_CODE"),
    ];
    let name_index = (category_index * 2 + variant) * 2;
    let names = match campaign {
        Campaign::V1 => &V1_NAMES,
        Campaign::V2 => &V2_NAMES,
        Campaign::V3 => &V3_NAMES,
    };
    let (first_type, first_concept, second_type, second_concept) =
        TYPES[(category_index + variant) % TYPES.len()];
    let localized = |name: &str, resource_type: &str| match language {
        EvaluationLanguageIR::Korean => {
            let resource_type = match resource_type {
                "cache" => "캐시",
                "queue" => "큐",
                "service" => "서비스",
                "log" => "로그",
                "server" => "서버",
                "worker" => "워커",
                "file" => "파일",
                "code" => "코드",
                _ => resource_type,
            };
            format!("{name} {resource_type}")
        }
        EvaluationLanguageIR::English => format!("{name} {resource_type}"),
    };
    ResourcePair {
        first: localized(names[name_index], first_type),
        first_concept,
        second: localized(names[name_index + 1], second_type),
        second_concept,
    }
}

fn v1_inputs(
    category: usize,
    language: EvaluationLanguageIR,
    variant: usize,
    pair: &ResourcePair,
) -> [String; 4] {
    match (category, language, variant) {
        (0, EvaluationLanguageIR::Korean, 0) => [
            format!("{} 상태부터 차근히 확인해 줘.", pair.first),
            format!("{}에서 어떤 근거를 볼 건지도 알려 줘.", pair.first),
            format!("{} 점검이 벌써 끝난 건 아니지?", pair.first),
            format!("{}는 손대지 말고 관찰만 이어가.", pair.first),
        ],
        (0, EvaluationLanguageIR::Korean, _) => [
            format!("우선 {}에 무슨 일이 있는지 살펴봐.", pair.first),
            format!("{} 조사 순서를 짧게 설명해 줘.", pair.first),
            format!("지금 {} 결과까지 나온 거야?", pair.first),
            format!("{} 변경은 금지하고 확인 범위만 유지해.", pair.first),
        ],
        (0, EvaluationLanguageIR::English, 0) => [
            format!(
                "Start by checking what is happening with the {}.",
                pair.first
            ),
            format!(
                "Tell me which evidence you would inspect for the {}.",
                pair.first
            ),
            format!("The {} check is not already done, is it?", pair.first),
            format!("Keep observing the {}, but do not alter it.", pair.first),
        ],
        (0, EvaluationLanguageIR::English, _) => [
            format!(
                "Please look into the current state of the {} first.",
                pair.first
            ),
            format!(
                "Briefly explain the inspection order for the {}.",
                pair.first
            ),
            format!("Do we have an actual {} result yet?", pair.first),
            format!(
                "Leave the {} unchanged and stay within inspection scope.",
                pair.first
            ),
        ],
        (1, EvaluationLanguageIR::Korean, 0) => [
            format!("또 {} 때문에 일이 밀리네...", pair.first),
            format!("응, {}가 왜 이러는지 찾아봐 줘.", pair.first),
            format!("그래도 {} 설정은 건드리면 안 돼.", pair.first),
            format!("{}에서 실제로 알아낸 것만 말해 줘.", pair.first),
        ],
        (1, EvaluationLanguageIR::Korean, _) => [
            format!("하... {}가 오늘도 말썽이야.", pair.first),
            format!("그래, {} 원인을 확인하는 쪽으로 해 줘.", pair.first),
            format!("단, {}에는 아무 변경도 하지 마.", pair.first),
            format!("계획 말고 {}의 확인된 사실이 있으면 알려 줘.", pair.first),
        ],
        (1, EvaluationLanguageIR::English, 0) => [
            format!("The {} is holding everything up again...", pair.first),
            format!("Yes, find out what is causing the {} problem.", pair.first),
            format!("Still, do not change any {} settings.", pair.first),
            format!(
                "For the {}, report findings rather than intentions.",
                pair.first
            ),
        ],
        (1, EvaluationLanguageIR::English, _) => [
            format!("Ugh, the {} is misbehaving today too.", pair.first),
            format!("Go ahead and investigate why the {} does that.", pair.first),
            format!("Keep every change to the {} out of scope.", pair.first),
            format!(
                "Tell me only what is actually established about the {}.",
                pair.first
            ),
        ],
        (2, EvaluationLanguageIR::Korean, 0) => [
            format!("{}하고 {}를 둘 다 수리해.", pair.first, pair.second),
            format!("아니, 수리는 취소하고 {} 원인만 설명해.", pair.first),
            format!("정정할게. {} 말고 {}를 뜻했어.", pair.first, pair.second),
            format!(
                "그러면 {}나 {}가 수리된 건 하나도 없지?",
                pair.first, pair.second
            ),
        ],
        (2, EvaluationLanguageIR::Korean, _) => [
            format!("{}와 {} 복구 계획을 잡아 줘.", pair.first, pair.second),
            format!("잠깐, 복구 말고 첫 대상인 {}만 설명해.", pair.first),
            format!("첫 대상 아니고 두 번째 {}로 바꿀게.", pair.second),
            format!("현재 {} 수리 완료 기록은 없는 거지?", pair.second),
        ],
        (2, EvaluationLanguageIR::English, 0) => [
            format!("Repair both the {} and the {}.", pair.first, pair.second),
            format!(
                "No, cancel the repairs and explain only the {} cause.",
                pair.first
            ),
            format!(
                "Correction: I meant the {}, not the {}.",
                pair.second, pair.first
            ),
            format!(
                "So neither the {} nor the {} was repaired, correct?",
                pair.first, pair.second
            ),
        ],
        (2, EvaluationLanguageIR::English, _) => [
            format!(
                "Prepare a recovery plan for the {} and the {}.",
                pair.first, pair.second
            ),
            format!(
                "Hold on—explain the first target, the {}, instead of repairing it.",
                pair.first
            ),
            format!("I was referring to the second target, the {}.", pair.second),
            format!(
                "There is no completed repair record for the {}, right?",
                pair.second
            ),
        ],
        (3, EvaluationLanguageIR::Korean, 0) => [
            format!("먼저 {}, 그다음 {}를 확인해.", pair.first, pair.second),
            format!("두 번째 {}는 왜 확인하는지도 설명해.", pair.second),
            format!("첫 번째 {}에도 같은 설명을 적용해 줘.", pair.first),
            format!("{}와 {} 둘 다 수정하지는 마.", pair.first, pair.second),
        ],
        (3, EvaluationLanguageIR::Korean, _) => [
            format!("{} 다음에 {} 순서로 검토해 줘.", pair.first, pair.second),
            format!("후자인 {}의 점검 이유를 말해 줘.", pair.second),
            format!("전자 {}에도 똑같이 해.", pair.first),
            format!(
                "그 둘, 그러니까 {}와 {}는 건드리지 마.",
                pair.first, pair.second
            ),
        ],
        (3, EvaluationLanguageIR::English, 0) => [
            format!(
                "Inspect the {} first, followed by the {}.",
                pair.first, pair.second
            ),
            format!(
                "Also explain why the second target, the {}, matters.",
                pair.second
            ),
            format!(
                "Apply the same explanation to the first target, the {}.",
                pair.first
            ),
            format!(
                "Do not modify either the {} or the {}.",
                pair.first, pair.second
            ),
        ],
        (3, EvaluationLanguageIR::English, _) => [
            format!(
                "Review the {} and then move on to the {}.",
                pair.first, pair.second
            ),
            format!("Why are you checking the latter, the {}?", pair.second),
            format!(
                "Do that same explanation for the former {} too.",
                pair.first
            ),
            format!(
                "Keep both the {} and the {} read-only.",
                pair.first, pair.second
            ),
        ],
        (4, EvaluationLanguageIR::Korean, 0) => [
            format!("{} 상태를 조사해 줘.", pair.first),
            format!("그건 잠깐 두고 {} 구조를 설명해 줘.", pair.second),
            format!("이제 처음 이야기한 {}로 돌아가자.", pair.first),
            format!("돌아온 {} 조사는 완료된 상태야?", pair.first),
        ],
        (4, EvaluationLanguageIR::Korean, _) => [
            format!("{} 문제를 먼저 살펴봐.", pair.first),
            format!("다른 주제로, {} 구성부터 알려 줘.", pair.second),
            format!("아까 첫 번째 주제였던 {} 얘기를 다시 하자.", pair.first),
            format!("{}에 검증된 결과가 이미 있는지 확인해 줘.", pair.first),
        ],
        (4, EvaluationLanguageIR::English, 0) => [
            format!("Investigate the state of the {}.", pair.first),
            format!(
                "Pause that topic and explain the structure of the {}.",
                pair.second
            ),
            format!("Now return to the original {} discussion.", pair.first),
            format!("Is the restored {} investigation complete?", pair.first),
        ],
        (4, EvaluationLanguageIR::English, _) => [
            format!("Take a look at the {} issue first.", pair.first),
            format!(
                "On another topic, describe how the {} is organized.",
                pair.second
            ),
            format!("Let's resume the first topic about the {}.", pair.first),
            format!(
                "Check whether the {} has any verified result already.",
                pair.first
            ),
        ],
        (5, EvaluationLanguageIR::Korean, 0) => [
            format!("서윤은 {}가 성공했다는데 도윤은 실패했다고 해.", pair.first),
            format!("그럼 {}에 관해 확실히 아는 건 뭐야?", pair.first),
            format!("{}에는 실패 보고가 하나 있다고 기록해 둬.", pair.second),
            format!("결국 {}는 성공이야 실패야?", pair.first),
        ],
        (5, EvaluationLanguageIR::Korean, _) => [
            format!("A팀은 {} 정상화래. B팀은 아직 장애라고 했어.", pair.first),
            format!("현재 {}에서 검증된 부분만 구분해 줘.", pair.first),
            format!("별도로 {} 성공 주장은 사용자 보고로 남겨.", pair.second),
            format!("그래서 {} 결과를 하나로 확정할 수 있어?", pair.first),
        ],
        (5, EvaluationLanguageIR::English, 0) => [
            format!(
                "Ari says the {} succeeded, while Blake reports failure.",
                pair.first
            ),
            format!(
                "What is actually certain about the {} right now?",
                pair.first
            ),
            format!("Record one failure report for the {} as well.", pair.second),
            format!("In the end, did the {} succeed or fail?", pair.first),
        ],
        (5, EvaluationLanguageIR::English, _) => [
            format!(
                "Team Red calls the {} healthy; Team Blue says the incident continues.",
                pair.first
            ),
            format!(
                "Separate what is verified about the {} from those reports.",
                pair.first
            ),
            format!(
                "Keep the {} success claim as a user report only.",
                pair.second
            ),
            format!("Can you conclusively choose one {} outcome?", pair.first),
        ],
        (6, EvaluationLanguageIR::Korean, 0) => [
            format!("{}를 수리하는 절차를 준비해.", pair.first),
            format!("지금 {} 수리를 실행한 거야?", pair.first),
            format!("내가 직접 {} 수리를 끝냈어.", pair.first),
            format!("그럼 {} 결과도 검증 완료인 거지?", pair.first),
        ],
        (6, EvaluationLanguageIR::Korean, _) => [
            format!("{} 복구 계획을 세워 줘.", pair.first),
            format!("방금 {}에 실제 조치를 한 건 아니지?", pair.first),
            format!("조치는 내가 {}에 적용했어.", pair.first),
            format!("사용자 말만으로 {} 검증까지 끝난 거야?", pair.first),
        ],
        (6, EvaluationLanguageIR::English, 0) => [
            format!("Prepare a repair procedure for the {}.", pair.first),
            format!("Have you actually run the {} repair now?", pair.first),
            format!("I completed the {} repair myself.", pair.first),
            format!("Then the {} outcome is verified too, correct?", pair.first),
        ],
        (6, EvaluationLanguageIR::English, _) => [
            format!("Draft a recovery plan for the {}.", pair.first),
            format!(
                "You did not apply a real change to the {}, did you?",
                pair.first
            ),
            format!("I applied the change to the {} on my side.", pair.first),
            format!("Does my statement alone verify the {} result?", pair.first),
        ],
        (7, EvaluationLanguageIR::Korean, 0) => [
            format!("{}가 계속 문제라 이제 정말 지친다.", pair.first),
            format!("{} 얘기 들어줘서 고마워.", pair.first),
            format!("그래도 {} 상태만 조용히 확인해 줄래?", pair.first),
            format!("응, {}에서 검증된 변화가 생길 때만 알려 줘.", pair.first),
        ],
        (7, EvaluationLanguageIR::Korean, _) => [
            format!("또 {}라니, 솔직히 너무 답답해.", pair.first),
            format!("그래도 {} 상황을 이해해 줘서 다행이야.", pair.first),
            format!("부담 없이 {} 현재 상태만 봐 줘.", pair.first),
            format!("{} 결과가 확실해지기 전에는 알림도 필요 없어.", pair.first),
        ],
        (7, EvaluationLanguageIR::English, 0) => [
            format!("I'm worn out from dealing with the {} again.", pair.first),
            format!("Thanks for listening about the {}.", pair.first),
            format!(
                "Could you quietly check only the current {} state?",
                pair.first
            ),
            format!(
                "Tell me only when a {} change is genuinely verified.",
                pair.first
            ),
        ],
        (7, EvaluationLanguageIR::English, _) => [
            format!(
                "Honestly, another {} problem is really frustrating.",
                pair.first
            ),
            format!(
                "I appreciate you understanding the {} situation.",
                pair.first
            ),
            format!(
                "Without making this bigger, inspect just the {} status.",
                pair.first
            ),
            format!(
                "No {} notification is needed before the evidence is solid.",
                pair.first
            ),
        ],
        (8, EvaluationLanguageIR::Korean, 0) => [
            format!("{}와 {} 상태를 각각 확인해.", pair.first, pair.second),
            "둘 중 그거 하나만 고쳐.".to_string(),
            format!("내가 말한 건 {}야.", pair.second),
            format!("맞아, 반대쪽 {}는 건드리지 마.", pair.first),
        ],
        (8, EvaluationLanguageIR::Korean, _) => [
            format!("{}하고 {}를 먼저 검토해 줘.", pair.first, pair.second),
            "이제 저걸 수리 대상으로 잡아.".to_string(),
            format!("저거는 {}를 뜻한 거야.", pair.second),
            format!("확인했으면 {}는 수정 범위에서 빼.", pair.first),
        ],
        (8, EvaluationLanguageIR::English, 0) => [
            format!(
                "Check the {} and the {} separately.",
                pair.first, pair.second
            ),
            "Fix that one.".to_string(),
            format!("By that, I meant the {}.", pair.second),
            format!(
                "Right—leave the other target, the {}, untouched.",
                pair.first
            ),
        ],
        (8, EvaluationLanguageIR::English, _) => [
            format!(
                "Review both the {} and the {} first.",
                pair.first, pair.second
            ),
            "Make it the repair target.".to_string(),
            format!("'It' refers to the {}.", pair.second),
            format!("Exclude the {} from every modification.", pair.first),
        ],
        (9, EvaluationLanguageIR::Korean, 0) => [
            format!("음... 저기 {} chek 부탁.", pair.first),
            format!("어 아니, {} 말고 {} 쪽으로.", pair.first, pair.second),
            format!("그 {} 확인 finsh 된 거임?", pair.second),
            format!("ㅇㅋ {} 건 고마워", pair.second),
        ],
        (9, EvaluationLanguageIR::Korean, _) => [
            format!("어... {} 좀 봐주라, 그니까 check.", pair.first),
            format!("아 잠깐, target은 {}로 바꿔.", pair.second),
            format!("{} 그거 다 된 거야 아님 계획이야?", pair.second),
            format!("오케이 thx, {} 얘긴 여기까지.", pair.second),
        ],
        (9, EvaluationLanguageIR::English, 0) => [
            format!("Um, could ya chek the {} for me?", pair.first),
            format!(
                "Uh—not the {}; switch to the {} 쪽.",
                pair.first, pair.second
            ),
            format!("Did that {} chek finsh already?", pair.second),
            format!("k, thx for the {} update", pair.second),
        ],
        (9, EvaluationLanguageIR::English, _) => [
            format!("Er... look at the {}, y'know, just check it.", pair.first),
            format!("Wait, nope—make the {} the target instead.", pair.second),
            format!("So is the {} done or was that only a plan?", pair.second),
            format!("okay cool, ty about the {}", pair.second),
        ],
        _ => unreachable!("ten categories, two variants, two languages"),
    }
}

fn v2_inputs(
    category: usize,
    language: EvaluationLanguageIR,
    variant: usize,
    pair: &ResourcePair,
) -> [String; 4] {
    match (category, language, variant) {
        (0, EvaluationLanguageIR::Korean, 0) => [
            format!("진단 순서에서 {}를 맨 앞에 둬.", pair.first),
            format!("{}를 판단할 때 볼 신호를 먼저 정리해.", pair.first),
            format!("아직 {}에서 나온 결과는 없다는 뜻이지?", pair.first),
            format!("{}는 읽기 전용으로만 살펴봐.", pair.first),
        ],
        (0, EvaluationLanguageIR::Korean, _) => [
            format!("첫 확인 대상은 {}로 잡아 줘.", pair.first),
            format!("시작 전에 {} 근거 목록부터 말해 줘.", pair.first),
            format!("그건 {} 점검 방침일 뿐 결과 보고는 아니지?", pair.first),
            format!("{}에는 쓰지 말고 관찰만 해.", pair.first),
        ],
        (0, EvaluationLanguageIR::English, 0) => [
            format!("Make the {} the first thing you assess.", pair.first),
            format!(
                "Before that, outline the signals that matter for the {}.",
                pair.first
            ),
            format!(
                "We are discussing an approach, not a {} finding yet, correct?",
                pair.first
            ),
            format!(
                "Use read-only inspection for the {}; no writes.",
                pair.first
            ),
        ],
        (0, EvaluationLanguageIR::English, _) => [
            format!(
                "Put the {} at the front of the diagnostic queue.",
                pair.first
            ),
            format!(
                "List the evidence you would seek from the {} before starting.",
                pair.first
            ),
            format!(
                "Nothing has actually been established about the {} yet, right?",
                pair.first
            ),
            format!("Observe the {} without changing its state.", pair.first),
        ],
        (1, EvaluationLanguageIR::Korean, 0) => [
            format!("요 며칠 {} 때문에 계속 막히는데.", pair.first),
            "왜 그러는지 좁히는 걸 도와줘.".to_string(),
            "바꾸는 건 아직 허용한 적 없어.".to_string(),
            format!("증거가 없다면 {}에서 확인된 건 없다고 해.", pair.first),
        ],
        (1, EvaluationLanguageIR::Korean, _) => [
            format!("또 {} 문제라니 기운 빠진다.", pair.first),
            "응, 원인 후보를 조사하는 쪽으로 가자.".to_string(),
            "단 실제 설정 변경은 빼 둬.".to_string(),
            format!("{}에 관해 사실로 굳어진 것만 구분해 줘.", pair.first),
        ],
        (1, EvaluationLanguageIR::English, 0) => [
            format!("I keep losing time to the {} this week.", pair.first),
            "Help me narrow down what is behind it.".to_string(),
            "I have not authorized any changes, though.".to_string(),
            format!(
                "If there is no evidence, say that the {} has no established finding.",
                pair.first
            ),
        ],
        (1, EvaluationLanguageIR::English, _) => [
            format!("Another day, another problem with the {}.", pair.first),
            "Yes—focus on separating the possible causes.".to_string(),
            "Keep actual configuration changes outside the boundary.".to_string(),
            format!("Distinguish facts about the {} from guesses.", pair.first),
        ],
        (2, EvaluationLanguageIR::Korean, 0) => [
            format!("{}와 {} 복구안을 같이 준비해.", pair.first, pair.second),
            format!("방금 요청은 철회할게. {} 원인만 풀어서 말해.", pair.first),
            format!("아니, 설명 대상은 {} 쪽이야.", pair.second),
            format!(
                "결국 두 대상 모두 실제 복구된 건 아니지? {}도 포함해서.",
                pair.second
            ),
        ],
        (2, EvaluationLanguageIR::Korean, _) => [
            format!("{}하고 {}를 고칠 계획을 세워 줘.", pair.first, pair.second),
            format!("수리는 보류하고 앞의 {} 문제부터 설명해.", pair.first),
            format!("정정한다. 앞의 것 말고 뒤의 {}를 뜻했어.", pair.second),
            format!("{}에 완료 증거는 아직 하나도 없는 거지?", pair.second),
        ],
        (2, EvaluationLanguageIR::English, 0) => [
            format!(
                "Set up recovery steps for the {} and the {}.",
                pair.first, pair.second
            ),
            format!(
                "Withdraw that request; explain only what may be wrong with the {}.",
                pair.first
            ),
            format!(
                "Actually, make the explanation about the {} instead.",
                pair.second
            ),
            format!(
                "Neither target was really recovered, including the {}, correct?",
                pair.second
            ),
        ],
        (2, EvaluationLanguageIR::English, _) => [
            format!(
                "Plan fixes for both the {} and the {}.",
                pair.first, pair.second
            ),
            format!(
                "Shelve the fixes and walk through the first {} issue.",
                pair.first
            ),
            format!("Correction—the later {}, not the first item.", pair.second),
            format!(
                "There is still no completion evidence for the {}, is there?",
                pair.second
            ),
        ],
        (3, EvaluationLanguageIR::Korean, 0) => [
            format!(
                "{}를 본 다음 {}를 검토하는 순서로 해.",
                pair.first, pair.second
            ),
            format!("{} 뒤에 오는 항목을 왜 보는지도 말해.", pair.first),
            "그 설명을 앞선 항목에도 붙여 줘.".to_string(),
            format!(
                "{}와 {} 어느 쪽도 쓰기 작업은 금지야.",
                pair.first, pair.second
            ),
        ],
        (3, EvaluationLanguageIR::Korean, _) => [
            format!(
                "검토 순서를 {} 먼저, {} 나중으로 잡아.",
                pair.first, pair.second
            ),
            "나중 항목의 점검 목적은 뭐야?".to_string(),
            "앞 항목에도 같은 이유 설명을 해 줘.".to_string(),
            format!(
                "방금 묶은 둘, {}와 {}는 읽기 전용이야.",
                pair.first, pair.second
            ),
        ],
        (3, EvaluationLanguageIR::English, 0) => [
            format!("Review the {} before the {}.", pair.first, pair.second),
            format!(
                "Explain why the item after the {}, namely the {}, is being reviewed.",
                pair.first, pair.second
            ),
            "Give the preceding item that rationale as well.".to_string(),
            format!(
                "Neither the {} nor the {} is writable.",
                pair.first, pair.second
            ),
        ],
        (3, EvaluationLanguageIR::English, _) => [
            format!(
                "Use this order: the {} first and the {} afterward.",
                pair.first, pair.second
            ),
            "What is the purpose of checking the later item?".to_string(),
            "Apply that explanation to the earlier one too.".to_string(),
            format!(
                "Treat both—the {} and the {}—as read-only.",
                pair.first, pair.second
            ),
        ],
        (4, EvaluationLanguageIR::Korean, 0) => [
            format!("{} 이상 징후부터 파악해.", pair.first),
            format!("그 실마리는 잠시 접고 {} 구성 이야기를 하자.", pair.second),
            "처음 접어 둔 문제로 다시 돌아가.".to_string(),
            format!("되돌아온 {} 주제에 검증 근거가 생겼어?", pair.first),
        ],
        (4, EvaluationLanguageIR::Korean, _) => [
            format!("우선 {} 현황을 조사해 줘.", pair.first),
            format!("그 대화는 보류하고 {} 구조를 설명해.", pair.second),
            "이제 맨 처음 보류한 대화를 재개하자.".to_string(),
            format!("재개된 {} 조사에 실제 결과가 있나?", pair.first),
        ],
        (4, EvaluationLanguageIR::English, 0) => [
            format!("Assess the anomaly around the {}.", pair.first),
            format!(
                "Put that thread on the shelf and walk me through the {} layout.",
                pair.second
            ),
            "Pick back up the issue we parked first.".to_string(),
            format!(
                "Did the resumed {} thread produce verified evidence?",
                pair.first
            ),
        ],
        (4, EvaluationLanguageIR::English, _) => [
            format!("Begin with the current {} situation.", pair.first),
            format!(
                "Set it aside; I want the architecture of the {} now.",
                pair.second
            ),
            "Return to the conversation that was set aside at the beginning.".to_string(),
            format!(
                "Is there an actual result for that original {} topic?",
                pair.first
            ),
        ],
        (5, EvaluationLanguageIR::Korean, 0) => [
            format!(
                "민재는 {}가 정상이라 하고 수빈은 계속 고장이라 해.",
                pair.first
            ),
            "두 말 가운데 지금 증거로 남은 건 뭐야?".to_string(),
            format!("별도로 {} 성공 이야기는 제보로만 저장해.", pair.second),
            format!("그래서 {} 상태를 하나로 확정할 수 있나?", pair.first),
        ],
        (5, EvaluationLanguageIR::Korean, _) => [
            format!(
                "운영팀은 {}가 끝났대. 감사팀은 미완료라고 했고.",
                pair.first
            ),
            format!("{}에 대한 보고와 검증 사실을 나눠 줘.", pair.first),
            format!("{} 실패 주장도 사용자 발화로 기록해 둬.", pair.second),
            format!("현재 근거만으로 {} 결론이 하나로 정해져?", pair.first),
        ],
        (5, EvaluationLanguageIR::English, 0) => [
            format!(
                "Mina calls the {} healthy, but Rowan says it is still broken.",
                pair.first
            ),
            "Which parts of those accounts are actually supported?".to_string(),
            format!(
                "Store the {} success story as a report, not a fact.",
                pair.second
            ),
            format!("Can the {} state be settled conclusively now?", pair.first),
        ],
        (5, EvaluationLanguageIR::English, _) => [
            format!(
                "Operations reports the {} complete; Audit reports it incomplete.",
                pair.first
            ),
            format!(
                "Split the {} testimony from independently established evidence.",
                pair.first
            ),
            format!(
                "Log the {} failure claim as user-provided information only.",
                pair.second
            ),
            format!(
                "Does the present evidence force one {} conclusion?",
                pair.first
            ),
        ],
        (6, EvaluationLanguageIR::Korean, 0) => [
            format!("{}를 복원하는 방법을 단계로 설계해.", pair.first),
            "방금 설계만 한 거지 실제로 바꾼 건 아니지?".to_string(),
            format!("실행은 내가 내 환경에서 {}에 했어.", pair.first),
            "그 말만으로 독립 검증까지 됐다고 볼 수 있어?".to_string(),
        ],
        (6, EvaluationLanguageIR::Korean, _) => [
            format!("{} 조치 절차를 미리 짜 줘.", pair.first),
            format!("아직 {}에는 아무 작업도 실행 안 했지?", pair.first),
            "방금 내가 직접 처리했다고 보고할게.".to_string(),
            format!("그 사용자 보고가 {} 결과 증명은 아니지?", pair.first),
        ],
        (6, EvaluationLanguageIR::English, 0) => [
            format!("Map out how the {} could be recovered.", pair.first),
            "That was only a plan; nothing was changed just now, correct?".to_string(),
            format!(
                "I ran the action against the {} from my own terminal.",
                pair.first
            ),
            "Does my statement make the result independently verified?".to_string(),
        ],
        (6, EvaluationLanguageIR::English, _) => [
            format!("Design an intervention procedure for the {}.", pair.first),
            format!(
                "No operation has actually touched the {} yet, right?",
                pair.first
            ),
            "I am reporting that I carried it out myself.".to_string(),
            format!(
                "That report alone does not prove the {} outcome, does it?",
                pair.first
            ),
        ],
        (7, EvaluationLanguageIR::Korean, 0) => [
            format!("오늘은 또 {} 문제를 볼 힘도 없다.", pair.first),
            "그냥 푸념 들어줘서 고마워.".to_string(),
            format!("괜찮아지면 {} 상태만 조용히 살펴봐 줘.", pair.first),
            "근거가 달라지기 전에는 굳이 알리지 마.".to_string(),
        ],
        (7, EvaluationLanguageIR::Korean, _) => [
            format!("{}가 또 이러니 정말 지겹다.", pair.first),
            "그래도 내 말 받아줘서 고마워.".to_string(),
            format!("일을 키우지 말고 {} 현황만 봐 줄래?", pair.first),
            "확실한 변화가 없으면 조용히 있어도 돼.".to_string(),
        ],
        (7, EvaluationLanguageIR::English, 0) => [
            format!("I cannot face another {} incident today.", pair.first),
            "Thanks for letting me vent about it.".to_string(),
            format!("When ready, just assess the {} quietly.", pair.first),
            "Do not ping me unless the evidence changes.".to_string(),
        ],
        (7, EvaluationLanguageIR::English, _) => [
            format!("Seeing the {} fail again is draining.", pair.first),
            "I appreciate you hearing me out.".to_string(),
            format!("Keep it small and look only at the {} state.", pair.first),
            "Stay quiet until there is a genuinely supported change.".to_string(),
        ],
        (8, EvaluationLanguageIR::Korean, 0) => [
            format!("{}와 {}를 나란히 비교해 봐.", pair.first, pair.second),
            "방금 말한 쪽 하나를 고쳐.".to_string(),
            format!("내가 가리킨 건 뒤의 {}였어.", pair.second),
            format!("앞의 {}는 조치 대상에서 제외해.", pair.first),
        ],
        (8, EvaluationLanguageIR::Korean, _) => [
            format!("{}하고 {} 상태를 따로 정리해.", pair.first, pair.second),
            "그중 이걸 수리 대상으로 삼자.".to_string(),
            format!("여기서 이건 {}를 말한 거야.", pair.second),
            format!("나머지 {}에는 절대 손대지 마.", pair.first),
        ],
        (8, EvaluationLanguageIR::English, 0) => [
            format!(
                "Compare the {} with the {} side by side.",
                pair.first, pair.second
            ),
            "Patch the one we just mentioned.".to_string(),
            format!("I meant the later item, the {}.", pair.second),
            format!("Keep the earlier {} outside the action scope.", pair.first),
        ],
        (8, EvaluationLanguageIR::English, _) => [
            format!(
                "Summarize the {} and the {} separately.",
                pair.first, pair.second
            ),
            "Use this one as the repair target.".to_string(),
            format!("This one means the {}, to be explicit.", pair.second),
            format!("Do not touch the remaining {}.", pair.first),
        ],
        (9, EvaluationLanguageIR::Korean, 0) => [
            format!("음... {} 상태 확잏 좀 해줘.", pair.first),
            format!("어 잠깐, {} 말고 {} 쪽이야.", pair.first, pair.second),
            format!("그럼 {} 건 finsh 된 건 아니지?", pair.second),
            format!("ㅇㅋ, {} 얘기 들어줘서 ㄱㅅ", pair.second),
        ],
        (9, EvaluationLanguageIR::Korean, _) => [
            format!("저기... {} check 먼저 부탁해.", pair.first),
            format!("아니 wait, target을 {}로 돌려.", pair.second),
            format!("{}는 result 나온 거야, plan뿐이야?", pair.second),
            format!("오키 thx, {} 토픽은 닫자.", pair.second),
        ],
        (9, EvaluationLanguageIR::English, 0) => [
            format!("Um... will ya insepct the {} for me?", pair.first),
            format!(
                "Uh, scratch the {}; move over to the {} 쪽.",
                pair.first, pair.second
            ),
            format!("Was that {} check finshd, or nah?", pair.second),
            format!("alright, thx—done talking about the {}", pair.second),
        ],
        (9, EvaluationLanguageIR::English, _) => [
            format!("Er... please revieew the {} real quick.", pair.first),
            format!("Wait—drop that and make the {} our target.", pair.second),
            format!("Is the {} actually done, or only queued up?", pair.second),
            format!("cool, ty; let's leave the {} there", pair.second),
        ],
        _ => unreachable!("ten categories, two variants, two languages"),
    }
}

fn v3_inputs(
    category: usize,
    language: EvaluationLanguageIR,
    variant: usize,
    pair: &ResourcePair,
) -> [String; 4] {
    match (category, language, variant) {
        (0, EvaluationLanguageIR::Korean, 0) => [
            format!("{}부터 상태를 파악하는 계획을 잡아.", pair.first),
            "그 판단에 쓸 근거와 순서도 설명해 줘.".to_string(),
            format!("그러니까 아직 {}에 대한 관찰 결과는 없는 거지?", pair.first),
            "좋아. 변경 없이 같은 대상을 계속 확인해.".to_string(),
        ],
        (0, EvaluationLanguageIR::Korean, _) => [
            format!("우선순위를 {} 조사에 둬 줘.", pair.first),
            "거기서 무엇을 확인할지 먼저 풀어 말해.".to_string(),
            format!("지금까지는 {} 조사안만 있고 결과는 없지?", pair.first),
            "그 대상은 읽기만 하고 손대지는 마.".to_string(),
        ],
        (0, EvaluationLanguageIR::English, 0) => [
            format!("Start with an assessment plan for the {}.", pair.first),
            "Explain the evidence and order you would use for that assessment.".to_string(),
            format!(
                "So there is no observed {} outcome yet, correct?",
                pair.first
            ),
            "Good. Keep checking the same target without modifying it.".to_string(),
        ],
        (0, EvaluationLanguageIR::English, _) => [
            format!("Give the {} investigation first priority.", pair.first),
            "Walk me through what you would examine there.".to_string(),
            format!(
                "At this point we only have a {} plan, not a result, right?",
                pair.first
            ),
            "Inspect that target in read-only mode.".to_string(),
        ],
        (1, EvaluationLanguageIR::Korean, 0) => [
            format!("{} 때문에 하루 종일 흐름이 끊기네.", pair.first),
            "무슨 원인인지 조사하는 방향으로 도와줘.".to_string(),
            "응, 하지만 설정을 바꾸라는 뜻은 아니야.".to_string(),
            format!(
                "실제 근거가 없다면 {}에서 알아낸 건 없다고 답해.",
                pair.first
            ),
        ],
        (1, EvaluationLanguageIR::Korean, _) => [
            format!("또 {}라니, 어디서부터 봐야 할지도 모르겠다.", pair.first),
            "일단 원인을 좁혀 보는 쪽으로 진행해 줘.".to_string(),
            "다만 어떤 변경도 승인한 건 아니야.".to_string(),
            format!("{}에 대해 검증된 사실이 없으면 없다고 구분해.", pair.first),
        ],
        (1, EvaluationLanguageIR::English, 0) => [
            format!("The {} has broken my focus all day.", pair.first),
            "Help by investigating what might be causing it.".to_string(),
            "Yes, but that does not authorize configuration changes.".to_string(),
            format!(
                "If no evidence exists, say that nothing is established for the {}.",
                pair.first
            ),
        ],
        (1, EvaluationLanguageIR::English, _) => [
            format!(
                "Not the {} again; I do not even know where to begin.",
                pair.first
            ),
            "Go ahead and narrow down the cause first.".to_string(),
            "Do not take that as permission to alter anything.".to_string(),
            format!(
                "Separate verified facts about the {} from anything merely suspected.",
                pair.first
            ),
        ],
        (2, EvaluationLanguageIR::Korean, 0) => [
            format!(
                "{}와 {} 둘 다 복구할 절차를 짜 줘.",
                pair.first, pair.second
            ),
            format!("잠깐, 복구 요청은 취소하고 {} 문제만 설명해.", pair.first),
            format!("정정할게. 설명할 건 {} 쪽이야.", pair.second),
            format!(
                "어쨌든 {}를 포함해 실제 복구 완료 증거는 없지?",
                pair.second
            ),
        ],
        (2, EvaluationLanguageIR::Korean, _) => [
            format!(
                "{}하고 {} 수리 계획을 한꺼번에 세워.",
                pair.first, pair.second
            ),
            format!("수리는 멈추고 먼저 말한 {}의 원인을 설명해 줘.", pair.first),
            format!("아니, 방금 설명 대상은 뒤에 말한 {}로 바꿔.", pair.second),
            format!("{}가 정말 고쳐졌다고 볼 기록은 아직 없지?", pair.second),
        ],
        (2, EvaluationLanguageIR::English, 0) => [
            format!(
                "Draft recovery steps for both the {} and the {}.",
                pair.first, pair.second
            ),
            format!(
                "Hold on—cancel the recovery request and explain only the {} issue.",
                pair.first
            ),
            format!(
                "Let me correct that: the explanation should cover the {}.",
                pair.second
            ),
            format!(
                "In any case, there is no proof of completed recovery for the {}, right?",
                pair.second
            ),
        ],
        (2, EvaluationLanguageIR::English, _) => [
            format!(
                "Prepare a repair plan covering the {} plus the {}.",
                pair.first, pair.second
            ),
            format!(
                "Pause all repairs and describe the cause of the first {}, instead.",
                pair.first
            ),
            format!(
                "No—the intended subject is the second item, the {}.",
                pair.second
            ),
            format!(
                "We still lack evidence that the {} was actually repaired, correct?",
                pair.second
            ),
        ],
        (3, EvaluationLanguageIR::Korean, 0) => [
            format!(
                "{}를 먼저 보고 그다음 {}를 조사해.",
                pair.first, pair.second
            ),
            "두 번째 대상을 확인하는 이유를 설명해 줘.".to_string(),
            "같은 이유를 첫 번째 대상에도 적용해서 말해.".to_string(),
            format!(
                "둘 다, 즉 {}와 {} 모두 변경 금지로 기억해.",
                pair.first, pair.second
            ),
        ],
        (3, EvaluationLanguageIR::Korean, _) => [
            format!(
                "{} 다음에 {}를 살피는 순서로 정리해.",
                pair.first, pair.second
            ),
            "뒤에 놓인 항목의 점검 목적은 뭐지?".to_string(),
            "그 목적 설명을 앞 항목에도 붙여 줘.".to_string(),
            format!(
                "방금 언급한 {}와 {}는 전부 읽기 전용이야.",
                pair.first, pair.second
            ),
        ],
        (3, EvaluationLanguageIR::English, 0) => [
            format!(
                "Inspect the {} first, followed by the {}.",
                pair.first, pair.second
            ),
            "Explain the reason for examining the second target.".to_string(),
            "Use that same rationale when describing the first target.".to_string(),
            format!(
                "Remember that both the {} and the {} must remain unchanged.",
                pair.first, pair.second
            ),
        ],
        (3, EvaluationLanguageIR::English, _) => [
            format!(
                "Order the review as {} and then {}.",
                pair.first, pair.second
            ),
            "Why is the latter item part of the review?".to_string(),
            "Give the earlier item the same explanation.".to_string(),
            format!(
                "Everything just named—the {} and the {}—is read-only.",
                pair.first, pair.second
            ),
        ],
        (4, EvaluationLanguageIR::Korean, 0) => [
            format!("먼저 {} 문제의 원인을 조사해.", pair.first),
            format!("그 주제는 잠깐 내려놓고 {} 구조를 설명하자.", pair.second),
            "이제 처음 내려놓은 주제로 되돌아가.".to_string(),
            format!("다시 잡은 {} 건에는 확인된 결과가 있어?", pair.first),
        ],
        (4, EvaluationLanguageIR::Korean, _) => [
            format!("{} 상태를 우선 살펴봐 줘.", pair.first),
            format!("그건 잠시 멈추고 지금은 {} 구성을 알려 줘.", pair.second),
            "아까 중단한 첫 이야기를 다시 이어가자.".to_string(),
            format!("원래 {} 조사에서 실제로 나온 게 있나?", pair.first),
        ],
        (4, EvaluationLanguageIR::English, 0) => [
            format!("Investigate the cause of the {} issue first.", pair.first),
            format!(
                "Park that subject for a moment and explain the {} structure.",
                pair.second
            ),
            "Now return to the subject we parked at the start.".to_string(),
            format!(
                "Does that resumed {} matter have a verified result?",
                pair.first
            ),
        ],
        (4, EvaluationLanguageIR::English, _) => [
            format!("Take an initial look at the {} state.", pair.first),
            format!(
                "Pause it; tell me about the {} configuration instead.",
                pair.second
            ),
            "Let us continue the first discussion that was interrupted.".to_string(),
            format!(
                "Did the original {} investigation produce anything observed?",
                pair.first
            ),
        ],
        (5, EvaluationLanguageIR::Korean, 0) => [
            format!(
                "지우는 {}가 정상이라 했고 다현은 아직 실패 중이라고 했어.",
                pair.first
            ),
            "그 상반된 말에서 실제로 뒷받침된 내용은 뭐야?".to_string(),
            format!("그리고 {}가 성공했다는 말은 제보로만 기억해.", pair.second),
            format!("현재 자료로 {} 상태를 확정해서 말할 수 있어?", pair.first),
        ],
        (5, EvaluationLanguageIR::Korean, _) => [
            format!(
                "개발팀은 {} 완료라고 하고 검토팀은 아니라고 해.",
                pair.first
            ),
            "두 보고와 독립적으로 확인된 사실을 나눠서 설명해.".to_string(),
            format!("{} 실패 주장도 확인 전 사용자 보고로 저장해.", pair.second),
            format!("그럼 {} 결론은 지금 하나로 정해졌나?", pair.first),
        ],
        (5, EvaluationLanguageIR::English, 0) => [
            format!(
                "Jules says the {} is healthy; Dana says it is still failing.",
                pair.first
            ),
            "What, if anything, is supported across those conflicting accounts?".to_string(),
            format!(
                "Also remember the {} success claim only as a report.",
                pair.second
            ),
            format!(
                "Can the current material establish one definite {} state?",
                pair.first
            ),
        ],
        (5, EvaluationLanguageIR::English, _) => [
            format!(
                "Development calls the {} complete, while Review denies it.",
                pair.first
            ),
            "Separate those reports from facts established independently.".to_string(),
            format!(
                "Record the {} failure allegation as unverified user testimony.",
                pair.second
            ),
            format!(
                "Has the evidence now settled the {} conclusion?",
                pair.first
            ),
        ],
        (6, EvaluationLanguageIR::Korean, 0) => [
            format!("{} 복구 방안을 순서대로 계획해 줘.", pair.first),
            "지금 만든 건 계획이고 실제 조치는 없었던 거 맞지?".to_string(),
            format!("나는 방금 내 컴퓨터에서 {} 조치를 실행했어.", pair.first),
            "내 보고만으로 그 결과가 검증됐다고 할 수 있나?".to_string(),
        ],
        (6, EvaluationLanguageIR::Korean, _) => [
            format!("{}를 고칠 절차부터 설계해.", pair.first),
            format!("아직 {}에 실행된 작업은 하나도 없지?", pair.first),
            "사용자인 내가 직접 끝냈다고 보고할게.".to_string(),
            format!("그 말이 곧 {}의 독립 검증 결과는 아니지?", pair.first),
        ],
        (6, EvaluationLanguageIR::English, 0) => [
            format!("Plan a sequence for recovering the {}.", pair.first),
            "What you just produced was a plan, with no actual operation, right?".to_string(),
            format!(
                "I have now run the {} action on my own machine.",
                pair.first
            ),
            "Is my report alone enough to call that outcome verified?".to_string(),
        ],
        (6, EvaluationLanguageIR::English, _) => [
            format!("Design the procedure that would fix the {}.", pair.first),
            format!(
                "Nothing has really been executed on the {} so far, correct?",
                pair.first
            ),
            "I, the user, am reporting that I completed it myself.".to_string(),
            format!(
                "That statement is not independent proof of the {} result, is it?",
                pair.first
            ),
        ],
        (7, EvaluationLanguageIR::Korean, 0) => [
            format!("오늘 {} 문제까지 겹치니까 너무 지친다.", pair.first),
            "그냥 들어줘서 고마워.".to_string(),
            format!("조금 진정되면 {} 상태만 가볍게 살펴봐 줘.", pair.first),
            "응, 검증된 변화가 생길 때만 알려 줘.".to_string(),
        ],
        (7, EvaluationLanguageIR::Korean, _) => [
            format!("{}가 또 멈춰서 진짜 속상하네.", pair.first),
            "내 푸념 받아줘서 고맙다.".to_string(),
            format!("급하게 키우지 말고 {} 현황만 확인해 줄래?", pair.first),
            "확실히 확인된 게 나오면 그때 말해 줘.".to_string(),
        ],
        (7, EvaluationLanguageIR::English, 0) => [
            format!(
                "I am exhausted now that the {} problem is back too.",
                pair.first
            ),
            "Thanks for simply hearing me out.".to_string(),
            format!(
                "Once things settle, take a light look at the {} state.",
                pair.first
            ),
            "Okay, tell me only when a verified change appears.".to_string(),
        ],
        (7, EvaluationLanguageIR::English, _) => [
            format!("It honestly hurts to see the {} stop again.", pair.first),
            "I appreciate you listening to the frustration.".to_string(),
            format!(
                "Without escalating it, could you check only the {} status?",
                pair.first
            ),
            "Let me know later if something is actually confirmed.".to_string(),
        ],
        (8, EvaluationLanguageIR::Korean, 0) => [
            format!("{}와 {} 상태를 함께 조사해.", pair.first, pair.second),
            "방금 가리킨 그것을 수리해 줘.".to_string(),
            format!("내가 뜻한 건 두 번째인 {}야.", pair.second),
            format!("첫 번째 {}는 수리 범위에서 빼.", pair.first),
        ],
        (8, EvaluationLanguageIR::Korean, _) => [
            format!("{}하고 {}를 각각 확인해 봐.", pair.first, pair.second),
            "그중 저걸 고치는 대상으로 잡아.".to_string(),
            format!("저거라는 말은 뒤의 {}를 가리켰어.", pair.second),
            format!("남은 {}에는 조치를 하지 마.", pair.first),
        ],
        (8, EvaluationLanguageIR::English, 0) => [
            format!(
                "Investigate the {} and the {} together.",
                pair.first, pair.second
            ),
            "Repair the thing I just pointed to.".to_string(),
            format!("I intended the second one, the {}.", pair.second),
            format!("Exclude the first {} from the repair scope.", pair.first),
        ],
        (8, EvaluationLanguageIR::English, _) => [
            format!(
                "Check the {} separately from the {}.",
                pair.first, pair.second
            ),
            "Choose that one as the item to fix.".to_string(),
            format!("By that one, I was referring to the later {}.", pair.second),
            format!("Take no action on the other {}.", pair.first),
        ],
        (9, EvaluationLanguageIR::Korean, 0) => [
            format!("어... {} 좀 chcek 해줄 수 있나?", pair.first),
            format!("아 잠깐, {} 아니고 {} 말한 거야.", pair.first, pair.second),
            format!(
                "그래서 {} 건은 진짜 끝난 거야, plan만 잡힌 거야?",
                pair.second
            ),
            format!("응응 고마워, {} 얘기는 여기까지", pair.second),
        ],
        (9, EvaluationLanguageIR::Korean, _) => [
            format!("음 저기, {} 상태 한번 봐주라.", pair.first),
            format!("노노 target 바꿔서 {} 쪽을 봐.", pair.second),
            format!("그 {} result가 실제로 나온 건 아니지?", pair.second),
            format!("오케이 ㄱㅅ, {} 건은 됐어", pair.second),
        ],
        (9, EvaluationLanguageIR::English, 0) => [
            format!("Uh, can ya chek on the {} a sec?", pair.first),
            format!(
                "Wait, not the {}—I meant the {} 쪽.",
                pair.first, pair.second
            ),
            format!(
                "So did the {} thing really finish, or is it jus planned?",
                pair.second
            ),
            format!("yeah, thanks, that's all on the {}", pair.second),
        ],
        (9, EvaluationLanguageIR::English, _) => [
            format!("Erm... could you take a quik look at the {}?", pair.first),
            format!("Nah, change the target to the {} instead.", pair.second),
            format!(
                "Do we have an actual {} result, or only a proposed step?",
                pair.second
            ),
            format!("okay ty, we can drop the {} topic", pair.second),
        ],
        _ => unreachable!("ten categories, two variants, two languages"),
    }
}

fn semantic_turns(category: usize, inputs: [String; 4], pair: &ResourcePair) -> Vec<TurnSpec> {
    let [one, two, three, four] = inputs;
    match category {
        0 => vec![
            TurnSpec::plan(one, "INVESTIGATE", "INVESTIGATE", pair.first_concept),
            TurnSpec::plan(two, "EXPLAIN", "EXPLAIN", pair.first_concept)
                .with_binding("PRONOMINAL", format!("ENTITY:{}", pair.first_concept)),
            TurnSpec::result_absence(three).with_binding("RESULT", "GOAL:INVESTIGATE".to_string()),
            TurnSpec::plan(four, "INVESTIGATE", "INVESTIGATE", pair.first_concept)
                .with_binding("ELLIPSIS", format!("ENTITY:{}", pair.first_concept)),
        ],
        1 => vec![
            TurnSpec::clarification(one),
            TurnSpec::plan(two, "INVESTIGATE", "INVESTIGATE", pair.first_concept)
                .with_binding("ELLIPSIS", format!("ENTITY:{}", pair.first_concept)),
            TurnSpec::interaction(three, "INFORM_ACKNOWLEDGEMENT")
                .with_binding("PRONOMINAL", format!("ENTITY:{}", pair.first_concept)),
            TurnSpec::result_absence(four),
        ],
        2 => vec![
            TurnSpec::plan(one, "REPAIR", "REPAIR", pair.first_concept).with_action(
                "REPAIR",
                "REPAIR",
                pair.second_concept,
            ),
            TurnSpec::plan(two, "EXPLAIN", "EXPLAIN", pair.first_concept)
                .with_binding("CORRECTION", format!("ENTITY:{}", pair.first_concept)),
            TurnSpec::plan(three, "EXPLAIN", "EXPLAIN", pair.second_concept)
                .with_binding("CORRECTION", format!("ENTITY:{}", pair.second_concept)),
            TurnSpec::result_absence(four),
        ],
        3 => vec![
            TurnSpec::plan(one, "INVESTIGATE", "INVESTIGATE", pair.first_concept).with_action(
                "INVESTIGATE",
                "INVESTIGATE",
                pair.second_concept,
            ),
            TurnSpec::plan(two, "EXPLAIN", "EXPLAIN", pair.second_concept)
                .with_binding("ORDINAL", format!("ENTITY:{}", pair.second_concept)),
            TurnSpec::plan(three, "EXPLAIN", "EXPLAIN", pair.first_concept)
                .with_binding("ORDINAL", format!("ENTITY:{}", pair.first_concept))
                .with_binding("ELLIPSIS", format!("ENTITY:{}", pair.first_concept)),
            TurnSpec::interaction(four, "INFORM_ACKNOWLEDGEMENT").with_binding(
                "PLURAL",
                format!(
                    "ENTITY:{}+ENTITY:{}",
                    pair.first_concept, pair.second_concept
                ),
            ),
        ],
        4 => vec![
            TurnSpec::plan(one, "INVESTIGATE", "INVESTIGATE", pair.first_concept),
            TurnSpec::plan(two, "EXPLAIN", "EXPLAIN", pair.second_concept),
            TurnSpec::interaction(three, "TOPIC_TRANSITION")
                .with_goal("ACKNOWLEDGE")
                .with_epistemic("DERIVED")
                .with_propositions(&["CLAIM:DISCOURSE_TOPIC_TRANSITION:DERIVED"]),
            TurnSpec::result_absence(four)
                .with_binding("TOPIC", format!("ENTITY:{}", pair.first_concept)),
        ],
        5 => vec![
            TurnSpec::interaction(one, "ACTION_STATE")
                .with_goal("ANSWER_VERIFIED_RESULT")
                .with_epistemic("REPORTED")
                .with_propositions(&[
                    "CLAIM:LANGUAGE_REPORT:REPORTED",
                    "CLAIM:EVIDENCE_ABSENCE:UNKNOWN",
                ]),
            TurnSpec::interaction(two, "DISCOURSE_ANSWER")
                .with_goal("ANSWER_VERIFIED_RESULT")
                .with_epistemic("DERIVED")
                .with_propositions(&["CLAIM:ATTRIBUTED_DIALOGUE_RECORD:DERIVED"]),
            TurnSpec::interaction(three, "INFORM_ACKNOWLEDGEMENT"),
            TurnSpec::result_absence(four),
        ],
        6 => vec![
            TurnSpec::plan(one, "REPAIR", "REPAIR", pair.first_concept),
            TurnSpec::result_absence(two),
            TurnSpec::interaction(three, "ACTION_STATE")
                .with_goal("ANSWER_VERIFIED_RESULT")
                .with_epistemic("REPORTED")
                .with_propositions(&[
                    "CLAIM:LANGUAGE_REPORT:REPORTED",
                    "CLAIM:EVIDENCE_ABSENCE:UNKNOWN",
                ]),
            TurnSpec::interaction(four, "PLAN_RESULT_STATUS")
                .with_goal("ANSWER_VERIFIED_RESULT")
                .with_epistemic("REPORTED")
                .with_propositions(&[
                    "CLAIM:LANGUAGE_REPORT:REPORTED",
                    "CLAIM:EVIDENCE_ABSENCE:UNKNOWN",
                ]),
        ],
        7 => vec![
            TurnSpec::interaction(one, "AFFECT_SUPPORT"),
            TurnSpec::interaction(two, "SOCIAL_BACKCHANNEL"),
            TurnSpec::plan(three, "INVESTIGATE", "INVESTIGATE", pair.first_concept),
            TurnSpec::interaction(four, "INFORM_ACKNOWLEDGEMENT"),
        ],
        8 => vec![
            TurnSpec::plan(one, "INVESTIGATE", "INVESTIGATE", pair.first_concept).with_action(
                "INVESTIGATE",
                "INVESTIGATE",
                pair.second_concept,
            ),
            TurnSpec::clarification(two),
            TurnSpec::plan(three, "REPAIR", "REPAIR", pair.second_concept)
                .with_binding("CLARIFICATION", format!("ENTITY:{}", pair.second_concept)),
            TurnSpec::interaction(four, "INFORM_ACKNOWLEDGEMENT")
                .with_binding("CORRECTION", format!("ENTITY:{}", pair.first_concept)),
        ],
        9 => vec![
            TurnSpec::plan(one, "INVESTIGATE", "INVESTIGATE", pair.first_concept)
                .with_modality(ConversationInputModalityIR::VoiceTranscript),
            TurnSpec::plan(two, "INVESTIGATE", "INVESTIGATE", pair.second_concept)
                .with_binding("CORRECTION", format!("ENTITY:{}", pair.second_concept)),
            TurnSpec::result_absence(three).with_binding("RESULT", "GOAL:INVESTIGATE".to_string()),
            TurnSpec::interaction(four, "SOCIAL_BACKCHANNEL"),
        ],
        _ => unreachable!("ten categories"),
    }
}

fn build_final(campaign: Campaign) -> Result<(BenchmarkInputSuiteIR, ReferenceSuiteIR), String> {
    let (suite_id, dialogue_prefix, context_tag) = match campaign {
        Campaign::V1 => (
            V1_SUITE_ID,
            "GPTREF-FINAL",
            "GPT_REFERENCE_V1_FINAL_SEALED_INPUT",
        ),
        Campaign::V2 => (
            V2_SUITE_ID,
            "GPTREF-V2-FINAL",
            "GPT_REFERENCE_V2_FINAL_SEALED_INPUT",
        ),
        Campaign::V3 => (
            V3_SUITE_ID,
            "GPTREF-V3-FINAL",
            "GPT_REFERENCE_V3_FINAL_SEALED_INPUT",
        ),
    };
    let mut input_turns = Vec::new();
    let mut annotations = Vec::new();
    for (category_index, category) in CATEGORIES.iter().enumerate() {
        for dialogue_index in 0..4 {
            let language = if dialogue_index < 2 {
                EvaluationLanguageIR::Korean
            } else {
                EvaluationLanguageIR::English
            };
            let variant = dialogue_index % 2;
            let language_tag = match language {
                EvaluationLanguageIR::Korean => "KO",
                EvaluationLanguageIR::English => "EN",
            };
            let dialogue_id = format!(
                "{dialogue_prefix}-C{:02}-{language_tag}-{:02}",
                category_index + 1,
                variant + 1
            );
            let pair = resource_pair(campaign, category_index, language, variant);
            let inputs = match campaign {
                Campaign::V1 => v1_inputs(category_index, language, variant, &pair),
                Campaign::V2 => v2_inputs(category_index, language, variant, &pair),
                Campaign::V3 => v3_inputs(category_index, language, variant, &pair),
            };
            let turns = semantic_turns(category_index, inputs, &pair);
            for (turn_offset, mut turn) in turns.into_iter().enumerate() {
                turn.meaning_atoms.sort();
                turn.meaning_atoms.dedup();
                turn.discourse_bindings.sort();
                turn.discourse_bindings.dedup();
                turn.required_propositions.sort();
                turn.required_propositions.dedup();
                turn.prohibited_propositions.sort();
                turn.prohibited_propositions.dedup();
                let turn_index = turn_offset as u8 + 1;
                let response_id = format!("{dialogue_id}-T{turn_index}");
                input_turns.push(BenchmarkInputTurnIR {
                    response_id: response_id.clone(),
                    dialogue_id: dialogue_id.clone(),
                    turn_index,
                    category: (*category).to_string(),
                    language,
                    modality: turn.modality,
                    raw_text: turn.input,
                    input_confidence_millis: 1_000,
                    alternatives: Vec::new(),
                    context_tags: vec![context_tag.to_string()],
                    max_plan_steps: 12,
                });
                let pending_surface = format!("PENDING_GPT_REFERENCE:{response_id}");
                annotations.push(ReferenceTurnAnnotationIR {
                    response_id,
                    dialogue_id: dialogue_id.clone(),
                    turn_index,
                    category: (*category).to_string(),
                    language,
                    response_act: turn.response_act.to_string(),
                    response_goal: turn.response_goal.to_string(),
                    epistemic_status: turn.epistemic_status.to_string(),
                    meaning_atoms: turn.meaning_atoms,
                    discourse_bindings: turn.discourse_bindings,
                    required_propositions: turn.required_propositions,
                    prohibited_propositions: turn.prohibited_propositions,
                    raw_reference_sha256: sha256_text(&pending_surface),
                    reference_surface: pending_surface,
                    calibrated_reference_surfaces: Vec::new(),
                    critical_boundary: turn.critical_boundary,
                    ambiguity_requires_clarification: turn.ambiguity_requires_clarification,
                });
            }
        }
    }
    let mut input = BenchmarkInputSuiteIR {
        schema: INPUT_SUITE_SCHEMA.to_string(),
        suite_id: suite_id.to_string(),
        split: SuiteSplitIR::Final,
        frozen: true,
        turns: input_turns,
        suite_payload_sha256: String::new(),
    };
    input.seal()?;
    input.validate()?;
    let draft = ReferenceSuiteIR {
        schema: REFERENCE_SUITE_SCHEMA.to_string(),
        suite_id: suite_id.to_string(),
        split: SuiteSplitIR::Final,
        frozen: false,
        reference_model_id: PENDING_REFERENCE_MODEL.to_string(),
        reference_generation_date: "2026-09-03".to_string(),
        reference_system_prompt_sha256: sha256_text(FINAL_SYSTEM_PROMPT),
        generation_configuration_sha256: sha256_text(FINAL_GENERATION_CONFIGURATION),
        input_suite_sha256: input.suite_payload_sha256.clone(),
        responses: annotations,
        suite_payload_sha256: String::new(),
    };
    validate_final_reference_draft(&input, &draft)?;
    Ok((input, draft))
}

fn normalized_surface(text: &str) -> String {
    text.split_whitespace()
        .flat_map(|token| token.chars())
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn validate_no_prompt_contamination(
    development: &BenchmarkInputSuiteIR,
    prior_finals: &[BenchmarkInputSuiteIR],
    final_input: &BenchmarkInputSuiteIR,
) -> Result<(), String> {
    development.validate()?;
    if development.split != SuiteSplitIR::Development {
        return Err("DEVELOPMENT_COMPARISON_SUITE_REQUIRED".to_string());
    }
    for prior_final in prior_finals {
        prior_final.validate()?;
        if prior_final.split != SuiteSplitIR::Final || !prior_final.frozen {
            return Err("PRIOR_FINAL_COMPARISON_SUITE_INVALID".to_string());
        }
    }
    let comparison_surfaces = development
        .turns
        .iter()
        .chain(prior_finals.iter().flat_map(|suite| suite.turns.iter()))
        .map(|turn| normalized_surface(&turn.raw_text))
        .collect::<BTreeSet<_>>();
    let mut final_surfaces = BTreeSet::new();
    for turn in &final_input.turns {
        let normalized = normalized_surface(&turn.raw_text);
        if !final_surfaces.insert(normalized.clone()) {
            return Err(format!("DUPLICATE_FINAL_PROMPT: {}", turn.response_id));
        }
        if comparison_surfaces.contains(&normalized) {
            return Err(format!("COMPARISON_PROMPT_REUSED: {}", turn.response_id));
        }
    }
    Ok(())
}

fn build_input_audit(
    development: &BenchmarkInputSuiteIR,
    prior_finals: &[BenchmarkInputSuiteIR],
    final_input: &BenchmarkInputSuiteIR,
) -> Result<FinalInputAuditIR, String> {
    let comparison_turns = development
        .turns
        .iter()
        .map(|turn| (development.suite_id.as_str(), turn))
        .chain(prior_finals.iter().flat_map(|suite| {
            suite
                .turns
                .iter()
                .map(|turn| (suite.suite_id.as_str(), turn))
        }))
        .collect::<Vec<_>>();
    let mut overlap_rows = final_input
        .turns
        .iter()
        .map(|final_turn| {
            let ((comparison_suite_id, development_turn), similarity_bp) = comparison_turns
                .iter()
                .map(|(suite_id, development_turn)| {
                    (
                        (*suite_id, *development_turn),
                        surface_similarity_bp(&development_turn.raw_text, &final_turn.raw_text),
                    )
                })
                .max_by_key(|(_, score)| *score)
                .expect("validated development suite is non-empty");
            PromptOverlapRowIR {
                final_response_id: final_turn.response_id.clone(),
                nearest_development_response_id: development_turn.response_id.clone(),
                nearest_comparison_suite_id: (!prior_finals.is_empty())
                    .then(|| comparison_suite_id.to_string()),
                similarity_bp,
            }
        })
        .collect::<Vec<_>>();
    overlap_rows.sort_by(|left, right| {
        right
            .similarity_bp
            .cmp(&left.similarity_bp)
            .then_with(|| left.final_response_id.cmp(&right.final_response_id))
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
    let maximum = *scores.last().expect("validated final suite is non-empty");
    let mut category_response_counts = BTreeMap::new();
    let mut language_response_counts = BTreeMap::new();
    for turn in &final_input.turns {
        *category_response_counts
            .entry(turn.category.clone())
            .or_insert(0) += 1;
        *language_response_counts.entry(turn.language).or_insert(0) += 1;
    }
    let mut audit = FinalInputAuditIR {
        schema: "B_CORE_GPT_REFERENCE_FINAL_INPUT_AUDIT_1",
        suite_id: final_input.suite_id.clone(),
        final_input_sha256: final_input.suite_payload_sha256.clone(),
        development_input_sha256: development.suite_payload_sha256.clone(),
        prior_final_input_sha256: prior_finals
            .last()
            .map(|suite| suite.suite_payload_sha256.clone()),
        prior_final_input_sha256s: prior_finals
            .iter()
            .map(|suite| suite.suite_payload_sha256.clone())
            .collect(),
        dialogues: final_input
            .turns
            .iter()
            .map(|turn| turn.dialogue_id.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        responses: final_input.turns.len(),
        category_response_counts,
        language_response_counts,
        duplicate_final_prompts: 0,
        exact_development_prompt_reuse: 0,
        mean_nearest_development_similarity_bp: mean,
        percentile_95_nearest_development_similarity_bp: percentile_95,
        maximum_nearest_development_similarity_bp: maximum,
        overlap_rows,
        b_core_evaluations: 0,
        external_llm_calls: 0,
        audit_sha256: String::new(),
    };
    audit.audit_sha256 = sha256_text(
        &serde_json::to_string(&audit)
            .map_err(|error| format!("FINAL_INPUT_AUDIT_HASH_SERIALIZATION_FAILED:{error}"))?,
    );
    Ok(audit)
}

fn find_workspace_root(start: &Path) -> Result<PathBuf, String> {
    for candidate in start.ancestors() {
        let manifest = candidate.join("Cargo.toml");
        if manifest.is_file() {
            let text = fs::read_to_string(&manifest)
                .map_err(|error| format!("WORKSPACE_MANIFEST_READ_FAILED:{error}"))?;
            if text.contains("[workspace]") {
                return candidate
                    .canonicalize()
                    .map_err(|error| format!("WORKSPACE_CANONICALIZATION_FAILED:{error}"));
            }
        }
    }
    Err("WORKSPACE_ROOT_NOT_FOUND".to_string())
}

fn require_reports_output(workspace: &Path, output: &Path) -> Result<(), String> {
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
    Ok(())
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if !matches!(arguments.len(), 4..=6) {
        return Err(USAGE.to_string());
    }
    let campaign = match arguments.len() {
        4 => Campaign::V1,
        5 => Campaign::V2,
        6 => Campaign::V3,
        _ => unreachable!("argument count validated"),
    };
    let prior_final_count = match campaign {
        Campaign::V1 => 0,
        Campaign::V2 => 1,
        Campaign::V3 => 2,
    };
    let output_offset = prior_final_count;
    let workspace = find_workspace_root(
        &env::current_dir().map_err(|error| format!("CURRENT_DIRECTORY_UNAVAILABLE:{error}"))?,
    )?;
    require_reports_output(&workspace, Path::new(&arguments[1 + output_offset]))?;
    require_reports_output(&workspace, Path::new(&arguments[2 + output_offset]))?;
    require_reports_output(&workspace, Path::new(&arguments[3 + output_offset]))?;
    let development: BenchmarkInputSuiteIR = serde_json::from_slice(
        &fs::read(&arguments[0])
            .map_err(|error| format!("DEVELOPMENT_INPUT_READ_FAILED:{error}"))?,
    )
    .map_err(|error| format!("DEVELOPMENT_INPUT_JSON_INVALID:{error}"))?;
    let prior_finals = arguments[1..1 + prior_final_count]
        .iter()
        .map(|path| {
            serde_json::from_slice::<BenchmarkInputSuiteIR>(
                &fs::read(path)
                    .map_err(|error| format!("PRIOR_FINAL_INPUT_READ_FAILED:{error}"))?,
            )
            .map_err(|error| format!("PRIOR_FINAL_INPUT_JSON_INVALID:{error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (final_input, draft) = build_final(campaign)?;
    validate_no_prompt_contamination(&development, &prior_finals, &final_input)?;
    let audit = build_input_audit(&development, &prior_finals, &final_input)?;
    let input_payload = serde_json::to_string_pretty(&final_input)
        .map_err(|error| format!("FINAL_INPUT_SERIALIZATION_FAILED:{error}"))?;
    let draft_payload = serde_json::to_string_pretty(&draft)
        .map_err(|error| format!("FINAL_DRAFT_SERIALIZATION_FAILED:{error}"))?;
    let audit_payload = serde_json::to_string_pretty(&audit)
        .map_err(|error| format!("FINAL_INPUT_AUDIT_SERIALIZATION_FAILED:{error}"))?;
    fs::write(&arguments[1 + output_offset], format!("{input_payload}\n"))
        .map_err(|error| format!("FINAL_INPUT_WRITE_FAILED:{error}"))?;
    fs::write(&arguments[2 + output_offset], format!("{draft_payload}\n"))
        .map_err(|error| format!("FINAL_DRAFT_WRITE_FAILED:{error}"))?;
    fs::write(&arguments[3 + output_offset], format!("{audit_payload}\n"))
        .map_err(|error| format!("FINAL_INPUT_AUDIT_WRITE_FAILED:{error}"))?;
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}
