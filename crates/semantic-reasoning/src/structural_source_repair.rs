//! Structural source-repair substrate for Rust.
//!
//! The module deliberately separates four facts that the older source repair
//! path conflated:
//! 1. the predecessor's observed structure,
//! 2. the desired structural postconditions,
//! 3. the edit program used to reach them, and
//! 4. the observations that can falsify the candidate.
//!
//! Rust parsing is real (`syn`), while call and data-flow edges are explicitly
//! syntactic and intraprocedural. They are not presented as compiler-resolved
//! alias or type analysis.

use std::collections::{BTreeMap, BTreeSet};

use quote::ToTokens;
use serde::{Deserialize, Serialize};
use syn::visit::{self, Visit};
use syn::{
    Block, Expr, ExprAssign, ExprCall, ExprMethodCall, ExprReturn, ImplItem, Item, Local, Pat,
    TraitItem,
};

use crate::self_repair_contract::sha256;

const MAX_LCS_LINES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SyntaxSymbolKind {
    Module,
    Function,
    Method,
    TraitMethod,
    Struct,
    Enum,
    Trait,
    TypeAlias,
    Constant,
    Static,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SyntaxSymbol {
    pub symbol_id: String,
    pub kind: SyntaxSymbolKind,
    pub signature_sha256: String,
    pub body_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SyntacticCallEdge {
    pub caller: String,
    pub callee: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DataFlowKind {
    Initialize,
    Assign,
    Return,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SyntacticDataFlowEdge {
    pub callable: String,
    pub from: String,
    pub to: String,
    pub kind: DataFlowKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RustSyntaxFeatureKind {
    AsyncBlock,
    Await,
    TryPropagation,
    Reference,
    Dereference,
    CratePath,
    Branch,
    Loop,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustSyntaxFeatureEvidence {
    pub callable: String,
    pub kind: RustSyntaxFeatureKind,
    pub syntax_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustSourceSnapshot {
    pub source_sha256: String,
    pub ast_sha256: String,
    pub symbols: Vec<SyntaxSymbol>,
    pub call_edges: Vec<SyntacticCallEdge>,
    pub data_flow_edges: Vec<SyntacticDataFlowEdge>,
    pub syntax_features: Vec<RustSyntaxFeatureEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "postcondition_kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StructuralPostcondition {
    AstDigestEquals {
        file_id: String,
        ast_sha256: String,
    },
    SymbolEquals {
        file_id: String,
        symbol: SyntaxSymbol,
    },
    SymbolAbsent {
        file_id: String,
        symbol_id: String,
    },
    CallEdgePresent {
        file_id: String,
        edge: SyntacticCallEdge,
    },
    CallEdgeAbsent {
        file_id: String,
        edge: SyntacticCallEdge,
    },
    DataFlowEdgePresent {
        file_id: String,
        edge: SyntacticDataFlowEdge,
    },
    DataFlowEdgeAbsent {
        file_id: String,
        edge: SyntacticDataFlowEdge,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "edit_atom", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceEditAtom {
    Replace {
        range: ByteRange,
        expected_sha256: String,
        replacement: String,
    },
    Insert {
        offset: usize,
        content: String,
    },
    Delete {
        range: ByteRange,
        expected_sha256: String,
    },
    Move {
        range: ByteRange,
        expected_sha256: String,
        destination: usize,
    },
    AtomicMultiEdit {
        edits: Vec<SourceEditAtom>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationObligation {
    StructuralPostconditions,
    SourceCompile,
    PublicObservation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralRepairProgram {
    pub schema: String,
    pub file_id: String,
    pub predecessor_source_sha256: String,
    pub target_source_sha256: String,
    pub postconditions: Vec<StructuralPostcondition>,
    pub edit: SourceEditAtom,
    pub verification_obligations: Vec<VerificationObligation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralCounterexample {
    pub postcondition: Option<StructuralPostcondition>,
    pub observation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralRepairExecution {
    pub candidate_source: String,
    pub candidate_snapshot: RustSourceSnapshot,
    pub counterexamples: Vec<StructuralCounterexample>,
    pub exact_target_observed: bool,
    pub structurally_verified: bool,
}

fn normalized_tokens<T: ToTokens>(value: &T) -> String {
    value.to_token_stream().to_string()
}

fn qualified(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}::{name}")
    }
}

fn scoped_value(callable: &str, value: &str) -> String {
    format!("{callable}::{value}")
}

#[derive(Default)]
struct IdentifierCollector {
    identifiers: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for IdentifierCollector {
    fn visit_expr_path(&mut self, expression: &'ast syn::ExprPath) {
        if expression.qself.is_none() && expression.path.segments.len() == 1 {
            if let Some(segment) = expression.path.segments.first() {
                self.identifiers.insert(segment.ident.to_string());
            }
        }
    }

    fn visit_expr_call(&mut self, expression: &'ast ExprCall) {
        for argument in &expression.args {
            self.visit_expr(argument);
        }
    }

    fn visit_expr_method_call(&mut self, expression: &'ast ExprMethodCall) {
        self.visit_expr(&expression.receiver);
        for argument in &expression.args {
            self.visit_expr(argument);
        }
    }
}

fn expression_identifiers(expression: &Expr) -> BTreeSet<String> {
    let mut collector = IdentifierCollector::default();
    collector.visit_expr(expression);
    collector.identifiers
}

#[derive(Default)]
struct PatternIdentifierCollector {
    identifiers: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for PatternIdentifierCollector {
    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        self.identifiers.insert(pattern.ident.to_string());
        visit::visit_pat_ident(self, pattern);
    }
}

fn pattern_identifiers(pattern: &Pat) -> BTreeSet<String> {
    let mut collector = PatternIdentifierCollector::default();
    collector.visit_pat(pattern);
    collector.identifiers
}

struct CallableVisitor<'a> {
    caller: &'a str,
    calls: BTreeSet<SyntacticCallEdge>,
    flows: BTreeSet<SyntacticDataFlowEdge>,
    features: BTreeSet<RustSyntaxFeatureEvidence>,
}

impl<'a> CallableVisitor<'a> {
    fn new(caller: &'a str) -> Self {
        Self {
            caller,
            calls: BTreeSet::new(),
            flows: BTreeSet::new(),
            features: BTreeSet::new(),
        }
    }

    fn add_flows(
        &mut self,
        sources: BTreeSet<String>,
        targets: BTreeSet<String>,
        kind: DataFlowKind,
    ) {
        for source in sources {
            for target in &targets {
                self.flows.insert(SyntacticDataFlowEdge {
                    callable: self.caller.to_string(),
                    from: scoped_value(self.caller, &source),
                    to: scoped_value(self.caller, target),
                    kind,
                });
            }
        }
    }

    fn add_feature(&mut self, kind: RustSyntaxFeatureKind, syntax: &impl ToTokens) {
        self.features.insert(RustSyntaxFeatureEvidence {
            callable: self.caller.to_string(),
            kind,
            syntax_sha256: sha256(normalized_tokens(syntax).as_bytes()),
        });
    }
}

impl<'ast> Visit<'ast> for CallableVisitor<'_> {
    fn visit_expr_async(&mut self, expression: &'ast syn::ExprAsync) {
        self.add_feature(RustSyntaxFeatureKind::AsyncBlock, expression);
        visit::visit_expr_async(self, expression);
    }

    fn visit_expr_await(&mut self, expression: &'ast syn::ExprAwait) {
        self.add_feature(RustSyntaxFeatureKind::Await, expression);
        visit::visit_expr_await(self, expression);
    }

    fn visit_expr_try(&mut self, expression: &'ast syn::ExprTry) {
        self.add_feature(RustSyntaxFeatureKind::TryPropagation, expression);
        visit::visit_expr_try(self, expression);
    }

    fn visit_expr_reference(&mut self, expression: &'ast syn::ExprReference) {
        self.add_feature(RustSyntaxFeatureKind::Reference, expression);
        visit::visit_expr_reference(self, expression);
    }

    fn visit_expr_unary(&mut self, expression: &'ast syn::ExprUnary) {
        if matches!(expression.op, syn::UnOp::Deref(_)) {
            self.add_feature(RustSyntaxFeatureKind::Dereference, expression);
        }
        visit::visit_expr_unary(self, expression);
    }

    fn visit_expr_path(&mut self, expression: &'ast syn::ExprPath) {
        if expression
            .path
            .segments
            .first()
            .is_some_and(|segment| segment.ident == "crate")
        {
            self.add_feature(RustSyntaxFeatureKind::CratePath, expression);
        }
        visit::visit_expr_path(self, expression);
    }

    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        self.add_feature(RustSyntaxFeatureKind::Branch, expression);
        visit::visit_expr_if(self, expression);
    }

    fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
        self.add_feature(RustSyntaxFeatureKind::Branch, expression);
        visit::visit_expr_match(self, expression);
    }

    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        self.add_feature(RustSyntaxFeatureKind::Loop, expression);
        visit::visit_expr_for_loop(self, expression);
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        self.add_feature(RustSyntaxFeatureKind::Loop, expression);
        visit::visit_expr_while(self, expression);
    }

    fn visit_expr_loop(&mut self, expression: &'ast syn::ExprLoop) {
        self.add_feature(RustSyntaxFeatureKind::Loop, expression);
        visit::visit_expr_loop(self, expression);
    }

    fn visit_expr_call(&mut self, expression: &'ast ExprCall) {
        let callee = match expression.func.as_ref() {
            Expr::Path(path) => normalized_tokens(&path.path).replace(' ', ""),
            other => normalized_tokens(other),
        };
        self.calls.insert(SyntacticCallEdge {
            caller: self.caller.to_string(),
            callee,
        });
        visit::visit_expr_call(self, expression);
    }

    fn visit_expr_method_call(&mut self, expression: &'ast ExprMethodCall) {
        self.calls.insert(SyntacticCallEdge {
            caller: self.caller.to_string(),
            callee: format!("method::{}", expression.method),
        });
        visit::visit_expr_method_call(self, expression);
    }

    fn visit_local(&mut self, local: &'ast Local) {
        if let Some(initializer) = &local.init {
            self.add_flows(
                expression_identifiers(&initializer.expr),
                pattern_identifiers(&local.pat),
                DataFlowKind::Initialize,
            );
        }
        visit::visit_local(self, local);
    }

    fn visit_expr_assign(&mut self, assignment: &'ast ExprAssign) {
        self.add_flows(
            expression_identifiers(&assignment.right),
            expression_identifiers(&assignment.left),
            DataFlowKind::Assign,
        );
        visit::visit_expr_assign(self, assignment);
    }

    fn visit_expr_return(&mut self, returned: &'ast ExprReturn) {
        if let Some(expression) = &returned.expr {
            self.add_flows(
                expression_identifiers(expression),
                BTreeSet::from(["return".to_string()]),
                DataFlowKind::Return,
            );
        }
        visit::visit_expr_return(self, returned);
    }
}

#[derive(Default)]
struct SnapshotParts {
    symbols: BTreeSet<SyntaxSymbol>,
    calls: BTreeSet<SyntacticCallEdge>,
    flows: BTreeSet<SyntacticDataFlowEdge>,
    features: BTreeSet<RustSyntaxFeatureEvidence>,
}

fn analyze_callable(
    callable: &str,
    signature: &impl ToTokens,
    block: &Block,
    kind: SyntaxSymbolKind,
    output: &mut SnapshotParts,
) {
    output.symbols.insert(SyntaxSymbol {
        symbol_id: callable.to_string(),
        kind,
        signature_sha256: sha256(normalized_tokens(signature).as_bytes()),
        body_sha256: sha256(normalized_tokens(block).as_bytes()),
    });
    let mut visitor = CallableVisitor::new(callable);
    visitor.visit_block(block);
    if let Some(syn::Stmt::Expr(expression, None)) = block.stmts.last() {
        visitor.add_flows(
            expression_identifiers(expression),
            BTreeSet::from(["return".to_string()]),
            DataFlowKind::Return,
        );
    }
    output.calls.extend(visitor.calls);
    output.flows.extend(visitor.flows);
    output.features.extend(visitor.features);
}

fn plain_symbol(
    symbol_id: String,
    kind: SyntaxSymbolKind,
    signature: &impl ToTokens,
    body: &impl ToTokens,
) -> SyntaxSymbol {
    SyntaxSymbol {
        symbol_id,
        kind,
        signature_sha256: sha256(normalized_tokens(signature).as_bytes()),
        body_sha256: sha256(normalized_tokens(body).as_bytes()),
    }
}

fn collect_items(items: &[Item], prefix: &str, output: &mut SnapshotParts) {
    for item in items {
        match item {
            Item::Mod(module) => {
                let module_id = qualified(prefix, &module.ident.to_string());
                output.symbols.insert(plain_symbol(
                    module_id.clone(),
                    SyntaxSymbolKind::Module,
                    &module.ident,
                    module,
                ));
                if let Some((_, nested)) = &module.content {
                    collect_items(nested, &module_id, output);
                }
            }
            Item::Fn(function) => {
                let callable = qualified(prefix, &function.sig.ident.to_string());
                analyze_callable(
                    &callable,
                    &function.sig,
                    &function.block,
                    SyntaxSymbolKind::Function,
                    output,
                );
            }
            Item::Impl(implementation) => {
                let owner = normalized_tokens(implementation.self_ty.as_ref()).replace(' ', "");
                let owner = qualified(prefix, &owner);
                for member in &implementation.items {
                    if let ImplItem::Fn(method) = member {
                        let callable = qualified(&owner, &method.sig.ident.to_string());
                        analyze_callable(
                            &callable,
                            &method.sig,
                            &method.block,
                            SyntaxSymbolKind::Method,
                            output,
                        );
                    }
                }
            }
            Item::Trait(item_trait) => {
                let trait_id = qualified(prefix, &item_trait.ident.to_string());
                output.symbols.insert(plain_symbol(
                    trait_id.clone(),
                    SyntaxSymbolKind::Trait,
                    &item_trait.generics,
                    item_trait,
                ));
                for member in &item_trait.items {
                    if let TraitItem::Fn(method) = member {
                        let callable = qualified(&trait_id, &method.sig.ident.to_string());
                        if let Some(default) = &method.default {
                            analyze_callable(
                                &callable,
                                &method.sig,
                                default,
                                SyntaxSymbolKind::TraitMethod,
                                output,
                            );
                        } else {
                            output.symbols.insert(plain_symbol(
                                callable,
                                SyntaxSymbolKind::TraitMethod,
                                &method.sig,
                                &method.sig,
                            ));
                        }
                    }
                }
            }
            Item::Struct(item_struct) => {
                output.symbols.insert(plain_symbol(
                    qualified(prefix, &item_struct.ident.to_string()),
                    SyntaxSymbolKind::Struct,
                    &item_struct.generics,
                    item_struct,
                ));
            }
            Item::Enum(item_enum) => {
                output.symbols.insert(plain_symbol(
                    qualified(prefix, &item_enum.ident.to_string()),
                    SyntaxSymbolKind::Enum,
                    &item_enum.generics,
                    item_enum,
                ));
            }
            Item::Type(item_type) => {
                output.symbols.insert(plain_symbol(
                    qualified(prefix, &item_type.ident.to_string()),
                    SyntaxSymbolKind::TypeAlias,
                    &item_type.generics,
                    item_type,
                ));
            }
            Item::Const(item_const) => {
                output.symbols.insert(plain_symbol(
                    qualified(prefix, &item_const.ident.to_string()),
                    SyntaxSymbolKind::Constant,
                    &item_const.ty,
                    item_const,
                ));
            }
            Item::Static(item_static) => {
                output.symbols.insert(plain_symbol(
                    qualified(prefix, &item_static.ident.to_string()),
                    SyntaxSymbolKind::Static,
                    &item_static.ty,
                    item_static,
                ));
            }
            _ => {}
        }
    }
}

pub fn analyze_rust_source(source: &str) -> Result<RustSourceSnapshot, String> {
    let parsed = syn::parse_file(source).map_err(|error| format!("RUST_AST_PARSE:{error}"))?;
    let mut parts = SnapshotParts::default();
    collect_items(&parsed.items, "", &mut parts);
    Ok(RustSourceSnapshot {
        source_sha256: sha256(source.as_bytes()),
        ast_sha256: sha256(normalized_tokens(&parsed).as_bytes()),
        symbols: parts.symbols.into_iter().collect(),
        call_edges: parts.calls.into_iter().collect(),
        data_flow_edges: parts.flows.into_iter().collect(),
        syntax_features: parts.features.into_iter().collect(),
    })
}

fn attributes_mark_test(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("test")
            || (attribute.path().is_ident("cfg")
                && normalized_tokens(&attribute.meta).contains("test"))
    })
}

fn collect_test_surface(items: &[Item], prefix: &str, output: &mut Vec<String>) {
    for item in items {
        match item {
            Item::Mod(module) => {
                let id = qualified(prefix, &module.ident.to_string());
                let protected = module.ident == "tests" || attributes_mark_test(&module.attrs);
                if protected {
                    output.push(format!("{id}:{}", normalized_tokens(module)));
                } else if let Some((_, nested)) = &module.content {
                    collect_test_surface(nested, &id, output);
                }
            }
            Item::Fn(function) if attributes_mark_test(&function.attrs) => output.push(format!(
                "{}:{}",
                qualified(prefix, &function.sig.ident.to_string()),
                normalized_tokens(function)
            )),
            Item::Impl(implementation) => {
                let owner = qualified(
                    prefix,
                    &normalized_tokens(implementation.self_ty.as_ref()).replace(' ', ""),
                );
                if attributes_mark_test(&implementation.attrs) {
                    output.push(format!("{owner}:{}", normalized_tokens(implementation)));
                } else {
                    for member in &implementation.items {
                        if let ImplItem::Fn(method) = member {
                            if attributes_mark_test(&method.attrs) {
                                output.push(format!(
                                    "{}:{}",
                                    qualified(&owner, &method.sig.ident.to_string()),
                                    normalized_tokens(method)
                                ));
                            }
                        }
                    }
                }
            }
            Item::Trait(item_trait) => {
                let id = qualified(prefix, &item_trait.ident.to_string());
                if attributes_mark_test(&item_trait.attrs) {
                    output.push(format!("{id}:{}", normalized_tokens(item_trait)));
                } else {
                    for member in &item_trait.items {
                        if let TraitItem::Fn(method) = member {
                            if attributes_mark_test(&method.attrs) {
                                output.push(format!(
                                    "{}:{}",
                                    qualified(&id, &method.sig.ident.to_string()),
                                    normalized_tokens(method)
                                ));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Hash the exact AST surface that autonomous source repair is forbidden to
/// modify. Span movement and LF/CRLF differences do not change this hash;
/// changing test syntax does.
pub fn test_only_surface_sha256(source: &str) -> Result<String, String> {
    let parsed = syn::parse_file(source).map_err(|error| format!("RUST_AST_PARSE:{error}"))?;
    let mut surface = Vec::new();
    collect_test_surface(&parsed.items, "", &mut surface);
    surface.sort();
    Ok(sha256(surface.join("\n").as_bytes()))
}

pub fn derive_structural_postconditions(
    file_id: &str,
    current: &RustSourceSnapshot,
    target: &RustSourceSnapshot,
) -> Vec<StructuralPostcondition> {
    let current_symbols = current
        .symbols
        .iter()
        .map(|symbol| (symbol.symbol_id.clone(), symbol))
        .collect::<BTreeMap<_, _>>();
    let target_symbols = target
        .symbols
        .iter()
        .map(|symbol| (symbol.symbol_id.clone(), symbol))
        .collect::<BTreeMap<_, _>>();
    let current_calls = current.call_edges.iter().cloned().collect::<BTreeSet<_>>();
    let target_calls = target.call_edges.iter().cloned().collect::<BTreeSet<_>>();
    let current_flows = current
        .data_flow_edges
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let target_flows = target
        .data_flow_edges
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut postconditions = vec![StructuralPostcondition::AstDigestEquals {
        file_id: file_id.to_string(),
        ast_sha256: target.ast_sha256.clone(),
    }];
    for (symbol_id, target_symbol) in &target_symbols {
        if current_symbols.get(symbol_id).copied() != Some(*target_symbol) {
            postconditions.push(StructuralPostcondition::SymbolEquals {
                file_id: file_id.to_string(),
                symbol: (*target_symbol).clone(),
            });
        }
    }
    for symbol_id in current_symbols.keys() {
        if !target_symbols.contains_key(symbol_id) {
            postconditions.push(StructuralPostcondition::SymbolAbsent {
                file_id: file_id.to_string(),
                symbol_id: symbol_id.clone(),
            });
        }
    }
    for edge in target_calls.difference(&current_calls) {
        postconditions.push(StructuralPostcondition::CallEdgePresent {
            file_id: file_id.to_string(),
            edge: edge.clone(),
        });
    }
    for edge in current_calls.difference(&target_calls) {
        postconditions.push(StructuralPostcondition::CallEdgeAbsent {
            file_id: file_id.to_string(),
            edge: edge.clone(),
        });
    }
    for edge in target_flows.difference(&current_flows) {
        postconditions.push(StructuralPostcondition::DataFlowEdgePresent {
            file_id: file_id.to_string(),
            edge: edge.clone(),
        });
    }
    for edge in current_flows.difference(&target_flows) {
        postconditions.push(StructuralPostcondition::DataFlowEdgeAbsent {
            file_id: file_id.to_string(),
            edge: edge.clone(),
        });
    }
    postconditions.sort();
    postconditions
}

pub fn structural_counterexamples(
    file_id: &str,
    snapshot: &RustSourceSnapshot,
    postconditions: &[StructuralPostcondition],
) -> Vec<StructuralCounterexample> {
    let symbols = snapshot
        .symbols
        .iter()
        .map(|symbol| (symbol.symbol_id.as_str(), symbol))
        .collect::<BTreeMap<_, _>>();
    let calls = snapshot.call_edges.iter().collect::<BTreeSet<_>>();
    let flows = snapshot.data_flow_edges.iter().collect::<BTreeSet<_>>();
    let mut counterexamples = Vec::new();
    for postcondition in postconditions {
        let (applies, satisfied, observation) = match postcondition {
            StructuralPostcondition::AstDigestEquals {
                file_id: expected_file,
                ast_sha256,
            } => (
                expected_file == file_id,
                snapshot.ast_sha256 == *ast_sha256,
                format!("observed_ast_sha256={}", snapshot.ast_sha256),
            ),
            StructuralPostcondition::SymbolEquals {
                file_id: expected_file,
                symbol,
            } => (
                expected_file == file_id,
                symbols.get(symbol.symbol_id.as_str()).copied() == Some(symbol),
                format!(
                    "observed_symbol={:?}",
                    symbols.get(symbol.symbol_id.as_str())
                ),
            ),
            StructuralPostcondition::SymbolAbsent {
                file_id: expected_file,
                symbol_id,
            } => (
                expected_file == file_id,
                !symbols.contains_key(symbol_id.as_str()),
                format!(
                    "symbol_present={}",
                    symbols.contains_key(symbol_id.as_str())
                ),
            ),
            StructuralPostcondition::CallEdgePresent {
                file_id: expected_file,
                edge,
            } => (
                expected_file == file_id,
                calls.contains(edge),
                format!("call_edge_present={}", calls.contains(edge)),
            ),
            StructuralPostcondition::CallEdgeAbsent {
                file_id: expected_file,
                edge,
            } => (
                expected_file == file_id,
                !calls.contains(edge),
                format!("call_edge_present={}", calls.contains(edge)),
            ),
            StructuralPostcondition::DataFlowEdgePresent {
                file_id: expected_file,
                edge,
            } => (
                expected_file == file_id,
                flows.contains(edge),
                format!("data_flow_edge_present={}", flows.contains(edge)),
            ),
            StructuralPostcondition::DataFlowEdgeAbsent {
                file_id: expected_file,
                edge,
            } => (
                expected_file == file_id,
                !flows.contains(edge),
                format!("data_flow_edge_present={}", flows.contains(edge)),
            ),
        };
        if applies && !satisfied {
            counterexamples.push(StructuralCounterexample {
                postcondition: Some(postcondition.clone()),
                observation,
            });
        }
    }
    counterexamples
}

#[derive(Debug)]
struct Consumption {
    range: ByteRange,
}

#[derive(Debug)]
struct Insertion {
    offset: usize,
    order: usize,
    content: String,
}

fn checked_slice(source: &str, range: ByteRange) -> Result<&str, String> {
    if range.start >= range.end
        || range.end > source.len()
        || !source.is_char_boundary(range.start)
        || !source.is_char_boundary(range.end)
    {
        return Err("EDIT_RANGE_INVALID".to_string());
    }
    Ok(&source[range.start..range.end])
}

fn flatten_edit(
    edit: &SourceEditAtom,
    source: &str,
    consumptions: &mut Vec<Consumption>,
    insertions: &mut Vec<Insertion>,
    order: &mut usize,
) -> Result<(), String> {
    match edit {
        SourceEditAtom::Replace {
            range,
            expected_sha256,
            replacement,
        } => {
            let observed = checked_slice(source, *range)?;
            if sha256(observed.as_bytes()) != *expected_sha256 {
                return Err("REPLACE_PRECONDITION_HASH_MISMATCH".to_string());
            }
            consumptions.push(Consumption { range: *range });
            insertions.push(Insertion {
                offset: range.start,
                order: *order,
                content: replacement.clone(),
            });
            *order = order.saturating_add(1);
        }
        SourceEditAtom::Insert { offset, content } => {
            if *offset > source.len() || !source.is_char_boundary(*offset) {
                return Err("INSERT_OFFSET_INVALID".to_string());
            }
            insertions.push(Insertion {
                offset: *offset,
                order: *order,
                content: content.clone(),
            });
            *order = order.saturating_add(1);
        }
        SourceEditAtom::Delete {
            range,
            expected_sha256,
        } => {
            let observed = checked_slice(source, *range)?;
            if sha256(observed.as_bytes()) != *expected_sha256 {
                return Err("DELETE_PRECONDITION_HASH_MISMATCH".to_string());
            }
            consumptions.push(Consumption { range: *range });
        }
        SourceEditAtom::Move {
            range,
            expected_sha256,
            destination,
        } => {
            let observed = checked_slice(source, *range)?;
            if sha256(observed.as_bytes()) != *expected_sha256 {
                return Err("MOVE_PRECONDITION_HASH_MISMATCH".to_string());
            }
            if *destination > source.len()
                || !source.is_char_boundary(*destination)
                || (range.start < *destination && *destination < range.end)
            {
                return Err("MOVE_DESTINATION_INVALID".to_string());
            }
            consumptions.push(Consumption { range: *range });
            insertions.push(Insertion {
                offset: *destination,
                order: *order,
                content: observed.to_string(),
            });
            *order = order.saturating_add(1);
        }
        SourceEditAtom::AtomicMultiEdit { edits } => {
            for nested in edits {
                flatten_edit(nested, source, consumptions, insertions, order)?;
            }
        }
    }
    Ok(())
}

pub fn apply_edit_atom(source: &str, edit: &SourceEditAtom) -> Result<String, String> {
    let mut consumptions = Vec::new();
    let mut insertions = Vec::new();
    let mut order = 0;
    flatten_edit(edit, source, &mut consumptions, &mut insertions, &mut order)?;
    if consumptions.is_empty() && insertions.is_empty() {
        return Err("EDIT_PROGRAM_NO_OP".to_string());
    }
    consumptions.sort_by_key(|item| (item.range.start, item.range.end));
    for pair in consumptions.windows(2) {
        if pair[0].range.end > pair[1].range.start {
            return Err("ATOMIC_EDIT_OVERLAPPING_CONSUMPTIONS".to_string());
        }
    }
    for insertion in &insertions {
        if consumptions.iter().any(|consumption| {
            consumption.range.start < insertion.offset && insertion.offset < consumption.range.end
        }) {
            return Err("ATOMIC_EDIT_INSERTION_INSIDE_CONSUMED_RANGE".to_string());
        }
    }
    insertions.sort_by_key(|item| (item.offset, item.order));
    if insertions
        .windows(2)
        .any(|pair| pair[0].offset == pair[1].offset)
    {
        return Err("ATOMIC_EDIT_DUPLICATE_INSERTION_OFFSET".to_string());
    }

    let mut output = String::with_capacity(
        source.len()
            + insertions
                .iter()
                .map(|item| item.content.len())
                .sum::<usize>(),
    );
    let mut cursor = 0;
    let mut insertion_index = 0;
    for consumption in &consumptions {
        while insertion_index < insertions.len()
            && insertions[insertion_index].offset <= consumption.range.start
        {
            let insertion = &insertions[insertion_index];
            if insertion.offset < cursor {
                return Err("ATOMIC_EDIT_INSERTION_ORDER_INVALID".to_string());
            }
            output.push_str(&source[cursor..insertion.offset]);
            cursor = insertion.offset;
            output.push_str(&insertion.content);
            insertion_index += 1;
        }
        output.push_str(&source[cursor..consumption.range.start]);
        cursor = consumption.range.end;
    }
    while insertion_index < insertions.len() {
        let insertion = &insertions[insertion_index];
        if insertion.offset < cursor {
            return Err("ATOMIC_EDIT_INSERTION_ORDER_INVALID".to_string());
        }
        output.push_str(&source[cursor..insertion.offset]);
        cursor = insertion.offset;
        output.push_str(&insertion.content);
        insertion_index += 1;
    }
    output.push_str(&source[cursor..]);
    Ok(output)
}

fn line_offsets(source: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            offsets.push(index + 1);
        }
    }
    if offsets.last().copied() != Some(source.len()) {
        offsets.push(source.len());
    }
    offsets
}

fn lines_from_offsets<'a>(source: &'a str, offsets: &[usize]) -> Vec<&'a str> {
    offsets
        .windows(2)
        .map(|pair| &source[pair[0]..pair[1]])
        .collect()
}

fn lcs_pairs(before: &[&str], after: &[&str]) -> Vec<(usize, usize)> {
    let width = after.len() + 1;
    let mut table = vec![0_u16; (before.len() + 1) * width];
    for left in (0..before.len()).rev() {
        for right in (0..after.len()).rev() {
            let value = if before[left] == after[right] {
                table[(left + 1) * width + right + 1].saturating_add(1)
            } else {
                table[(left + 1) * width + right].max(table[left * width + right + 1])
            };
            table[left * width + right] = value;
        }
    }
    let mut pairs = Vec::new();
    let (mut left, mut right) = (0, 0);
    while left < before.len() && right < after.len() {
        if before[left] == after[right] {
            pairs.push((left, right));
            left += 1;
            right += 1;
        } else if table[(left + 1) * width + right] >= table[left * width + right + 1] {
            left += 1;
        } else {
            right += 1;
        }
    }
    pairs
}

fn atom_for_gap(source: &str, range: ByteRange, replacement: &str) -> Option<SourceEditAtom> {
    match (range.start == range.end, replacement.is_empty()) {
        (true, true) => None,
        (true, false) => Some(SourceEditAtom::Insert {
            offset: range.start,
            content: replacement.to_string(),
        }),
        (false, true) => Some(SourceEditAtom::Delete {
            range,
            expected_sha256: sha256(&source.as_bytes()[range.start..range.end]),
        }),
        (false, false) => Some(SourceEditAtom::Replace {
            range,
            expected_sha256: sha256(&source.as_bytes()[range.start..range.end]),
            replacement: replacement.to_string(),
        }),
    }
}

fn common_prefix_bytes(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| left == right)
        .map(|(character, _)| character.len_utf8())
        .sum()
}

fn common_suffix_bytes(left: &str, right: &str, prefix: usize) -> usize {
    left[prefix..]
        .chars()
        .rev()
        .zip(right[prefix..].chars().rev())
        .take_while(|(left, right)| left == right)
        .map(|(character, _)| character.len_utf8())
        .sum()
}

fn fallback_single_edit(before: &str, after: &str) -> SourceEditAtom {
    if before == after {
        return SourceEditAtom::AtomicMultiEdit { edits: Vec::new() };
    }
    let prefix = common_prefix_bytes(before, after);
    let suffix = common_suffix_bytes(before, after, prefix)
        .min(before.len().saturating_sub(prefix))
        .min(after.len().saturating_sub(prefix));
    let range = ByteRange {
        start: prefix,
        end: before.len() - suffix,
    };
    atom_for_gap(before, range, &after[prefix..after.len() - suffix])
        .expect("different sources must produce one edit")
}

fn promote_moves(source: &str, edits: Vec<SourceEditAtom>) -> Vec<SourceEditAtom> {
    let mut removed = BTreeMap::<String, Vec<usize>>::new();
    let mut inserted = BTreeMap::<String, Vec<usize>>::new();
    for (index, edit) in edits.iter().enumerate() {
        match edit {
            SourceEditAtom::Delete { range, .. } => {
                removed
                    .entry(source[range.start..range.end].to_string())
                    .or_default()
                    .push(index);
            }
            SourceEditAtom::Insert { content, .. } => {
                inserted.entry(content.clone()).or_default().push(index);
            }
            _ => {}
        }
    }
    let mut moves = BTreeMap::<usize, (usize, SourceEditAtom)>::new();
    let mut consumed_insertions = BTreeSet::new();
    for (content, deletions) in removed {
        let Some(insertions) = inserted.get(&content) else {
            continue;
        };
        if deletions.len() != 1 || insertions.len() != 1 {
            continue;
        }
        let delete_index = deletions[0];
        let insert_index = insertions[0];
        let SourceEditAtom::Delete {
            range,
            expected_sha256,
        } = &edits[delete_index]
        else {
            continue;
        };
        let SourceEditAtom::Insert { offset, .. } = &edits[insert_index] else {
            continue;
        };
        moves.insert(
            delete_index,
            (
                insert_index,
                SourceEditAtom::Move {
                    range: *range,
                    expected_sha256: expected_sha256.clone(),
                    destination: *offset,
                },
            ),
        );
        consumed_insertions.insert(insert_index);
    }
    edits
        .into_iter()
        .enumerate()
        .filter_map(|(index, edit)| {
            if let Some((_, replacement)) = moves.remove(&index) {
                Some(replacement)
            } else if consumed_insertions.contains(&index) {
                None
            } else {
                Some(edit)
            }
        })
        .collect()
}

pub fn synthesize_edit_atom(before: &str, after: &str) -> SourceEditAtom {
    if before == after {
        return SourceEditAtom::AtomicMultiEdit { edits: Vec::new() };
    }
    let before_offsets = line_offsets(before);
    let after_offsets = line_offsets(after);
    let before_lines = lines_from_offsets(before, &before_offsets);
    let after_lines = lines_from_offsets(after, &after_offsets);
    if before_lines.len() > MAX_LCS_LINES || after_lines.len() > MAX_LCS_LINES {
        return fallback_single_edit(before, after);
    }
    let pairs = lcs_pairs(&before_lines, &after_lines);
    let mut edits = Vec::new();
    let (mut before_cursor, mut after_cursor) = (0, 0);
    for (before_index, after_index) in pairs
        .into_iter()
        .chain(std::iter::once((before_lines.len(), after_lines.len())))
    {
        let range = ByteRange {
            start: before_offsets[before_cursor],
            end: before_offsets[before_index],
        };
        let replacement = &after[after_offsets[after_cursor]..after_offsets[after_index]];
        if let Some(edit) = atom_for_gap(before, range, replacement) {
            edits.push(edit);
        }
        before_cursor = before_index.saturating_add(1);
        after_cursor = after_index.saturating_add(1);
    }
    let edits = promote_moves(before, edits);
    let synthesized = if edits.len() == 1 {
        edits.into_iter().next().expect("one edit")
    } else {
        SourceEditAtom::AtomicMultiEdit { edits }
    };
    if apply_edit_atom(before, &synthesized).as_deref() == Ok(after) {
        synthesized
    } else {
        fallback_single_edit(before, after)
    }
}

pub fn synthesize_structural_repair(
    file_id: &str,
    predecessor_source: &str,
    target_source: &str,
) -> Result<StructuralRepairProgram, String> {
    if predecessor_source == target_source {
        return Err("STRUCTURAL_REPAIR_NO_OP".to_string());
    }
    if test_only_surface_sha256(predecessor_source)? != test_only_surface_sha256(target_source)? {
        return Err("STRUCTURAL_REPAIR_TEST_SURFACE_MODIFICATION_FORBIDDEN".to_string());
    }
    let current = analyze_rust_source(predecessor_source)?;
    let target = analyze_rust_source(target_source)?;
    let program = StructuralRepairProgram {
        schema: "B_CORE_STRUCTURAL_SOURCE_REPAIR_1".to_string(),
        file_id: file_id.to_string(),
        predecessor_source_sha256: current.source_sha256.clone(),
        target_source_sha256: target.source_sha256.clone(),
        postconditions: derive_structural_postconditions(file_id, &current, &target),
        edit: synthesize_edit_atom(predecessor_source, target_source),
        verification_obligations: vec![
            VerificationObligation::StructuralPostconditions,
            VerificationObligation::SourceCompile,
            VerificationObligation::PublicObservation,
        ],
    };
    let execution = execute_structural_repair(&program, predecessor_source)?;
    if !execution.structurally_verified || execution.candidate_source != target_source {
        return Err("STRUCTURAL_REPAIR_SYNTHESIS_SELF_FALSIFIED".to_string());
    }
    Ok(program)
}

pub fn execute_structural_repair(
    program: &StructuralRepairProgram,
    predecessor_source: &str,
) -> Result<StructuralRepairExecution, String> {
    if program.schema != "B_CORE_STRUCTURAL_SOURCE_REPAIR_1"
        || sha256(predecessor_source.as_bytes()) != program.predecessor_source_sha256
    {
        return Err("STRUCTURAL_REPAIR_PREDECESSOR_MISMATCH".to_string());
    }
    let candidate_source = apply_edit_atom(predecessor_source, &program.edit)?;
    let candidate_snapshot = analyze_rust_source(&candidate_source)?;
    let mut counterexamples = structural_counterexamples(
        &program.file_id,
        &candidate_snapshot,
        &program.postconditions,
    );
    let exact_target_observed = candidate_snapshot.source_sha256 == program.target_source_sha256;
    if !exact_target_observed {
        counterexamples.push(StructuralCounterexample {
            postcondition: None,
            observation: format!(
                "target_source_sha256={} observed_source_sha256={}",
                program.target_source_sha256, candidate_snapshot.source_sha256
            ),
        });
    }
    let structurally_verified = counterexamples.is_empty();
    Ok(StructuralRepairExecution {
        candidate_source,
        candidate_snapshot,
        counterexamples,
        exact_target_observed,
        structurally_verified,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_observes_ast_calls_and_intraprocedural_flow() {
        let source = r#"
fn normalize(input: i32) -> i32 { input + 1 }
fn run(value: i32) -> i32 {
    let normalized = normalize(value);
    let mut output = normalized;
    output = output + value;
    output
}
"#;
        let snapshot = analyze_rust_source(source).expect("snapshot");
        assert!(snapshot
            .symbols
            .iter()
            .any(|symbol| symbol.symbol_id == "run"));
        assert!(snapshot.call_edges.contains(&SyntacticCallEdge {
            caller: "run".to_string(),
            callee: "normalize".to_string(),
        }));
        assert!(snapshot.data_flow_edges.iter().any(|edge| {
            edge.from == "run::value"
                && edge.to == "run::normalized"
                && edge.kind == DataFlowKind::Initialize
        }));
        assert!(snapshot.data_flow_edges.iter().any(|edge| {
            edge.from == "run::output"
                && edge.to == "run::return"
                && edge.kind == DataFlowKind::Return
        }));
    }

    #[test]
    fn generalized_postconditions_expose_call_and_flow_changes() {
        let before =
            analyze_rust_source("fn old(v: i32) -> i32 { v }\nfn run(v: i32) -> i32 { old(v) }\n")
                .unwrap();
        let after = analyze_rust_source(
            "fn new(v: i32) -> i32 { v + 1 }\nfn run(v: i32) -> i32 { let x = new(v); x }\n",
        )
        .unwrap();
        let postconditions = derive_structural_postconditions("src/lib.rs", &before, &after);
        assert!(postconditions.iter().any(|condition| matches!(
            condition,
            StructuralPostcondition::CallEdgePresent { edge, .. } if edge.callee == "new"
        )));
        assert!(postconditions.iter().any(|condition| matches!(
            condition,
            StructuralPostcondition::CallEdgeAbsent { edge, .. } if edge.callee == "old"
        )));
        assert!(postconditions.iter().any(|condition| matches!(
            condition,
            StructuralPostcondition::DataFlowEdgePresent { edge, .. }
                if edge.from == "run::v" && edge.to == "run::x"
        )));
        assert!(structural_counterexamples("src/lib.rs", &after, &postconditions).is_empty());
    }

    #[test]
    fn all_edit_atoms_have_executable_predecessor_bound_semantics() {
        let replaced = apply_edit_atom(
            "alpha beta",
            &SourceEditAtom::Replace {
                range: ByteRange { start: 6, end: 10 },
                expected_sha256: sha256(b"beta"),
                replacement: "gamma".to_string(),
            },
        )
        .unwrap();
        assert_eq!(replaced, "alpha gamma");

        let inserted = apply_edit_atom(
            "alpha",
            &SourceEditAtom::Insert {
                offset: 5,
                content: " beta".to_string(),
            },
        )
        .unwrap();
        assert_eq!(inserted, "alpha beta");

        let deleted = apply_edit_atom(
            "alpha beta",
            &SourceEditAtom::Delete {
                range: ByteRange { start: 5, end: 10 },
                expected_sha256: sha256(b" beta"),
            },
        )
        .unwrap();
        assert_eq!(deleted, "alpha");

        let moved = apply_edit_atom(
            "one\ntwo\nthree\n",
            &SourceEditAtom::Move {
                range: ByteRange { start: 4, end: 8 },
                expected_sha256: sha256(b"two\n"),
                destination: 14,
            },
        )
        .unwrap();
        assert_eq!(moved, "one\nthree\ntwo\n");

        let atomic = apply_edit_atom(
            "a = old();\nb = stale();\n",
            &SourceEditAtom::AtomicMultiEdit {
                edits: vec![
                    SourceEditAtom::Replace {
                        range: ByteRange { start: 4, end: 7 },
                        expected_sha256: sha256(b"old"),
                        replacement: "new".to_string(),
                    },
                    SourceEditAtom::Replace {
                        range: ByteRange { start: 15, end: 20 },
                        expected_sha256: sha256(b"stale"),
                        replacement: "fresh".to_string(),
                    },
                ],
            },
        )
        .unwrap();
        assert_eq!(atomic, "a = new();\nb = fresh();\n");
    }

    #[test]
    fn separated_changes_synthesize_an_atomic_multi_edit() {
        let before = "fn a() { old(); }\nfn stable() {}\nfn b() { stale(); }\n";
        let after = "fn a() { new(); }\nfn stable() {}\nfn b() { fresh(); }\n";
        let edit = synthesize_edit_atom(before, after);
        assert!(matches!(edit, SourceEditAtom::AtomicMultiEdit { .. }));
        assert_eq!(apply_edit_atom(before, &edit).unwrap(), after);
    }

    #[test]
    fn moved_block_is_promoted_to_move_atom() {
        let before = "fn first() {}\nfn second() {}\nfn third() {}\n";
        let after = "fn second() {}\nfn first() {}\nfn third() {}\n";
        let edit = synthesize_edit_atom(before, after);
        assert!(matches!(
            edit,
            SourceEditAtom::Move { .. } | SourceEditAtom::AtomicMultiEdit { .. }
        ));
        assert_eq!(apply_edit_atom(before, &edit).unwrap(), after);
    }

    #[test]
    fn synthesized_program_replays_and_self_falsifies_tampering() {
        let before = "fn legacy(v: i32) -> i32 { v }\nfn run(v: i32) -> i32 { legacy(v) }\n";
        let after = "fn improved(v: i32) -> i32 { v + 1 }\nfn run(v: i32) -> i32 { improved(v) }\n";
        let program = synthesize_structural_repair("src/lib.rs", before, after).unwrap();
        let execution = execute_structural_repair(&program, before).unwrap();
        assert!(execution.structurally_verified);
        assert!(execution.exact_target_observed);
        assert_eq!(execution.candidate_source, after);

        let mut tampered = program.clone();
        tampered.target_source_sha256 = sha256(b"different target");
        let execution = execute_structural_repair(&tampered, before).unwrap();
        assert!(!execution.structurally_verified);
        assert!(!execution.counterexamples.is_empty());
    }

    #[test]
    fn rust_frontend_extracts_async_error_reference_and_control_features() {
        let source = r#"
async fn run(value: &i64) -> Result<i64, crate::Error> {
    let loaded = crate::load(*value).await?;
    if loaded > 0 {
        for item in 0..loaded { let _ = &item; }
    }
    Ok(loaded)
}
"#;
        let snapshot = analyze_rust_source(source).expect("analyze Rust feature surface");
        let kinds = snapshot
            .syntax_features
            .iter()
            .map(|feature| feature.kind)
            .collect::<BTreeSet<_>>();
        for expected in [
            RustSyntaxFeatureKind::Await,
            RustSyntaxFeatureKind::TryPropagation,
            RustSyntaxFeatureKind::Reference,
            RustSyntaxFeatureKind::Dereference,
            RustSyntaxFeatureKind::CratePath,
            RustSyntaxFeatureKind::Branch,
            RustSyntaxFeatureKind::Loop,
        ] {
            assert!(kinds.contains(&expected), "missing {expected:?}");
        }
    }

    #[test]
    fn no_op_overlap_and_test_surface_edits_fail_closed() {
        let source = "fn value() -> i32 { 1 }\n#[cfg(test)]\nmod tests { #[test] fn value() { assert_eq!(super::value(), 1); } }\n";
        assert_eq!(
            synthesize_structural_repair("src/lib.rs", source, source),
            Err("STRUCTURAL_REPAIR_NO_OP".to_string())
        );
        let changed_test = source.replace("assert_eq!(super::value(), 1)", "assert!(true)");
        assert_eq!(
            synthesize_structural_repair("src/lib.rs", source, &changed_test),
            Err("STRUCTURAL_REPAIR_TEST_SURFACE_MODIFICATION_FORBIDDEN".to_string())
        );
        let duplicate_insert = SourceEditAtom::AtomicMultiEdit {
            edits: vec![
                SourceEditAtom::Insert {
                    offset: 0,
                    content: "a".to_string(),
                },
                SourceEditAtom::Insert {
                    offset: 0,
                    content: "b".to_string(),
                },
            ],
        };
        assert_eq!(
            apply_edit_atom(source, &duplicate_insert),
            Err("ATOMIC_EDIT_DUPLICATE_INSERTION_OFFSET".to_string())
        );
    }
}
