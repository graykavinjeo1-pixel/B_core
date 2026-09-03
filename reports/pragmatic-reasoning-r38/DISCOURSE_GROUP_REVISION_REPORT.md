# R38 Discourse-Group Revision and Composite Identity Report

Status: **PASS**

R38 turns persistent discourse groups from immutable snapshots into bounded, typed discourse objects that can be revised without changing identity and combined without flattening provenance. Addition and removal retain the original `group_id`, increment `revision`, and change a membership hash. Merge creates a new group whose two immediate parents remain explicit and whose member set is recomputed from live parent groups.

## Blind results

- Frozen diagnostic: **28/28** (baseline **0/28**, then **22/28**, final **28/28**)
- Frozen transfer, first exposure: **16/20**
- Frozen transfer, final after one product-only repair cycle: **20/20**
- Fresh R38 tasks: **48/48**
- Cumulative R1-R38 blind continuity: **1,579/1,579**
- The diagnostic and transfer files retained their frozen SHA-256 hashes.

The four first-exposure misses were preserved. Two successful revisions were followed by the idiomatic English question `How is that task group doing?`; reference binding was correct, but the action-state layer did not recognize the idiom as a status query and therefore returned no targets. Two quoted speaker-group examples correctly avoided group mutation, but their outer grammar-explanation request was incorrectly added to the world-action ledger. Product code was repaired to recognize the bounded English idiom and to keep quoted metalinguistic analysis outside the action ledger. The oracle was not edited.

## Behavior now sealed

- `DiscourseGroupIR` has stable identity, monotonic revision, component parent IDs, and a hash-bound member snapshot.
- Addition and removal validate exact set transitions. Rehashing an unrelated replacement set does not make it valid.
- Merge accepts exactly two existing same-kind parents, recomputes their deduplicated union, and records both parent IDs.
- Removing a required parent invalidates the composite even if the outer conversation state is rehashed.
- Group edits are represented by a separate `DiscourseGroupUpdateIR` with no semantic authority and no external execution.
- Quoted edit commands cannot mutate a group. Requests to explain quoted grammar do not create a world-action goal.
- Revised groups remain queryable across bounded neutral interruptions and Korean/English surface changes.

## Relation to the six-axis program

R38 mainly advances discourse and topic state, but the implementation crosses all six axes. Grammar compiles controlled add/remove/merge forms; group deixis selects a stable object; imperative edits are separated from adjectival and quoted mentions; discourse mutation is neither a plan nor an execution result; and the realized acknowledgement is bound to the update hash and source groups.

At the present technical level, B_Core is a verified structured semantic dialogue engine, not a GPT-equivalent open-domain conversational model. The categorical claim that natural dialogue is impossible without an LLM is refuted within the tested bounded domain: these 48 new cases and the earlier language suites run in Rust with zero LLM or teacher calls. The stronger claim that B_Core already matches GPT-level open vocabulary, world knowledge, implicature, and expressive variety is not supported.

## Verification

- Metadata-discovered canary binaries: **67/67**
- Canary and special-harness cases: **1,586/1,586**
- Adapter library tests: **329/329**
- Workspace library tests: **852/852**
- `cargo fmt --all --check`: **PASS**
- Workspace Clippy with warnings denied and the two historical bounded harness allowances: **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: **PASS**
- Build cache cleanup: **13,180 files / 19.1 GiB removed**; `target` is absent.

## Safety boundary

Unsupported realization claims, external LLM calls, local teacher calls, network calls, Python calls in the language path, recursive source mutations, language-promoted verified executions, and question-promoted language reports were all zero. The pre-existing user change in `crates/semantic-reasoning/src/growth_supervisor.rs` remains untouched.

## Remaining limits

Revision is restricted to controlled Korean and English surfaces, eight live groups, existing action or attributed-source members, and a sixteen-turn reference horizon. Composite groups record immediate parents but do not yet provide arbitrary nested group algebra, archival restoration, or eviction-safe ancestry. Open-domain pragmatic inference and varied humanlike realization remain the largest gaps.

The next dependency is topic suspension and restoration over revised and composite groups, followed by broader typed deixis and ellipsis, compositional pragmatics, deeper plan/result provenance, and more natural evidence-grounded realization.

No commit or push was performed.
