use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldSet {
    Development,
    FinalHoldout,
    NovelPrediction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldFamily {
    EcologicalRelay,
    CatalyticExchange,
    TemporalContainment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticVariable {
    Signal,
    Context,
    Catalyst,
    Load,
    Phase,
    Buffer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafeIntervention {
    pub variable: SemanticVariable,
    pub value: i16,
    pub cost: u16,
    pub disturbance: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlindWorldCase {
    pub case_id: u64,
    pub set: WorldSet,
    pub family: WorldFamily,
    pub entity_bindings: Vec<u32>,
    pub relation_topology: Vec<u8>,
    pub temporal_context: u16,
    pub visible_state: BTreeMap<SemanticVariable, i16>,
    pub existing_law_prediction: i16,
    pub planning_importance: u16,
    pub allowed_interventions: Vec<SafeIntervention>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlindObservation {
    pub case_id: u64,
    pub family: WorldFamily,
    pub intervention: Option<SafeIntervention>,
    pub visible_state_after_intervention: BTreeMap<SemanticVariable, i16>,
    pub existing_law_prediction: i16,
    pub observed_outcome: i16,
    pub observation_ordinal: u64,
}

impl BlindObservation {
    pub fn residual(&self) -> i16 {
        self.observed_outcome - self.existing_law_prediction
    }
}

pub trait SafeClosedWorld {
    fn public_cases(&self) -> Vec<BlindWorldCase>;
    fn observe(
        &mut self,
        case_id: u64,
        intervention: Option<SafeIntervention>,
    ) -> Result<BlindObservation, String>;
    fn outcome_reads(&self) -> u64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HiddenDynamics {
    ContextualInteraction,
    CatalyticRelation,
    DelayedBufferRelease,
    LowValueOffset,
    IrreducibleNoise,
    RedundantExistingLaw,
    UnidentifiableLatent,
}

#[derive(Debug, Clone)]
struct SealedWorldSpec {
    public: BlindWorldCase,
    hidden: HiddenDynamics,
    coefficient: i16,
    noise_salt: u64,
}

#[derive(Debug, Clone)]
pub struct WorldOracle {
    specifications: BTreeMap<u64, SealedWorldSpec>,
    observation_ordinal: u64,
    outcome_reads: u64,
}

impl WorldOracle {
    pub fn sealed(set: WorldSet, seed: u64, count: usize) -> Self {
        let mut state = seed;
        let mut specifications = BTreeMap::new();
        for index in 0..count {
            state = splitmix64(state);
            let family = match index % 3 {
                0 => WorldFamily::EcologicalRelay,
                1 => WorldFamily::CatalyticExchange,
                _ => WorldFamily::TemporalContainment,
            };
            let hidden = match index % 8 {
                0 | 3 => HiddenDynamics::ContextualInteraction,
                1 => HiddenDynamics::CatalyticRelation,
                2 => HiddenDynamics::DelayedBufferRelease,
                4 => HiddenDynamics::LowValueOffset,
                5 => HiddenDynamics::IrreducibleNoise,
                6 => HiddenDynamics::RedundantExistingLaw,
                _ => HiddenDynamics::UnidentifiableLatent,
            };
            let case_id = state ^ ((index as u64 + 1) << 32) ^ set_tag(set);
            let signal = i16::from(((state >> 4) & 1) as u8);
            let context = i16::from(((state >> 7) & 1) as u8);
            let catalyst = i16::from(((state >> 10) & 1) as u8);
            let load = 1 + i16::from(((state >> 12) % 3) as u8);
            let phase = i16::from(((state >> 15) & 1) as u8);
            let buffer = i16::from(((state >> 18) & 1) as u8);
            let visible_state = [
                (SemanticVariable::Signal, signal),
                (SemanticVariable::Context, context),
                (SemanticVariable::Catalyst, catalyst),
                (SemanticVariable::Load, load),
                (SemanticVariable::Phase, phase),
                (SemanticVariable::Buffer, buffer),
            ]
            .into_iter()
            .collect::<BTreeMap<_, _>>();
            let existing_law_prediction = signal + load - buffer;
            let allowed_interventions = if hidden == HiddenDynamics::UnidentifiableLatent {
                vec![SafeIntervention {
                    variable: SemanticVariable::Load,
                    value: 2,
                    cost: 1,
                    disturbance: 1,
                }]
            } else {
                vec![
                    SafeIntervention {
                        variable: SemanticVariable::Signal,
                        value: 1 - signal,
                        cost: 2,
                        disturbance: 1,
                    },
                    SafeIntervention {
                        variable: SemanticVariable::Context,
                        value: 1 - context,
                        cost: 2,
                        disturbance: 1,
                    },
                    SafeIntervention {
                        variable: SemanticVariable::Catalyst,
                        value: 1 - catalyst,
                        cost: 3,
                        disturbance: 2,
                    },
                    SafeIntervention {
                        variable: SemanticVariable::Phase,
                        value: 1 - phase,
                        cost: 2,
                        disturbance: 1,
                    },
                ]
            };
            let public = BlindWorldCase {
                case_id,
                set,
                family,
                entity_bindings: vec![
                    10_000 + index as u32 * 11 + u32::from((state & 7) as u8),
                    20_000 + index as u32 * 13 + u32::from(((state >> 3) & 7) as u8),
                ],
                relation_topology: vec![
                    1 + (index % 5) as u8,
                    11 + ((state >> 8) % 7) as u8,
                    31 + (index % 3) as u8,
                ],
                temporal_context: 20 + index as u16 * 3 + ((state >> 22) % 3) as u16,
                visible_state,
                existing_law_prediction,
                planning_importance: match hidden {
                    HiddenDynamics::ContextualInteraction
                    | HiddenDynamics::CatalyticRelation
                    | HiddenDynamics::DelayedBufferRelease => 8,
                    HiddenDynamics::LowValueOffset => 1,
                    HiddenDynamics::IrreducibleNoise => 2,
                    HiddenDynamics::RedundantExistingLaw => 1,
                    HiddenDynamics::UnidentifiableLatent => 5,
                },
                allowed_interventions,
            };
            specifications.insert(
                case_id,
                SealedWorldSpec {
                    public,
                    hidden,
                    coefficient: 4 + (index % 3) as i16,
                    noise_salt: state.rotate_left(23),
                },
            );
        }
        Self {
            specifications,
            observation_ordinal: 0,
            outcome_reads: 0,
        }
    }

    pub fn public_fingerprint(&self) -> String {
        let public = self.public_cases();
        let bytes = serde_json::to_vec(&public).expect("serializable public worlds");
        format!("{:x}", Sha256::digest(bytes))
    }
}

impl SafeClosedWorld for WorldOracle {
    fn public_cases(&self) -> Vec<BlindWorldCase> {
        self.specifications
            .values()
            .map(|specification| specification.public.clone())
            .collect()
    }

    fn observe(
        &mut self,
        case_id: u64,
        intervention: Option<SafeIntervention>,
    ) -> Result<BlindObservation, String> {
        let specification = self
            .specifications
            .get(&case_id)
            .ok_or("SEM36_UNKNOWN_WORLD_CASE")?;
        if let Some(candidate) = &intervention {
            if !specification
                .public
                .allowed_interventions
                .contains(candidate)
            {
                return Err("SEM36_UNSAFE_OR_UNDECLARED_INTERVENTION".to_string());
            }
        }
        let mut state = specification.public.visible_state.clone();
        if let Some(candidate) = &intervention {
            state.insert(candidate.variable, candidate.value);
        }
        let signal = state[&SemanticVariable::Signal];
        let context = state[&SemanticVariable::Context];
        let catalyst = state[&SemanticVariable::Catalyst];
        let load = state[&SemanticVariable::Load];
        let phase = state[&SemanticVariable::Phase];
        let buffer = state[&SemanticVariable::Buffer];
        let existing = signal + load - buffer;
        let residual = match specification.hidden {
            HiddenDynamics::ContextualInteraction => specification.coefficient * signal * context,
            HiddenDynamics::CatalyticRelation => {
                specification.coefficient * catalyst * i16::from(load > 1)
            }
            HiddenDynamics::DelayedBufferRelease => specification.coefficient * phase * buffer,
            HiddenDynamics::LowValueOffset => i16::from(context > 0),
            HiddenDynamics::IrreducibleNoise => {
                let mixed = splitmix64(
                    specification.noise_salt
                        ^ self.observation_ordinal
                        ^ intervention_tag(intervention.as_ref()),
                );
                match mixed % 3 {
                    0 => -5,
                    1 => 0,
                    _ => 5,
                }
            }
            HiddenDynamics::RedundantExistingLaw => 0,
            HiddenDynamics::UnidentifiableLatent => specification.coefficient,
        };
        self.observation_ordinal += 1;
        self.outcome_reads += 1;
        Ok(BlindObservation {
            case_id,
            family: specification.public.family,
            intervention,
            visible_state_after_intervention: state,
            existing_law_prediction: existing,
            observed_outcome: existing + residual,
            observation_ordinal: self.observation_ordinal,
        })
    }

    fn outcome_reads(&self) -> u64 {
        self.outcome_reads
    }
}

fn set_tag(set: WorldSet) -> u64 {
    match set {
        WorldSet::Development => 0xD36D_0000_0000_0001,
        WorldSet::FinalHoldout => 0xF36F_0000_0000_0002,
        WorldSet::NovelPrediction => 0xA36A_0000_0000_0003,
    }
}

fn intervention_tag(intervention: Option<&SafeIntervention>) -> u64 {
    intervention.map_or(0, |candidate| {
        (candidate.variable as u64 + 1) << 40 ^ (candidate.value as i64 as u64).rotate_left(9)
    })
}

fn splitmix64(mut state: u64) -> u64 {
    state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut mixed = state;
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    mixed ^ (mixed >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_world_transport_contains_no_hidden_mechanism_or_gold() {
        let oracle = WorldOracle::sealed(WorldSet::Development, 11, 18);
        let encoded = serde_json::to_string(&oracle.public_cases()).unwrap();
        assert!(!encoded.contains("HiddenDynamics"));
        assert!(!encoded.contains("CONTEXTUAL_INTERACTION"));
        assert!(!encoded.contains("expected_discovery"));
    }

    #[test]
    fn development_and_final_public_worlds_are_disjoint() {
        let development = WorldOracle::sealed(WorldSet::Development, 11, 18);
        let final_worlds = WorldOracle::sealed(WorldSet::FinalHoldout, 19, 24);
        assert_ne!(
            development.public_fingerprint(),
            final_worlds.public_fingerprint()
        );
    }
}
