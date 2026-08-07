# Safety Rules

Always preserve:

```text
teacher_call_count = 0
network_used = false
vram_required_mb = 0
production_core_mutated = false
expected_answer_leakage_count = 0
proof_library_answer_storage_violation_count = 0
broken_count = 0
wrong_on_failed_count = 0
```

Forbidden:

- answer cache
- expected-answer cache
- output cache
- problem ID lookup
- record-key lookup
- verifier bypass
- broad solver insertion
- full node/proof scan fallback after routing miss
- verified fallback learning without shadow replay
- answer prediction
- prediction-only proof acceptance
- confidence-only activation promotion
- activation prediction promotion without shadow replay
- actual kernel split in v0.17
- child kernel production registration in v0.17
- production child-kernel active registration in v0.18
- production activation from a promotion candidate without a later activation policy
- answer-based shard splitting
- expected-answer-based shard splitting
- proofless shard contract
- proofless reducer contract
- production swarm runner mutation
- production reducer mutation
- production scheduler mutation or production resource enforcement in v0.22
- new parallel runner execution in v0.19
- global reducer proof merge completion inside v0.20
- production child-kernel activation from v0.20 shadow execution output
- production promotion from v0.21 global proof without a later promotion gate
- production resource-governor promotion from v0.22 without a later promotion gate
- unbounded fanout, retry, queue depth, or parallelism
- terminal failure retry
- retry until success
- failed shard marked verified
- quarantined shard merge
- failed proof artifact learning promotion
- recovery governance policy production use from v0.23 without a later promotion gate
- benchmark-driven production optimization from v0.24 without a later promotion gate
- optimization execution from v0.25 triage output without a later shadow implementation gate
- treating v0.25 selected branches as production approval
- Rust/Mojo/C porting from v0.25 without a native boundary profiler gate
- production reducer use from v0.26C shadow optimization policy without a later promotion gate
- weakening global independent verification during reducer optimization
- deleting reducer proof evidence during merge artifact slimming
- new optimization implementation during v0.27 re-benchmark
- production runner/reducer/core/router/kernel-registry mutation during v0.27 re-benchmark
- treating v0.27 re-benchmark as production optimization approval without a later promotion gate
- verifier bypass during v0.28 proof batch optimization
- treating shared verifier calls as proof omission
- production verifier mutation during v0.28 proof batch optimization
- proof batch optimization policy production use from v0.28 without a later promotion gate
- new optimization implementation during v0.29 system re-benchmark
- treating v0.29 selected v0.30 branch as implementation approval
- optimization without benchmark and bottleneck evidence
- native implementation from v0.29 without a profiler boundary gate
- Rust/Mojo/C code generation during v0.30E
- native code compilation during v0.30E
- native code execution during v0.30E
- moving verifier decisions, learning promotion decisions, or safety guards into
  a native boundary
- treating v0.30E native boundary contracts as implementation approval
- production replacement from v0.31A native prototype output
- production import path changes during v0.31A
- verifier, reducer, or core production changes during v0.31A
- moving verifier decisions, learning promotion decisions, or safety guard
  decisions to the v0.31A native prototype
- native execution without Python fallback or rollback pointer
- accepting nondeterministic native graph traversal output
- automatic toolchain or package-manager installation during v0.31A
- production replacement from v0.32 native shadow integration output
- production import path changes during v0.32
- production verifier, reducer, core, router, kernel-registry, or swarm-runner
  changes during v0.32
- moving verifier decisions, learning promotion decisions, or safety guard
  decisions to the v0.32 native shadow path
- native shadow execution without Python fallback or rollback pointer
- treating v0.32 native shadow runtime improvement as production approval
- production replacement from v0.33 reduced boundary output
- production import path changes during v0.33
- expanding native graph traversal scope during v0.33
- moving verifier decisions, learning promotion decisions, or safety guard
  decisions to the v0.33 reduced boundary adapter
- treating v0.33 boundary overhead reduction as production approval
- native reduced-boundary replay without Python fallback or rollback pointer
- production replacement from v0.34 system benchmark output
- production import path changes during v0.34
- production verifier, reducer, core, router, kernel-registry, or swarm-runner
  changes during v0.34
- moving verifier decisions, learning promotion decisions, or safety guard
  decisions to the v0.34 native graph system benchmark path
- treating v0.34 system benchmark speedup as production approval without a
  later staged policy and promotion gate
- native graph system benchmark without Python fallback or rollback pointer
- production native traversal enablement from v0.35 policy output
- setting native traversal as the default path during v0.35
- production replacement from v0.35 staged policy output
- production import path changes during v0.35
- production verifier, reducer, core, router, kernel-registry, or swarm-runner
  changes during v0.35
