use semantic_core_adapters::{
    AnswerClaimKindIR, AttributionAttitudeIR, CognitiveApi, ConversationInputModalityIR,
    ConversationTurnRequestIR, DiscourseAnswerDispositionIR, DiscourseQueryKindIR,
    EpistemicStatusIR, LanguageCodeIR, ModalWorldIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
struct QuestionCase {
    id: &'static str,
    family: &'static str,
    statements: &'static [&'static str],
    question: &'static str,
    language: LanguageCodeIR,
    kind: DiscourseQueryKindIR,
    disposition: DiscourseAnswerDispositionIR,
    evidence: usize,
    source: Option<&'static str>,
    attitude: Option<AttributionAttitudeIR>,
    status: Option<EpistemicStatusIR>,
    world: Option<ModalWorldIR>,
    realized_fragment: &'static str,
}

#[derive(Serialize)]
struct CanaryRow {
    id: String,
    family: String,
    input: String,
    pass: bool,
    observed: String,
}

#[derive(Serialize)]
struct CanaryReport {
    schema: &'static str,
    status: &'static str,
    total: usize,
    passed: usize,
    failed: usize,
    english_questions: usize,
    korean_questions: usize,
    source_and_attitude: usize,
    actuality_and_modality: usize,
    conflict_and_revision: usize,
    presupposition_and_abstention: usize,
    adversarial_and_tamper: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    rows: Vec<CanaryRow>,
}

fn request(
    conversation_id: &str,
    turn_index: u64,
    text: &str,
    language: LanguageCodeIR,
) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: conversation_id.to_string(),
        turn_index,
        request_id: format!("{conversation_id}-{turn_index}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(language),
        context_tags: Vec::new(),
        max_plan_steps: 12,
    }
}

