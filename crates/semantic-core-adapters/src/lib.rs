//! Adapters are deliberately outside `dockable-semantic-core`.

pub mod cognitive;
pub mod generic;
pub mod language;
pub mod language_knowledge;

pub use cognitive::{
    CognitiveApi, CognitiveApiCommandIR, CognitiveApiError, CognitiveApiPayloadIR,
    CognitiveApiResponseIR, NaturalLanguageOutputIR, NaturalLanguageRequestIR,
    NaturalLanguageResponseIR, NATURAL_LANGUAGE_REQUEST_SCHEMA, NATURAL_LANGUAGE_RESPONSE_SCHEMA,
};
pub use generic::DeterministicOffsetCapability;
pub use language::{LanguageAdapter, LanguageAdapterError};
pub use language_knowledge::{
    LanguageCodeIR, LanguageKnowledgeBase, LanguageKnowledgeCategoryIR, LanguageKnowledgeEntryIR,
    LanguageKnowledgeError, LanguageKnowledgeStatisticsIR, LanguageRegisterIR,
    LanguageUnderstandingIR, PragmaticFunctionIR, LANGUAGE_KNOWLEDGE_SCHEMA,
};
