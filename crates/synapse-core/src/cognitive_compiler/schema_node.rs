use crate::ConceptSchema;

#[derive(Debug, Clone)]
pub struct SchemaNode {
    pub schema: ConceptSchema,
    pub use_count: u64,
}
