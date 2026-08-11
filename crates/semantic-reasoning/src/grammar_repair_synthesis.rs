//! Bounded grammar composition for explicit Rust implementation holes.
//!
//! This is intentionally an enumerative synthesizer, not a bag of defect-
//! specific patches. It extracts callable types and bindings from the AST,
//! composes expressions from a small language-independent basis (values,
//! calls, unary/binary relations, constructors, and conditionals), and leaves
//! semantic selection to compile and public-test observations.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use serde::{Deserialize, Serialize};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Block, FnArg, ImplItem, Item, Pat, ReturnType};

use crate::self_repair_contract::sha256;
use crate::structural_source_repair::{
    synthesize_structural_repair, ByteRange, StructuralRepairProgram,
};

const MAX_GRAMMAR_CANDIDATES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarRepairCandidate {
    pub relative_path: PathBuf,
    pub predecessor_sha256: String,
    pub candidate_source: String,
    pub candidate_sha256: String,
    pub transformation: String,
    pub solution_strategy: String,
    pub consequence_predictions: Vec<String>,
    pub predicted_value: u16,
    pub structural_repair_program: StructuralRepairProgram,
    pub grammar_expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TypedBinding {
    name: String,
    type_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CallableSignature {
    callable: String,
    short_name: String,
    inputs: Vec<TypedBinding>,
    output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Hole {
    callable: CallableSignature,
    kind: String,
    range: ByteRange,
}

fn normalized_tokens<T: ToTokens>(value: &T) -> String {
    value.to_token_stream().to_string().replace(' ', "")
}

fn return_type(output: &ReturnType) -> String {
    match output {
        ReturnType::Default => "()".to_string(),
        ReturnType::Type(_, value) => normalized_tokens(value.as_ref()),
    }
}

fn typed_inputs(inputs: &syn::punctuated::Punctuated<FnArg, syn::Token![,]>) -> Vec<TypedBinding> {
    inputs
        .iter()
        .filter_map(|input| match input {
            FnArg::Typed(typed) => match typed.pat.as_ref() {
                Pat::Ident(identifier) => Some(TypedBinding {
                    name: identifier.ident.to_string(),
                    type_name: normalized_tokens(typed.ty.as_ref()),
                }),
                _ => None,
            },
            FnArg::Receiver(_) => None,
        })
        .collect()
}

fn qualified(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}::{name}")
    }
}

fn attributes_mark_test(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("test")
            || (attribute.path().is_ident("cfg")
                && normalized_tokens(&attribute.meta).contains("test"))
    })
}

fn collect_callables(items: &[Item], prefix: &str, output: &mut Vec<(CallableSignature, Block)>) {
    for item in items {
        match item {
            Item::Fn(function) if !attributes_mark_test(&function.attrs) => {
                output.push((
                    CallableSignature {
                        callable: qualified(prefix, &function.sig.ident.to_string()),
                        short_name: function.sig.ident.to_string(),
                        inputs: typed_inputs(&function.sig.inputs),
                        output: return_type(&function.sig.output),
                    },
                    (*function.block).clone(),
                ));
            }
            Item::Mod(module) => {
                if module.ident != "tests" && !attributes_mark_test(&module.attrs) {
                    if let Some((_, nested)) = &module.content {
                        collect_callables(
                            nested,
                            &qualified(prefix, &module.ident.to_string()),
                            output,
                        );
                    }
                }
            }
            Item::Impl(implementation) => {
                let owner = qualified(prefix, &normalized_tokens(implementation.self_ty.as_ref()));
                for member in &implementation.items {
                    if let ImplItem::Fn(method) = member {
                        if !attributes_mark_test(&method.attrs) {
                            output.push((
                                CallableSignature {
                                    callable: qualified(&owner, &method.sig.ident.to_string()),
                                    short_name: method.sig.ident.to_string(),
                                    inputs: typed_inputs(&method.sig.inputs),
                                    output: return_type(&method.sig.output),
                                },
                                method.block.clone(),
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(index + 1);
        }
    }
    starts
}

fn line_column_offset(
    source: &str,
    starts: &[usize],
    location: proc_macro2::LineColumn,
) -> Option<usize> {
    let line_start = *starts.get(location.line.checked_sub(1)?)?;
    let line_end = source[line_start..]
        .find('\n')
        .map(|offset| line_start + offset)
        .unwrap_or(source.len());
    let line = &source[line_start..line_end];
    let byte_column = if location.column == 0 {
        0
    } else {
        line.char_indices()
            .nth(location.column)
            .map(|(offset, _)| offset)
            .unwrap_or(line.len())
    };
    let result = line_start + byte_column;
    (result <= source.len() && source.is_char_boundary(result)).then_some(result)
}

struct HoleVisitor<'a> {
    source: &'a str,
    line_starts: &'a [usize],
    callable: &'a CallableSignature,
    holes: Vec<Hole>,
}

impl<'ast> Visit<'ast> for HoleVisitor<'_> {
    fn visit_expr_macro(&mut self, expression: &'ast syn::ExprMacro) {
        let kind = if expression.mac.path.is_ident("todo") {
            Some("TODO")
        } else if expression.mac.path.is_ident("unimplemented") {
            Some("UNIMPLEMENTED")
        } else {
            None
        };
        if let Some(kind) = kind {
            let span = expression.span();
            if let (Some(start), Some(end)) = (
                line_column_offset(self.source, self.line_starts, span.start()),
                line_column_offset(self.source, self.line_starts, span.end()),
            ) {
                if start < end {
                    self.holes.push(Hole {
                        callable: self.callable.clone(),
                        kind: kind.to_string(),
                        range: ByteRange { start, end },
                    });
                }
            }
        }
        visit::visit_expr_macro(self, expression);
    }
}

fn is_integer(value: &str) -> bool {
    matches!(
        value,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
    )
}

fn push_expression(
    output: &mut Vec<(String, String)>,
    seen: &mut BTreeSet<String>,
    family: &str,
    expression: String,
) {
    if output.len() < MAX_GRAMMAR_CANDIDATES && seen.insert(expression.clone()) {
        output.push((family.to_string(), expression));
    }
}

fn matching_binding<'a>(inputs: &'a [TypedBinding], type_name: &str) -> Option<&'a str> {
    inputs
        .iter()
        .find(|binding| binding.type_name == type_name)
        .map(|binding| binding.name.as_str())
}

fn matching_arguments<'a>(
    available: &'a [TypedBinding],
    required: &[TypedBinding],
) -> Option<Vec<&'a str>> {
    let mut used = BTreeSet::new();
    let mut arguments = Vec::with_capacity(required.len());
    for input in required {
        let (index, binding) = available.iter().enumerate().find(|(index, binding)| {
            !used.contains(index) && binding.type_name == input.type_name
        })?;
        used.insert(index);
        arguments.push(binding.name.as_str());
    }
    Some(arguments)
}

