use dockable_semantic_core::{
    dsl::ScalarOperator, task::Demonstration, GoalIR, CORE_ABI_VERSION, SEMANTIC_STATE_VERSION,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LanguageAdapterError {
    UnsupportedExpression,
    MissingParameter,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LanguageAdapter;

impl LanguageAdapter {
    pub const COMPATIBLE_CORE_ABI_VERSION: u32 = CORE_ABI_VERSION;

    pub fn compile(
        &self,
        request_id: &str,
        text: &str,
        query_input: Vec<i64>,
    ) -> Result<GoalIR, LanguageAdapterError> {
        let parameter = extract_integer(text).ok_or(LanguageAdapterError::MissingParameter)?;
        let lower = text.to_ascii_lowercase();
        let operator = if lower.contains("add")
            || lower.contains("plus")
            || text.contains("더해")
            || text.contains("더하기")
        {
            ScalarOperator::Add(parameter)
        } else if lower.contains("multiply") || lower.contains("times") || text.contains("곱해") {
            ScalarOperator::Mul(parameter)
        } else if lower.contains("subtract") || text.contains("빼") {
            ScalarOperator::Sub(parameter)
        } else {
            return Err(LanguageAdapterError::UnsupportedExpression);
        };
        let calibration_inputs = [vec![1, -2, 4], vec![0, 3]];
        let demonstrations = calibration_inputs
            .into_iter()
            .map(|input| {
                let observed_output = input
                    .iter()
                    .map(|value| operator.apply(*value).expect("small calibration values"))
                    .collect();
                Demonstration {
                    input,
                    observed_output,
                }
            })
            .collect();
        Ok(GoalIR {
            request_id: request_id.to_string(),
            core_abi_version: CORE_ABI_VERSION,
            semantic_state_version: SEMANTIC_STATE_VERSION.to_string(),
            target_concept_id: "C000001".to_string(),
            scalar_parameter: parameter,
            demonstrations,
            query_input,
            constraints: vec![
                "FINITE_SEQUENCE".to_string(),
                "CHECKED_ARITHMETIC".to_string(),
            ],
        })
    }
}

fn extract_integer(text: &str) -> Option<i64> {
    text.split(|character: char| !(character.is_ascii_digit() || character == '-'))
        .find(|token| !token.is_empty() && *token != "-")
        .and_then(|token| token.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::LanguageAdapter;

    #[test]
    fn korean_and_english_compile_to_equivalent_goal_ir() {
        let adapter = LanguageAdapter;
        let korean = adapter
            .compile("K", "각 값에 3을 더해", vec![1, 2])
            .expect("Korean grounding");
        let english = adapter
            .compile("E", "add 3 to each value", vec![1, 2])
            .expect("English grounding");
        assert_eq!(korean.scalar_parameter, english.scalar_parameter);
        assert_eq!(korean.target_concept_id, english.target_concept_id);
        assert_eq!(korean.demonstrations, english.demonstrations);
    }
}
