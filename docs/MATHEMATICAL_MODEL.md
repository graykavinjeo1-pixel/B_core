# SYNAPSE CORE Mathematical Model

This document defines the language-independent mathematical model for SYNAPSE
CORE. The model is intended to be implementable in Python, Rust, Mojo, or any
future runtime.

## 1. Notation

Let the cognitive graph be:

```text
G = (V, E)
```

Where:

- `V` is the set of neuron nodes.
- `E` is the set of directed synapses.
- `i, j, k` are node indices.
- `t` is the current reasoning cycle.
- `a_i(t)` is activation of node `i` at cycle `t`.
- `theta_i` is firing threshold of node `i`.
- `I_i` is long-term importance of node `i`.
- `p_i` is plasticity of node `i`.
- `d_i` is decay rate of node `i`.
- `w_ij` is synapse strength from node `i` to node `j`.
- `r_ij` is relation type of synapse `i -> j`.
- `s_i(x)` is similarity between input stimulus `x` and node `i`.

All activation, importance, plasticity, decay, and strength values are clamped:

```text
clip(z) = min(1, max(0, z))
```

## 2. Activation Function

### 2.1 Initial Activation Field

For an input stimulus `x`, only a bounded candidate set is activated:

```text
C(x) = top_k({ i in V | s_i(x) > epsilon_s })
```

Initial activation:

```text
a_i(0) =
  clip(s_i(x) * I_i * M_i)     if i in C(x)
  0                            otherwise
```

Where:

- `M_i` is an optional modulation factor from emotion, desire, or goal state.
- `epsilon_s` is the minimum similarity needed to enter the activation field.

Default modulation:

```text
M_i = 1
```

With emotion, desire, and goal engines:

```text
M_i = 1 + alpha_e E_i + alpha_d D_i + alpha_g G_i
```

Where:

- `E_i` is emotional relevance.
- `D_i` is desire relevance.
- `G_i` is goal relevance.
- `alpha_e`, `alpha_d`, `alpha_g` are tunable weights.

### 2.2 Firing Condition

A node fires when:

```text
fire_i(t) = 1 if a_i(t) > theta_i
fire_i(t) = 0 otherwise
```

## 3. Wave Propagation

For every active node `i`, activation propagates through outgoing synapses.

Raw propagated signal:

```text
m_ij(t) = a_i(t) * w_ij * (1 - d_i) * lambda_r(r_ij)
```

Where:

- `lambda_r(r_ij)` is a relation-type propagation coefficient.
- For normal association/support/cause/sequence relations, `lambda_r > 0`.
- For contradiction relations, propagation is handled by inhibition instead.

Candidate activation for node `j`:

```text
u_j(t + 1) = max(
  a_j(t) * (1 - d_j),
  max_{i -> j in E} m_ij(t)
)
```

Final non-inhibitory activation:

```text
a'_j(t + 1) = clip(u_j(t + 1))
```

Sparse execution rule:

```text
evaluate i only if a_i(t) > epsilon_a
```

`epsilon_a` prevents the whole graph from being evaluated.

## 4. Synapse Reinforcement

Synapse reinforcement follows a Hebbian-style rule:

```text
Delta w_ij = eta * p_i * p_j * a_i(t) * a_j(t) * R_ij(t)
```

Where:

- `eta` is the global learning rate.
- `R_ij(t)` is reinforcement eligibility.

Eligibility:

```text
R_ij(t) =
  1                              if i and j co-activate within tau cycles
  rho_relation(r_ij)             if relation-specific weighting applies
  0                              otherwise
```

Updated strength:

```text
w_ij(t + 1) = clip(w_ij(t) + Delta w_ij - L_ij(t))
```

Where `L_ij(t)` is weakening pressure:

```text
L_ij(t) = eta_l * idle_ij(t) * (1 - I_i) * (1 - I_j)
```

`idle_ij(t)` increases when the synapse is not used.

Relation-specific reinforcement can be expressed as:

```text
rho_relation(association)   = 1.00
rho_relation(support)       = 1.10
rho_relation(cause)         = 1.20
rho_relation(sequence)      = 1.05
rho_relation(hierarchy)     = 0.95
rho_relation(goal)          = 1.25
rho_relation(emotion)       = 1.15
rho_relation(contradiction) = 0.80
```

## 5. Forgetting Function

For node `i`, memory retention is:

```text
R_i(t) = beta_I I_i + beta_A A_i + beta_U U_i + beta_P p_i
```

Where:

- `A_i` is long-term average activation.
- `U_i` is normalized access frequency.
- `beta_*` are retention weights.

