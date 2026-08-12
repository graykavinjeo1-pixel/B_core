//! Bounded grammar composition for explicit Rust implementation holes.
//!
//! This is intentionally an enumerative synthesizer, not a bag of defect-
//! specific patches. It extracts callable types and bindings from the AST,
//! composes expressions from a small language-independent basis (values,
//! calls, unary/binary relations, constructors, and conditionals), and leaves
//! semantic selection to compile and public-test observations.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use serde::{Deserialize, Serialize};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Block, Expr, FnArg, ImplItem, Item, Lit, Pat, ReturnType, Stmt, Token, UnOp};

use crate::self_repair_contract::sha256;
use crate::structural_source_repair::{
    synthesize_structural_repair, ByteRange, StructuralRepairProgram,
};

const MAX_GRAMMAR_CANDIDATES: usize = 128;
const MAX_CANDIDATES_PER_HOLE: usize = 8;
const MAX_GRAMMAR_HOLES_PER_GENERATION: usize = MAX_GRAMMAR_CANDIDATES / MAX_CANDIDATES_PER_HOLE;
const MAX_GRAMMAR_HOLES_SCANNED_PER_GENERATION: usize = 256;
const MAX_REPOSITORY_CONTEXT_FILES: usize = 512;
const MAX_REPOSITORY_CONTEXT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CALL_COMPOSITION_CATALOG: usize = 64;

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
    pub public_examples_observed: usize,
    pub public_examples_evaluated: usize,
    pub public_examples_satisfied: usize,
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
    lexical_scope: String,
    impl_owner: Option<String>,
    inputs: Vec<TypedBinding>,
    output: String,
    has_receiver: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Hole {
    callable: CallableSignature,
    kind: String,
    range: ByteRange,
    current_expression_family: Option<String>,
    current_expression: Option<String>,
}

#[derive(Debug)]
struct FileCandidateBatch {
    candidates: Vec<GrammarRepairCandidate>,
    holes_scanned: usize,
}

struct RepositoryGrammarContext {
    parsed_files: Vec<syn::File>,
    globally_unique_short_names: BTreeSet<String>,
    external_examples_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum PublicValue {
    Int(i128),
    Bool(bool),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PublicExample {
    inputs: Vec<PublicValue>,
    expected: PublicValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PublicExampleScore {
    observed: usize,
    evaluated: usize,
    satisfied: usize,
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
                        lexical_scope: prefix.to_string(),
                        impl_owner: None,
                        inputs: typed_inputs(&function.sig.inputs),
                        output: return_type(&function.sig.output),
                        has_receiver: false,
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
                                    lexical_scope: prefix.to_string(),
                                    impl_owner: Some(owner.clone()),
                                    inputs: typed_inputs(&method.sig.inputs),
                                    output: return_type(&method.sig.output),
                                    has_receiver: method
                                        .sig
                                        .inputs
                                        .iter()
                                        .any(|input| matches!(input, FnArg::Receiver(_))),
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
                        current_expression_family: None,
                        current_expression: None,
                    });
                }
            }
        }
        visit::visit_expr_macro(self, expression);
    }
}

fn public_literal(expression: &Expr) -> Option<PublicValue> {
    match expression {
        Expr::Lit(value) => match &value.lit {
            Lit::Int(value) => value.base10_parse::<i128>().ok().map(PublicValue::Int),
            Lit::Bool(value) => Some(PublicValue::Bool(value.value)),
            Lit::Str(value) => Some(PublicValue::String(value.value())),
            _ => None,
        },
        Expr::Unary(value) if matches!(value.op, UnOp::Neg(_)) => {
            match public_literal(value.expr.as_ref())? {
                PublicValue::Int(number) => number.checked_neg().map(PublicValue::Int),
                PublicValue::Bool(_) | PublicValue::String(_) => None,
            }
        }
        Expr::Paren(value) => public_literal(value.expr.as_ref()),
        Expr::Group(value) => public_literal(value.expr.as_ref()),
        _ => None,
    }
}

fn contradicted_stub_family(expression: &Expr) -> Option<(&'static str, String)> {
    match public_literal(expression) {
        Some(PublicValue::Int(_)) => Some(("INTEGER_LITERAL", normalized_tokens(expression))),
        Some(PublicValue::Bool(_)) => Some(("BOOLEAN_LITERAL", normalized_tokens(expression))),
        Some(PublicValue::String(_)) => Some(("STRING_LITERAL", normalized_tokens(expression))),
        None if normalized_tokens(expression) == "Default::default()" => {
            Some(("DEFAULT_CONSTRUCTOR", "Default::default()".to_string()))
        }
        None => None,
    }
}

fn expression_grammar_family(
    expression: &Expr,
    callable: &CallableSignature,
    catalog: &[CallableSignature],
) -> Option<(String, String, bool)> {
    if let Some((family, normalized)) = contradicted_stub_family(expression) {
        return Some((family.to_string(), normalized, true));
    }
    let normalized = normalized_tokens(expression);
    compose_expressions(callable, catalog)
        .into_iter()
        .find_map(|(family, candidate)| {
            let candidate = syn::parse_str::<Expr>(&candidate).ok()?;
            (normalized_tokens(&candidate) == normalized)
                .then(|| (family, normalized.clone(), false))
        })
}

fn contradicted_behavior_hole(
    source: &str,
    starts: &[usize],
    items: &[Item],
    context: &RepositoryGrammarContext,
    callable: &CallableSignature,
    block: &Block,
    catalog: &[CallableSignature],
) -> Option<Hole> {
    let [Stmt::Expr(expression, None)] = block.stmts.as_slice() else {
        return None;
    };
    let (family, normalized_expression, is_stub) =
        expression_grammar_family(expression, callable, catalog)?;
    let examples = collect_repository_public_examples(items, callable, context);
    let current_score = public_example_score(&family, &normalized_expression, callable, &examples);
    if current_score.observed == 0
        || current_score.evaluated != current_score.observed
        || current_score.satisfied == current_score.observed
    {
        return None;
    }
    let span = expression.span();
    let start = line_column_offset(source, starts, span.start())?;
    let end = line_column_offset(source, starts, span.end())?;
    (start < end).then(|| Hole {
        callable: callable.clone(),
        kind: if is_stub {
            "PUBLIC_EXAMPLE_CONTRADICTED_STUB"
        } else {
            "PUBLIC_EXAMPLE_CONTRADICTED_EXPRESSION"
        }
        .to_string(),
        range: ByteRange { start, end },
        current_expression_family: Some(family),
        current_expression: Some(normalized_expression),
    })
}