- moving verifier decisions, learning promotion decisions, or safety guard
  decisions to the v0.35 native policy
- native policy execution without Python fallback or rollback pointer
- treating v0.35 shadow-verified policy status as production approval
- production native traversal enablement from v0.36 canary output
- setting native traversal as the default path during v0.36
- production replacement from v0.36 aggressive canary output
- production import path changes during v0.36
- production verifier, reducer, core, router, kernel-registry, or swarm-runner
  changes during v0.36
- moving verifier decisions, learning promotion decisions, or safety guard
  decisions to the v0.36 canary native path
- accepting native canary output after proof-equivalence failure
- native canary replay without Python fallback or rollback pointer
- treating v0.36 aggressive canary replay as production approval
- production native traversal enablement from v0.37 stress-matrix output
- setting native traversal as the default path during v0.37
- production replacement from v0.37 native canary stress-matrix output
- production import path changes during v0.37
- production verifier, reducer, core, router, kernel-registry, swarm-runner, or
  native-path changes during v0.37
- moving verifier decisions, learning promotion decisions, or safety guard
  decisions to the v0.37 canary native path
- executing ineligible v0.37 stress requests through native traversal
- accepting native stress output after proof-equivalence failure
- promoting learning from failed or discarded native stress output
- treating v0.37 stress-matrix evidence as production approval
- production native traversal enablement from v0.38 clean-path isolation output
- treating v0.38 native promotion candidates as production approval
- setting any v0.38 candidate class as the default native path
- running v0.38 rejected classes through native traversal outside shadow replay
- reducing, skipping, or weakening equivalence checks to improve v0.38 speed
- promoting failed, fallback-required, ineligible, or rejected native classes
- production verifier, reducer, core, router, kernel-registry, swarm-runner, or
  native-path changes during v0.38
- running self-test, source replay, policy self-test, py_compile, docs update,
  full report generation, learning promotion, or registry write from
  service_fast
- running learning promotion or registry write from service_verified
- blocking user responses from learning_shadow
- removing or weakening development_gate validation
- using service runtime mode split as permission for expected answers, output
  cache, problem ID lookup, teacher signal, verifier bypass, network, VRAM, or
  production mutation
- network server creation during v0.40 service runtime harnessing
- HTTP production server creation during v0.40 service runtime harnessing
- external service binding during v0.40
- running service_fast self-test, source replay, policy self-test, py_compile,
  docs update, full report generation, native stress matrix, failure probe,
  fallback probe, learning promotion, or registry write
- running service_verified learning promotion or registry write
- running development_gate work from invalid-request or safety-blocked error
  paths
- treating the v0.40 learning_shadow hook stub as permission to enqueue,
  execute, promote, or store learning events
- mutating production core, router, kernel registry, swarm runner, reducer,
  native path, verifier, imports, or files from v0.40 service paths
- background daemon creation during v0.41
- production worker execution during v0.41
- network server or HTTP production server creation during v0.41
- learning execution before service response creation
- learning promotion from service_fast or service_verified
- registry write from any service path or queue stub path
- queue job envelopes containing expected answers, final answers, solver
  outputs, output caches, problem ID lookup keys, or teacher signals
- development_gate calls during queue enqueue or queue drain stub validation
- treating v0.41 queue jobs as verified learning events
- treating v0.41 drain stubs as permission to promote or store proof structures
- unbounded queue growth or unbounded retry in learning_shadow queue stubs
- mutating production core, router, kernel registry, swarm runner, reducer,
  native path, verifier, imports, or files from v0.41 queue paths
- production HTTP server creation during v0.42 local service API harnessing
- public API exposure, socket listener creation, external network service
  creation, or background daemon creation during v0.42
- calling self-test, source replay, policy self-test, py_compile, docs update,
  full benchmark runs, full development reports, native stress probes, failure
  probes, fallback probes, learning execution, learning promotion, registry
  writes, or development_gate work from v0.42 service API paths
- calling MATH, GSM8K, regression, or benchmark harnesses directly through the
  v0.42 service API
- treating v0.42 local service API responses as verified learning events,
  promotion approval, registry write approval, or production API approval
- local service API request envelopes containing expected answers, final
  answers, solver outputs, output caches, problem IDs, problem ID lookup keys,
  teacher signals, verifier final decisions, or learning promotion decisions
- mutating production core, router, kernel registry, swarm runner, reducer,
  native path, verifier, imports, or files from v0.42 service API paths
