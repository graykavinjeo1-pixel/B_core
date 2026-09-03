//! Small executable ontology for discourse reference matching.
//!
//! The ontology is adapter-local. Its lexical forms can select a typed
//! relation path, but they never create facts, authorize actions, or mutate a
//! promoted semantic payload. A reference resolves only when exactly one
//! bounded discourse candidate satisfies the requested concept and role path.

use std::collections::BTreeMap;

use crate::conversation::{DiscourseReferentKindIR, DynamicDiscourseReferentIR};
use crate::typed_coreference::{
    TypedEntityKindIR, TypedEntityReferentIR, TypedMentionRoleIR, MAX_TYPED_ENTITY_REFERENTS,
    MAX_TYPED_REFERENCE_TURN_DISTANCE,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OntologyBindingKind {
    Entity,
    Event,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyReferenceResolution {
    pub resolved_text: String,
    pub source_surface: Option<String>,
    pub resolved_surface: Option<String>,
    pub referent_ids: Vec<String>,
    pub binding_kind: Option<OntologyBindingKind>,
    pub confidence_millis: u16,
    pub evidence: Vec<String>,
    pub ambiguous_surfaces: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct EntityLexeme {
    form: &'static str,
    direct_concept: &'static str,
    parent: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
struct EventLexeme {
    form: &'static str,
    concept: &'static str,
}

// Forms are lexical adapter data. Concept IDs and parent edges, not the forms,
// are the matching substrate after grounding.
const ENTITY_LEXEMES: &[EntityLexeme] = &[
    EntityLexeme {
        form: "storage layer",
        direct_concept: "E_CACHE",
        parent: Some("E_SOFTWARE_COMPONENT"),
    },
    EntityLexeme {
        form: "저장 계층",
        direct_concept: "E_CACHE",
        parent: Some("E_SOFTWARE_COMPONENT"),
    },
    EntityLexeme {
        form: "application",
        direct_concept: "E_SOFTWARE_APPLICATION",
        parent: Some("E_SOFTWARE_SYSTEM"),
    },
    EntityLexeme {
        form: "애플리케이션",
        direct_concept: "E_SOFTWARE_APPLICATION",
        parent: Some("E_SOFTWARE_SYSTEM"),
    },
    EntityLexeme {
        form: "repository",
        direct_concept: "E_REPOSITORY",
        parent: Some("E_SOFTWARE_ARTIFACT"),
    },
    EntityLexeme {
        form: "codebase",
        direct_concept: "E_REPOSITORY",
        parent: Some("E_SOFTWARE_ARTIFACT"),
    },
    EntityLexeme {
        form: "코드베이스",
        direct_concept: "E_REPOSITORY",
        parent: Some("E_SOFTWARE_ARTIFACT"),
    },
    EntityLexeme {
        form: "저장소",
        direct_concept: "E_REPOSITORY",
        parent: Some("E_SOFTWARE_ARTIFACT"),
    },
    EntityLexeme {
        form: "directory",
        direct_concept: "E_DIRECTORY",
        parent: Some("E_SOFTWARE_ARTIFACT"),
    },
    EntityLexeme {
        form: "디렉터리",
        direct_concept: "E_DIRECTORY",
        parent: Some("E_SOFTWARE_ARTIFACT"),
    },
    EntityLexeme {
        form: "folder",
        direct_concept: "E_DIRECTORY",
        parent: Some("E_SOFTWARE_ARTIFACT"),
    },
    EntityLexeme {
        form: "폴더",
        direct_concept: "E_DIRECTORY",
        parent: Some("E_SOFTWARE_ARTIFACT"),
    },
    EntityLexeme {
        form: "service",
        direct_concept: "E_SOFTWARE_APPLICATION",
        parent: Some("E_SOFTWARE_SYSTEM"),
    },
    EntityLexeme {
        form: "서비스",
        direct_concept: "E_SOFTWARE_APPLICATION",
        parent: Some("E_SOFTWARE_SYSTEM"),
    },
    EntityLexeme {
        form: "backend",
        direct_concept: "E_SOFTWARE_APPLICATION",
        parent: Some("E_SOFTWARE_SYSTEM"),
    },
    EntityLexeme {
        form: "백엔드",
        direct_concept: "E_SOFTWARE_APPLICATION",
        parent: Some("E_SOFTWARE_SYSTEM"),
    },
    EntityLexeme {
        form: "daemon",
        direct_concept: "E_SOFTWARE_APPLICATION",
        parent: Some("E_SOFTWARE_SYSTEM"),
    },
    EntityLexeme {
        form: "데몬",
        direct_concept: "E_SOFTWARE_APPLICATION",
        parent: Some("E_SOFTWARE_SYSTEM"),
    },
    EntityLexeme {
        form: "app",
        direct_concept: "E_SOFTWARE_APPLICATION",
        parent: Some("E_SOFTWARE_SYSTEM"),
    },
    EntityLexeme {
        form: "앱",
        direct_concept: "E_SOFTWARE_APPLICATION",
        parent: Some("E_SOFTWARE_SYSTEM"),
    },
    EntityLexeme {
        form: "report",
        direct_concept: "E_REPORT",
        parent: Some("E_DOCUMENT"),
    },
    EntityLexeme {
        form: "보고서",
        direct_concept: "E_REPORT",
        parent: Some("E_DOCUMENT"),
    },
    EntityLexeme {
        form: "manual",
        direct_concept: "E_MANUAL",
        parent: Some("E_DOCUMENT"),
    },
    EntityLexeme {
        form: "매뉴얼",
        direct_concept: "E_MANUAL",
        parent: Some("E_DOCUMENT"),
    },
    EntityLexeme {
        form: "document",
        direct_concept: "E_DOCUMENT",
        parent: Some("E_SOFTWARE_ARTIFACT"),
    },
    EntityLexeme {
        form: "문서",
        direct_concept: "E_DOCUMENT",
        parent: Some("E_SOFTWARE_ARTIFACT"),
    },
    EntityLexeme {
        form: "cache",
        direct_concept: "E_CACHE",
        parent: Some("E_SOFTWARE_COMPONENT"),
    },
    EntityLexeme {
        form: "캐시",
        direct_concept: "E_CACHE",
        parent: Some("E_SOFTWARE_COMPONENT"),
    },
    EntityLexeme {
        form: "parser",
        direct_concept: "E_PARSER",
        parent: Some("E_SOFTWARE_COMPONENT"),
    },
    EntityLexeme {
        form: "파서",
        direct_concept: "E_PARSER",
        parent: Some("E_SOFTWARE_COMPONENT"),
    },
    EntityLexeme {
        form: "server",
        direct_concept: "E_SERVER",
        parent: Some("E_SOFTWARE_SYSTEM"),
    },
    EntityLexeme {
        form: "서버",
        direct_concept: "E_SERVER",
        parent: Some("E_SOFTWARE_SYSTEM"),
    },
    EntityLexeme {
        form: "archive",
        direct_concept: "E_ARCHIVE",
        parent: Some("E_SOFTWARE_ARTIFACT"),
    },
    EntityLexeme {
        form: "보관함",
        direct_concept: "E_ARCHIVE",
        parent: Some("E_SOFTWARE_ARTIFACT"),
    },
    EntityLexeme {
        form: "file",
        direct_concept: "E_FILE",
        parent: Some("E_SOFTWARE_ARTIFACT"),
    },
    EntityLexeme {
        form: "파일",
        direct_concept: "E_FILE",
        parent: Some("E_SOFTWARE_ARTIFACT"),
    },
];

const EVENT_LEXEMES: &[EventLexeme] = &[
    EventLexeme {
        form: "drafting",
        concept: "A_CREATE",
    },
    EventLexeme {
        form: "authorship",
        concept: "A_CREATE",
    },
    EventLexeme {
        form: "초안 작성",
        concept: "A_CREATE",
    },
    EventLexeme {
        form: "deployment",
        concept: "A_DEPLOY",
    },
    EventLexeme {
        form: "rollout",
        concept: "A_DEPLOY",
    },
    EventLexeme {
        form: "release",
        concept: "A_DEPLOY",
    },
    EventLexeme {
        form: "deployed",
        concept: "A_DEPLOY",
    },
    EventLexeme {
        form: "deploy",
        concept: "A_DEPLOY",
    },
    EventLexeme {
        form: "배포",
        concept: "A_DEPLOY",
    },
    EventLexeme {
        form: "출시",
        concept: "A_DEPLOY",
    },
    EventLexeme {
        form: "transfer",
        concept: "A_TRANSFER",
    },
    EventLexeme {
        form: "relocation",
        concept: "A_TRANSFER",
    },
    EventLexeme {
        form: "moved",
        concept: "A_TRANSFER",
    },
    EventLexeme {
        form: "move",
        concept: "A_TRANSFER",
    },
    EventLexeme {
        form: "이동",
        concept: "A_TRANSFER",
    },
    EventLexeme {
        form: "옮김",
        concept: "A_TRANSFER",
    },
    EventLexeme {
        form: "옮겨",
        concept: "A_TRANSFER",
    },
    EventLexeme {
        form: "examination",
        concept: "A_INVESTIGATE",
    },
    EventLexeme {
        form: "inspection",
        concept: "A_INVESTIGATE",
    },
    EventLexeme {
        form: "reviewed",
        concept: "A_INVESTIGATE",
    },
    EventLexeme {
        form: "review",
        concept: "A_INVESTIGATE",
    },
    EventLexeme {
        form: "inspect",
        concept: "A_INVESTIGATE",
    },
    EventLexeme {
        form: "analysis",
        concept: "A_INVESTIGATE",
    },
    EventLexeme {
        form: "검토",
        concept: "A_INVESTIGATE",
    },
    EventLexeme {
        form: "확인",
        concept: "A_INVESTIGATE",
    },
    EventLexeme {
        form: "점검",
        concept: "A_INVESTIGATE",
    },
    EventLexeme {
        form: "분석",
        concept: "A_INVESTIGATE",
    },
    EventLexeme {
        form: "repaired",
        concept: "A_REPAIR",
    },
    EventLexeme {
        form: "repair",
        concept: "A_REPAIR",
    },
    EventLexeme {
        form: "fix",
        concept: "A_REPAIR",
    },
    EventLexeme {
        form: "correction",
        concept: "A_REPAIR",
    },
    EventLexeme {
        form: "수리",
        concept: "A_REPAIR",
    },
    EventLexeme {
        form: "수정",
        concept: "A_REPAIR",
    },
    EventLexeme {
        form: "복구",
        concept: "A_REPAIR",
    },
    EventLexeme {
        form: "create",
        concept: "A_CREATE",
    },
    EventLexeme {
        form: "draft",
        concept: "A_CREATE",
    },
    EventLexeme {
        form: "creation",
        concept: "A_CREATE",
    },
    EventLexeme {
        form: "작성",
        concept: "A_CREATE",
    },
    EventLexeme {
        form: "생성",
        concept: "A_CREATE",
    },
    EventLexeme {
        form: "delete",
        concept: "A_DELETE",
    },
    EventLexeme {
        form: "deletion",
        concept: "A_DELETE",
    },
    EventLexeme {
        form: "removal",
        concept: "A_DELETE",
    },
    EventLexeme {
        form: "삭제",
        concept: "A_DELETE",
    },
    EventLexeme {
        form: "제거",
        concept: "A_DELETE",
    },
];

#[derive(Debug)]
struct EntityReference {
    marker: String,
    query_concept: String,
}

#[derive(Debug)]
struct EventReference {
    marker: String,
    action_concept: String,
    role_concepts: BTreeMap<String, String>,
}

pub fn merge_ontology_mentions(referents: &mut Vec<TypedEntityReferentIR>, turn: u64, text: &str) {
    let lower = text.to_lowercase();
    for lexeme in ENTITY_LEXEMES {
        // Single-token entity mentions already come from the semantic-role
        // graph. This supplemental path only closes multiword-span gaps and
        // therefore cannot duplicate or reinterpret the normal mention path.
        if !lexeme.form.contains(' ') {
            continue;
        }
        let Some(position) = find_form(&lower, lexeme.form) else {
            continue;
        };
        if marker_is_quoted(text, lexeme.form) {
            continue;
        }
        let prefix = text[..position].trim_end();
        let Some(previous) = prefix.split_whitespace().next_back() else {
            continue;
        };
        let previous = previous.trim_matches(|character: char| !character.is_alphanumeric());
        if previous.is_empty()
            || matches!(
                previous.to_lowercase().as_str(),
                "a" | "an"
                    | "the"
                    | "this"
                    | "that"
                    | "some"
                    | "any"
                    | "그"
                    | "이"
                    | "저"
                    | "inspect"
                    | "review"
                    | "check"
                    | "analyze"
                    | "repair"
                    | "fix"
                    | "deploy"
                    | "move"
                    | "create"
                    | "delete"
            )
        {
            continue;
        }
        let end = position + lexeme.form.len();
        let surface = format!("{previous} {}", &text[position..end]);
        let normalized = surface.to_lowercase();
        if let Some(existing) = referents
            .iter_mut()
            .find(|referent| referent.normalized_label == normalized)
        {
            if existing.last_mentioned_turn != turn {
                existing.last_mentioned_turn = turn;
                existing.mention_count = existing.mention_count.saturating_add(1);
            }
            continue;
        }
        let suffix = referents
            .iter()
            .filter(|referent| referent.introduced_turn == turn)
            .count()
            + 1;
        referents.push(TypedEntityReferentIR {
            entity_id: format!("TREF-{turn:06}-{suffix:02}"),
            canonical_surface: surface,
            normalized_label: normalized,
            kind: ontology_entity_kind(lexeme.direct_concept),
            mention_roles: vec![TypedMentionRoleIR::Other],
            introduced_turn: turn,
            last_mentioned_turn: turn,
            mention_count: 1,
            semantic_authority: false,
        });
    }
    referents.sort_by(|left, right| {
        right
            .last_mentioned_turn
            .cmp(&left.last_mentioned_turn)
            .then_with(|| right.mention_count.cmp(&left.mention_count))
            .then_with(|| left.entity_id.cmp(&right.entity_id))
    });
    referents.truncate(MAX_TYPED_ENTITY_REFERENTS);
}

fn ontology_entity_kind(concept: &str) -> TypedEntityKindIR {
    match concept {
        "E_SOFTWARE_APPLICATION" | "E_SERVER" | "E_PARSER" => TypedEntityKindIR::System,
        "E_REPOSITORY" | "E_DIRECTORY" | "E_REPORT" | "E_MANUAL" | "E_DOCUMENT" | "E_CACHE"
        | "E_ARCHIVE" | "E_FILE" => TypedEntityKindIR::Artifact,
        _ => TypedEntityKindIR::Unknown,
    }
}

pub fn resolve_ontology_entity_reference(
    referents: &[TypedEntityReferentIR],
    completed_turns: u64,
    text: &str,
) -> OntologyReferenceResolution {
    // A longer event nominal such as "that parser fix" owns the complete
    // span. Let the event resolver handle it instead of consuming only the
    // embedded entity phrase.
    if event_reference(text).is_some() {
        return unchanged(text);
    }
    let Some(reference) = entity_reference(text) else {
        return unchanged(text);
    };
    if marker_is_quoted(text, &reference.marker) {
        return unchanged(text);
    }
    let mut candidates = referents
        .iter()
        .filter(|referent| {
            completed_turns.saturating_sub(referent.last_mentioned_turn)
                <= MAX_TYPED_REFERENCE_TURN_DISTANCE
        })
        .filter_map(|referent| {
            let concepts = entity_concepts(&referent.canonical_surface);
            concepts
                .get(&reference.query_concept)
                .cloned()
                .map(|path| (referent, path))
        })
        .collect::<Vec<_>>();
    if candidates.len() > 1 {
        let individuated = candidates
            .iter()
            .filter(|(referent, _)| has_individuating_entity_mention(&referent.canonical_surface))
            .cloned()
            .collect::<Vec<_>>();
        if individuated.len() == 1 {
            candidates = individuated;
        }
    }
    if candidates.len() != 1 {
        return ambiguous(text, &reference.marker, "ENTITY_ONTOLOGY_REFERENCE");
    }
    let (referent, path) = &candidates[0];
    let (source_surface, replacement) =
        case_adjusted_replacement(text, &reference.marker, &referent.canonical_surface);
    OntologyReferenceResolution {
        resolved_text: replace_first_case_insensitive(text, &source_surface, &replacement),
        source_surface: Some(source_surface),
        resolved_surface: Some(replacement),
        referent_ids: vec![referent.entity_id.clone()],
        binding_kind: Some(OntologyBindingKind::Entity),
        confidence_millis: if path.contains("->") { 900 } else { 940 },
        evidence: vec![
            format!("ONTOLOGY_PATH:{path}"),
            format!("ONTOLOGY_QUERY:{}", reference.query_concept),
            "SEMANTIC_AUTHORITY:false".to_string(),
        ],
        ambiguous_surfaces: Vec::new(),
    }
}

pub fn resolve_ontology_event_reference(
    referents: &[DynamicDiscourseReferentIR],
    completed_turns: u64,
    text: &str,
) -> OntologyReferenceResolution {
    let Some(reference) = event_reference(text) else {
        return unchanged(text);
    };
    if marker_is_quoted(text, &reference.marker) {
        return unchanged(text);
    }
    let mut candidates = referents
        .iter()
        .filter(|referent| {
            referent.kind == DiscourseReferentKindIR::Event
                && completed_turns.saturating_sub(referent.last_referenced_turn)
                    <= MAX_TYPED_REFERENCE_TURN_DISTANCE
        })
        .filter_map(|referent| {
            let action = event_concept(&referent.semantic_summary)?;
            if action != reference.action_concept {
                return None;
            }
            let role_concepts = entity_concepts(&referent.semantic_summary);
            let role_paths = reference
                .role_concepts
                .keys()
                .map(|concept| role_concepts.get(concept).cloned())
                .collect::<Option<Vec<_>>>()?;
            Some((referent, role_paths))
        })
        .collect::<Vec<_>>();
    if candidates.len() > 1 {
        let individuated = candidates
            .iter()
            .filter(|(referent, _)| has_individuating_entity_mention(&referent.semantic_summary))
            .cloned()
            .collect::<Vec<_>>();
        if individuated.len() == 1 {
            candidates = individuated;
        }
    }
    if candidates.len() != 1 {
        return ambiguous(text, &reference.marker, "EVENT_ONTOLOGY_REFERENCE");
    }
    let (referent, role_paths) = &candidates[0];
    let summary = referent
        .semantic_summary
        .trim_matches(|character| matches!(character, '‘' | '’' | '“' | '”' | '"' | '\''));
    let replacement_base = if text_is_english(text) {
        format!("the action ‘{summary}’")
    } else {
        format!("‘{summary}’라는 작업")
    };
    let (source_surface, replacement) =
        case_adjusted_replacement(text, &reference.marker, &replacement_base);
    let mut evidence = vec![
        format!("ONTOLOGY_PATH:{}", reference.action_concept),
        "SEMANTIC_AUTHORITY:false".to_string(),
    ];
    evidence.extend(
        role_paths
            .iter()
            .map(|path| format!("ONTOLOGY_ROLE:{path}")),
    );
    OntologyReferenceResolution {
        resolved_text: replace_first_case_insensitive(text, &source_surface, &replacement),
        source_surface: Some(source_surface),
        resolved_surface: Some(replacement),
        referent_ids: vec![referent.referent_id.clone()],
        binding_kind: Some(OntologyBindingKind::Event),
        confidence_millis: if role_paths.is_empty() { 920 } else { 960 },
        evidence,
        ambiguous_surfaces: Vec::new(),
    }
}

fn entity_reference(text: &str) -> Option<EntityReference> {
    let lower = text.to_lowercase();
    let determiner = if text_is_english(text) {
        "that "
    } else {
        "그 "
    };
    let start = lower.find(determiner)?;
    if text_is_english(text) && english_that_introduces_clause(&lower, start) {
        return None;
    }
    let content_start = start + determiner.len();
    let tail = &lower[content_start..];
    let lexeme = ENTITY_LEXEMES
        .iter()
        .filter(|lexeme| form_at_start(tail, lexeme.form))
        .max_by_key(|lexeme| lexeme.form.chars().count())?;
    let marker_end = content_start + lexeme.form.len();
    Some(EntityReference {
        marker: text.get(start..marker_end)?.to_string(),
        query_concept: lexeme.direct_concept.to_string(),
    })
}

fn event_reference(text: &str) -> Option<EventReference> {
    let lower = text.to_lowercase();
    let determiner = if text_is_english(text) {
        "that "
    } else {
        "그 "
    };
    let start = lower.find(determiner)?;
    if text_is_english(text) && english_that_introduces_clause(&lower, start) {
        return None;
    }
    let content_start = start + determiner.len();
    let tail = &lower[content_start..];
    let (event_start, lexeme) = EVENT_LEXEMES
        .iter()
        .filter_map(|lexeme| find_form(tail, lexeme.form).map(|position| (position, lexeme)))
        .min_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| right.1.form.len().cmp(&left.1.form.len()))
        })?;
    if !text_is_english(text) {
        let after_event = &tail[event_start + lexeme.form.len()..];
        if !after_event.starts_with('을') && !after_event.starts_with('를') {
            return None;
        }
    }
    let role_surface = tail[..event_start].trim();
    if role_surface
        .chars()
        .any(|character| matches!(character, '.' | '?' | '!' | ';'))
    {
        return None;
    }
    let role_concepts = entity_concepts(role_surface);
    let marker_end = content_start + event_start + lexeme.form.len();
    Some(EventReference {
        marker: text.get(start..marker_end)?.to_string(),
        action_concept: lexeme.concept.to_string(),
        role_concepts,
    })
}

fn english_that_introduces_clause(lower: &str, that_start: usize) -> bool {
    lower[..that_start]
        .split_whitespace()
        .next_back()
        .map(|word| word.trim_matches(|character: char| !character.is_ascii_alphabetic()))
        .is_some_and(|word| {
            matches!(
                word,
                "say"
                    | "says"
                    | "said"
                    | "report"
                    | "reports"
                    | "reported"
                    | "believe"
                    | "believes"
                    | "believed"
                    | "think"
                    | "thinks"
                    | "thought"
                    | "know"
                    | "knows"
                    | "knew"
                    | "expect"
                    | "expects"
                    | "expected"
                    | "want"
                    | "wants"
                    | "wanted"
                    | "claim"
                    | "claims"
                    | "claimed"
                    | "deny"
                    | "denies"
                    | "denied"
                    | "doubt"
                    | "doubts"
                    | "doubted"
                    | "observe"
                    | "observes"
                    | "observed"
                    | "correct"
                    | "corrects"
                    | "corrected"
                    | "warn"
                    | "warns"
                    | "warned"
            )
        })
}

fn has_individuating_entity_mention(text: &str) -> bool {
    let lower = text.to_lowercase();
    ENTITY_LEXEMES.iter().any(|lexeme| {
        find_form(&lower, lexeme.form).is_some_and(|position| {
            let prefix = lower[..position].trim_end();
            let previous = prefix
                .split_whitespace()
                .next_back()
                .unwrap_or_default()
                .trim_matches(|character: char| !character.is_alphanumeric());
            !previous.is_empty()
                && !matches!(
                    previous,
                    "a" | "an"
                        | "the"
                        | "this"
                        | "that"
                        | "some"
                        | "any"
                        | "inspect"
                        | "review"
                        | "check"
                        | "analyze"
                        | "repair"
                        | "fix"
                        | "deploy"
                        | "move"
                        | "create"
                        | "delete"
                )
                && event_concept(previous).is_none()
        })
    })
}

fn entity_concepts(text: &str) -> BTreeMap<String, String> {
    let lower = text.to_lowercase();
    let mut concepts = BTreeMap::new();
    for lexeme in ENTITY_LEXEMES {
        if find_form(&lower, lexeme.form).is_none() {
            continue;
        }
        concepts
            .entry(lexeme.direct_concept.to_string())
            .or_insert_with(|| lexeme.direct_concept.to_string());
        if let Some(parent) = lexeme.parent {
            concepts
                .entry(parent.to_string())
                .or_insert_with(|| format!("{}->{parent}", lexeme.direct_concept));
        }
    }
    concepts
}

fn event_concept(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    EVENT_LEXEMES
        .iter()
        .filter_map(|lexeme| find_form(&lower, lexeme.form).map(|position| (position, lexeme)))
        .min_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| right.1.form.len().cmp(&left.1.form.len()))
        })
        .map(|(_, lexeme)| lexeme.concept.to_string())
}