fn question_cases() -> Vec<QuestionCase> {
    use AttributionAttitudeIR::{Believe, Expect, Know, Report, Say, Think, Want};
    use DiscourseAnswerDispositionIR::{
        AnsweredFromDialogueRecords, ConflictingDialogueRecords, DialogueTruthNotEstablished,
        MultipleDialogueRecords, NoConflictRecorded, NoMatchingRecord, PresuppositionUnverified,
    };
    use DiscourseQueryKindIR::{
        ActualityStatus, ConflictStatus, ModalStatus, PresuppositionCheck, PropositionSources,
        SourceContent,
    };
    use EpistemicStatusIR::{Believed, Desired, Expected, PresentedAsKnown, Reported};
    use LanguageCodeIR::{English, Korean};
    use ModalWorldIR::{
        Actual, Counterfactual, Desired as DesiredWorld, EpistemicPossible, Predicted,
    };

    vec![
        QuestionCase {
            id: "SRC_EN_SAY",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["Alice says that the server is down."],
            question: "What did Alice say?",
            language: English,
            kind: SourceContent,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("alice"),
            attitude: Some(Say),
            status: Some(Reported),
            world: Some(Actual),
            realized_fragment: "source-attributed",
        },
        QuestionCase {
            id: "SRC_EN_REPORT",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["Kai reports that the database is ready."],
            question: "What did Kai report?",
            language: English,
            kind: SourceContent,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("kai"),
            attitude: Some(Report),
            status: Some(Reported),
            world: Some(Actual),
            realized_fragment: "source-attributed",
        },
        QuestionCase {
            id: "SRC_EN_BELIEVE",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["Nora believes that the cache might be stale."],
            question: "What does Nora believe?",
            language: English,
            kind: SourceContent,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("nora"),
            attitude: Some(Believe),
            status: Some(Believed),
            world: Some(EpistemicPossible),
            realized_fragment: "not established facts",
        },
        QuestionCase {
            id: "SRC_EN_THINK",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["Omar thinks that the worker might be blocked."],
            question: "What does Omar think?",
            language: English,
            kind: SourceContent,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("omar"),
            attitude: Some(Think),
            status: Some(Believed),
            world: Some(EpistemicPossible),
            realized_fragment: "not established facts",
        },
        QuestionCase {
            id: "SRC_EN_KNOW",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["Priya knows that the release is ready."],
            question: "What does Priya know?",
            language: English,
            kind: SourceContent,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("priya"),
            attitude: Some(Know),
            status: Some(PresentedAsKnown),
            world: Some(Actual),
            realized_fragment: "not established facts",
        },
        QuestionCase {
            id: "SRC_EN_EXPECT",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["Uma expects that the rollout will finish."],
            question: "What does Uma expect?",
            language: English,
            kind: SourceContent,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("uma"),
            attitude: Some(Expect),
            status: Some(Expected),
            world: Some(Predicted),
            realized_fragment: "not established facts",
        },
        QuestionCase {
            id: "SRC_EN_WANT",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["Leo wants the team to retry."],
            question: "What does Leo want?",
            language: English,
            kind: SourceContent,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("leo"),
            attitude: Some(Want),
            status: Some(Desired),
            world: Some(DesiredWorld),
            realized_fragment: "not established facts",
        },
        QuestionCase {
            id: "SRC_KO_SAY",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["민수는 서버가 다운됐다고 말했다."],
            question: "민수는 뭐라고 말했어?",
            language: Korean,
            kind: SourceContent,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("민수"),
            attitude: Some(Say),
            status: Some(Reported),
            world: Some(Actual),
            realized_fragment: "사실 확정은 아니야",
        },
        QuestionCase {
            id: "SRC_KO_BELIEVE",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["지수는 캐시가 오래됐다고 믿는다."],
            question: "지수는 무엇을 믿어?",
            language: Korean,
            kind: SourceContent,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("지수"),
            attitude: Some(Believe),
            status: Some(Believed),
            world: Some(Actual),
            realized_fragment: "사실 확정은 아니야",
        },
        QuestionCase {
            id: "SRC_KO_REPORT",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["수아는 데이터베이스가 준비됐다고 보고했다."],
            question: "수아는 뭐라고 보고했어?",
            language: Korean,
            kind: SourceContent,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("수아"),
            attitude: Some(Report),
            status: Some(Reported),
            world: Some(Actual),
            realized_fragment: "사실 확정은 아니야",
        },
        QuestionCase {
            id: "SRC_KO_KNOW",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["준은 릴리스가 준비됐다고 알고 있다."],
            question: "준은 무엇을 알고 있어?",
            language: Korean,
            kind: SourceContent,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("준"),
            attitude: Some(Know),
            status: Some(PresentedAsKnown),
            world: Some(Actual),
            realized_fragment: "사실 확정은 아니야",
        },
        QuestionCase {
            id: "SRC_CROSS_EN_TO_KO",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["Alice says that the cache is clean."],
            question: "Alice는 뭐라고 말했어?",
            language: Korean,
            kind: SourceContent,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("alice"),
            attitude: Some(Say),
            status: Some(Reported),
            world: Some(Actual),
            realized_fragment: "사실 확정은 아니야",
        },
        QuestionCase {
            id: "SRC_CROSS_KO_TO_EN",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["민수는 서버가 느리다고 말했다."],
            question: "What did 민수 say?",
            language: English,
            kind: SourceContent,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("민수"),
            attitude: Some(Say),
            status: Some(Reported),
            world: Some(Actual),
            realized_fragment: "source-attributed",
        },
        QuestionCase {
            id: "WHO_EN_SAY",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["Alice says that the server is down."],
            question: "Who said the server is down?",
            language: English,
            kind: PropositionSources,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("alice"),
            attitude: Some(Say),
            status: None,
            world: None,
            realized_fragment: "identifies recorded sources",
        },
        QuestionCase {
            id: "WHO_EN_REPORT",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["Kai reports that the database is ready."],
            question: "Who reported that the database is ready?",
            language: English,
            kind: PropositionSources,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("kai"),
            attitude: Some(Report),
            status: None,
            world: None,
            realized_fragment: "identifies recorded sources",
        },
        QuestionCase {
            id: "WHO_EN_BELIEVE",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["Nora believes that the cache is stale."],
            question: "Who believes the cache is stale?",
            language: English,
            kind: PropositionSources,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("nora"),
            attitude: Some(Believe),
            status: None,
            world: None,
            realized_fragment: "identifies recorded sources",
        },
        QuestionCase {
            id: "WHO_EN_KNOW",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["Priya knows that the release is ready."],
            question: "Who knows the release is ready?",
            language: English,
            kind: PropositionSources,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("priya"),
            attitude: Some(Know),
            status: None,
            world: None,
            realized_fragment: "identifies recorded sources",
        },
        QuestionCase {
            id: "WHO_KO_SAY",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["민수는 서버가 다운됐다고 말했다."],
            question: "누가 서버가 다운됐다고 말했어?",
            language: Korean,
            kind: PropositionSources,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("민수"),
            attitude: Some(Say),
            status: None,
            world: None,
            realized_fragment: "출처 식별",
        },
        QuestionCase {
            id: "WHO_KO_BELIEVE",
            family: "SOURCE_AND_ATTITUDE",
            statements: &["지수는 캐시가 오래됐다고 믿는다."],
            question: "누가 캐시가 오래됐다고 믿어?",
            language: Korean,
            kind: PropositionSources,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("지수"),
            attitude: Some(Believe),
            status: None,
            world: None,
            realized_fragment: "출처 식별",
        },
        QuestionCase {
            id: "ACT_EN_TRUE",
            family: "ACTUALITY_AND_MODALITY",
            statements: &["Alice says that the server is down."],
            question: "Is it true that the server is down?",
            language: English,
            kind: ActualityStatus,
            disposition: DialogueTruthNotEstablished,
            evidence: 1,
            source: None,
            attitude: None,
            status: None,
            world: Some(Actual),
            realized_fragment: "cannot say it is actually true",
        },
        QuestionCase {
            id: "ACT_EN_FACT",
            family: "ACTUALITY_AND_MODALITY",
            statements: &["Alice says that the server is down."],
            question: "Is that a fact about the server?",
            language: English,
            kind: ActualityStatus,
            disposition: DialogueTruthNotEstablished,
            evidence: 1,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "cannot say it is actually true",
        },
        QuestionCase {
            id: "ACT_EN_KNOW_WHETHER",
            family: "ACTUALITY_AND_MODALITY",
            statements: &["Alice believes that the server might be down."],
            question: "Do we know whether the server is down?",
            language: English,
            kind: ActualityStatus,
            disposition: DialogueTruthNotEstablished,
            evidence: 1,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "cannot say it is actually true",
        },
        QuestionCase {
            id: "ACT_KO_FACT",
            family: "ACTUALITY_AND_MODALITY",
            statements: &["민수는 서버가 다운됐다고 말했다."],
            question: "서버가 다운된 게 사실이야?",
            language: Korean,
            kind: ActualityStatus,
            disposition: DialogueTruthNotEstablished,
            evidence: 1,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "확정할 수 없어",
        },
        QuestionCase {
            id: "ACT_KO_CERTAIN",
            family: "ACTUALITY_AND_MODALITY",
            statements: &["민수는 서버가 다운됐다고 말했다."],
            question: "서버가 다운된 게 확실해?",
            language: Korean,
            kind: ActualityStatus,
            disposition: DialogueTruthNotEstablished,
            evidence: 1,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "확정할 수 없어",
        },
        QuestionCase {
            id: "ACT_EMPTY",
            family: "ACTUALITY_AND_MODALITY",
            statements: &[],
            question: "Is it true that the server is down?",
            language: English,
            kind: ActualityStatus,
            disposition: DialogueTruthNotEstablished,
            evidence: 0,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "does not establish",
        },
        QuestionCase {
            id: "MOD_EN_POSSIBLE",
            family: "ACTUALITY_AND_MODALITY",
            statements: &["Alice believes that the server might be down."],
            question: "Is the server merely possible or actual?",
            language: English,
            kind: ModalStatus,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("alice"),
            attitude: Some(Believe),
            status: None,
            world: Some(EpistemicPossible),
            realized_fragment: "epistemic possibility",
        },
        QuestionCase {
            id: "MOD_EN_PREDICTED",
            family: "ACTUALITY_AND_MODALITY",
            statements: &["Uma expects that the rollout will finish."],
            question: "Is the rollout a prediction or fact?",
            language: English,
            kind: ModalStatus,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("uma"),
            attitude: Some(Expect),
            status: None,
            world: Some(Predicted),
            realized_fragment: "prediction",
        },
        QuestionCase {
            id: "MOD_EN_COUNTERFACTUAL",
            family: "ACTUALITY_AND_MODALITY",
            statements: &[
                "Alice says that if the backup had existed, the restore would have succeeded.",
            ],
            question: "Is the restore counterfactual or actual?",
            language: English,
            kind: ModalStatus,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("alice"),
            attitude: Some(Say),
            status: None,
            world: Some(Counterfactual),
            realized_fragment: "counterfactual world",
        },
        QuestionCase {
            id: "MOD_KO_POSSIBLE",
            family: "ACTUALITY_AND_MODALITY",
            statements: &["민수는 서버가 느릴 수도 있다고 믿는다."],
            question: "서버가 느린 건 가능성인지 사실인지?",
            language: Korean,
            kind: ModalStatus,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("민수"),
            attitude: Some(Believe),
            status: None,
            world: Some(EpistemicPossible),
            realized_fragment: "인식적 가능성",
        },
        QuestionCase {
            id: "MOD_KO_PREDICTED",
            family: "ACTUALITY_AND_MODALITY",
            statements: &["지수는 빌드가 실패할 거라고 예상했다."],
            question: "빌드 실패는 예측이야 아니면 사실이야?",
            language: Korean,
            kind: ModalStatus,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("지수"),
            attitude: Some(Expect),
            status: None,
            world: Some(Predicted),
            realized_fragment: "예측",
        },
        QuestionCase {
            id: "MOD_EMPTY",
            family: "ACTUALITY_AND_MODALITY",
            statements: &[],
            question: "Is the restore counterfactual or actual?",
            language: English,
            kind: ModalStatus,
            disposition: NoMatchingRecord,
            evidence: 0,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "no matching dialogue record",
        },
        QuestionCase {
            id: "CONFLICT_EN_DIRECT",
            family: "CONFLICT_AND_REVISION",
            statements: &[
                "Alice says that the server is up.",
                "Bob says that the server is down.",
            ],
            question: "Are Alice and Bob in conflict about the server?",
            language: English,
            kind: ConflictStatus,
            disposition: ConflictingDialogueRecords,
            evidence: 2,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "No source has been selected",
        },
        QuestionCase {
            id: "CONFLICT_EN_DISAGREE",
            family: "CONFLICT_AND_REVISION",
            statements: &[
                "Alice says that the cache is valid.",
                "Bob says that the cache is invalid.",
            ],
            question: "Do Alice and Bob disagree about the cache?",
            language: English,
            kind: ConflictStatus,
            disposition: ConflictingDialogueRecords,
            evidence: 2,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "truth winner",
        },
        QuestionCase {
            id: "CONFLICT_EN_ACCOUNT",
            family: "CONFLICT_AND_REVISION",
            statements: &[
                "Kai reports that the database is ready.",
                "Nora reports that the database is not ready.",
            ],
            question: "Are the database accounts conflicting?",
            language: English,
            kind: ConflictStatus,
            disposition: ConflictingDialogueRecords,
            evidence: 2,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "in conflict",
        },
        QuestionCase {
            id: "CONFLICT_KO_DIRECT",
            family: "CONFLICT_AND_REVISION",
            statements: &[
                "민수는 서버가 정상이라고 말했다.",
                "지수는 서버가 비정상이라고 말했다.",
            ],
            question: "민수와 지수의 서버 설명이 충돌해?",
            language: Korean,
            kind: ConflictStatus,
            disposition: ConflictingDialogueRecords,
            evidence: 2,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "사실 승자",
        },
        QuestionCase {
            id: "CONFLICT_NONE",
            family: "CONFLICT_AND_REVISION",
            statements: &["Alice says that the server is up."],
            question: "Are Alice and Bob in conflict about the server?",
            language: English,
            kind: ConflictStatus,
            disposition: NoConflictRecorded,
            evidence: 1,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "No matching active source conflict",
        },
        QuestionCase {
            id: "REVISION_CURRENT",
            family: "CONFLICT_AND_REVISION",
            statements: &[
                "Alice says that the cache is stale.",
                "Alice corrected that the cache is healthy.",
            ],
            question: "What did Alice say?",
            language: English,
            kind: SourceContent,
            disposition: AnsweredFromDialogueRecords,
            evidence: 1,
            source: Some("alice"),
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "cache is healthy",
        },
        QuestionCase {
            id: "REVISION_HISTORICAL",
            family: "CONFLICT_AND_REVISION",
            statements: &[
                "Alice says that the cache is stale.",
                "Alice corrected that the cache is healthy.",
            ],
            question: "What did Alice say before?",
            language: English,
            kind: SourceContent,
            disposition: MultipleDialogueRecords,
            evidence: 2,
            source: Some("alice"),
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "cache is stale",
        },
        QuestionCase {
            id: "PRESUP_EN_WHY",
            family: "PRESUPPOSITION_AND_ABSTENTION",
            statements: &["Alice believes that the server might fail."],
            question: "Why did the server fail?",
            language: English,
            kind: PresuppositionCheck,
            disposition: PresuppositionUnverified,
            evidence: 1,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "not established as true",
        },
        QuestionCase {
            id: "PRESUP_EN_WHEN",
            family: "PRESUPPOSITION_AND_ABSTENTION",
            statements: &["Alice believes that the cache might corrupt."],
            question: "When did the cache corrupt?",
            language: English,
            kind: PresuppositionCheck,
            disposition: PresuppositionUnverified,
            evidence: 1,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "not established as true",
        },
        QuestionCase {
            id: "PRESUP_EN_HOW",
            family: "PRESUPPOSITION_AND_ABSTENTION",
            statements: &["Alice believes that the worker might crash."],
            question: "How did the worker crash?",
            language: English,
            kind: PresuppositionCheck,
            disposition: PresuppositionUnverified,
            evidence: 1,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "silently accepting",
        },
        QuestionCase {
            id: "PRESUP_EN_REALIZE",
            family: "PRESUPPOSITION_AND_ABSTENTION",
            statements: &["Alice believes that the server might fail."],
            question: "Did Alice realize that the server failed?",
            language: English,
            kind: PresuppositionCheck,
            disposition: PresuppositionUnverified,
            evidence: 1,
            source: Some("alice"),
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "presupposes",
        },
        QuestionCase {
            id: "PRESUP_KO_WHY",
            family: "PRESUPPOSITION_AND_ABSTENTION",
            statements: &["민수는 서버가 실패할 수도 있다고 믿는다."],
            question: "왜 서버가 실패했어?",
            language: Korean,
            kind: PresuppositionCheck,
            disposition: PresuppositionUnverified,
            evidence: 1,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "전제로",
        },
        QuestionCase {
            id: "PRESUP_KO_WHEN",
            family: "PRESUPPOSITION_AND_ABSTENTION",
            statements: &["민수는 캐시가 손상됐을 수도 있다고 믿는다."],
            question: "언제 캐시가 손상됐어?",
            language: Korean,
            kind: PresuppositionCheck,
            disposition: PresuppositionUnverified,
            evidence: 1,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "검증되지 않았어",
        },
        QuestionCase {
            id: "PRESUP_KO_HOW",
            family: "PRESUPPOSITION_AND_ABSTENTION",
            statements: &["민수는 워커가 중단됐을 수도 있다고 믿는다."],
            question: "어떻게 워커가 중단됐어?",
            language: Korean,
            kind: PresuppositionCheck,
            disposition: PresuppositionUnverified,
            evidence: 1,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "몰래 받아들이지",
        },
        QuestionCase {
            id: "ADV_UNKNOWN_EN",
            family: "ADVERSARIAL_AND_TAMPER",
            statements: &["Alice says that the server is down."],
            question: "What did Charlie say?",
            language: English,
            kind: SourceContent,
            disposition: NoMatchingRecord,
            evidence: 0,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "will not invent",
        },
        QuestionCase {
            id: "ADV_UNKNOWN_KO",
            family: "ADVERSARIAL_AND_TAMPER",
            statements: &["민수는 서버가 다운됐다고 말했다."],
            question: "영희는 뭐라고 말했어?",
            language: Korean,
            kind: SourceContent,
            disposition: NoMatchingRecord,
            evidence: 0,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "추측해서 채우지",
        },
        QuestionCase {
            id: "ADV_NAME_PREFIX",
            family: "ADVERSARIAL_AND_TAMPER",
            statements: &["Ann says that the server is down."],
            question: "What did Annabelle say?",
            language: English,
            kind: SourceContent,
            disposition: NoMatchingRecord,
            evidence: 0,
            source: None,
            attitude: None,
            status: None,
            world: None,
            realized_fragment: "will not invent",
        },
    ]
}

