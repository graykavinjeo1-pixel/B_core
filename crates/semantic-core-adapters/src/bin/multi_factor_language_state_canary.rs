//! Controlled one-factor language-state contrast canary.
//!
//! Every record in this report uses one Approved Response IR and changes one
//! explicit realization-state condition at a time.  The report is research
//! evidence only: it never promotes a state variable to a language operator.

use semantic_core_adapters::{
    ApprovedCompositionalResponseIR, CanonicalResponseDatasetSplitIR,
    CanonicalResponseTrainingCampaignIR, ConstructionRuntime, KoreanDialectIR, LanguageCodeIR,
    LanguageRegisterIR, LanguageStateConditionIR, PersonaConstructionContextIR,
    PersonaConstructionModeIR, RoleplayAgeBandIR, RoleplayBackgroundIR, RoleplayPersonaIR,
    RoleplayRelationshipIR, RoleplaySceneIR, RoleplayVoiceIR,
};
use serde::Serialize;
use serde_json::json;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
struct ContrastVariant {
    value: String,
    output: serde_json::Value,
}

fn context(
    mode: PersonaConstructionModeIR,
    persona: RoleplayPersonaIR,
    persona_label: Option<&str>,
) -> PersonaConstructionContextIR {
    let controller_active = std::env::var_os("BCORE_LANGUAGE_STATE_CONTROLLER_OUTPUT").is_some();
    PersonaConstructionContextIR {
        mode,
        persona,
        persona_label: (!controller_active)
            .then(|| persona_label.map(str::to_string))
            .flatten(),
        verbosity_delta_millis: 0,
        latent_operator_deltas_millis: Vec::new(),
        language_state: LanguageStateConditionIR::default(),
    }
}

fn run_variant(
    runtime: &ConstructionRuntime,
    source: &ApprovedCompositionalResponseIR,
    value: &str,
    context: PersonaConstructionContextIR,
) -> Result<ContrastVariant, String> {
    let output = runtime.realize(source, LanguageCodeIR::Korean, context)?;
    let inverse_ok = output.validate(source);
    if !inverse_ok {
        return Err(format!("SEMANTIC_INVERSE_FAILED:{value}"));
    }
    Ok(ContrastVariant {
        value: value.into(),
        output: json!({
            "markdown": output.markdown,
            "output_sha256": output.output_sha256,
            "semantic_inverse": inverse_ok,
            "context": output.context,
            "marker": output.marker,
            "language_state_projection": output.language_state_projection,
        }),
    })
}

fn route_state(
    factor: &str,
    value: &str,
    mut context: PersonaConstructionContextIR,
) -> PersonaConstructionContextIR {
    let mut state = LanguageStateConditionIR::default();
    match factor {
        "personality" => state.personality = Some(value.into()),
        "current_speaker_emotion" => state.speaker_emotion = Some(value.into()),
        "inferred_user_emotion" => state.inferred_user_emotion = Some(value.into()),
        "relationship_social_distance" => state.relationship = Some(context.persona.relationship),
        "dialogue_context" => state.dialogue_context = Some(context.persona.scene),
        "age_register_condition" => state.age_band = Some(context.persona.age_band),
        "register_condition" => {
            state.register = match value {
                "FORMAL" => Some(LanguageRegisterIR::Formal),
                "INFORMAL" => Some(LanguageRegisterIR::Informal),
                _ => Some(LanguageRegisterIR::Neutral),
            }
        }
        "region_dialect_condition" => {
            state.korean_dialect = match value {
                "BASELINE" | "STANDARD" => Some(KoreanDialectIR::Standard),
                "GYEONGSANG" => Some(KoreanDialectIR::Gyeongsang),
                "CHUNGCHEONG" => Some(KoreanDialectIR::Chungcheong),
                _ => None,
            }
        }
        _ => {}
    }
    context.language_state = state;
    context
}

