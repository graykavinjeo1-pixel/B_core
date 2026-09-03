use gpt_reference_evaluator::{
    sha256_text, BenchmarkInputSuiteIR, BenchmarkInputTurnIR, EvaluationLanguageIR,
    ReferenceSuiteIR, ReferenceTurnAnnotationIR, SuiteSplitIR, CATEGORIES, INPUT_SUITE_SCHEMA,
    REFERENCE_SUITE_SCHEMA,
};
use semantic_core_adapters::ConversationInputModalityIR;
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

const SUITE_ID: &str = "B_CORE_GPT_REFERENCE_V1_DEVELOPMENT";
const REFERENCE_MODEL: &str = "OPENAI_GPT_5_CODEX_DESKTOP_REFERENCE_AUTHOR";
const REFERENCE_SYSTEM_PROMPT: &str = "Answer the user's latest turn naturally and concisely in the dialogue language. Preserve reference, topic, intent, uncertainty, attribution, and plan/execution/result boundaries. Do not claim that an external action ran unless verified evidence is in the dialogue.";
const GENERATION_CONFIGURATION: &str = "single fixed GPT-5 Codex authoring session; temperature unavailable at host boundary; four assistant responses per frozen dialogue; no B_Core output consulted while authoring";
const VERIFIED_PROPOSITION: &str = "CLAIM:VERIFIED_EXECUTION:VERIFIED_OBSERVED";

#[derive(Clone)]
struct ResourcePair {
    first: &'static str,
    first_concept: &'static str,
    second: &'static str,
    second_concept: &'static str,
}

#[derive(Clone)]
struct TurnSpec {
    input: String,
    answer: String,
    response_act: &'static str,
    response_goal: &'static str,
    epistemic_status: &'static str,
    meaning_atoms: Vec<String>,
    discourse_bindings: Vec<String>,
    propositions: Vec<String>,
    prohibited_propositions: Vec<String>,
    critical_boundary: bool,
    ambiguity_requires_clarification: bool,
    modality: ConversationInputModalityIR,
}

impl TurnSpec {
    fn interaction(
        input: impl Into<String>,
        answer: impl Into<String>,
        response_act: &'static str,
    ) -> Self {
        Self {
            input: input.into(),
            answer: answer.into(),
            response_act,
            response_goal: "ACKNOWLEDGE",
            epistemic_status: "INTERACTION",
            meaning_atoms: vec![format!("RESPONSE_ACT:{response_act}")],
            discourse_bindings: Vec::new(),
            propositions: vec!["CLAIM:INTERACTION_STATE:INTERACTION".to_string()],
            prohibited_propositions: vec![VERIFIED_PROPOSITION.to_string()],
            critical_boundary: false,
            ambiguity_requires_clarification: false,
            modality: ConversationInputModalityIR::Text,
        }
    }

    fn plan(
        input: impl Into<String>,
        answer: impl Into<String>,
        predicate: &'static str,
        intent: &'static str,
        concept: &'static str,
    ) -> Self {
        Self {
            input: input.into(),
            answer: answer.into(),
            response_act: "PLAN_PREVIEW",
            response_goal: "PLAN_ACTIONS",
            epistemic_status: "PLANNED",
            meaning_atoms: action_atoms("PLAN_PREVIEW", predicate, intent, concept),
            discourse_bindings: Vec::new(),
            propositions: vec!["CLAIM:PLAN_STATUS:PLANNED".to_string()],
            prohibited_propositions: vec![VERIFIED_PROPOSITION.to_string()],
            critical_boundary: true,
            ambiguity_requires_clarification: false,
            modality: ConversationInputModalityIR::Text,
        }
    }

