use crate::model::{
    ConceptSchema, DesireKind, EmotionKind, GoalKind, NeuronParams, NodeModulation, RelationType,
    SynapseCore,
};

pub fn build_demo_core() -> SynapseCore {
    let mut core = SynapseCore::new();

    let cat = core.add_node(
        "고양이",
        "concept",
        "고양이",
        NeuronParams {
            importance: 0.95,
            modulation: NodeModulation::default()
                .with_emotion(EmotionKind::Joy, 0.6)
                .with_desire(DesireKind::Relationship, 0.4),
            ..NeuronParams::default()
        },
    );
    let pet = core.add_node(
        "애완동물",
        "concept",
        "애완동물",
        NeuronParams {
            importance: 0.8,
            modulation: NodeModulation::default()
                .with_emotion(EmotionKind::Joy, 0.5)
                .with_desire(DesireKind::Relationship, 0.5),
            ..NeuronParams::default()
        },
    );
    let fur = core.add_node(
        "털",
        "attribute",
        "털",
        NeuronParams {
            importance: 0.65,
            ..NeuronParams::default()
        },
    );
    let dog = core.add_node(
        "강아지",
        "concept",
        "강아지",
        NeuronParams {
            importance: 0.75,
            modulation: NodeModulation::default()
                .with_emotion(EmotionKind::Joy, 0.4)
                .with_desire(DesireKind::Relationship, 0.5),
            ..NeuronParams::default()
        },
    );
    let samsung_down = core.add_node(
        "삼성전자 하락",
        "event",
        "삼성전자 하락",
        NeuronParams {
            importance: 0.9,
            ..NeuronParams::default()
        },
    );
    let earnings_problem = core.add_node(
        "실적 문제",
        "hypothesis",
        "실적 문제",
        NeuronParams {
            importance: 0.75,
            modulation: NodeModulation::default()
                .with_emotion(EmotionKind::Anxiety, 0.4)
                .with_goal(GoalKind::PreserveCoherence, 0.8),
            ..NeuronParams::default()
        },
    );
    let supply_problem = core.add_node(
        "수급 문제",
        "hypothesis",
        "수급 문제",
        NeuronParams {
            importance: 0.85,
            modulation: NodeModulation::default()
                .with_emotion(EmotionKind::Anxiety, 0.8)
                .with_desire(DesireKind::Safety, 0.8)
                .with_goal(GoalKind::HelpUser, 0.5),
            ..NeuronParams::default()
        },
    );
    let rate_problem = core.add_node(
        "금리 문제",
        "hypothesis",
        "금리 문제",
        NeuronParams {
            importance: 0.7,
            modulation: NodeModulation::default()
                .with_emotion(EmotionKind::Anxiety, 0.5)
                .with_desire(DesireKind::Safety, 0.6),
            ..NeuronParams::default()
        },
    );
    let earnings_good = core.add_node(
        "실적 양호",
        "evidence",
        "실적 양호",
        NeuronParams {
            importance: 0.9,
            modulation: NodeModulation::default()
                .with_emotion(EmotionKind::Trust, 0.8)
                .with_goal(GoalKind::PreserveCoherence, 0.9),
            ..NeuronParams::default()
        },
    );
    let foreign_sell = core.add_node(
        "외국인 매도",
        "evidence",
        "외국인 매도",
        NeuronParams {
            importance: 0.82,
            modulation: NodeModulation::default()
                .with_emotion(EmotionKind::Anxiety, 0.9)
                .with_desire(DesireKind::Safety, 0.8),
            ..NeuronParams::default()
        },
    );
    let institution_buy = core.add_node(
        "기관 매수",
        "evidence",
        "기관 매수",
        NeuronParams {
            importance: 0.68,
            modulation: NodeModulation::default()
                .with_emotion(EmotionKind::Trust, 0.7)
                .with_goal(GoalKind::HelpUser, 0.4),
            ..NeuronParams::default()
        },
    );
    let margin_call = core.add_node(
        "반대매매",
        "evidence",
        "반대매매",
        NeuronParams {
            importance: 0.72,
            modulation: NodeModulation::default()
                .with_emotion(EmotionKind::Anxiety, 0.9)
                .with_desire(DesireKind::Safety, 0.9),
            ..NeuronParams::default()
        },
    );

    core.connect(cat, pet, 0.85, RelationType::Association);
    core.connect(cat, fur, 0.72, RelationType::Association);
    core.connect(cat, dog, 0.55, RelationType::Association);
    core.connect(pet, cat, 0.85, RelationType::Association);
    core.connect(fur, cat, 0.72, RelationType::Association);
    core.connect(dog, cat, 0.55, RelationType::Association);
    core.connect(dog, pet, 0.8, RelationType::Association);
    core.connect(samsung_down, earnings_problem, 0.72, RelationType::Cause);
    core.connect(samsung_down, supply_problem, 0.86, RelationType::Cause);
    core.connect(samsung_down, rate_problem, 0.62, RelationType::Cause);
    core.connect(
        earnings_problem,
        supply_problem,
        0.30,
        RelationType::Association,
    );
    core.connect(supply_problem, foreign_sell, 0.86, RelationType::Support);
    core.connect(supply_problem, institution_buy, 0.48, RelationType::Support);
    core.connect(supply_problem, margin_call, 0.78, RelationType::Support);
    core.connect(
        earnings_good,
        earnings_problem,
        0.92,
        RelationType::Contradiction,
    );
    core.connect(foreign_sell, supply_problem, 0.86, RelationType::Support);
    core.connect(institution_buy, supply_problem, 0.48, RelationType::Support);
    core.connect(margin_call, supply_problem, 0.78, RelationType::Support);

    core.add_concept_schema(
        ConceptSchema::new(
            "market.supply_driven_decline",
            "Supply-driven decline",
            "A price decline caused primarily by trading pressure, forced selling, liquidity stress, or investor flow rather than business fundamentals",
            "market",
        )
        .with_abstraction_level(0.86)
        .with_importance(0.9)
        .with_reflex_bonus(0.25)
        .with_cue("foreign selling", "selling_actor", 0.9)
        .with_cue("외국인 매도", "selling_actor", 0.9)
        .with_cue("margin call", "forced_selling", 0.95)
        .with_cue("반대매매", "forced_selling", 0.95)
        .with_cue("supply pressure", "liquidity_pressure", 0.85)
        .with_cue("수급 악화", "liquidity_pressure", 0.85)
        .with_cue("decline", "price_result", 0.65)
        .with_cue("하락", "price_result", 0.65),
    );
    core.add_concept_schema(
        ConceptSchema::new(
            "market.earnings_deterioration",
            "Earnings deterioration",
            "A price decline caused by weaker profit, worse guidance, margin pressure, or damaged business fundamentals",
            "market",
        )
        .with_abstraction_level(0.78)
        .with_importance(0.75)
        .with_cue("earnings problem", "fundamental_cause", 0.9)
        .with_cue("실적 악화", "fundamental_cause", 0.9)
        .with_cue("profit decline", "fundamental_cause", 0.8)
        .with_cue("weak guidance", "fundamental_signal", 0.8),
    );

    core
}