fn run_question_case(case: QuestionCase) -> CanaryRow {
    let mut api = CognitiveApi::new_embedded().expect("embedded cognitive API");
    let conversation_id = format!("R10-{}", case.id);
    for (index, statement) in case.statements.iter().enumerate() {
        api.process_conversation_turn(&request(
            &conversation_id,
            u64::try_from(index + 1).expect("bounded case"),
            statement,
            case.language,
        ))
        .expect("seed discourse state");
    }
    let records_before = api
        .conversation_state(&conversation_id)
        .map_or(0, |state| state.epistemic_ledger.records.len());
    let question_turn = u64::try_from(case.statements.len() + 1).expect("bounded case");
    let response = api
        .process_conversation_turn(&request(
            &conversation_id,
            question_turn,
            case.question,
            case.language,
        ))
        .expect("question turn");
    let answer = response.discourse_answer.as_ref();
    let expected_source = case.source.map(str::to_lowercase);
    let pass = answer.is_some_and(|answer| {
        answer.validate()
            && answer.query.kind == case.kind
            && answer.disposition == case.disposition
            && answer.evidence.len() == case.evidence
            && answer.realized_text.contains(case.realized_fragment)
            && !answer.dialogue_truth_established
            && !answer.external_execution_authorized
            && answer.unsupported_claims == 0
            && case.source.is_none_or(|_| {
                answer
                    .evidence
                    .iter()
                    .any(|item| Some(item.source_actor.to_lowercase()) == expected_source)
            })
            && case
                .attitude
                .is_none_or(|expected| answer.evidence.iter().any(|item| item.attitude == expected))
            && case.status.is_none_or(|expected| {
                answer
                    .evidence
                    .iter()
                    .any(|item| item.epistemic_status == expected)
            })
            && case.world.is_none_or(|expected| {
                answer
                    .evidence
                    .iter()
                    .any(|item| item.modal_world == expected)
            })
    }) && response.conversation_state.epistemic_ledger.records.len() == records_before
        && response.conversation_state.active_goals.is_empty()
        && response.grounded_response.is_none()
        && response.output.grounded_plan_sha256.is_none()
        && response.output.unsupported_freeform_claims == 0;
    CanaryRow {
        id: case.id.to_string(),
        family: case.family.to_string(),
        input: case.question.to_string(),
        pass,
        observed: answer.map_or_else(
            || {
                format!(
                    "answer=NONE;turn_disposition={:?};normalization={:?};ambiguous_input={};resolved={};ambiguous_refs={:?};records_before={records_before};records_after={}",
                    response.disposition,
                    response.normalization.disposition,
                    response.normalization.ambiguous_input,
                    response.reference_resolution.resolved_semantic_text,
                    response.reference_resolution.ambiguous_reference_surfaces,
                    response.conversation_state.epistemic_ledger.records.len(),
                )
            },
            |answer| {
                format!(
                    "kind={:?};disposition={:?};evidence={};records={};text={}",
                    answer.query.kind,
                    answer.disposition,
                    answer.evidence.len(),
                    response.conversation_state.epistemic_ledger.records.len(),
                    answer.realized_text
                )
            },
        ),
    }
}