fn one_factor(
    runtime: &ConstructionRuntime,
    source: &ApprovedCompositionalResponseIR,
    name: &str,
    baseline: PersonaConstructionContextIR,
    variants: Vec<(&str, PersonaConstructionContextIR)>,
    note: &str,
) -> Result<serde_json::Value, String> {
    let mut rows = vec![run_variant(
        runtime,
        source,
        "BASELINE",
        route_state(name, "BASELINE", baseline),
    )?];
    for (value, context) in variants {
        rows.push(run_variant(
            runtime,
            source,
            value,
            route_state(name, value, context),
        )?);
    }
    Ok(json!({
        "factor": name,
        "one_factor_at_a_time": true,
        "note": note,
        "variants": rows,
    }))
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = std::env::var_os("BCORE_CANONICAL_TRAINING_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\B_Core_validation\canonical_response_training"));
    let contrastive_root = std::env::var_os("BCORE_PERSONA_CONTRASTIVE_TRAINING_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\B_Core_validation\persona_contrastive_training_v1"));
    let model_path = root.join("canonical_response_codec_model_ko_v1.json");
    let contrastive_path =
        contrastive_root.join("persona_contrastive_construction_model_ko_v1.json");
    let campaign: CanonicalResponseTrainingCampaignIR = serde_json::from_slice(&fs::read(
        root.join("canonical_plan_description_campaign_ko_v1.json"),
    )?)?;
    if !campaign.validate() {
        return Err("CAMPAIGN_INVALID".into());
    }
    let requested_split = std::env::var("BCORE_MULTI_FACTOR_SPLIT")
        .unwrap_or_else(|_| "BLIND".into())
        .to_ascii_uppercase();
    let requested_split = match requested_split.as_str() {
        "TRAIN" => CanonicalResponseDatasetSplitIR::Train,
        "VALIDATION" | "VAL" => CanonicalResponseDatasetSplitIR::Validation,
        "BLIND" => CanonicalResponseDatasetSplitIR::Blind,
        _ => return Err("MULTI_FACTOR_SPLIT_INVALID".into()),
    };
    let record_index = std::env::var("BCORE_MULTI_FACTOR_RECORD_INDEX")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let source_record = campaign
        .records
        .iter()
        .filter(|record| record.split == requested_split)
        .nth(record_index)
        .ok_or("MULTI_FACTOR_SOURCE_MISSING")?;
    let latent_path = std::env::var_os("BCORE_PERSONA_LATENT_OPERATOR_OUTPUT")
        .map(PathBuf::from)
        .map(|path| path.join("persona_latent_operator_residual_discovery_ko_v1.json"));
    let controller_path = std::env::var_os("BCORE_LANGUAGE_STATE_CONTROLLER_OUTPUT")
        .map(PathBuf::from)
        .map(|path| path.join("language_state_controller_ko_v1.json"));
    let inventory_path = std::env::var_os("BCORE_LANGUAGE_STATE_EXPRESSION_INVENTORY_OUTPUT")
        .map(PathBuf::from)
        .map(|path| path.join("expression_construction_inventory_shadow_ko_v1.json"));
    let runtime = match (
        latent_path.as_deref(),
        controller_path.as_deref(),
        inventory_path.as_deref(),
    ) {
        (Some(latent), Some(controller), Some(inventory)) => {
            ConstructionRuntime::load_from_paths_with_latent_controller_and_inventory(
                &model_path,
                &contrastive_path,
                latent,
                controller,
                inventory,
            )?
        }
        (Some(latent), Some(controller), _) => {
            ConstructionRuntime::load_from_paths_with_latent_and_controller(
                &model_path,
                &contrastive_path,
                latent,
                controller,
            )?
        }
        (Some(latent), None, _) => ConstructionRuntime::load_from_paths_with_latent(
            &model_path,
            &contrastive_path,
            latent,
        )?,
        _ => ConstructionRuntime::load_from_paths(&model_path, &contrastive_path)?,
    };
    let source = &source_record.approved_response;

    let default_persona = RoleplayPersonaIR::default();
    let factors = vec![
        one_factor(
            &runtime,
            source,
            "personality",
            context(PersonaConstructionModeIR::DefaultPolite, default_persona, None),
            vec![
                ("INTJ", context(PersonaConstructionModeIR::DefaultPolite, default_persona, Some("INTJ"))),
                ("ENFP", context(PersonaConstructionModeIR::DefaultPolite, default_persona, Some("ENFP"))),
            ],
            "Contrastive profile label only; canonical meaning and all runtime social fields remain fixed.",
        )?,
        one_factor(
            &runtime,
            source,
            "current_speaker_emotion",
            context(PersonaConstructionModeIR::DefaultPolite, default_persona, None),
            vec![
                ("CALM", context(PersonaConstructionModeIR::CalmSupportive, RoleplayPersonaIR { voice: RoleplayVoiceIR::Calm, ..default_persona }, None)),
                ("LIVELY", context(PersonaConstructionModeIR::FriendlyPolite, RoleplayPersonaIR { voice: RoleplayVoiceIR::Lively, ..default_persona }, None)),
                ("GENTLE", context(PersonaConstructionModeIR::CalmSupportive, RoleplayPersonaIR { voice: RoleplayVoiceIR::Gentle, ..default_persona }, None)),
            ],
            "Current emotion is represented only by the explicit bounded voice/mode controls available to this runtime.",
        )?,
        one_factor(
            &runtime,
            source,
            "inferred_user_emotion",
            context(PersonaConstructionModeIR::DefaultPolite, RoleplayPersonaIR { scene: RoleplaySceneIR::Everyday, ..default_persona }, None),
            vec![
                ("TENSE", context(PersonaConstructionModeIR::DefaultPolite, RoleplayPersonaIR { scene: RoleplaySceneIR::Tense, ..default_persona }, None)),
                ("COMPANION", context(PersonaConstructionModeIR::DefaultPolite, RoleplayPersonaIR { scene: RoleplaySceneIR::Companion, ..default_persona }, None)),
            ],
            "Scene is an explicit interaction-state proxy; no user emotion is inferred from the approved meaning.",
        )?,
        one_factor(
            &runtime,
            source,
            "relationship_social_distance",
            context(PersonaConstructionModeIR::Concise, RoleplayPersonaIR { relationship: RoleplayRelationshipIR::Peer, ..default_persona }, None),
            vec![
                ("PROFESSIONAL", context(PersonaConstructionModeIR::Concise, RoleplayPersonaIR { relationship: RoleplayRelationshipIR::Professional, ..default_persona }, None)),
                ("CLOSE", context(PersonaConstructionModeIR::Concise, RoleplayPersonaIR { relationship: RoleplayRelationshipIR::Close, ..default_persona }, None)),
            ],
            "Relationship varies while concise mode and all other persona fields remain fixed.",
        )?,
        one_factor(
            &runtime,
            source,
            "dialogue_context",
            context(PersonaConstructionModeIR::DefaultPolite, RoleplayPersonaIR { scene: RoleplaySceneIR::Everyday, ..default_persona }, None),
            vec![
                ("SERVICE", context(PersonaConstructionModeIR::DefaultPolite, RoleplayPersonaIR { scene: RoleplaySceneIR::Service, ..default_persona }, None)),
                ("TENSE", context(PersonaConstructionModeIR::DefaultPolite, RoleplayPersonaIR { scene: RoleplaySceneIR::Tense, ..default_persona }, None)),
                ("COMPANION", context(PersonaConstructionModeIR::DefaultPolite, RoleplayPersonaIR { scene: RoleplaySceneIR::Companion, ..default_persona }, None)),
            ],
            "Explicit scene condition only; meaning remains supplied by the Approved Response IR.",
        )?,
        one_factor(
            &runtime,
            source,
            "age_register_condition",
            context(PersonaConstructionModeIR::DefaultPolite, RoleplayPersonaIR { age_band: RoleplayAgeBandIR::Adult, ..default_persona }, None),
            vec![
                ("YOUNG", context(PersonaConstructionModeIR::DefaultPolite, RoleplayPersonaIR { age_band: RoleplayAgeBandIR::Young, ..default_persona }, None)),
                ("ELDER", context(PersonaConstructionModeIR::DefaultPolite, RoleplayPersonaIR { age_band: RoleplayAgeBandIR::Elder, ..default_persona }, None)),
            ],
            "Age band is held as explicit metadata; register is locked by DefaultPolite for this contrast.",
        )?,
        one_factor(
            &runtime,
            source,
            "register_condition",
            context(PersonaConstructionModeIR::DefaultPolite, default_persona, None),
            vec![
                ("FORMAL", context(PersonaConstructionModeIR::DefaultPolite, default_persona, None)),
                ("INFORMAL", context(PersonaConstructionModeIR::DefaultPolite, default_persona, None)),
            ],
            "Register is routed as typed state while the existing mode remains fixed.",
        )?,
        one_factor(
            &runtime,
            source,
            "region_dialect_condition",
            context(PersonaConstructionModeIR::DefaultPolite, default_persona, None),
            vec![
                ("STANDARD", context(PersonaConstructionModeIR::DefaultPolite, default_persona, None)),
                ("GYEONGSANG", context(PersonaConstructionModeIR::DefaultPolite, default_persona, None)),
                ("CHUNGCHEONG", context(PersonaConstructionModeIR::DefaultPolite, default_persona, None)),
            ],
            "Dialect is recorded as an explicit condition, but this construction runtime has no dialect field; zero effect is an expected diagnostic.",
        )?,
    ];

    let blind_composition = run_variant(
        &runtime,
        source,
        "UNSEEN_COMPOSITION",
        context(
            PersonaConstructionModeIR::CalmSupportive,
            RoleplayPersonaIR {
                relationship: RoleplayRelationshipIR::Close,
                scene: RoleplaySceneIR::Tense,
                voice: RoleplayVoiceIR::Gentle,
                age_band: RoleplayAgeBandIR::Elder,
                background: RoleplayBackgroundIR::Community,
                ..default_persona
            },
            Some("ENFP"),
        ),
    )?;
    let report = json!({
        "schema": "BCORE.MULTI_FACTOR_LANGUAGE_STATE_DISCOVERY.V1",
        "source_split": format!("{requested_split:?}").to_ascii_uppercase(),
        "source_record_id": source_record.record_id,
        "source_response_sha256": source.semantic_sha256,
        "factor_count": factors.len(),
        "factors": factors,
        "blind_multi_factor_composition": blind_composition,
        "promotion": "DISCOVERY_ONLY_NO_OPERATOR_PROMOTION",
        "meaning_authority": "APPROVED_CANONICAL_RESPONSE_IR",
    });
    let output_root = std::env::var_os("BCORE_MULTI_FACTOR_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\B_Core_validation\multi_factor_language_state_v1"));
    fs::create_dir_all(&output_root)?;
    let output_path = std::env::var_os("BCORE_MULTI_FACTOR_OUTPUT_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| output_root.join("multi_factor_language_state_canary_ko_v1.json"));
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
