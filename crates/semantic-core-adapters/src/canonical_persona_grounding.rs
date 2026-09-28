//! Canonical grounding for the reviewed synthetic Korean persona gold corpus.
//!
//! The source's `approved_meaning` is structured synthetic world state, not a
//! user utterance.  Every accepted row is rebuilt through the real semantic
//! planner and replay-verified approval boundary before it can contribute a
//! construction observation.  Its supplied surface is retained only as an
//! auditable candidate; it is never an authority or a runtime phrase cache.

use dockable_semantic_core::{
    DeliberationEngine, DeliberationRequestIR, DockableCore, EvidenceIR, LiteralIR, PlanIntentIR,
    SemanticPlanArgumentIR, SemanticPlanEventIR, SemanticPlanGoalIR, SemanticPlanProjectionIR,
    SemanticPlanRoleIR, DELIBERATION_REQUEST_SCHEMA, SEMANTIC_PLAN_GOAL_SCHEMA,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    interpret_document_semantics, ApprovedClaimBindingIR, ApprovedCompositionalClaimIR,
    ApprovedCompositionalResponseBuilder, ApprovedDiscourseRelationIR, ApprovedLexicalNodeIR,
    ApprovedModalityIR, ApprovedOpenValueIR, ApprovedRelationTypeIR, ApprovedResponseBuilder,
    ApprovedResponseCandidateIR, ApprovedResponseStyleIR, ApprovedSemanticTypeIR,
    ApprovedSpeechActIR, ApprovedValueIR, ApprovedVerbosityIR, CanonicalResponseDatasetSplitIR,
    CanonicalResponseTrainingRecordIR, LanguageCodeIR, LanguageRegisterIR,
};

pub const CANONICAL_PERSONA_GOLD_RECORD_SCHEMA: &str = "BCORE.CANONICAL_PERSONA_GOLD_RECORD.V1";

/// A grounded source row. `source_surface_construction_candidate` remains
/// non-authoritative until the codec's own semantic inverse accepts it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalPersonaGoldRecordIR {
    pub schema: String,
    pub source_example_id: String,
    pub source_record_sha256: String,
    pub semantic_family_id: String,
    pub persona_label: String,
    pub persona_register: String,
    pub persona_relationship: String,
    pub persona_voice: String,
    pub canonical_record: CanonicalResponseTrainingRecordIR,
    pub source_surface_construction_candidate: String,
    pub source_surface_sha256: String,
    pub source_surface_inverse_verified: bool,
    pub record_sha256: String,
}

impl CanonicalPersonaGoldRecordIR {
    pub fn validate(&self) -> bool {
        self.schema == CANONICAL_PERSONA_GOLD_RECORD_SCHEMA
            && !self.source_example_id.trim().is_empty()
            && sha256_shape(&self.source_record_sha256)
            && sha256_shape(&self.semantic_family_id)
            && !self.persona_label.trim().is_empty()
            && !self.persona_register.trim().is_empty()
            && !self.persona_relationship.trim().is_empty()
            && !self.persona_voice.trim().is_empty()
            && self.canonical_record.validate()
            && !self.source_surface_construction_candidate.trim().is_empty()
            && sha256_shape(&self.source_surface_sha256)
            && self.source_surface_sha256
                == sha256_text(&self.source_surface_construction_candidate)
            && self.record_sha256 == canonical_persona_gold_record_sha256(self)
    }
}

pub fn canonical_persona_gold_record_sha256(record: &CanonicalPersonaGoldRecordIR) -> String {
    let mut canonical = record.clone();
    canonical.record_sha256.clear();
    sha256_json(&canonical)
}