fn compose_expressions(
    callable: &CallableSignature,
    catalog: &[CallableSignature],
) -> Vec<(String, String)> {
    let mut output = Vec::new();
    let mut seen = BTreeSet::new();
    let exact_inputs = callable
        .inputs
        .iter()
        .filter(|binding| binding.type_name == callable.output)
        .collect::<Vec<_>>();
    let numeric_inputs = callable
        .inputs
        .iter()
        .filter(|binding| is_integer(&binding.type_name))
        .collect::<Vec<_>>();
    let boolean_inputs = callable
        .inputs
        .iter()
        .filter(|binding| binding.type_name == "bool")
        .collect::<Vec<_>>();

    if is_integer(&callable.output) && numeric_inputs.len() >= 2 {
        let left = &numeric_inputs[0].name;
        let right = &numeric_inputs[1].name;
        for (family, expression) in [
            ("BINARY_ADD", format!("{left} + {right}")),
            ("BINARY_SUBTRACT", format!("{left} - {right}")),
            ("BINARY_REVERSE_SUBTRACT", format!("{right} - {left}")),
            ("BINARY_MULTIPLY", format!("{left} * {right}")),
            (
                "CONDITIONAL_MIN",
                format!("if {left} <= {right} {{ {left} }} else {{ {right} }}"),
            ),
            (
                "CONDITIONAL_MAX",
                format!("if {left} >= {right} {{ {left} }} else {{ {right} }}"),
            ),
        ] {
            push_expression(&mut output, &mut seen, family, expression);
        }
    }

    if callable.output == "bool" && numeric_inputs.len() >= 2 {
        let left = &numeric_inputs[0].name;
        let right = &numeric_inputs[1].name;
        for (family, expression) in [
            ("RELATION_EQUAL", format!("{left} == {right}")),
            ("RELATION_LESS", format!("{left} < {right}")),
            ("RELATION_LESS_EQUAL", format!("{left} <= {right}")),
            ("RELATION_GREATER", format!("{left} > {right}")),
            ("RELATION_GREATER_EQUAL", format!("{left} >= {right}")),
        ] {
            push_expression(&mut output, &mut seen, family, expression);
        }
    }

    if callable.output == "bool" && boolean_inputs.len() >= 2 {
        let left = &boolean_inputs[0].name;
        let right = &boolean_inputs[1].name;
        push_expression(
            &mut output,
            &mut seen,
            "BOOLEAN_AND",
            format!("{left} && {right}"),
        );
        push_expression(
            &mut output,
            &mut seen,
            "BOOLEAN_OR",
            format!("{left} || {right}"),
        );
    }

    for candidate in catalog {
        if candidate.callable == callable.callable
            || candidate.output != callable.output
            || candidate.inputs.is_empty()
        {
            continue;
        }
        let arguments = matching_arguments(&callable.inputs, &candidate.inputs);
        if let Some(arguments) = arguments {
            push_expression(
                &mut output,
                &mut seen,
                "EXISTING_CALL",
                format!("{}({})", candidate.short_name, arguments.join(", ")),
            );
        }
    }

    for binding in exact_inputs {
        push_expression(&mut output, &mut seen, "BOUND_VALUE", binding.name.clone());
        push_expression(
            &mut output,
            &mut seen,
            "BOUND_VALUE_CLONE",
            format!("{}.clone()", binding.name),
        );
    }

    if callable.output == "bool" {
        for binding in boolean_inputs {
            push_expression(
                &mut output,
                &mut seen,
                "BOOLEAN_NOT",
                format!("!{}", binding.name),
            );
        }
        push_expression(&mut output, &mut seen, "BOOLEAN_TRUE", "true".to_string());
        push_expression(&mut output, &mut seen, "BOOLEAN_FALSE", "false".to_string());
    } else if is_integer(&callable.output) {
        push_expression(&mut output, &mut seen, "INTEGER_ZERO", "0".to_string());
        push_expression(&mut output, &mut seen, "INTEGER_ONE", "1".to_string());
    } else if callable.output == "String" {
        push_expression(
            &mut output,
            &mut seen,
            "STRING_EMPTY",
            "String::new()".to_string(),
        );
    } else if callable.output.starts_with("Option<") {
        push_expression(&mut output, &mut seen, "OPTION_NONE", "None".to_string());
        let inner = callable
            .output
            .trim_start_matches("Option<")
            .trim_end_matches('>');
        if let Some(binding) = matching_binding(&callable.inputs, inner) {
            push_expression(
                &mut output,
                &mut seen,
                "OPTION_SOME",
                format!("Some({binding})"),
            );
        }
    } else if callable.output.starts_with("Result<") {
        let inner = callable
            .output
            .trim_start_matches("Result<")
            .split(',')
            .next()
            .unwrap_or("");
        if let Some(binding) = matching_binding(&callable.inputs, inner) {
            push_expression(
                &mut output,
                &mut seen,
                "RESULT_OK",
                format!("Ok({binding})"),
            );
        }
    }
    push_expression(
        &mut output,
        &mut seen,
        "DEFAULT_CONSTRUCTOR",
        "Default::default()".to_string(),
    );
    output
}

