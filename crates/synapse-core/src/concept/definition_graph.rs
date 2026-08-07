#[derive(Debug, Clone, Default)]
pub struct DefinitionGraph {
    pub roles: Vec<String>,
    pub edges: Vec<(String, String)>,
}
