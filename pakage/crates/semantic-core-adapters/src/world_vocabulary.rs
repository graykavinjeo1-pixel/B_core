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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldLexicalGrammarIR {
    Copular,
    KoreanHadaState,
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
}

impl Default for WorldVocabularyIR {
    fn default() -> Self {
        Self {
            predicates: BTreeMap::new(),
            lexical_history: vec![BTreeMap::new()],
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
                    WorldLexicalGrammarIR::KoreanHadaState,
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
            (A::Unary, LanguageCodeIR::Korean, G::KoreanHadaState) => {
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

    pub(crate) fn parse(&self, text: &str, revision: usize) -> Option<(WorldAtomIR, bool)> {
        let mut matches = self
            .lexical_history
            .get(revision)?
            .values()
            .filter_map(|a| a.parse(text));
        let first = matches.next()?;
        matches.all(|other| other == first).then_some(first)
    }
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
    fn parse(&self, text: &str) -> Option<(WorldAtomIR, bool)> {
        use WorldLexicalGrammarIR as G;
        let (subject, object, value) = match self.grammar {
            G::KoreanHadaState => {
                let (subject, predicate) = text.split_once(' ')?;
                let subject = subject
                    .strip_suffix(['은', '는', '이', '가'])
                    .or_else(|| matches!(subject, "나" | "저").then_some(subject))?;
                let tail = predicate.strip_prefix(&self.root)?;
                (subject, None, hada_polarity(tail)?)
            }
            G::Copular => {
                let (subject, predicate) = copular_parts(text)?;
                let (root, value) = if predicate == self.root {
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
                let subject = subject.strip_suffix(['은', '는', '이', '가'])?;
                let (object, predicate) = rest.split_once(' ')?;
                let object = if self.grammar == G::KoreanHadaLocative {
                    object.strip_suffix('에')?
                } else {
                    object.strip_suffix(['을', '를'])?
                };
                let tail = predicate.strip_prefix(&self.root)?;
                let value = hada_polarity(tail)?;
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
}

pub(crate) fn copular_parts(text: &str) -> Option<(&str, &str)> {
    if let Some(body) = text.strip_prefix("is ") {
        body.split_once(' ')
    } else if let Some(pair) = text.split_once(" is ") {
        Some(pair)
    } else if let Some(pair) = text.split_once(" am ") {
        (pair.0 == "i").then_some(pair)
    } else {
        let (s, p) = text.split_once(' ')?;
        Some((s.strip_suffix(['은', '는', '이', '가'])?, p))
    }
}

pub(crate) fn copular_root(predicate: &str) -> (&str, bool) {
    if let Some(body) = predicate.strip_prefix("not ") {
        return (body, false);
    }
    for ending in ["가 아니다", "가 아니야", "이 아니다", "이 아니야"] {
        if let Some(body) = predicate.strip_suffix(ending) {
            return (body, false);
        }
    }
    let root = ["인가요", "인가", "이다", "이야", "다", "야"]
        .iter()
        .find_map(|ending| predicate.strip_suffix(ending))
        .unwrap_or(predicate);
    (root, true)
}

fn hada_polarity(tail: &str) -> Option<bool> {
    match tail {
        "하다" | "한다" | "해" | "하나요" | "하는가" | "하면" | "하고" | "해요" | "합니다" => {
            Some(true)
        }
        "하지 않는다" | "하지 않아" | "하지 않나요" | "하지 않으면" | "하지 않고"
        | "하지 않아요" => Some(false),
        _ => None,
    }
}