/// Grounds one source JSON row. Unsupported speech acts and fields fail
/// closed with stable reason codes rather than being relabelled as Inform.
pub fn ground_synthetic_persona_gold_row(
    core: &DockableCore,
    row: &Value,
) -> Result<CanonicalPersonaGoldRecordIR, String> {
    let row = row
        .as_object()
        .ok_or_else(|| "SOURCE_ROW_NOT_OBJECT".to_string())?;
    let meaning = required_object(row, "approved_meaning")?;
    if required_string(meaning, "act")? != "INFORM" {
        return Err("UNSUPPORTED_SPEECH_ACT".into());
    }
    let source_example_id = required_string(row, "example_id")?.to_string();
    let source_record_sha256 = required_string(row, "record_sha256")?.to_string();
    if !sha256_shape(&source_record_sha256) {
        return Err("SOURCE_RECORD_SHA256_INVALID".into());
    }
    let source_surface = required_string(row, "surface")?.to_string();
    let persona = required_object(row, "persona")?;
    let style = style_from_persona(persona)?;
    let primary = primary_subject(meaning)?;
    let source_meaning_sha = sha256_json(meaning);
    let goal = planner_goal(&source_example_id, &source_meaning_sha, &primary)?;
    let bundle = core
        .generate_semantic_plan(&goal)
        .map_err(|_| "PERSONA_GOLD_PLANNER_FAILED")?;
    if !bundle.validate_against(&goal) {
        return Err("PERSONA_GOLD_PLANNER_BUNDLE_INVALID".into());
    }
    let (claims, compositional, relation) = claims_from_meaning(meaning, &primary)?;
    let approved = approve_claims(&goal, &bundle.bundle_sha256, claims, relation, style)?;
    let compositional = ApprovedCompositionalResponseBuilder
        .enrich(&approved, compositional)
        .map_err(|_| "PERSONA_GOLD_COMPOSITIONAL_ENRICH_FAILED")?;
    let split = split_from_source(required_string(row, "split")?)?;
    let canonical_record = CanonicalResponseTrainingRecordIR::from_approved_with_split(
        compositional.clone(),
        LanguageCodeIR::Korean,
        split,
    )?;
    let inverse_verified =
        interpret_document_semantics(&source_surface, &compositional, LanguageCodeIR::Korean)
            .is_ok_and(|inverse| inverse.validate(&compositional));
    let mut grounded = CanonicalPersonaGoldRecordIR {
        schema: CANONICAL_PERSONA_GOLD_RECORD_SCHEMA.into(),
        source_example_id,
        source_record_sha256,
        // The source has no stable semantic ID.  This family key is derived
        // only from structured meaning, never surface text or persona labels.
        semantic_family_id: source_meaning_sha,
        persona_label: persona
            .get("persona_label")
            .or_else(|| persona.get("mbti_benchmark_label"))
            .or_else(|| persona.get("fictional_register"))
            .and_then(Value::as_str)
            .unwrap_or("UNSPECIFIED")
            .to_string(),
        persona_register: required_string(persona, "register")?.to_string(),
        persona_relationship: required_string(persona, "relationship")?.to_string(),
        persona_voice: required_string(persona, "voice")?.to_string(),
        canonical_record,
        source_surface_construction_candidate: source_surface.clone(),
        source_surface_sha256: sha256_text(&source_surface),
        source_surface_inverse_verified: inverse_verified,
        record_sha256: String::new(),
    };
    grounded.record_sha256 = canonical_persona_gold_record_sha256(&grounded);
    grounded
        .validate()
        .then_some(grounded)
        .ok_or_else(|| "PERSONA_GOLD_GROUNDING_INVALID".into())
}

#[derive(Debug, Clone)]
struct PrimarySubject {
    node_id: String,
    label: String,
    semantic_type: ApprovedSemanticTypeIR,
}

fn primary_subject(meaning: &serde_json::Map<String, Value>) -> Result<PrimarySubject, String> {
    let selected = [
        ("event", ApprovedSemanticTypeIR::Event),
        ("theme", ApprovedSemanticTypeIR::Concept),
        ("feature", ApprovedSemanticTypeIR::Concept),
        ("place", ApprovedSemanticTypeIR::Location),
    ]
    .into_iter()
    .find_map(|(field, kind)| {
        meaning
            .get(field)
            .and_then(Value::as_str)
            .map(|v| (field, kind, v.to_string()))
    })
    .or_else(|| {
        meaning.get("weather").and_then(Value::as_str).map(|_| {
            let day = meaning.get("day").and_then(Value::as_str).unwrap_or("현재");
            (
                "weather",
                ApprovedSemanticTypeIR::Concept,
                format!("{day} 날씨"),
            )
        })
    });
    let (_, semantic_type, label) =
        selected.ok_or_else(|| "INFORM_PRIMARY_SUBJECT_MISSING".to_string())?;
    if label.trim().is_empty() {
        return Err("INFORM_PRIMARY_SUBJECT_EMPTY".into());
    }
    Ok(PrimarySubject {
        node_id: format!(
            "PERSONA:SUBJECT:{}",
            sha256_text(&label)[..24].to_uppercase()
        ),
        label,
        semantic_type,
    })
}