fn unchanged(text: &str) -> OntologyReferenceResolution {
    OntologyReferenceResolution {
        resolved_text: text.to_string(),
        source_surface: None,
        resolved_surface: None,
        referent_ids: Vec::new(),
        binding_kind: None,
        confidence_millis: 0,
        evidence: Vec::new(),
        ambiguous_surfaces: Vec::new(),
    }
}

fn ambiguous(text: &str, marker: &str, class: &str) -> OntologyReferenceResolution {
    OntologyReferenceResolution {
        resolved_text: text.to_string(),
        source_surface: None,
        resolved_surface: None,
        referent_ids: Vec::new(),
        binding_kind: None,
        confidence_millis: 0,
        evidence: Vec::new(),
        ambiguous_surfaces: vec![format!("{class}:{marker}")],
    }
}

fn form_at_start(text: &str, form: &str) -> bool {
    text.starts_with(form)
        && text[form.len()..]
            .chars()
            .next()
            .is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn find_form(text: &str, form: &str) -> Option<usize> {
    text.match_indices(form).find_map(|(start, _)| {
        let before = text[..start].chars().next_back();
        let after = text[start + form.len()..].chars().next();
        let ascii = form.is_ascii();
        ((!ascii
            || (before.is_none_or(|character| !character.is_ascii_alphanumeric())
                && after.is_none_or(|character| !character.is_ascii_alphanumeric())))
            && text.is_char_boundary(start)
            && text.is_char_boundary(start + form.len()))
        .then_some(start)
    })
}

fn marker_is_quoted(text: &str, marker: &str) -> bool {
    let lower = text.to_lowercase();
    let Some(start) = lower.find(&marker.to_lowercase()) else {
        return false;
    };
    let end = start + marker.len();
    quote_ranges(text)
        .iter()
        .any(|(quote_start, quote_end)| start >= *quote_start && end <= *quote_end)
}

fn quote_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut stack = Vec::<(char, usize)>::new();
    for (position, character) in text.char_indices() {
        match character {
            '‘' | '“' | '「' | '『' => stack.push((character, position)),
            '’' => close_quote(text, &mut stack, &mut ranges, position, '‘'),
            '”' => close_quote(text, &mut stack, &mut ranges, position, '“'),
            '」' => close_quote(text, &mut stack, &mut ranges, position, '「'),
            '』' => close_quote(text, &mut stack, &mut ranges, position, '『'),
            '"' | '\'' => {
                if let Some(index) = stack.iter().rposition(|(open, _)| *open == character) {
                    let (_, start) = stack.remove(index);
                    ranges.push((start, position + character.len_utf8()));
                } else {
                    stack.push((character, position));
                }
            }
            _ => {}
        }
    }
    ranges
}

