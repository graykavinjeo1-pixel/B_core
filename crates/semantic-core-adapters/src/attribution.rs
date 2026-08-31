//! Typed attribution, belief, and evidential provenance for language input.
//!
//! An attributed proposition is not a system fact and is never an execution
//! request merely because its content contains an imperative-looking verb.
//! This graph is discourse-local adapter IR; it does not promote actors or
//! proposition surfaces into canonical semantic concepts.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::compositional_semantics::PredicateFrameIR;

pub const ATTRIBUTION_GRAPH_SCHEMA: &str = "B_CORE_ATTRIBUTION_GRAPH_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscourseActorKindIR {
    DialogueSpeaker,
    NamedEntity,
    DocumentSource,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseActorIR {
    pub actor_id: String,
    pub kind: DiscourseActorKindIR,
    pub surface: String,
    pub normalized_label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AttributionAttitudeIR {
    Say,
    Report,
    Claim,
    Believe,
    Think,
    Know,
    Doubt,
    Deny,
    Hear,
    Observe,
    Infer,
    Want,
    Expect,
    Correct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AttributionStanceIR {
    Endorses,
    Rejects,
    Withholds,
    Desires,
    Predicts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EpistemicStatusIR {
    Reported,
    Claimed,
    Believed,
    PresentedAsKnown,
    Doubted,
    Denied,
    Hearsay,
    Observed,
    Inferred,
    Desired,
    Expected,
    Corrected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AttributionEvidenceKindIR {
    Unspecified,
    Speech,
    Hearsay,
    DirectObservation,
    Inference,
    Document,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AttributedPropositionPolarityIR {
    Positive,
    Negative,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributedPropositionIR {
    pub proposition_id: String,
    pub surface_text: String,
    pub normalized_text: String,
    pub polarity: AttributedPropositionPolarityIR,
    pub source_start_byte: usize,
    pub source_end_byte: usize,
    pub quoted: bool,
    pub dialogue_truth_established: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributionEdgeIR {
    pub attribution_id: String,
    pub actor_id: String,
    pub proposition_id: String,
    pub attitude: AttributionAttitudeIR,
    pub stance: AttributionStanceIR,
    pub epistemic_status: EpistemicStatusIR,
    pub evidence_kind: AttributionEvidenceKindIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_source_actor_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_proposition_id: Option<String>,
    pub evidence_surface: String,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributionGraphIR {
    pub schema: String,
    pub actors: Vec<DiscourseActorIR>,
    pub propositions: Vec<AttributedPropositionIR>,
    pub attributions: Vec<AttributionEdgeIR>,
    pub attributed_frame_ids: Vec<String>,
    pub unresolved_attributions: Vec<String>,
    pub structural_coverage_millis: u16,
}

impl Default for AttributionGraphIR {
    fn default() -> Self {
        Self {
            schema: ATTRIBUTION_GRAPH_SCHEMA.to_string(),
            actors: Vec::new(),
            propositions: Vec::new(),
            attributions: Vec::new(),
            attributed_frame_ids: Vec::new(),
            unresolved_attributions: Vec::new(),
            structural_coverage_millis: 0,
        }
    }
}

impl AttributionGraphIR {
    pub fn attributes_frame(&self, frame_id: &str) -> bool {
        self.attributed_frame_ids
            .iter()
            .any(|candidate| candidate == frame_id)
    }

    pub fn actor(&self, actor_id: &str) -> Option<&DiscourseActorIR> {
        self.actors.iter().find(|actor| actor.actor_id == actor_id)
    }

    pub fn proposition(&self, proposition_id: &str) -> Option<&AttributedPropositionIR> {
        self.propositions
            .iter()
            .find(|proposition| proposition.proposition_id == proposition_id)
    }

    pub fn root_attributions(&self) -> impl Iterator<Item = &AttributionEdgeIR> {
        self.attributions
            .iter()
            .filter(|edge| edge.parent_proposition_id.is_none())
    }

    pub fn validate(&self) -> bool {
        if self.schema != ATTRIBUTION_GRAPH_SCHEMA || self.structural_coverage_millis > 1_000 {
            return false;
        }
        let actor_ids = self
            .actors
            .iter()
            .map(|actor| actor.actor_id.as_str())
            .collect::<BTreeSet<_>>();
        let proposition_ids = self
            .propositions
            .iter()
            .map(|proposition| proposition.proposition_id.as_str())
            .collect::<BTreeSet<_>>();
        actor_ids.len() == self.actors.len()
            && proposition_ids.len() == self.propositions.len()
            && self.actors.iter().all(|actor| {
                !actor.actor_id.trim().is_empty()
                    && !actor.surface.trim().is_empty()
                    && !actor.normalized_label.trim().is_empty()
            })
            && self.propositions.iter().all(|proposition| {
                !proposition.proposition_id.trim().is_empty()
                    && !proposition.surface_text.trim().is_empty()
                    && proposition.source_start_byte < proposition.source_end_byte
                    && !proposition.dialogue_truth_established
                    && !proposition.external_execution_authorized
            })
            && self.attributions.iter().all(|edge| {
                actor_ids.contains(edge.actor_id.as_str())
                    && proposition_ids.contains(edge.proposition_id.as_str())
                    && edge
                        .evidence_source_actor_id
                        .as_deref()
                        .is_none_or(|id| actor_ids.contains(id))
                    && edge
                        .parent_proposition_id
                        .as_deref()
                        .is_none_or(|id| proposition_ids.contains(id))
                    && edge.confidence_millis <= 1_000
            })
            && self
                .attributed_frame_ids
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                == self.attributed_frame_ids.len()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AttributionAnalyzer;

#[derive(Debug, Clone, Copy)]
struct AttributionLexeme {
    attitude: AttributionAttitudeIR,
    forms: &'static [&'static str],
}

const ATTRIBUTION_LEXEMES: &[AttributionLexeme] = &[
    AttributionLexeme {
        attitude: AttributionAttitudeIR::Say,
        forms: &[
            "말했습니다",
            "말하였다",
            "말했다",
            "말했지만",
            "말했으나",
            "말한다",
            "말하며",
            "말한",
            "said",
            "says",
            "say",
            "stated",
            "states",
        ],
    },
    AttributionLexeme {
        attitude: AttributionAttitudeIR::Report,
        forms: &[
            "보고했습니다",
            "보고했다",
            "보고한다",
            "전했다",
            "reported",
            "reports",
        ],
    },
    AttributionLexeme {
        attitude: AttributionAttitudeIR::Claim,
        forms: &["주장했습니다", "주장했다", "주장한다", "claimed", "claims"],
    },
    AttributionLexeme {
        attitude: AttributionAttitudeIR::Believe,
        forms: &[
            "믿지 않는다",
            "믿지 않았다",
            "믿습니다",
            "믿는다",
            "믿었다",
            "믿는",
            "does not believe",
            "did not believe",
            "doesn't believe",
            "believed",
            "believes",
        ],
    },
    AttributionLexeme {
        attitude: AttributionAttitudeIR::Think,
        forms: &[
            "생각하지 않는다",
            "생각하지 않았다",
            "생각한다",
            "생각했다",
            "생각하는",
            "does not think",
            "did not think",
            "doesn't think",
            "thought",
            "thinks",
        ],
    },
    AttributionLexeme {
        attitude: AttributionAttitudeIR::Know,
        forms: &["안다고", "알고 있다", "알았다", "knows", "knew"],
    },
    AttributionLexeme {
        attitude: AttributionAttitudeIR::Doubt,
        forms: &["의심한다", "의심했다", "doubts", "doubted"],
    },
    AttributionLexeme {
        attitude: AttributionAttitudeIR::Deny,
        forms: &["부인한다", "부인했다", "부정한다", "denies", "denied"],
    },
    AttributionLexeme {
        attitude: AttributionAttitudeIR::Hear,
        forms: &["들었습니다", "들었다", "들었다고", "들었", "heard", "hears"],
    },
    AttributionLexeme {
        attitude: AttributionAttitudeIR::Observe,
        forms: &["관찰했다", "목격했다", "보았다", "observed", "saw"],
    },
    AttributionLexeme {
        attitude: AttributionAttitudeIR::Infer,
        forms: &["추론했다", "추정했다", "inferred", "concluded"],
    },
    AttributionLexeme {
        attitude: AttributionAttitudeIR::Want,
        forms: &["원한다", "원했다", "바란다", "wants", "wanted", "hopes"],
    },
    AttributionLexeme {
        attitude: AttributionAttitudeIR::Expect,
        forms: &["예상한다", "예상했다", "기대한다", "expects", "expected"],
    },
    AttributionLexeme {
        attitude: AttributionAttitudeIR::Correct,
        forms: &[
            "정정했습니다",
            "정정했다",
            "바로잡았다",
            "corrected",
            "corrects",
        ],
    },
];

#[derive(Debug, Clone)]
struct AttributionMention {
    start: usize,
    end: usize,
    form: String,
    attitude: AttributionAttitudeIR,
}

#[derive(Debug)]
struct PendingAttribution {
    actor_surface: String,
    actor_kind: DiscourseActorKindIR,
    evidence_source: Option<(String, DiscourseActorKindIR)>,
    proposition: AttributedPropositionIR,
    mention_start: usize,
    form: String,
    attitude: AttributionAttitudeIR,
    stance: AttributionStanceIR,
    epistemic_status: EpistemicStatusIR,
    evidence_kind: AttributionEvidenceKindIR,
}

impl AttributionAnalyzer {
    pub fn analyze(&self, text: &str, frames: &[PredicateFrameIR]) -> AttributionGraphIR {
        let normalized = text.to_lowercase();
        let mentions = attribution_mentions(&normalized);
        let mut pending = mentions
            .iter()
            .enumerate()
            .filter_map(|(index, mention)| {
                pending_from_mention(&normalized, &mentions, mention, index)
            })
            .collect::<Vec<_>>();
        if pending.is_empty() {
            if let Some(source) = source_construction(&normalized) {
                pending.push(source);
            }
        }
        if pending.is_empty() {
            return AttributionGraphIR::default();
        }

        for (index, item) in pending.iter_mut().enumerate() {
            item.proposition.proposition_id = format!("PROP-{:02}", index + 1);
        }
        let mut actors = Vec::<DiscourseActorIR>::new();
        let mut attributions = Vec::new();
        for (index, item) in pending.iter().enumerate() {
            let actor_id = add_actor(&mut actors, &item.actor_surface, item.actor_kind);
            let evidence_source_actor_id = item
                .evidence_source
                .as_ref()
                .map(|(surface, kind)| add_actor(&mut actors, surface, *kind));
            let parent_proposition_id = pending
                .iter()
                .filter(|candidate| {
                    candidate.proposition.proposition_id != item.proposition.proposition_id
                        && candidate.proposition.source_start_byte <= item.mention_start
                        && item.mention_start < candidate.proposition.source_end_byte
                })
                .min_by_key(|candidate| {
                    candidate
                        .proposition
                        .source_end_byte
                        .saturating_sub(candidate.proposition.source_start_byte)
                })
                .map(|candidate| candidate.proposition.proposition_id.clone());
            attributions.push(AttributionEdgeIR {
                attribution_id: format!("ATTR-{:02}", index + 1),
                actor_id,
                proposition_id: item.proposition.proposition_id.clone(),
                attitude: item.attitude,
                stance: item.stance,
                epistemic_status: item.epistemic_status,
                evidence_kind: item.evidence_kind,
                evidence_source_actor_id,
                parent_proposition_id,
                evidence_surface: item.form.clone(),
                confidence_millis: 900,
            });
        }
        actors.sort_by(|left, right| left.actor_id.cmp(&right.actor_id));
        let propositions = pending
            .into_iter()
            .map(|item| item.proposition)
            .collect::<Vec<_>>();
        let proposition_ranges = propositions
            .iter()
            .map(|proposition| (proposition.source_start_byte, proposition.source_end_byte))
            .collect::<Vec<_>>();
        let mut attributed_frame_ids = frames
            .iter()
            .filter(|frame| {
                proposition_ranges.iter().any(|(start, end)| {
                    *start <= frame.source_start_byte && frame.source_start_byte < *end
                })
            })
            .map(|frame| frame.frame_id.clone())
            .collect::<Vec<_>>();
        attributed_frame_ids.sort();
        attributed_frame_ids.dedup();
        let resolved = attributions.len();
        let structural_coverage_millis = u16::try_from(
            resolved
                .saturating_mul(1_000)
                .checked_div(mentions.len().max(1))
                .unwrap_or_default()
                .min(1_000),
        )
        .unwrap_or(1_000);
        let graph = AttributionGraphIR {
            schema: ATTRIBUTION_GRAPH_SCHEMA.to_string(),
            actors,
            propositions,
            attributions,
            attributed_frame_ids,
            unresolved_attributions: Vec::new(),
            structural_coverage_millis,
        };
        debug_assert!(graph.validate());
        graph
    }
}

fn attribution_mentions(text: &str) -> Vec<AttributionMention> {
    let mut mentions = Vec::new();
    for lexeme in ATTRIBUTION_LEXEMES {
        for form in lexeme.forms {
            for (start, _) in text.match_indices(form) {
                let end = start + form.len();
                if form.is_ascii() && !ascii_word_bounds(text, start, end) {
                    continue;
                }
                mentions.push(AttributionMention {
                    start,
                    end,
                    form: (*form).to_string(),
                    attitude: lexeme.attitude,
                });
            }
        }
    }
    mentions.sort_by(|left, right| {
        left.start
            .cmp(&right.start)
            .then_with(|| right.end.cmp(&left.end))
    });
    let mut selected = Vec::<AttributionMention>::new();
    for mention in mentions {
        if selected
            .iter()
            .any(|existing| existing.start <= mention.start && mention.end <= existing.end)
        {
            continue;
        }
        selected.push(mention);
    }
    selected.sort_by_key(|mention| mention.start);
    selected
}

fn pending_from_mention(
    text: &str,
    mentions: &[AttributionMention],
    mention: &AttributionMention,
    index: usize,
) -> Option<PendingAttribution> {
    let korean = contains_hangul(text);
    let clause_start = text[..mention.start]
        .rfind(['.', '?', '!', ';', '\n', '\r'])
        .map_or(0, |position| position + 1);
    let clause_end = text[mention.end..]
        .find(['.', '?', '!', ';', '\n', '\r'])
        .map_or(text.len(), |offset| mention.end + offset);
    let prior_end = index
        .checked_sub(1)
        .and_then(|prior| mentions.get(prior))
        .filter(|prior| prior.end > clause_start)
        .map_or(clause_start, |prior| prior.end);
    let outermost = !mentions
        .iter()
        .skip(index + 1)
        .any(|candidate| candidate.start < clause_end);
    let (actor_surface, actor_kind, actor_end) = if korean {
        extract_korean_actor(text, clause_start, mention.start, outermost)
    } else {
        extract_english_actor(text, prior_end, mention.start)
    }?;
    let raw_complement = text.get(actor_end..mention.start).unwrap_or_default();
    let complement_is_quoted = is_quoted_surface(raw_complement);
    let has_korean_complementizer = ["다고", "라고", "라며", "기를", "고 싶"]
        .iter()
        .any(|marker| raw_complement.contains(marker));
    if korean
        && attitude_requires_proposition(mention.attitude)
        && !has_korean_complementizer
        && !complement_is_quoted
    {
        return None;
    }
    let (mut proposition_start, proposition_end) = if korean {
        let end = korean_complement_end(text, actor_end, mention.start);
        (actor_end, end)
    } else {
        (
            mention.end,
            english_complement_end(text, mention.end, clause_end),
        )
    };
    proposition_start = skip_complement_prefix(text, proposition_start, proposition_end);
    let proposition_source_is_quoted = text
        .get(proposition_start..proposition_end)
        .is_some_and(is_quoted_surface);
    let mut proposition_surface = text
        .get(proposition_start..proposition_end)?
        .trim()
        .trim_matches(|character| {
            matches!(character, ':' | ',' | '"' | '\'' | '‘' | '’' | '“' | '”')
        })
        .trim()
        .to_string();
    let mut evidence_source = None;
    if mention.attitude == AttributionAttitudeIR::Hear {
        let (cleaned, source) =
            extract_hearsay_source(&proposition_surface, text, actor_end, mention.start);
        proposition_surface = cleaned;
        evidence_source = source.map(|surface| (surface, DiscourseActorKindIR::NamedEntity));
    }
    if proposition_surface.is_empty() {
        return None;
    }
    if !korean
        && !proposition_source_is_quoted
        && !looks_like_english_proposition(&proposition_surface)
    {
        return None;
    }
    let actual_start = text[proposition_start..proposition_end]
        .find(&proposition_surface)
        .map_or(proposition_start, |offset| proposition_start + offset);
    let actual_end = actual_start + proposition_surface.len();
    let negative_attribution = is_negative_attribution(text, mention);
    let (stance, epistemic_status, evidence_kind) =
        attribution_semantics(mention.attitude, negative_attribution);
    Some(PendingAttribution {
        actor_surface,
        actor_kind,
        evidence_source,
        proposition: AttributedPropositionIR {
            proposition_id: String::new(),
            normalized_text: normalize_proposition(&proposition_surface),
            polarity: proposition_polarity(&proposition_surface),
            quoted: proposition_source_is_quoted,
            surface_text: proposition_surface,
            source_start_byte: actual_start,
            source_end_byte: actual_end,
            dialogue_truth_established: false,
            external_execution_authorized: false,
        },
        mention_start: mention.start,
        form: mention.form.clone(),
        attitude: mention.attitude,
        stance,
        epistemic_status,
        evidence_kind,
    })
}

fn source_construction(text: &str) -> Option<PendingAttribution> {
    let patterns = [
        ("according to ", ",", DiscourseActorKindIR::DocumentSource),
        ("에 따르면 ", "", DiscourseActorKindIR::DocumentSource),
    ];
    for (prefix, delimiter, kind) in patterns {
        let Some(start) = text.find(prefix) else {
            continue;
        };
        if prefix.is_ascii() {
            let source_start = start + prefix.len();
            let Some(comma_offset) = text[source_start..].find(delimiter) else {
                continue;
            };
            let comma = comma_offset + source_start;
            let source = text[source_start..comma].trim();
            let content_start = comma + delimiter.len();
            let content = text[content_start..].trim();
            if source.is_empty() || content.is_empty() {
                return None;
            }
            return Some(synthetic_source_attribution(
                text,
                source,
                content,
                content_start,
                kind,
            ));
        }
        let marker = start;
        let before = text[..marker].trim();
        let source = before.split_whitespace().last()?.trim_end_matches('에');
        let content_start = marker + prefix.len();
        let content = text[content_start..].trim();
        if source.is_empty() || content.is_empty() {
            return None;
        }
        return Some(synthetic_source_attribution(
            text,
            source,
            content,
            content_start,
            kind,
        ));
    }
    None
}

fn synthetic_source_attribution(
    text: &str,
    source: &str,
    content: &str,
    content_start: usize,
    kind: DiscourseActorKindIR,
) -> PendingAttribution {
    let actual_start = text[content_start..]
        .find(content)
        .map_or(content_start, |offset| content_start + offset);
    PendingAttribution {
        actor_surface: source.to_string(),
        actor_kind: kind,
        evidence_source: None,
        proposition: AttributedPropositionIR {
            proposition_id: String::new(),
            surface_text: content.to_string(),
            normalized_text: normalize_proposition(content),
            polarity: proposition_polarity(content),
            source_start_byte: actual_start,
            source_end_byte: actual_start + content.len(),
            quoted: false,
            dialogue_truth_established: false,
            external_execution_authorized: false,
        },
        mention_start: content_start,
        form: "SOURCE_CONSTRUCTION".to_string(),
        attitude: AttributionAttitudeIR::Report,
        stance: AttributionStanceIR::Withholds,
        epistemic_status: EpistemicStatusIR::Reported,
        evidence_kind: AttributionEvidenceKindIR::Document,
    }
}

fn extract_english_actor(
    text: &str,
    start: usize,
    end: usize,
) -> Option<(String, DiscourseActorKindIR, usize)> {
    let segment = text.get(start..end)?.trim();
    let segment = segment
        .trim_start_matches("that ")
        .trim_start_matches("and ")
        .trim_start_matches("actually, ")
        .trim_start_matches("actually ")
        .trim_start_matches("in fact, ")
        .trim_start_matches("in fact ")
        .trim_matches(|character| {
            matches!(character, ',' | ':' | '"' | '\'' | '‘' | '’' | '“' | '”')
        })
        .trim();
    if segment.is_empty() {
        return None;
    }
    let mut words = segment.split_whitespace().collect::<Vec<_>>();
    while words
        .last()
        .is_some_and(|word| matches!(*word, "now" | "currently" | "actually"))
    {
        words.pop();
    }
    if words.is_empty() {
        return None;
    }
    let actor_words = if words.len() > 4 {
        &words[words.len() - 4..]
    } else {
        &words[..]
    };
    let surface = actor_words.join(" ");
    let local = text[start..end].rfind(&surface)?;
    let actor_end = start + local + surface.len();
    let kind = if matches!(surface.as_str(), "i" | "we") {
        DiscourseActorKindIR::DialogueSpeaker
    } else {
        DiscourseActorKindIR::NamedEntity
    };
    Some((surface, kind, actor_end))
}

fn extract_korean_actor(
    text: &str,
    start: usize,
    end: usize,
    outermost: bool,
) -> Option<(String, DiscourseActorKindIR, usize)> {
    let segment = text.get(start..end)?;
    let tokens = segment
        .split_whitespace()
        .filter_map(|raw| {
            let clean = raw.trim_matches(|character: char| {
                matches!(character, ',' | ':' | '"' | '\'' | '‘' | '’' | '“' | '”')
            });
            let particle_len = ["은", "는", "이", "가"]
                .iter()
                .find(|particle| clean.ends_with(**particle))
                .map(|particle| particle.len())?;
            let surface = clean[..clean.len().saturating_sub(particle_len)].trim();
            (!surface.is_empty()).then_some((clean, surface))
        })
        .collect::<Vec<_>>();
    let selected = if outermost {
        tokens.first()
    } else {
        tokens.get(1).or_else(|| tokens.first())
    }?;
    let local = segment.find(selected.0)?;
    let actor_end = start + local + selected.0.len();
    let kind = if matches!(selected.1, "나" | "저" | "우리") {
        DiscourseActorKindIR::DialogueSpeaker
    } else {
        DiscourseActorKindIR::NamedEntity
    };
    Some((selected.1.to_string(), kind, actor_end))
}

fn korean_complement_end(text: &str, start: usize, end: usize) -> usize {
    let segment = text.get(start..end).unwrap_or_default();
    for suffix in ["는다고", "이라고", "다고", "라고", "라며"] {
        if let Some(position) = segment.rfind(suffix) {
            return start + position;
        }
    }
    end
}

fn english_complement_end(text: &str, start: usize, clause_end: usize) -> usize {
    let segment = text.get(start..clause_end).unwrap_or_default();
    [", but ", ", however ", ", and now ", "; but "]
        .iter()
        .filter_map(|marker| segment.find(marker))
        .min()
        .map_or(clause_end, |offset| start + offset)
}

fn attitude_requires_proposition(attitude: AttributionAttitudeIR) -> bool {
    !matches!(attitude, AttributionAttitudeIR::Want)
}

fn looks_like_english_proposition(text: &str) -> bool {
    let words = text
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    words.len() >= 3
        || (words.first() == Some(&"to") && words.len() >= 2)
        || words.iter().any(|word| {
            [
                "is", "are", "was", "were", "will", "can", "did", "does", "has", "have", "delete",
                "save", "run", "deploy", "repair", "stop", "failed", "finished",
            ]
            .contains(word)
        })
        || words
            .get(1)
            .is_some_and(|predicate| predicate.ends_with("ed"))
}

fn skip_complement_prefix(text: &str, mut start: usize, end: usize) -> usize {
    let Some(segment) = text.get(start..end) else {
        return start;
    };
    let trimmed = segment.trim_start();
    start += segment.len().saturating_sub(trimmed.len());
    for prefix in ["that ", "whether ", "if ", ": ", ", "] {
        if text[start..end].starts_with(prefix) {
            start += prefix.len();
            break;
        }
    }
    start
}

fn extract_hearsay_source(
    proposition: &str,
    text: &str,
    actor_end: usize,
    mention_start: usize,
) -> (String, Option<String>) {
    let lower = proposition.to_lowercase();
    if let Some(rest) = lower.strip_prefix("from ") {
        if let Some(that) = rest.find(" that ") {
            let source = proposition[5..5 + that].trim().to_string();
            let content = proposition[5 + that + 6..].trim().to_string();
            return (content, Some(source));
        }
    }
    let between = text.get(actor_end..mention_start).unwrap_or_default();
    for particle in ["에게서", "한테서"] {
        if let Some(position) = between.find(particle) {
            let before = &between[..position];
            if let Some(source) = before.split_whitespace().last() {
                let cleaned = proposition
                    .trim_start_matches(source)
                    .trim_start_matches(particle)
                    .trim()
                    .to_string();
                return (cleaned, Some(source.to_string()));
            }
        }
    }
    (proposition.to_string(), None)
}

fn attribution_semantics(
    attitude: AttributionAttitudeIR,
    explicitly_negated: bool,
) -> (
    AttributionStanceIR,
    EpistemicStatusIR,
    AttributionEvidenceKindIR,
) {
    if explicitly_negated {
        return (
            AttributionStanceIR::Rejects,
            EpistemicStatusIR::Denied,
            AttributionEvidenceKindIR::Unspecified,
        );
    }
    match attitude {
        AttributionAttitudeIR::Say | AttributionAttitudeIR::Report => (
            AttributionStanceIR::Withholds,
            EpistemicStatusIR::Reported,
            AttributionEvidenceKindIR::Speech,
        ),
        AttributionAttitudeIR::Claim => (
            AttributionStanceIR::Endorses,
            EpistemicStatusIR::Claimed,
            AttributionEvidenceKindIR::Speech,
        ),
        AttributionAttitudeIR::Believe | AttributionAttitudeIR::Think => (
            AttributionStanceIR::Endorses,
            EpistemicStatusIR::Believed,
            AttributionEvidenceKindIR::Unspecified,
        ),
        AttributionAttitudeIR::Know => (
            AttributionStanceIR::Endorses,
            EpistemicStatusIR::PresentedAsKnown,
            AttributionEvidenceKindIR::Unspecified,
        ),
        AttributionAttitudeIR::Doubt => (
            AttributionStanceIR::Withholds,
            EpistemicStatusIR::Doubted,
            AttributionEvidenceKindIR::Unspecified,
        ),
        AttributionAttitudeIR::Deny => (
            AttributionStanceIR::Rejects,
            EpistemicStatusIR::Denied,
            AttributionEvidenceKindIR::Speech,
        ),
        AttributionAttitudeIR::Hear => (
            AttributionStanceIR::Withholds,
            EpistemicStatusIR::Hearsay,
            AttributionEvidenceKindIR::Hearsay,
        ),
        AttributionAttitudeIR::Observe => (
            AttributionStanceIR::Endorses,
            EpistemicStatusIR::Observed,
            AttributionEvidenceKindIR::DirectObservation,
        ),
        AttributionAttitudeIR::Infer => (
            AttributionStanceIR::Endorses,
            EpistemicStatusIR::Inferred,
            AttributionEvidenceKindIR::Inference,
        ),
        AttributionAttitudeIR::Want => (
            AttributionStanceIR::Desires,
            EpistemicStatusIR::Desired,
            AttributionEvidenceKindIR::Unspecified,
        ),
        AttributionAttitudeIR::Expect => (
            AttributionStanceIR::Predicts,
            EpistemicStatusIR::Expected,
            AttributionEvidenceKindIR::Inference,
        ),
        AttributionAttitudeIR::Correct => (
            AttributionStanceIR::Endorses,
            EpistemicStatusIR::Corrected,
            AttributionEvidenceKindIR::Speech,
        ),
    }
}

fn is_negative_attribution(text: &str, mention: &AttributionMention) -> bool {
    let form = mention.form.as_str();
    if form.contains("not ")
        || form.contains("n't ")
        || form.contains("지 않")
        || form.contains("지 못")
    {
        return true;
    }
    if form.is_ascii() {
        let start = nearest_char_boundary(text, mention.start.saturating_sub(16));
        let prefix = text[start..mention.start].trim_end();
        return prefix.ends_with("not") || prefix.ends_with("never");
    }
    false
}

fn proposition_polarity(text: &str) -> AttributedPropositionPolarityIR {
    let lower = text.to_lowercase();
    if [" not ", "never ", "no ", "않", "없", "아니", "못"]
        .iter()
        .any(|marker| lower.contains(marker))
        || lower.starts_with("not ")
        || lower.starts_with("no ")
    {
        AttributedPropositionPolarityIR::Negative
    } else {
        AttributedPropositionPolarityIR::Positive
    }
}

fn normalize_proposition(text: &str) -> String {
    text.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_quoted_surface(text: &str) -> bool {
    text.chars()
        .any(|character| matches!(character, '"' | '\'' | '‘' | '’' | '“' | '”'))
}

fn add_actor(
    actors: &mut Vec<DiscourseActorIR>,
    surface: &str,
    kind: DiscourseActorKindIR,
) -> String {
    let normalized = surface
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if let Some(actor) = actors
        .iter()
        .find(|actor| actor.normalized_label == normalized)
    {
        return actor.actor_id.clone();
    }
    let actor_id = format!("ACTOR-{:02}", actors.len() + 1);
    actors.push(DiscourseActorIR {
        actor_id: actor_id.clone(),
        kind,
        surface: surface.trim().to_string(),
        normalized_label: normalized,
    });
    actor_id
}

fn ascii_word_bounds(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    !before.is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
        && !after.is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn contains_hangul(text: &str) -> bool {
    text.chars()
        .any(|character| ('\u{ac00}'..='\u{d7a3}').contains(&character))
}

fn nearest_char_boundary(text: &str, mut index: usize) -> usize {
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositional_semantics::CompositionalSemanticAnalyzer;

    fn analyze(text: &str) -> AttributionGraphIR {
        let composition = CompositionalSemanticAnalyzer.analyze(text);
        AttributionAnalyzer.analyze(text, &composition.frames)
    }

    #[test]
    fn korean_report_keeps_source_and_uncommitted_truth() {
        let graph = analyze("민수는 서버가 멈췄다고 말했다.");
        assert!(graph.validate());
        assert_eq!(graph.actors[0].normalized_label, "민수");
        assert_eq!(graph.propositions[0].normalized_text, "서버가 멈췄");
        assert_eq!(
            graph.attributions[0].epistemic_status,
            EpistemicStatusIR::Reported
        );
        assert!(!graph.propositions[0].dialogue_truth_established);
    }

    #[test]
    fn english_negative_belief_is_source_stance_not_proposition_negation() {
        let graph = analyze("Alice does not believe that the server is down.");
        assert!(graph.validate());
        assert_eq!(graph.attributions[0].stance, AttributionStanceIR::Rejects);
        assert_eq!(
            graph.propositions[0].polarity,
            AttributedPropositionPolarityIR::Positive
        );
    }

    #[test]
    fn english_nested_attribution_has_parent_proposition() {
        let graph = analyze("Alice says Bob believes that the server is down.");
        assert!(graph.validate());
        assert_eq!(graph.attributions.len(), 2);
        assert_eq!(
            graph
                .attributions
                .iter()
                .filter(|edge| edge.parent_proposition_id.is_some())
                .count(),
            1
        );
    }

    #[test]
    fn attributed_command_frame_has_no_authority() {
        let composition = CompositionalSemanticAnalyzer.analyze("Alice said, delete the file.");
        assert!(composition
            .candidates
            .iter()
            .filter(|candidate| candidate.intent == dockable_semantic_core::PlanIntentIR::Execute)
            .all(|candidate| !candidate.external_execution_authorized));
    }

    #[test]
    fn document_source_construction_is_not_dialogue_truth() {
        let graph = analyze("According to the audit, the cache is corrupt.");
        assert!(graph.validate());
        assert_eq!(
            graph.attributions[0].evidence_kind,
            AttributionEvidenceKindIR::Document
        );
        assert!(!graph.propositions[0].dialogue_truth_established);
    }
}