fn called_short_name(expression: &Expr) -> Option<(&syn::Ident, &Punctuated<Expr, Token![,]>)> {
    let Expr::Call(call) = expression else {
        return None;
    };
    let Expr::Path(path) = call.func.as_ref() else {
        return None;
    };
    Some((&path.path.segments.last()?.ident, &call.args))
}

fn public_example_from_call(
    call_expression: &Expr,
    expected: PublicValue,
    callable: &CallableSignature,
) -> Option<PublicExample> {
    let (name, arguments) = called_short_name(call_expression)?;
    if name != callable.short_name.as_str() || arguments.len() != callable.inputs.len() {
        return None;
    }
    let inputs = arguments
        .iter()
        .map(public_literal)
        .collect::<Option<Vec<_>>>()?;
    Some(PublicExample { inputs, expected })
}

fn assertion_example(macro_: &syn::Macro, callable: &CallableSignature) -> Option<PublicExample> {
    let assertion = macro_.path.segments.last()?.ident.to_string();
    let arguments = Punctuated::<Expr, Token![,]>::parse_terminated
        .parse2(macro_.tokens.clone())
        .ok()?
        .into_iter()
        .collect::<Vec<_>>();
    match assertion.as_str() {
        "assert_eq" if arguments.len() >= 2 => {
            public_example_from_call(&arguments[0], public_literal(&arguments[1])?, callable)
                .or_else(|| {
                    public_example_from_call(
                        &arguments[1],
                        public_literal(&arguments[0])?,
                        callable,
                    )
                })
        }
        "assert" if !arguments.is_empty() && callable.output == "bool" => {
            if let Expr::Unary(unary) = &arguments[0] {
                if matches!(unary.op, UnOp::Not(_)) {
                    return public_example_from_call(
                        unary.expr.as_ref(),
                        PublicValue::Bool(false),
                        callable,
                    );
                }
            }
            public_example_from_call(&arguments[0], PublicValue::Bool(true), callable)
        }
        _ => None,
    }
}

struct PublicExampleVisitor<'a> {
    callable: &'a CallableSignature,
    examples: BTreeSet<PublicExample>,
}

impl<'ast> Visit<'ast> for PublicExampleVisitor<'_> {
    fn visit_macro(&mut self, macro_: &'ast syn::Macro) {
        if let Some(example) = assertion_example(macro_, self.callable) {
            self.examples.insert(example);
        }
        visit::visit_macro(self, macro_);
    }
}

fn collect_public_examples_from_items(
    items: &[Item],
    in_test_scope: bool,
    callable: &CallableSignature,
    output: &mut BTreeSet<PublicExample>,
) {
    for item in items {
        match item {
            Item::Mod(module) => {
                let nested_test_scope =
                    in_test_scope || module.ident == "tests" || attributes_mark_test(&module.attrs);
                if let Some((_, nested)) = &module.content {
                    collect_public_examples_from_items(nested, nested_test_scope, callable, output);
                }
            }
            Item::Fn(function) if in_test_scope || attributes_mark_test(&function.attrs) => {
                let mut visitor = PublicExampleVisitor {
                    callable,
                    examples: BTreeSet::new(),
                };
                visitor.visit_block(function.block.as_ref());
                output.append(&mut visitor.examples);
            }
            Item::Impl(implementation) if in_test_scope => {
                for member in &implementation.items {
                    if let ImplItem::Fn(method) = member {
                        let mut visitor = PublicExampleVisitor {
                            callable,
                            examples: BTreeSet::new(),
                        };
                        visitor.visit_block(&method.block);
                        output.append(&mut visitor.examples);
                    }
                }
            }
            _ => {}
        }
    }
}

fn collect_public_examples(items: &[Item], callable: &CallableSignature) -> Vec<PublicExample> {
    let mut examples = BTreeSet::new();
    collect_public_examples_from_items(items, false, callable, &mut examples);
    examples.into_iter().collect()
}

fn collect_repository_public_examples(
    local_items: &[Item],
    callable: &CallableSignature,
    context: &RepositoryGrammarContext,
) -> Vec<PublicExample> {
    let mut examples = collect_public_examples(local_items, callable)
        .into_iter()
        .collect::<BTreeSet<_>>();
    if context.external_examples_enabled
        && context
            .globally_unique_short_names
            .contains(&callable.short_name)
    {
        for parsed in &context.parsed_files {
            collect_public_examples_from_items(&parsed.items, false, callable, &mut examples);
        }
    }
    examples.into_iter().collect()
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

fn is_string_like(value: &str) -> bool {
    matches!(value, "String" | "&str")
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

fn matching_argument_indices(
    available: &[TypedBinding],
    required: &[TypedBinding],
) -> Option<Vec<usize>> {
    let mut used = BTreeSet::new();
    let mut arguments = Vec::with_capacity(required.len());
    for input in required {
        let (index, _) = available.iter().enumerate().find(|(index, binding)| {
            !used.contains(index) && binding.type_name == input.type_name
        })?;
        used.insert(index);
        arguments.push(index);
    }
    Some(arguments)
}

fn matching_arguments<'a>(
    available: &'a [TypedBinding],
    required: &[TypedBinding],
) -> Option<Vec<&'a str>> {
    matching_argument_indices(available, required).map(|indices| {
        indices
            .into_iter()
            .map(|index| available[index].name.as_str())
            .collect()
    })
}