fn close_quote(
    text: &str,
    stack: &mut Vec<(char, usize)>,
    ranges: &mut Vec<(usize, usize)>,
    position: usize,
    expected: char,
) {
    if let Some(index) = stack.iter().rposition(|(open, _)| *open == expected) {
        let (_, start) = stack.remove(index);
        let end = position + text[position..].chars().next().map_or(0, char::len_utf8);
        ranges.push((start, end));
    }
}

fn replace_first_case_insensitive(text: &str, marker: &str, replacement: &str) -> String {
    let lower = text.to_lowercase();
    let Some(start) = lower.find(&marker.to_lowercase()) else {
        return text.to_string();
    };
    let end = start + marker.len();
    if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return text.to_string();
    }
    format!("{}{}{}", &text[..start], replacement, &text[end..])
}

fn case_adjusted_replacement(text: &str, marker: &str, replacement: &str) -> (String, String) {
    if text_is_english(text) {
        return (marker.to_string(), replacement.to_string());
    }
    let lower = text.to_lowercase();
    let Some(start) = lower.find(&marker.to_lowercase()) else {
        return (marker.to_string(), replacement.to_string());
    };
    let end = start + marker.len();
    let Some(particle) = text[end..].chars().next() else {
        return (marker.to_string(), replacement.to_string());
    };
    let adjusted = match particle {
        '을' | '를' => object_particle(replacement),
        '이' | '가' => subject_particle(replacement),
        '은' | '는' => topic_particle(replacement),
        _ => return (marker.to_string(), replacement.to_string()),
    };
    (
        format!("{marker}{particle}"),
        format!("{replacement}{adjusted}"),
    )
}

