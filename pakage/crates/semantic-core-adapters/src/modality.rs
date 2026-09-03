//! Typed modal, conditional, and counterfactual scope for the language cortex.
//!
//! Modal language is metadata about a proposition, never evidence that the
//! proposition is true and never execution authority by itself.  The analyzer
//! deliberately keeps open proposition text while compiling the closed class
//! operators that determine actuality, scope, and speech-act projection.

use std::cmp::Reverse;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const MODAL_SCOPE_GRAPH_SCHEMA: &str = "B_CORE_MODAL_SCOPE_GRAPH_IR_1";
const MAX_MODAL_OPERATORS: usize = 16;
const MAX_CONDITIONALS: usize = 8;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModalWorldIR {
    #[default]
    Actual,
    EpistemicPossible,
    EpistemicProbable,
    EpistemicCertain,
    Normative,
    Desired,
    Intended,
    Ability,
    Predicted,
    Hypothetical,
    Counterfactual,
    Questioned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModalOperatorKindIR {
    EpistemicPossibility,
    EpistemicProbability,
    EpistemicCertainty,
    DeonticObligation,
    DeonticPermission,
    DeonticProhibition,
    Desire,
    Intention,
    Ability,
    Prediction,
}

impl ModalOperatorKindIR {
    pub fn world(self) -> ModalWorldIR {
        match self {
            Self::EpistemicPossibility => ModalWorldIR::EpistemicPossible,
            Self::EpistemicProbability => ModalWorldIR::EpistemicProbable,
            Self::EpistemicCertainty => ModalWorldIR::EpistemicCertain,
            Self::DeonticObligation | Self::DeonticPermission | Self::DeonticProhibition => {
                ModalWorldIR::Normative
            }
            Self::Desire => ModalWorldIR::Desired,
            Self::Intention => ModalWorldIR::Intended,
            Self::Ability => ModalWorldIR::Ability,
            Self::Prediction => ModalWorldIR::Predicted,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModalNegationScopeIR {
    None,
    Proposition,
    Operator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConditionalKindIR {
    Indicative,
    Hypothetical,
    Counterfactual,
    Unless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModalIllocutionIR {
    Assertion,
    ModalStatement,
    PolarQuestion,
    WhQuestion,
    PoliteRequest,
    ConditionalDirective,
    Wish,
    CounterfactualReflection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModalPropositionIR {
    pub proposition_id: String,
    pub surface_text: String,
    pub normalized_text: String,
    pub world: ModalWorldIR,
    pub dialogue_truth_established: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModalOperatorIR {
    pub operator_id: String,
    pub kind: ModalOperatorKindIR,
    pub surface_form: String,
    pub source_start_byte: usize,
    pub source_end_byte: usize,
    pub strength_millis: u16,
    pub negation_scope: ModalNegationScopeIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_operator_id: Option<String>,
    pub scope_proposition_id: String,
    pub dialogue_truth_established: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalRelationIR {
    pub conditional_id: String,
    pub kind: ConditionalKindIR,
    pub antecedent: String,
    pub consequent: String,
    pub antecedent_negated: bool,
    pub antecedent_world: ModalWorldIR,
    pub consequent_world: ModalWorldIR,
    pub consequent_is_directive: bool,
    pub condition_satisfied: bool,
    pub reverse_inference_authorized: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModalScopeGraphIR {
    pub schema: String,
    pub propositions: Vec<ModalPropositionIR>,
    pub operators: Vec<ModalOperatorIR>,
    pub conditionals: Vec<ConditionalRelationIR>,
    pub root_world: ModalWorldIR,
    pub illocution: ModalIllocutionIR,
    pub unresolved_ambiguities: Vec<String>,
    pub dialogue_truth_established: bool,
    pub external_execution_authorized: bool,
    pub structural_coverage_millis: u16,
}

impl Default for ModalScopeGraphIR {
    fn default() -> Self {
        Self {
            schema: MODAL_SCOPE_GRAPH_SCHEMA.to_string(),
            propositions: Vec::new(),
            operators: Vec::new(),
            conditionals: Vec::new(),
            root_world: ModalWorldIR::Actual,
            illocution: ModalIllocutionIR::Assertion,
            unresolved_ambiguities: Vec::new(),
            dialogue_truth_established: false,
            external_execution_authorized: false,
            structural_coverage_millis: 0,
        }
    }
}

impl ModalScopeGraphIR {
    pub fn validate(&self) -> bool {
        if self.schema != MODAL_SCOPE_GRAPH_SCHEMA
            || self.operators.len() > MAX_MODAL_OPERATORS
            || self.conditionals.len() > MAX_CONDITIONALS
            || self.dialogue_truth_established
        {
            return false;
        }
        if self.external_execution_authorized
            != matches!(self.illocution, ModalIllocutionIR::PoliteRequest)
        {
            return false;
        }
        if self.external_execution_authorized
            && !self.propositions.first().is_some_and(|proposition| {
                is_polite_request(&proposition.normalized_text) && self.conditionals.is_empty()
            })
        {
            return false;
        }
        let proposition_ids = self
            .propositions
            .iter()
            .map(|proposition| proposition.proposition_id.as_str())
            .collect::<BTreeSet<_>>();
        let operator_ids = self
            .operators
            .iter()
            .map(|operator| operator.operator_id.as_str())
            .collect::<BTreeSet<_>>();
        if proposition_ids.len() != self.propositions.len()
            || operator_ids.len() != self.operators.len()
            || self.propositions.iter().any(|proposition| {
                proposition.proposition_id.trim().is_empty()
                    || proposition.normalized_text.trim().is_empty()
                    || proposition.dialogue_truth_established
                    || proposition.external_execution_authorized
            })
        {
            return false;
        }
        self.operators.iter().enumerate().all(|(index, operator)| {
            !operator.dialogue_truth_established
                && !operator.external_execution_authorized
                && proposition_ids.contains(operator.scope_proposition_id.as_str())
                && operator.scope_operator_id.as_ref().is_none_or(|scope| {
                    self.operators
                        .iter()
                        .enumerate()
                        .any(|(scope_index, candidate)| {
                            &candidate.operator_id == scope && scope_index > index
                        })
                })
        }) && self.conditionals.iter().all(|conditional| {
            !conditional.condition_satisfied
                && !conditional.reverse_inference_authorized
                && !conditional.external_execution_authorized
        })
    }

    pub fn blocks_goal_projection(&self) -> bool {
        if matches!(self.illocution, ModalIllocutionIR::PoliteRequest) {
            return false;
        }
        if !self.conditionals.is_empty() {
            return true;
        }
        if self.operators.first().is_some_and(|operator| {
            matches!(
                operator.kind,
                ModalOperatorKindIR::DeonticPermission | ModalOperatorKindIR::DeonticProhibition
            ) || (operator.kind == ModalOperatorKindIR::DeonticObligation
                && operator.negation_scope == ModalNegationScopeIR::Operator)
        }) {
            return true;
        }
        matches!(
            self.root_world,
            ModalWorldIR::EpistemicPossible
                | ModalWorldIR::EpistemicProbable
                | ModalWorldIR::EpistemicCertain
                | ModalWorldIR::Desired
                | ModalWorldIR::Intended
                | ModalWorldIR::Ability
                | ModalWorldIR::Predicted
                | ModalWorldIR::Hypothetical
                | ModalWorldIR::Counterfactual
        )
    }

    pub fn is_polite_request(&self) -> bool {
        self.illocution == ModalIllocutionIR::PoliteRequest
            && self.conditionals.is_empty()
            && self.external_execution_authorized
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ModalSemanticAnalyzer;

#[derive(Debug, Clone, Copy)]
struct ModalMarker {
    kind: ModalOperatorKindIR,
    form: &'static str,
    strength_millis: u16,
    negation_scope: ModalNegationScopeIR,
}

#[derive(Debug, Clone)]
struct MarkerMatch {
    marker: ModalMarker,
    start: usize,
    end: usize,
}

const MODAL_MARKERS: &[ModalMarker] = &[
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticObligation,
        form: "does not have to",
        strength_millis: 900,
        negation_scope: ModalNegationScopeIR::Operator,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticObligation,
        form: "doesn't have to",
        strength_millis: 900,
        negation_scope: ModalNegationScopeIR::Operator,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticObligation,
        form: "do not have to",
        strength_millis: 900,
        negation_scope: ModalNegationScopeIR::Operator,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticObligation,
        form: "don't have to",
        strength_millis: 900,
        negation_scope: ModalNegationScopeIR::Operator,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticObligation,
        form: "need not",
        strength_millis: 900,
        negation_scope: ModalNegationScopeIR::Operator,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticProhibition,
        form: "must not",
        strength_millis: 1000,
        negation_scope: ModalNegationScopeIR::Proposition,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticProhibition,
        form: "mustn't",
        strength_millis: 1000,
        negation_scope: ModalNegationScopeIR::Proposition,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticPermission,
        form: "allowed to",
        strength_millis: 900,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticPermission,
        form: "permitted to",
        strength_millis: 900,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicProbability,
        form: "probably",
        strength_millis: 760,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicProbability,
        form: "likely",
        strength_millis: 720,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicCertainty,
        form: "definitely",
        strength_millis: 960,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicCertainty,
        form: "certainly",
        strength_millis: 950,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicPossibility,
        form: "perhaps",
        strength_millis: 430,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicPossibility,
        form: "possibly",
        strength_millis: 450,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicPossibility,
        form: "maybe",
        strength_millis: 450,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicPossibility,
        form: "might",
        strength_millis: 420,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicPossibility,
        form: "may",
        strength_millis: 500,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticObligation,
        form: "have to",
        strength_millis: 930,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticObligation,
        form: "need to",
        strength_millis: 880,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticObligation,
        form: "must",
        strength_millis: 1000,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticObligation,
        form: "should",
        strength_millis: 760,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Desire,
        form: "would like to",
        strength_millis: 850,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Desire,
        form: "want to",
        strength_millis: 900,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Desire,
        form: "wish to",
        strength_millis: 850,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Intention,
        form: "intend to",
        strength_millis: 900,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Intention,
        form: "plan to",
        strength_millis: 850,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Intention,
        form: "going to",
        strength_millis: 800,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Ability,
        form: "cannot",
        strength_millis: 850,
        negation_scope: ModalNegationScopeIR::Proposition,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Ability,
        form: "able to",
        strength_millis: 900,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Ability,
        form: "could",
        strength_millis: 650,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Ability,
        form: "can",
        strength_millis: 850,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Prediction,
        form: "will",
        strength_millis: 780,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticObligation,
        form: "필요는 없다",
        strength_millis: 900,
        negation_scope: ModalNegationScopeIR::Operator,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticObligation,
        form: "필요하지 않",
        strength_millis: 900,
        negation_scope: ModalNegationScopeIR::Operator,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticProhibition,
        form: "해서는 안",
        strength_millis: 1000,
        negation_scope: ModalNegationScopeIR::Proposition,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticProhibition,
        form: "하면 안",
        strength_millis: 1000,
        negation_scope: ModalNegationScopeIR::Proposition,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticPermission,
        form: "해도 된다",
        strength_millis: 900,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticPermission,
        form: "해도 돼",
        strength_millis: 900,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicProbability,
        form: "가능성이 높",
        strength_millis: 760,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicProbability,
        form: "아마도",
        strength_millis: 700,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicCertainty,
        form: "틀림없이",
        strength_millis: 960,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicCertainty,
        form: "확실히",
        strength_millis: 940,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicPossibility,
        form: "가능성이 없",
        strength_millis: 500,
        negation_scope: ModalNegationScopeIR::Operator,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicPossibility,
        form: "수도 없",
        strength_millis: 420,
        negation_scope: ModalNegationScopeIR::Proposition,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicPossibility,
        form: "가능성이 있",
        strength_millis: 500,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicPossibility,
        form: "수도 있",
        strength_millis: 420,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicPossibility,
        form: "어쩌면",
        strength_millis: 400,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::EpistemicPossibility,
        form: "혹시",
        strength_millis: 380,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticObligation,
        form: "해야",
        strength_millis: 950,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::DeonticObligation,
        form: "할 필요가 있",
        strength_millis: 880,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Desire,
        form: "하고 싶",
        strength_millis: 880,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Desire,
        form: "고 싶",
        strength_millis: 880,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Desire,
        form: "원한다",
        strength_millis: 900,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Intention,
        form: "려고 한다",
        strength_millis: 900,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Intention,
        form: "계획이다",
        strength_millis: 850,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Intention,
        form: "예정이다",
        strength_millis: 850,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Ability,
        form: "해 줄 수 있",
        strength_millis: 850,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Ability,
        form: "해줄 수 있",
        strength_millis: 850,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Ability,
        form: "할 수 없",
        strength_millis: 850,
        negation_scope: ModalNegationScopeIR::Proposition,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Ability,
        form: "할 수 있",
        strength_millis: 850,
        negation_scope: ModalNegationScopeIR::None,
    },
    ModalMarker {
        kind: ModalOperatorKindIR::Prediction,
        form: "것이다",
        strength_millis: 780,
        negation_scope: ModalNegationScopeIR::None,
    },
];

impl ModalSemanticAnalyzer {
    pub fn analyze(&self, text: &str) -> ModalScopeGraphIR {
        let normalized = text.trim().to_lowercase();
        if normalized.is_empty() {
            return ModalScopeGraphIR::default();
        }
        let conditionals = detect_conditionals(&normalized);
        let mut matches = marker_matches(&normalized);
        contextualize_ambiguous_markers(&normalized, &mut matches);
        let english = normalized.is_ascii();
        let semantic_order = semantic_scope_order(&matches, english);
        let mut root_world = semantic_order
            .first()
            .map(|matched| matched.marker.kind.world())
            .unwrap_or(ModalWorldIR::Actual);
        let question = normalized.ends_with('?');
        let wh_question = question
            && [
                "why ",
                "what ",
                "how ",
                "when ",
                "where ",
                "who ",
                "왜 ",
                "무엇",
                "어떻게",
                "언제",
                "어디",
                "누가",
            ]
            .iter()
            .any(|marker| normalized.starts_with(marker) || normalized.contains(marker));
        let polite_request = conditionals.is_empty() && is_polite_request(&normalized);
        let consequent_directive = conditionals.iter().any(|item| item.consequent_is_directive);
        let counterfactual = conditionals
            .iter()
            .any(|item| item.kind == ConditionalKindIR::Counterfactual);
        if counterfactual {
            root_world = ModalWorldIR::Counterfactual;
        } else if !conditionals.is_empty() {
            root_world = ModalWorldIR::Hypothetical;
        } else if question && !polite_request {
            root_world = ModalWorldIR::Questioned;
        }
        let illocution = if polite_request {
            ModalIllocutionIR::PoliteRequest
        } else if counterfactual {
            ModalIllocutionIR::CounterfactualReflection
        } else if consequent_directive {
            ModalIllocutionIR::ConditionalDirective
        } else if wh_question {
            ModalIllocutionIR::WhQuestion
        } else if question {
            ModalIllocutionIR::PolarQuestion
        } else if semantic_order
            .first()
            .is_some_and(|matched| matched.marker.kind == ModalOperatorKindIR::Desire)
        {
            ModalIllocutionIR::Wish
        } else if semantic_order.is_empty() {
            ModalIllocutionIR::Assertion
        } else {
            ModalIllocutionIR::ModalStatement
        };
        let proposition = ModalPropositionIR {
            proposition_id: "MODAL-PROP-01".to_string(),
            surface_text: text.trim().to_string(),
            normalized_text: normalized.clone(),
            world: root_world,
            dialogue_truth_established: false,
            external_execution_authorized: false,
        };
        let operators = semantic_order
            .iter()
            .enumerate()
            .map(|(index, matched)| ModalOperatorIR {
                operator_id: format!("MODAL-OP-{:02}", index + 1),
                kind: matched.marker.kind,
                surface_form: matched.marker.form.to_string(),
                source_start_byte: matched.start,
                source_end_byte: matched.end,
                strength_millis: matched.marker.strength_millis,
                negation_scope: matched.marker.negation_scope,
                scope_operator_id: (index + 1 < semantic_order.len())
                    .then(|| format!("MODAL-OP-{:02}", index + 2)),
                scope_proposition_id: proposition.proposition_id.clone(),
                dialogue_truth_established: false,
                external_execution_authorized: false,
            })
            .collect::<Vec<_>>();
        let mut ambiguities = modal_ambiguities(&normalized, &matches);
        ambiguities.sort();
        ambiguities.dedup();
        let structural_coverage_millis = if conditionals.is_empty() && matches.is_empty() {
            0
        } else if ambiguities.is_empty() {
            1000
        } else {
            820
        };
        ModalScopeGraphIR {
            schema: MODAL_SCOPE_GRAPH_SCHEMA.to_string(),
            propositions: vec![proposition],
            operators,
            conditionals,
            root_world,
            illocution,
            unresolved_ambiguities: ambiguities,
            dialogue_truth_established: false,
            external_execution_authorized: polite_request,
            structural_coverage_millis,
        }
    }
}

fn marker_matches(text: &str) -> Vec<MarkerMatch> {
    let mut matches = Vec::new();
    for marker in MODAL_MARKERS {
        for (start, _) in text.match_indices(marker.form) {
            let end = start + marker.form.len();
            if marker.form.is_ascii() && !ascii_word_bounds(text, start, end) {
                continue;
            }
            matches.push(MarkerMatch {
                marker: *marker,
                start,
                end,
            });
        }
    }
    matches.sort_by(|left, right| {
        left.start
            .cmp(&right.start)
            .then_with(|| (right.end - right.start).cmp(&(left.end - left.start)))
    });
    let mut selected: Vec<MarkerMatch> = Vec::new();
    for candidate in matches {
        if selected
            .iter()
            .any(|prior| candidate.start < prior.end && candidate.end > prior.start)
        {
            continue;
        }
        selected.push(candidate);
    }
    selected.truncate(MAX_MODAL_OPERATORS);
    selected
}

fn contextualize_ambiguous_markers(text: &str, matches: &mut [MarkerMatch]) {
    for matched in matches {
        let before = text[..matched.start].trim_end();
        let after = text[matched.end..].trim_start();
        if matched.marker.negation_scope == ModalNegationScopeIR::None {
            if before.ends_with("not") {
                matched.marker.negation_scope = ModalNegationScopeIR::Operator;
            } else if after.starts_with("not ")
                || (matched.marker.form == "수도 있"
                    && ["못할", "않을"]
                        .iter()
                        .any(|ending| before.ends_with(ending)))
            {
                matched.marker.negation_scope = ModalNegationScopeIR::Proposition;
            }
        }
        match matched.marker.form {
            "may" if subject_is_second_person(text, matched.start) => {
                matched.marker.kind = ModalOperatorKindIR::DeonticPermission;
                matched.marker.strength_millis = 700;
            }
            "can" | "could" if is_polite_request(text) => {
                matched.marker.kind = ModalOperatorKindIR::Ability;
            }
            "will" if subject_is_first_person(text, matched.start) => {
                matched.marker.kind = ModalOperatorKindIR::Intention;
            }
            "should" if text.ends_with('?') => {
                matched.marker.kind = ModalOperatorKindIR::DeonticObligation;
            }
            _ => {}
        }
    }
}

fn semantic_scope_order(matches: &[MarkerMatch], english: bool) -> Vec<MarkerMatch> {
    let mut ordered = matches.to_vec();
    if english {
        ordered.sort_by_key(|matched| matched.start);
    } else {
        ordered.sort_by_key(|matched| Reverse(matched.start));
    }
    ordered
}

fn modal_ambiguities(text: &str, matches: &[MarkerMatch]) -> Vec<String> {
    let mut ambiguities = Vec::new();
    for matched in matches {
        match matched.marker.form {
            "may" if !subject_is_second_person(text, matched.start) => ambiguities.push(
                "MAY_EPISTEMIC_OR_DEONTIC: preserve possibility/permission competition".to_string(),
            ),
            "should"
                if !text.ends_with('?') && !text[matched.end..].trim_start().starts_with("be ") =>
            {
                ambiguities.push(
                    "SHOULD_DEONTIC_OR_EXPECTATION: preserve obligation/prediction competition"
                        .to_string(),
                )
            }
            "could" if !is_polite_request(text) => ambiguities.push(
                "COULD_ABILITY_OR_POSSIBILITY: preserve ability/possibility competition"
                    .to_string(),
            ),
            _ => {}
        }
    }
    ambiguities
}

fn detect_conditionals(text: &str) -> Vec<ConditionalRelationIR> {
    let mut result = Vec::new();
    if ["하면 안", "해서는 안", "면 안 돼", "면 안돼"]
        .iter()
        .any(|marker| text.contains(marker))
        || korean_desiderative_request(text)
    {
        return result;
    }
    if let Some((kind, antecedent, consequent, negated)) = split_conditional(text) {
        let consequent_directive = looks_directive(&consequent);
        result.push(ConditionalRelationIR {
            conditional_id: "CONDITIONAL-01".to_string(),
            kind,
            antecedent,
            consequent: consequent.clone(),
            antecedent_negated: negated,
            antecedent_world: if kind == ConditionalKindIR::Counterfactual {
                ModalWorldIR::Counterfactual
            } else {
                ModalWorldIR::Hypothetical
            },
            consequent_world: if kind == ConditionalKindIR::Counterfactual {
                ModalWorldIR::Counterfactual
            } else {
                ModalWorldIR::Hypothetical
            },
            consequent_is_directive: consequent_directive,
            condition_satisfied: false,
            reverse_inference_authorized: false,
            external_execution_authorized: false,
        });
    }
    result.truncate(MAX_CONDITIONALS);
    result
}

fn korean_desiderative_request(text: &str) -> bool {
    [
        "해줬으면 해",
        "해줬으면 좋",
        "해 주었으면 해",
        "해 주었으면 좋",
        "해 주셨으면 해",
        "해 주셨으면 좋",
    ]
    .iter()
    .any(|marker| text.contains(marker))
}

fn split_conditional(text: &str) -> Option<(ConditionalKindIR, String, String, bool)> {
    let counterfactual = [
        "if only ",
        "would have",
        "could have",
        "더라면",
        "했을 텐데",
        "했을텐데",
    ]
    .iter()
    .any(|marker| text.contains(marker));
    if let Some(rest) = text.strip_prefix("unless ") {
        let (antecedent, consequent) = split_arms(rest)?;
        return Some((ConditionalKindIR::Unless, antecedent, consequent, true));
    }
    if let Some(rest) = text.strip_prefix("if ") {
        let (antecedent, consequent) = split_arms(rest)?;
        return Some((
            if counterfactual {
                ConditionalKindIR::Counterfactual
            } else {
                ConditionalKindIR::Indicative
            },
            antecedent,
            consequent,
            false,
        ));
    }
    if let Some(position) = text.find(" if ") {
        let coordinated_prefix = text[..position].trim_end().ends_with(" and");
        let independent_prefix = text[..position]
            .trim()
            .trim_end_matches([',', ';'])
            .trim_end_matches("and")
            .trim();
        let rest = text[position + 1..].strip_prefix("if ")?;
        if looks_directive(independent_prefix) {
            if coordinated_prefix {
                let (antecedent, consequent) = split_arms(rest)?;
                return Some((
                    if counterfactual {
                        ConditionalKindIR::Counterfactual
                    } else {
                        ConditionalKindIR::Indicative
                    },
                    antecedent,
                    consequent,
                    false,
                ));
            }
            let antecedent_end = [
                "; otherwise",
                ". otherwise",
                ", otherwise",
                ", then",
                "; then",
            ]
            .iter()
            .filter_map(|delimiter| rest.find(delimiter))
            .min()
            .unwrap_or(rest.len());
            let antecedent = rest[..antecedent_end]
                .trim()
                .trim_end_matches([',', ';', '.', '!', '?'])
                .trim()
                .to_string();
            if antecedent.is_empty() {
                return None;
            }
            return Some((
                if counterfactual {
                    ConditionalKindIR::Counterfactual
                } else {
                    ConditionalKindIR::Indicative
                },
                antecedent,
                independent_prefix.to_string(),
                false,
            ));
        }
    }
    for marker in [" only after ", " after ", " once ", " only when ", " when "] {
        if let Some(position) = text.find(marker) {
            let consequent = text[..position].trim().to_string();
            let antecedent = text[position + marker.len()..]
                .trim()
                .trim_end_matches(['.', '!', '?'])
                .to_string();
            if !antecedent.is_empty() && !consequent.is_empty() && looks_directive(&consequent) {
                return Some((ConditionalKindIR::Indicative, antecedent, consequent, false));
            }
        }
    }
    for marker in ["하지 않으면 ", "않으면 "] {
        if let Some(position) = text.find(marker) {
            let start = coordinated_conditional_clause_start(text, position);
            let antecedent = text[start..position].trim().to_string();
            let consequent = text[position + marker.len()..].trim().to_string();
            let compound_antecedent = [" or ", " and ", " 또는 ", " 혹은 ", "거나 ", "고 "]
                .iter()
                .any(|connector| antecedent.contains(connector));
            if compound_antecedent {
                continue;
            }
            if !antecedent.is_empty() && !consequent.is_empty() {
                return Some((ConditionalKindIR::Unless, antecedent, consequent, true));
            }
        }
    }
    if let Some((token_start, antecedent_end)) = korean_conditional_token_span(text) {
        let antecedent_start = coordinated_conditional_clause_start(text, token_start);
        let antecedent = text[antecedent_start..antecedent_end].trim().to_string();
        let consequent = text[antecedent_end..].trim().to_string();
        if !antecedent.is_empty() && !consequent.is_empty() && looks_directive(&consequent) {
            return Some((
                if counterfactual {
                    ConditionalKindIR::Counterfactual
                } else {
                    ConditionalKindIR::Hypothetical
                },
                antecedent,
                consequent,
                false,
            ));
        }
    }
    for marker in [
        "었더라면 ",
        "았더라면 ",
        "했더라면 ",
        "더라면 ",
        "라면 ",
        "이면 ",
        "다면 ",
        "하면 ",
        "되면 ",
    ] {
        if let Some(position) = text.find(marker) {
            let start = coordinated_conditional_clause_start(text, position);
            let antecedent = format!("{}{}", text[start..position].trim(), marker.trim());
            let consequent = text[position + marker.len()..].trim().to_string();
            if !antecedent.is_empty() && !consequent.is_empty() {
                return Some((
                    if counterfactual {
                        ConditionalKindIR::Counterfactual
                    } else {
                        ConditionalKindIR::Hypothetical
                    },
                    antecedent,
                    consequent,
                    false,
                ));
            }
        }
    }
    for marker in [
        " 경우에만 ",
        " 때에만 ",
        " 뒤에만 ",
        " 후에만 ",
        " 뒤에 ",
        " 후에 ",
    ] {
        if let Some(position) = text.find(marker) {
            let antecedent = text[..position].trim().to_string();
            let consequent = text[position + marker.len()..].trim().to_string();
            if !antecedent.is_empty() && !consequent.is_empty() && looks_directive(&consequent) {
                return Some((
                    ConditionalKindIR::Hypothetical,
                    antecedent,
                    consequent,
                    false,
                ));
            }
        }
    }
    None
}

fn coordinated_conditional_clause_start(text: &str, anchor_start: usize) -> usize {
    let prefix = &text[..anchor_start];
    let fixed_boundary = ["고 나서 ", "; ", ". "]
        .iter()
        .filter_map(|marker| prefix.rfind(marker).map(|position| position + marker.len()))
        .max()
        .unwrap_or(0);
    let action_coordination = prefix
        .rmatch_indices("하고 ")
        .find_map(|(position, marker)| {
            let preceding = prefix[fixed_boundary..position].trim();
            [
                "검사", "확인", "분석", "조사", "읽", "열", "저장", "변환", "수리", "복구", "고치",
                "실행", "배포", "삭제", "제거", "기록",
            ]
            .iter()
            .any(|action| preceding.ends_with(action))
            .then_some(position + marker.len())
        });
    action_coordination.unwrap_or(fixed_boundary)
}

fn korean_conditional_token_span(text: &str) -> Option<(usize, usize)> {
    let mut token_start = None;
    for (index, character) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        if character.is_whitespace() {
            if let Some(start) = token_start.take() {
                let raw_token = &text[start..index];
                let token =
                    raw_token.trim_matches(|candidate: char| candidate.is_ascii_punctuation());
                if token.ends_with('면')
                    && !["반면", "측면", "장면", "화면", "표면"].contains(&token)
                {
                    return Some((start, index));
                }
            }
        } else if token_start.is_none() {
            token_start = Some(index);
        }
    }
    None
}

fn split_arms(text: &str) -> Option<(String, String)> {
    for separator in [", then ", ", ", " then "] {
        if let Some(position) = text.find(separator) {
            let antecedent = text[..position].trim().to_string();
            let consequent = text[position + separator.len()..].trim().to_string();
            if !antecedent.is_empty() && !consequent.is_empty() {
                return Some((antecedent, consequent));
            }
        }
    }
    None
}

fn is_polite_request(text: &str) -> bool {
    let english = ["can you ", "could you ", "would you ", "will you "]
        .iter()
        .any(|prefix| text.starts_with(prefix))
        && (text.ends_with('?') || text.contains(" please"));
    let korean = [
        "해 줄래",
        "해줄래",
        "해 주시겠",
        "해주시겠",
        "해 줄 수 있",
        "해줄 수 있",
        "줄래?",
    ]
    .iter()
    .any(|marker| text.contains(marker));
    english || korean
}

fn looks_directive(text: &str) -> bool {
    let trimmed = text.trim().trim_end_matches(['.', '!', '?']);
    let english_first = trimmed
        .strip_prefix("only ")
        .unwrap_or(trimmed)
        .split_whitespace()
        .next()
        .unwrap_or_default();
    [
        "run",
        "delete",
        "remove",
        "save",
        "deploy",
        "publish",
        "open",
        "read",
        "write",
        "transform",
        "convert",
        "check",
        "inspect",
        "analyze",
        "fix",
        "repair",
        "restore",
        "verify",
        "report",
        "stop",
        "continue",
        "keep",
    ]
    .contains(&english_first)
        || [
            "해",
            "해줘",
            "하세요",
            "하라",
            "삭제해",
            "지워",
            "제거해",
            "고쳐",
            "고쳐줘",
            "저장해",
            "실행해",
            "배포해",
            "멈춰",
            "계속해",
        ]
        .iter()
        .any(|ending| trimmed.ends_with(ending))
}

fn subject_is_second_person(text: &str, marker_start: usize) -> bool {
    let prefix = text[..marker_start].trim_end();
    prefix.ends_with("you") || prefix.ends_with("너") || prefix.ends_with("당신")
}

fn subject_is_first_person(text: &str, marker_start: usize) -> bool {
    let prefix = text[..marker_start].trim_end();
    prefix.ends_with('i')
        || prefix.ends_with("we")
        || prefix.ends_with("나")
        || prefix.ends_with("저")
        || prefix.ends_with("우리")
}

fn ascii_word_bounds(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    !before.is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
        && !after.is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_possibility_scopes_over_obligation() {
        let graph = ModalSemanticAnalyzer.analyze("We might need to delete the cache.");
        assert_eq!(graph.root_world, ModalWorldIR::EpistemicPossible);
        assert_eq!(graph.operators.len(), 2);
        assert_eq!(
            graph.operators[0].kind,
            ModalOperatorKindIR::EpistemicPossibility
        );
        assert_eq!(
            graph.operators[1].kind,
            ModalOperatorKindIR::DeonticObligation
        );
        assert_eq!(
            graph.operators[0].scope_operator_id.as_deref(),
            Some("MODAL-OP-02")
        );
        assert!(graph.blocks_goal_projection());
        assert!(graph.validate());
    }

    #[test]
    fn embedded_english_condition_excludes_independent_directive_prefix() {
        let graph = ModalSemanticAnalyzer
            .analyze("Inspect the cache and if the cache is stale, repair the cache.");
        assert_eq!(graph.conditionals.len(), 1);
        assert_eq!(graph.conditionals[0].antecedent, "the cache is stale");
        assert_eq!(graph.conditionals[0].consequent, "repair the cache.");
        assert!(graph.conditionals[0].consequent_is_directive);
        assert!(!graph.conditionals[0].external_execution_authorized);
    }

    #[test]
    fn postposed_condition_keeps_the_matrix_continuation_as_its_consequent() {
        let graph = ModalSemanticAnalyzer.analyze(
            "Only keep integrating Aurora if it expands real coverage; otherwise ask whether to stop.",
        );
        assert_eq!(graph.conditionals.len(), 1);
        assert_eq!(graph.conditionals[0].antecedent, "it expands real coverage");
        assert_eq!(
            graph.conditionals[0].consequent,
            "only keep integrating aurora"
        );
        assert!(graph.conditionals[0].consequent_is_directive);
        assert!(!graph.conditionals[0].external_execution_authorized);
    }

    #[test]
    fn korean_condition_excludes_independent_directive_prefix() {
        let graph = ModalSemanticAnalyzer.analyze("캐시를 검사하고 캐시에 문제가 있으면 수리해");
        assert_eq!(graph.conditionals.len(), 1);
        assert_eq!(graph.conditionals[0].antecedent, "캐시에 문제가 있으면");
        assert_eq!(graph.conditionals[0].consequent, "수리해");
        assert!(graph.conditionals[0].consequent_is_directive);
    }

    #[test]
    fn korean_fix_imperative_is_a_conditional_consequent() {
        let graph = ModalSemanticAnalyzer.analyze("로그를 분석하고 오류가 있으면 고쳐");
        assert_eq!(graph.conditionals.len(), 1);
        assert_eq!(graph.conditionals[0].antecedent, "오류가 있으면");
        assert_eq!(graph.conditionals[0].consequent, "고쳐");
        assert!(graph.conditionals[0].consequent_is_directive);
    }

    #[test]
    fn korean_prohibition_and_desiderative_request_are_not_conditionals() {
        let prohibition =
            ModalSemanticAnalyzer.analyze("캐시를 지우면 안 돼. 대신 상태를 검증해줘.");
        let desiderative =
            ModalSemanticAnalyzer.analyze("팀은 워커를 수리하자고 했지만 큐를 확인해줬으면 해");
        assert!(prohibition.conditionals.is_empty());
        assert!(desiderative.conditionals.is_empty());
        assert_eq!(prohibition.root_world, ModalWorldIR::Actual);
        assert_eq!(desiderative.root_world, ModalWorldIR::Actual);
    }

    #[test]
    fn proposition_and_operator_negation_are_distinct() {
        let prohibited = ModalSemanticAnalyzer.analyze("You must not delete the cache.");
        let optional = ModalSemanticAnalyzer.analyze("You do not have to delete the cache.");
        assert_eq!(
            prohibited.operators[0].negation_scope,
            ModalNegationScopeIR::Proposition
        );
        assert_eq!(
            optional.operators[0].negation_scope,
            ModalNegationScopeIR::Operator
        );
        assert!(prohibited.blocks_goal_projection());
        assert!(optional.blocks_goal_projection());
    }

    #[test]
    fn korean_suffix_order_preserves_outer_possibility() {
        let graph = ModalSemanticAnalyzer.analyze("캐시를 삭제해야 할 수도 있다.");
        assert_eq!(graph.root_world, ModalWorldIR::EpistemicPossible);
        assert_eq!(graph.operators.len(), 2);
        assert_eq!(
            graph.operators[0].kind,
            ModalOperatorKindIR::EpistemicPossibility
        );
        assert_eq!(
            graph.operators[1].kind,
            ModalOperatorKindIR::DeonticObligation
        );
    }

    #[test]
    fn conditional_directive_has_no_current_authority() {
        let graph = ModalSemanticAnalyzer.analyze("If the tests pass, deploy the service.");
        assert_eq!(graph.illocution, ModalIllocutionIR::ConditionalDirective);
        assert!(graph.blocks_goal_projection());
        assert!(!graph.external_execution_authorized);
        assert!(!graph.conditionals[0].condition_satisfied);
        assert!(!graph.conditionals[0].reverse_inference_authorized);
        assert!(graph.validate());
    }

    #[test]
    fn polite_question_projects_request_without_modal_truth() {
        let graph = ModalSemanticAnalyzer.analyze("Could you delete the cache?");
        assert_eq!(graph.illocution, ModalIllocutionIR::PoliteRequest);
        assert!(graph.external_execution_authorized);
        assert!(!graph.dialogue_truth_established);
        assert!(!graph.blocks_goal_projection());
        assert!(graph.validate());
    }

    #[test]
    fn counterfactual_arms_remain_nonactual() {
        let graph = ModalSemanticAnalyzer
            .analyze("If the backup had existed, the restore would have succeeded.");
        assert_eq!(graph.root_world, ModalWorldIR::Counterfactual);
        assert_eq!(
            graph.conditionals[0].kind,
            ConditionalKindIR::Counterfactual
        );
        assert!(!graph.conditionals[0].reverse_inference_authorized);
    }

    #[test]
    fn ambiguous_may_is_preserved() {
        let graph = ModalSemanticAnalyzer.analyze("The service may restart.");
        assert!(graph
            .unresolved_ambiguities
            .iter()
            .any(|item| item.starts_with("MAY_EPISTEMIC_OR_DEONTIC")));
    }

    #[test]
    fn authority_and_scope_cycle_tampering_fail_validation() {
        let mut assertion = ModalSemanticAnalyzer.analyze("The worker can retry.");
        assertion.illocution = ModalIllocutionIR::PoliteRequest;
        assertion.external_execution_authorized = true;
        assert!(!assertion.validate());

        let mut nested = ModalSemanticAnalyzer.analyze("We might need to retry.");
        nested.operators[1].scope_operator_id = Some("MODAL-OP-01".to_string());
        assert!(!nested.validate());
    }
}
