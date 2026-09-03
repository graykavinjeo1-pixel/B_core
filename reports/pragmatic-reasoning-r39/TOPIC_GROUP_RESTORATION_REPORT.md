# R39 Topic-Group Suspension and Restoration Report

Status: **PASS**

R39 makes a discourse topic capable of referring to an exact persistent group rather than only a surface string or concept hint. A group topic now carries the group ID, live revision, membership hash, a stable topic identity, and a separate transition hash. Returning to a suspended topic restores that exact group and refreshes the current revision without giving language semantic or execution authority.

## Blind results

- Diagnostic: **28/28** (baseline **0/28**, first product implementation **24/28**, final **28/28**)
- Transfer, first exposure: **3/20**
- Transfer after one product repair cycle: **19/20**
- Transfer final after correcting a chronological-order harness bug: **20/20**
- Fresh R39 tasks: **48/48**
- Cumulative R1-R39 row-based continuity: **1,627/1,627**
- Adapter canary binaries: **69/69**
- Canary and special-harness cases: **1,634/1,634**

The first transfer exposure showed a bounded lexical-operator gap: `switch/resume/전환/복귀`, `attach/detach`, and `결합` were not connected to the existing typed topic and group operations, while `Okay, noted.` polluted the topic stack instead of remaining a backchannel. One product repair mapped those expressions to the same typed operations and raised transfer to 19/20.

Two harness defects were corrected transparently. The diagnostic compared a missing pre-conversation JSON value with an empty group array; both now normalize to the same empty set. The transfer harness selected “second group” from a recency-sorted state array rather than by `introduced_turn`; the product had selected the chronological second group correctly. A local Clippy allowance later preserved the explicit sequential construction of the twenty transfer cases. These edits changed source hashes but did not relax any behavioral criterion.

Final suite hashes:

- Diagnostic: `ca8107505c26a91fe7b46e6c843e79e91eacac4f05948e0c200745dd040da6f3`
- Transfer: `d5f9f86030ba1d04a73fa452e11f81ca9bb3c7de7ff791fb0e5c3b542a892055`

## Behavior now sealed

- `DiscourseTopicIR` distinguishes surface, concept, action-group, and attributed-proposition-group anchors.
- A group anchor binds exact `group_id`, revision, and membership hash; topic identity remains stable while its revision snapshot refreshes.
- `TopicTransitionIR` is serialized, hash sealed, non-authoritative, and non-executing. Applied and unresolved transitions are structurally distinct.
- Group activation, named-topic switching, indexed history, and previous-topic restoration use one typed transition path.
- A suspended group revision refreshes every matching topic anchor without changing its topic ID.
- Active group topics override fuzzy topic-key overlap and bounded recency when resolving a generic group reference.
- Composite and overlapping groups restore by exact identity rather than by member or topic similarity.
- Ambiguous, missing, out-of-range, and quoted topic requests fail closed without changing group, action, or topic authority.
- Realization emits a `DISCOURSE_TOPIC_TRANSITION` claim bound to transition, topic, and membership hashes.

## Relation to the six-axis program

R39 directly advances discourse/topic state and deixis/ellipsis, while exercising all six axes. Korean and English discourse-management grammar compiles to typed transitions; group identity survives topic suspension; generic group deixis resolves through the restored anchor; topic-management intent is separated from world action; no transition is treated as plan or execution evidence; and the acknowledgement is grounded in three independent hashes.

This strengthens the rebuttal to the categorical claim that an LLM is required for every form of natural human dialogue. B_Core now demonstrates deterministic Rust-only structural generalization over cross-language topic switching, long interruption, revised and composite referents, ambiguity, quotation, and evidence-grounded output. It still does not establish GPT-equivalent open-domain language ability, broad world knowledge, unrestricted implicature, or freely varied generation.

## Verification

- Adapter library tests: **333/333**
- Workspace library tests: **856/856**
- `cargo fmt --all --check`: **PASS**
- Workspace Clippy with warnings denied and bounded historical harness allowances: **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: **PASS**
- New R39 invariant tests: **4/4**
- Build cache cleanup: **15,159 files / 23,792,457,865 bytes** before; Cargo removed **22.2 GiB** and `target` is absent.

## Safety boundary

Unsupported realization claims, external LLM calls, local teacher calls, network calls, Python calls in the language path, recursive source mutations, topic-derived semantic authority, and topic-derived external execution were all zero. The pre-existing user edit in `crates/semantic-reasoning/src/growth_supervisor.rs` remains unchanged.

## Remaining limits and integration path

Group/topic language remains controlled Korean and English, live discourse groups remain bounded, and eviction-safe archival identity is not implemented. The model does not yet cover arbitrary conversational syntax, broad cultural background, unconstrained reference chains, or human-level expressive variety.

Assuming continued clean passes, the optimistic remaining program is four work units after R39: broader typed deixis and ellipsis over the new anchors; compositional intent and pragmatic inference; plan/report/observation/result provenance across complex discourse; then a dedicated six-axis integration and final frozen blind/package gate. Additional repair rounds remain possible if integration exposes cross-axis defects.

No commit or push was performed.
