//! Restart-persistence and runtime-realization canary for the learned codec.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use semantic_core_adapters::{
    compositional_response_sha256, ApprovedCompositionalClaimIR, ApprovedCompositionalResponseIR,
    ApprovedDiscourseRelationIR, ApprovedLexicalNodeIR, ApprovedModalityIR, ApprovedOpenValueIR,
    ApprovedOperationIR, ApprovedRelationTypeIR, ApprovedResponseStyleIR, ApprovedSemanticTypeIR,
    ApprovedSpeechActIR, ApprovedVerbosityIR, CanonicalResponseDatasetSplitIR,
    CanonicalResponseTrainingCampaignIR, ConstructionRuntime, LanguageCodeIR, LanguageRegisterIR,
    LanguageStateConditionIR, PersonaConstructionContextIR, PersonaConstructionModeIR,
    RoleplayPersonaIR, RoleplayRelationshipIR, RoleplayVoiceIR,
    COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

fn main() -> Result<(), Box<dyn Error>> {
    let root = std::env::var_os("BCORE_CANONICAL_TRAINING_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\B_Core_validation\canonical_response_training"));
    let model_path = root.join("canonical_response_codec_model_ko_v1.json");
    let campaign_path = root.join("canonical_plan_description_campaign_ko_v1.json");
    let contrastive_path = std::env::var_os("BCORE_PERSONA_CONTRASTIVE_TRAINING_OUTPUT")
        .map(PathBuf::from)
        .map(|root| root.join("persona_contrastive_construction_model_ko_v1.json"));
    let latent_path = std::env::var_os("BCORE_PERSONA_LATENT_OPERATOR_OUTPUT")
        .map(PathBuf::from)
        .map(|root| root.join("persona_latent_operator_residual_discovery_ko_v1.json"));
    let runtime = match (contrastive_path.as_deref(), latent_path.as_deref()) {
        (Some(contrastive), Some(latent)) => {
            ConstructionRuntime::load_from_paths_with_latent(&model_path, contrastive, latent)?
        }
        (Some(contrastive), None) => {
            ConstructionRuntime::load_from_paths(&model_path, contrastive)?
        }
        (None, _) => ConstructionRuntime::load_from_path(&model_path)?,
    };
    let campaign =
        serde_json::from_slice::<CanonicalResponseTrainingCampaignIR>(&fs::read(campaign_path)?)?;
    if !campaign.validate() {
        return Err("BLIND_CAMPAIGN_ARTIFACT_INVALID".into());
    }
    let source = time_revision();
    let mut contexts = vec![
        (
            "DEFAULT_POLITE",
            PersonaConstructionContextIR {
                mode: PersonaConstructionModeIR::DefaultPolite,
                persona: RoleplayPersonaIR::default(),
                persona_label: contrastive_path.is_some().then(|| "INTJ".into()),
                verbosity_delta_millis: 0,
                latent_operator_deltas_millis: Vec::new(),
                language_state: LanguageStateConditionIR::default(),
            },
        ),
        (
            "FRIENDLY_POLITE",
            PersonaConstructionContextIR {
                mode: PersonaConstructionModeIR::FriendlyPolite,
                persona: RoleplayPersonaIR {
                    relationship: RoleplayRelationshipIR::Peer,
                    ..RoleplayPersonaIR::default()
                },
                persona_label: contrastive_path.is_some().then(|| "ENFP".into()),
                verbosity_delta_millis: 0,
                latent_operator_deltas_millis: Vec::new(),
                language_state: LanguageStateConditionIR::default(),
            },
        ),
        (
            "CONCISE",
            PersonaConstructionContextIR {
                mode: PersonaConstructionModeIR::Concise,
                persona: RoleplayPersonaIR {
                    relationship: RoleplayRelationshipIR::Close,
                    voice: RoleplayVoiceIR::Direct,
                    ..RoleplayPersonaIR::default()
                },
                persona_label: contrastive_path.is_some().then(|| "ISTP".into()),
                verbosity_delta_millis: 0,
                latent_operator_deltas_millis: Vec::new(),
                language_state: LanguageStateConditionIR::default(),
            },
        ),
        (
            "EXPLANATORY",
            PersonaConstructionContextIR {
                mode: PersonaConstructionModeIR::Explanatory,
                persona: RoleplayPersonaIR {
                    relationship: RoleplayRelationshipIR::Professional,
                    ..RoleplayPersonaIR::default()
                },
                persona_label: contrastive_path.is_some().then(|| "ISFJ".into()),
                verbosity_delta_millis: 0,
                latent_operator_deltas_millis: Vec::new(),
                language_state: LanguageStateConditionIR::default(),
            },
        ),
        (
            "CALM_SUPPORTIVE",
            PersonaConstructionContextIR {
                mode: PersonaConstructionModeIR::CalmSupportive,
                persona: RoleplayPersonaIR {
                    relationship: RoleplayRelationshipIR::Caregiving,
                    voice: RoleplayVoiceIR::Gentle,
                    ..RoleplayPersonaIR::default()
                },
                persona_label: contrastive_path.is_some().then(|| "COARSE_ELDER".into()),
                verbosity_delta_millis: 0,
                latent_operator_deltas_millis: Vec::new(),
                language_state: LanguageStateConditionIR::default(),
            },
        ),
    ];
    if contrastive_path.is_some() {
        for label in [
            "ENTJ", "ENTP", "ESFJ", "ESFP", "ESTJ", "ESTP", "INFJ", "INFP", "INTP", "ISFP", "ISTJ",
            "ISTP",
        ] {
            contexts.push((
                label,
                PersonaConstructionContextIR {
                    mode: PersonaConstructionModeIR::DefaultPolite,
                    persona: RoleplayPersonaIR::default(),
                    persona_label: Some(label.into()),
                    verbosity_delta_millis: 0,
                    latent_operator_deltas_millis: Vec::new(),
                    language_state: LanguageStateConditionIR::default(),
                },
            ));
        }
    }
    let outputs = contexts
        .into_iter()
        .map(|(label, context)| {
            let output = runtime.realize(&source, LanguageCodeIR::Korean, context)?;
            output
                .validate(&source)
                .then_some(serde_json::json!({
                    "condition": label,
                    "markdown": output.markdown,
                    "inverse_claim_count": output.semantic_inverse.claims.len(),
                    "inverse_time": output.semantic_inverse.claims[0].value,
                    "contrastive_profile": output.contrastive_profile,
                    "output_sha256": output.output_sha256,
                }))
                .ok_or_else(|| "PERSONA_CANARY_INVERSE_FAILED".into())
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let surfaces = outputs
        .iter()
        .filter_map(|output| output["markdown"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if surfaces.len() < 4 {
        return Err("PERSONA_CANARY_VARIATION_INSUFFICIENT".into());
    }
    if contrastive_path.is_some()
        && outputs
            .iter()
            .any(|output| output["contrastive_profile"].is_null())
    {
        return Err("PERSONA_CONTRASTIVE_PROFILE_NOT_APPLIED".into());
    }

    let verbosity_deltas = [-1_000_i16, -500, 0, 500, 1_000];
    let verbosity_outputs = verbosity_deltas
        .into_iter()
        .map(|delta| {
            let output = runtime.realize(
                &source,
                LanguageCodeIR::Korean,
                PersonaConstructionContextIR {
                    mode: PersonaConstructionModeIR::DefaultPolite,
                    persona: RoleplayPersonaIR::default(),
                    persona_label: None,
                    verbosity_delta_millis: delta,
                    latent_operator_deltas_millis: Vec::new(),
                    language_state: LanguageStateConditionIR::default(),
                },
            )?;
            if !output.validate(&source) {
                return Err("VERBOSITY_CAUSAL_SEMANTIC_INVARIANCE_FAILED".into());
            }
            Ok(serde_json::json!({
                "delta_millis": delta,
                "chars": output.markdown.chars().count(),
                "markdown": output.markdown,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let verbosity_lengths = verbosity_outputs
        .iter()
        .filter_map(|output| output["chars"].as_u64())
        .collect::<Vec<_>>();
    if verbosity_lengths
        .windows(2)
        .any(|window| window[0] > window[1])
        || verbosity_outputs
            .iter()
            .any(|output| output["markdown"].as_str().unwrap_or("").trim().is_empty())
    {
        return Err("VERBOSITY_CAUSAL_MONOTONICITY_FAILED".into());
    }

    let latent_probe_outputs = if latent_path.is_some() {
        (0..4)
            .map(|latent_index| {
                let outputs = [-1_000_i16, -500, 0, 500, 1_000]
                    .into_iter()
                    .map(|delta| {
                        let mut intervention = vec![0_i16; latent_index + 1];
                        intervention[latent_index] = delta;
                        let output = runtime.realize(
                            &source,
                            LanguageCodeIR::Korean,
                            PersonaConstructionContextIR {
                                mode: PersonaConstructionModeIR::DefaultPolite,
                                persona: RoleplayPersonaIR::default(),
                                persona_label: None,
                                verbosity_delta_millis: 0,
                                latent_operator_deltas_millis: intervention,
                                language_state: LanguageStateConditionIR::default(),
                            },
                        )?;
                        if !output.validate(&source) {
                            return Err("LATENT_CAUSAL_SEMANTIC_INVARIANCE_FAILED".into());
                        }
                        Ok(serde_json::json!({
                            "delta_millis": delta,
                            "features": surface_features(&output.markdown),
                            "semantic_inverse": true,
                            "markdown": output.markdown,
                        }))
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Ok(serde_json::json!({
                    "latent_index": latent_index + 1,
                    "intervention_values": [-1.0, -0.5, 0.0, 0.5, 1.0],
                    "outputs": outputs,
                    "semantic_invariance": true,
                    "runtime_operator": "COMPOSITE_UNNAMED_LATENT",
                }))
            })
            .collect::<Result<Vec<_>, String>>()?
    } else {
        Vec::new()
    };

    let composition = blind_three_clause_composition();
    let composition_output = runtime.realize(
        &composition,
        LanguageCodeIR::Korean,
        PersonaConstructionContextIR {
            mode: PersonaConstructionModeIR::FriendlyPolite,
            persona: RoleplayPersonaIR::default(),
            persona_label: contrastive_path.is_some().then(|| "ENFP".into()),
            verbosity_delta_millis: 0,
            latent_operator_deltas_millis: Vec::new(),
            language_state: LanguageStateConditionIR::default(),
        },
    )?;
    if !composition_output.validate(&composition)
        || composition_output.semantic_inverse.claims.len() != 3
    {
        return Err("BLIND_COMPOSITION_CANARY_FAILED".into());
    }
    let composition_lengths = [-1_000_i16, -500, 0, 500, 1_000]
        .into_iter()
        .map(|delta| {
            runtime
                .realize(
                    &composition,
                    LanguageCodeIR::Korean,
                    PersonaConstructionContextIR {
                        mode: PersonaConstructionModeIR::DefaultPolite,
                        persona: RoleplayPersonaIR::default(),
                        persona_label: None,
                        verbosity_delta_millis: delta,
                        latent_operator_deltas_millis: Vec::new(),
                        language_state: LanguageStateConditionIR::default(),
                    },
                )
                .map(|output| {
                    if !output.validate(&composition) {
                        return Err("VERBOSITY_COMPOSITION_INVARIANCE_FAILED".to_string());
                    }
                    Ok(output.markdown.chars().count())
                })
                .and_then(|result| result)
        })
        .collect::<Result<Vec<_>, String>>()?;
    if composition_lengths
        .windows(2)
        .any(|window| window[0] > window[1])
    {
        return Err("VERBOSITY_COMPOSITION_MONOTONICITY_FAILED".into());
    }
    let blind_record = campaign
        .records
        .iter()
        .find(|record| record.split == CanonicalResponseDatasetSplitIR::Blind)
        .ok_or("BLIND_RECORD_MISSING")?;
    let blind_output = runtime.realize(
        &blind_record.approved_response,
        LanguageCodeIR::Korean,
        PersonaConstructionContextIR {
            mode: PersonaConstructionModeIR::CalmSupportive,
            persona: RoleplayPersonaIR {
                relationship: RoleplayRelationshipIR::Caregiving,
                voice: RoleplayVoiceIR::Gentle,
                ..RoleplayPersonaIR::default()
            },
            persona_label: contrastive_path.is_some().then(|| "ISFJ".into()),
            verbosity_delta_millis: 0,
            latent_operator_deltas_millis: Vec::new(),
            language_state: LanguageStateConditionIR::default(),
        },
    )?;
    if !blind_output.validate(&blind_record.approved_response) {
        return Err("SEALED_BLIND_RUNTIME_INVERSE_FAILED".into());
    }
    let blind_verbosity_outputs = [-1_000_i16, -500, 0, 500, 1_000]
        .into_iter()
        .map(|delta| {
            runtime
                .realize(
                    &blind_record.approved_response,
                    LanguageCodeIR::Korean,
                    PersonaConstructionContextIR {
                        mode: PersonaConstructionModeIR::DefaultPolite,
                        persona: RoleplayPersonaIR::default(),
                        persona_label: None,
                        verbosity_delta_millis: delta,
                        latent_operator_deltas_millis: Vec::new(),
                        language_state: LanguageStateConditionIR::default(),
                    },
                )
                .map(|output| {
                    if !output.validate(&blind_record.approved_response) {
                        return Err("BLIND_VERBOSITY_SEMANTIC_INVARIANCE_FAILED".to_string());
                    }
                    Ok(serde_json::json!({
                        "delta_millis": delta,
                        "chars": output.markdown.chars().count(),
                        "markdown": output.markdown,
                    }))
                })
                .and_then(|result| result)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let blind_lengths = blind_verbosity_outputs
        .iter()
        .filter_map(|output| output["chars"].as_u64())
        .collect::<Vec<_>>();
    if blind_lengths.windows(2).any(|window| window[0] > window[1]) {
        return Err("BLIND_VERBOSITY_MONOTONICITY_FAILED".into());
    }
    let report = serde_json::json!({
        "schema": "BCORE.CANONICAL_RESPONSE_CODEC_SHADOW_CANARY.V1",
        "model_sha256": runtime.model_sha256(),
        "contrastive_runtime_loaded": contrastive_path.is_some(),
        "restart_loaded": true,
        "source_response_sha256": source.semantic_sha256,
        "persona_outputs": outputs,
        "distinct_persona_surfaces": surfaces.len(),
        "persona_profile_count": outputs
            .iter()
            .filter(|output| !output["contrastive_profile"].is_null())
            .count(),
        "verbosity_causal": {
            "candidate": "LATENT_01_VERBOSITY_CANDIDATE",
            "runtime_promotion": "HOLD",
            "outputs": verbosity_outputs,
            "monotonic_chars": true,
            "semantic_invariance": true,
        },
        "residual_latent_causal_probe": {
            "runtime_loaded": latent_path.is_some(),
            "outputs": latent_probe_outputs,
            "promotion": "HOLD_UNTIL_TRAIN_VALIDATION_BLIND_EFFECT_DIRECTION_PASS",
        },
        "semantic_invariance": true,
        "blind_three_clause": {
            "markdown": composition_output.markdown,
            "inverse_claim_count": composition_output.semantic_inverse.claims.len(),
            "semantic_invariance": true,
        },
        "sealed_blind_family_runtime": {
            "record_id": blind_record.record_id,
            "source_response_sha256": blind_record.approved_response.semantic_sha256,
            "inverse_claim_count": blind_output.semantic_inverse.claims.len(),
            "semantic_invariance": true,
        },
        "blind_verbosity_causal": {
            "outputs": blind_verbosity_outputs,
            "semantic_invariance": true,
            "monotonic_chars": true,
            "runtime_promotion": "HOLD",
        },
    });
    let report_path = root.join("canonical_response_codec_shadow_canary_ko_v1.json");
    write_json(&report_path, &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn surface_features(markdown: &str) -> serde_json::Value {
    let ending = if markdown.trim_end().ends_with("습니다.") {
        "FORMAL"
    } else if markdown.trim_end().ends_with("어요.") {
        "POLITE"
    } else if markdown.trim_end().ends_with("야.")
        || markdown.trim_end().ends_with("어.")
        || markdown.trim_end().ends_with("아.")
        || markdown.trim_end().ends_with("해.")
    {
        "INFORMAL"
    } else {
        "OTHER"
    };
    let discourse_markers = [
        "편하게 말씀드리면",
        "확인된 내용만",
        "핵심 내용을",
        "정정하면",
    ]
    .iter()
    .filter(|marker| markdown.contains(**marker))
    .count();
    let emotional_markers = ["아이구", "정말", "걱정", "다행", "괜찮"]
        .iter()
        .filter(|marker| markdown.contains(**marker))
        .count();
    let directness_markers = ["바로", "핵심은", "정리하면"]
        .iter()
        .filter(|marker| markdown.contains(**marker))
        .count();
    let hedging_markers = ["아마", "것 같", "가능"]
        .iter()
        .filter(|marker| markdown.contains(**marker))
        .count();
    serde_json::json!({
        "length": markdown.chars().count(),
        "clause_count": markdown
            .chars()
            .filter(|character| ".!?。！？".contains(*character))
            .count(),
        "ending": ending,
        "subject_front": markdown.starts_with("회의") || markdown.starts_with("정정"),
        "discourse_markers": discourse_markers,
        "emotional_markers": emotional_markers,
        "directness_markers": directness_markers,
        "hedging_markers": hedging_markers,
        "information_order": if markdown.find("회의").unwrap_or(usize::MAX)
            < markdown.find("오후").unwrap_or(usize::MAX)
        {
            "SUBJECT_BEFORE_TIME"
        } else {
            "OTHER"
        },
    })
}

fn time_revision() -> ApprovedCompositionalResponseIR {
    let mut response = ApprovedCompositionalResponseIR {
        schema: COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA.into(),
        speech_act: ApprovedSpeechActIR::Inform,
        operation: ApprovedOperationIR::Revise,
        claims: vec![ApprovedCompositionalClaimIR {
            proposition_id: "MEETING_TIME".into(),
            subject: ApprovedLexicalNodeIR {
                node_id: "MEETING".into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: "회의".into(),
            },
            relation: ApprovedRelationTypeIR::Time,
            value: ApprovedOpenValueIR::Clock {
                hour: 16,
                minute: 0,
            },
            polarity: true,
            modality: ApprovedModalityIR::Asserted,
    status_frame: None,
}],
        event_realizations: Vec::new(),
        discourse_relation: ApprovedDiscourseRelationIR::Correction,
        clause_plan: Vec::new(),
        style: ApprovedResponseStyleIR {
            register: LanguageRegisterIR::Neutral,
            verbosity: ApprovedVerbosityIR::Short,
        },
        source_world_state_sha256: "a".repeat(64),
        source_deliberation_sha256: "b".repeat(64),
        approval_replay_verified: true,
        unsupported_claims: 0,
        semantic_sha256: String::new(),
    };
    response.semantic_sha256 = compositional_response_sha256(&response);
    response
}

fn blind_three_clause_composition() -> ApprovedCompositionalResponseIR {
    let mut response = time_revision();
    response.operation = ApprovedOperationIR::Assert;
    response.discourse_relation = ApprovedDiscourseRelationIR::Statement;
    response.claims = ["회의", "자료", "참석 확인"]
        .into_iter()
        .enumerate()
        .map(|(index, label)| ApprovedCompositionalClaimIR {
            proposition_id: format!("BLIND_STATUS_{index}"),
            subject: ApprovedLexicalNodeIR {
                node_id: format!("BLIND_EVENT_{index}"),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: label.into(),
            },
            relation: ApprovedRelationTypeIR::Status,
            value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                node_id: "PLANNED".into(),
                semantic_type: ApprovedSemanticTypeIR::State,
                canonical_lexical_label: "계획됨".into(),
            }),
            polarity: true,
            modality: ApprovedModalityIR::Asserted,
    status_frame: None,
})
        .collect();
    response.semantic_sha256 = compositional_response_sha256(&response);
    response
}

fn write_json(path: &PathBuf, value: &impl Serialize) -> Result<(), Box<dyn Error>> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, path)?;
    Ok(())
}

#[allow(dead_code)]
fn file_sha256(path: &PathBuf) -> Result<String, Box<dyn Error>> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}
