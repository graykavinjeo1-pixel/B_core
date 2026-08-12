#![allow(dead_code, unused_imports, unused_parens, unused_variables)]

use crate::sem5::model::Value;
use std::collections::BTreeMap;

pub const GENERATED_CAPABILITY_ACTIVE: bool = true;
pub const GENERATED_SOURCE_SCHEMA_REVISION: u64 = 4;
pub const GENERATED_PROGRAM_ID: &str = "P-T-000-2ec94494-FirstPrinciplesD";
pub const GENERATED_PROGRAM_IR_SHA256: &str =
    "e637fb422e2907369033dcd45f83ffe293d2b5d27ce969ddb2b6babc70f73d86";
pub const GENERATED_CAPABILITY_COUNT: usize = 3;

// B_CORE_CAPABILITY_BEGIN:dda8fe9adfe0e2c09c4a0e47496e5ea51dbf929863626df3e0d6cda87b449717
mod capability_dda8fe9adfe0e2c0 {
    #![allow(dead_code, unused_imports, unused_parens, unused_variables)]

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
    pub const GENERATED_SOURCE_SCHEMA_REVISION: u64 = 3;
    pub const GENERATED_PROGRAM_ID: &str = "P-T-000-1c2a3cce-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "dda8fe9adfe0e2c09c4a0e47496e5ea51dbf929863626df3e0d6cda87b449717";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 5i64) == 4i64 {
                stage_value.push(item + -5i64);
            }
        }
        let mut state: i64 = 0i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 2i64) == 1i64 {
                state = 0i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:dda8fe9adfe0e2c09c4a0e47496e5ea51dbf929863626df3e0d6cda87b449717

// B_CORE_CAPABILITY_BEGIN:a05aacf33f4962b9092797ef76820657a7a3d80be9ff9fb9c8bf0bb924722f0e
mod capability_a05aacf33f4962b9 {
    #![allow(dead_code, unused_imports, unused_parens, unused_variables)]

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
    pub const GENERATED_SOURCE_SCHEMA_REVISION: u64 = 3;
    pub const GENERATED_PROGRAM_ID: &str = "P-T-013-185888ca-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "a05aacf33f4962b9092797ef76820657a7a3d80be9ff9fb9c8bf0bb924722f0e";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 3i64) == 2i64 {
                stage_value.push((item * 3i64) + 6i64);
            }
        }
        let mut state: i64 = 0i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:a05aacf33f4962b9092797ef76820657a7a3d80be9ff9fb9c8bf0bb924722f0e

// B_CORE_CAPABILITY_BEGIN:e637fb422e2907369033dcd45f83ffe293d2b5d27ce969ddb2b6babc70f73d86
mod capability_e637fb422e290736 {
    #![allow(dead_code, unused_imports, unused_parens, unused_variables)]

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
    pub const GENERATED_SOURCE_SCHEMA_REVISION: u64 = 3;
    pub const GENERATED_PROGRAM_ID: &str = "P-T-000-2ec94494-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "e637fb422e2907369033dcd45f83ffe293d2b5d27ce969ddb2b6babc70f73d86";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 3i64) == 2i64 {
                stage_value.push(item * 3i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 4i64) == 1i64 {
                state = 1i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:e637fb422e2907369033dcd45f83ffe293d2b5d27ce969ddb2b6babc70f73d86

pub fn generated_capability_hashes() -> &'static [&'static str] {
    &[
        "dda8fe9adfe0e2c09c4a0e47496e5ea51dbf929863626df3e0d6cda87b449717",
        "a05aacf33f4962b9092797ef76820657a7a3d80be9ff9fb9c8bf0bb924722f0e",
        "e637fb422e2907369033dcd45f83ffe293d2b5d27ce969ddb2b6babc70f73d86",
    ]
}

pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
    capability_e637fb422e290736::run_generated_capability(inputs)
}

pub fn run_generated_capability_by_sha256(
    program_ir_sha256: &str,
    inputs: &BTreeMap<String, Value>,
) -> Result<Value, String> {
    match program_ir_sha256 {
        "dda8fe9adfe0e2c09c4a0e47496e5ea51dbf929863626df3e0d6cda87b449717" => {
            capability_dda8fe9adfe0e2c0::run_generated_capability(inputs)
        }
        "a05aacf33f4962b9092797ef76820657a7a3d80be9ff9fb9c8bf0bb924722f0e" => {
            capability_a05aacf33f4962b9::run_generated_capability(inputs)
        }
        "e637fb422e2907369033dcd45f83ffe293d2b5d27ce969ddb2b6babc70f73d86" => {
            capability_e637fb422e290736::run_generated_capability(inputs)
        }
        _ => Err("GENERATED_CAPABILITY_NOT_FOUND".to_string()),
    }
}