fn callable_expression_name(
    caller: &CallableSignature,
    candidate: &CallableSignature,
) -> Option<String> {
    if candidate.has_receiver {
        return None;
    }
    if let Some(owner) = &candidate.impl_owner {
        return (caller.impl_owner.as_ref() == Some(owner))
            .then(|| format!("Self::{}", candidate.short_name));
    }
    (caller.lexical_scope == candidate.lexical_scope).then(|| candidate.short_name.clone())
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
    let string_inputs = callable
        .inputs
        .iter()
        .filter(|binding| is_string_like(&binding.type_name))
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

    if callable.output == "String" {
        if string_inputs.len() >= 2 {
            let left = &string_inputs[0].name;
            let right = &string_inputs[1].name;
            push_expression(
                &mut output,
                &mut seen,
                "STRING_CONCAT",
                format!("format!(\"{{}}{{}}\", {left}, {right})"),
            );
        }
        for binding in &string_inputs {
            if binding.type_name == "&str" {
                push_expression(
                    &mut output,
                    &mut seen,
                    "STRING_TO_OWNED",
                    format!("{}.to_string()", binding.name),
                );
            }
        }
    }

    for candidate in catalog {
        if candidate.callable == callable.callable
            || candidate.output != callable.output
            || candidate.inputs.is_empty()
        {
            continue;
        }
        let Some(candidate_name) = callable_expression_name(callable, candidate) else {
            continue;
        };
        let arguments = matching_arguments(&callable.inputs, &candidate.inputs);
        if let Some(arguments) = arguments {
            push_expression(
                &mut output,
                &mut seen,
                "EXISTING_CALL",
                format!("{}({})", candidate_name, arguments.join(", ")),
            );
        }
    }

    // Compose a bounded two-hop typed call graph. Input bindings consumed by
    // the inner call are reserved, so a non-Copy value is not blindly moved a
    // second time into the outer call. The normal compiler/test gate remains
    // the final authority for ownership and semantic behavior.
    let call_catalog = catalog
        .iter()
        .filter(|candidate| {
            candidate.callable != callable.callable
                && !candidate.inputs.is_empty()
                && callable_expression_name(callable, candidate).is_some()
        })
        .take(MAX_CALL_COMPOSITION_CATALOG)
        .collect::<Vec<_>>();
    'chains: for inner in &call_catalog {
        let Some(inner_indices) = matching_argument_indices(&callable.inputs, &inner.inputs) else {
            continue;
        };
        let inner_arguments = inner_indices
            .iter()
            .map(|index| callable.inputs[*index].name.as_str())
            .collect::<Vec<_>>();
        let Some(inner_name) = callable_expression_name(callable, inner) else {
            continue;
        };
        let inner_expression = format!("{}({})", inner_name, inner_arguments.join(", "));
        for outer in &call_catalog {
            if inner.callable == outer.callable || outer.output != callable.output {
                continue;
            }
            let Some(outer_name) = callable_expression_name(callable, outer) else {
                continue;
            };
            for inner_slot in outer
                .inputs
                .iter()
                .enumerate()
                .filter_map(|(index, input)| (input.type_name == inner.output).then_some(index))
            {
                let mut used = inner_indices.iter().copied().collect::<BTreeSet<_>>();
                let mut arguments = Vec::with_capacity(outer.inputs.len());
                let mut complete = true;
                for (index, required) in outer.inputs.iter().enumerate() {
                    if index == inner_slot {
                        arguments.push(inner_expression.clone());
                        continue;
                    }
                    let Some((binding_index, binding)) =
                        callable
                            .inputs
                            .iter()
                            .enumerate()
                            .find(|(binding_index, binding)| {
                                !used.contains(binding_index)
                                    && binding.type_name == required.type_name
                            })
                    else {
                        complete = false;
                        break;
                    };
                    used.insert(binding_index);
                    arguments.push(binding.name.clone());
                }
                if complete {
                    push_expression(
                        &mut output,
                        &mut seen,
                        "EXISTING_CALL_CHAIN",
                        format!("{}({})", outer_name, arguments.join(", ")),
                    );
                    if output.len() >= MAX_GRAMMAR_CANDIDATES {
                        break 'chains;
                    }
                }
            }
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

fn numeric_inputs(callable: &CallableSignature, example: &PublicExample) -> Option<Vec<i128>> {
    if callable.inputs.len() != example.inputs.len() {
        return None;
    }
    callable
        .inputs
        .iter()
        .zip(&example.inputs)
        .filter(|(binding, _)| is_integer(&binding.type_name))
        .map(|(_, value)| match value {
            PublicValue::Int(value) => Some(*value),
            PublicValue::Bool(_) | PublicValue::String(_) => None,
        })
        .collect()
}

fn boolean_inputs(callable: &CallableSignature, example: &PublicExample) -> Option<Vec<bool>> {
    if callable.inputs.len() != example.inputs.len() {
        return None;
    }
    callable
        .inputs
        .iter()
        .zip(&example.inputs)
        .filter(|(binding, _)| binding.type_name == "bool")
        .map(|(_, value)| match value {
            PublicValue::Bool(value) => Some(*value),
            PublicValue::Int(_) | PublicValue::String(_) => None,
        })
        .collect()
}

fn string_inputs(callable: &CallableSignature, example: &PublicExample) -> Option<Vec<String>> {
    if callable.inputs.len() != example.inputs.len() {
        return None;
    }
    callable
        .inputs
        .iter()
        .zip(&example.inputs)
        .filter(|(binding, _)| is_string_like(&binding.type_name))
        .map(|(_, value)| match value {
            PublicValue::String(value) => Some(value.clone()),
            PublicValue::Int(_) | PublicValue::Bool(_) => None,
        })
        .collect()
}

fn bound_public_value(
    callable: &CallableSignature,
    example: &PublicExample,
    expression: &str,
) -> Option<PublicValue> {
    let expression = expression.strip_prefix('!').unwrap_or(expression);
    callable
        .inputs
        .iter()
        .zip(&example.inputs)
        .find_map(|(binding, value)| {
            (expression == binding.name
                || expression == format!("{}.clone()", binding.name)
                || expression == format!("{}.to_string()", binding.name))
            .then(|| value.clone())
        })
}

fn first_two<T: Copy>(values: &[T]) -> Option<(T, T)> {
    Some((*values.first()?, *values.get(1)?))
}

fn evaluate_public_expression(
    family: &str,
    expression: &str,
    callable: &CallableSignature,
    example: &PublicExample,
) -> Option<PublicValue> {
    let numeric = || first_two(&numeric_inputs(callable, example)?);
    let boolean = || first_two(&boolean_inputs(callable, example)?);
    let string = || {
        let values = string_inputs(callable, example)?;
        Some((values.first()?.clone(), values.get(1)?.clone()))
    };
    match family {
        "BINARY_ADD" => {
            let (left, right) = numeric()?;
            left.checked_add(right).map(PublicValue::Int)
        }
        "BINARY_SUBTRACT" => {
            let (left, right) = numeric()?;
            left.checked_sub(right).map(PublicValue::Int)
        }
        "BINARY_REVERSE_SUBTRACT" => {
            let (left, right) = numeric()?;
            right.checked_sub(left).map(PublicValue::Int)
        }
        "BINARY_MULTIPLY" => {
            let (left, right) = numeric()?;
            left.checked_mul(right).map(PublicValue::Int)
        }
        "CONDITIONAL_MIN" => {
            let (left, right) = numeric()?;
            Some(PublicValue::Int(left.min(right)))
        }
        "CONDITIONAL_MAX" => {
            let (left, right) = numeric()?;
            Some(PublicValue::Int(left.max(right)))
        }
        "RELATION_EQUAL" => {
            let (left, right) = numeric()?;
            Some(PublicValue::Bool(left == right))
        }
        "RELATION_LESS" => {
            let (left, right) = numeric()?;
            Some(PublicValue::Bool(left < right))
        }
        "RELATION_LESS_EQUAL" => {
            let (left, right) = numeric()?;
            Some(PublicValue::Bool(left <= right))
        }
        "RELATION_GREATER" => {
            let (left, right) = numeric()?;
            Some(PublicValue::Bool(left > right))
        }
        "RELATION_GREATER_EQUAL" => {
            let (left, right) = numeric()?;
            Some(PublicValue::Bool(left >= right))
        }
        "BOOLEAN_AND" => {
            let (left, right) = boolean()?;
            Some(PublicValue::Bool(left && right))
        }
        "BOOLEAN_OR" => {
            let (left, right) = boolean()?;
            Some(PublicValue::Bool(left || right))
        }
        "STRING_CONCAT" => {
            let (left, right) = string()?;
            Some(PublicValue::String(format!("{left}{right}")))
        }
        "BOOLEAN_NOT" | "BOUND_VALUE" | "BOUND_VALUE_CLONE" | "STRING_TO_OWNED" => {
            bound_public_value(callable, example, expression).map(|value| match family {
                "BOOLEAN_NOT" => match value {
                    PublicValue::Bool(value) => PublicValue::Bool(!value),
                    other => other,
                },
                _ => value,
            })
        }
        "BOOLEAN_TRUE" => Some(PublicValue::Bool(true)),
        "BOOLEAN_FALSE" => Some(PublicValue::Bool(false)),
        "INTEGER_ZERO" => Some(PublicValue::Int(0)),
        "INTEGER_ONE" => Some(PublicValue::Int(1)),
        "INTEGER_LITERAL" => expression.parse::<i128>().ok().map(PublicValue::Int),
        "BOOLEAN_LITERAL" => expression.parse::<bool>().ok().map(PublicValue::Bool),
        "STRING_LITERAL" => syn::parse_str::<Expr>(expression)
            .ok()
            .and_then(|expression| public_literal(&expression)),
        "STRING_EMPTY" => Some(PublicValue::String(String::new())),
        "DEFAULT_CONSTRUCTOR" if callable.output == "bool" => Some(PublicValue::Bool(false)),
        "DEFAULT_CONSTRUCTOR" if is_integer(&callable.output) => Some(PublicValue::Int(0)),
        "DEFAULT_CONSTRUCTOR" if callable.output == "String" => {
            Some(PublicValue::String(String::new()))
        }
        _ => None,
    }
}

fn public_example_score(
    family: &str,
    expression: &str,
    callable: &CallableSignature,
    examples: &[PublicExample],
) -> PublicExampleScore {
    let mut evaluated = 0;
    let mut satisfied = 0;
    for example in examples {
        if let Some(observed) = evaluate_public_expression(family, expression, callable, example) {
            evaluated += 1;
            if observed == example.expected {
                satisfied += 1;
            }
        }
    }
    PublicExampleScore {
        observed: examples.len(),
        evaluated,
        satisfied,
    }
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

fn repository_grammar_context(
    files: &[PathBuf],
    max_candidate_bytes: u64,
) -> Result<RepositoryGrammarContext, String> {
    let mut parsed_files = Vec::new();
    let mut short_name_counts = BTreeMap::<String, usize>::new();
    let mut bytes_seen = 0_u64;
    let mut complete = files.len() <= MAX_REPOSITORY_CONTEXT_FILES;
    for path in files.iter().take(MAX_REPOSITORY_CONTEXT_FILES) {
        let bytes = fs::read(path)
            .map_err(|error| format!("GRAMMAR_CONTEXT_READ:{}:{error}", path.display()))?;
        if bytes.len() as u64 > max_candidate_bytes
            || bytes_seen.saturating_add(bytes.len() as u64) > MAX_REPOSITORY_CONTEXT_BYTES
        {
            complete = false;
            continue;
        }
        bytes_seen = bytes_seen.saturating_add(bytes.len() as u64);
        let Ok(source) = std::str::from_utf8(&bytes) else {
            complete = false;
            continue;
        };
        let Ok(parsed) = syn::parse_file(source) else {
            complete = false;
            continue;
        };
        let mut callables = Vec::new();
        collect_callables(&parsed.items, "", &mut callables);
        for (callable, _) in callables {
            *short_name_counts.entry(callable.short_name).or_default() += 1;
        }
        parsed_files.push(parsed);
    }
    let globally_unique_short_names = if complete {
        short_name_counts
            .into_iter()
            .filter_map(|(name, count)| (count == 1).then_some(name))
            .collect()
    } else {
        BTreeSet::new()
    };
    Ok(RepositoryGrammarContext {
        parsed_files,
        globally_unique_short_names,
        external_examples_enabled: complete,
    })
}

fn candidates_for_file(
    root: &Path,
    path: &Path,
    max_candidate_bytes: u64,
    source_generation: u64,
    max_holes_to_scan: usize,
    context: &RepositoryGrammarContext,
) -> Result<FileCandidateBatch, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("GRAMMAR_REPAIR_READ:{}:{error}", path.display()))?;
    if bytes.len() as u64 > max_candidate_bytes {
        return Ok(FileCandidateBatch {
            candidates: Vec::new(),
            holes_scanned: 0,
        });
    }
    let source =
        std::str::from_utf8(&bytes).map_err(|_| "GRAMMAR_REPAIR_SOURCE_NOT_UTF8".to_string())?;
    let parsed = match syn::parse_file(source) {
        Ok(parsed) => parsed,
        Err(_) => {
            return Ok(FileCandidateBatch {
                candidates: Vec::new(),
                holes_scanned: 0,
            });
        }
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
        if let Some(stub) = contradicted_behavior_hole(
            source,
            &starts,
            &parsed.items,
            context,
            callable,
            block,
            &catalog,
        ) {
            holes.push(stub);
        }
    }
    holes.sort_by_key(|hole| (hole.range.start, hole.callable.callable.clone()));
    if holes.is_empty() || max_holes_to_scan == 0 {
        return Ok(FileCandidateBatch {
            candidates: Vec::new(),
            holes_scanned: 0,
        });
    }
    let available_holes = holes.len();
    holes.rotate_left(generation_offset(
        source_generation,
        MAX_GRAMMAR_HOLES_PER_GENERATION,
        available_holes,
    ));
    holes.truncate(max_holes_to_scan.min(available_holes));
    let holes_scanned = holes.len();
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
        let public_examples =
            collect_repository_public_examples(&parsed.items, &hole.callable, context);
        if let (Some(current_family), Some(current_expression)) = (
            hole.current_expression_family.as_deref(),
            hole.current_expression.as_deref(),
        ) {
            let current_score = public_example_score(
                current_family,
                current_expression,
                &hole.callable,
                &public_examples,
            );
            if current_score.observed == 0
                || current_score.evaluated != current_score.observed
                || current_score.satisfied == current_score.observed
            {
                continue;
            }
        }
        let mut compositions = compose_expressions(&hole.callable, &catalog)
            .into_iter()
            .filter(|(_, expression)| {
                let normalized = syn::parse_str::<Expr>(expression)
                    .map(|expression| normalized_tokens(&expression))
                    .unwrap_or_else(|_| expression.clone());
                hole.current_expression.as_deref() != Some(normalized.as_str())
            })
            .enumerate()
            .map(|(index, (family, expression))| {
                let score =
                    public_example_score(&family, &expression, &hole.callable, &public_examples);
                (index, family, expression, score)
            })
            // A candidate that disagrees with any example it can evaluate is
            // already falsified and must not consume a compile/test attempt.
            // Unevaluable typed calls remain hypotheses for the authoritative
            // source-validation gate.
            .filter(|(_, _, _, score)| score.evaluated == 0 || score.satisfied == score.evaluated)
            .collect::<Vec<_>>();
        compositions.sort_by_key(|(index, _, _, score)| {
            let satisfies_every_observed_example = score.observed > 0
                && score.evaluated == score.observed
                && score.satisfied == score.observed;
            (
                Reverse(satisfies_every_observed_example),
                Reverse(score.satisfied),
                Reverse(score.evaluated),
                *index,
            )
        });
        for (index, family, expression, public_score) in
            compositions.into_iter().take(MAX_CANDIDATES_PER_HOLE)
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
            let mut consequence_predictions = vec![
                if hole.kind.starts_with("PUBLIC_EXAMPLE_CONTRADICTED_") {
                    "replace a typed implementation expression only after repository-visible examples contradict its current behavior"
                        .to_string()
                } else {
                    format!("remove explicit {} implementation hole", hole.kind)
                },
                "compose only from AST-visible typed bindings, calls, operators, constructors, and conditionals"
                    .to_string(),
                "accept semantics only after compile and public regression observations".to_string(),
            ];
            if public_score.observed > 0 {
                consequence_predictions.push(format!(
                    "pre-falsify against repository-visible public examples: {}/{} satisfied, {} evaluable",
                    public_score.satisfied, public_score.observed, public_score.evaluated
                ));
            }
            candidates.push(GrammarRepairCandidate {
                relative_path: relative_path.clone(),
                predecessor_sha256: predecessor_sha256.clone(),
                candidate_sha256: sha256(candidate_source.as_bytes()),
                candidate_source,
                transformation: transformation.clone(),
                solution_strategy,
                consequence_predictions,
                predicted_value: match hole.kind.as_str() {
                    "TODO" => 100,
                    "PUBLIC_EXAMPLE_CONTRADICTED_EXPRESSION" => 99,
                    "PUBLIC_EXAMPLE_CONTRADICTED_STUB" => 98,
                    _ => 95,
                },
                structural_repair_program,
                grammar_expression: expression,
                public_examples_observed: public_score.observed,
                public_examples_evaluated: public_score.evaluated,
                public_examples_satisfied: public_score.satisfied,
            });
        }
    }
    Ok(FileCandidateBatch {
        candidates,
        holes_scanned,
    })
}

