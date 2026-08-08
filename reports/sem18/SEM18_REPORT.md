# SEM-18 Growth-Law Diagnosis Report

- Status: `"PASS"`
- Disposition: `"GROWTH_LIMIT_DIAGNOSED_AND_GENESIS_EFFICIENCY_ACCELERATED_WITH_LINEAR_FRONTIER_YIELD"`
- Reconciled SEM-17 metric: `true`
- Diagnosed limit: `"CAPABILITY_INDEPENDENCE_LIMIT"`
- Direct wave gains: `24`, `24`, `24`, `24`
- Growth regime: `"LINEAR"`
- Genesis costs: `120`, `96`, `83`, `72`
- Frontier-yield acceleration: `false`
- Genesis-efficiency acceleration: `true`
- Wall-time acceleration: `false`
- Final fresh blind solved: `192`
- Next stage: `"OPERATOR_REVIEW_FOR_SEM19"`

The reconciled wave metric counts newly solved tasks on each unopened 24-case wave-local target bank. The additional 72 SEM-17 final-blind gains came from cross-capability reuse and transfer, not from the three direct wave-gain observations. SEM-18 therefore retains a `LINEAR` frontier-yield classification. G1 changes the capability-genesis process: later waves reuse verified schema roles and preserve the same direct yield at monotonically lower deterministic genesis cost. Its ON/OFF ablation restores the independent-genesis cost when disabled. No wall-time acceleration is inferred unless the predeclared 10% threshold is met.
