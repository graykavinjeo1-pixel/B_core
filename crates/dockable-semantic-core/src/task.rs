use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Split {
    TrainDiscovery,
    Calibration,
    FreshBlind,
    AdversarialCounterfactual,
    DirectSemanticRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Demonstration {
    pub input: Vec<i64>,
    pub observed_output: Vec<i64>,
}

/// Solver-visible request content. Hidden answers and evaluator metadata are not
/// represented in this runtime type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleTask {
    pub task_id: String,
    pub split: Split,
    pub scalar_parameter: i64,
    pub demonstrations: Vec<Demonstration>,
    pub query_input: Vec<i64>,
}