fn excluded_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| {
            [".git", "target", "vendor", "reports", "artifacts"]
                .iter()
                .any(|excluded| name.eq_ignore_ascii_case(excluded))
        })
}

fn rust_source_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(&directory)
            .map_err(|error| format!("GRAMMAR_REPAIR_READ_DIR:{}:{error}", directory.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("GRAMMAR_REPAIR_ENTRY:{error}"))?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let file_type = entry
                .file_type()
                .map_err(|error| format!("GRAMMAR_REPAIR_FILE_TYPE:{error}"))?;
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() && !excluded_directory(&path) {
                pending.push(path);
            } else if file_type.is_file() && path.extension().and_then(OsStr::to_str) == Some("rs")
            {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn candidates_for_file(
    root: &Path,
    path: &Path,
    max_candidate_bytes: u64,
) -> Result<Vec<GrammarRepairCandidate>, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("GRAMMAR_REPAIR_READ:{}:{error}", path.display()))?;
    if bytes.len() as u64 > max_candidate_bytes {
        return Ok(Vec::new());
    }
    let source =
        std::str::from_utf8(&bytes).map_err(|_| "GRAMMAR_REPAIR_SOURCE_NOT_UTF8".to_string())?;
    let parsed = match syn::parse_file(source) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(Vec::new()),
    };
    let mut callables = Vec::new();
    collect_callables(&parsed.items, "", &mut callables);
    let catalog = callables
        .iter()
        .map(|(signature, _)| signature.clone())
        .collect::<Vec<_>>();
    let starts = line_starts(source);
    let mut holes = Vec::new();
    for (callable, block) in &callables {
        let mut visitor = HoleVisitor {
            source,
            line_starts: &starts,
            callable,
            holes: Vec::new(),
        };
        visitor.visit_block(block);
        holes.extend(visitor.holes);
    }
    holes.sort_by_key(|hole| (hole.range.start, hole.callable.callable.clone()));
    let relative_path = path
        .strip_prefix(root)
        .map_err(|_| "GRAMMAR_REPAIR_PATH_OUTSIDE_ROOT".to_string())?
        .to_path_buf();
    let file_id = relative_path.to_string_lossy().replace('\\', "/");
    let predecessor_sha256 = sha256(&bytes);
    let mut candidates = Vec::new();
    for hole in holes {
        let original = &source[hole.range.start..hole.range.end];
        let hole_identity = sha256(
            format!(
                "{}:{}:{}:{}",
                file_id,
                hole.callable.callable,
                hole.range.start,
                sha256(original.as_bytes())
            )
            .as_bytes(),
        );
        let transformation = format!("AST_GRAMMAR_HOLE:{}:{}", hole.kind, &hole_identity[..16]);
        for (index, (family, expression)) in compose_expressions(&hole.callable, &catalog)
            .into_iter()
            .enumerate()
        {
            let mut candidate_source = String::with_capacity(
                source.len() - (hole.range.end - hole.range.start) + expression.len(),
            );
            candidate_source.push_str(&source[..hole.range.start]);
            candidate_source.push_str(&expression);
            candidate_source.push_str(&source[hole.range.end..]);
            if candidate_source.len() as u64 > max_candidate_bytes {
                continue;
            }
            let Ok(structural_repair_program) =
                synthesize_structural_repair(&file_id, source, &candidate_source)
            else {
                continue;
            };
            let solution_strategy = format!(
                "GRAMMAR_COMPOSITION:{family}:{index}:{}",
                &sha256(expression.as_bytes())[..12]
            );
            candidates.push(GrammarRepairCandidate {
                relative_path: relative_path.clone(),
                predecessor_sha256: predecessor_sha256.clone(),
                candidate_sha256: sha256(candidate_source.as_bytes()),
                candidate_source,
                transformation: transformation.clone(),
                solution_strategy,
                consequence_predictions: vec![
                    format!("remove explicit {} implementation hole", hole.kind),
                    "compose only from AST-visible typed bindings, calls, operators, constructors, and conditionals"
                        .to_string(),
                    "accept semantics only after compile and public regression observations"
                        .to_string(),
                ],
                predicted_value: if hole.kind == "TODO" { 100 } else { 95 },
                structural_repair_program,
                grammar_expression: expression,
            });
            if candidates.len() >= MAX_GRAMMAR_CANDIDATES {
                return Ok(candidates);
            }
        }
    }
    Ok(candidates)
}