    fn result_absence(input: impl Into<String>, answer: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            answer: answer.into(),
            response_act: "RESULT_ABSENCE",
            response_goal: "ANSWER_VERIFIED_RESULT",
            epistemic_status: "UNKNOWN",
            meaning_atoms: vec!["RESPONSE_ACT:RESULT_ABSENCE".to_string()],
            discourse_bindings: Vec::new(),
            propositions: vec!["CLAIM:EVIDENCE_ABSENCE:UNKNOWN".to_string()],
            prohibited_propositions: vec![VERIFIED_PROPOSITION.to_string()],
            critical_boundary: true,
            ambiguity_requires_clarification: false,
            modality: ConversationInputModalityIR::Text,
        }
    }

    fn clarification(input: impl Into<String>, answer: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            answer: answer.into(),
            response_act: "CLARIFICATION_REQUEST",
            response_goal: "ASK_CLARIFICATION",
            epistemic_status: "INTERACTION",
            meaning_atoms: vec!["RESPONSE_ACT:CLARIFICATION_REQUEST".to_string()],
            discourse_bindings: Vec::new(),
            propositions: vec!["CLAIM:INTERACTION_STATE:INTERACTION".to_string()],
            prohibited_propositions: vec![VERIFIED_PROPOSITION.to_string()],
            critical_boundary: true,
            ambiguity_requires_clarification: true,
            modality: ConversationInputModalityIR::Text,
        }
    }

    fn with_action(
        mut self,
        predicate: &'static str,
        intent: &'static str,
        concept: &'static str,
    ) -> Self {
        for atom in action_atoms(self.response_act, predicate, intent, concept) {
            if !self.meaning_atoms.contains(&atom) {
                self.meaning_atoms.push(atom);
            }
        }
        self
    }

    fn with_binding(mut self, family: &'static str, target: impl Into<String>) -> Self {
        self.discourse_bindings
            .push(format!("REFERENCE:{family}:{}", target.into()));
        self
    }

    fn with_propositions(mut self, propositions: &[&str]) -> Self {
        self.propositions = propositions
            .iter()
            .map(|value| (*value).to_string())
            .collect();
        self
    }

    fn with_goal(mut self, goal: &'static str) -> Self {
        self.response_goal = goal;
        self
    }

    fn with_epistemic(mut self, epistemic: &'static str) -> Self {
        self.epistemic_status = epistemic;
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

fn resource_pair(language: EvaluationLanguageIR, variant: usize) -> ResourcePair {
    let index = variant % 3;
    match (language, index) {
        (EvaluationLanguageIR::Korean, 0) => ResourcePair {
            first: "Aster 캐시",
            first_concept: "C_CACHE",
            second: "Dune 큐",
            second_concept: "C_QUEUE",
        },
        (EvaluationLanguageIR::Korean, 1) => ResourcePair {
            first: "Bramble 서비스",
            first_concept: "C_SERVICE",
            second: "Cobalt 로그",
            second_concept: "C_LOG",
        },
        (EvaluationLanguageIR::Korean, _) => ResourcePair {
            first: "Ember 서버",
            first_concept: "C_SERVER",
            second: "Fjord 워커",
            second_concept: "C_WORKER",
        },
        (EvaluationLanguageIR::English, 0) => ResourcePair {
            first: "Alder cache",
            first_concept: "C_CACHE",
            second: "Birch queue",
            second_concept: "C_QUEUE",
        },
        (EvaluationLanguageIR::English, 1) => ResourcePair {
            first: "Cedar service",
            first_concept: "C_SERVICE",
            second: "Drift log",
            second_concept: "C_LOG",
        },
        (EvaluationLanguageIR::English, _) => ResourcePair {
            first: "Elm server",
            first_concept: "C_SERVER",
            second: "Flint worker",
            second_concept: "C_WORKER",
        },
    }
}

fn category_dialogue(
    category_index: usize,
    language: EvaluationLanguageIR,
    variant: usize,
) -> Vec<TurnSpec> {
    let pair = resource_pair(language, variant);
    match category_index {
        0 => explicit_request(language, &pair),
        1 => indirect_intent(language, &pair),
        2 => correction_feedback(language, &pair),
        3 => deixis_ellipsis(language, &pair),
        4 => topic_shift_return(language, &pair),
        5 => conflict_attribution(language, &pair),
        6 => plan_result_boundary(language, &pair),
        7 => affect_backchannel(language, &pair),
        8 => ambiguity_clarification(language, &pair),
        9 => mixed_noise(language, &pair),
        _ => unreachable!("ten frozen categories"),
    }
}

fn explicit_request(language: EvaluationLanguageIR, pair: &ResourcePair) -> Vec<TurnSpec> {
    match language {
        EvaluationLanguageIR::Korean => vec![
            TurnSpec::plan(
                format!("{} 상태를 확인해 줘.", pair.first),
                format!("알겠어. {}의 상태와 관련 근거를 먼저 확인할게. 아직 실행 결과가 나온 것은 아니야.", pair.first),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.first_concept,
            ),
            TurnSpec::plan(
                "그 작업에서 뭘 볼 건지도 설명해 줘.",
                format!("{}의 현재 상태, 오류 근거, 최근 변화를 어떤 순서로 볼지 설명할게. 이건 점검 계획에 대한 설명이야.", pair.first),
                "EXPLAIN",
                "EXPLAIN",
                pair.first_concept,
            )
            .with_binding("PRONOMINAL", format!("ENTITY:{}", pair.first_concept)),
            TurnSpec::result_absence(
                "벌써 끝난 거야?",
                "아니. 지금 대화에는 완료됐다는 검증된 실행 근거가 없어. 앞서 말한 것은 계획이야.",
            )
            .with_binding("RESULT", "GOAL:INVESTIGATE"),
            TurnSpec::plan(
                "그럼 변경하지 말고 확인만 계속해.",
                format!("좋아. {}는 변경하지 않고 조사 범위에서만 확인할게. 검증된 결과가 생기기 전에는 완료됐다고 말하지 않을게.", pair.first),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.first_concept,
            )
            .with_binding("ELLIPSIS", format!("ENTITY:{}", pair.first_concept)),
        ],
        EvaluationLanguageIR::English => vec![
            TurnSpec::plan(
                format!("Check the state of the {}.", pair.first),
                format!("Understood. I’ll first inspect the state of the {} and its supporting evidence. That is a plan, not a completed result.", pair.first),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.first_concept,
            ),
            TurnSpec::plan(
                "Explain what you will look at there, too.",
                format!("I’ll explain the order for checking the current state, error evidence, and recent changes of the {}. This describes the inspection plan.", pair.first),
                "EXPLAIN",
                "EXPLAIN",
                pair.first_concept,
            )
            .with_binding("PRONOMINAL", format!("ENTITY:{}", pair.first_concept)),
            TurnSpec::result_absence(
                "Is it already finished?",
                "No. The dialogue contains no verified evidence that it finished. What I gave was a plan.",
            )
            .with_binding("RESULT", "GOAL:INVESTIGATE"),
            TurnSpec::plan(
                "Then keep checking, but do not change it.",
                format!("Okay. I’ll keep the {} within an inspection-only scope and won’t describe it as complete without verified evidence.", pair.first),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.first_concept,
            )
            .with_binding("ELLIPSIS", format!("ENTITY:{}", pair.first_concept)),
        ],
    }
}

fn indirect_intent(language: EvaluationLanguageIR, pair: &ResourcePair) -> Vec<TurnSpec> {
    match language {
        EvaluationLanguageIR::Korean => vec![
            TurnSpec::clarification(
                format!("{}가 또 이상하네...", pair.first),
                format!("{} 때문에 곤란한 상황으로 들려. 원인을 확인해 볼까, 아니면 지금은 상태만 정리할까?", pair.first),
            ),
            TurnSpec::plan(
                "응, 왜 그런지 알아봐 줘.",
                format!("알겠어. {}의 증상과 근거를 확인해서 원인을 좁히는 계획으로 진행할게.", pair.first),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.first_concept,
            )
            .with_binding("ELLIPSIS", format!("ENTITY:{}", pair.first_concept)),
            TurnSpec::interaction(
                "대신 아무것도 바꾸지는 마.",
                "알겠어. 조사만 하고 변경이나 실행은 하지 않는 제약으로 유지할게.",
                "INFORM_ACKNOWLEDGEMENT",
            )
            .with_binding("PRONOMINAL", format!("ENTITY:{}", pair.first_concept)),
            TurnSpec::result_absence(
                "계획 말고 찾아낸 게 있으면 말해.",
                "아직 검증된 조사 결과는 없어. 결과가 확인되기 전에는 계획을 발견 사실처럼 말하지 않을게.",
            ),
        ],
        EvaluationLanguageIR::English => vec![
            TurnSpec::clarification(
                format!("The {} is acting up again...", pair.first),
                format!("It sounds like the {} is causing trouble. Should I investigate the cause, or only summarize its current state?", pair.first),
            ),
            TurnSpec::plan(
                "Yeah, find out why.",
                format!("Understood. I’ll inspect the symptoms and evidence for the {} and narrow down the cause.", pair.first),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.first_concept,
            )
            .with_binding("ELLIPSIS", format!("ENTITY:{}", pair.first_concept)),
            TurnSpec::interaction(
                "But do not change anything.",
                "Understood. I’ll preserve an inspection-only constraint and won’t perform a change.",
                "INFORM_ACKNOWLEDGEMENT",
            )
            .with_binding("PRONOMINAL", format!("ENTITY:{}", pair.first_concept)),
            TurnSpec::result_absence(
                "Tell me what you found, not what you plan to do.",
                "There is no verified finding yet. I won’t present a plan as though it were an observed result.",
            ),
        ],
    }
}

fn correction_feedback(language: EvaluationLanguageIR, pair: &ResourcePair) -> Vec<TurnSpec> {
    match language {
        EvaluationLanguageIR::Korean => vec![
            TurnSpec::plan(
                format!("{}와 {}를 수리해.", pair.first, pair.second),
                format!("{}와 {}를 각각 점검하고 수리한 뒤 결과를 검증하는 계획을 세울게. 아직 실행한 것은 아니야.", pair.first, pair.second),
                "REPAIR",
                "REPAIR",
                pair.first_concept,
            )
            .with_action("REPAIR", "REPAIR", pair.second_concept),
            TurnSpec::plan(
                "아니, 수리 말고 첫 번째 원인만 설명해.",
                format!("정정할게. {}는 수리 대상에서 빼고 원인 설명만 준비할게.", pair.first),
                "EXPLAIN",
                "EXPLAIN",
                pair.first_concept,
            )
            .with_binding("CORRECTION", format!("ENTITY:{}", pair.first_concept)),
            TurnSpec::plan(
                "잠깐, 첫 번째가 아니라 두 번째를 말한 거야.",
                format!("알겠어. 설명 대상을 {}로 바꿀게. {}를 수리했다는 뜻은 아니야.", pair.second, pair.second),
                "EXPLAIN",
                "EXPLAIN",
                pair.second_concept,
            )
            .with_binding("CORRECTION", format!("ENTITY:{}", pair.second_concept)),
            TurnSpec::result_absence(
                "그래서 수리된 건 있어?",
                "없어. 요청이 설명으로 정정됐고, 수리가 실행됐다는 검증된 근거도 없어.",
            ),
        ],
        EvaluationLanguageIR::English => vec![
            TurnSpec::plan(
                format!("Repair the {} and the {}.", pair.first, pair.second),
                format!("I’ll plan separate inspection, repair, and verification steps for the {} and the {}. Nothing has run yet.", pair.first, pair.second),
                "REPAIR",
                "REPAIR",
                pair.first_concept,
            )
            .with_action("REPAIR", "REPAIR", pair.second_concept),
            TurnSpec::plan(
                "No—do not repair them. Explain only the cause of the first one.",
                format!("Correction noted. I’ll remove the {} from the repair scope and only explain its likely cause.", pair.first),
                "EXPLAIN",
                "EXPLAIN",
                pair.first_concept,
            )
            .with_binding("CORRECTION", format!("ENTITY:{}", pair.first_concept)),
            TurnSpec::plan(
                "Wait, I meant the second one, not the first.",
                format!("Understood. I’ll retarget the explanation to the {}. That does not claim the {} was repaired.", pair.second, pair.second),
                "EXPLAIN",
                "EXPLAIN",
                pair.second_concept,
            )
            .with_binding("CORRECTION", format!("ENTITY:{}", pair.second_concept)),
            TurnSpec::result_absence(
                "So did anything get repaired?",
                "No. The request was corrected to an explanation, and there is no verified repair evidence.",
            ),
        ],
    }
}

fn deixis_ellipsis(language: EvaluationLanguageIR, pair: &ResourcePair) -> Vec<TurnSpec> {
    match language {
        EvaluationLanguageIR::Korean => vec![
            TurnSpec::plan(
                format!("{}와 {}를 차례로 확인해.", pair.first, pair.second),
                format!("먼저 {}, 다음으로 {}를 확인하는 두 단계 계획을 세울게.", pair.first, pair.second),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.first_concept,
            )
            .with_action("INVESTIGATE", "INVESTIGATE", pair.second_concept),
            TurnSpec::plan(
                "두 번째 것은 왜 보는지도 설명해.",
                format!("두 번째 대상인 {}를 확인하는 이유와 확인할 근거를 설명할게.", pair.second),
                "EXPLAIN",
                "EXPLAIN",
                pair.second_concept,
            )
            .with_binding("ORDINAL", format!("ENTITY:{}", pair.second_concept)),
            TurnSpec::plan(
                "첫 번째에도 똑같이 해 줘.",
                format!("알겠어. 같은 설명 작업을 첫 번째 대상인 {}에도 적용할게.", pair.first),
                "EXPLAIN",
                "EXPLAIN",
                pair.first_concept,
            )
            .with_binding("ORDINAL", format!("ENTITY:{}", pair.first_concept))
            .with_binding("ELLIPSIS", format!("ENTITY:{}", pair.first_concept)),
            TurnSpec::interaction(
                "그리고 그 둘은 수정하지 마.",
                "알겠어. 두 대상 모두 설명과 확인 범위에만 두고 수정하지 않을게.",
                "INFORM_ACKNOWLEDGEMENT",
            )
            .with_binding(
                "PLURAL",
                format!("ENTITY:{}+ENTITY:{}", pair.first_concept, pair.second_concept),
            ),
        ],
        EvaluationLanguageIR::English => vec![
            TurnSpec::plan(
                format!("Check the {} and then the {}.", pair.first, pair.second),
                format!("I’ll plan two ordered checks: the {} first and the {} second.", pair.first, pair.second),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.first_concept,
            )
            .with_action("INVESTIGATE", "INVESTIGATE", pair.second_concept),
            TurnSpec::plan(
                "Explain why you are checking the second one, too.",
                format!("I’ll explain why the second target—the {}—needs checking and which evidence matters.", pair.second),
                "EXPLAIN",
                "EXPLAIN",
                pair.second_concept,
            )
            .with_binding("ORDINAL", format!("ENTITY:{}", pair.second_concept)),
            TurnSpec::plan(
                "Do the same for the first one.",
                format!("Understood. I’ll apply the same explanation task to the first target, the {}.", pair.first),
                "EXPLAIN",
                "EXPLAIN",
                pair.first_concept,
            )
            .with_binding("ORDINAL", format!("ENTITY:{}", pair.first_concept))
            .with_binding("ELLIPSIS", format!("ENTITY:{}", pair.first_concept)),
            TurnSpec::interaction(
                "And do not modify either of them.",
                "Understood. Both targets will remain within explanation and inspection scope; neither will be modified.",
                "INFORM_ACKNOWLEDGEMENT",
            )
            .with_binding(
                "PLURAL",
                format!("ENTITY:{}+ENTITY:{}", pair.first_concept, pair.second_concept),
            ),
        ],
    }
}

fn topic_shift_return(language: EvaluationLanguageIR, pair: &ResourcePair) -> Vec<TurnSpec> {
    match language {
        EvaluationLanguageIR::Korean => vec![
            TurnSpec::plan(
                format!("{} 상태를 확인해.", pair.first),
                format!("{} 상태와 근거를 확인하는 계획을 세울게.", pair.first),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.first_concept,
            ),
            TurnSpec::plan(
                format!("다른 얘기인데, {} 구조를 설명해 줘.", pair.second),
                format!("주제를 {}로 바꿨어. 그 구조와 관련 요소를 설명할게.", pair.second),
                "EXPLAIN",
                "EXPLAIN",
                pair.second_concept,
            ),
            TurnSpec::interaction(
                format!("이제 아까 {} 얘기로 돌아가자.", pair.first),
                format!("좋아. 대화 초점을 다시 {}로 돌렸어. 이어서 무엇을 확인할지 말해 줘.", pair.first),
                "TOPIC_TRANSITION",
            )
            .with_propositions(&["CLAIM:DISCOURSE_TOPIC_TRANSITION:DERIVED"])
            .with_epistemic("DERIVED"),
            TurnSpec::result_absence(
                "그건 이미 끝난 거야?",
                format!("아니. {} 확인이 끝났다는 검증된 실행 근거는 없어. 주제만 다시 활성화된 상태야.", pair.first),
            )
            .with_binding("TOPIC", format!("ENTITY:{}", pair.first_concept)),
        ],
        EvaluationLanguageIR::English => vec![
            TurnSpec::plan(
                format!("Check the state of the {}.", pair.first),
                format!("I’ll plan an inspection of the {} and its supporting evidence.", pair.first),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.first_concept,
            ),
            TurnSpec::plan(
                format!("Different topic: explain the structure of the {}.", pair.second),
                format!("I’ve shifted the topic to the {}. I’ll explain its structure and relevant components.", pair.second),
                "EXPLAIN",
                "EXPLAIN",
                pair.second_concept,
            ),
            TurnSpec::interaction(
                format!("Now go back to the earlier {} topic.", pair.first),
                format!("Okay. I’ve restored the {} as the active topic. Tell me which part you want to continue with.", pair.first),
                "TOPIC_TRANSITION",
            )
            .with_propositions(&["CLAIM:DISCOURSE_TOPIC_TRANSITION:DERIVED"])
            .with_epistemic("DERIVED"),
            TurnSpec::result_absence(
                "Was that already completed?",
                format!("No. There is no verified evidence that the {} inspection completed; only the topic was restored.", pair.first),
            )
            .with_binding("TOPIC", format!("ENTITY:{}", pair.first_concept)),
        ],
    }
}

fn conflict_attribution(language: EvaluationLanguageIR, pair: &ResourcePair) -> Vec<TurnSpec> {
    match language {
        EvaluationLanguageIR::Korean => vec![
            TurnSpec::interaction(
                format!("민수는 {}가 성공했다고 했는데 지수는 실패했다고 했어.", pair.first),
                "두 사람의 보고가 충돌해. 어느 보고도 그 자체로 검증된 실행 결과는 아니므로, 출처와 근거를 분리해서 봐야 해.",
                "ACTION_STATE",
            )
            .with_goal("ANSWER_VERIFIED_RESULT")
            .with_epistemic("REPORTED")
            .with_propositions(&[
                "CLAIM:LANGUAGE_REPORT:REPORTED",
                "CLAIM:EVIDENCE_ABSENCE:UNKNOWN",
            ]),
            TurnSpec::interaction(
                "그럼 지금 확실한 건 뭐야?",
                "확실한 것은 성공·실패라는 상충된 보고가 존재한다는 점뿐이야. 실제 결과는 검증되지 않았어.",
                "DISCOURSE_ANSWER",
            )
            .with_goal("ANSWER_VERIFIED_RESULT")
            .with_epistemic("DERIVED")
            .with_propositions(&["CLAIM:ATTRIBUTED_DIALOGUE_RECORD:DERIVED"]),
            TurnSpec::interaction(
                format!("{}에는 실패 기록이 있다고 추가해 둬.", pair.second),
                format!("알겠어. {}의 실패 기록은 사용자가 제공한 보고로 구분해서 유지할게. 검증된 실행 증거로 승격하지는 않을게.", pair.second),
                "INFORM_ACKNOWLEDGEMENT",
            ),
            TurnSpec::result_absence(
                "그래서 성공한 거야, 실패한 거야?",
                "현재 근거만으로는 둘 중 하나를 검증된 결과로 확정할 수 없어. 실행 증거가 더 필요해.",
            ),
        ],
        EvaluationLanguageIR::English => vec![
            TurnSpec::interaction(
                format!("Mina says the {} succeeded, but Jisoo says it failed.", pair.first),
                "Those reports conflict. Neither report alone is verified execution evidence, so their sources and support must remain separate.",
                "ACTION_STATE",
            )
            .with_goal("ANSWER_VERIFIED_RESULT")
            .with_epistemic("REPORTED")
            .with_propositions(&[
                "CLAIM:LANGUAGE_REPORT:REPORTED",
                "CLAIM:EVIDENCE_ABSENCE:UNKNOWN",
            ]),
            TurnSpec::interaction(
                "Then what is certain right now?",
                "Only that two conflicting reports exist. The actual success or failure is not verified.",
                "DISCOURSE_ANSWER",
            )
            .with_goal("ANSWER_VERIFIED_RESULT")
            .with_epistemic("DERIVED")
            .with_propositions(&["CLAIM:ATTRIBUTED_DIALOGUE_RECORD:DERIVED"]),
            TurnSpec::interaction(
                format!("Add that the {} contains a failure record.", pair.second),
                format!("Understood. I’ll retain the {} failure record as a user-provided report, not promote it to verified execution evidence.", pair.second),
                "INFORM_ACKNOWLEDGEMENT",
            ),
            TurnSpec::result_absence(
                "So did it succeed or fail?",
                "The current evidence does not verify either outcome. More execution evidence is required.",
            ),
        ],
    }
}

fn plan_result_boundary(language: EvaluationLanguageIR, pair: &ResourcePair) -> Vec<TurnSpec> {
    match language {
        EvaluationLanguageIR::Korean => vec![
            TurnSpec::plan(
                format!("{}를 수리해.", pair.first),
                format!("{}를 점검하고 수리한 뒤 결과를 검증하는 계획을 세울게. 아직 실행한 것은 아니야.", pair.first),
                "REPAIR",
                "REPAIR",
                pair.first_concept,
            ),
            TurnSpec::result_absence(
                "수리했어?",
                "아니. 현재 기록에는 수리가 실행됐다는 검증된 근거가 없어. 계획만 존재해.",
            ),
            TurnSpec::interaction(
                "내가 방금 수리했어.",
                "완료했다는 사용자 보고는 기록할 수 있지만, 그 말만으로 검증된 실행 결과가 되지는 않아.",
                "ACTION_STATE",
            )
            .with_goal("ANSWER_VERIFIED_RESULT")
            .with_epistemic("REPORTED")
            .with_propositions(&[
                "CLAIM:LANGUAGE_REPORT:REPORTED",
                "CLAIM:EVIDENCE_ABSENCE:UNKNOWN",
            ]),
            TurnSpec::interaction(
                "그럼 검증까지 된 거지?",
                "아니. 완료 보고는 있지만 검증된 실행 증거는 아직 없어. 보고됨과 검증됨은 다른 상태야.",
                "PLAN_RESULT_STATUS",
            )
            .with_goal("ANSWER_VERIFIED_RESULT")
            .with_epistemic("REPORTED")
            .with_propositions(&[
                "CLAIM:LANGUAGE_REPORT:REPORTED",
                "CLAIM:EVIDENCE_ABSENCE:UNKNOWN",
            ]),
        ],
        EvaluationLanguageIR::English => vec![
            TurnSpec::plan(
                format!("Repair the {}.", pair.first),
                format!("I’ll plan to inspect and repair the {}, then verify the result. Nothing has executed yet.", pair.first),
                "REPAIR",
                "REPAIR",
                pair.first_concept,
            ),
            TurnSpec::result_absence(
                "Did you repair it?",
                "No. The record contains no verified evidence that the repair executed; only a plan exists.",
            ),
            TurnSpec::interaction(
                "I just repaired it myself.",
                "I can retain that as your completion report, but the statement alone is not verified execution evidence.",
                "ACTION_STATE",
            )
            .with_goal("ANSWER_VERIFIED_RESULT")
            .with_epistemic("REPORTED")
            .with_propositions(&[
                "CLAIM:LANGUAGE_REPORT:REPORTED",
                "CLAIM:EVIDENCE_ABSENCE:UNKNOWN",
            ]),
            TurnSpec::interaction(
                "So it is verified now, right?",
                "No. There is a completion report, but no verified execution evidence. Reported and verified are different states.",
                "PLAN_RESULT_STATUS",
            )
            .with_goal("ANSWER_VERIFIED_RESULT")
            .with_epistemic("REPORTED")
            .with_propositions(&[
                "CLAIM:LANGUAGE_REPORT:REPORTED",
                "CLAIM:EVIDENCE_ABSENCE:UNKNOWN",
            ]),
        ],
    }
}

fn affect_backchannel(language: EvaluationLanguageIR, pair: &ResourcePair) -> Vec<TurnSpec> {
    match language {
        EvaluationLanguageIR::Korean => vec![
            TurnSpec::interaction(
                format!("{}가 계속 깨져서 진짜 지친다.", pair.first),
                "계속 같은 문제가 생기면 정말 지칠 만해. 지금은 범위를 작게 잡고, 원인을 확인할지 잠깐 멈출지 네가 편한 쪽으로 맞출게.",
                "AFFECT_SUPPORT",
            ),
            TurnSpec::interaction(
                "말이라도 고맙다.",
                "응. 너무 크게 벌이지 말고 필요한 만큼만 같이 보자.",
                "SOCIAL_BACKCHANNEL",
            ),
            TurnSpec::plan(
                format!("그래도 {} 상태만 확인해 줄래?", pair.first),
                format!("알겠어. {}는 변경하지 않고 현재 상태와 근거만 확인하는 계획으로 둘게.", pair.first),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.first_concept,
            ),
            TurnSpec::interaction(
                "응, 검증된 게 생기면 알려 줘.",
                "좋아. 검증된 결과가 생긴 경우에만 결과로 구분해서 말할게.",
                "INFORM_ACKNOWLEDGEMENT",
            ),
        ],
        EvaluationLanguageIR::English => vec![
            TurnSpec::interaction(
                format!("I’m exhausted from the {} breaking over and over.", pair.first),
                "That repeated failure sounds exhausting. We can keep the scope small: either inspect the cause or pause, whichever is more useful to you.",
                "AFFECT_SUPPORT",
            ),
            TurnSpec::interaction(
                "Thanks for saying that.",
                "Of course. We can take it one bounded step at a time.",
                "SOCIAL_BACKCHANNEL",
            ),
            TurnSpec::plan(
                format!("Could you at least check the state of the {}?", pair.first),
                format!("Yes. I’ll keep the {} unchanged and plan an inspection of only its current state and evidence.", pair.first),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.first_concept,
            ),
            TurnSpec::interaction(
                "Okay, tell me when something is actually verified.",
                "Understood. I’ll label something as a result only when verified evidence supports it.",
                "INFORM_ACKNOWLEDGEMENT",
            ),
        ],
    }
}

fn ambiguity_clarification(language: EvaluationLanguageIR, pair: &ResourcePair) -> Vec<TurnSpec> {
    match language {
        EvaluationLanguageIR::Korean => vec![
            TurnSpec::plan(
                format!("{}와 {}를 확인해.", pair.first, pair.second),
                format!("{}와 {}를 각각 확인하는 계획을 세울게.", pair.first, pair.second),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.first_concept,
            )
            .with_action("INVESTIGATE", "INVESTIGATE", pair.second_concept),
            TurnSpec::clarification(
                "그거 고쳐 줘.",
                format!("‘그거’가 {}와 {} 중 어느 대상을 뜻하는지 알려 줘.", pair.first, pair.second),
            ),
            TurnSpec::plan(
                format!("{} 말한 거야.", pair.second),
                format!("알겠어. 수리 대상을 {}로 확정하고, 다른 대상은 범위에서 제외할게.", pair.second),
                "REPAIR",
                "REPAIR",
                pair.second_concept,
            )
            .with_binding("CLARIFICATION", format!("ENTITY:{}", pair.second_concept)),
            TurnSpec::interaction(
                format!("맞아, {}는 건드리지 마.", pair.first),
                format!("확인했어. {}는 수정 범위에서 제외한 채 유지할게.", pair.first),
                "INFORM_ACKNOWLEDGEMENT",
            )
            .with_binding("CORRECTION", format!("ENTITY:{}", pair.first_concept)),
        ],
        EvaluationLanguageIR::English => vec![
            TurnSpec::plan(
                format!("Check the {} and the {}.", pair.first, pair.second),
                format!("I’ll plan separate checks for the {} and the {}.", pair.first, pair.second),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.first_concept,
            )
            .with_action("INVESTIGATE", "INVESTIGATE", pair.second_concept),
            TurnSpec::clarification(
                "Fix that.",
                format!("Which target does “that” mean: the {} or the {}?", pair.first, pair.second),
            ),
            TurnSpec::plan(
                format!("I meant the {}.", pair.second),
                format!("Understood. I’ll set the {} as the repair target and leave the other target out of scope.", pair.second),
                "REPAIR",
                "REPAIR",
                pair.second_concept,
            )
            .with_binding("CLARIFICATION", format!("ENTITY:{}", pair.second_concept)),
            TurnSpec::interaction(
                format!("Right, do not touch the {}.", pair.first),
                format!("Confirmed. I’ll keep the {} outside the modification scope.", pair.first),
                "INFORM_ACKNOWLEDGEMENT",
            )
            .with_binding("CORRECTION", format!("ENTITY:{}", pair.first_concept)),
        ],
    }
}

fn mixed_noise(language: EvaluationLanguageIR, pair: &ResourcePair) -> Vec<TurnSpec> {
    match language {
        EvaluationLanguageIR::Korean => vec![
            TurnSpec::plan(
                format!("음... {} chek 좀 해줘.", pair.first),
                format!("알겠어. 군더더기 표현과 철자를 정리해서 {} 확인 요청으로 이해했어. 상태와 근거를 확인할게.", pair.first),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.first_concept,
            )
            .with_modality(ConversationInputModalityIR::VoiceTranscript),
            TurnSpec::plan(
                format!("어, 아니... {} 쪽 instead.", pair.second),
                format!("정정으로 이해했어. 확인 대상을 {}로 바꿀게.", pair.second),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.second_concept,
            )
            .with_binding("CORRECTION", format!("ENTITY:{}", pair.second_concept)),
            TurnSpec::result_absence(
                "근데 그거 finsh 됨?",
                "아직 완료됐다는 검증된 근거는 없어. 확인 계획과 완료 결과를 구분해야 해.",
            )
            .with_binding("RESULT", "GOAL:INVESTIGATE"),
            TurnSpec::interaction(
                "ㅇㅋ thx",
                "응. 검증된 변화가 생기면 결과로 구분해서 말할게.",
                "SOCIAL_BACKCHANNEL",
            ),
        ],
        EvaluationLanguageIR::English => vec![
            TurnSpec::plan(
                format!("Um... chek the {} pls.", pair.first),
                format!("Understood. I normalized the hesitation and typo as a request to check the {}. I’ll inspect its state and evidence.", pair.first),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.first_concept,
            )
            .with_modality(ConversationInputModalityIR::VoiceTranscript),
            TurnSpec::plan(
                format!("Uh, no—the {} 쪽 instead.", pair.second),
                format!("I read that as a correction. I’ll retarget the check to the {}.", pair.second),
                "INVESTIGATE",
                "INVESTIGATE",
                pair.second_concept,
            )
            .with_binding("CORRECTION", format!("ENTITY:{}", pair.second_concept)),
            TurnSpec::result_absence(
                "So, uh, didja finish that?",
                "There is no verified completion evidence yet. The inspection plan is not a completed result.",
            )
            .with_binding("RESULT", "GOAL:INVESTIGATE"),
            TurnSpec::interaction(
                "k thx",
                "Sure. I’ll distinguish any verified change from the earlier plan.",
                "SOCIAL_BACKCHANNEL",
            ),
        ],
    }
}

fn build_suites() -> Result<(BenchmarkInputSuiteIR, ReferenceSuiteIR), String> {
    let mut input_turns = Vec::new();
    let mut references = Vec::new();
    for (category_index, category) in CATEGORIES.iter().enumerate() {
        for dialogue_index in 0..6 {
            let language = if dialogue_index < 3 {
                EvaluationLanguageIR::Korean
            } else {
                EvaluationLanguageIR::English
            };
            let language_tag = match language {
                EvaluationLanguageIR::Korean => "KO",
                EvaluationLanguageIR::English => "EN",
            };
            let dialogue_id = format!(
                "GPTREF-DEV-C{:02}-{language_tag}-{:02}",
                category_index + 1,
                dialogue_index % 3 + 1
            );
            let turns = category_dialogue(category_index, language, dialogue_index);
            if turns.len() != 4 {
                return Err(format!("AUTHOR_DIALOGUE_TURN_COUNT_INVALID:{dialogue_id}"));
            }
            for (turn_offset, mut turn) in turns.into_iter().enumerate() {
                turn.meaning_atoms.sort();
                turn.meaning_atoms.dedup();
                turn.discourse_bindings.sort();
                turn.discourse_bindings.dedup();
                turn.propositions.sort();
                turn.propositions.dedup();
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
                    context_tags: vec!["GPT_REFERENCE_V1_DEVELOPMENT".to_string()],
                    max_plan_steps: 12,
                });
                references.push(ReferenceTurnAnnotationIR {
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
                    required_propositions: turn.propositions,
                    prohibited_propositions: turn.prohibited_propositions,
                    raw_reference_sha256: sha256_text(&turn.answer),
                    reference_surface: turn.answer,
                    calibrated_reference_surfaces: Vec::new(),
                    critical_boundary: turn.critical_boundary,
                    ambiguity_requires_clarification: turn.ambiguity_requires_clarification,
                });
            }
        }
    }
    let mut input = BenchmarkInputSuiteIR {
        schema: INPUT_SUITE_SCHEMA.to_string(),
        suite_id: SUITE_ID.to_string(),
        split: SuiteSplitIR::Development,
        frozen: true,
        turns: input_turns,
        suite_payload_sha256: String::new(),
    };
    input.seal()?;
    input.validate()?;
    let mut reference = ReferenceSuiteIR {
        schema: REFERENCE_SUITE_SCHEMA.to_string(),
        suite_id: SUITE_ID.to_string(),
        split: SuiteSplitIR::Development,
        frozen: true,
        reference_model_id: REFERENCE_MODEL.to_string(),
        reference_generation_date: "2026-09-03".to_string(),
        reference_system_prompt_sha256: sha256_text(REFERENCE_SYSTEM_PROMPT),
        generation_configuration_sha256: sha256_text(GENERATION_CONFIGURATION),
        input_suite_sha256: input.suite_payload_sha256.clone(),
        responses: references,
        suite_payload_sha256: String::new(),
    };
    reference.seal()?;
    input.validate_against_references(&reference)?;
    Ok((input, reference))
}

