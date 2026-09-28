//! Conversation-local predicate contracts and versioned lexical knowledge.
//! Registration supplies a boolean predicate, NOT a discovered concept or a
//! world fact. Only supplied premises/mechanisms give it inferential content.
use crate::language_knowledge::LanguageCodeIR;
use crate::world_dialogue::{WorldAtomIR, WorldPropertyIR};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const ENTRIES: usize = 128;
const REVISIONS: usize = 32;
const SYNTAX_OBSERVATIONS: usize = 128;
pub const WORLD_SYNTAX_MODEL_SCHEMA: &str = "B_CORE_WORLD_SYNTAX_MODEL_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldBinaryOrderIR {
    SubjectObject,
    ObjectSubject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldSyntaxRoleIR {
    Subject,
    Object,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldSyntaxResolutionModeIR {
    CandidateOnly,
    Resolve,
}

/// One supplied structural annotation. It contains no sentence, answer or
/// lexical identity; only a bounded Korean binary argument-order observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldSyntaxObservationIR {
    pub language: LanguageCodeIR,
    pub grammar: WorldLexicalGrammarIR,
    pub subject_particle: String,
    pub object_particle: String,
    pub subject_position: u8,
    pub object_position: u8,
    pub source_ref: String,
    pub source_version: String,
}

