# SEM-32 World-Model Literature Mechanism Audit

Status: `FROZEN_DESIGN_INPUT_CANDIDATE`

Scope: mechanism audit for the independent B_Core semantic-reasoning lineage. This is not a benchmark ranking, architecture-selection contest, or permission to import a neural world model. The canonical SEM-32 run remains local, offline, CPU-capable, language-free on the reasoning path, and independently verified.

## Admission rule

Every retained mechanism was screened on four axes:

- `MECHANISM_VALUE`: does it address prediction, uncertainty, causality, factorization, sparsity, or reachability?
- `SEMANTIC_COMPATIBILITY`: can the principle operate on typed semantic entities, relations, belief states, mechanisms, and deltas rather than pixels or an opaque latent?
- `CAUSAL_VALUE`: does it improve intervention, counterfactual, hidden-context, or mechanism reasoning rather than correlation alone?
- `RESOURCE_VALUE`: can the adapted form remain bounded, sparse, local, offline, and CPU-capable?

Paper reputation, scale, citation count, reported score, and parameter count have no authority. `WHOLE_ARCHITECTURE_TRANSPLANTS=0`.

## Classification matrix

| Research line | Primary class | Value screen (M/S/C/R) | Mechanism retained for SEM-32 | Canonical exclusion |
|---|---|---|---|---|
| [World Models](https://arxiv.org/abs/1803.10122) | ADAPT_TO_B_CORE | H/M/L/H | compact predictive internal state and rollout as a testable principle | VAE/RNN latent, image generation, dream-trained controller |
| [PlaNet](https://arxiv.org/abs/1811.04551) | ADAPT_TO_B_CORE | H/M/M/M | belief-state update, deterministic/stochastic separation, open-loop multi-step evaluation | pixel encoder/decoder, latent planner, reward model |
| [Dreamer](https://arxiv.org/abs/1912.01603) | ADAPT_TO_B_CORE | M/M/L/M | rollout inside a learned dynamics representation, evaluated only as semantic prediction | actor/critic, policy gradients, image latent authority |
| [DreamerV3](https://arxiv.org/abs/2301.04104) | DEFER | M/L/L/L | robustness lesson only; no SEM-32 mechanism dependency | RL agent, reward/value heads, pixel-scale training |
| [Dreamer4](https://arxiv.org/abs/2509.24527) | SIMULATOR_ONLY | M/L/L/L | future external simulation comparison only | video world model, transformer/GPU dependency, imagined policy training |
| [MuZero](https://arxiv.org/abs/1911.08265) | REJECT_AS_CANONICAL | M/L/L/L | none for SEM-32; planning is explicitly deferred | reward/value/policy-only task latent and MCTS as world-state authority |
| [TD-MPC2](https://arxiv.org/abs/2310.16828) | DEFER | M/L/L/L | local trajectory optimization is a later planning comparison | decoder-free opaque latent, CEM/MPC, large neural agent |
| [DINO-WM](https://arxiv.org/abs/2411.04983) | PERCEPTION_ONLY | M/L/L/L | predictive abstraction without reconstruction for a future vision adapter | DINO features and action optimization as canonical semantics |
| [V-JEPA 2](https://arxiv.org/abs/2506.09985) | PERCEPTION_ONLY | M/L/L/L | representation-space prediction as a future observation adapter principle | internet-video encoder, robot action model, GPU runtime |
| [V-JEPA 2.1](https://arxiv.org/abs/2603.14482) | PERCEPTION_ONLY | M/L/L/L | dense temporally coherent perception features for a future adapter | multimodal tokenizer and dense video model in the core |
| [MuDreamer](https://arxiv.org/abs/2405.15083) | ADAPT_TO_B_CORE | H/H/L/H | avoid reconstructing irrelevant distractors; predict task-relevant abstract structure directly | value/action auxiliary heads as semantic authority |
| [Dreamer-CDP](https://arxiv.org/abs/2603.07083) | ADAPT_TO_B_CORE | M/H/L/H | continuous representation prediction without observation reconstruction | Dreamer agent and learned dense representation as canonical memory |
| [Slot Attention](https://arxiv.org/abs/2006.15055) | PERCEPTION_ONLY | M/M/L/M | future perceptual proposal mechanism for entity candidates | visual slots as already-grounded entity identity |
| [Interaction Networks](https://arxiv.org/abs/1612.00222) | ADOPT | H/H/H/H | object/relation-factored, relation-local interaction mechanisms | neural message vectors and fixed architecture |
| [Graph Network Simulator](https://arxiv.org/abs/2002.09405) | ADOPT | H/H/M/H | sparse graph-local rollout, explicit interaction neighborhoods, invariant checking | particle simulator and learned neural dynamics wholesale |
| [Recurrent Independent Mechanisms](https://arxiv.org/abs/1909.10893) | ADOPT | H/H/M/H | sparse activation of independent mechanisms with explicit communication dependencies | recurrent neural modules and attention weights as meaning |
| [Slot Structured World Models](https://arxiv.org/abs/2402.03326) | ADAPT_TO_B_CORE | H/H/M/M | object-centric graph dynamics and entity-count/topology generalization tests | slot encoder and latent GNN implementation |
| [Causal-JEPA](https://arxiv.org/abs/2602.11389) | ADAPT_TO_B_CORE | H/H/H/M | structured partial observability and object-level masking as shortcut resistance | image-object latent and JEPA loss in the core |
| [Towards Causal Representation Learning](https://arxiv.org/abs/2102.11107) | ADOPT | H/H/H/H | explicit causal variables/mechanisms, interventions, distribution shifts, identifiability caution | any claim that representation alone proves causality |
| [Interventional Causal Representation Learning](https://arxiv.org/abs/2209.11924) | ADOPT | H/H/H/H | intervention-supported identification and competing-hypothesis preservation | neural representation estimator as required architecture |
| [Meta-Causal World](https://arxiv.org/abs/2506.23068) | ADAPT_TO_B_CORE | H/H/H/M | hidden causal regimes, context-triggered submechanisms, causality-seeking interventions | latent meta-state neural agent wholesale |
| [Relational Structural Causal Models](https://arxiv.org/abs/2606.14892) | ADOPT | H/H/H/H | relational SCM-style applicability across varying objects and unseen combinations; non-identifiability checks | relational neural SCM implementation as mandatory core |
| [PETS](https://arxiv.org/abs/1805.12114) | ADOPT | H/H/M/H | separate epistemic model uncertainty from stochastic transition uncertainty; propagate both | probabilistic neural ensemble and MPC |
| [MOPO](https://arxiv.org/abs/2005.13239) | ADAPT_TO_B_CORE | H/H/M/H | explicit unsupported-model boundary and conservative confidence under OOD rollout | offline policy optimization and reward penalty machinery |
| [Plan2Explore](https://arxiv.org/abs/2005.05960) | ADAPT_TO_B_CORE | H/H/M/H | disagreement as reducible-information signal, bounded by irreducible-noise detection | exploration policy/actor training |
| [Director](https://arxiv.org/abs/2206.04114) | DEFER | M/L/L/L | hierarchical subgoal lesson reserved for SEM-33 planning | latent goal policy and image-decoded planner |
| [FF-JEPA](https://arxiv.org/abs/2606.09311) | DEFER | M/L/L/L | hierarchical horizon decomposition reserved for planning | latent subgoal planner and CEM-style action optimization |
| [RC-aux](https://arxiv.org/abs/2605.07278) | ADOPT | H/H/H/H | budget-conditioned reachability, temporal hard negatives, and prediction-versus-plannability separation | learned Euclidean latent distance or planner in SEM-32 |
| [Genie](https://arxiv.org/abs/2402.15391) | SIMULATOR_ONLY | L/L/L/L | possible future synthetic observation source outside canonical authority | generative interactive video as world memory |
| [GameNGen](https://arxiv.org/abs/2408.14837) | SIMULATOR_ONLY | L/L/L/L | possible external simulator comparison only | diffusion game engine in the core |
| [Cosmos](https://arxiv.org/abs/2501.03575) | SIMULATOR_ONLY | L/L/L/L | possible future physical-observation generator outside the core | foundation video model, tokenizer, GPU infrastructure |

`M/S/C/R` abbreviates mechanism, semantic compatibility, causal value, and resource value. `H/M/L` are qualitative design-screen outcomes, not paper scores.

## Frozen mechanism decisions

Directly adopted mechanisms (`8`):

1. relation-local object interaction;
2. graph-local multi-step dynamics with invariant checks;
3. sparsely activated independent causal mechanisms;
4. observation/intervention separation and identifiability caution;
5. intervention-supported hypothesis reduction;
6. relational causal applicability across identities/cardinalities/topologies;
7. epistemic versus aleatoric uncertainty separation;
8. budget-conditioned reachability distinct from semantic similarity.

Adapted mechanisms (`10`):

1. compact internal predictive state becomes typed persistent/belief/active semantic layers;
2. latent belief update becomes evidence-backed semantic belief revision;
3. imagined rollout becomes open-loop semantic-delta rollout;
4. reconstruction-free prediction becomes direct structured-delta prediction;
5. distractor robustness becomes active-slice relevance without world amnesia;
6. object-centric graph dynamics becomes entity/relation/context mechanism IR;
7. object masking becomes explicit partial observability and hidden-state hypotheses;
8. meta-causal regimes become context-indexed competing mechanisms;
9. uncertainty penalties become explicit unsupported-model boundaries;
10. disagreement exploration becomes bounded expected causal information selection with stochastic-noise rejection.

Rejected as canonical (`5` paper lines): MuZero's reward/value/policy-only latent plus the three generative-video systems and Dreamer4's video simulator path. Deferred planning systems and perception-only systems are not counted as canonical rejections, because their later adapter/comparison roles remain open.

## SEM-32 architecture consequences

- Canonical state has three non-authority-equivalent layers: persistent semantic world, uncertain belief world, and a read-only sparse active projection.
- Prediction emits typed deltas and continuity commitments, never a future full-world copy.
- The fixture contains deterministic, stochastic, hidden-context, delayed, distractor, relational-transfer, and unreachable-shortcut cases.
- Observational data may create hypotheses but cannot promote a causal mechanism without interventional support.
- The independent verifier may know fixture truth; the B_Core path sees only permitted observations after its prediction is frozen.
- Counterfactuals use an anchor plus copy-on-write deltas and cannot mutate actual history.
- Reachability requires a mechanism path certificate within budget; semantic closeness is never sufficient.
- No pixel reconstruction, text prediction, reward/value-only latent, actor/critic, MCTS, CEM, video generator, external teacher, GPU runtime, or whole architecture is part of canonical SEM-32.

## Freeze declarations

```text
LITERATURE_REFERENCE_COUNT=31
LITERATURE_MECHANISMS_ADOPTED=8
LITERATURE_MECHANISMS_ADAPTED=10
LITERATURE_MECHANISMS_REJECTED_AS_CANONICAL=5
WHOLE_ARCHITECTURE_TRANSPLANTS=0
PAPERS_FETCHED_DURING_CANONICAL_RUN=0
LITERATURE_IS_SUCCESS_AUTHORITY=false
```

This audit is design input only. All SEM-32 scientific claims require frozen local fixtures, pre-observation prediction commitments, independent mechanical verification, raw sequences, and causal ablations.
