//! Bounded long-horizon repository indexing and multi-file causal tracing.
//!
//! A repository is content-addressed once, then repair queries traverse the
//! resulting symbol/call/import graph. Query execution never rescans the full
//! catalog and never routes by repository or benchmark identity.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::repository_coding_knowledge::RepositoryLanguage;
use crate::self_repair_contract::sha256;

pub const REPOSITORY_HORIZON_SCHEMA: &str = "B_REPOSITORY_HORIZON_1";
pub const MAX_INDEX_FILES: usize = 8_192;
pub const MAX_INDEX_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_SOURCE_FILE_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_SYMBOLS_PER_FILE: usize = 2_048;
pub const MAX_REFERENCES_PER_FILE: usize = 8_192;
pub const MAX_TRACE_STEPS: usize = 256;
pub const MAX_TRACE_FRONTIER: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepositoryFileKind {
    Source,
    Test,
    Manifest,
    Configuration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepositoryEdgeKind {
    Imports,
    Calls,
    TestReferences,
    Configures,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryFileNodeIR {
    pub node_id: usize,
    pub relative_path: PathBuf,
    pub language: RepositoryLanguage,
    pub kind: RepositoryFileKind,
    pub content_sha256: String,
    pub byte_count: usize,
    pub defined_symbols: Vec<String>,
    pub referenced_symbols: Vec<String>,
    pub import_targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RepositoryCausalEdgeIR {
    pub from_node: usize,
    pub to_node: usize,
    pub kind: RepositoryEdgeKind,
    pub evidence_symbol: String,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryCausalGraphIR {
    pub schema: String,
    pub root_sha256: String,
    pub files: Vec<RepositoryFileNodeIR>,
    pub edges: Vec<RepositoryCausalEdgeIR>,
    pub indexed_files: usize,
    pub indexed_bytes: u64,
    pub skipped_files: usize,
    pub duplicate_symbol_definitions: usize,
    pub initial_catalog_scans: u64,
    pub full_catalog_rescans: u64,
    pub external_llm_calls: u64,
    pub network_reads: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryHorizonBuildRequestIR {
    pub schema: String,
    pub repository_root: PathBuf,
    pub max_files: usize,
    pub max_total_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryCausalTraceRequestIR {
    pub schema: String,
    pub entry_symbols: Vec<String>,
    pub evidence_symbols: Vec<String>,
    pub path_hints: Vec<PathBuf>,
    pub max_steps: usize,
    pub max_frontier: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryCausalPathIR {
    pub file_nodes: Vec<usize>,
    pub relative_paths: Vec<PathBuf>,
    pub edge_kinds: Vec<RepositoryEdgeKind>,
    pub evidence_symbols: Vec<String>,
    pub depth: usize,
    pub path_score: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryCausalTraceIR {
    pub schema: String,
    pub causal_paths: Vec<RepositoryCausalPathIR>,
    pub selected_file_nodes: Vec<usize>,
    pub selected_relative_paths: Vec<PathBuf>,
    pub entry_anchor_nodes: Vec<usize>,
    pub evidence_anchor_nodes: Vec<usize>,
    pub visited_files: usize,
    pub peak_frontier: usize,
    pub deepest_path: usize,
    pub unresolved_entries: Vec<String>,
    pub unresolved_evidence: Vec<String>,
    pub budget_exhausted: bool,
    pub full_catalog_rescans: u64,
    pub task_identity_routing_events: u64,
    pub repository_identity_routing_events: u64,
    pub external_llm_calls: u64,
    pub network_reads: u64,
}

fn language_for_path(path: &Path) -> RepositoryLanguage {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("rs") => RepositoryLanguage::Rust,
        Some("py") => RepositoryLanguage::Python,
        Some("ts" | "tsx") => RepositoryLanguage::TypeScript,
        Some("js" | "jsx" | "mjs" | "cjs") => RepositoryLanguage::JavaScript,
        Some("go") => RepositoryLanguage::Go,
        _ => RepositoryLanguage::Unknown,
    }
}

fn file_kind(path: &Path, language: RepositoryLanguage) -> RepositoryFileKind {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let path_text = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    if ["cargo.toml", "package.json", "go.mod", "pyproject.toml"].contains(&name.as_str()) {
        RepositoryFileKind::Manifest
    } else if [
        "tsconfig.json",
        "rust-toolchain.toml",
        "rust-toolchain",
        "package-lock.json",
        "go.sum",
    ]
    .contains(&name.as_str())
    {
        RepositoryFileKind::Configuration
    } else if path_text.contains("/tests/")
        || path_text.contains("/test/")
        || path_text.starts_with("tests/")
        || path_text.starts_with("test/")
        || name.starts_with("test_")
        || name.ends_with("_test.go")
        || name.ends_with(".test.ts")
        || name.ends_with(".test.js")
        || name.ends_with(".spec.ts")
        || name.ends_with(".spec.js")
        || (language == RepositoryLanguage::Rust && path_text.contains("/benches/"))
    {
        RepositoryFileKind::Test
    } else {
        RepositoryFileKind::Source
    }
}

fn supported_repository_file(path: &Path) -> bool {
    if language_for_path(path).supported() {
        return true;
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    [
        "cargo.toml",
        "package.json",
        "package-lock.json",
        "tsconfig.json",
        "go.mod",
        "go.sum",
        "pyproject.toml",
        "rust-toolchain.toml",
        "rust-toolchain",
    ]
    .contains(&name.as_str())
}

fn excluded_directory(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        ".git"
            | ".hg"
            | ".svn"
            | "target"
            | "node_modules"
            | "vendor"
            | "dist"
            | "build"
            | ".next"
            | ".cache"
            | "__pycache__"
    )
}

fn repository_relative_path_valid(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn collect_repository_files(
    canonical_root: &Path,
    max_files: usize,
) -> Result<(Vec<PathBuf>, usize), String> {
    let mut queue = VecDeque::from([canonical_root.to_path_buf()]);
    let mut files = Vec::new();
    let mut skipped = 0usize;
    while let Some(directory) = queue.pop_front() {
        let mut entries = fs::read_dir(&directory)
            .map_err(|error| format!("REPOSITORY_HORIZON_READ_DIR:{error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("REPOSITORY_HORIZON_READ_ENTRY:{error}"))?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let metadata = fs::symlink_metadata(entry.path())
                .map_err(|error| format!("REPOSITORY_HORIZON_METADATA:{error}"))?;
            if metadata.file_type().is_symlink() {
                skipped = skipped.saturating_add(1);
                continue;
            }
            if metadata.is_dir() {
                let name = entry.file_name();
                if name.to_str().is_some_and(excluded_directory) {
                    skipped = skipped.saturating_add(1);
                } else {
                    queue.push_back(entry.path());
                }
                continue;
            }
            if !metadata.is_file() || !supported_repository_file(&entry.path()) {
                skipped = skipped.saturating_add(1);
                continue;
            }
            if metadata.len() > MAX_SOURCE_FILE_BYTES {
                skipped = skipped.saturating_add(1);
                continue;
            }
            let canonical = fs::canonicalize(entry.path())
                .map_err(|error| format!("REPOSITORY_HORIZON_CANONICAL_FILE:{error}"))?;
            if !canonical.starts_with(canonical_root) {
                return Err("REPOSITORY_HORIZON_PATH_ESCAPE".to_string());
            }
            let relative = canonical
                .strip_prefix(canonical_root)
                .map_err(|_| "REPOSITORY_HORIZON_STRIP_ROOT".to_string())?
                .to_path_buf();
            if !repository_relative_path_valid(&relative) {
                return Err("REPOSITORY_HORIZON_RELATIVE_PATH".to_string());
            }
            files.push(relative);
            if files.len() > max_files {
                return Err("REPOSITORY_HORIZON_FILE_BUDGET".to_string());
            }
        }
    }
    files.sort();
    Ok((files, skipped))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LexicalToken {
    value: String,
    followed_by_call: bool,
}

fn lexical_tokens(source: &str) -> Vec<LexicalToken> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0usize;
    let mut block_comment_depth = 0usize;
    while index < bytes.len() {
        if block_comment_depth > 0 {
            if bytes.get(index..index + 2) == Some(b"/*") {
                block_comment_depth = block_comment_depth.saturating_add(1);
                index += 2;
            } else if bytes.get(index..index + 2) == Some(b"*/") {
                block_comment_depth = block_comment_depth.saturating_sub(1);
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        if bytes.get(index..index + 2) == Some(b"/*") {
            block_comment_depth = 1;
            index += 2;
            continue;
        }
        if bytes.get(index..index + 2) == Some(b"//") || bytes[index] == b'#' {
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if matches!(bytes[index], b'\'' | b'"' | b'`') {
            let quote = bytes[index];
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else if bytes[index] == quote {
                    index += 1;
                    break;
                } else {
                    index += 1;
                }
            }
            continue;
        }
        let identifier_start = bytes[index].is_ascii_alphabetic() || bytes[index] == b'_';
        if !identifier_start {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
        {
            index += 1;
        }
        let mut next = index;
        while next < bytes.len() && bytes[next].is_ascii_whitespace() {
            next += 1;
        }
        tokens.push(LexicalToken {
            value: source[start..index].to_string(),
            followed_by_call: bytes.get(next) == Some(&b'('),
        });
    }
    tokens
}

fn definition_prefixes(language: RepositoryLanguage) -> &'static [&'static str] {
    match language {
        RepositoryLanguage::Rust => &[
            "fn", "struct", "enum", "trait", "type", "const", "static", "mod",
        ],
        RepositoryLanguage::Python => &["def", "class"],
        RepositoryLanguage::TypeScript => &["function", "class", "interface", "type", "enum"],
        RepositoryLanguage::JavaScript => &["function", "class"],
        RepositoryLanguage::Go => &["func", "type", "const", "var"],
        RepositoryLanguage::Unknown => &[],
    }
}

fn extract_symbols_and_references(
    source: &str,
    language: RepositoryLanguage,
) -> (Vec<String>, Vec<String>) {
    let tokens = lexical_tokens(source);
    let prefixes = definition_prefixes(language);
    let mut definitions = BTreeSet::new();
    let mut references = BTreeSet::new();
    for (index, token) in tokens.iter().enumerate() {
        if prefixes.contains(&token.value.as_str()) {
            if let Some(next) = tokens.get(index + 1) {
                if !prefixes.contains(&next.value.as_str()) {
                    definitions.insert(next.value.clone());
                }
            }
        }
        if token.followed_by_call
            && !matches!(
                token.value.as_str(),
                "if" | "for" | "while" | "match" | "switch" | "catch" | "return" | "func"
            )
        {
            references.insert(token.value.clone());
        }
    }
    (
        definitions.into_iter().take(MAX_SYMBOLS_PER_FILE).collect(),
        references
            .into_iter()
            .take(MAX_REFERENCES_PER_FILE)
            .collect(),
    )
}

fn extract_quoted_values(line: &str) -> Vec<String> {
    let mut output = Vec::new();
    let bytes = line.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if !matches!(bytes[index], b'\'' | b'"') {
            index += 1;
            continue;
        }
        let quote = bytes[index];
        let start = index + 1;
        index += 1;
        while index < bytes.len() && bytes[index] != quote {
            if bytes[index] == b'\\' {
                index = (index + 2).min(bytes.len());
            } else {
                index += 1;
            }
        }
        if index <= bytes.len() && start < index {
            output.push(line[start..index].to_string());
        }
        index = (index + 1).min(bytes.len());
    }
    output
}

fn extract_import_targets(source: &str, language: RepositoryLanguage) -> Vec<String> {
    let mut imports = BTreeSet::new();
    for line in source.lines().map(str::trim) {
        let lower = line.to_ascii_lowercase();
        let relevant = match language {
            RepositoryLanguage::Rust => {
                lower.starts_with("mod ")
                    || lower.starts_with("use ")
                    || lower.starts_with("pub use ")
            }
            RepositoryLanguage::Python => {
                lower.starts_with("import ") || lower.starts_with("from ")
            }
            RepositoryLanguage::TypeScript | RepositoryLanguage::JavaScript => {
                lower.starts_with("import ")
                    || lower.starts_with("export ")
                    || lower.contains("require(")
            }
            RepositoryLanguage::Go => lower.starts_with("import ") || lower.starts_with('"'),
            RepositoryLanguage::Unknown => false,
        };
        if !relevant {
            continue;
        }
        for value in extract_quoted_values(line) {
            if value.len() <= 512 {
                imports.insert(value);
            }
        }
        if language == RepositoryLanguage::Rust && lower.starts_with("mod ") {
            let value = line
                .trim_start_matches("pub ")
                .trim_start_matches("mod ")
                .trim_end_matches(';')
                .trim();
            if !value.is_empty() {
                imports.insert(value.to_string());
            }
        }
    }
    imports.into_iter().collect()
}

fn resolve_import_candidate(
    from: &RepositoryFileNodeIR,
    import: &str,
    paths: &BTreeMap<String, usize>,
) -> Option<usize> {
    let normalized = import.replace('\\', "/");
    let parent = from.relative_path.parent().unwrap_or_else(|| Path::new(""));
    let mut candidates = Vec::new();
    if normalized.starts_with('.') {
        let joined = parent.join(&normalized);
        candidates.push(joined.clone());
        for extension in ["rs", "ts", "tsx", "js", "jsx", "go", "py"] {
            candidates.push(joined.with_extension(extension));
        }
        for extension in ["ts", "tsx", "js", "jsx"] {
            candidates.push(joined.join(format!("index.{extension}")));
        }
    } else if from.language == RepositoryLanguage::Rust && !normalized.contains('/') {
        candidates.push(parent.join(format!("{normalized}.rs")));
        candidates.push(parent.join(&normalized).join("mod.rs"));
    }
    candidates.into_iter().find_map(|candidate| {
        let key = candidate.to_string_lossy().replace('\\', "/");
        paths.get(&key).copied()
    })
}

fn graph_root_hash(files: &[RepositoryFileNodeIR], edges: &[RepositoryCausalEdgeIR]) -> String {
    let bytes = serde_json::to_vec(&(files, edges)).unwrap_or_default();
    sha256(&bytes)
}

/// Build a bounded, deterministic repository graph from regular UTF-8 files.
pub fn build_repository_causal_graph(
    request: &RepositoryHorizonBuildRequestIR,
) -> Result<RepositoryCausalGraphIR, String> {
    if request.schema != REPOSITORY_HORIZON_SCHEMA
        || !request.repository_root.is_absolute()
        || request.max_files == 0
        || request.max_files > MAX_INDEX_FILES
        || request.max_total_bytes == 0
        || request.max_total_bytes > MAX_INDEX_BYTES
    {
        return Err("REPOSITORY_HORIZON_BUILD_ENVELOPE".to_string());
    }
    let canonical_root = fs::canonicalize(&request.repository_root)
        .map_err(|error| format!("REPOSITORY_HORIZON_CANONICAL_ROOT:{error}"))?;
    if !canonical_root.is_dir() {
        return Err("REPOSITORY_HORIZON_ROOT_NOT_DIRECTORY".to_string());
    }
    let (paths, mut skipped_files) = collect_repository_files(&canonical_root, request.max_files)?;
    let mut indexed_bytes = 0u64;
    let mut files = Vec::new();
    for relative_path in paths {
        let absolute = canonical_root.join(&relative_path);
        let bytes =
            fs::read(&absolute).map_err(|error| format!("REPOSITORY_HORIZON_READ_FILE:{error}"))?;
        indexed_bytes = indexed_bytes
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| "REPOSITORY_HORIZON_BYTE_OVERFLOW".to_string())?;
        if indexed_bytes > request.max_total_bytes {
            return Err("REPOSITORY_HORIZON_BYTE_BUDGET".to_string());
        }
        let Ok(source) = std::str::from_utf8(&bytes) else {
            skipped_files = skipped_files.saturating_add(1);
            continue;
        };
        let language = language_for_path(&relative_path);
        let kind = file_kind(&relative_path, language);
        let (defined_symbols, referenced_symbols) =
            extract_symbols_and_references(source, language);
        let import_targets = extract_import_targets(source, language);
        files.push(RepositoryFileNodeIR {
            node_id: files.len(),
            relative_path,
            language,
            kind,
            content_sha256: sha256(&bytes),
            byte_count: bytes.len(),
            defined_symbols,
            referenced_symbols,
            import_targets,
        });
    }

    let mut owners = BTreeMap::<String, Vec<usize>>::new();
    let mut paths = BTreeMap::<String, usize>::new();
    for file in &files {
        paths.insert(
            file.relative_path.to_string_lossy().replace('\\', "/"),
            file.node_id,
        );
        for symbol in &file.defined_symbols {
            owners.entry(symbol.clone()).or_default().push(file.node_id);
        }
    }
    let duplicate_symbol_definitions = owners
        .values()
        .filter(|owner_nodes| owner_nodes.len() > 1)
        .count();
    let mut edges = BTreeSet::new();
    for file in &files {
        for import in &file.import_targets {
            if let Some(target) = resolve_import_candidate(file, import, &paths) {
                if target != file.node_id {
                    edges.insert(RepositoryCausalEdgeIR {
                        from_node: file.node_id,
                        to_node: target,
                        kind: RepositoryEdgeKind::Imports,
                        evidence_symbol: import.clone(),
                        confidence_millis: 1_000,
                    });
                }
            }
        }
        for symbol in &file.referenced_symbols {
            let Some(owner_nodes) = owners.get(symbol) else {
                continue;
            };
            let confidence = if owner_nodes.len() == 1 { 950 } else { 500 };
            for owner in owner_nodes {
                if *owner == file.node_id {
                    continue;
                }
                edges.insert(RepositoryCausalEdgeIR {
                    from_node: file.node_id,
                    to_node: *owner,
                    kind: if file.kind == RepositoryFileKind::Test {
                        RepositoryEdgeKind::TestReferences
                    } else {
                        RepositoryEdgeKind::Calls
                    },
                    evidence_symbol: symbol.clone(),
                    confidence_millis: confidence,
                });
            }
        }
    }
    let edges = edges.into_iter().collect::<Vec<_>>();
    Ok(RepositoryCausalGraphIR {
        schema: REPOSITORY_HORIZON_SCHEMA.to_string(),
        root_sha256: graph_root_hash(&files, &edges),
        indexed_files: files.len(),
        indexed_bytes,
        files,
        edges,
        skipped_files,
        duplicate_symbol_definitions,
        initial_catalog_scans: 1,
        full_catalog_rescans: 0,
        external_llm_calls: 0,
        network_reads: 0,
    })
}

fn anchor_nodes(
    graph: &RepositoryCausalGraphIR,
    symbols: &[String],
    path_hints: &[PathBuf],
) -> (Vec<usize>, Vec<String>) {
    let normalized_symbols = symbols
        .iter()
        .filter_map(|symbol| {
            let value = symbol.trim();
            (!value.is_empty() && value.len() <= 256).then(|| value.to_string())
        })
        .collect::<BTreeSet<_>>();
    let normalized_paths = path_hints
        .iter()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .collect::<BTreeSet<_>>();
    let mut found_symbols = BTreeSet::new();
    let mut nodes = BTreeSet::new();
    for file in &graph.files {
        let path = file.relative_path.to_string_lossy().replace('\\', "/");
        if normalized_paths.contains(&path) {
            nodes.insert(file.node_id);
        }
    }
    for symbol in &normalized_symbols {
        let definition_nodes = graph
            .files
            .iter()
            .filter(|file| file.defined_symbols.contains(symbol))
            .map(|file| file.node_id)
            .collect::<Vec<_>>();
        if definition_nodes.is_empty() {
            let reference_nodes = graph
                .files
                .iter()
                .filter(|file| file.referenced_symbols.contains(symbol))
                .map(|file| file.node_id)
                .collect::<Vec<_>>();
            if !reference_nodes.is_empty() {
                nodes.extend(reference_nodes);
                found_symbols.insert(symbol.clone());
            }
        } else {
            nodes.extend(definition_nodes);
            found_symbols.insert(symbol.clone());
        }
    }
    (
        nodes.into_iter().collect(),
        normalized_symbols
            .difference(&found_symbols)
            .cloned()
            .collect(),
    )
}

fn edge_cost(edge: &RepositoryCausalEdgeIR) -> u64 {
    let kind_cost = match edge.kind {
        RepositoryEdgeKind::TestReferences => 1u64,
        RepositoryEdgeKind::Calls => 2,
        RepositoryEdgeKind::Imports => 3,
        RepositoryEdgeKind::Configures => 4,
    };
    kind_cost
        .saturating_mul(1_000)
        .saturating_add(1_000u64.saturating_sub(u64::from(edge.confidence_millis)))
}

/// Find bounded causal paths between public entry anchors and failure evidence
/// anchors without rescanning repository contents.
pub fn trace_repository_causality(
    graph: &RepositoryCausalGraphIR,
    request: &RepositoryCausalTraceRequestIR,
) -> Result<RepositoryCausalTraceIR, String> {
    if graph.schema != REPOSITORY_HORIZON_SCHEMA
        || request.schema != REPOSITORY_HORIZON_SCHEMA
        || graph.files.len() != graph.indexed_files
        || request.entry_symbols.is_empty() && request.path_hints.is_empty()
        || request.evidence_symbols.is_empty()
        || request.max_steps == 0
        || request.max_steps > MAX_TRACE_STEPS
        || request.max_frontier == 0
        || request.max_frontier > MAX_TRACE_FRONTIER
    {
        return Err("REPOSITORY_CAUSAL_TRACE_ENVELOPE".to_string());
    }
    let (entry_anchor_nodes, unresolved_entries) =
        anchor_nodes(graph, &request.entry_symbols, &request.path_hints);
    let (evidence_anchor_nodes, unresolved_evidence) =
        anchor_nodes(graph, &request.evidence_symbols, &[]);
    let evidence_set = evidence_anchor_nodes
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut outgoing = vec![Vec::<usize>::new(); graph.files.len()];
    for (edge_index, edge) in graph.edges.iter().enumerate() {
        if edge.from_node >= graph.files.len() || edge.to_node >= graph.files.len() {
            return Err("REPOSITORY_CAUSAL_EDGE_RANGE".to_string());
        }
        outgoing[edge.from_node].push(edge_index);
    }
    for edges in &mut outgoing {
        edges.sort_by_key(|edge_index| {
            let edge = &graph.edges[*edge_index];
            (
                edge_cost(edge),
                edge.to_node,
                edge.kind,
                edge.evidence_symbol.clone(),
            )
        });
    }

    let mut distance = vec![u64::MAX; graph.files.len()];
    let mut depth = vec![usize::MAX; graph.files.len()];
    let mut predecessor = vec![None::<(usize, usize)>; graph.files.len()];
    let mut frontier = BinaryHeap::<Reverse<(u64, usize, usize)>>::new();
    for entry in &entry_anchor_nodes {
        distance[*entry] = 0;
        depth[*entry] = 0;
        frontier.push(Reverse((0, 0, *entry)));
    }
    let mut visited = BTreeSet::new();
    let mut peak_frontier = frontier.len();
    let mut budget_exhausted = false;
    let mut reached_evidence = BTreeSet::new();
    while let Some(Reverse((cost, current_depth, node))) = frontier.pop() {
        if visited.contains(&node) || cost != distance[node] {
            continue;
        }
        if visited.len() >= request.max_frontier {
            budget_exhausted = true;
            break;
        }
        visited.insert(node);
        if evidence_set.contains(&node) {
            reached_evidence.insert(node);
            if reached_evidence == evidence_set {
                break;
            }
        }
        if current_depth >= request.max_steps {
            budget_exhausted = true;
            continue;
        }
        for edge_index in &outgoing[node] {
            let edge = &graph.edges[*edge_index];
            let next = edge.to_node;
            let next_depth = current_depth.saturating_add(1);
            let next_cost = cost.saturating_add(edge_cost(edge));
            if (next_cost, next_depth) < (distance[next], depth[next]) {
                distance[next] = next_cost;
                depth[next] = next_depth;
                predecessor[next] = Some((node, *edge_index));
                frontier.push(Reverse((next_cost, next_depth, next)));
            }
        }
        peak_frontier = peak_frontier.max(frontier.len());
    }

    let mut causal_paths = Vec::new();
    let mut selected = BTreeSet::new();
    for evidence in reached_evidence {
        let mut nodes = vec![evidence];
        let mut edge_indices = Vec::new();
        let mut cursor = evidence;
        while let Some((previous, edge_index)) = predecessor[cursor] {
            edge_indices.push(edge_index);
            nodes.push(previous);
            cursor = previous;
        }
        nodes.reverse();
        edge_indices.reverse();
        selected.extend(nodes.iter().copied());
        let path_score = distance[evidence];
        causal_paths.push(RepositoryCausalPathIR {
            relative_paths: nodes
                .iter()
                .map(|node| graph.files[*node].relative_path.clone())
                .collect(),
            edge_kinds: edge_indices
                .iter()
                .map(|edge| graph.edges[*edge].kind)
                .collect(),
            evidence_symbols: edge_indices
                .iter()
                .map(|edge| graph.edges[*edge].evidence_symbol.clone())
                .collect(),
            depth: edge_indices.len(),
            file_nodes: nodes,
            path_score,
        });
    }
    causal_paths.sort_by_key(|path| (path.path_score, path.relative_paths.clone()));
    let selected_file_nodes = selected.into_iter().collect::<Vec<_>>();
    let selected_relative_paths = selected_file_nodes
        .iter()
        .map(|node| graph.files[*node].relative_path.clone())
        .collect();
    let deepest_path = causal_paths
        .iter()
        .map(|path| path.depth)
        .max()
        .unwrap_or(0);
    Ok(RepositoryCausalTraceIR {
        schema: REPOSITORY_HORIZON_SCHEMA.to_string(),
        causal_paths,
        selected_file_nodes,
        selected_relative_paths,
        entry_anchor_nodes,
        evidence_anchor_nodes,
        visited_files: visited.len(),
        peak_frontier,
        deepest_path,
        unresolved_entries,
        unresolved_evidence,
        budget_exhausted,
        full_catalog_rescans: 0,
        task_identity_routing_events: 0,
        repository_identity_routing_events: 0,
        external_llm_calls: 0,
        network_reads: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_repository(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "b-core-horizon-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(path.join("src")).expect("create repository");
        path
    }

    #[test]
    fn long_horizon_trace_crosses_forty_eight_files_without_catalog_rescan() {
        let root = temp_repository("long");
        for index in 0..49usize {
            let name = if index == 0 {
                "public_entry".to_string()
            } else if index == 48 {
                "defective_leaf".to_string()
            } else {
                format!("layer_{index:02}")
            };
            let body = if index == 48 {
                "value - 1".to_string()
            } else {
                let next = if index + 1 == 48 {
                    "defective_leaf".to_string()
                } else {
                    format!("layer_{:02}", index + 1)
                };
                format!("{next}(value)")
            };
            fs::write(
                root.join("src").join(format!("layer_{index:02}.rs")),
                format!("pub fn {name}(value: i64) -> i64 {{ {body} }}\n"),
            )
            .expect("write chain file");
        }
        for index in 0..64usize {
            fs::write(
                root.join("src").join(format!("decoy_{index:02}.rs")),
                format!("pub fn decoy_{index:02}(value: i64) -> i64 {{ value }}\n"),
            )
            .expect("write decoy");
        }
        let graph = build_repository_causal_graph(&RepositoryHorizonBuildRequestIR {
            schema: REPOSITORY_HORIZON_SCHEMA.to_string(),
            repository_root: fs::canonicalize(&root).expect("canonical root"),
            max_files: 256,
            max_total_bytes: 8 * 1024 * 1024,
        })
        .expect("build graph");
        assert_eq!(graph.indexed_files, 113);
        let trace = trace_repository_causality(
            &graph,
            &RepositoryCausalTraceRequestIR {
                schema: REPOSITORY_HORIZON_SCHEMA.to_string(),
                entry_symbols: vec!["public_entry".to_string()],
                evidence_symbols: vec!["defective_leaf".to_string()],
                path_hints: Vec::new(),
                max_steps: 64,
                max_frontier: 128,
            },
        )
        .expect("trace graph");
        assert_eq!(trace.causal_paths.len(), 1);
        assert_eq!(trace.deepest_path, 48);
        assert_eq!(trace.selected_relative_paths.len(), 49);
        assert!(trace.visited_files <= 49);
        assert_eq!(trace.full_catalog_rescans, 0);
        assert!(!trace.budget_exhausted);
        fs::remove_dir_all(root).expect("remove repository");
    }

    #[test]
    fn test_reference_and_cross_file_calls_form_one_causal_path() {
        let root = temp_repository("test-call");
        fs::create_dir_all(root.join("tests")).expect("tests");
        fs::write(
            root.join("src/public.rs"),
            "pub fn public_api(v: i64) -> i64 { normalize(v) }\n",
        )
        .expect("public");
        fs::write(
            root.join("src/internal.rs"),
            "pub fn normalize(v: i64) -> i64 { unstable_order(v) }\n",
        )
        .expect("internal");
        fs::write(
            root.join("src/order.rs"),
            "pub fn unstable_order(v: i64) -> i64 { v }\n",
        )
        .expect("order");
        fs::write(
            root.join("tests/public_test.rs"),
            "fn regression() { assert_eq!(public_api(1), 1); }\n",
        )
        .expect("test");
        let graph = build_repository_causal_graph(&RepositoryHorizonBuildRequestIR {
            schema: REPOSITORY_HORIZON_SCHEMA.to_string(),
            repository_root: fs::canonicalize(&root).expect("canonical root"),
            max_files: 32,
            max_total_bytes: 1024 * 1024,
        })
        .expect("graph");
        let trace = trace_repository_causality(
            &graph,
            &RepositoryCausalTraceRequestIR {
                schema: REPOSITORY_HORIZON_SCHEMA.to_string(),
                entry_symbols: vec!["regression".to_string()],
                evidence_symbols: vec!["unstable_order".to_string()],
                path_hints: Vec::new(),
                max_steps: 8,
                max_frontier: 16,
            },
        )
        .expect("trace");
        assert_eq!(trace.deepest_path, 3);
        assert_eq!(
            trace.causal_paths[0].edge_kinds[0],
            RepositoryEdgeKind::TestReferences
        );
        assert_eq!(trace.selected_relative_paths.len(), 4);
        fs::remove_dir_all(root).expect("remove repository");
    }

    #[test]
    fn symlinks_and_build_caches_never_enter_the_graph() {
        let root = temp_repository("boundary");
        fs::create_dir_all(root.join("target/cache")).expect("cache");
        fs::write(root.join("src/live.rs"), "pub fn live() {}\n").expect("live");
        fs::write(
            root.join("target/cache/generated.rs"),
            "pub fn hidden() {}\n",
        )
        .expect("hidden");
        let graph = build_repository_causal_graph(&RepositoryHorizonBuildRequestIR {
            schema: REPOSITORY_HORIZON_SCHEMA.to_string(),
            repository_root: fs::canonicalize(&root).expect("canonical root"),
            max_files: 16,
            max_total_bytes: 1024 * 1024,
        })
        .expect("graph");
        assert_eq!(graph.indexed_files, 1);
        assert_eq!(graph.files[0].relative_path, PathBuf::from("src/live.rs"));
        fs::remove_dir_all(root).expect("remove repository");
    }
}
