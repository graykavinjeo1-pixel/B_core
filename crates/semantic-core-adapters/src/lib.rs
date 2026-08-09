//! Adapters are deliberately outside `dockable-semantic-core`.

pub mod external_problem;
pub mod generic;
pub mod language;

pub use external_problem::{
    AdapterCompatibility, ExecutableObservation, ExternalAdapterError, ExternalGoalIr,
    ExternalProblemAdapter, ExternalProblemInput, RepositoryArtifact,
};
pub use generic::DeterministicOffsetCapability;
pub use language::{LanguageAdapter, LanguageAdapterError};