#[cfg(test)]
mod tests {
    use super::build_demo_core;
    use crate::model::{
        DesireKind, EmotionKind, NeuronParams, NodeModulation, RelationType, SynapseCore,
    };

    #[test]
    fn activation_field_keeps_unrelated_nodes_inactive() {
        let mut core = build_demo_core();
        let active = core.activate("고양이", 8);
        let labels = active
            .iter()
            .map(|id| core.node(*id).label.as_str())
            .collect::<Vec<_>>();

        assert!(labels.contains(&"고양이"));
        assert!(!labels.contains(&"삼성전자 하락"));
    }

    #[test]
    fn propagation_spreads_activation() {
        let mut core = SynapseCore::new();
        let cat = core.add_node(
            "고양이",
            "concept",
            "고양이",
            NeuronParams {
                importance: 1.0,
                ..NeuronParams::default()
            },
        );
        let dog = core.add_node(
            "강아지",
            "concept",
            "강아지",
            NeuronParams {
                importance: 1.0,
                ..NeuronParams::default()
            },
        );
        core.connect(cat, dog, 0.8, RelationType::Association);

        core.stimulate(cat, 0.9);
        core.propagate_once();

        assert!(core.activation(dog) > 0.5);
    }

    #[test]
    fn inhibition_suppresses_contradicted_hypothesis() {
        let mut core = build_demo_core();
        let good = 8;
        let problem = 5;

        core.stimulate(good, 0.9);
        core.stimulate(problem, 0.8);
        core.inhibit();

        assert_eq!(core.activation(problem), 0.0);
    }

