# SEM-27 LIVE AUTONOMOUS STAIRCASE OBSERVATION

Start from the exact sealed SEM-27-R2 descendant at commit
`9dbebdd280db2c292bff4865bcf8f1d8c39c335d` and state
`reports/sem27_r2/final_r2_state.json`.

The purpose is to test whether the already-autonomous system eventually performs a
second genuine structural difficulty transition. This is a continuation, not SEM-28.

## Frozen boundaries

- Do not modify the reasoning engine, autonomous policy, plateau classifier,
  repair synthesis, difficulty selection policy, evaluator, or R1 ontology.
- Do not select a research topic, bottleneck, repair, experiment, frontier, or
  difficulty dimension on behalf of the system.
- Use the existing closed mechanical work universe and fixed resource ceiling.
- Future concrete instances remain unopened until their frozen epoch request.
- Human research and difficulty-selection event counts remain zero.

## Campaign freeze

The operator may select only the fixed epoch budget before launch. The campaign
seed, per-epoch seed commitments, predecessor state hash, engine hash, evaluator
hash, runner hash, toolchain, and budget are frozen before the first epoch.

The default budget is 256 autonomous continuation epochs. The frozen budget may
not be changed after any outcome is observed. A stopped campaign may resume only
against the same freeze.

## Success condition

Stop successfully when raw artifacts mechanically establish all of:

- `current_regime_id >= 3`;
- cumulative difficulty transition count is at least 2;
- the new transition has `operator_selected=false`;
- every structural dimension is monotonic and at least one strictly increases;
- human difficulty escalation and level-selection events remain zero.

Otherwise stop at the fixed budget, an inherited autonomous/safety stop, or an
operator-requested epoch boundary. An operator stop is resumable and is not a
scientific outcome.
