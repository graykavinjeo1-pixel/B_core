//! Evidence-bound expansion of the source-repair compiler.
//!
//! The five structural edit atoms remain the trusted substrate.  This module
//! learns reusable *composite* edit primitives from independently verified
//! repairs and lowers every application back to those atoms.  It also admits
//! an unknown source extension only as a compiler-validated exact-span
//! backend.  A learned rule is therefore executable, but never becomes an
//! approval authority for its own output.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::self_repair_contract::sha256;
use crate::structural_source_repair::{apply_edit_atom, ByteRange, SourceEditAtom};

pub const META_COMPILER_REGISTRY_SCHEMA: &str = "b_core_meta_compiler_registry_v1";
pub const META_COMPILER_GAP_SCHEMA: &str = "b_core_meta_compiler_gap_v1";
pub const VERIFIED_REPAIR_EXAMPLE_SCHEMA: &str = "b_core_verified_repair_example_v1";
pub const INDUCED_EDIT_PRIMITIVE_SCHEMA: &str = "b_core_induced_edit_primitive_v1";
pub const INDUCED_LANGUAGE_BACKEND_SCHEMA: &str = "b_core_induced_language_backend_v1";
pub const META_COMPILER_KNOWLEDGE_UNIVERSE_SHA256: &str =
    "BF9F10C31F6504473050028280C1DCB22AC54CFD29DFC9215EC29DDD42BBE52C";
pub const META_COMPILER_KNOWLEDGE_MECHANISMS: &[(&str, &str)] = &[
    (
        "abstraction",
        "563004B81323A8C538748BB7B5C716B5D80AAA33F2681F06700268C220167A64",
    ),
    (
        "compiler_runtime",
        "FE389FCFFD1DDB30FF503A69D944720E9948C5F500282F24DA78715AFFADBC91",
    ),
    (
        "macro_system",
        "00B5D94307C7C3B8D4EF66B0D48D002356F7988609E358BE22C7546226E1E843",
    ),
    (
        "pattern_matching",
        "D5AE94F750559C76FC0710F36A488864905E82B9CE38CB38F0F2A784B9F86F8D",
    ),
    (
        "static_typing",
        "FEE4EDE168E80A7995BA1309A316975B24BA06583284A666833DA480499A2BCA",
    ),
];