- production HTTP server creation during v0.43 local service API contract fuzzing
- public API exposure, socket listener creation, external network service
  creation, or background daemon creation during v0.43
- treating v0.43 fuzz probes as permission to call development_gate,
  benchmark full runs, expected-answer scoring, source replay, policy
  self-test, py_compile, docs update, learning execution, learning promotion,
  registry writes, or production mutation from service API paths
- accepting malformed v0.43 service API fuzz requests as successful service
  results when they violate schema, mode, mutation, benchmark, artifact,
  latency, payload, or async learning queue boundaries
- echoing expected answer values, forbidden field values, raw stack traces, or
  internal file paths in v0.43 hardened error envelopes
- treating v0.43 local service API contract hardening as production API
  approval, learning promotion approval, or registry write approval
- mutating production core, router, kernel registry, swarm runner, reducer,
  native path, verifier, imports, or files from v0.43 service API fuzz paths
- production background daemon creation during v0.44 shadow worker drain
- network worker, external queue server, HTTP worker, or public worker endpoint
  creation during v0.44
- service response blocking from v0.44 shadow drain or replay work
- running learning replay inside service_fast or service_verified response paths
- production promotion, proof-library write, registry write, routing patch
  registry write, kernel registry write, native registry write, or production
  activation from v0.44 shadow worker output
- queue jobs containing expected answers, final answers, solver outputs, output
  caches, cached results, problem ID lookup keys, benchmark scores, ground
  truth, teacher signals, verifier bypass requests, registry write requests,
  learning promotion requests, or production mutation requests
- full development gate calls, policy self-test calls, py_compile calls, docs
  update calls, source replay calls, full benchmark runs, unbounded queue drain,
  or unbounded retry during v0.44
- treating v0.44 shadow promotion candidates as production approval or registry
  write approval
- treating v0.45 staged promotion deltas or promotion packages as production
  apply approval, proof-library write approval, registry write approval, routing
  patch registry write approval, activation prediction registry write approval,
  native registry write approval, reducer policy write approval, resource
  governor policy write approval, service path mutation approval, or production
  activation approval
- v0.45 staged deltas with `apply_allowed = true`, missing rollback pointers,
  missing service impact checks, or `requires_next_gate = false`
- quarantined or rejected candidates promoted by the v0.45 gate
- treating v0.46-pre sidecar direct validation or guarded policy validation as
  Twin-Core approval
- using `SYNAPSE_CORE_AUTONOMOUS_RUNNER=1` or any runner-origin flag to bypass
  Twin-Core registry pointer readiness
- creating, restoring, or repairing the Twin-Core pointer inside v0.46-pre
- treating v0.46a Twin-Core pointer provisioning as production apply approval
- applying staged packages or running shadow registry dry-run inside v0.46a
- granting Primary Core self-approval or Twin-Core production apply authority
  from v0.46a pointer, manifest, or policy artifacts
- applying staged promotion packages when the Twin-Core pointer is missing,
  invalid, stale, identity-mismatched, unsafe, or blocked
- treating v0.47 Safe Blocked Green as approval to apply staged deltas
- forcing v0.47 shadow registry dry-run when the Twin-Core pointer is missing
- using v0.47 Twin-Core approval simulation as actual Twin-Core approval
- writing production registries, the production proof library, or the
  production learning registry from v0.47 staged registry dry-run artifacts
- bypassing rollback pointers, Twin-Core readiness, or contamination checks
  before any future staged registry dry-run
- treating v0.47_replay shadow registry dry-run success as production apply
  approval, proof-library write approval, registry write approval, or actual
  Twin-Core production approval
- writing production registries, the production proof library, the production
  learning registry, routing patch registries, activation prediction
  registries, native registries, reducer policy registries, or resource
  governor registries from v0.47_replay artifacts
- using v0.47_replay rollback success or service impact safety as permission
  to skip a later Twin-Core review gate
- bypassing production contamination checks after v0.47_replay shadow dry-run
  or rollback replay
- treating v0.48 Twin-Core approval packages as production apply approval,
  proof-library write approval, registry write approval, learning registry
  write approval, or production activation approval
- including rejected deltas in a v0.48 approval package
- including held deltas in a v0.48 approval package without additional
  evidence
- using v0.48 approval packages to bypass a later production apply gate,
  rollback replay, service replay, or final human/operator gate
- treating v0.49 dry-run apply simulation as production apply approval,
  operator approval, proof-library write approval, registry write approval, or
  production activation approval
