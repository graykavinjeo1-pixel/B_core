#![allow(
    clippy::assign_op_pattern,
    clippy::double_parens,
    clippy::identity_op,
    clippy::no_effect,
    dead_code,
    unused_imports,
    unused_parens,
    unused_variables
)]

use crate::sem5::model::{ImageValue, Value};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
struct Sem5Image {
    width: usize,
    height: usize,
    channels: usize,
    pixels: Vec<i64>,
}

pub const GENERATED_CAPABILITY_ACTIVE: bool = true;
pub const GENERATED_SOURCE_SCHEMA_REVISION: u64 = 1;
pub const GENERATED_PROGRAM_ID: &str = "P-T-000-1c2a3cce-FirstPrinciplesD";
pub const GENERATED_PROGRAM_IR_SHA256: &str =
    "dda8fe9adfe0e2c09c4a0e47496e5ea51dbf929863626df3e0d6cda87b449717";

pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
    let v0: Vec<i64> = match inputs.get("v0") {
        Some(Value::Sequence(value)) => value.clone(),
        _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
    };
    let mut stage_value: Vec<i64> = vec![];
    for (raw_index_0, raw_item_0) in (v0).clone().into_iter().enumerate() {
        let item: i64 = raw_item_0;
        let position: i64 = raw_index_0 as i64;
        if ((item % 5i64) == 4i64) {
            stage_value.push(((item * 1i64) + -5i64));
        } else {
            ();
        }
    }
    let mut state: i64 = 0i64;
    for (raw_index_1, raw_item_1) in (stage_value).clone().into_iter().enumerate() {
        let item: i64 = raw_item_1;
        let position: i64 = raw_index_1 as i64;
        if ((item % 2i64) == 1i64) {
            state = 0i64;
        } else {
            state = (state + item);
        }
    }
    let sem5_result: i64 = state;
    Ok(Value::Int(sem5_result))
}
