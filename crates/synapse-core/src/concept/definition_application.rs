use super::interpretation_result::InterpretationResult;

#[derive(Debug, Clone)]
pub struct DefinitionApplication;

impl DefinitionApplication {
    pub fn apply(definition: &str, context: &str) -> InterpretationResult {
        InterpretationResult {
            interpretation: format!("{definition} :: {context}"),
            confidence: 1.0,
        }
    }
}
