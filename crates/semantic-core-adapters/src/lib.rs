//! Adapters are deliberately outside `dockable-semantic-core`.

pub mod generic;
pub mod language;

pub use generic::DeterministicOffsetCapability;
pub use language::{LanguageAdapter, LanguageAdapterError};
