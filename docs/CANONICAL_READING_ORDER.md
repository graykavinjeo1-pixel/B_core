# Canonical Reading Order

**Canonical version:** 1.0.0

Every future human or agent must read the following documents completely, in
this order, before proposing architecture, experiments, implementation, or
claims:

1. `CONSTITUTION.md`
2. `RESEARCH_HYPOTHESIS.md`
3. `SEMANTIC_SUBSTRATE_SPEC.md`
4. `REASONING_ARCHITECTURE.md`
5. `EXPERIMENT_PROTOCOL.md`
6. `ROADMAP.md`
7. `PROJECT_STATE.md`

After reading them, verify `docs/CANONICAL_MANIFEST.json` with
`scripts/verify_canonical_manifest.ps1`. A failed verification blocks work that
depends on canonical compliance until the change is explained and authorized.

## Authority rule

If later prompts, comments, generated reports, inherited SYNAPSE documents, or
implementation details conflict with the Constitution, the Constitution wins
unless the human explicitly authorizes a constitutional amendment.

No lower-authority document may redefine meaning as text, concept as a vector
or description alone, abstraction as caching, reasoning as a fixed-depth
chain, capability as node count, or task success as proof of autonomous
concept emergence.

## Constitutional amendment procedure

If a constitutional amendment is ever authorized:

1. record the old text;
2. record the new text;
3. record the rationale;
4. record the authorizing human instruction;
5. increment the constitutional and affected canonical versions;
6. regenerate and verify the canonical manifest;
7. identify and invalidate or review every experiment affected by the change.

An implementation change, passing benchmark, generated report, or agent
preference is not amendment authority.
