//! Bounded affective-pragmatic estimates, never psychological facts or action
//! authority. Signed fixed-point weights represent a continuous field; they
//! are not mutually exclusive emotion labels or calibrated probabilities.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// A bounded linguistic cue for response selection, not a psychological fact.
/// Field weights may still reflect tone; an unendorsed cue cannot justify an
/// affirmative affect sentence or select a task on the user's behalf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpressedAffectIR {
    Frustrated,
    Angry,
    Hurt,
    Worried,
    Annoyed,
}

pub(crate) fn expressed_affect(text: &str) -> Option<ExpressedAffectIR> {
    use ExpressedAffectIR::*;
    let mut unquoted = String::new();
    let mut closing = None;
    for c in text.chars() {
        if let Some(end) = closing {
            if c == end {
                closing = None;
            }
            unquoted.push(' ');
        } else {
            closing = match c {
                '‘' => Some('’'),
                '“' => Some('”'),
                '"' => Some('"'),
                '`' => Some('`'),
                _ => None,
            };
            unquoted.push(if closing.is_some() { ' ' } else { c });
        }
    }
    let mut lower = unquoted.to_lowercase();
    // Attribution already owns these proposition spans. A reported, denied,
    // imagined or believed emotion is not an affect assertion by this speaker.
    // Preserve independent matrix clauses outside the attributed spans.
    let attribution = crate::attribution::AttributionAnalyzer.analyze(&lower, &[]);
    for proposition in &attribution.propositions {
        let start = proposition.source_start_byte;
        let end = proposition.source_end_byte;
        if start < end
            && end <= lower.len()
            && lower.is_char_boundary(start)
            && lower.is_char_boundary(end)
        {
            lower.replace_range(start..end, &" ".repeat(end - start));
        }
    }
    for clause in lower.split(['.', '!', '?', ';', '\n']) {
        let words = clause.split_whitespace().collect::<Vec<_>>();
        for (i, word) in words.iter().enumerate() {
            let word = word.trim_matches(|c: char| c.is_ascii_punctuation());
            let kind = if word.is_ascii() {
                match word {
                    "frustrating" | "frustrated" => Frustrated,
                    "angry" => Angry,
                    "upset" | "hurt" => Hurt,
                    "worried" | "worrying" => Worried,
                    "annoying" | "annoyed" => Annoyed,
                    _ => continue,
                }
            } else if ["답답", "지친", "지쳤", "힘들어"]
                .iter()
                .any(|s| word.starts_with(s))
                || word == "진이" && words.get(i + 1).is_some_and(|w| w.starts_with("빠"))
            {
                Frustrated
            } else if word.starts_with("화나")
                || word == "화가" && words.get(i + 1).is_some_and(|w| w.starts_with('나'))
            {
                Angry
            } else if word.starts_with("속상") {
                Hurt
            } else if ["불안", "걱정"].iter().any(|s| word.starts_with(s)) {
                Worried
            } else if ["짜증", "킹받"].iter().any(|s| word.starts_with(s)) {
                Annoyed
            } else {
                continue;
            };
            let before = words[..i].iter().rev().find(|w| {
                !matches!(
                    **w,
                    "very" | "really" | "particularly" | "별로" | "정말" | "좀" | "전혀"
                )
            });
            let prefix_negated = before.is_some_and(|w| {
                matches!(
                    *w,
                    "not" | "never" | "isn't" | "wasn't" | "aren't" | "weren't" | "안" | "못"
                )
            });
            let next = words.get(i + 1).copied().unwrap_or("");
            let negation_host = ["는", "도", "만"]
                .iter()
                .find_map(|particle| word.strip_suffix(particle))
                .unwrap_or(word);
            let suffix_negated = negation_host.ends_with('지')
                && (next.starts_with("않") || next.starts_with("못"))
                || word.contains("없")
                || next.starts_with("없");
            let quoted_complement = word.ends_with("다고") || word.ends_with("다며");
            if !prefix_negated && !suffix_negated && !quoted_complement {
                // A complete attributed state already owns its bearer and
                // predicate. Tone may shape delivery, not replace the meaning
                // or transfer somebody else's state to the user. Only consult
                // this boundary when there is an actual affect candidate.
                if crate::proposition_content::described_event(text, false).is_some_and(|event| {
                    event.kind == crate::proposition_content::DescriptionKindIR::State
                }) {
                    return None;
                }
                return Some(kind);
            }
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AffectAxisIR {
    Valence,
    Arousal,
    Dominance,
    Certainty,
    Urgency,
    Frustration,
    Affiliation,
    Playfulness,
    Formality,
    Trust,
    Confrontation,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectEstimateIR {
    pub value_millis: i16,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectSignalIR {
    pub axis: AffectAxisIR,
    pub weight_millis: i16,
    pub cue: String,
    pub token_index: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectiveFieldIR {
    pub axes: BTreeMap<AffectAxisIR, AffectEstimateIR>,
    pub observations: Vec<AffectSignalIR>,
    pub observed_tokens: usize,
    pub repeated_tokens: usize,
    pub punctuation_count: usize,
    pub tone_delta_millis: u16,
    pub response_interval_ms: Option<u64>,
    pub field_sha256: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KoreanDialectIR {
    #[default]
    Standard,
    Gyeongsang,
    Chungcheong,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectiveRealizationPolicyIR {
    pub formal: bool,
    pub warmth_millis: u16,
    pub playfulness_millis: u16,
    pub brevity_millis: u16,
    pub urgency_millis: u16,
    #[serde(default)]
    pub korean_dialect: KoreanDialectIR,
}

/// Host-configured expression tendencies, separate from observations about
/// the user's tone. These are not beliefs, emotions experienced by the core,
/// confidence in an answer, or permission to act.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialoguePersonalityIR {
    pub warmth_millis: u16,
    pub playfulness_millis: u16,
    pub formality_millis: u16,
    pub concision_millis: u16,
    #[serde(default)]
    pub korean_dialect: KoreanDialectIR,
}

impl DialoguePersonalityIR {
    pub fn validate(&self) -> bool {
        [
            self.warmth_millis,
            self.playfulness_millis,
            self.formality_millis,
            self.concision_millis,
        ]
        .into_iter()
        .all(|value| value <= 1_000)
    }
}

impl AffectiveFieldIR {
    pub fn value(&self, axis: AffectAxisIR) -> i16 {
        self.axes.get(&axis).map_or(0, |value| value.value_millis)
    }

    pub fn observe(previous: Option<&Self>, text: &str, interval_ms: Option<u64>) -> Self {
        use AffectAxisIR::*;
        let previous = previous.filter(|field| field.validate());
        // Absence of timing is unknown, not fast/slow. Timing alone does not
        // infer emotion: it can have many unrelated causes.
        let mut field = Self {
            response_interval_ms: interval_ms,
            ..Self::default()
        };
        if let Some(previous) = previous {
            for (axis, estimate) in &previous.axes {
                field.axes.insert(
                    *axis,
                    AffectEstimateIR {
                        value_millis: estimate.value_millis * 3 / 4,
                        confidence_millis: estimate.confidence_millis * 3 / 4,
                    },
                );
            }
        }
        // Quoted/code content is not evidence of the speaker's affect.
        let mut quote_end = None;
        let surface = text
            .chars()
            .take(4096)
            .map(|c| {
                if let Some(end) = quote_end {
                    if c == end {
                        quote_end = None;
                    }
                    return ' ';
                }
                quote_end = match c {
                    '"' => Some('"'),
                    '`' => Some('`'),
                    '“' => Some('”'),
                    '‘' => Some('’'),
                    _ => None,
                };
                if quote_end.is_some() {
                    ' '
                } else {
                    c
                }
            })
            .collect::<String>()
            .to_lowercase();
        let tokens = surface.split_whitespace().take(512).collect::<Vec<_>>();
        field.observed_tokens = tokens.len();
        field.punctuation_count = surface
            .chars()
            .filter(|c| matches!(c, '!' | '?' | '！' | '？'))
            .count();
        // Punctuation is weak arousal evidence, never certainty or hostility.
        let exclamations = surface.chars().filter(|c| matches!(c, '!' | '！')).count();
        if exclamations > 0 {
            field.observations.push(AffectSignalIR {
                axis: Arousal,
                weight_millis: (exclamations.min(5) * 30) as i16,
                cue: "EXCLAMATION_DENSITY".into(),
                token_index: 0,
            });
        }
        let mut seen = BTreeMap::new();
        for (index, token) in tokens.iter().enumerate() {
            let word = token.trim_matches(|c: char| !c.is_alphanumeric());
            let count = seen.entry(word).or_insert(0usize);
            *count += 1;
            field.repeated_tokens += usize::from(*count > 1);
            let emphasis = if *count > 1 || token.ends_with('!') {
                3
            } else {
                2
            };
            let negated = index > 0 && matches!(tokens[index - 1], "not" | "안" | "못");
            let cues: &[(AffectAxisIR, i16)] = match word {
                "ㅋㅋ" | "ㅋㅋㅋ" | "ㅎㅎ" | "lol" | "haha" => {
                    &[(Playfulness, 650), (Affiliation, 220), (Arousal, 200)]
                }
                "고마워" | "감사합니다" | "thanks" | "thank" => {
                    &[(Affiliation, 650), (Valence, 400)]
                }
                "아니" | "아니라" | "no" => &[(Certainty, 180), (Confrontation, 100)],
                "답답해" | "짜증나" | "frustrated" | "annoyed" => {
                    &[(Frustration, 700), (Valence, -500), (Arousal, 400)]
                }
                "씨발" | "젠장" | "fuck" | "damn" => &[(Arousal, 600), (Frustration, 300)],
                "빨리" | "급해" | "당장" | "urgent" | "quickly" | "asap" => {
                    &[(Urgency, 800), (Arousal, 400)]
                }
                "아마" | "maybe" | "perhaps" => &[(Certainty, -550), (Dominance, -150)],
                "확실히" | "분명히" | "certainly" | "definitely" => &[(Certainty, 600)],
                "제발" | "부탁" | "please" => {
                    &[(Affiliation, 200), (Dominance, -200), (Formality, 200)]
                }
                "믿어" | "trust" => &[(Trust, 450)],
                "행복해" | "happy" => &[(Valence, 600)],
                "슬퍼" | "sad" => &[(Valence, -600)],
                _ => &[],
            };
            for &(axis, weight) in cues {
                if field.observations.len() == 32 {
                    break;
                }
                let sign = if negated && matches!(axis, Valence | Trust) {
                    -1
                } else {
                    1
                };
                // Fronted cues carry mild emphasis, not semantic precedence.
                let positional = if index == 0 { 110 } else { 100 };
                field.observations.push(AffectSignalIR {
                    axis,
                    weight_millis: (i32::from(weight) * emphasis * sign * positional / 200)
                        .clamp(-1000, 1000) as i16,
                    cue: word.to_string(),
                    token_index: index,
                });
            }
        }
        let tail =
            surface.trim_end_matches(|c: char| c.is_whitespace() || c.is_ascii_punctuation());
        if (["습니다", "습니까", "주세요", "요"]
            .iter()
            .any(|suffix| tail.ends_with(suffix))
            || crate::compositional_semantics::formal_korean_benefactive(tail))
            && field.observations.len() < 32
        {
            field.observations.push(AffectSignalIR {
                axis: Formality,
                weight_millis: 700,
                cue: "POLITE_ENDING".into(),
                token_index: tokens.len().saturating_sub(1),
            });
        }
        for observation in &field.observations {
            let estimate = field.axes.entry(observation.axis).or_default();
            estimate.value_millis = (i32::from(estimate.value_millis)
                + i32::from(observation.weight_millis) / 3)
                .clamp(-1000, 1000) as i16;
            estimate.confidence_millis = (estimate.confidence_millis + 120).min(650);
        }
        // Laughter moderates confrontational style without negating a refusal.
        if field.value(Playfulness) > 100 {
            if let Some(value) = field.axes.get_mut(&Confrontation) {
                value.value_millis /= 2;
            }
        }
        field.tone_delta_millis = field
            .axes
            .iter()
            .map(|(axis, value)| {
                (i32::from(value.value_millis)
                    - i32::from(previous.map_or(0, |prior| prior.value(*axis))))
                .unsigned_abs()
            })
            .max()
            .unwrap_or(0)
            .min(2000) as u16;
        field.field_sha256 = field.hash();
        field
    }

    pub fn policy(&self) -> AffectiveRealizationPolicyIR {
        use AffectAxisIR::*;
        AffectiveRealizationPolicyIR {
            formal: self.value(Formality) >= 150,
            warmth_millis: self.value(Affiliation).max(0) as u16,
            playfulness_millis: if self.value(Urgency) > 150 || self.value(Frustration) > 150 {
                0
            } else {
                self.value(Playfulness).max(0) as u16
            },
            brevity_millis: self.value(Urgency).max(self.value(Frustration)).max(0) as u16,
            urgency_millis: self.value(Urgency).max(0) as u16,
            korean_dialect: KoreanDialectIR::Standard,
        }
    }

    pub fn policy_with_personality(
        &self,
        personality: &DialoguePersonalityIR,
    ) -> AffectiveRealizationPolicyIR {
        let mut policy = self.policy();
        // Invalid host configuration cannot participate in realization.
        if !personality.validate() {
            return policy;
        }
        let blend = |observed: u16, tendency: u16| observed.saturating_add(tendency).min(1_000);
        policy.formal |= personality.formality_millis >= 150;
        policy.warmth_millis = blend(policy.warmth_millis, personality.warmth_millis);
        policy.brevity_millis = blend(policy.brevity_millis, personality.concision_millis);
        policy.korean_dialect = personality.korean_dialect;
        // A playful personality must not override current distress or urgency.
        policy.playfulness_millis = if self.value(AffectAxisIR::Urgency) > 150
            || self.value(AffectAxisIR::Frustration) > 150
        {
            0
        } else {
            blend(policy.playfulness_millis, personality.playfulness_millis)
        };
        policy
    }

    pub fn validate(&self) -> bool {
        self.axes.len() <= 11
            && self.observations.len() <= 32
            && self.observed_tokens <= 512
            && self.tone_delta_millis <= 2000
            && self.axes.values().all(|value| {
                (-1000..=1000).contains(&value.value_millis) && value.confidence_millis <= 650
            })
            && self.field_sha256 == self.hash()
    }

    fn hash(&self) -> String {
        let mut value = self.clone();
        value.field_sha256.clear();
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&value).expect("serializable affect field"))
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn field_accumulates_decays_and_retains_uncertainty() {
        let a = AffectiveFieldIR::observe(None, "ㅋㅋ 아니 그건 아니지", None);
        let b = AffectiveFieldIR::observe(Some(&a), "급해 빨리!", None);
        assert!(a.validate() && b.validate());
        assert!(a.value(AffectAxisIR::Playfulness) > 0);
        assert!(b.value(AffectAxisIR::Playfulness) < a.value(AffectAxisIR::Playfulness));
        assert!(b.value(AffectAxisIR::Urgency) > 0);
        assert_eq!(b.policy().playfulness_millis, 0);
        assert!(b.response_interval_ms.is_none());
        assert!(AffectiveFieldIR::observe(None, "\"fuck\"이라는 단어", None)
            .observations
            .is_empty());
        for _ in 0..100 {
            assert!(
                AffectiveFieldIR::observe(Some(&b), "급해 ".repeat(1000).as_str(), None).validate()
            );
        }
    }
}