const MAX_EXAMPLES: usize = 128;
const MAX_PRIMITIVES: usize = 64;
const MAX_BACKENDS: usize = 32;
const MAX_FRAGMENT_BYTES: usize = 16 * 1024;
const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
const MIN_INDEPENDENT_CONTEXTS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MetaCompilerGapKind {
    MissingCompositeEditPrimitive,
    MissingLanguageBackend,
    UnrepresentablePostcondition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaCompilerGapIR {
    pub schema: String,
    pub gap_id: String,
    pub kind: MetaCompilerGapKind,
    pub failure_code: String,
    pub language_id: String,
    pub source_extension: String,
    pub required_mechanism_ids: Vec<String>,
    pub public_evidence_sha256s: Vec<String>,
    pub counterexample_sha256s: Vec<String>,
    pub target_symbols: Vec<String>,
    pub independent_context_ids: Vec<String>,
    pub resolved_by_capability_id: Option<String>,
}

impl MetaCompilerGapIR {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: MetaCompilerGapKind,
        failure_code: impl Into<String>,
        language_id: impl Into<String>,
        source_extension: impl Into<String>,
        required_mechanism_ids: Vec<String>,
        public_evidence_sha256s: Vec<String>,
        counterexample_sha256s: Vec<String>,
        target_symbols: Vec<String>,
        independent_context_ids: Vec<String>,
    ) -> Result<Self, String> {
        let failure_code = failure_code.into();
        let language_id = normalize_language_id(&language_id.into())?;
        let source_extension = normalize_extension(&source_extension.into())?;
        let mut value = Self {
            schema: META_COMPILER_GAP_SCHEMA.to_string(),
            gap_id: String::new(),
            kind,
            failure_code,
            language_id,
            source_extension,
            required_mechanism_ids: sorted_unique(required_mechanism_ids),
            public_evidence_sha256s: sorted_unique(public_evidence_sha256s),
            counterexample_sha256s: sorted_unique(counterexample_sha256s),
            target_symbols: sorted_unique(target_symbols),
            independent_context_ids: sorted_unique(independent_context_ids),
            resolved_by_capability_id: None,
        };
        value.gap_id = sha256(
            serde_json::to_vec(&(
                &value.kind,
                &value.failure_code,
                &value.language_id,
                &value.source_extension,
                &value.required_mechanism_ids,
                &value.public_evidence_sha256s,
                &value.counterexample_sha256s,
                &value.target_symbols,
                &value.independent_context_ids,
            ))
            .map_err(|error| format!("META_COMPILER_GAP_SERIALIZE:{error}"))?
            .as_slice(),
        );
        Ok(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LexicalTokenKind {
    Identifier,
    Number,
    StringLiteral,
    Whitespace,
    Punctuation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "segment", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SyntaxTemplateSegment {
    Literal {
        text: String,
    },
    Hole {
        hole_id: String,
        kind: LexicalTokenKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTemplateIR {
    pub segments: Vec<SyntaxTemplateSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedRepairExampleIR {
    pub schema: String,
    pub example_id: String,
    pub independent_context_id: String,
    pub language_id: String,
    pub source_extension: String,
    pub source_relative_path: PathBuf,
    pub before_template: SyntaxTemplateIR,
    pub after_template: SyntaxTemplateIR,
    pub before_fragment_sha256: String,
    pub after_fragment_sha256: String,
    #[serde(skip_serializing, default)]
    pub before_fragment: String,
    #[serde(skip_serializing, default)]
    pub after_fragment: String,
    pub base_edit_topology: Vec<String>,
    pub public_verification_sha256: String,
    pub compile_verification_sha256: String,
    pub public_observation_passed: bool,
    pub source_compile_passed: bool,
    pub gold_answer_used: bool,
    pub utf8_roundtrip_passed: bool,
}

impl VerifiedRepairExampleIR {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        independent_context_id: impl Into<String>,
        language_id: impl Into<String>,
        source_extension: impl Into<String>,
        source_relative_path: PathBuf,
        before_fragment: impl Into<String>,
        after_fragment: impl Into<String>,
        base_edit_topology: Vec<String>,
        public_verification_sha256: impl Into<String>,
        compile_verification_sha256: impl Into<String>,
    ) -> Result<Self, String> {
        let independent_context_id = independent_context_id.into();
        let language_id = normalize_language_id(&language_id.into())?;
        let source_extension = normalize_extension(&source_extension.into())?;
        let before_fragment = before_fragment.into();
        let after_fragment = after_fragment.into();
        let public_verification_sha256 = public_verification_sha256.into();
        let compile_verification_sha256 = compile_verification_sha256.into();
        if independent_context_id.trim().is_empty() {
            return Err("META_COMPILER_EXAMPLE_CONTEXT_EMPTY".to_string());
        }
        if path_is_test_target(&source_relative_path) {
            return Err("META_COMPILER_EXAMPLE_TEST_TARGET".to_string());
        }
        if before_fragment == after_fragment {
            return Err("META_COMPILER_EXAMPLE_NO_OP".to_string());
        }
        if before_fragment.len() > MAX_FRAGMENT_BYTES || after_fragment.len() > MAX_FRAGMENT_BYTES {
            return Err("META_COMPILER_EXAMPLE_FRAGMENT_BUDGET".to_string());
        }
        if base_edit_topology.is_empty() {
            return Err("META_COMPILER_EXAMPLE_EDIT_TOPOLOGY_EMPTY".to_string());
        }
        validate_sha256(&public_verification_sha256, "PUBLIC_VERIFICATION")?;
        validate_sha256(&compile_verification_sha256, "COMPILE_VERIFICATION")?;
        let (before_template, after_template) =
            canonical_example_templates(&language_id, &before_fragment, &after_fragment)?;
        let before_fragment_sha256 = sha256(before_fragment.as_bytes());
        let after_fragment_sha256 = sha256(after_fragment.as_bytes());
        let example_id = sha256(
            serde_json::to_vec(&(
                &independent_context_id,
                &language_id,
                &source_extension,
                &source_relative_path,
                &before_template,
                &after_template,
                &before_fragment_sha256,
                &after_fragment_sha256,
                &base_edit_topology,
                &public_verification_sha256,
                &compile_verification_sha256,
            ))
            .map_err(|error| format!("META_COMPILER_EXAMPLE_SERIALIZE:{error}"))?
            .as_slice(),
        );
        Ok(Self {
            schema: VERIFIED_REPAIR_EXAMPLE_SCHEMA.to_string(),
            example_id,
            independent_context_id,
            language_id,
            source_extension,
            source_relative_path,
            before_template,
            after_template,
            before_fragment_sha256,
            after_fragment_sha256,
            before_fragment,
            after_fragment,
            base_edit_topology,
            public_verification_sha256,
            compile_verification_sha256,
            public_observation_passed: true,
            source_compile_passed: true,
            gold_answer_used: false,
            utf8_roundtrip_passed: true,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InducedEditPrimitiveIR {
    pub schema: String,
    pub primitive_id: String,
    pub language_id: String,
    pub source_extension: String,
    pub required_mechanism_ids: Vec<String>,
    pub before_template: SyntaxTemplateIR,
    pub after_template: SyntaxTemplateIR,
    pub base_edit_topology: Vec<String>,
    pub evidence_example_ids: Vec<String>,
    pub independent_context_ids: Vec<String>,
    pub verification_evidence_sha256s: Vec<String>,
    pub source_compile_required: bool,
    pub public_observation_required: bool,
    pub promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializedPrimitiveApplicationIR {
    pub primitive_id: String,
    pub source_relative_path: PathBuf,
    pub predecessor_sha256: String,
    pub candidate_sha256: String,
    pub bound_roles: BTreeMap<String, String>,
    pub edit: SourceEditAtom,
    pub candidate_source: String,
    pub requires_source_compile: bool,
    pub requires_public_observation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DynamicBackendParserMode {
    CompilerValidatedExactSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageBackendCandidateIR {
    pub language_id: String,
    pub extensions: Vec<String>,
    pub parser_mode: DynamicBackendParserMode,
    pub required_mechanism_ids: Vec<String>,
    pub knowledge_universe_sha256: String,
    pub knowledge_node_sha256s: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendValidationEvidenceIR {
    pub independent_context_ids: Vec<String>,
    pub source_roundtrip_passed: bool,
    pub utf8_byte_spans_stable: bool,
    pub no_op_roundtrip_passed: bool,
    pub compile_verification_sha256: String,
    pub public_verification_sha256: String,
    pub source_compile_passed: bool,
    pub public_observation_passed: bool,
    pub test_target_write_events: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InducedLanguageBackendIR {
    pub schema: String,
    pub backend_id: String,
    pub language_id: String,
    pub extensions: Vec<String>,
    pub parser_mode: DynamicBackendParserMode,
    pub required_mechanism_ids: Vec<String>,
    pub knowledge_universe_sha256: String,
    pub knowledge_node_sha256s: Vec<String>,
    pub independent_context_ids: Vec<String>,
    pub compile_verification_sha256: String,
    pub public_verification_sha256: String,
    pub promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaCompilerRegistryIR {
    pub schema: String,
    pub knowledge_universe_sha256: String,
    pub knowledge_mechanism_node_sha256s: BTreeMap<String, String>,
    pub verified_examples: Vec<VerifiedRepairExampleIR>,
    pub edit_primitives: Vec<InducedEditPrimitiveIR>,
    pub language_backends: Vec<InducedLanguageBackendIR>,
    pub unresolved_gaps: Vec<MetaCompilerGapIR>,
    pub registry_sha256: String,
}

impl Default for MetaCompilerRegistryIR {
    fn default() -> Self {
        let mut value = Self {
            schema: META_COMPILER_REGISTRY_SCHEMA.to_string(),
            knowledge_universe_sha256: META_COMPILER_KNOWLEDGE_UNIVERSE_SHA256.to_string(),
            knowledge_mechanism_node_sha256s: META_COMPILER_KNOWLEDGE_MECHANISMS
                .iter()
                .map(|(mechanism, node)| ((*mechanism).to_string(), (*node).to_string()))
                .collect(),
            verified_examples: Vec::new(),
            edit_primitives: Vec::new(),
            language_backends: Vec::new(),
            unresolved_gaps: Vec::new(),
            registry_sha256: String::new(),
        };
        value
            .refresh_hash()
            .expect("empty registry is serializable");
        value
    }
}

impl MetaCompilerRegistryIR {
    pub fn capability_sha256(&self) -> Result<String, String> {
        Ok(sha256(
            serde_json::to_vec(&(
                &self.schema,
                &self.knowledge_universe_sha256,
                &self.knowledge_mechanism_node_sha256s,
                &self.edit_primitives,
                &self.language_backends,
            ))
            .map_err(|error| format!("META_COMPILER_CAPABILITY_SERIALIZE:{error}"))?
            .as_slice(),
        ))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != META_COMPILER_REGISTRY_SCHEMA {
            return Err("META_COMPILER_REGISTRY_SCHEMA".to_string());
        }
        if self.knowledge_universe_sha256 != META_COMPILER_KNOWLEDGE_UNIVERSE_SHA256 {
            return Err("META_COMPILER_KNOWLEDGE_UNIVERSE".to_string());
        }
        if self.verified_examples.len() > MAX_EXAMPLES
            || self.edit_primitives.len() > MAX_PRIMITIVES
            || self.language_backends.len() > MAX_BACKENDS
        {
            return Err("META_COMPILER_REGISTRY_BUDGET".to_string());
        }
        let example_ids = self
            .verified_examples
            .iter()
            .map(|example| example.example_id.as_str())
            .collect::<BTreeSet<_>>();
        if example_ids.len() != self.verified_examples.len() {
            return Err("META_COMPILER_EXAMPLE_ID_DUPLICATE".to_string());
        }
        let primitive_ids = self
            .edit_primitives
            .iter()
            .map(|primitive| primitive.primitive_id.as_str())
            .collect::<BTreeSet<_>>();
        if primitive_ids.len() != self.edit_primitives.len() {
            return Err("META_COMPILER_PRIMITIVE_ID_DUPLICATE".to_string());
        }
        for primitive in &self.edit_primitives {
            validate_induced_primitive(primitive)?;
            if primitive
                .evidence_example_ids
                .iter()
                .any(|id| !example_ids.contains(id.as_str()))
            {
                return Err("META_COMPILER_PRIMITIVE_EVIDENCE_MISSING".to_string());
            }
        }
        for backend in &self.language_backends {
            validate_induced_backend(backend)?;
        }
        if self.registry_sha256 != registry_payload_sha256(self)? {
            return Err("META_COMPILER_REGISTRY_HASH".to_string());
        }
        Ok(())
    }

    pub fn record_gap(&mut self, gap: MetaCompilerGapIR) -> Result<(), String> {
        if let Some(existing) = self
            .unresolved_gaps
            .iter_mut()
            .find(|existing| existing.gap_id == gap.gap_id)
        {
            *existing = gap;
        } else {
            self.unresolved_gaps.push(gap);
            self.unresolved_gaps
                .sort_by(|left, right| left.gap_id.cmp(&right.gap_id));
            self.unresolved_gaps.truncate(MAX_EXAMPLES);
        }
        self.refresh_hash()
    }

    /// Records a real verified repair and attempts to induce an executable
    /// primitive from every compatible, independent pair.  Candidate rules
    /// are not retained: induction either proves all constraints or fails.
    pub fn learn_verified_example(
        &mut self,
        example: VerifiedRepairExampleIR,
    ) -> Result<Vec<String>, String> {
        validate_verified_example(&example)?;
        if !self
            .verified_examples
            .iter()
            .any(|existing| existing.example_id == example.example_id)
        {
            self.verified_examples.push(example.clone());
            self.verified_examples
                .sort_by(|left, right| left.example_id.cmp(&right.example_id));
            if self.verified_examples.len() > MAX_EXAMPLES {
                self.verified_examples.remove(0);
            }
        }
        let mut promoted = Vec::new();
        let compatible = self
            .verified_examples
            .iter()
            .filter(|other| {
                other.language_id == example.language_id
                    && other.source_extension == example.source_extension
                    && other.base_edit_topology == example.base_edit_topology
                    && other.independent_context_id != example.independent_context_id
            })
            .cloned()
            .collect::<Vec<_>>();
        for other in compatible {
            let evidence = if other.example_id < example.example_id {
                vec![other, example.clone()]
            } else {
                vec![example.clone(), other]
            };
            let Ok(primitive) = induce_edit_primitive(&evidence) else {
                continue;
            };
            if !self
                .edit_primitives
                .iter()
                .any(|existing| existing.primitive_id == primitive.primitive_id)
            {
                promoted.push(primitive.primitive_id.clone());
                self.edit_primitives.push(primitive);
            }
        }
        self.edit_primitives
            .sort_by(|left, right| left.primitive_id.cmp(&right.primitive_id));
        if self.edit_primitives.len() > MAX_PRIMITIVES {
            self.edit_primitives
                .drain(0..self.edit_primitives.len() - MAX_PRIMITIVES);
        }
        self.try_authorize_exact_span_backend_from_examples(&example)?;
        self.resolve_gaps();
        self.refresh_hash()?;
        Ok(promoted)
    }

    pub fn authorize_language_backend(
        &mut self,
        candidate: LanguageBackendCandidateIR,
        evidence: BackendValidationEvidenceIR,
    ) -> Result<String, String> {
        let backend = induce_language_backend(candidate, evidence)?;
        let backend_id = backend.backend_id.clone();
        if let Some(existing) = self
            .language_backends
            .iter_mut()
            .find(|existing| existing.backend_id == backend_id)
        {
            *existing = backend;
        } else {
            self.language_backends.push(backend);
        }
        self.language_backends
            .sort_by(|left, right| left.backend_id.cmp(&right.backend_id));
        if self.language_backends.len() > MAX_BACKENDS {
            self.language_backends.remove(0);
        }
        self.resolve_gaps();
        self.refresh_hash()?;
        Ok(backend_id)
    }

    pub fn backend_for_path(&self, path: &Path) -> Option<&InducedLanguageBackendIR> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        self.language_backends
            .iter()
            .find(|backend| backend.promoted && backend.extensions.contains(&extension))
    }

    pub fn materialize_candidates(
        &self,
        language_id: &str,
        source_relative_path: &Path,
        source: &str,
    ) -> Vec<MaterializedPrimitiveApplicationIR> {
        let extension = source_relative_path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(extension.as_str(), "rs" | "py")
            && self.backend_for_path(source_relative_path).is_none()
        {
            return Vec::new();
        }
        self.edit_primitives
            .iter()
            .filter(|primitive| {
                primitive.promoted
                    && primitive.language_id == language_id
                    && primitive.source_extension == extension
            })
            .filter_map(|primitive| {
                materialize_induced_primitive(primitive, source_relative_path, source).ok()
            })
            .collect()
    }

    fn resolve_gaps(&mut self) {
        for gap in &mut self.unresolved_gaps {
            if gap.resolved_by_capability_id.is_some() {
                continue;
            }
            gap.resolved_by_capability_id = match gap.kind {
                MetaCompilerGapKind::MissingLanguageBackend => self
                    .language_backends
                    .iter()
                    .find(|backend| {
                        backend.promoted
                            && backend.language_id == gap.language_id
                            && backend.extensions.contains(&gap.source_extension)
                    })
                    .map(|backend| backend.backend_id.clone()),
                MetaCompilerGapKind::MissingCompositeEditPrimitive
                | MetaCompilerGapKind::UnrepresentablePostcondition => {
                    self.edit_primitives
                        .iter()
                        .find(|primitive| {
                            primitive.promoted
                                && primitive.language_id == gap.language_id
                                && primitive.source_extension == gap.source_extension
                                && gap.required_mechanism_ids.iter().all(|required| {
                                    primitive.required_mechanism_ids.contains(required)
                                })
                        })
                        .map(|primitive| primitive.primitive_id.clone())
                }
            };
        }
    }

    fn try_authorize_exact_span_backend_from_examples(
        &mut self,
        example: &VerifiedRepairExampleIR,
    ) -> Result<(), String> {
        if matches!(example.source_extension.as_str(), "rs" | "py")
            || self.language_backends.iter().any(|backend| {
                backend.promoted
                    && backend.language_id == example.language_id
                    && backend.extensions.contains(&example.source_extension)
            })
        {
            return Ok(());
        }
        let evidence = self
            .verified_examples
            .iter()
            .filter(|candidate| {
                candidate.language_id == example.language_id
                    && candidate.source_extension == example.source_extension
                    && candidate.public_observation_passed
                    && candidate.source_compile_passed
                    && !candidate.gold_answer_used
            })
            .cloned()
            .collect::<Vec<_>>();
        let contexts = evidence
            .iter()
            .map(|candidate| candidate.independent_context_id.clone())
            .collect::<BTreeSet<_>>();
        if contexts.len() < MIN_INDEPENDENT_CONTEXTS {
            return Ok(());
        }
        if evidence
            .iter()
            .any(|observed| !observed.utf8_roundtrip_passed)
        {
            return Err("META_COMPILER_BACKEND_ROUNDTRIP".to_string());
        }
        let compile_verification_sha256 = sha256(
            evidence
                .iter()
                .flat_map(|candidate| candidate.compile_verification_sha256.bytes())
                .collect::<Vec<_>>()
                .as_slice(),
        );
        let public_verification_sha256 = sha256(
            evidence
                .iter()
                .flat_map(|candidate| candidate.public_verification_sha256.bytes())
                .collect::<Vec<_>>()
                .as_slice(),
        );
        self.authorize_language_backend(
            LanguageBackendCandidateIR {
                language_id: example.language_id.clone(),
                extensions: vec![example.source_extension.clone()],
                parser_mode: DynamicBackendParserMode::CompilerValidatedExactSpan,
                required_mechanism_ids: vec![
                    "compiler_runtime".to_string(),
                    "pattern_matching".to_string(),
                ],
                knowledge_universe_sha256: META_COMPILER_KNOWLEDGE_UNIVERSE_SHA256.to_string(),
                knowledge_node_sha256s: vec![
                    META_COMPILER_KNOWLEDGE_MECHANISMS[1].1.to_string(),
                    META_COMPILER_KNOWLEDGE_MECHANISMS[3].1.to_string(),
                ],
            },
            BackendValidationEvidenceIR {
                independent_context_ids: contexts.into_iter().collect(),
                source_roundtrip_passed: true,
                utf8_byte_spans_stable: true,
                no_op_roundtrip_passed: true,
                compile_verification_sha256,
                public_verification_sha256,
                source_compile_passed: true,
                public_observation_passed: true,
                test_target_write_events: 0,
            },
        )?;
        Ok(())
    }

    fn refresh_hash(&mut self) -> Result<(), String> {
        self.registry_sha256 = registry_payload_sha256(self)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LexicalToken {
    kind: LexicalTokenKind,
    text: String,
    start: usize,
    end: usize,
}

fn sorted_unique(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

fn validate_sha256(value: &str, label: &str) -> Result<(), String> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!("META_COMPILER_{label}_SHA256"))
    }
}

fn normalize_language_id(value: &str) -> Result<String, String> {
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'+'))
    {
        return Err("META_COMPILER_LANGUAGE_ID".to_string());
    }
    Ok(value)
}

fn normalize_extension(value: &str) -> Result<String, String> {
    let value = value.trim().trim_start_matches('.').to_ascii_lowercase();
    if value.is_empty()
        || value.len() > 16
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err("META_COMPILER_SOURCE_EXTENSION".to_string());
    }
    Ok(value)
}

fn path_is_test_target(path: &Path) -> bool {
    path.components().any(|component| {
        let component = component.as_os_str().to_string_lossy().to_ascii_lowercase();
        component == "tests" || component == "test" || component == "__tests__"
    }) || path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            let name = name.to_ascii_lowercase();
            name.starts_with("test_")
                || name.ends_with("_test.rs")
                || name.ends_with("_test.py")
                || name.ends_with(".test.js")
        })
}

fn tokenize(source: &str) -> Result<Vec<LexicalToken>, String> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let start = index;
        let first = source[index..]
            .chars()
            .next()
            .ok_or_else(|| "META_COMPILER_TOKEN_UTF8".to_string())?;
        let kind = if first.is_whitespace() {
            index += first.len_utf8();
            while index < bytes.len() {
                let current = source[index..].chars().next().unwrap();
                if !current.is_whitespace() {
                    break;
                }
                index += current.len_utf8();
            }
            LexicalTokenKind::Whitespace
        } else if first == '_' || first.is_alphabetic() {
            index += first.len_utf8();
            while index < bytes.len() {
                let current = source[index..].chars().next().unwrap();
                if current != '_' && !current.is_alphanumeric() {
                    break;
                }
                index += current.len_utf8();
            }
            LexicalTokenKind::Identifier
        } else if first.is_ascii_digit() {
            index += 1;
            while index < bytes.len() {
                let current = bytes[index];
                if !current.is_ascii_alphanumeric() && !matches!(current, b'_' | b'.') {
                    break;
                }
                index += 1;
            }
            LexicalTokenKind::Number
        } else if matches!(first, '\'' | '"' | '`') {
            let quote = first as u8;
            index += 1;
            let mut escaped = false;
            while index < bytes.len() {
                let current = bytes[index];
                index += 1;
                if escaped {
                    escaped = false;
                } else if current == b'\\' {
                    escaped = true;
                } else if current == quote {
                    break;
                }
            }
            if bytes.get(index.wrapping_sub(1)).copied() != Some(quote) {
                return Err("META_COMPILER_UNTERMINATED_STRING".to_string());
            }
            LexicalTokenKind::StringLiteral
        } else {
            index += first.len_utf8();
            LexicalTokenKind::Punctuation
        };
        tokens.push(LexicalToken {
            kind,
            text: source[start..index].to_string(),
            start,
            end: index,
        });
    }
    if tokens.is_empty() {
        return Err("META_COMPILER_EMPTY_FRAGMENT".to_string());
    }
    Ok(tokens)
}

fn is_language_keyword(language_id: &str, text: &str) -> bool {
    let common = matches!(
        text,
        "return"
            | "if"
            | "else"
            | "for"
            | "while"
            | "break"
            | "continue"
            | "async"
            | "await"
            | "try"
            | "catch"
            | "throw"
            | "raise"
            | "yield"
            | "match"
            | "switch"
            | "case"
            | "class"
            | "struct"
            | "enum"
            | "trait"
            | "impl"
            | "import"
            | "from"
            | "use"
            | "mod"
            | "pub"
            | "let"
            | "const"
            | "mut"
            | "true"
            | "false"
            | "null"
            | "None"
    );
    common
        || (language_id == "python" && matches!(text, "def" | "in" | "is" | "and" | "or" | "not"))
        || (language_id == "rust" && matches!(text, "fn" | "where" | "Self" | "self" | "crate"))
        || (language_id == "zig" && matches!(text, "fn" | "var" | "comptime" | "defer"))
}

fn canonical_example_templates(
    language_id: &str,
    before: &str,
    after: &str,
) -> Result<(SyntaxTemplateIR, SyntaxTemplateIR), String> {
    let before_tokens = tokenize(before)?;
    let after_tokens = tokenize(after)?;
    if before_tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<String>()
        != before
        || after_tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<String>()
            != after
    {
        return Err("META_COMPILER_TEMPLATE_UTF8_ROUNDTRIP".to_string());
    }
    let mut roles = BTreeMap::<(LexicalTokenKind, String), String>::new();
    let mut before_segments = Vec::new();
    for token in before_tokens {
        let dynamic = matches!(
            token.kind,
            LexicalTokenKind::Identifier
                | LexicalTokenKind::Number
                | LexicalTokenKind::StringLiteral
        ) && !(token.kind == LexicalTokenKind::Identifier
            && is_language_keyword(language_id, &token.text));
        if dynamic {
            let next_id = format!("role_{}", roles.len());
            let hole_id = roles
                .entry((token.kind, token.text))
                .or_insert(next_id)
                .clone();
            before_segments.push(SyntaxTemplateSegment::Hole {
                hole_id,
                kind: token.kind,
            });
        } else {
            before_segments.push(SyntaxTemplateSegment::Literal { text: token.text });
        }
    }
    let mut after_segments = Vec::new();
    for token in after_tokens {
        let dynamic = matches!(
            token.kind,
            LexicalTokenKind::Identifier
                | LexicalTokenKind::Number
                | LexicalTokenKind::StringLiteral
        ) && !(token.kind == LexicalTokenKind::Identifier
            && is_language_keyword(language_id, &token.text));
        if dynamic {
            let hole_id = roles
                .get(&(token.kind, token.text.clone()))
                .ok_or_else(|| "META_COMPILER_AFTER_ONLY_DYNAMIC_HOLE".to_string())?;
            after_segments.push(SyntaxTemplateSegment::Hole {
                hole_id: hole_id.clone(),
                kind: token.kind,
            });
        } else {
            after_segments.push(SyntaxTemplateSegment::Literal { text: token.text });
        }
    }
    Ok((
        SyntaxTemplateIR {
            segments: before_segments,
        },
        SyntaxTemplateIR {
            segments: after_segments,
        },
    ))
}

pub fn induce_edit_primitive(
    examples: &[VerifiedRepairExampleIR],
) -> Result<InducedEditPrimitiveIR, String> {
    if examples.len() < MIN_INDEPENDENT_CONTEXTS {
        return Err("META_COMPILER_INDUCTION_EVIDENCE_COUNT".to_string());
    }
    for example in examples {
        validate_verified_example(example)?;
    }
    let contexts = examples
        .iter()
        .map(|example| example.independent_context_id.clone())
        .collect::<BTreeSet<_>>();
    if contexts.len() < MIN_INDEPENDENT_CONTEXTS {
        return Err("META_COMPILER_INDUCTION_CONTEXT_DIVERSITY".to_string());
    }
    let language_id = &examples[0].language_id;
    let source_extension = &examples[0].source_extension;
    let topology = &examples[0].base_edit_topology;
    if examples.iter().any(|example| {
        &example.language_id != language_id
            || &example.source_extension != source_extension
            || &example.base_edit_topology != topology
    }) {
        return Err("META_COMPILER_INDUCTION_CONTRACT_MISMATCH".to_string());
    }
    let before_template = examples[0].before_template.clone();
    let after_template = examples[0].after_template.clone();
    if examples.iter().any(|example| {
        example.before_template != before_template || example.after_template != after_template
    }) {
        return Err("META_COMPILER_INDUCTION_TEMPLATE_MISMATCH".to_string());
    }
    if before_template == after_template {
        return Err("META_COMPILER_INDUCTION_NO_OP".to_string());
    }
    let required_mechanism_ids = vec![
        "abstraction".to_string(),
        "compiler_runtime".to_string(),
        "macro_system".to_string(),
        "pattern_matching".to_string(),
        "static_typing".to_string(),
    ];
    let evidence_example_ids = sorted_unique(
        examples
            .iter()
            .map(|example| example.example_id.clone())
            .collect(),
    );
    let independent_context_ids = sorted_unique(contexts.into_iter().collect());
    let verification_evidence_sha256s = sorted_unique(
        examples
            .iter()
            .flat_map(|example| {
                [
                    example.public_verification_sha256.clone(),
                    example.compile_verification_sha256.clone(),
                ]
            })
            .collect(),
    );
    let primitive_id = sha256(
        serde_json::to_vec(&(
            language_id,
            source_extension,
            &required_mechanism_ids,
            &before_template,
            &after_template,
            topology,
            &evidence_example_ids,
        ))
        .map_err(|error| format!("META_COMPILER_PRIMITIVE_SERIALIZE:{error}"))?
        .as_slice(),
    );
    let primitive = InducedEditPrimitiveIR {
        schema: INDUCED_EDIT_PRIMITIVE_SCHEMA.to_string(),
        primitive_id,
        language_id: language_id.clone(),
        source_extension: source_extension.clone(),
        required_mechanism_ids,
        before_template,
        after_template,
        base_edit_topology: topology.clone(),
        evidence_example_ids,
        independent_context_ids,
        verification_evidence_sha256s,
        source_compile_required: true,
        public_observation_required: true,
        promoted: true,
    };
    validate_induced_primitive(&primitive)?;
    Ok(primitive)
}

fn validate_verified_example(example: &VerifiedRepairExampleIR) -> Result<(), String> {
    if example.schema != VERIFIED_REPAIR_EXAMPLE_SCHEMA
        || !example.public_observation_passed
        || !example.source_compile_passed
        || example.gold_answer_used
    {
        return Err("META_COMPILER_EXAMPLE_NOT_VERIFIED".to_string());
    }
    if path_is_test_target(&example.source_relative_path) {
        return Err("META_COMPILER_EXAMPLE_TEST_TARGET".to_string());
    }
    if example.before_template == example.after_template {
        return Err("META_COMPILER_EXAMPLE_NO_OP".to_string());
    }
    validate_sha256(&example.before_fragment_sha256, "BEFORE_FRAGMENT")?;
    validate_sha256(&example.after_fragment_sha256, "AFTER_FRAGMENT")?;
    if !example.utf8_roundtrip_passed {
        return Err("META_COMPILER_EXAMPLE_UTF8_ROUNDTRIP".to_string());
    }
    validate_sha256(&example.public_verification_sha256, "PUBLIC_VERIFICATION")?;
    validate_sha256(&example.compile_verification_sha256, "COMPILE_VERIFICATION")?;
    Ok(())
}

fn validate_induced_primitive(primitive: &InducedEditPrimitiveIR) -> Result<(), String> {
    if primitive.schema != INDUCED_EDIT_PRIMITIVE_SCHEMA
        || !primitive.promoted
        || !primitive.source_compile_required
        || !primitive.public_observation_required
        || primitive.independent_context_ids.len() < MIN_INDEPENDENT_CONTEXTS
        || primitive.evidence_example_ids.len() < MIN_INDEPENDENT_CONTEXTS
        || primitive.before_template == primitive.after_template
    {
        return Err("META_COMPILER_PRIMITIVE_AUTHORITY".to_string());
    }
    validate_sha256(&primitive.primitive_id, "PRIMITIVE_ID")?;
    for required in [
        "abstraction",
        "compiler_runtime",
        "macro_system",
        "pattern_matching",
        "static_typing",
    ] {
        if !primitive
            .required_mechanism_ids
            .iter()
            .any(|mechanism| mechanism == required)
        {
            return Err(format!("META_COMPILER_PRIMITIVE_MECHANISM:{required}"));
        }
    }
    Ok(())
}

fn match_template_at(
    template: &SyntaxTemplateIR,
    tokens: &[LexicalToken],
    start: usize,
) -> Option<BTreeMap<String, String>> {
    if start + template.segments.len() > tokens.len() {
        return None;
    }
    let mut bindings = BTreeMap::new();
    for (offset, segment) in template.segments.iter().enumerate() {
        let token = &tokens[start + offset];
        match segment {
            SyntaxTemplateSegment::Literal { text } if &token.text == text => {}
            SyntaxTemplateSegment::Literal { .. } => return None,
            SyntaxTemplateSegment::Hole { hole_id, kind } if token.kind == *kind => {
                if let Some(existing) = bindings.get(hole_id) {
                    if existing != &token.text {
                        return None;
                    }
                } else {
                    bindings.insert(hole_id.clone(), token.text.clone());
                }
            }
            SyntaxTemplateSegment::Hole { .. } => return None,
        }
    }
    Some(bindings)
}

fn render_template(
    template: &SyntaxTemplateIR,
    bindings: &BTreeMap<String, String>,
) -> Result<String, String> {
    let mut rendered = String::new();
    for segment in &template.segments {
        match segment {
            SyntaxTemplateSegment::Literal { text } => rendered.push_str(text),
            SyntaxTemplateSegment::Hole { hole_id, .. } => rendered.push_str(
                bindings
                    .get(hole_id)
                    .ok_or_else(|| format!("META_COMPILER_ROLE_UNBOUND:{hole_id}"))?,
            ),
        }
    }
    Ok(rendered)
}

pub fn materialize_induced_primitive(
    primitive: &InducedEditPrimitiveIR,
    source_relative_path: &Path,
    source: &str,
) -> Result<MaterializedPrimitiveApplicationIR, String> {
    validate_induced_primitive(primitive)?;
    if path_is_test_target(source_relative_path) {
        return Err("META_COMPILER_APPLICATION_TEST_TARGET".to_string());
    }
    if source.len() > MAX_SOURCE_BYTES {
        return Err("META_COMPILER_APPLICATION_SOURCE_BUDGET".to_string());
    }
    let extension = source_relative_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if extension != primitive.source_extension {
        return Err("META_COMPILER_APPLICATION_EXTENSION".to_string());
    }
    let tokens = tokenize(source)?;
    let matches = (0..tokens.len())
        .filter_map(|start| {
            match_template_at(&primitive.before_template, &tokens, start)
                .map(|bindings| (start, bindings))
        })
        .collect::<Vec<_>>();
    if matches.is_empty() {
        return Err("META_COMPILER_APPLICATION_NO_MATCH".to_string());
    }
    if matches.len() != 1 {
        return Err("META_COMPILER_APPLICATION_AMBIGUOUS".to_string());
    }
    let (start, bindings) = &matches[0];
    let end_index = start + primitive.before_template.segments.len() - 1;
    let range = ByteRange {
        start: tokens[*start].start,
        end: tokens[end_index].end,
    };
    let before = &source[range.start..range.end];
    let replacement = render_template(&primitive.after_template, bindings)?;
    if before == replacement {
        return Err("META_COMPILER_APPLICATION_NO_OP".to_string());
    }
    let edit = SourceEditAtom::Replace {
        range,
        expected_sha256: sha256(before.as_bytes()),
        replacement,
    };
    let candidate_source = apply_edit_atom(source, &edit)?;
    Ok(MaterializedPrimitiveApplicationIR {
        primitive_id: primitive.primitive_id.clone(),
        source_relative_path: source_relative_path.to_path_buf(),
        predecessor_sha256: sha256(source.as_bytes()),
        candidate_sha256: sha256(candidate_source.as_bytes()),
        bound_roles: bindings.clone(),
        edit,
        candidate_source,
        requires_source_compile: primitive.source_compile_required,
        requires_public_observation: primitive.public_observation_required,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn verified_examples_from_edit(
    independent_context_id: &str,
    language_id: &str,
    source_extension: &str,
    source_relative_path: &Path,
    predecessor_source: &str,
    edit: &SourceEditAtom,
    public_verification_sha256: &str,
    compile_verification_sha256: &str,
) -> Result<Vec<VerifiedRepairExampleIR>, String> {
    let mut examples = Vec::new();
    collect_verified_examples(
        independent_context_id,
        language_id,
        source_extension,
        source_relative_path,
        predecessor_source,
        edit,
        public_verification_sha256,
        compile_verification_sha256,
        &mut examples,
    )?;
    Ok(examples)
}

#[allow(clippy::too_many_arguments)]
fn collect_verified_examples(
    independent_context_id: &str,
    language_id: &str,
    source_extension: &str,
    source_relative_path: &Path,
    predecessor_source: &str,
    edit: &SourceEditAtom,
    public_verification_sha256: &str,
    compile_verification_sha256: &str,
    output: &mut Vec<VerifiedRepairExampleIR>,
) -> Result<(), String> {
    match edit {
        SourceEditAtom::Replace {
            range, replacement, ..
        } => {
            let before = predecessor_source
                .get(range.start..range.end)
                .ok_or_else(|| "META_COMPILER_EXAMPLE_RANGE".to_string())?;
            let example = VerifiedRepairExampleIR::new(
                independent_context_id,
                language_id,
                source_extension,
                source_relative_path.to_path_buf(),
                before,
                replacement,
                vec!["REPLACE".to_string()],
                public_verification_sha256,
                compile_verification_sha256,
            );
            // A repair may be valid while falling outside the conservative
            // anti-unification grammar (for example it introduces a fresh
            // helper identifier).  Learning is optional and must never make
            // an already verified product repair fail.
            if let Ok(example) = example {
                output.push(example);
            }
        }
        SourceEditAtom::AtomicMultiEdit { edits } => {
            for child in edits {
                collect_verified_examples(
                    independent_context_id,
                    language_id,
                    source_extension,
                    source_relative_path,
                    predecessor_source,
                    child,
                    public_verification_sha256,
                    compile_verification_sha256,
                    output,
                )?;
            }
        }
        SourceEditAtom::Insert { .. }
        | SourceEditAtom::Delete { .. }
        | SourceEditAtom::Move { .. } => {
            // Insert/delete/move remain executable base atoms.  Their safe
            // anti-unification needs anchor context, so a bare atom is not
            // promoted as a context-free macro.
        }
    }
    Ok(())
}

pub fn induce_language_backend(
    candidate: LanguageBackendCandidateIR,
    evidence: BackendValidationEvidenceIR,
) -> Result<InducedLanguageBackendIR, String> {
    let language_id = normalize_language_id(&candidate.language_id)?;
    let extensions = sorted_unique(
        candidate
            .extensions
            .iter()
            .map(|extension| normalize_extension(extension))
            .collect::<Result<Vec<_>, _>>()?,
    );
    if extensions.is_empty() {
        return Err("META_COMPILER_BACKEND_EXTENSIONS_EMPTY".to_string());
    }
    let contexts = sorted_unique(evidence.independent_context_ids);
    if contexts.len() < MIN_INDEPENDENT_CONTEXTS
        || !evidence.source_roundtrip_passed
        || !evidence.utf8_byte_spans_stable
        || !evidence.no_op_roundtrip_passed
        || !evidence.source_compile_passed
        || !evidence.public_observation_passed
        || evidence.test_target_write_events != 0
    {
        return Err("META_COMPILER_BACKEND_VALIDATION".to_string());
    }
    validate_sha256(&evidence.compile_verification_sha256, "BACKEND_COMPILE")?;
    validate_sha256(&evidence.public_verification_sha256, "BACKEND_PUBLIC")?;
    if candidate.knowledge_universe_sha256 != META_COMPILER_KNOWLEDGE_UNIVERSE_SHA256 {
        return Err("META_COMPILER_BACKEND_KNOWLEDGE_UNIVERSE".to_string());
    }
    let required_mechanism_ids = sorted_unique(candidate.required_mechanism_ids);
    for required in ["compiler_runtime", "pattern_matching"] {
        if !required_mechanism_ids
            .iter()
            .any(|mechanism| mechanism == required)
        {
            return Err(format!("META_COMPILER_BACKEND_MECHANISM:{required}"));
        }
    }
    let known_nodes = META_COMPILER_KNOWLEDGE_MECHANISMS
        .iter()
        .map(|(_, node)| *node)
        .collect::<BTreeSet<_>>();
    if candidate
        .knowledge_node_sha256s
        .iter()
        .any(|node| !known_nodes.contains(node.as_str()))
    {
        return Err("META_COMPILER_BACKEND_KNOWLEDGE_NODE".to_string());
    }
    let backend_id = sha256(
        serde_json::to_vec(&(
            &language_id,
            &extensions,
            &candidate.parser_mode,
            &required_mechanism_ids,
            &candidate.knowledge_universe_sha256,
            &candidate.knowledge_node_sha256s,
            &contexts,
            &evidence.compile_verification_sha256,
            &evidence.public_verification_sha256,
        ))
        .map_err(|error| format!("META_COMPILER_BACKEND_SERIALIZE:{error}"))?
        .as_slice(),
    );
    let backend = InducedLanguageBackendIR {
        schema: INDUCED_LANGUAGE_BACKEND_SCHEMA.to_string(),
        backend_id,
        language_id,
        extensions,
        parser_mode: candidate.parser_mode,
        required_mechanism_ids,
        knowledge_universe_sha256: candidate.knowledge_universe_sha256,
        knowledge_node_sha256s: sorted_unique(candidate.knowledge_node_sha256s),
        independent_context_ids: contexts,
        compile_verification_sha256: evidence.compile_verification_sha256,
        public_verification_sha256: evidence.public_verification_sha256,
        promoted: true,
    };
    validate_induced_backend(&backend)?;
    Ok(backend)
}

fn validate_induced_backend(backend: &InducedLanguageBackendIR) -> Result<(), String> {
    if backend.schema != INDUCED_LANGUAGE_BACKEND_SCHEMA
        || !backend.promoted
        || backend.extensions.is_empty()
        || backend.independent_context_ids.len() < MIN_INDEPENDENT_CONTEXTS
        || backend.knowledge_universe_sha256 != META_COMPILER_KNOWLEDGE_UNIVERSE_SHA256
    {
        return Err("META_COMPILER_BACKEND_AUTHORITY".to_string());
    }
    validate_sha256(&backend.backend_id, "BACKEND_ID")?;
    validate_sha256(&backend.compile_verification_sha256, "BACKEND_COMPILE")?;
    validate_sha256(&backend.public_verification_sha256, "BACKEND_PUBLIC")?;
    Ok(())
}

fn registry_payload_sha256(registry: &MetaCompilerRegistryIR) -> Result<String, String> {
    Ok(sha256(
        serde_json::to_vec(&(
            &registry.schema,
            &registry.knowledge_universe_sha256,
            &registry.knowledge_mechanism_node_sha256s,
            &registry.verified_examples,
            &registry.edit_primitives,
            &registry.language_backends,
            &registry.unresolved_gaps,
        ))
        .map_err(|error| format!("META_COMPILER_REGISTRY_SERIALIZE:{error}"))?
        .as_slice(),
    ))
}

pub fn load_registry(path: &Path) -> Result<MetaCompilerRegistryIR, String> {
    if !path.exists() {
        return Ok(MetaCompilerRegistryIR::default());
    }
    let bytes = fs::read(path).map_err(|error| format!("META_COMPILER_REGISTRY_READ:{error}"))?;
    let registry: MetaCompilerRegistryIR = serde_json::from_slice(&bytes)
        .map_err(|error| format!("META_COMPILER_REGISTRY_PARSE:{error}"))?;
    registry.validate()?;
    Ok(registry)
}

pub fn persist_registry(path: &Path, registry: &MetaCompilerRegistryIR) -> Result<(), String> {
    registry.validate()?;
    let parent = path
        .parent()
        .ok_or_else(|| "META_COMPILER_REGISTRY_PARENT".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("META_COMPILER_REGISTRY_DIRECTORY:{error}"))?;
    let bytes = serde_json::to_vec_pretty(registry)
        .map_err(|error| format!("META_COMPILER_REGISTRY_SERIALIZE:{error}"))?;
    let temp = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("registry"),
        &sha256(&bytes)[..16]
    ));
    fs::write(&temp, &bytes).map_err(|error| format!("META_COMPILER_REGISTRY_WRITE:{error}"))?;
    for attempt in 0..4 {
        match fs::rename(&temp, path) {
            Ok(()) => return Ok(()),
            Err(error) if path.exists() => {
                fs::remove_file(path)
                    .map_err(|remove| format!("META_COMPILER_REGISTRY_REPLACE:{error}:{remove}"))?;
            }
            Err(error) if attempt == 3 => {
                let _ = fs::remove_file(&temp);
                return Err(format!("META_COMPILER_REGISTRY_RENAME:{error}"));
            }
            Err(_) => thread::sleep(Duration::from_millis(10 * (attempt + 1) as u64)),
        }
    }
    Err("META_COMPILER_REGISTRY_RENAME_EXHAUSTED".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(seed: &str) -> String {
        sha256(seed.as_bytes())
    }

    fn example(context: &str, path: &str, before: &str, after: &str) -> VerifiedRepairExampleIR {
        VerifiedRepairExampleIR::new(
            context,
            "python",
            "py",
            PathBuf::from(path),
            before,
            after,
            vec!["REPLACE".to_string()],
            digest(&format!("public:{context}")),
            digest(&format!("compile:{context}")),
        )
        .unwrap()
    }

    #[test]
    fn independently_verified_repairs_induce_and_execute_a_new_primitive() {
        let first = example(
            "repo-a",
            "src/policy.py",
            "return left_value + right_value",
            "return left_value - right_value",
        );
        let second = example(
            "repo-b",
            "lib/cost.py",
            "return gross_cost + refund_cost",
            "return gross_cost - refund_cost",
        );
        let primitive = induce_edit_primitive(&[first, second]).unwrap();
        let source =
            "def net(invoice_total, credit_total):\n    return invoice_total + credit_total\n";
        let application =
            materialize_induced_primitive(&primitive, Path::new("product/billing.py"), source)
                .unwrap();
        assert_eq!(
            application.candidate_source,
            "def net(invoice_total, credit_total):\n    return invoice_total - credit_total\n"
        );
        assert!(matches!(application.edit, SourceEditAtom::Replace { .. }));
        assert!(application.requires_source_compile);
        assert!(application.requires_public_observation);
    }

    #[test]
    fn learned_primitive_is_fail_closed_for_ambiguous_and_test_targets() {
        let primitive = induce_edit_primitive(&[
            example("a", "src/a.py", "return a + b", "return a - b"),
            example("b", "src/b.py", "return x + y", "return x - y"),
        ])
        .unwrap();
        let ambiguous = "def a(a,b): return a + b\ndef c(x,y): return x + y\n";
        assert_eq!(
            materialize_induced_primitive(&primitive, Path::new("src/live.py"), ambiguous)
                .unwrap_err(),
            "META_COMPILER_APPLICATION_AMBIGUOUS"
        );
        assert_eq!(
            materialize_induced_primitive(
                &primitive,
                Path::new("tests/test_live.py"),
                "return a + b"
            )
            .unwrap_err(),
            "META_COMPILER_APPLICATION_TEST_TARGET"
        );
    }

    #[test]
    fn registry_promotes_only_after_independent_verified_contexts() {
        let mut registry = MetaCompilerRegistryIR::default();
        assert!(registry
            .learn_verified_example(example("same", "src/a.py", "return a + b", "return a - b"))
            .unwrap()
            .is_empty());
        assert!(registry.edit_primitives.is_empty());
        let promoted = registry
            .learn_verified_example(example("other", "src/b.py", "return x + y", "return x - y"))
            .unwrap();
        assert_eq!(promoted.len(), 1);
        registry.validate().unwrap();
    }

    #[test]
    fn unknown_language_backend_requires_two_contexts_and_full_validation() {
        let candidate = LanguageBackendCandidateIR {
            language_id: "zig".to_string(),
            extensions: vec!["zig".to_string()],
            parser_mode: DynamicBackendParserMode::CompilerValidatedExactSpan,
            required_mechanism_ids: vec![
                "compiler_runtime".to_string(),
                "pattern_matching".to_string(),
            ],
            knowledge_universe_sha256: META_COMPILER_KNOWLEDGE_UNIVERSE_SHA256.to_string(),
            knowledge_node_sha256s: vec![
                META_COMPILER_KNOWLEDGE_MECHANISMS[1].1.to_string(),
                META_COMPILER_KNOWLEDGE_MECHANISMS[3].1.to_string(),
            ],
        };
        let mut evidence = BackendValidationEvidenceIR {
            independent_context_ids: vec!["repo-a".to_string()],
            source_roundtrip_passed: true,
            utf8_byte_spans_stable: true,
            no_op_roundtrip_passed: true,
            compile_verification_sha256: digest("zig-compile"),
            public_verification_sha256: digest("zig-public"),
            source_compile_passed: true,
            public_observation_passed: true,
            test_target_write_events: 0,
        };
        assert_eq!(
            induce_language_backend(candidate.clone(), evidence.clone()).unwrap_err(),
            "META_COMPILER_BACKEND_VALIDATION"
        );
        evidence.independent_context_ids.push("repo-b".to_string());
        let backend = induce_language_backend(candidate, evidence).unwrap();
        assert!(backend.promoted);
        assert_eq!(backend.extensions, vec!["zig"]);
    }

    #[test]
    fn two_verified_unknown_language_repairs_invent_backend_and_primitive() {
        let mut registry = MetaCompilerRegistryIR::default();
        let make = |context: &str, path: &str, before: &str, after: &str| {
            VerifiedRepairExampleIR::new(
                context,
                "zig",
                "zig",
                PathBuf::from(path),
                before,
                after,
                vec!["REPLACE".to_string()],
                digest(&format!("public:{context}")),
                digest(&format!("compile:{context}")),
            )
            .unwrap()
        };
        registry
            .learn_verified_example(make(
                "repo-a",
                "src/a.zig",
                "return a + b;",
                "return a - b;",
            ))
            .unwrap();
        registry
            .learn_verified_example(make(
                "repo-b",
                "src/b.zig",
                "return x + y;",
                "return x - y;",
            ))
            .unwrap();
        assert!(registry
            .backend_for_path(Path::new("src/new.zig"))
            .is_some());
        let candidates = registry.materialize_candidates(
            "zig",
            Path::new("src/new.zig"),
            "fn net(a: i32, b: i32) i32 { return a + b; }",
        );
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].candidate_source.contains("return a - b;"));
    }

    #[test]
    fn registry_round_trip_preserves_authority_hash() {
        let root =
            std::env::temp_dir().join(format!("b_core_meta_registry_{}", digest("roundtrip")));
        let path = root.join("registry.json");
        let mut registry = MetaCompilerRegistryIR::default();
        registry
            .learn_verified_example(example(
                "repo-a",
                "src/a.py",
                "return a + b",
                "return a - b",
            ))
            .unwrap();
        persist_registry(&path, &registry).unwrap();
        let loaded = load_registry(&path).unwrap();
        assert_eq!(loaded.registry_sha256, registry.registry_sha256);
        assert!(loaded.verified_examples[0].before_fragment.is_empty());
        assert!(loaded.verified_examples[0].after_fragment.is_empty());
        assert_eq!(
            loaded.verified_examples[0].before_template,
            registry.verified_examples[0].before_template
        );
        fs::remove_dir_all(root).unwrap();
    }
}
