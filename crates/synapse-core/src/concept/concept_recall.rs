#[derive(Debug, Clone)]
pub struct ConceptRecall {
    pub concept_id: String,
    pub score: f32,
    pub context_fit: f32,
}