fn planner_goal(
    example_id: &str,
    source_meaning_sha: &str,
    primary: &PrimarySubject,
) -> Result<SemanticPlanGoalIR, String> {
    let argument_id = format!("PERSONA:ARG:{}", &source_meaning_sha[..20]);
    let event_id = format!("PERSONA:EVENT:{}", &source_meaning_sha[..20]);
    let mut goal = SemanticPlanGoalIR {
        schema: SEMANTIC_PLAN_GOAL_SCHEMA.into(),
        goal_id: format!("PERSONA:GOLD:{example_id}"),
        events: vec![SemanticPlanEventIR {
            event_id: event_id.clone(),
            predicate_concept_id: "PERSONA_GOLD_INFORM".into(),
            intent: PlanIntentIR::Communicate,
            argument_ids: vec![argument_id.clone()],
            goal_subject_argument_ids: vec![argument_id.clone()],
            projection: SemanticPlanProjectionIR::LiveRequest,
            user_request_present: true,
            external_execution_authorized: false,
        }],
        arguments: vec![SemanticPlanArgumentIR {
            argument_id,
            role: SemanticPlanRoleIR::Theme,
            concept_ids: vec![primary.node_id.clone()],
            grounded_label: primary.label.clone(),
        }],
        relations: Vec::new(),
        selected_live_event_ids: vec![event_id],
        context_semantic_ids: vec![format!("PERSONA_GOLD:{source_meaning_sha}")],
        source_semantic_sha256: source_meaning_sha.to_string(),
        max_steps_per_event: 16,
        semantic_authority: false,
        language_can_execute: false,
        semantic_sha256: String::new(),
    };
    goal.seal();
    goal.validate()
        .then_some(goal)
        .ok_or_else(|| "PERSONA_GOLD_PLAN_GOAL_INVALID".into())
}

fn claims_from_meaning(
    meaning: &serde_json::Map<String, Value>,
    primary: &PrimarySubject,
) -> Result<
    (
        Vec<ApprovedClaimBindingIR>,
        Vec<ApprovedCompositionalClaimIR>,
        ApprovedDiscourseRelationIR,
    ),
    String,
