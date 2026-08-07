#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionKind {
    ThoughtCrystal,
    ReflexCircuit,
    ConceptNode,
}

#[derive(Debug, Clone)]
pub struct CircuitCompression {
    pub kind: CompressionKind,
    pub label: String,
    pub stability: f32,
}
