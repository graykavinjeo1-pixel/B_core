# R37 Recursive Action Expression Report

Status: **PASS**

R37 replaces the flat R36 operator trace with a bounded recursive `ActionSetExpressionIR`. Parentheses, mixed union/difference precedence, complement scope, and relative reported-state filters now compile into a typed tree evaluated only over action IDs supplied by the persistent discourse group and action ledger.

## Blind results

- Frozen diagnostic: **28/28** (baseline **0/28**, then **6/28**, **20/28**, **25/28**, final **28/28**)
- Frozen transfer, first exposure: **20/20**
- Frozen transfer, final: **20/20**, with no held-out-driven product repair
- Fresh R37 tasks: **48/48**
- Cumulative R1-R37 blind continuity: **1,531/1,531**
- Frozen diagnostic and transfer hashes remained unchanged.

The diagnostic progression separated four general defects: no recursive expression representation, parentheses lost during normalization, valid Korean report particles or conjugations fuzzily rewritten as the report-document noun, and negated report filters misclassified as completion assertions. All repairs were made in product code; the frozen oracle was not edited.

## Behavior now sealed

- `ActionSetExpressionIR` represents source sets, subject terms, state predicates, union, intersection, difference, and complement recursively.
- Parentheses survive normalization and determine the AST rather than disappearing into whitespace.
- Each recursive node carries evaluated action IDs, while validation recomputes operator results against the typed source set.
- Trees exceeding depth 8 or 32 nodes fail validation.
- Malformed parentheses, unknown branches, out-of-source IDs, and tampered cached evaluations fail closed.
- Relative completion-report predicates filter parenthesized subject unions without manufacturing a completion report or verified execution.
- Korean `보고가`, `보고를`, `보고된`, and related conjugated forms remain grammatical surfaces rather than fuzzy aliases of `보고서`.
- The query surface owns grammar; the independently resolved discourse group owns membership.

## Relation to the six-axis program

R37 advances grammatical composition from flat set algebra to a recursive typed tree. It also protects the other five axes: discourse state supplies the source set, resolved group references precede evaluation, report filters retain interrogative selection force, reported and verified execution remain different state axes, and realization remains bound to selected ledger records with no unsupported claim.

This is not GPT-level language understanding. It is a tested recursive action-expression boundary for the controlled Korean and English cortex. The next dependency is explicit discourse-group membership revision and composite-group identity, followed by topic restoration and broader typed ellipsis over those groups.

## Verification

- Metadata-discovered canary binaries: **65/65**
- Canary cases: **1,538/1,538**
- Adapter library tests: **324/324**
- Workspace library tests: **847/847**
- `cargo fmt --all --check`: **PASS**
- Workspace Clippy with warnings denied and historical bounded harness exceptions: **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: **PASS**
- Build cache cleanup: **14,871 files / 19.4 GiB removed**; `target` is absent.

## Safety boundary

Unsupported realization claims, external LLM calls, local teacher calls, network calls, Python calls in the language path, recursive source mutations, language-promoted verified executions, and question-promoted language reports were all zero. The pre-existing user change in `crates/semantic-reasoning/src/growth_supervisor.rs` remains untouched.

## Remaining limits

The parser remains bounded to depth 8, 32 nodes, controlled Korean and English operator surfaces, ledger-bound subjects, and the existing action-state predicates. Arbitrary relational subqueries, unrestricted nested relative clauses, explicit group membership edits, nested composite-group identity, open-domain pragmatics, and freely varied natural realization remain open.

No commit or push was performed.