> {
    let allowed = [
        "act",
        "event",
        "theme",
        "feature",
        "place",
        "state",
        "operation",
        "time",
        "day",
        "channel",
        "delay_minutes",
        "reason",
        "weather",
        "reference_day",
        "reference_time",
        "reference_place",
    ];
    if meaning.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err("UNREPRESENTABLE_MEANING_FIELD".into());
    }
    let mut rows = Vec::new();
    let relation = match meaning.get("operation").and_then(Value::as_str) {
        Some("REVISED") => ApprovedDiscourseRelationIR::Correction,
        Some(_) => return Err("UNSUPPORTED_INFORM_OPERATION".into()),
        None => ApprovedDiscourseRelationIR::Statement,
    };
    if let Some(state) = meaning.get("state").and_then(Value::as_str) {
        rows.push((ApprovedRelationTypeIR::Status, lexical_state(state)?));
    }
    if let Some(time) = meaning.get("time").and_then(Value::as_str) {
        rows.push((ApprovedRelationTypeIR::Time, time_value(time)));
    }
    if let Some(day) = meaning.get("day").and_then(Value::as_str) {
        rows.push((ApprovedRelationTypeIR::Date, text_value(day)));
    }
    if let Some(place) = meaning.get("place").and_then(Value::as_str) {
        // When `place` is the primary entity it still may carry its own
        // location state only if another location value was supplied. This
        // corpus does not have that shape, so preserve it as the subject.
        if primary.label != place {
            rows.push((
                ApprovedRelationTypeIR::Location,
                lexical_value(place, ApprovedSemanticTypeIR::Location),
            ));
        }
    }
    if let Some(channel) = meaning.get("channel").and_then(Value::as_str) {
        rows.push((
            ApprovedRelationTypeIR::Manner,
            lexical_value(&channel_label(channel), ApprovedSemanticTypeIR::Concept),
        ));
    }
    if let Some(minutes) = meaning.get("delay_minutes").and_then(Value::as_i64) {
        rows.push((ApprovedRelationTypeIR::Duration, integer_value(minutes)));
    }
    if let Some(reason) = meaning.get("reason").and_then(Value::as_str) {
        rows.push((
            ApprovedRelationTypeIR::Manner,
            lexical_value(&reason_label(reason), ApprovedSemanticTypeIR::Concept),
        ));
    }
    if let Some(weather) = meaning.get("weather").and_then(Value::as_str) {
        rows.push((
            ApprovedRelationTypeIR::Status,
            lexical_value(&weather_label(weather), ApprovedSemanticTypeIR::State),
        ));
    }
    if rows.is_empty() {
        return Err("INFORM_HAS_NO_APPROVABLE_CLAIM".into());
    }
    if rows.len() > 64 {
        return Err("INFORM_TOO_MANY_CLAIMS".into());
    }
    let mut bindings = Vec::with_capacity(rows.len());
    let mut compositional = Vec::with_capacity(rows.len());
    for (index, (relation_type, value)) in rows.into_iter().enumerate() {
        let proposition_id = format!("PERSONA:CLAIM:{index}");
        bindings.push(ApprovedClaimBindingIR {
            proposition_id: proposition_id.clone(),
            subject: primary.node_id.clone(),
            property: relation_symbol(relation_type).into(),
            value: value.0,
            polarity: true,
        });
        compositional.push(ApprovedCompositionalClaimIR {
            proposition_id,
            subject: ApprovedLexicalNodeIR {
                node_id: primary.node_id.clone(),
                semantic_type: primary.semantic_type,
                canonical_lexical_label: primary.label.clone(),
            },
            relation: relation_type,
            value: value.1,
            polarity: true,
            modality: ApprovedModalityIR::Asserted,
    status_frame: None,
});
    }
    // A response anchor is a distinct semantic node.  Attaching these values
    // to the event itself conflates an event's scheduled time with the
    // discourse context in which it was uttered and creates inverse ambiguity.
    let anchor = PrimarySubject {
        node_id: format!(
            "PERSONA:ANCHOR:{}",
            sha256_json(meaning)[..24].to_uppercase()
        ),
        label: "참조 맥락".into(),
        semantic_type: ApprovedSemanticTypeIR::Concept,
    };
    let reference_rows = [
        meaning
            .get("reference_day")
            .and_then(Value::as_str)
            .map(|day| (ApprovedRelationTypeIR::Date, text_value(day))),
        meaning
            .get("reference_time")
            .and_then(Value::as_str)
            .map(|time| (ApprovedRelationTypeIR::Time, time_value(time))),
        meaning
            .get("reference_place")
            .and_then(Value::as_str)
            .map(|place| {
                (
                    ApprovedRelationTypeIR::Location,
                    lexical_value(place, ApprovedSemanticTypeIR::Location),
                )
            }),
    ];
    for (offset, (relation_type, value)) in reference_rows.into_iter().flatten().enumerate() {
        let proposition_id = format!("PERSONA:ANCHOR:{offset}");
        bindings.push(ApprovedClaimBindingIR {
            proposition_id: proposition_id.clone(),
            subject: anchor.node_id.clone(),
            property: relation_symbol(relation_type).into(),
            value: value.0,
            polarity: true,
        });
        compositional.push(ApprovedCompositionalClaimIR {
            proposition_id,
            subject: ApprovedLexicalNodeIR {
                node_id: anchor.node_id.clone(),
                semantic_type: anchor.semantic_type,
                canonical_lexical_label: anchor.label.clone(),
            },
            relation: relation_type,
            value: value.1,
            polarity: true,
            modality: ApprovedModalityIR::Asserted,
    status_frame: None,
});
    }
    Ok((bindings, compositional, relation))
}

