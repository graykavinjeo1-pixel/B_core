# SEM-33 planning literature adapter

This adapter carries forward the sealed SEM-32 mechanism-level audit and adds the one planning family named by SEM-33 that was not already present there.

| Source | SEM-33 mechanism-level use | Explicitly excluded canonical transplant |
|---|---|---|
| MuZero | planning through a predictive model rather than reactive surface matching | neural reward/value/policy heads and MCTS authority |
| Dreamer / DreamerV3 | bounded imagination followed by real observation | actor-critic training and latent reward authority |
| Director | autonomous high-level subgoal decomposition | latent goal policy transplant |
| [Hieros](https://arxiv.org/abs/2310.05167) | separate subgoal depth from temporal horizon and permit several planning timescales | S5 world model and hierarchical policy transplant |
| TD-MPC2 | bounded local trajectory evaluation | CEM/MPC and opaque neural latent authority |
| RC-aux | distinguish prediction from budget-conditioned reachability/plannability | learned Euclidean reachability authority |
| FF-JEPA | high-level subgoal prediction with short lower-level causal planning | latent planner and action optimizer transplant |

The adopted B_Core properties are semantic goals, causal reachability, backward subgoal synthesis, forward verification, bounded local rollout, uncertainty-aware information actions, and closed-loop replanning. No complete architecture, neural policy/value component, reward model, MCTS, or CEM is imported.

`WHOLE_PLANNER_ARCHITECTURE_TRANSPLANTS=0`

The external Hieros read occurred during literature preflight. Canonical fixture generation, planning, execution, verification, and acceptance remain local and offline.
