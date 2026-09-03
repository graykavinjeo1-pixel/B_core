# R40 Topic-Anchored Deixis and Ellipsis Report

Status: **PASS**

R40 connects the exact topic/group identity sealed in R39 to typed deixis and
ellipsis resolution. A reference is no longer resolved from flat recency when
the active topic is an action or attributed-proposition group. The resolver
binds the live group revision, membership hash, selector, and selected member
keys without granting semantic or execution authority.

## Blind results

- Diagnostic `R40-RUN-0001`: **28/28**
  - authoritative pre-implementation baseline: **0/28**
  - first implementation: **24/28**
  - final after two bounded product repairs: **28/28**
- Held-out transfer `R40-RUN-0002`: **20/20 on first exposure**
- Fresh R40 total: **48/48**
- Diagnostic SHA-256: `5C9699FADA874E8470C103FE75762AEE7E17CF3AE930614FEEF6CD3E504AE283`
- Transfer SHA-256: `536E33B4DD3D076E822A59037149C56B6C9733C57480D64908B25EDDF9E9C99C`

Before transfer exposure, its fixture IDs were normalized from an assumed
conversation-prefixed Goal ID to the public `GOAL-000001-..` format. This was
done before the final transfer hash was frozen and did not change any
behavioral criterion.

The two diagnostic repairs were product repairs. `analyze/분석` shares the
canonical `INVESTIGATE` predicate with inspection, so predicate-role selection
now retains the original predicate surface family. Speaker ordinals now follow
the referents' introduction turns rather than the lexical ordering of group
member keys.

## Behavior now sealed

- Ordinal references select members inside the exact active group.
- Predicate-role references distinguish repair, analysis, and inspection roles
  using typed action records rather than topic words alone.
- Plural pronouns and argument ellipsis bind the whole active action or speaker
  group.
- Suspended, restored, revised, merged, overlapping, cross-language, and
  long-horizon group topics resolve through the current live revision.
- Singular references to multi-member groups, out-of-range ordinals, anchor
  type mismatches, stale anchors, and quoted metalinguistic references fail
  closed.
- Successful within-topic requests reassert the exact prior topic object after
  normal goal projection, preserving the topic ID and topic hash.
- `TopicAnchoredReferenceIR` is hash sealed and records the topic hash, group
  revision, membership hash, complete member set, selected member set, and
  non-authority flags.
- Grounded realization emits a `TOPIC_ANCHORED_REFERENCE` claim citing the
  resolution, topic, and membership hashes.

## Verification

- Metadata-discovered adapter canary binaries: **71/71**
- Adapter canary cases including direct-response harnesses: **1682/1682**
  - row-based blind/regression cases: **1675/1675**
  - direct-response special cases: **7/7**
- Adapter library tests: **339/339**
- Workspace library tests: **862/862**
- `cargo test --workspace`: **PASS**
- `cargo fmt --all --check`: **PASS**
- `cargo clippy --workspace --all-targets -- -D warnings -A clippy::too_many_arguments -A clippy::manual_is_multiple_of`: **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: **PASS**

The first canary aggregation script double-counted eleven multi-JSON binaries
after an array-to-integer conversion failure. The corrected aggregation counts
each top-level row object once and parses the four conversation-cortex JSON
responses line by line. No product code or blind oracle was changed by this
aggregation correction.

## Safety boundary

- External LLM calls: **0**
- Local teacher calls: **0**
- Network calls: **0**
- Python calls in the Language Cortex path: **0**
- Recursive source mutations: **0**
- Unsupported realization claims: **0**
- Topic/reference semantic authority: **false**
- Topic/reference external execution authority: **false**
- Implementation language: **Rust**

## Cleanup

`cargo clean` removed **15,161 files / 21.1 GiB**. `I:\B_Core\target`
does not exist after cleanup.

## Remaining limits and integration path

R40 materially advances the third axis, deixis and ellipsis, but it does not
establish GPT-equivalent open-domain language understanding. The remaining
largest gap is compositional pragmatic intent inference across implicit,
conflicting, and weakly signaled speech acts, followed by provenance continuity
and a final six-axis integration gate.

Assuming continued clean passes, three work units remain after R40:

1. R41: compositional speech-intent and pragmatic inference with preserved ambiguity.
2. R42: request, plan, report, observation, and verified-result provenance across complex discourse.
3. R43: six-axis integration, final frozen blind regression, and package-boundary gate.

Additional repair rounds remain possible if the final integration exposes
cross-axis defects.

No commit or push was performed.