Forgetting pressure:

```text
F_i(t) = (1 - R_i(t)) * exp(-gamma * age_i)
```

Node importance update during sleep/consolidation:

```text
I_i(t + 1) = clip(I_i(t) - eta_f F_i(t) + eta_c C_i(t))
```

Where:

- `eta_f` is forgetting rate.
- `eta_c` is consolidation rate.
- `C_i(t)` is consolidation support from repeated activation or crystal use.

Deletion condition:

```text
delete_i = true if
  I_i(t) < epsilon_I
  and A_i < epsilon_A
  and U_i < epsilon_U
```

Synapse forgetting:

```text
w_ij(t + 1) = clip(w_ij(t) * exp(-mu * idle_ij(t)))
```

Synapse deletion:

```text
delete_ij = true if w_ij < epsilon_w
```

## 6. Inhibition Function

Contradictory circuits suppress each other through inhibitory synapses.

For contradiction edge `i -> j`:

```text
h_ij(t) = a_i(t) * w_ij * kappa
```

Where `kappa` is global inhibition strength.

Activation after inhibition:

```text
a_j(t + 1) = clip(a'_j(t + 1) - sum_{i -> j, r_ij = contradiction} h_ij(t))
```

If activation falls below the active floor:

```text
a_j(t + 1) = 0 if a_j(t + 1) < epsilon_a
```

## 7. Resonance Criterion

At every reasoning cycle, compute mean activation delta:

```text
Delta(t) = (1 / |A_t|) * sum_{i in A_t} |a_i(t) - a_i(t - 1)|
```

Where:

```text
A_t = { i | a_i(t) > epsilon_a or a_i(t - 1) > epsilon_a }
```

Resonance is achieved when:

```text
Delta(t) < epsilon_res
```

for `N` consecutive cycles:

```text
resonance = true if
  Delta(t - n) < epsilon_res
  for all n in {0, 1, ..., N - 1}
```

The resulting stable activation pattern is the `Thought State`:

```text
ThoughtState = { (i, a_i) | a_i > epsilon_a }
```

## 8. Competition Algorithm

Competition operates over candidate clusters, not isolated nodes.

### 8.1 Cluster Formation

For each active root node `r`:

```text
K_r = BFS_r(depth = D, include node j if a_j > epsilon_a)
```

Only active or recently active nodes are included.

### 8.2 Cluster Score

Cluster score:

```text
Score(K) =
  alpha_A * sum_{i in K} a_i
  + alpha_W * sum_{(i,j) in E_K} w_ij
  + alpha_I * mean_{i in K} I_i
  + alpha_G * GoalFit(K)
  + alpha_C * Coherence(K)
  - alpha_X * Contradiction(K)
```

Where:

- `E_K` is the set of synapses internal to cluster `K`.
- `GoalFit(K)` measures alignment with active goals.
- `Coherence(K)` measures relation consistency.
- `Contradiction(K)` measures internal contradiction pressure.

Internal coherence:

```text
Coherence(K) =
  support_edges(K) + cause_edges(K) + sequence_edges(K)
  ------------------------------------------------------
              max(1, total_edges(K))
```

Contradiction pressure:

```text
Contradiction(K) =
  sum_{(i,j) in E_K, r_ij = contradiction} a_i * a_j * w_ij
```

Winner:

```text
K* = argmax_K Score(K)
```

### 8.3 Soft Competition

Instead of keeping only the winner, cluster weights can be normalized:

```text
P(K_m) = exp(Score(K_m) / T) / sum_n exp(Score(K_n) / T)
```

Where `T` is competition temperature.

Low temperature creates decisive thought. High temperature preserves ambiguity.

### 8.4 Winner-Take-Most Update

After scoring:

```text
a_i = clip(a_i * (1 + sigma * P(K*)))       if i in K*
a_i = clip(a_i * (1 - omega * P(K*)))       if i not in K*
```

Where:

- `sigma` is winner amplification.
- `omega` is loser suppression.

## 9. Thought Crystal Formation

When resonance is achieved, the winning cluster is compressed:

```text
Crystal = compress(K*)
```

Crystal activation:

```text
a_c = max_{i in K*} a_i
```

Crystal importance:

```text
I_c = clip(
  zeta_A * mean_{i in K*} a_i
  + zeta_I * mean_{i in K*} I_i
  + zeta_S * normalized Score(K*)
)
```

Crystal creation condition:

```text
create_crystal = true if
  resonance = true
  and Score(K*) > epsilon_K
  and |K*| >= min_cluster_size
```

Repeated equivalent crystals can later be promoted into reflex circuits.