fn tamper_rows() -> Vec<CanaryRow> {
    let mut api = CognitiveApi::new_embedded().expect("embedded cognitive API");
    api.process_conversation_turn(&request(
        "R10-TAMPER",
        1,
        "Alice knows that the server is down.",
        LanguageCodeIR::English,
    ))
    .expect("seed attributed record");
    let response = api
        .process_conversation_turn(&request(
            "R10-TAMPER",
            2,
            "What does Alice know?",
            LanguageCodeIR::English,
        ))
        .expect("seed answer");
    let answer = response.discourse_answer.expect("typed discourse answer");
    let mut rows = Vec::new();

    let mut tampered = answer.clone();
    tampered.dialogue_truth_established = true;
    rows.push(tamper_row("TAMPER_ANSWER_TRUTH", !tampered.validate()));

    let mut tampered = answer.clone();
    tampered.external_execution_authorized = true;
    rows.push(tamper_row("TAMPER_ANSWER_AUTHORITY", !tampered.validate()));

    let mut tampered = answer.clone();
    tampered.evidence[0].dialogue_truth_established = true;
    rows.push(tamper_row("TAMPER_EVIDENCE_TRUTH", !tampered.validate()));

    let mut tampered = answer.clone();
    tampered.evidence[0].external_execution_authorized = true;
    rows.push(tamper_row(
        "TAMPER_EVIDENCE_AUTHORITY",
        !tampered.validate(),
    ));

    let mut tampered = answer.clone();
    tampered.evidence.clear();
    rows.push(tamper_row("TAMPER_MISSING_EVIDENCE", !tampered.validate()));

    let mut tampered = answer.clone();
    tampered.unsupported_claims = 1;
    rows.push(tamper_row("TAMPER_UNSUPPORTED_CLAIM", !tampered.validate()));

    let has_factive_status_not_truth = answer.evidence.iter().any(|item| {
        item.epistemic_status == EpistemicStatusIR::PresentedAsKnown
            && !item.dialogue_truth_established
    }) && answer
        .claims
        .iter()
        .all(|claim| claim.kind != AnswerClaimKindIR::DialogueTruthNotEstablished);
    rows.push(CanaryRow {
        id: "ADVERSARIAL_FACTIVE_IS_NOT_FACT".to_string(),
        family: "ADVERSARIAL_AND_TAMPER".to_string(),
        input: "Alice knows that ... -> What does Alice know?".to_string(),
        pass: has_factive_status_not_truth,
        observed: format!(
            "status={:?};truth={}",
            answer.evidence[0].epistemic_status, answer.dialogue_truth_established
        ),
    });
    rows
}