fn approve_claims(
    goal: &SemanticPlanGoalIR,
    bundle_sha: &str,
    claims: Vec<ApprovedClaimBindingIR>,
    relation: ApprovedDiscourseRelationIR,
    style: ApprovedResponseStyleIR,
) -> Result<crate::ApprovedResponseIR, String> {
    let evidence = claims
        .iter()
        .enumerate()
        .map(|(index, claim)| EvidenceIR {
            evidence_id: format!("PERSONA:GOLD:EVIDENCE:{index}"),
            literal: LiteralIR {
                proposition_id: claim.proposition_id.clone(),
                value: claim.polarity,
            },
            reliability_millis: 1_000,
            source_ref: format!("SEMANTIC_PLAN:{}:{bundle_sha}", goal.semantic_sha256),
        })
        .collect::<Vec<_>>();
    let goals = claims
        .iter()
        .map(|claim| LiteralIR {
            proposition_id: claim.proposition_id.clone(),
            value: claim.polarity,
        })
        .collect::<Vec<_>>();
    let request = DeliberationRequestIR {
        schema: DELIBERATION_REQUEST_SCHEMA.into(),
        request_id: format!("PERSONA:GOLD:{}", &goal.semantic_sha256[..24]),
        subject: "approved_persona_gold_inform".into(),
        evidence,
        mechanisms: Vec::new(),
        goals,
        authority_envelope: Default::default(),
        immutable_constraints: Vec::new(),
        max_depth: 1,
        beam_width: 1,
        max_hypotheses: 1,
        max_counterfactuals: 1,
    };
    let deliberation = DeliberationEngine
        .deliberate(&request)
        .map_err(|_| "PERSONA_GOLD_DELIBERATION_FAILED")?;
    ApprovedResponseBuilder
        .approve(ApprovedResponseCandidateIR {
            world_state_sha256: goal.semantic_sha256.clone(),
            request,
            deliberation,
            speech_act: ApprovedSpeechActIR::Inform,
            claims,
            discourse_relation: relation,
            style,
        })
        .map_err(|_| "PERSONA_GOLD_APPROVAL_FAILED".into())
}

fn style_from_persona(
    persona: &serde_json::Map<String, Value>,
) -> Result<ApprovedResponseStyleIR, String> {
    let register = match required_string(persona, "register")? {
        "FORMAL" => LanguageRegisterIR::Formal,
        "INFORMAL" => LanguageRegisterIR::Informal,
        "POLITE" => LanguageRegisterIR::Neutral,
        _ => return Err("UNSUPPORTED_PERSONA_REGISTER".into()),
    };
    Ok(ApprovedResponseStyleIR {
        register,
        verbosity: ApprovedVerbosityIR::Short,
    })
}

fn split_from_source(value: &str) -> Result<CanonicalResponseDatasetSplitIR, String> {
    match value {
        "TRAIN" => Ok(CanonicalResponseDatasetSplitIR::Train),
        "VALIDATION" => Ok(CanonicalResponseDatasetSplitIR::Validation),
        "BLIND" => Ok(CanonicalResponseDatasetSplitIR::Blind),
        _ => Err("SOURCE_SPLIT_INVALID".into()),
    }
}

fn lexical_state(value: &str) -> Result<(ApprovedValueIR, ApprovedOpenValueIR), String> {
    let label = match value {
        "SCHEDULED" => "예정됨",
        "CANCELLED" => "취소됨",
        "RECEIVED" => "수신됨",
        "SENT" => "전송됨",
        "DELAYED" => "지연됨",
        "MOVED" => "이동됨",
        "READY" => "준비됨",
        "REMINDER" => "알림 예정",
        "AVAILABLE" => "사용 가능",
        "STARTED" => "시작됨",
        "UPDATED" => "갱신됨",
        "CONFIRMED" => "확정됨",
        "PAYMENT_COMPLETE" => "결제 완료",
        _ => return Err("UNSUPPORTED_INFORM_STATE".into()),
    };
    Ok(lexical_value(label, ApprovedSemanticTypeIR::State))
}

fn weather_label(value: &str) -> String {
    match value {
        "RAIN_LIKELY" => "비가 올 가능성이 있음".into(),
        _ => value.to_string(),
    }
}

fn reason_label(value: &str) -> String {
    match value {
        "TRAFFIC" => "교통 상황".into(),
        _ => value.to_string(),
    }
}

fn channel_label(value: &str) -> String {
    match value {
        "EMAIL" => "이메일".into(),
        _ => value.to_string(),
    }
}

fn lexical_value(
    label: &str,
    semantic_type: ApprovedSemanticTypeIR,
) -> (ApprovedValueIR, ApprovedOpenValueIR) {
    let node_id = format!("PERSONA:VALUE:{}", sha256_text(label)[..24].to_uppercase());
    (
        ApprovedValueIR::Symbol(node_id.clone()),
        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
            node_id,
            semantic_type,
            canonical_lexical_label: label.into(),
        }),
    )
}

