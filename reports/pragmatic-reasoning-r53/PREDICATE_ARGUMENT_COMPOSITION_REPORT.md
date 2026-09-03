# R53 — Structural predicate/argument composition generalization

Status: `PASS`

R53 repairs the grammatical distinction between predicate coordination and
argument coordination. English coordinated objects now become distinct typed
entity nodes rather than one joined string. When multiple predicates share a
coordinated argument set, the complete set is reused through typed bindings;
the previous path copied only the primary object. Existing Korean
particle-marked coordination now participates in the same cross-product rule.

## Frozen evaluation

The first diagnostic execution was preserved and invalidated because the
evaluator incorrectly required a one-node `CompositionalGoalGraphIR` for a
single selected-candidate directive. No utterance, frame, argument, authority,
or sharing expectation changed. The versioned diagnostic then established a
valid preimplementation baseline of `5/13`; product rules raised the unchanged
suite to `13/13`.

The independent eight-case transfer suite was frozen after diagnostic repair
and before first execution. It passed `8/8` on first exposure. A subsequent
full-library regression exposed an `and if` clause-boundary conflict; a general
noun-versus-clause coordination repair restored the pre-existing guarded
workflow tests while the unchanged transfer suite remained `8/8`.

## Product boundary

- Active, passive, quantified, comparison-peer, and instrument coordination
  produce distinct typed members.
- Two- and three-predicate chains share the complete member set in Korean and
  English.
- Every shared member records provider, dependent, direction, clause relation,
  evidence, and confidence through the existing `SharedArgumentBindingIR`.
- Per-member quantifiers stay attached to the same entity nodes after sharing.
- Explicitly different argument groups are never overwritten.
- Quoted structure may be parsed but cannot grant semantic or execution
  authority.
- `and if`, `unless`, `when`, `once`, `because`, `then`, and `but` close a
  direct-argument span instead of becoming nominal members.
- No whole-sentence dispatch, public-schema change, semantic-payload mutation,
  new concept generation, external action, LLM dependency, or recursive source
  mutation was introduced.

## Verification and reintegration

- Fresh diagnostic and held-out tasks: `21/21`
- Adapter library tests: `428/428`
- Root workspace substantive tests: `952/952`
- Portable package tests: `455/455`
- Portable runtime boundary canaries: `4/4`
- Root and package format checks: pass
- Root and package all-target Clippy with warnings denied: pass
- Canonical manifest: pass, 10 files,
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- Package adapter sources: 44 files, hash mismatches `0`; R53 research
  canaries included `0`
- Authority violations, external LLM calls, local teacher calls, network
  calls, recursive source mutations, full-catalog scans, and routing false
  negatives: `0`

The existing semantic-role graph schema, response schema 12, conversation
state 25, and core ABI 1 remain unchanged. Product sources are synchronized
into `pakage`; no migration is required. After verification, the root build
cache (10,822 files; 6,635,515,613 bytes) and package build cache (5,144 files;
3,410,562,207 bytes) were removed. No commit or push was performed.

## Bounded residuals

Bare multiword conjunctions without a repeated determiner remain
conservatively unsplit when grammar alone cannot distinguish them from a
lexical compound. The current Theme/CoTheme representation records conjunct
membership but not an explicit AND-versus-OR set operator. Those boundaries
prevent this bounded result from supporting a claim of unrestricted grammar
or GPT-level equivalence.

## Completion boundary

R53 includes its own response-boundary regression, package synchronization,
and runtime verification. No separate R53 integration stage remains. Under
the success-assumed macro roadmap, six stages remain; the next is R54 discourse
topic state and long-context continuity. The broader GPT-level objective
remains active and unproven.