pub fn discover_grammar_repairs(
    root: &Path,
    max_candidate_bytes: u64,
) -> Result<Vec<GrammarRepairCandidate>, String> {
    discover_grammar_repairs_for_generation(root, max_candidate_bytes, 0)
}

fn generation_offset(generation: u64, stride: usize, population: usize) -> usize {
    if population == 0 {
        return 0;
    }
    ((u128::from(generation) * stride as u128) % population as u128) as usize
}

pub fn discover_grammar_repairs_for_generation(
    root: &Path,
    max_candidate_bytes: u64,
    source_generation: u64,
) -> Result<Vec<GrammarRepairCandidate>, String> {
    let mut files = rust_source_files(root)?;
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let context = repository_grammar_context(&files, max_candidate_bytes)?;
    let file_count = files.len();
    files.rotate_left(generation_offset(
        source_generation,
        MAX_GRAMMAR_HOLES_PER_GENERATION,
        file_count,
    ));

    let mut candidate_groups: Vec<Vec<GrammarRepairCandidate>> = Vec::new();
    let mut holes_scanned = 0usize;
    'files: for path in files {
        let remaining_scan_budget =
            MAX_GRAMMAR_HOLES_SCANNED_PER_GENERATION.saturating_sub(holes_scanned);
        if remaining_scan_budget == 0 {
            break;
        }
        let batch = candidates_for_file(
            root,
            &path,
            max_candidate_bytes,
            source_generation,
            remaining_scan_budget,
            &context,
        )?;
        holes_scanned += batch.holes_scanned;
        for candidate in batch.candidates {
            if candidate_groups.last().is_some_and(|group| {
                group
                    .first()
                    .is_some_and(|existing| existing.transformation == candidate.transformation)
            }) {
                candidate_groups
                    .last_mut()
                    .expect("candidate group exists")
                    .push(candidate);
            } else if candidate_groups.len() < MAX_GRAMMAR_HOLES_PER_GENERATION {
                candidate_groups.push(vec![candidate]);
            } else {
                break 'files;
            }
        }
    }

    Ok(candidate_groups
        .into_iter()
        .flatten()
        .take(MAX_GRAMMAR_CANDIDATES)
        .collect())
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
    fn public_examples_pre_falsify_grammar_candidates_without_becoming_patch_targets() {
        let root = std::env::temp_dir().join(format!(
            "b-core-grammar-public-example-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn combine(left: i32, right: i32) -> i32 { todo!() }\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn multiplies() {\n        assert_eq!(super::combine(3, 4), 12);\n        assert_eq!(super::combine(-2, 5), -10);\n    }\n}\n",
        )
        .unwrap();

        let candidates = discover_grammar_repairs(&root, 4_096).unwrap();

        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].grammar_expression, "left * right");
        assert!(candidates[0]
            .solution_strategy
            .starts_with("GRAMMAR_COMPOSITION:BINARY_MULTIPLY"));
        assert_eq!(candidates[0].public_examples_observed, 2);
        assert_eq!(candidates[0].public_examples_evaluated, 2);
        assert_eq!(candidates[0].public_examples_satisfied, 2);
        assert!(candidates
            .iter()
            .all(|candidate| !candidate.relative_path.to_string_lossy().contains("tests")));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn boolean_assertions_become_positive_and_negative_public_examples() {
        let root = std::env::temp_dir().join(format!(
            "b-core-grammar-boolean-assertions-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn greater(left: i32, right: i32) -> bool { todo!() }\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn orders() {\n        assert!(super::greater(4, 3));\n        assert!(!super::greater(2, 3));\n        assert!(!super::greater(3, 3), \"equality is not greater\");\n    }\n}\n",
        )
        .unwrap();

        let candidates = discover_grammar_repairs(&root, 4_096).unwrap();

        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].grammar_expression, "left > right");
        assert_eq!(candidates[0].public_examples_observed, 3);
        assert_eq!(candidates[0].public_examples_evaluated, 3);
        assert_eq!(candidates[0].public_examples_satisfied, 3);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn string_examples_select_typed_concatenation() {
        let root = std::env::temp_dir().join(format!(
            "b-core-grammar-string-concat-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn join(left: &str, right: &str) -> String { todo!() }\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn joins() {\n        assert_eq!(super::join(\"a\", \"b\"), \"ab\");\n        assert_eq!(super::join(\"left\", \"right\"), \"leftright\");\n    }\n}\n",
        )
        .unwrap();

        let candidates = discover_grammar_repairs(&root, 4_096).unwrap();

        assert!(!candidates.is_empty());
        assert_eq!(
            candidates[0].grammar_expression,
            "format!(\"{}{}\", left, right)"
        );
        assert!(candidates[0]
            .solution_strategy
            .starts_with("GRAMMAR_COMPOSITION:STRING_CONCAT"));
        assert_eq!(candidates[0].public_examples_observed, 2);
        assert_eq!(candidates[0].public_examples_evaluated, 2);
        assert_eq!(candidates[0].public_examples_satisfied, 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn public_examples_turn_contradicted_literal_stub_into_bounded_search_frontier() {
        let root = std::env::temp_dir().join(format!(
            "b-core-grammar-contradicted-stub-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn combine(left: i32, right: i32) -> i32 { 0 }\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn combines() {\n        assert_eq!(super::combine(3, 4), 7);\n        assert_eq!(super::combine(-2, 5), 3);\n    }\n}\n",
        )
        .unwrap();

        let candidates = discover_grammar_repairs(&root, 4_096).unwrap();

        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].grammar_expression, "left + right");
        assert!(candidates[0]
            .transformation
            .starts_with("AST_GRAMMAR_HOLE:PUBLIC_EXAMPLE_CONTRADICTED_STUB:"));
        assert_eq!(candidates[0].public_examples_satisfied, 2);
        assert!(candidates.len() <= MAX_CANDIDATES_PER_HOLE);
        assert!(candidates.iter().all(|candidate| {
            candidate.public_examples_evaluated == 0
                || candidate.public_examples_satisfied == candidate.public_examples_evaluated
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn falsified_primitives_do_not_starve_a_typed_call_chain() {
        let root = std::env::temp_dir().join(format!(
            "b-core-grammar-call-chain-frontier-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn widen(value: i32) -> i64 { i64::from(value) * i64::from(value) }\npub fn compare(wide: i64, limit: i32) -> bool { wide > i64::from(limit) }\npub fn decide(raw: i32, limit: i32) -> bool { todo!() }\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn decides_after_widening() {\n        assert_eq!(super::decide(3, 5), true);\n        assert_eq!(super::decide(2, 5), false);\n        assert_eq!(super::decide(-3, 5), true);\n    }\n}\n",
        )
        .unwrap();

        let candidates = discover_grammar_repairs(&root, 4_096).unwrap();

        assert!(!candidates.is_empty());
        assert_eq!(
            candidates[0].grammar_expression,
            "compare(widen(raw), limit)"
        );
        assert!(candidates[0]
            .solution_strategy
            .starts_with("GRAMMAR_COMPOSITION:EXISTING_CALL_CHAIN"));
        assert_eq!(candidates[0].public_examples_observed, 3);
        assert_eq!(candidates[0].public_examples_evaluated, 0);
        assert!(candidates.iter().all(|candidate| {
            candidate.public_examples_evaluated == 0
                || candidate.public_examples_satisfied == candidate.public_examples_evaluated
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn public_examples_repair_a_wrong_non_stub_expression() {
        let root = std::env::temp_dir().join(format!(
            "b-core-grammar-contradicted-expression-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn combine(left: i32, right: i32) -> i32 { left - right }\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn combines() {\n        assert_eq!(super::combine(3, 4), 7);\n        assert_eq!(super::combine(-2, 5), 3);\n    }\n}\n",
        )
        .unwrap();

        let candidates = discover_grammar_repairs(&root, 4_096).unwrap();

        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].grammar_expression, "left + right");
        assert!(candidates[0]
            .transformation
            .starts_with("AST_GRAMMAR_HOLE:PUBLIC_EXAMPLE_CONTRADICTED_EXPRESSION:"));
        assert_eq!(candidates[0].public_examples_satisfied, 2);
        assert!(candidates
            .iter()
            .all(|candidate| candidate.grammar_expression != "left - right"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn public_examples_preserve_literal_behavior_when_the_stub_hypothesis_is_false() {
        let root = std::env::temp_dir().join(format!(
            "b-core-grammar-valid-literal-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn constant() -> i32 { 0 }\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn remains_constant() { assert_eq!(super::constant(), 0); }\n}\n",
        )
        .unwrap();

        let candidates = discover_grammar_repairs(&root, 4_096).unwrap();

        assert!(candidates.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unique_callable_uses_repository_external_public_examples() {
        let root = std::env::temp_dir().join(format!(
            "b-core-grammar-external-example-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn combine(left: i32, right: i32) -> i32 { 0 }\n",
        )
        .unwrap();
        fs::write(
            root.join("tests/combine.rs"),
            "#[test]\nfn combines() {\n    assert_eq!(semantic_reasoning::combine(3, 4), 7);\n    assert_eq!(semantic_reasoning::combine(-2, 5), 3);\n}\n",
        )
        .unwrap();

        let candidates = discover_grammar_repairs(&root, 4_096).unwrap();

        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].grammar_expression, "left + right");
        assert_eq!(candidates[0].public_examples_observed, 2);
        assert_eq!(candidates[0].public_examples_satisfied, 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ambiguous_callable_name_does_not_borrow_external_examples() {
        let root = std::env::temp_dir().join(format!(
            "b-core-grammar-ambiguous-external-example-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::write(
            root.join("src/a.rs"),
            "pub fn combine(left: i32, right: i32) -> i32 { 0 }\n",
        )
        .unwrap();
        fs::write(
            root.join("src/b.rs"),
            "pub fn combine(left: i32, right: i32) -> i32 { 0 }\n",
        )
        .unwrap();
        fs::write(
            root.join("tests/combine.rs"),
            "#[test]\nfn combines() { assert_eq!(crate::combine(3, 4), 7); }\n",
        )
        .unwrap();

        let candidates = discover_grammar_repairs(&root, 4_096).unwrap();

        assert!(candidates.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn grammar_basis_includes_calls_relations_and_conditionals() {
        let current = CallableSignature {
            callable: "select".to_string(),
            short_name: "select".to_string(),
            lexical_scope: String::new(),
            impl_owner: None,
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
            has_receiver: false,
        };
        let helper = CallableSignature {
            callable: "combine".to_string(),
            short_name: "combine".to_string(),
            lexical_scope: String::new(),
            impl_owner: None,
            inputs: current.inputs.clone(),
            output: "i32".to_string(),
            has_receiver: false,
        };
        let receiver_method = CallableSignature {
            callable: "Accumulator::combine_method".to_string(),
            short_name: "combine_method".to_string(),
            lexical_scope: String::new(),
            impl_owner: Some("Accumulator".to_string()),
            inputs: current.inputs.clone(),
            output: "i32".to_string(),
            has_receiver: true,
        };
        let foreign_helper = CallableSignature {
            callable: "nested::foreign".to_string(),
            short_name: "foreign".to_string(),
            lexical_scope: "nested".to_string(),
            impl_owner: None,
            inputs: current.inputs.clone(),
            output: "i32".to_string(),
            has_receiver: false,
        };
        let expressions = compose_expressions(
            &current,
            &[current.clone(), helper, receiver_method, foreign_helper],
        );
        assert!(expressions.iter().any(|(family, _)| family == "BINARY_ADD"));
        assert!(expressions
            .iter()
            .any(|(family, _)| family == "CONDITIONAL_MAX"));
        assert!(expressions
            .iter()
            .any(|(family, expression)| family == "EXISTING_CALL"
                && expression == "combine(left, right)"));
        assert!(!expressions
            .iter()
            .any(|(_, expression)| expression.contains("combine_method")
                || expression.contains("foreign")));
    }

    #[test]
    fn grammar_basis_composes_a_typed_two_hop_call_chain_without_reusing_inputs() {
        let current = CallableSignature {
            callable: "decide".to_string(),
            short_name: "decide".to_string(),
            lexical_scope: String::new(),
            impl_owner: None,
            inputs: vec![
                TypedBinding {
                    name: "raw".to_string(),
                    type_name: "i32".to_string(),
                },
                TypedBinding {
                    name: "limit".to_string(),
                    type_name: "i32".to_string(),
                },
            ],
            output: "bool".to_string(),
            has_receiver: false,
        };
        let widen = CallableSignature {
            callable: "widen".to_string(),
            short_name: "widen".to_string(),
            lexical_scope: String::new(),
            impl_owner: None,
            inputs: vec![TypedBinding {
                name: "value".to_string(),
                type_name: "i32".to_string(),
            }],
            output: "i64".to_string(),
            has_receiver: false,
        };
        let compare = CallableSignature {
            callable: "compare".to_string(),
            short_name: "compare".to_string(),
            lexical_scope: String::new(),
            impl_owner: None,
            inputs: vec![
                TypedBinding {
                    name: "wide".to_string(),
                    type_name: "i64".to_string(),
                },
                TypedBinding {
                    name: "limit".to_string(),
                    type_name: "i32".to_string(),
                },
            ],
            output: "bool".to_string(),
            has_receiver: false,
        };

        let expressions = compose_expressions(&current, &[current.clone(), widen, compare]);

        assert!(expressions.iter().any(|(family, expression)| {
            family == "EXISTING_CALL_CHAIN" && expression == "compare(widen(raw), limit)"
        }));
        assert!(!expressions.iter().any(|(_, expression)| {
            expression == "compare(widen(raw), raw)" || expression == "compare(widen(limit), limit)"
        }));
    }

    #[test]
    fn associated_calls_are_only_composed_inside_the_same_impl_owner() {
        let caller = CallableSignature {
            callable: "Accumulator::select".to_string(),
            short_name: "select".to_string(),
            lexical_scope: String::new(),
            impl_owner: Some("Accumulator".to_string()),
            inputs: vec![TypedBinding {
                name: "value".to_string(),
                type_name: "i32".to_string(),
            }],
            output: "i32".to_string(),
            has_receiver: true,
        };
        let same_owner = CallableSignature {
            callable: "Accumulator::normalize".to_string(),
            short_name: "normalize".to_string(),
            lexical_scope: String::new(),
            impl_owner: Some("Accumulator".to_string()),
            inputs: caller.inputs.clone(),
            output: "i32".to_string(),
            has_receiver: false,
        };
        let other_owner = CallableSignature {
            callable: "Other::normalize".to_string(),
            short_name: "normalize".to_string(),
            lexical_scope: String::new(),
            impl_owner: Some("Other".to_string()),
            inputs: caller.inputs.clone(),
            output: "i32".to_string(),
            has_receiver: false,
        };

        let expressions = compose_expressions(&caller, &[same_owner, other_owner]);

        assert!(expressions.iter().any(|(family, expression)| {
            family == "EXISTING_CALL" && expression == "Self::normalize(value)"
        }));
        assert_eq!(
            expressions
                .iter()
                .filter(|(_, expression)| expression.contains("normalize"))
                .count(),
            1
        );
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

    #[test]
    fn bounded_generation_rotation_prevents_later_hole_starvation() {
        let root =
            std::env::temp_dir().join(format!("b-core-grammar-fairness-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        let source = (0..20)
            .map(|index| {
                format!("pub fn repair_{index}(left: i32, right: i32) -> i32 {{ todo!() }}\n")
            })
            .collect::<String>();
        fs::write(root.join("src/lib.rs"), source).unwrap();

        let generation_zero =
            discover_grammar_repairs_for_generation(&root, 64 * 1_024, 0).unwrap();
        let generation_one = discover_grammar_repairs_for_generation(&root, 64 * 1_024, 1).unwrap();
        let zero_holes = generation_zero
            .iter()
            .map(|candidate| candidate.transformation.clone())
            .collect::<BTreeSet<_>>();
        let one_holes = generation_one
            .iter()
            .map(|candidate| candidate.transformation.clone())
            .collect::<BTreeSet<_>>();
        let all_holes = zero_holes.union(&one_holes).collect::<BTreeSet<_>>();

        assert_eq!(generation_zero.len(), MAX_GRAMMAR_CANDIDATES);
        assert_eq!(generation_one.len(), MAX_GRAMMAR_CANDIDATES);
        assert_eq!(zero_holes.len(), MAX_GRAMMAR_HOLES_PER_GENERATION);
        assert_eq!(one_holes.len(), MAX_GRAMMAR_HOLES_PER_GENERATION);
        assert_eq!(all_holes.len(), 20);
        for transformation in zero_holes.union(&one_holes) {
            let candidates_for_hole = generation_zero
                .iter()
                .chain(&generation_one)
                .filter(|candidate| candidate.transformation == transformation.as_str())
                .count();
            assert!(candidates_for_hole <= MAX_CANDIDATES_PER_HOLE * 2);
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bounded_generation_rotation_prevents_later_file_starvation() {
        let root = std::env::temp_dir().join(format!(
            "b-core-grammar-file-fairness-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        for index in 0..20 {
            fs::write(
                root.join(format!("src/repair_{index:02}.rs")),
                format!("pub fn repair_{index}(left: i32, right: i32) -> i32 {{ todo!() }}\n"),
            )
            .unwrap();
        }

        let generation_zero = discover_grammar_repairs_for_generation(&root, 4 * 1_024, 0).unwrap();
        let generation_one = discover_grammar_repairs_for_generation(&root, 4 * 1_024, 1).unwrap();
        let observed_paths = generation_zero
            .iter()
            .chain(&generation_one)
            .map(|candidate| candidate.relative_path.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(generation_zero.len(), MAX_GRAMMAR_CANDIDATES);
        assert_eq!(generation_one.len(), MAX_GRAMMAR_CANDIDATES);
        assert_eq!(observed_paths.len(), 20);
        fs::remove_dir_all(root).unwrap();
    }
}