pub fn discover_grammar_repairs(
    root: &Path,
    max_candidate_bytes: u64,
) -> Result<Vec<GrammarRepairCandidate>, String> {
    let mut candidates = Vec::new();
    for path in rust_source_files(root)? {
        candidates.extend(candidates_for_file(root, &path, max_candidate_bytes)?);
        if candidates.len() >= MAX_GRAMMAR_CANDIDATES {
            break;
        }
    }
    candidates.truncate(MAX_GRAMMAR_CANDIDATES);
    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ast_hole_composes_typed_binary_programs_without_a_gold_patch() {
        let root =
            std::env::temp_dir().join(format!("b-core-grammar-repair-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(left: i32, right: i32) -> i32 { todo!() }\n",
        )
        .unwrap();

        let candidates = discover_grammar_repairs(&root, 1_024).unwrap();

        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].grammar_expression, "left + right");
        assert!(candidates[0].candidate_source.contains("{ left + right }"));
        assert!(candidates[0]
            .solution_strategy
            .starts_with("GRAMMAR_COMPOSITION:BINARY_ADD"));
        assert!(candidates[0]
            .structural_repair_program
            .postconditions
            .iter()
            .any(|_| true));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn grammar_basis_includes_calls_relations_and_conditionals() {
        let current = CallableSignature {
            callable: "select".to_string(),
            short_name: "select".to_string(),
            inputs: vec![
                TypedBinding {
                    name: "left".to_string(),
                    type_name: "i32".to_string(),
                },
                TypedBinding {
                    name: "right".to_string(),
                    type_name: "i32".to_string(),
                },
            ],
            output: "i32".to_string(),
        };
        let helper = CallableSignature {
            callable: "combine".to_string(),
            short_name: "combine".to_string(),
            inputs: current.inputs.clone(),
            output: "i32".to_string(),
        };
        let expressions = compose_expressions(&current, &[current.clone(), helper]);
        assert!(expressions.iter().any(|(family, _)| family == "BINARY_ADD"));
        assert!(expressions
            .iter()
            .any(|(family, _)| family == "CONDITIONAL_MAX"));
        assert!(expressions
            .iter()
            .any(|(family, expression)| family == "EXISTING_CALL"
                && expression == "combine(left, right)"));
    }

    #[test]
    fn public_test_code_is_never_a_repair_target() {
        let root = std::env::temp_dir().join(format!(
            "b-core-grammar-test-firewall-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn value() -> i32 { 1 }\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn public_observation() { todo!() }\n}\n",
        )
        .unwrap();

        let candidates = discover_grammar_repairs(&root, 1_024).unwrap();

        assert!(candidates.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