    #[test]
    fn resonance_creates_thought_crystal() {
        let mut core = build_demo_core();
        let result = core.resonate("삼성전자 하락 외국인 매도 반대매매");

        assert!(result.thought_crystal.is_some());
        assert!(!result.active_node_ids.is_empty());
    }

    #[test]
    fn repeated_crystals_promote_reflex_circuit() {
        let mut core = build_demo_core();

        for _ in 0..5 {
            let result = core.resonate("삼성전자 하락 외국인 매도 반대매매");
            assert!(result.thought_crystal.is_some());
        }

        assert_eq!(core.reflexes().len(), 1);

        let reflex_result = core.resonate("삼성전자 하락 외국인 매도 반대매매");
        assert!(reflex_result.reflex_hit);
        assert_eq!(reflex_result.cycles, 0);
    }

    #[test]
    fn sleep_promotes_reflex_crystal_to_learned_concept_schema() {
        let mut core = build_demo_core();

        for _ in 0..4 {
            let _ = core.resonate("삼성전자 하락 외국인 매도 반대매매");
        }

        let before = core.concept_schemas().len();
        let report = core.sleep_cycle();
        let after = core.concept_schemas().len();

        assert_eq!(report.promoted_concepts, 1);
        assert_eq!(after, before + 1);

        let recalls = core.recall_concepts("삼성전자 하락 외국인 매도 수급 문제", 8);
        assert!(recalls
            .iter()
            .any(|recall| recall.schema_id.starts_with("learned.crystal.")));
    }

    #[test]
    fn sleep_cycle_consolidates_active_memory() {
        let mut core = build_demo_core();
        let result = core.resonate("고양이 털 애완동물");
        assert!(result.achieved);

        let cat = 0;
        let before = core.importance(cat);
        let report = core.sleep_cycle();

        assert!(report.strengthened_nodes > 0);
        assert!(core.importance(cat) >= before);
    }

    #[test]
    fn cognitive_state_modulates_activation_priority() {
        let mut core = SynapseCore::new();
        let risk = core.add_node(
            "market risk",
            "hypothesis",
            "market risk",
            NeuronParams {
                importance: 0.7,
                modulation: NodeModulation::default()
                    .with_emotion(EmotionKind::Anxiety, 1.0)
                    .with_desire(DesireKind::Safety, 1.0),
                ..NeuronParams::default()
            },
        );
        let research = core.add_node(
            "market research",
            "hypothesis",
            "market research",
            NeuronParams {
                importance: 0.7,
                modulation: NodeModulation::default().with_desire(DesireKind::Learning, 1.0),
                ..NeuronParams::default()
            },
        );

        core.set_emotion(EmotionKind::Anxiety, 1.0);
        core.set_desire(DesireKind::Safety, 1.0);
        core.activate("market", 2);

        assert!(core.activation(risk) > core.activation(research));
    }