- auto-granting operator approval during v0.49
- setting `apply_allowed = true` in v0.49 without explicit operator approval
- using v0.49 production snapshot compatibility as permission to write
  production registries or libraries
- treating v0.50 non-production sandbox apply as production apply approval,
  operator approval, proof-library write approval, registry write approval, or
  production activation approval
- treating a v0.50 sandbox operator stub as production operator approval
- writing production registries, the production proof library, the production
  learning registry, routing patch registries, activation prediction
  registries, native registries, reducer policy registries, or resource
  governor registries from v0.50 sandbox artifacts
- bypassing v0.50 sandbox rollback replay, cleanup verification, production
  contamination checks, service replay, API boundary fuzz replay, or learning
  queue boundary replay before any later gate
- treating v0.51 service regression replay as production apply approval,
  proof-library write approval, registry write approval, or learning promotion
  approval
- calling development gate, benchmark full run, policy self-test, source
  replay, py_compile, docs update, learning promotion, or registry write from
  service_fast or service_verified paths during or after v0.51
- ignoring v0.51 latency budget failures before later service MVP work
- mutating production core, router, kernel registry, swarm runner, reducer,
  native path, verifier, imports, or files from v0.44 shadow worker paths
- treating answer-free shadow governance probes as production failures
- benchmark proof-obligation relaxation
- hot reload or dynamic code injection
- production kernel registry mutation
- parent kernel removal
- registry promotion without replay or rollback pointer
- production router mutation during routing-quality learning
- unguarded Production Rust Core mutation
- teacher/network dependency in the trusted Core path

Allowed:

- local bounded replay
- answer-free proof skeletons
- negative near-miss tests
- independent equivalence checks
- kernel routing metrics
- bounded routing fallback with recorded scope and candidate growth
- shadow-verified routing patch candidates with rollback pointers
- verifier-required activation prediction hints with rollback pointers
- answer-free kernel pressure profiles and shadow split plans
- shadow child-kernel replay reports and staged promotion candidates
- proof-aware shard plan contracts over existing swarm shard structures
- shadow shard execution records with local proof and resource observations
- shadow reducer merge reports and global proof verification reports
- shadow resource governor policies with bounded scheduling and rollback pointers
- shadow failure recovery, quarantine, retry replay, and contamination guard reports
- answer-free throughput benchmark and bottleneck profiler reports
- answer-free bottleneck triage, optimization candidate, priority, and shadow plan reports
- shadow reducer optimization, proof-equivalence, and policy reports
- answer-free parallel-efficiency re-benchmark, bottleneck-shift, correctness-regression, and safety-regression reports
- answer-free proof verification graph profiles, batch candidate plans, shadow batch replay, proof batch equivalence, and safety guard reports
- answer-free optimization cycle registry, system re-benchmark, multi-version comparison, optimization ledger, and cycle policy reports
- answer-free native hot path profiles, native candidate taxonomy reports, boundary contracts, Python-to-native interface contracts, shadow native replay plans, and native candidate safety guard reports
- shadow-only native graph traversal prototype reports, Python baseline reports,
  verifier revalidation reports, safety guard reports, and rollback pointers
- shadow-only native graph traversal integration reports, baseline-vs-native
  runtime reports, equivalence reports, fallback reports, safety guard reports,
  and native boundary overhead evidence
- shadow-only native boundary overhead breakdowns, reduced boundary adapter
  reports, baseline-vs-reduced replay reports, equivalence reports, fallback
  reports, safety guard reports, and rollback pointers
- shadow-only native graph system benchmark workload reports, baseline
  comparison reports, runtime reports, scaling reports, equivalence
  revalidation reports, fallback validation reports, and safety guard reports
- shadow-only staged native policy candidate reports, decision simulation
  reports, gated policy replay reports, policy registry reports, policy
  rollback pointers, equivalence reports, fallback/disable reports, and safety
  guard reports
- shadow-only aggressive native canary plans, canary replay reports,
  equivalence reports, fallback reports, discard-on-equivalence-failure probes,
  and safety guard reports
- shadow-only native canary stress matrices, eligibility boundary reports,
  canary execution reports, equivalence guard reports, shard safety reports,
  failed-native learning-promotion block evidence, and safety guard reports
- shadow-only native clean-path performance reports, stress cost decomposition
  reports, native promotion candidate reports, equivalence overhead reports,
  clean-path safety guard reports, rejection reason records, and rollback
  pointers
- runtime mode matrices, runtime mode router reports, service latency reports,
  learning shadow reports, development gate preservation reports, and runtime
  mode safety guard reports