impl WorldSyntaxObservationIR {
    fn validate(&self) -> bool {
        self.language == LanguageCodeIR::Korean
            && matches!(
                self.grammar,
                WorldLexicalGrammarIR::KoreanHadaLocative
                    | WorldLexicalGrammarIR::KoreanHadaAccusative
            )
            && matches!(self.subject_particle.as_str(), "은" | "는" | "이" | "가")
            && match self.grammar {
                WorldLexicalGrammarIR::KoreanHadaLocative => self.object_particle == "에",
                WorldLexicalGrammarIR::KoreanHadaAccusative => {
                    matches!(self.object_particle.as_str(), "을" | "를")
                }
                _ => false,
            }
            && self.subject_position <= 1
            && self.object_position <= 1
            && self.subject_position != self.object_position
            && !self.source_ref.trim().is_empty()
            && self.source_ref.chars().count() <= 128
            && !self.source_version.trim().is_empty()
            && self.source_version.chars().count() <= 64
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldSyntaxOrderStatsIR {
    pub grammar: WorldLexicalGrammarIR,
    pub role: WorldSyntaxRoleIR,
    pub particle: String,
    pub position: u8,
    pub support: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldSyntaxModelIR {
    pub schema: String,
    pub model_id: String,
    pub model_version: String,
    pub training_mode: String,
    pub resolution_mode: WorldSyntaxResolutionModeIR,
    pub source_refs: Vec<String>,
    pub source_versions: Vec<String>,
    pub observation_count: usize,
    pub observation_sha256: String,
    pub order_stats: Vec<WorldSyntaxOrderStatsIR>,
    pub training_sha256: String,
}

impl WorldSyntaxModelIR {
    /// Native bounded supervised training over structural annotations only.
    pub fn train(
        model_id: impl Into<String>,
        model_version: impl Into<String>,
        observations: &[WorldSyntaxObservationIR],
    ) -> Result<Self, String> {
        Self::train_with_mode(
            model_id,
            model_version,
            WorldSyntaxResolutionModeIR::Resolve,
            observations,
        )
    }

    pub fn train_with_mode(
        model_id: impl Into<String>,
        model_version: impl Into<String>,
        resolution_mode: WorldSyntaxResolutionModeIR,
        observations: &[WorldSyntaxObservationIR],
    ) -> Result<Self, String> {
        if observations.is_empty()
            || observations.len() > SYNTAX_OBSERVATIONS
            || observations
                .iter()
                .any(|observation| !observation.validate())
        {
            return Err("INVALID_WORLD_SYNTAX_OBSERVATIONS".into());
        }
        let model_id = model_id.into();
        let model_version = model_version.into();
        if model_id.trim().is_empty()
            || model_id.chars().count() > 64
            || model_version.trim().is_empty()
            || model_version.chars().count() > 64
        {
            return Err("INVALID_WORLD_SYNTAX_MODEL_IDENTITY".into());
        }
        let mut order_stats = Vec::<WorldSyntaxOrderStatsIR>::new();
        for observation in observations {
            for (role, particle, position) in [
                (
                    WorldSyntaxRoleIR::Subject,
                    &observation.subject_particle,
                    observation.subject_position,
                ),
                (
                    WorldSyntaxRoleIR::Object,
                    &observation.object_particle,
                    observation.object_position,
                ),
            ] {
                let index = order_stats.iter().position(|stats| {
                    stats.grammar == observation.grammar
                        && stats.role == role
                        && stats.particle == *particle
                        && stats.position == position
                });
                if let Some(index) = index {
                    order_stats[index].support = order_stats[index].support.saturating_add(1);
                } else {
                    order_stats.push(WorldSyntaxOrderStatsIR {
                        grammar: observation.grammar,
                        role,
                        particle: particle.clone(),
                        position,
                        support: 1,
                    });
                }
            }
        }
        order_stats.sort_by(|left, right| {
            (left.grammar, left.role, &left.particle, left.position).cmp(&(
                right.grammar,
                right.role,
                &right.particle,
                right.position,
            ))
        });
        let mut source_refs = observations
            .iter()
            .map(|observation| observation.source_ref.clone())
            .collect::<Vec<_>>();
        source_refs.sort();
        source_refs.dedup();
        let mut source_versions = observations
            .iter()
            .map(|observation| observation.source_version.clone())
            .collect::<Vec<_>>();
        source_versions.sort();
        source_versions.dedup();
        let observation_sha256 = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(observations).expect("syntax observations serialize")
            )
        );
        let model = Self {
            schema: WORLD_SYNTAX_MODEL_SCHEMA.into(),
            model_id,
            model_version,
            training_mode: "SUPPLIED_SUPERVISED_STRUCTURAL_OBSERVATIONS".into(),
            resolution_mode,
            source_refs,
            source_versions,
            observation_count: observations.len(),
            observation_sha256,
            order_stats,
            training_sha256: String::new(),
        };
        let mut model = model;
        model.training_sha256 = model.fingerprint();
        model
            .validate()
            .then_some(model)
            .ok_or_else(|| "INVALID_WORLD_SYNTAX_MODEL".into())
    }

    pub fn validate(&self) -> bool {
        self.schema == WORLD_SYNTAX_MODEL_SCHEMA
            && self.training_mode == "SUPPLIED_SUPERVISED_STRUCTURAL_OBSERVATIONS"
            && !self.model_id.trim().is_empty()
            && self.model_id.chars().count() <= 64
            && !self.model_version.trim().is_empty()
            && self.model_version.chars().count() <= 64
            && self.observation_count > 0
            && self.observation_count <= SYNTAX_OBSERVATIONS
            && !self.source_refs.is_empty()
            && self.source_refs.len() <= self.observation_count
            && self.source_refs.windows(2).all(|pair| pair[0] < pair[1])
            && self
                .source_refs
                .iter()
                .all(|source| !source.trim().is_empty() && source.chars().count() <= 128)
            && !self.source_versions.is_empty()
            && self.source_versions.len() <= self.observation_count
            && self
                .source_versions
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            && self
                .source_versions
                .iter()
                .all(|version| !version.trim().is_empty() && version.chars().count() <= 64)
            && self.observation_sha256.len() == 64
            && self
                .observation_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            && !self.order_stats.is_empty()
            && self.order_stats.len() <= self.observation_count.saturating_mul(2)
            && self.order_stats.windows(2).all(|pair| {
                (
                    pair[0].grammar,
                    pair[0].role,
                    &pair[0].particle,
                    pair[0].position,
                ) < (
                    pair[1].grammar,
                    pair[1].role,
                    &pair[1].particle,
                    pair[1].position,
                )
            })
            && self.order_stats.iter().all(|stats| {
                let valid_particle = match stats.role {
                    WorldSyntaxRoleIR::Subject => {
                        matches!(stats.particle.as_str(), "은" | "는" | "이" | "가")
                    }
                    WorldSyntaxRoleIR::Object => match stats.grammar {
                        WorldLexicalGrammarIR::KoreanHadaLocative => stats.particle == "에",
                        WorldLexicalGrammarIR::KoreanHadaAccusative => {
                            matches!(stats.particle.as_str(), "을" | "를")
                        }
                        _ => false,
                    },
                };
                matches!(
                    stats.grammar,
                    WorldLexicalGrammarIR::KoreanHadaLocative
                        | WorldLexicalGrammarIR::KoreanHadaAccusative
                ) && valid_particle
                    && stats.position <= 1
                    && stats.support > 0
            })
            && self
                .order_stats
                .iter()
                .map(|stats| usize::from(stats.support))
                .sum::<usize>()
                == self.observation_count.saturating_mul(2)
            && self.training_sha256 == self.fingerprint()
    }

    fn fingerprint(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&(
                    WORLD_SYNTAX_MODEL_SCHEMA,
                    &self.model_id,
                    &self.model_version,
                    &self.training_mode,
                    self.resolution_mode,
                    &self.source_refs,
                    &self.source_versions,
                    self.observation_count,
                    &self.observation_sha256,
                    &self.order_stats,
                ))
                .expect("syntax model serialize")
            )
        )
    }

    fn score(
        &self,
        grammar: WorldLexicalGrammarIR,
        subject_particle: &str,
        object_particle: &str,
        subject_position: u8,
        object_position: u8,
    ) -> u32 {
        [
            (
                WorldSyntaxRoleIR::Subject,
                subject_particle,
                subject_position,
            ),
            (WorldSyntaxRoleIR::Object, object_particle, object_position),
        ]
        .into_iter()
        .map(|(role, particle, position)| {
            self.order_stats
                .iter()
                .find(|stats| {
                    stats.grammar == grammar
                        && stats.role == role
                        && stats.particle == particle
                        && stats.position == position
                })
                .map_or(0, |stats| u32::from(stats.support))
        })
        .sum()
    }

    fn preferred_binary_order(
        &self,
        grammar: WorldLexicalGrammarIR,
        subject_particle: &str,
        object_particle: &str,
    ) -> Option<(WorldBinaryOrderIR, u16, u16)> {
        let subject_object = self.score(grammar, subject_particle, object_particle, 0, 1);
        let object_subject = self.score(grammar, subject_particle, object_particle, 1, 0);
        if object_subject > subject_object && object_subject >= 2 {
            Some((
                WorldBinaryOrderIR::ObjectSubject,
                u16::try_from(object_subject).unwrap_or(u16::MAX),
                u16::try_from(subject_object).unwrap_or(u16::MAX),
            ))
        } else if subject_object > object_subject && subject_object >= 2 {
            Some((
                WorldBinaryOrderIR::SubjectObject,
                u16::try_from(subject_object).unwrap_or(u16::MAX),
                u16::try_from(object_subject).unwrap_or(u16::MAX),
            ))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldSyntaxCandidateIR {
    pub schema: String,
    pub model_id: String,
    pub model_version: String,
    pub resolution_mode: WorldSyntaxResolutionModeIR,
    pub source_sha256: String,
    pub predicate_id: String,
    pub alias_id: String,
    pub grammar: WorldLexicalGrammarIR,
    pub order: WorldBinaryOrderIR,
    pub subject_particle: String,
    pub object_particle: String,
    pub subject_span: [usize; 2],
    pub object_span: [usize; 2],
    pub predicate_span: [usize; 2],
    pub support: u16,
    pub competing_support: u16,
    pub semantic_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldPredicateArityIR {
    Unary,
    Binary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldPredicateSpecIR {
    /// Opaque local identity. Never a promoted-concept identifier.
    pub predicate_id: String,
    /// Ordered arguments; no symmetry, transitivity or closed-world default.
    pub arity: WorldPredicateArityIR,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldLexicalGrammarIR {
    Copular,
    KoreanHadaState,
    /// Subjective state with grammatically omitted experiencer defaulting to the speaker.
    KoreanHadaExperiencer,
    EnglishRegularVerb,
    KoreanHadaLocative,
    KoreanHadaAccusative,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldLexemeIR {
    pub alias_id: String,
    pub predicate_id: String,
    pub language: LanguageCodeIR,
    /// Atomic state phrase, English base verb (optionally + preposition), or
    /// Korean nominal 하다 stem. Never an entire answer or solution template.
    pub root: String,
    pub grammar: WorldLexicalGrammarIR,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldVocabularyUpdateIR {
    pub predicates: Vec<WorldPredicateSpecIR>,
    pub aliases: Vec<WorldLexemeIR>,
    pub remove_alias_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldVocabularyIR {
    pub predicates: BTreeMap<String, WorldPredicateSpecIR>,
    /// Append-only via updated(). Old grounding is replayed against its revision.
    pub lexical_history: Vec<BTreeMap<String, WorldLexemeIR>>,
    /// Explicitly injected per-conversation grammar evidence; never global.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub syntax_model: Option<Box<WorldSyntaxModelIR>>,
}

impl Default for WorldVocabularyIR {
    fn default() -> Self {
        Self {
            predicates: BTreeMap::new(),
            lexical_history: vec![BTreeMap::new()],
            syntax_model: None,
        }
    }
}

impl WorldVocabularyIR {
    /// Supplied boolean/lexical primitives, not discoveries or emotion diagnoses.
    pub fn conversational() -> Self {
        let mut update = WorldVocabularyUpdateIR::default();
        for (id, english, korean) in [
            ("W_USER_900001", "tired", "피곤"),
            ("W_USER_900002", "free", "한가"),
            ("W_USER_900003", "frustrated", "답답"),
        ] {
            update.predicates.push(WorldPredicateSpecIR {
                predicate_id: id.into(),
                arity: WorldPredicateArityIR::Unary,
            });
            for (suffix, language, root, grammar) in [
                (
                    "en",
                    LanguageCodeIR::English,
                    english,
                    WorldLexicalGrammarIR::Copular,
                ),
                (
                    "ko",
                    LanguageCodeIR::Korean,
                    korean,
                    WorldLexicalGrammarIR::KoreanHadaExperiencer,
                ),
            ] {
                update.aliases.push(WorldLexemeIR {
                    alias_id: format!("{id}.{suffix}"),
                    predicate_id: id.into(),
                    language,
                    root: root.into(),
                    grammar,
                });
            }
        }
        Self::default()
            .updated(&update)
            .expect("supplied conversational lexemes")
    }
    pub fn revision(&self) -> usize {
        self.lexical_history.len().saturating_sub(1)
    }

    pub fn semantic_sha256(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&self.predicates).expect("predicate serialization"))
        )
    }

    pub fn with_syntax_model(&self, model: WorldSyntaxModelIR) -> Result<Self, String> {
        if !self.validate() || !model.validate() {
            return Err("INVALID_WORLD_SYNTAX_MODEL".into());
        }
        if self
            .syntax_model
            .as_ref()
            .is_some_and(|existing| existing.as_ref() != &model)
        {
            return Err("WORLD_SYNTAX_MODEL_IMMUTABLE".into());
        }
        let mut next = self.clone();
        next.syntax_model = Some(Box::new(model));
        next.validate()
            .then_some(next)
            .ok_or_else(|| "INVALID_WORLD_SYNTAX_MODEL".into())
    }

    pub fn updated(&self, update: &WorldVocabularyUpdateIR) -> Result<Self, String> {
        if !self.validate()
            || update.predicates.len() > ENTRIES
            || update.aliases.len() > ENTRIES
            || update.remove_alias_ids.len() > ENTRIES
        {
            return Err("INVALID_WORLD_VOCABULARY_UPDATE".into());
        }
        let mut next = self.clone();
        for spec in &update.predicates {
            if next
                .predicates
                .get(&spec.predicate_id)
                .is_some_and(|old| old != spec)
            {
                return Err("WORLD_PREDICATE_IMMUTABLE".into());
            }
            next.predicates
                .insert(spec.predicate_id.clone(), spec.clone());
        }
        let mut aliases = self.lexical_history[self.revision()].clone();
        let mut removed = BTreeSet::new();
        for id in &update.remove_alias_ids {
            if !removed.insert(id) || aliases.remove(id).is_none() {
                return Err("UNKNOWN_OR_DUPLICATE_WORLD_ALIAS_REMOVAL".into());
            }
        }
        let mut added = BTreeSet::new();
        for alias in &update.aliases {
            if !added.insert(&alias.alias_id)
                || aliases.get(&alias.alias_id).is_some_and(|old| old != alias)
            {
                return Err("WORLD_ALIAS_IDENTITY_CONFLICT".into());
            }
            aliases.insert(alias.alias_id.clone(), alias.clone());
        }
        if aliases != self.lexical_history[self.revision()] {
            next.lexical_history.push(aliases);
        }
        if !next.validate() {
            return Err("INVALID_OR_AMBIGUOUS_WORLD_VOCABULARY".into());
        }
        Ok(next)
    }

    pub fn validate(&self) -> bool {
        self.predicates.len() <= ENTRIES
            && !self.lexical_history.is_empty()
            && self.lexical_history.len() <= REVISIONS
            && self.predicates.iter().all(|(id, spec)| {
                id == &spec.predicate_id && valid_id(id) && id.starts_with("W_USER_")
            })
            && self
                .syntax_model
                .as_ref()
                .is_none_or(|model| model.validate())
            && self.lexical_history.iter().all(|aliases| {
                let mut surfaces = BTreeSet::new();
                let mut inflected_relations = BTreeSet::new();
                aliases.len() <= ENTRIES
                    && aliases.iter().all(|(id, a)| {
                        id == &a.alias_id && valid_id(id) && self.valid_lexeme(a)
                        // Ambiguous same-language roots fail, even with different grammars.
                        && surfaces.insert((a.language, a.root.clone()))
                        && (a.grammar != WorldLexicalGrammarIR::EnglishRegularVerb
                            || inflected_relations.insert(english_third_person(&a.root)))
                    })
            })
    }

    fn valid_lexeme(&self, a: &WorldLexemeIR) -> bool {
        let Some(spec) = self.predicates.get(&a.predicate_id) else {
            return false;
        };
        if a.root.trim() != a.root
            || a.root.is_empty()
            || a.root.chars().count() > 48
            || a.root.to_lowercase() != a.root
            || a.root.split_whitespace().collect::<Vec<_>>().join(" ") != a.root
            || !a.root.chars().all(|c| c.is_alphabetic() || c == ' ')
            || a.root.split(' ').any(|w| {
                matches!(
                    w,
                    "is" | "not"
                        | "if"
                        | "then"
                        | "and"
                        | "does"
                        | "do"
                        | "why"
                        | "suppose"
                        | "actually"
                        | "아니다"
                        | "그리고"
                        | "왜"
                )
            })
            || WorldPropertyIR::ALL.iter().any(|p| {
                let normalized = copular_root(&a.root).0;
                [a.root.as_str(), normalized]
                    .iter()
                    .any(|root| *root == p.expression(false) || *root == p.expression(true))
            })
        {
            return false;
        }
        use WorldLexicalGrammarIR as G;
        use WorldPredicateArityIR as A;
        match (spec.arity, a.language, a.grammar) {
            (A::Unary, LanguageCodeIR::Korean, G::KoreanHadaState | G::KoreanHadaExperiencer) => {
                a.root.chars().all(|c| ('가'..='힣').contains(&c))
            }
            (A::Unary, LanguageCodeIR::English | LanguageCodeIR::Korean, G::Copular) => {
                a.root.split(' ').count() <= 3
            }
            (A::Binary, LanguageCodeIR::English, G::EnglishRegularVerb) => {
                let words = a.root.split(' ').collect::<Vec<_>>();
                words.len() <= 2
                    && words[0].bytes().all(|b| b.is_ascii_lowercase())
                    && (words.len() == 1
                        || matches!(words[1], "on" | "to" | "with" | "from" | "for"))
            }
            (
                A::Binary,
                LanguageCodeIR::Korean,
                G::KoreanHadaLocative | G::KoreanHadaAccusative,
            ) => a.root.chars().all(|c| ('가'..='힣').contains(&c)),
            _ => false,
        }
    }

    pub fn accepts_atom(&self, atom: &WorldAtomIR) -> bool {
        match &atom.property {
            WorldPropertyIR::Registered(id) => self.predicates.get(id).is_some_and(|s| {
                (s.arity == WorldPredicateArityIR::Binary) == atom.object.is_some()
            }),
            _ => atom.object.is_none(),
        }
    }

    pub fn expression(
        &self,
        property: &WorldPropertyIR,
        language: LanguageCodeIR,
    ) -> Option<&WorldLexemeIR> {
        let WorldPropertyIR::Registered(id) = property else {
            return None;
        };
        self.lexical_history
            .get(self.revision())?
            .values()
            .find(|a| &a.predicate_id == id && a.language == language)
    }

    pub(crate) fn parse_form(
        &self,
        text: &str,
        revision: usize,
        attributive: bool,
        allow_question_forms: bool,
    ) -> Option<(WorldAtomIR, bool)> {
        let aliases = self
            .lexical_history
            .get(revision)?
            .values()
            .collect::<Vec<_>>();
        let deterministic = aliases
            .iter()
            .filter_map(|a| a.parse_form(text, attributive, allow_question_forms))
            .collect::<Vec<_>>();
        if let Some(first) = deterministic.first() {
            return deterministic
                .iter()
                .all(|other| other == first)
                .then_some(first.clone());
        }
        let mut learned = aliases
            .iter()
            .filter_map(|a| self.parse_learned_object_subject(a, text));
        let first = learned.next()?;
        learned.all(|other| other == first).then_some(first)
    }

    fn parse_learned_object_subject(
        &self,
        alias: &WorldLexemeIR,
        text: &str,
    ) -> Option<(WorldAtomIR, bool)> {
        let model = self.syntax_model.as_ref()?;
        if model.resolution_mode != WorldSyntaxResolutionModeIR::Resolve {
            return None;
        }
        let spec = self.predicates.get(&alias.predicate_id)?;
        if alias.language != LanguageCodeIR::Korean
            || spec.arity != WorldPredicateArityIR::Binary
            || !matches!(
                alias.grammar,
                WorldLexicalGrammarIR::KoreanHadaLocative
                    | WorldLexicalGrammarIR::KoreanHadaAccusative
            )
        {
            return None;
        }
        let tokens = text.split_whitespace().collect::<Vec<_>>();
        if tokens.len() < 3 {
            return None;
        }
        let (object, object_particle) = split_korean_object(tokens[0])?;
        let (subject, subject_particle) = split_korean_subject(tokens[1])?;
        let expected_object = match alias.grammar {
            WorldLexicalGrammarIR::KoreanHadaLocative => object_particle == "에",
            WorldLexicalGrammarIR::KoreanHadaAccusative => {
                matches!(object_particle, "을" | "를")
            }
            _ => false,
        };
        if !expected_object || !valid_korean_entity(subject) || !valid_korean_entity(object) {
            return None;
        }
        let predicate = tokens[2..].join(" ");
        let tail = predicate.strip_prefix(&alias.root)?;
        let value = hada_form_polarity(tail, false, false, false)?;
        let (order, _, _) =
            model.preferred_binary_order(alias.grammar, subject_particle, object_particle)?;
        if order != WorldBinaryOrderIR::ObjectSubject {
            return None;
        }
        Some((
            WorldAtomIR {
                entity: subject.into(),
                property: WorldPropertyIR::Registered(alias.predicate_id.clone()),
                object: Some(object.into()),
            },
            value,
        ))
    }

    /// Return the learned candidate metadata for the exact source form. This
    /// is attached to grounding, while the ordinary parser remains the sole
    /// source of the atom committed to world memory.
    pub(crate) fn syntax_candidate(
        &self,
        text: &str,
        revision: usize,
        source_sha256: &str,
    ) -> Option<WorldSyntaxCandidateIR> {
        let model = self.syntax_model.as_ref()?;
        let (body, _) = crate::proposition_content::strip_correction_prefix(text);
        let body_offset = text.len().saturating_sub(body.len());
        let body_leading_bytes = body.len().saturating_sub(body.trim_start().len());
        let leading_scalars = text[..body_offset + body_leading_bytes].chars().count();
        let content = body.trim().trim_end_matches(['.', '?', '!']).trim_end();
        let mut tokens = scalar_token_spans(content);
        for (_, span) in &mut tokens {
            span[0] += leading_scalars;
            span[1] += leading_scalars;
        }
        if tokens.len() < 3 {
            return None;
        }
        let (object_surface, object_particle) = split_korean_object(&tokens[0].0)?;
        let (subject_surface, subject_particle) = split_korean_subject(&tokens[1].0)?;
        let predicate_surface = tokens[2..]
            .iter()
            .map(|(surface, _)| surface.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let aliases = self.lexical_history.get(revision)?;
        let mut candidates = aliases.values().filter_map(|alias| {
            let spec = self.predicates.get(&alias.predicate_id)?;
            if alias.language != LanguageCodeIR::Korean
                || spec.arity != WorldPredicateArityIR::Binary
                || !matches!(
                    alias.grammar,
                    WorldLexicalGrammarIR::KoreanHadaLocative
                        | WorldLexicalGrammarIR::KoreanHadaAccusative
                )
            {
                return None;
            }
            let expected_object = match alias.grammar {
                WorldLexicalGrammarIR::KoreanHadaLocative => object_particle == "에",
                WorldLexicalGrammarIR::KoreanHadaAccusative => {
                    matches!(object_particle, "을" | "를")
                }
                _ => false,
            };
            let (order, support, competing_support) =
                model.preferred_binary_order(alias.grammar, subject_particle, object_particle)?;
            if !expected_object
                || !valid_korean_entity(subject_surface)
                || !valid_korean_entity(object_surface)
                || predicate_surface
                    .strip_prefix(&alias.root)
                    .and_then(|tail| hada_form_polarity(tail, false, false, false))
                    .is_none()
            {
                return None;
            }
            Some(WorldSyntaxCandidateIR {
                schema: WORLD_SYNTAX_MODEL_SCHEMA.into(),
                model_id: model.model_id.clone(),
                model_version: model.model_version.clone(),
                resolution_mode: model.resolution_mode,
                source_sha256: source_sha256.into(),
                predicate_id: alias.predicate_id.clone(),
                alias_id: alias.alias_id.clone(),
                grammar: alias.grammar,
                order,
                subject_particle: subject_particle.into(),
                object_particle: object_particle.into(),
                subject_span: tokens[1].1,
                object_span: tokens[0].1,
                predicate_span: [tokens[2].1[0], tokens.last()?.1[1]],
                support,
                competing_support,
                semantic_authority: false,
            })
        });
        let first = candidates.next()?;
        candidates
            .all(|other| other.predicate_id == first.predicate_id)
            .then_some(first)
    }

    /// Recognize only a registered unary HADA question surface. This is kept
    /// on the vocabulary boundary so punctuationless questions do not become
    /// a global sentence-level question heuristic, and binary HADA relations
    /// cannot borrow unary state morphology.
    pub(crate) fn has_unary_hada_question_form(&self, text: &str, revision: usize) -> bool {
        self.lexical_history.get(revision).is_some_and(|aliases| {
            aliases.values().any(|alias| {
                self.predicates
                    .get(&alias.predicate_id)
                    .is_some_and(|spec| spec.arity == WorldPredicateArityIR::Unary)
                    && alias.hada_question_polarity(text).is_some()
            })
        })
    }
}

fn valid_korean_entity(entity: &str) -> bool {
    !entity.is_empty()
        && entity.chars().count() <= 48
        && entity
            .chars()
            .all(|character| ('가'..='힣').contains(&character))
}

fn scalar_token_spans(text: &str) -> Vec<(String, [usize; 2])> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut start = None;
    for (index, character) in chars.iter().enumerate() {
        if character.is_whitespace() {
            if let Some(begin) = start.take() {
                tokens.push((chars[begin..index].iter().collect(), [begin, index]));
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(begin) = start {
        tokens.push((chars[begin..].iter().collect(), [begin, chars.len()]));
    }
    tokens
}

fn split_korean_subject(token: &str) -> Option<(&str, &str)> {
    ["은", "는", "이", "가"]
        .into_iter()
        .find_map(|particle| token.strip_suffix(particle).map(|stem| (stem, particle)))
        .filter(|(stem, _)| !stem.is_empty())
}

fn split_korean_object(token: &str) -> Option<(&str, &str)> {
    ["에", "을", "를"]
        .into_iter()
        .find_map(|particle| token.strip_suffix(particle).map(|stem| (stem, particle)))
        .filter(|(stem, _)| !stem.is_empty())
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
}

pub(crate) fn english_third_person(root: &str) -> String {
    let (verb, tail) = root.split_once(' ').map_or((root, ""), |(v, t)| (v, t));
    let inflected = if verb.ends_with('y')
        && verb.len() > 1
        && !matches!(
            verb.as_bytes()[verb.len() - 2],
            b'a' | b'e' | b'i' | b'o' | b'u'
        ) {
        format!("{}ies", &verb[..verb.len() - 1])
    } else if ["s", "x", "z", "ch", "sh", "o"]
        .iter()
        .any(|s| verb.ends_with(s))
    {
        format!("{verb}es")
    } else {
        format!("{verb}s")
    };
    if tail.is_empty() {
        inflected
    } else {
        format!("{inflected} {tail}")
    }
}

impl WorldLexemeIR {
    fn parse_form(
        &self,
        text: &str,
        attributive: bool,
        allow_question_forms: bool,
    ) -> Option<(WorldAtomIR, bool)> {
        use WorldLexicalGrammarIR as G;
        if attributive && self.language != LanguageCodeIR::Korean {
            return None;
        }
        if matches!(self.grammar, G::KoreanHadaState | G::KoreanHadaExperiencer) {
            let (subject, predicate) = korean_state_parts(text)?;
            let (predicate, pre_negated) = predicate
                .strip_prefix("안 ")
                .map_or((predicate, false), |p| (p, true));
            let value = hada_form_polarity(
                predicate.strip_prefix(&self.root)?,
                attributive,
                true,
                allow_question_forms,
            )?;
            if pre_negated && !value {
                return None;
            }
            return Some((
                WorldAtomIR {
                    entity: subject.into(),
                    property: WorldPropertyIR::Registered(self.predicate_id.clone()),
                    object: None,
                },
                value != pre_negated,
            ));
        }
        let (subject, object, value) = match self.grammar {
            G::KoreanHadaState | G::KoreanHadaExperiencer => unreachable!("handled above"),
            G::Copular => {
                let (subject, predicate) = copular_parts(text)?;
                let predicate = strip_state_modifiers(predicate, false);
                let (root, value) = if attributive {
                    copular_attributive_root(predicate)?
                } else if predicate == self.root {
                    (predicate, true)
                } else {
                    copular_root(predicate)
                };
                if root != self.root {
                    return None;
                }
                (subject, None, value)
            }
            G::EnglishRegularVerb => {
                let (subject, rest, base) = if let Some(body) = text
                    .strip_prefix("does ")
                    .or_else(|| text.strip_prefix("do "))
                {
                    let (s, r) = body.split_once(' ')?;
                    (s, r, true)
                } else {
                    let (s, r) = text.split_once(' ')?;
                    (s, r, s == "i")
                };
                let (rest, value, base) = if let Some(r) = rest
                    .strip_prefix("does not ")
                    .or_else(|| rest.strip_prefix("do not "))
                {
                    (r, false, true)
                } else if base {
                    rest.strip_prefix("not ")
                        .map_or((rest, true, true), |r| (r, false, true))
                } else {
                    (rest, true, false)
                };
                let root = if base {
                    self.root.clone()
                } else {
                    english_third_person(&self.root)
                };
                let object = rest.strip_prefix(&format!("{root} "))?;
                (subject, Some(object), value)
            }
            G::KoreanHadaLocative | G::KoreanHadaAccusative => {
                let (subject, rest) = text.split_once(' ')?;
                let subject = korean_subject(subject)?;
                let (object, predicate) = rest.split_once(' ')?;
                let object = if self.grammar == G::KoreanHadaLocative {
                    object.strip_suffix('에')?
                } else {
                    object.strip_suffix(['을', '를'])?
                };
                let tail = predicate.strip_prefix(&self.root)?;
                let value = hada_form_polarity(tail, attributive, false, false)?;
                (subject, Some(object), value)
            }
        };
        Some((
            WorldAtomIR {
                entity: subject.into(),
                property: WorldPropertyIR::Registered(self.predicate_id.clone()),
                object: object.map(str::to_string),
            },
            value,
        ))
    }

    fn hada_question_polarity(&self, text: &str) -> Option<bool> {
        if self.language != LanguageCodeIR::Korean
            || !matches!(
                self.grammar,
                WorldLexicalGrammarIR::KoreanHadaState
                    | WorldLexicalGrammarIR::KoreanHadaExperiencer
            )
        {
            return None;
        }
        let (_, predicate) = korean_state_parts(text)?;
        let predicate = predicate.strip_prefix("안 ").unwrap_or(predicate);
        hada_question_polarity(predicate.strip_prefix(&self.root)?)
    }
}

pub(crate) fn copular_parts(text: &str) -> Option<(&str, &str)> {
    if let Some(body) = text.strip_prefix("is ") {
        body.split_once(' ')
    } else if let Some(pair) = text.split_once(" is ") {
        Some(pair)
    } else if let Some(pair) = text.split_once(" am ") {
        (pair.0 == "i").then_some(pair)
    } else if let Some(pair) = text.split_once(" feel ") {
        (pair.0 == "i").then_some(pair)
    } else {
        let (s, p) = text.split_once(' ')?;
        Some((korean_subject(s)?, p))
    }
}

pub(crate) fn copular_root(predicate: &str) -> (&str, bool) {
    if let Some(body) = predicate.strip_prefix("not ") {
        return (body, false);
    }
    if let Some(analysis) = crate::korean_copula::analyze(predicate) {
        return (analysis.root, analysis.polarity);
    }
    (predicate, true)
}

fn hada_question_polarity(tail: &str) -> Option<bool> {
    crate::korean_hada::analyze_state_question_tail(tail).map(|analysis| analysis.polarity)
}

fn hada_form_polarity(
    tail: &str,
    attributive: bool,
    state: bool,
    allow_question_forms: bool,
) -> Option<bool> {
    if !attributive {
        return crate::korean_hada::analyze_finite_tail(tail, allow_question_forms)
            .map(|analysis| analysis.polarity);
    }
    crate::korean_hada::analyze_attributive_tail(tail, state).map(|analysis| analysis.polarity)
}

pub(crate) fn copular_attributive_root(predicate: &str) -> Option<(&str, bool)> {
    crate::korean_copula::analyze_attributive(predicate)
        .map(|analysis| (analysis.root, analysis.polarity))
}

/// Consume only current-scope modifiers. Historical, hypothetical, quoted and
/// unknown material stays unconsumed and therefore cannot become a current fact.
fn strip_state_modifiers(mut text: &str, korean: bool) -> &str {
    for _ in 0..8 {
        let Some((word, rest)) = text.split_once(' ') else {
            break;
        };
        let modifier = if korean {
            matches!(
                word,
                "오늘"
                    | "지금"
                    | "현재"
                    | "진짜"
                    | "정말"
                    | "매우"
                    | "아주"
                    | "너무"
                    | "좀"
                    | "조금"
                    | "꽤"
                    | "약간"
            )
        } else {
            matches!(word, "really" | "very" | "quite" | "so" | "currently")
        };
        if !modifier {
            break;
        }
        text = rest;
    }
    text
}

fn korean_state_parts(text: &str) -> Option<(&str, &str)> {
    let text = strip_state_modifiers(text, true);
    if let Some((first, rest)) = text.split_once(' ') {
        let subject = if matches!(first, "나" | "저") {
            Some(first)
        } else {
            korean_subject(first)
        };
        if let Some(subject) = subject.filter(|s| !s.is_empty()) {
            return Some((subject, strip_state_modifiers(rest, true)));
        }
    }
    None
}

fn korean_subject(surface: &str) -> Option<&str> {
    // 누구 + subject case has the contracted surface 누가. Preserve its open
    // argument identity before the world boundary rejects unbound variables.
    if surface == "누가" {
        Some("누구")
    } else {
        surface.strip_suffix(['은', '는', '이', '가'])
    }
}