fn text_value(value: &str) -> (ApprovedValueIR, ApprovedOpenValueIR) {
    (
        ApprovedValueIR::Text(value.into()),
        ApprovedOpenValueIR::Text(value.into()),
    )
}

fn integer_value(value: i64) -> (ApprovedValueIR, ApprovedOpenValueIR) {
    (
        ApprovedValueIR::Integer(value),
        ApprovedOpenValueIR::Integer(value),
    )
}

fn time_value(value: &str) -> (ApprovedValueIR, ApprovedOpenValueIR) {
    if let Some((hour, minute)) = parse_korean_clock(value) {
        (
            ApprovedValueIR::Clock { hour, minute },
            ApprovedOpenValueIR::Clock { hour, minute },
        )
    } else {
        text_value(value)
    }
}

fn parse_korean_clock(value: &str) -> Option<(u8, u8)> {
    let normalized = value.trim();
    let (meridiem, rest) = normalized.split_once(' ')?;
    let (hour_text, minute_text) = rest.split_once('시')?;
    let hour = hour_text.trim().parse::<u8>().ok()?;
    let minute = minute_text
        .trim()
        .strip_suffix('분')
        .filter(|text| !text.trim().is_empty())
        .map(|text| text.trim().parse::<u8>().ok())
        .flatten()
        .unwrap_or(0);
    let hour = match meridiem {
        "오전" if hour <= 11 => hour,
        "오후" if (1..=11).contains(&hour) => hour + 12,
        "오후" if hour == 12 => hour,
        _ => return None,
    };
    (hour < 24 && minute < 60).then_some((hour, minute))
}

fn relation_symbol(relation: ApprovedRelationTypeIR) -> &'static str {
    match relation {
        ApprovedRelationTypeIR::Status => "status",
        ApprovedRelationTypeIR::Time => "time",
        ApprovedRelationTypeIR::Location => "location",
        ApprovedRelationTypeIR::Duration => "duration",
        ApprovedRelationTypeIR::Date => "date",
        ApprovedRelationTypeIR::Manner => "manner",
        _ => unreachable!("persona grounding only emits supported relations"),
    }
}

fn required_object<'a>(
    value: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{key}_MISSING"))
}

fn required_string<'a>(
    value: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{key}_MISSING"))
}

fn sha256_json<T: Serialize>(value: &T) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("persona grounding serializes"))
    )
}

fn sha256_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn sha256_shape(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grounds_inform_through_planner_and_replay_without_trusting_surface() {
        let core = DockableCore::load_embedded().unwrap();
        let row = serde_json::json!({
            "example_id":"UNIT-001", "record_sha256":"a".repeat(64), "split":"TRAIN",
            "approved_meaning":{"act":"INFORM","event":"회의","operation":"REVISED","time":"오후 4시"},
            "persona":{"register":"POLITE","relationship":"PEER","voice":"GENTLE"},
            "surface":"회의 시간이 오후 4시로 바뀌었어요."
        });
        let grounded = ground_synthetic_persona_gold_row(&core, &row).unwrap();
        assert!(grounded.validate());
        assert!(
            grounded
                .canonical_record
                .approved_response
                .approval_replay_verified
        );
        assert_eq!(grounded.canonical_record.approved_response.claims.len(), 1);
        assert_eq!(
            grounded
                .canonical_record
                .approved_response
                .discourse_relation,
            ApprovedDiscourseRelationIR::Correction
        );
    }

    #[test]
    fn fails_closed_for_an_unimplemented_speech_act() {
        let core = DockableCore::load_embedded().unwrap();
        let row = serde_json::json!({
            "example_id":"UNIT-002", "record_sha256":"a".repeat(64), "split":"TRAIN",
            "approved_meaning":{"act":"QUERY","event":"회의"},
            "persona":{"register":"POLITE","relationship":"PEER","voice":"GENTLE"}, "surface":"회의는 언제예요?"
        });
        assert_eq!(
            ground_synthetic_persona_gold_row(&core, &row),
            Err("UNSUPPORTED_SPEECH_ACT".into())
        );
    }
}
