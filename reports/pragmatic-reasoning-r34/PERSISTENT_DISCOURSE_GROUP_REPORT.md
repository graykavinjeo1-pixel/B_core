# R34 Persistent Discourse Group Report

Status: **PASS**

R34 adds bounded, hash-bound discourse group memory to the pure-Rust Language Cortex. Action groups and attributed-proposition groups now survive neutral interruptions without acquiring semantic or execution authority. Explicit historical pair references recover the established group, while an unestablished pair among three candidates fails closed.

## Blind results

- Frozen diagnostic: **28/28** (baseline **2/28**, then **25/28**, final **28/28**)
- Frozen transfer, first exposure: **20/20**
- Fresh R34 tasks: **48/48**
- Cumulative R1-R34 continuity count: **1,387/1,387**
- Frozen hashes remained unchanged after product repair.

The final three diagnostic repairs were general mechanisms: English action-state vocabulary now observes word boundaries, `took care of` is recognized as a completion report, and Korean discourse-repair prefixes such as `사실은` are excluded from attributed actor identity. No held-out-driven repair was needed.

## Behavior now sealed

- Two coordinated actions remain a typed group across at least five neutral turns.
- An explicitly established two-speaker group survives a third speaker and resolves only the original pair.
- A later correction replaces the stale proposition for the same source when that source group is recalled.
- English and Korean pair/two/both, progress/status, and completion paraphrases use the same typed action-state path.
- Completion language creates unverified reports only; it cannot manufacture execution evidence.
- Three ungrouped actions or speakers cannot be guessed into a requested pair.
- Rehashed attempts to grant semantic authority to a discourse group are rejected.

## Verification

- Metadata-discovered canary binaries: **59/59**
- Canary cases observed across those binaries: **1,394/1,394**
- Adapter library tests: **310/310**
- Workspace library tests: **833/833**
- `cargo fmt --all -- --check`: **PASS**
- Workspace Clippy with warnings denied and the historical bounded harness exceptions: **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: **PASS**
- Build cache cleanup: **13,235 files / 17.0 GiB removed**; `target` is absent.

## Safety boundary

External LLM calls, local teacher calls, network calls, recursive source mutations, unsupported realization claims, language-promoted verified executions, and attributed propositions promoted to world truth were all zero. The implementation path is Rust. The pre-existing user change in `crates/semantic-reasoning/src/growth_supervisor.rs` remains untouched.

## Remaining limits

This result does not establish GPT-level open-domain language competence. Persistent groups are limited to action and attributed-proposition kinds, eight stored groups, sixteen turns, and a compact inspectable Korean/English paraphrase vocabulary. Nested or overlapping discourse sets, long topic-linked suspension/restoration, unrestricted associative bridging, and richer natural realization remain future work.

No commit or push was performed.