fn has_final_consonant(value: &str) -> bool {
    value.chars().next_back().is_some_and(|character| {
        let code = u32::from(character);
        (0xac00..=0xd7a3).contains(&code) && (code - 0xac00) % 28 != 0
    })
}

fn object_particle(value: &str) -> &'static str {
    if has_final_consonant(value) {
        "을"
    } else {
        "를"
    }
}

fn subject_particle(value: &str) -> &'static str {
    if has_final_consonant(value) {
        "이"
    } else {
        "가"
    }
}

fn topic_particle(value: &str) -> &'static str {
    if has_final_consonant(value) {
        "은"
    } else {
        "는"
    }
}

fn text_is_english(text: &str) -> bool {
    let ascii_letters = text
        .chars()
        .filter(|character| character.is_ascii_alphabetic())
        .count();
    let hangul = text
        .chars()
        .filter(|character| ('\u{ac00}'..='\u{d7a3}').contains(character))
        .count();
    ascii_letters > hangul
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_reaches_document_through_one_parent_edge() {
        let concepts = entity_concepts("the Atlas report");
        assert_eq!(
            concepts.get("E_REPORT").map(String::as_str),
            Some("E_REPORT")
        );
        assert_eq!(
            concepts.get("E_DOCUMENT").map(String::as_str),
            Some("E_REPORT->E_DOCUMENT")
        );
    }

    #[test]
    fn bilingual_event_forms_share_action_concepts() {
        assert_eq!(
            event_concept("repair the parser").as_deref(),
            Some("A_REPAIR")
        );
        assert_eq!(event_concept("파서를 수정해").as_deref(), Some("A_REPAIR"));
        assert_eq!(event_concept("deploy service").as_deref(), Some("A_DEPLOY"));
        assert_eq!(
            event_concept("서비스를 배포해").as_deref(),
            Some("A_DEPLOY")
        );
    }

    #[test]
    fn quoted_reference_is_detected_structurally() {
        let text = "quote ‘that fix failed’";
        assert!(marker_is_quoted(text, "that fix"));
    }

    #[test]
    fn only_modified_entity_mentions_are_individuated() {
        assert!(has_individuating_entity_mention("inspect the Atlas report"));
        assert!(has_individuating_entity_mention("다온 보고서를 확인해"));
        assert!(!has_individuating_entity_mention("review the report"));
        assert!(!has_individuating_entity_mention("보고서를 확인해"));
    }

    #[test]
    fn korean_event_nominal_requires_object_particle() {
        assert!(event_reference("그 파서 수리를 설명해").is_some());
        assert!(event_reference("그 수리를 설명해").is_some());
        assert!(event_reference("그 문서를 검토해").is_none());
    }

    #[test]
    fn english_complementizer_is_not_a_deictic_reference() {
        assert!(event_reference("Alice says that the deployment is complete").is_none());
        assert!(event_reference("Uma expects that rollout will fail").is_none());
        assert!(entity_reference("Alice knows that the application is slow").is_none());
        assert!(event_reference("explain that deployment").is_some());
        assert!(entity_reference("review that application").is_some());
    }

    #[test]
    fn supplemental_mentions_only_close_multiword_span_gaps() {
        let mut referents = Vec::new();
        merge_ontology_mentions(&mut referents, 1, "나중에 보고서를 확인해");
        assert!(referents.is_empty());
        merge_ontology_mentions(&mut referents, 2, "라움 저장 계층을 확인해");
        assert_eq!(referents.len(), 1);
        assert_eq!(referents[0].canonical_surface, "라움 저장 계층");
        assert!(!referents[0].semantic_authority);
    }

    #[test]
    fn korean_case_particle_is_realized_for_the_bound_referent() {
        assert_eq!(
            case_adjusted_replacement("그 앱을 검토해", "그 앱", "이든 백엔드"),
            ("그 앱을".to_string(), "이든 백엔드를".to_string())
        );
        assert_eq!(
            case_adjusted_replacement("그 출시를 설명해", "그 출시", "‘배포해’라는 작업"),
            ("그 출시를".to_string(), "‘배포해’라는 작업을".to_string())
        );
    }
}
