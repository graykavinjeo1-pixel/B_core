use crate::{ActivationIndexReport, NeuronId, SynapseCore};

#[derive(Debug, Clone)]
pub struct ActivationIndex;

impl ActivationIndex {
    pub fn candidates(core: &SynapseCore, stimulus: &str) -> Vec<NeuronId> {
        core.activation_candidates(stimulus)
    }

    pub fn report(core: &SynapseCore, stimulus: &str, limit: usize) -> ActivationIndexReport {
        core.activation_index_report(stimulus, limit)
    }
}
