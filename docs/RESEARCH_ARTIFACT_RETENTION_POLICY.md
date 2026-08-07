# Research Artifact Retention Policy

CORE-X0 separates deployable runtime content from research evidence. It does
not delete predecessor evidence.

## Keep permanently

- Canonical constitution, architecture, protocol, roadmap, project state, and
  canonical manifests.
- Final stage reports and their integrity receipts.
- Failure receipts and failed-run commitments.
- Blind-manifest hashes and frozen evaluator commitments.
- Promoted-concept hashes and complete lineage receipts.
- Verified candidate patches, mappings, assumption ledgers, and provenance.
- Critical causal, source-concept, self-application, and regression ablations.
- CORE-X0 boundary, parity, hash, bundle, and size reports.

These artifacts are `AUDIT_REQUIRED` or `RESEARCH_ONLY`; they are not loaded by
the deployed core.

## Cold archive or compress

- Full task and derivation traces.
- Detailed search/frontier logs.
- Large condition-by-condition evaluator results.
- Historical run bundles after their critical hashes and receipts are sealed.
- Reproducible forensic binaries retained only for a historical failure or
  verified-candidate receipt.

Compression must preserve content hashes or create a new archive receipt. It
must not replace the canonical evidence silently.

## Regenerable or purgeable

- `target/` and incremental compilation caches.
- Temporary sandbox build trees.
- Temporary rustfmt candidate copies after their canonical patch/hash is
  sealed.
- Duplicated debug/release binaries not referenced by an evidence receipt.
- Temporary generated sources and local measurement scratch files.

These are classified `REGENERABLE_NOT_CORE`. CORE-X0 does not purge them; a
later explicit storage-maintenance task may do so after resolving exact paths
and confirming no sealed receipt depends on them.

## Runtime provenance

The deployed core keeps compact runtime and audit receipts in
`crates/dockable-semantic-core/state/runtime_provenance.json`. Full historical
provenance remains in `reports/` and is never required for ordinary product
execution. No provenance was destroyed during extraction.