- service request/response/error envelope contracts, in-process service runtime
  harness reports, service_fast probe reports, service_verified probe reports,
  service error probe reports, learning shadow hook stub reports, and service
  runtime safety guard reports
- learning shadow queue schema reports, learning job envelope reports, service
  enqueue boundary reports, queue drain stub reports, queue backpressure stub
  reports, learning queue safety guard reports, and rollback-safe queue
  boundary evidence
- local-only service API schema reports, local API harness reports, local JSON
  request fixtures, service API probe reports, service API safety guard
  reports, and benchmark-harness isolation evidence
- local-only service API fuzz suite reports, request sanitizer reports,
  boundary hardening reports, fuzz execution reports, hardened error envelope
  evidence, service API contract safety guard reports, and rejected or
  safety-blocked negative fixtures
- shadow worker drain reports, learning job validation reports, learning
  candidate replay reports, shadow promotion candidate reports, learning job
  rejection reports, queue state reports, quarantine records, and shadow worker
  safety guard reports
- promotion candidate gate reports, provenance reports, replay evidence reports,
  safety evidence reports, service impact reports, rollback readiness reports,
  non-applying staged promotion delta reports, promotion package reports,
  rejected candidate reports, quarantine interaction reports, and promotion gate
  safety guard reports
- Twin-Core pointer discovery reports, pointer validation reports, role
  separation reports, protected-runner compatibility reports, validation mode
  classification reports, staged promotion preflight reports, and Twin-Core
  safety guard reports that do not create or repair the pointer
- Twin-Core identity manifest reports, registry pointer reports, approval
  policy reports, mutation policy reports, rollback policy reports, pointer
  validation reports, guarded-runner compatibility reports, and v0.47 re-entry
  readiness reports that keep production apply disabled
- staged registry input acceptance reports, staged package load reports,
  Twin-Core gate check reports, shadow registry dry-run reports, rollback
  replay reports, production contamination reports, Twin-Core approval
  simulation reports, and staged registry dry-run safety guard reports that
  do not apply deltas while Twin-Core readiness is false
- staged registry input acceptance reports, staged package load reports,
  Twin-Core gate check reports, shadow registry dry-run reports, shadow
  service impact replay reports, rollback replay reports, production
  contamination reports, Twin-Core approval simulation reports, and staged
  registry dry-run safety guard reports that apply deltas only to shadow
  registries after Twin-Core readiness is explicit
- Twin-Core review package reports, review checklist reports, per-delta review
  records, approval package reports, rejection/hold reports, and review safety
  guard reports that keep production apply disabled and require a later
  production apply gate
- approval package load reports, production apply plan reports, target registry
  mapping reports, production snapshot compatibility reports, operator approval
  stubs, apply gate decision reports, dry-run apply simulation reports,
  rollback plan verification reports, service replay requirement reports,
  production write block reports, and apply gate safety guard reports that keep
  operator approval ungranted and all production writes blocked
- non-production sandbox reports, sandbox snapshot reports, sandbox apply
  reports, sandbox registry consistency reports, post-apply service replay
  reports, post-apply API boundary fuzz replay reports, learning queue boundary
  replay reports, sandbox rollback replay reports, sandbox contamination check
  reports, and sandbox safety guard reports that keep production apply disabled
  and all production writes blocked
- service regression suite reports, service latency budget reports, service
  path isolation reports, API boundary regression reports, learning queue
  boundary regression reports, sandbox non-contamination recheck reports, and
  service regression safety guard reports that do not call development or
  benchmark paths and do not mutate production
- rollback-capable shadow artifacts

## v0.61 Clean Repository Snapshot Rules

The GitHub snapshot is a clean source and design checkpoint, not a runtime
artifact archive.

Allowed in the snapshot:

- source code
- tools
- tests when present
- docs
- README and PROJECT_STATE
- config files
- frozen service sample requests
- small answer-free fixtures needed for clean clone reproduction

Forbidden in the snapshot:

- old .git history
- persona/reports
- external-benchmark-training outputs
- capability_sprint_workspace outputs
- benchmark result dumps
- generated reports
- replay outputs
- logs
- caches
- virtual environments
- datasets
- models
- checkpoints
- large media files

Safety checks must record:

```text
old_report_artifacts_committed = false
old_git_history_preserved = false
public_api_exposed = false
network_server_created = false
expected_answer_leakage_count = 0
output_cache_used = false
problem_id_lookup_used = false
teacher_signal_used = false
verifier_bypass_detected = false
production_mutation = false
```

If clean clone reproduction requires a runtime manifest, commit only a compact
answer-free fixture. Do not commit full runtime report directories.