    #[test]
    fn concept_first_recall_applies_definition_to_context() {
        let core = build_demo_core();
        let recalls = core.recall_concepts("foreign selling margin call decline", 2);

        assert!(!recalls.is_empty());
        assert_eq!(recalls[0].schema_id, "market.supply_driven_decline");
        assert!(recalls[0].context_fit > 0.5);
        assert!(recalls[0].interpretation.contains("Current context"));
    }

    #[test]
    fn korean_market_cues_recall_supply_driven_decline() {
        let core = build_demo_core();
        let recalls = core.recall_concepts("삼성전자 하락 외국인 매도 반대매매", 2);

        assert!(!recalls.is_empty());
        assert_eq!(recalls[0].schema_id, "market.supply_driven_decline");
        assert!(recalls[0].context_fit > 0.5);
    }

    #[test]
    fn untrained_context_generalizes_existing_concepts_into_thought_crystal() {
        let mut core = build_demo_core();
        let result = core
            .resonate("Samsung decline foreign selling margin call earnings problem weak guidance");
        let generalization = result
            .generalization
            .expect("generalization should combine existing concepts");

        assert!(generalization
            .source_schema_ids
            .contains(&"market.supply_driven_decline".to_string()));
        assert!(generalization
            .source_schema_ids
            .contains(&"market.earnings_deterioration".to_string()));
        assert!(generalization
            .interpretation
            .contains("Untrained context generalized"));
        assert!(generalization
            .thought_crystal
            .id
            .starts_with("generalized:"));
        assert!(generalization.confidence > 0.25);
    }

    #[test]
    fn generalized_thought_chain_reuses_crystal_then_promotes_reflex_and_concept() {
        let mut core = build_demo_core();
        let first = core
            .resonate("Samsung decline foreign selling margin call earnings problem weak guidance");
        assert!(first.generalization.is_some());
        assert!(!first.reflex_hit);
        assert!(first.cycles > 0);

        let second = core
            .resonate("Samsung decline margin call foreign selling weak guidance earnings problem");
        assert!(second.generalization.is_none());
        assert!(second.thought_crystal.is_some());
        assert!(!second.reflex_hit);
        assert!(second.cycles < first.cycles);

        let third = core
            .resonate("Samsung decline foreign selling margin call earnings problem weak guidance");
        assert!(third.reflex_hit);
        assert_eq!(third.cycles, 0);

        let before = core.concept_schemas().len();
        let report = core.sleep_cycle();
        let after = core.concept_schemas().len();

        assert!(report.promoted_concepts >= 1);
        assert!(after > before);
        assert!(core
            .concept_schemas()
            .iter()
            .any(|schema| schema.id.starts_with("learned.generalized.")));
    }

    #[test]
    fn feedback_correction_suppresses_wrong_crystal_and_uses_alternative_concept() {
        let mut core = build_demo_core();
        let stimulus = "Samsung decline foreign selling margin call earnings problem weak guidance";
        let first = core.resonate(stimulus);
        let wrong_crystal = first
            .generalization
            .as_ref()
            .expect("wrong generalized crystal exists")
            .thought_crystal
            .clone();
        let before = core.recall_concepts(stimulus, 3);
        assert_eq!(before[0].schema_id, "market.earnings_deterioration");

        let correction =
            core.apply_feedback(stimulus, &wrong_crystal.id, "market.supply_driven_decline");
        assert!(correction.corrected);
        assert!(correction.wrong_confidence_after < wrong_crystal.confidence);
        assert!(correction.contradiction_edges >= 1);

        let after = core.recall_concepts(
            "weak guidance margin call foreign selling Samsung decline earnings problem",
            3,
        );
        assert_eq!(after[0].schema_id, "market.supply_driven_decline");

        let second = core
            .resonate("weak guidance margin call foreign selling Samsung decline earnings problem");
        assert!(second
            .concept_recalls
            .first()
            .is_some_and(|recall| recall.schema_id == "market.supply_driven_decline"));
    }
}