fn tamper_row(id: &str, pass: bool) -> CanaryRow {
    CanaryRow {
        id: id.to_string(),
        family: "ADVERSARIAL_AND_TAMPER".to_string(),
        input: id.to_string(),
        pass,
        observed: format!("tamper_rejected={pass}"),
    }
}

fn main() {
    let cases = question_cases();
    let english_questions = cases
        .iter()
        .filter(|case| case.language == LanguageCodeIR::English)
        .count();
    let korean_questions = cases.len() - english_questions;
    let mut rows = cases.into_iter().map(run_question_case).collect::<Vec<_>>();
    rows.extend(tamper_rows());
    let passed = rows.iter().filter(|row| row.pass).count();
    let report = CanaryReport {
        schema: "B_CORE_DISCOURSE_QA_CANARY_V1",
        status: if passed == rows.len() { "PASS" } else { "FAIL" },
        total: rows.len(),
        passed,
        failed: rows.len() - passed,
        english_questions,
        korean_questions,
        source_and_attitude: rows
            .iter()
            .filter(|row| row.family == "SOURCE_AND_ATTITUDE")
            .count(),
        actuality_and_modality: rows
            .iter()
            .filter(|row| row.family == "ACTUALITY_AND_MODALITY")
            .count(),
        conflict_and_revision: rows
            .iter()
            .filter(|row| row.family == "CONFLICT_AND_REVISION")
            .count(),
        presupposition_and_abstention: rows
            .iter()
            .filter(|row| row.family == "PRESUPPOSITION_AND_ABSTENTION")
            .count(),
        adversarial_and_tamper: rows
            .iter()
            .filter(|row| row.family == "ADVERSARIAL_AND_TAMPER")
            .count(),
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        rows,
    };
    println!(
        "{}",
        serde_json::to_string(&report).expect("serialize canary report")
    );
    if report.failed != 0 {
        std::process::exit(1);
    }
}
