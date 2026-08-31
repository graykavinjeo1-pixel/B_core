use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValueType {
    Integer,
    IntegerSequence,
    ScalarOperator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(
    tag = "code",
    content = "parameter",
    rename_all = "SCREAMING_SNAKE_CASE"
)]
pub enum ScalarOperator {
    Add(i64),
    Sub(i64),
    Mul(i64),
}

impl ScalarOperator {
    pub fn apply(self, value: i64) -> Result<i64, ExecutionError> {
        match self {
            Self::Add(parameter) => value
                .checked_add(parameter)
                .ok_or(ExecutionError::ArithmeticOverflow),
            Self::Sub(parameter) => value
                .checked_sub(parameter)
                .ok_or(ExecutionError::ArithmeticOverflow),
            Self::Mul(parameter) => value
                .checked_mul(parameter)
                .ok_or(ExecutionError::ArithmeticOverflow),
        }
    }

    pub fn with_parameter(self, parameter: i64) -> Self {
        match self {
            Self::Add(_) => Self::Add(parameter),
            Self::Sub(_) => Self::Sub(parameter),
            Self::Mul(_) => Self::Mul(parameter),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "instruction",
    content = "argument",
    rename_all = "SCREAMING_SNAKE_CASE"
)]
pub enum Instruction {
    InitOutput,
    BranchIfEmpty(usize),
    ReadCurrent,
    ApplyScalar(ScalarOperator),
    AppendCurrent,
    Advance,
    BranchIfRemaining(usize),
    Return,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "instruction",
    content = "argument",
    rename_all = "SCREAMING_SNAKE_CASE"
)]
pub enum InstructionPattern {
    InitOutput,
    BranchIfEmpty(usize),
    ReadCurrent,
    ScalarSlot,
    AppendCurrent,
    Advance,
    BranchIfRemaining(usize),
    Return,
}

impl InstructionPattern {
    pub fn bind(&self, scalar: ScalarOperator) -> Instruction {
        match self {
            Self::InitOutput => Instruction::InitOutput,
            Self::BranchIfEmpty(target) => Instruction::BranchIfEmpty(*target),
            Self::ReadCurrent => Instruction::ReadCurrent,
            Self::ScalarSlot => Instruction::ApplyScalar(scalar),
            Self::AppendCurrent => Instruction::AppendCurrent,
            Self::Advance => Instruction::Advance,
            Self::BranchIfRemaining(target) => Instruction::BranchIfRemaining(*target),
            Self::Return => Instruction::Return,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionError {
    InvalidProgramCounter,
    MissingCurrentValue,
    OutputNotInitialized,
    ArithmeticOverflow,
    StepBudgetExhausted,
    MissingReturn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineSnapshot {
    pub program_counter: usize,
    pub cursor: usize,
    pub current: Option<i64>,
    pub output: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub output: Vec<i64>,
    pub snapshots: Vec<MachineSnapshot>,
    pub instruction_indices: Vec<usize>,
    pub branch_count: usize,
}

pub fn execute_program(
    program: &[Instruction],
    input: &[i64],
    step_budget: usize,
) -> Result<ExecutionTrace, ExecutionError> {
    let mut pc = 0usize;
    let mut cursor = 0usize;
    let mut current = None;
    let mut output: Option<Vec<i64>> = None;
    let mut snapshots = Vec::new();
    let mut instruction_indices = Vec::new();
    let mut branch_count = 0usize;

    for _ in 0..step_budget {
        let instruction = program
            .get(pc)
            .ok_or(ExecutionError::InvalidProgramCounter)?;
        instruction_indices.push(pc);
        match instruction {
            Instruction::InitOutput => {
                output = Some(Vec::new());
                pc += 1;
            }
            Instruction::BranchIfEmpty(target) => {
                branch_count += 1;
                pc = if input.is_empty() { *target } else { pc + 1 };
            }
            Instruction::ReadCurrent => {
                current = input.get(cursor).copied();
                if current.is_none() {
                    return Err(ExecutionError::MissingCurrentValue);
                }
                pc += 1;
            }
            Instruction::ApplyScalar(operator) => {
                current =
                    Some(operator.apply(current.ok_or(ExecutionError::MissingCurrentValue)?)?);
                pc += 1;
            }
            Instruction::AppendCurrent => {
                output
                    .as_mut()
                    .ok_or(ExecutionError::OutputNotInitialized)?
                    .push(current.ok_or(ExecutionError::MissingCurrentValue)?);
                pc += 1;
            }
            Instruction::Advance => {
                cursor += 1;
                current = None;
                pc += 1;
            }
            Instruction::BranchIfRemaining(target) => {
                branch_count += 1;
                pc = if cursor < input.len() {
                    *target
                } else {
                    pc + 1
                };
            }
            Instruction::Return => {
                let final_output = output.ok_or(ExecutionError::OutputNotInitialized)?;
                snapshots.push(MachineSnapshot {
                    program_counter: pc,
                    cursor,
                    current,
                    output: final_output.clone(),
                });
                return Ok(ExecutionTrace {
                    output: final_output,
                    snapshots,
                    instruction_indices,
                    branch_count,
                });
            }
        }
        snapshots.push(MachineSnapshot {
            program_counter: pc,
            cursor,
            current,
            output: output.clone().unwrap_or_default(),
        });
    }
    Err(ExecutionError::StepBudgetExhausted)
}

#[cfg(test)]
mod tests {
    use super::{execute_program, ExecutionError, Instruction, ScalarOperator};

    fn executable(operator: ScalarOperator) -> Vec<Instruction> {
        vec![
            Instruction::InitOutput,
            Instruction::BranchIfEmpty(7),
            Instruction::ReadCurrent,
            Instruction::ApplyScalar(operator),
            Instruction::AppendCurrent,
            Instruction::Advance,
            Instruction::BranchIfRemaining(2),
            Instruction::Return,
        ]
    }

    #[test]
    fn primitive_semantics_execute_and_preserve_order() {
        let trace = execute_program(&executable(ScalarOperator::Mul(3)), &[2, -1, 4], 64)
            .expect("valid execution");
        assert_eq!(trace.output, vec![6, -3, 12]);
        assert!(trace.branch_count >= 2);
    }

    #[test]
    fn empty_input_is_defined() {
        let trace = execute_program(&executable(ScalarOperator::Add(9)), &[], 16)
            .expect("empty input is valid");
        assert!(trace.output.is_empty());
    }

    #[test]
    fn checked_arithmetic_enforces_precondition() {
        let error = execute_program(&executable(ScalarOperator::Add(1)), &[i64::MAX], 16)
            .expect_err("overflow must reject");
        assert_eq!(error, ExecutionError::ArithmeticOverflow);
    }
}