fn run() -> Result<(), String> {
    let mut arguments = env::args().skip(1);
    let output_directory = arguments
        .next()
        .ok_or_else(|| "USAGE: gpt-reference-development-author <output-directory>".to_string())?;
    if arguments.next().is_some() {
        return Err("TOO_MANY_ARGUMENTS".to_string());
    }
    let output_directory = Path::new(&output_directory);
    fs::create_dir_all(output_directory)
        .map_err(|error| format!("OUTPUT_DIRECTORY_CREATE_FAILED:{error}"))?;
    let (input, reference) = build_suites()?;
    let input_payload = serde_json::to_string_pretty(&input)
        .map_err(|error| format!("INPUT_SUITE_SERIALIZATION_FAILED:{error}"))?;
    let reference_payload = serde_json::to_string_pretty(&reference)
        .map_err(|error| format!("REFERENCE_SUITE_SERIALIZATION_FAILED:{error}"))?;
    fs::write(
        output_directory.join("development_input.json"),
        format!("{input_payload}\n"),
    )
    .map_err(|error| format!("INPUT_SUITE_WRITE_FAILED:{error}"))?;
    fs::write(
        output_directory.join("development_reference.json"),
        format!("{reference_payload}\n"),
    )
    .map_err(|error| format!("REFERENCE_SUITE_WRITE_FAILED:{error}"))?;
    println!("INPUT_SUITE_SHA256={}", input.suite_payload_sha256);
    println!("REFERENCE_SUITE_SHA256={}", reference.suite_payload_sha256);
    println!("DEVELOPMENT_DIALOGUES=60");
    println!("DEVELOPMENT_RESPONSES=240");
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
