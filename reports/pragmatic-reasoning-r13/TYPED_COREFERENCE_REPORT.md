# Pragmatic Reasoning R13 — Typed Long-Distance Coreference

Status: **PASS** for the bounded R13 increment.

R13 adds a dialogue-local typed entity memory to the pure-Rust language cortex. It carries names and other mentions from the semantic-role and attribution graphs into later turns, but it is not part of the promoted semantic catalog and every stored mention has `semantic_authority=false`.

The resolver now supports:

- unique person references across 6–12 intervening turns;
- Korean/English cross-language person binding;
- possessive belief-holder references such as `her claim`, `his belief`, `그녀의 주장`, and `그의 믿음`;
- typed service descriptors such as `that service` and `그 서비스`;
- long-distance event descriptions such as `that lattice migration operation`;
- explicit abstention for unbound, tied, quoted, and out-of-window references.

It does not infer gender from a name. `she`, `he`, `그녀`, and `그` only constrain the candidate to the person type. If more than one compatible person remains, the turn requires clarification.

## Evidence

| Validation | Result |
|---|---:|
| Initial diagnostic baseline | 0 / 30 |
| Diagnostic suite after repair | 30 / 30 |
| Fresh blind RUN2, first attempt | 26 / 29 |
| Fresh blind RUN2 after one root repair | 29 / 29 |
| Fresh blind RUN3 | 16 / 16 |
| Sealed R13 suite | 75 / 75 |
| Frozen R1–R12 canaries | 381 / 381 |
| Frozen R1–R13 canaries | 456 / 456 |
| Workspace tests | 752 / 752 |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS |

RUN2 exposed one shared failure: the Korean noun `스케줄러` inherited a person type merely because it occupied an agent-like grammatical role. The repair gives explicit system nouns precedence over role-based person fallback. A third suite with unseen names and surfaces then passed 16/16.

The typed store is bounded to 32 entities and 16 turns. Event/proposition memory is bounded to 48 records. The complete entity store is included in the conversation-state hash. A rehashed attempt to set `semantic_authority=true` is rejected by state validation.

## Boundary

This is not GPT-level language understanding. Plural group coreference, arbitrary nominal aliases, and ontology-mediated event paraphrases remain incomplete. Event descriptor selection currently requires bounded lexical evidence. The next high-value increment is ontology-mediated entity/event paraphrase matching, followed by causal and concessive discourse relations.

No external LLM, local teacher, network lookup, runtime source mutation, semantic-catalog mutation, commit, or push was used for R13. SEM-10 was not started.

The regenerated Cargo build cache was removed after final validation.
