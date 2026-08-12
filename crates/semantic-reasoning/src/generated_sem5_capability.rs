#![allow(dead_code, unused_imports, unused_parens, unused_variables)]

use crate::sem5::model::Value;
use std::collections::BTreeMap;

pub const GENERATED_CAPABILITY_ACTIVE: bool = true;
pub const GENERATED_SOURCE_SCHEMA_REVISION: u64 = 4;
pub const GENERATED_PROGRAM_ID: &str = "P-T-003-46657ac5-FirstPrinciplesD";
pub const GENERATED_PROGRAM_IR_SHA256: &str =
    "0815a71cd0f718cafb7582572e9eeac88e25df692e96c01f7c6473c59bf436fc";
pub const GENERATED_CAPABILITY_COUNT: usize = 62;

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

// B_CORE_CAPABILITY_BEGIN:1b4313ac8f9356bcf981e14f319c07064ffe3ad97f7d5445e8865844ebb65dec
mod capability_1b4313ac8f9356bc {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-014-7325aaee-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "1b4313ac8f9356bcf981e14f319c07064ffe3ad97f7d5445e8865844ebb65dec";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 3i64) == 1i64 {
                stage_value.push((item * 3i64) + 1i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:1b4313ac8f9356bcf981e14f319c07064ffe3ad97f7d5445e8865844ebb65dec

// B_CORE_CAPABILITY_BEGIN:3eb34270273d5a51d5f54d06afa54c512959b605f265d1a5eb4b3cc2803050fb
mod capability_3eb34270273d5a51 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-013-2161758a-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "3eb34270273d5a51d5f54d06afa54c512959b605f265d1a5eb4b3cc2803050fb";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 3i64) == 0i64 {
                stage_value.push((item * 3i64) + -2i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:3eb34270273d5a51d5f54d06afa54c512959b605f265d1a5eb4b3cc2803050fb

// B_CORE_CAPABILITY_BEGIN:962f67422edea84953f3ec46d4db7cfba8579a02a0fadb5ea29617cd4b8dab0e
mod capability_962f67422edea849 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-011-81f3d3bc-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "962f67422edea84953f3ec46d4db7cfba8579a02a0fadb5ea29617cd4b8dab0e";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 5i64) == 3i64 {
                stage_value.push(item + -1i64);
            }
        }
        let mut state: i64 = -1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:962f67422edea84953f3ec46d4db7cfba8579a02a0fadb5ea29617cd4b8dab0e

// B_CORE_CAPABILITY_BEGIN:4b7e0f9e0e467c0d00d0867850c30de9b49aedfa2a2cbd89e8e88dd988e525f9
mod capability_4b7e0f9e0e467c0d {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-013-b9d6b115-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "4b7e0f9e0e467c0d00d0867850c30de9b49aedfa2a2cbd89e8e88dd988e525f9";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 2i64) == 1i64 {
                stage_value.push(position * 2i64);
            }
        }
        let mut state: i64 = -2i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:4b7e0f9e0e467c0d00d0867850c30de9b49aedfa2a2cbd89e8e88dd988e525f9

// B_CORE_CAPABILITY_BEGIN:03bdbdbe80842c73f86c1dd82212757788fb4f349df781330fcdc815a3d76550
mod capability_03bdbdbe80842c73 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-019-b54b338e-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "03bdbdbe80842c73f86c1dd82212757788fb4f349df781330fcdc815a3d76550";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 3i64) == 0i64 {
                stage_value.push((item * 3i64) + 1i64);
            }
        }
        let mut state: i64 = -2i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:03bdbdbe80842c73f86c1dd82212757788fb4f349df781330fcdc815a3d76550

// B_CORE_CAPABILITY_BEGIN:c91c850c5c09e84795013b89b6ef185f2905c9949bbe22ad203709d8836773a5
mod capability_c91c850c5c09e847 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-018-af033ec7-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "c91c850c5c09e84795013b89b6ef185f2905c9949bbe22ad203709d8836773a5";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 2i64) == 1i64 {
                stage_value.push((position * 2i64) + -5i64);
            }
        }
        let mut state: i64 = -1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 3i64) == 1i64 {
                state = -1i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:c91c850c5c09e84795013b89b6ef185f2905c9949bbe22ad203709d8836773a5

// B_CORE_CAPABILITY_BEGIN:574e473e88991d9dfa6087654ff5b59ee3cb124b7abc3355e7737b501a4b80fa
mod capability_574e473e88991d9d {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-018-b9636db2-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "574e473e88991d9dfa6087654ff5b59ee3cb124b7abc3355e7737b501a4b80fa";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 5i64) == 2i64 {
                stage_value.push(item);
            }
        }
        let mut state: i64 = -1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 2i64) == 1i64 {
                state = -1i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:574e473e88991d9dfa6087654ff5b59ee3cb124b7abc3355e7737b501a4b80fa

// B_CORE_CAPABILITY_BEGIN:497dd951bbb5547f2817891ad91393b583883101331770303cdc378e0fb5ffec
mod capability_497dd951bbb5547f {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-017-247b84c7-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "497dd951bbb5547f2817891ad91393b583883101331770303cdc378e0fb5ffec";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 4i64) == 1i64 {
                stage_value.push((position * 4i64) + -6i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:497dd951bbb5547f2817891ad91393b583883101331770303cdc378e0fb5ffec

// B_CORE_CAPABILITY_BEGIN:0429b1c9611371348cb6ed89223a8883a6a065b6f0d8db4fc3e67188f1a7fb99
mod capability_0429b1c961137134 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-019-5e600655-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "0429b1c9611371348cb6ed89223a8883a6a065b6f0d8db4fc3e67188f1a7fb99";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 2i64) == 1i64 {
                stage_value.push((position * 2i64) + -7i64);
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
// B_CORE_CAPABILITY_END:0429b1c9611371348cb6ed89223a8883a6a065b6f0d8db4fc3e67188f1a7fb99

// B_CORE_CAPABILITY_BEGIN:caef2fe075601913d4878d04ac9faa84b0c4744a40c195db4e73f4f1cf9987a0
mod capability_caef2fe075601913 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-004-7ad0d15c-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "caef2fe075601913d4878d04ac9faa84b0c4744a40c195db4e73f4f1cf9987a0";

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
                stage_value.push(item + 2i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:caef2fe075601913d4878d04ac9faa84b0c4744a40c195db4e73f4f1cf9987a0

// B_CORE_CAPABILITY_BEGIN:4d0caf0c14bd98fcf6ebd9f7d29738d255b7c2f756ea48c6f8c41b0ed451d921
mod capability_4d0caf0c14bd98fc {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-016-8cd42f4e-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "4d0caf0c14bd98fcf6ebd9f7d29738d255b7c2f756ea48c6f8c41b0ed451d921";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 3i64) == 1i64 {
                stage_value.push((item * 3i64) + -2i64);
            }
        }
        let mut state: i64 = 2i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:4d0caf0c14bd98fcf6ebd9f7d29738d255b7c2f756ea48c6f8c41b0ed451d921

// B_CORE_CAPABILITY_BEGIN:2a976596fabdc1ebb958320d8d54485a015438d1bc9c08ae854e0761abf048a6
mod capability_2a976596fabdc1eb {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-006-b834a1f2-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "2a976596fabdc1ebb958320d8d54485a015438d1bc9c08ae854e0761abf048a6";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 5i64) == 0i64 {
                stage_value.push(item + 5i64);
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
// B_CORE_CAPABILITY_END:2a976596fabdc1ebb958320d8d54485a015438d1bc9c08ae854e0761abf048a6

// B_CORE_CAPABILITY_BEGIN:230304807723f2a956f7ce81ba9bbfb1dad9be3cf96699d1591793a8c4b1121a
mod capability_230304807723f2a9 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-005-e5467055-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "230304807723f2a956f7ce81ba9bbfb1dad9be3cf96699d1591793a8c4b1121a";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 2i64) == 1i64 {
                stage_value.push((position * 2i64) + 6i64);
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
// B_CORE_CAPABILITY_END:230304807723f2a956f7ce81ba9bbfb1dad9be3cf96699d1591793a8c4b1121a

// B_CORE_CAPABILITY_BEGIN:76b955741341723249680016cedea3931861982bb433222a9595ff43da07667c
mod capability_76b9557413417232 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-003-01044d3d-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "76b955741341723249680016cedea3931861982bb433222a9595ff43da07667c";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 4i64) == 1i64 {
                stage_value.push((position * 4i64) + -3i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 5i64) == 1i64 {
                state = 1i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:76b955741341723249680016cedea3931861982bb433222a9595ff43da07667c

// B_CORE_CAPABILITY_BEGIN:75f1b3f69dbadbcdd0e63a116efff9ab0f68d388824b9baed0a401535efee58c
mod capability_75f1b3f69dbadbcd {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-016-2665b610-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "75f1b3f69dbadbcdd0e63a116efff9ab0f68d388824b9baed0a401535efee58c";

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
                stage_value.push(item + 5i64);
            }
        }
        let mut state: i64 = -3i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:75f1b3f69dbadbcdd0e63a116efff9ab0f68d388824b9baed0a401535efee58c

// B_CORE_CAPABILITY_BEGIN:33bea1d499b1c41631142977959333b1def70035840d3c86db8817cab0ad5280
mod capability_33bea1d499b1c416 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-004-568e5ec7-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "33bea1d499b1c41631142977959333b1def70035840d3c86db8817cab0ad5280";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 4i64) == 1i64 {
                stage_value.push((position * 4i64) + -5i64);
            }
        }
        let mut state: i64 = -3i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:33bea1d499b1c41631142977959333b1def70035840d3c86db8817cab0ad5280

// B_CORE_CAPABILITY_BEGIN:247990c6555d7ee136f4a12e507d38e177bbc549e33ca8258159d9678623dc6e
mod capability_247990c6555d7ee1 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-004-8acd6199-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "247990c6555d7ee136f4a12e507d38e177bbc549e33ca8258159d9678623dc6e";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 2i64) == 1i64 {
                stage_value.push((position * 2i64) + -2i64);
            }
        }
        let mut state: i64 = -2i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:247990c6555d7ee136f4a12e507d38e177bbc549e33ca8258159d9678623dc6e

// B_CORE_CAPABILITY_BEGIN:f700111f88ac5f3aa1f47280d8aef76f66fb8e81f8e147d828e56ff1f5b0adc8
mod capability_f700111f88ac5f3a {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-016-861ea23e-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "f700111f88ac5f3aa1f47280d8aef76f66fb8e81f8e147d828e56ff1f5b0adc8";

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
                stage_value.push((item * 3i64) + 5i64);
            }
        }
        let mut state: i64 = 3i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:f700111f88ac5f3aa1f47280d8aef76f66fb8e81f8e147d828e56ff1f5b0adc8

// B_CORE_CAPABILITY_BEGIN:c0b52aa436f3ff9cfc2fa574ce08ab2b8095627dc8adb225644ef34b7a1dedcf
mod capability_c0b52aa436f3ff9c {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-011-e4225a31-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "c0b52aa436f3ff9cfc2fa574ce08ab2b8095627dc8adb225644ef34b7a1dedcf";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 2i64) == 1i64 {
                stage_value.push((position * 2i64) + 4i64);
            }
        }
        let mut state: i64 = 3i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:c0b52aa436f3ff9cfc2fa574ce08ab2b8095627dc8adb225644ef34b7a1dedcf

// B_CORE_CAPABILITY_BEGIN:5788e14dfcf4302ae6cd55d7e123e4b5beff84a36f06e3a6f9d75d466be90760
mod capability_5788e14dfcf4302a {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-004-33bf9862-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "5788e14dfcf4302ae6cd55d7e123e4b5beff84a36f06e3a6f9d75d466be90760";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 3i64) == 1i64 {
                stage_value.push((item * 3i64) + 6i64);
            }
        }
        let mut state: i64 = -1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:5788e14dfcf4302ae6cd55d7e123e4b5beff84a36f06e3a6f9d75d466be90760

// B_CORE_CAPABILITY_BEGIN:0e40e0f291f7734c31a4c32fe5b9fc8f57aa574e6f79ed7038903cbe4afb61a3
mod capability_0e40e0f291f7734c {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-018-129faa19-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "0e40e0f291f7734c31a4c32fe5b9fc8f57aa574e6f79ed7038903cbe4afb61a3";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 4i64) == 1i64 {
                stage_value.push((position * 4i64) + -4i64);
            }
        }
        let mut state: i64 = -3i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 5i64) == 4i64 {
                state = -3i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:0e40e0f291f7734c31a4c32fe5b9fc8f57aa574e6f79ed7038903cbe4afb61a3

// B_CORE_CAPABILITY_BEGIN:e28232536dfc2703b4388d25ce119a8150807c720e28409cc0cf88a8ea940c22
mod capability_e28232536dfc2703 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-011-083c1ef4-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "e28232536dfc2703b4388d25ce119a8150807c720e28409cc0cf88a8ea940c22";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 5i64) == 2i64 {
                stage_value.push(item + 3i64);
            }
        }
        let mut state: i64 = 3i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:e28232536dfc2703b4388d25ce119a8150807c720e28409cc0cf88a8ea940c22

// B_CORE_CAPABILITY_BEGIN:860c4dcb6487e20380db62a122470eab2e246fd485e6edb5c728aa2b2baa5f90
mod capability_860c4dcb6487e203 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-007-c3978829-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "860c4dcb6487e20380db62a122470eab2e246fd485e6edb5c728aa2b2baa5f90";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 2i64) == 1i64 {
                stage_value.push((position * 2i64) + -1i64);
            }
        }
        let mut state: i64 = 3i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:860c4dcb6487e20380db62a122470eab2e246fd485e6edb5c728aa2b2baa5f90

// B_CORE_CAPABILITY_BEGIN:3dd9a903b24d8315cd3eeb97b85c47f4f7ee3fb0987d5e35844104ff0622456e
mod capability_3dd9a903b24d8315 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-010-ae71e723-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "3dd9a903b24d8315cd3eeb97b85c47f4f7ee3fb0987d5e35844104ff0622456e";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 4i64) == 1i64 {
                stage_value.push(position * 4i64);
            }
        }
        let mut state: i64 = -2i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:3dd9a903b24d8315cd3eeb97b85c47f4f7ee3fb0987d5e35844104ff0622456e

// B_CORE_CAPABILITY_BEGIN:23f340838e55bc36356521bb94d5e184a3a775ecbdf06935e210215b3bb64721
mod capability_23f340838e55bc36 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-012-577ae237-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "23f340838e55bc36356521bb94d5e184a3a775ecbdf06935e210215b3bb64721";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 2i64) == 1i64 {
                stage_value.push((position * 2i64) + -3i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 3i64) == 0i64 {
                state = 1i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:23f340838e55bc36356521bb94d5e184a3a775ecbdf06935e210215b3bb64721

// B_CORE_CAPABILITY_BEGIN:77d080a6932df8eb2afd3ac8b570852a5c9496097162d799036732f9e7437937
mod capability_77d080a6932df8eb {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-019-24dec2ec-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "77d080a6932df8eb2afd3ac8b570852a5c9496097162d799036732f9e7437937";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 5i64) == 0i64 {
                stage_value.push(item);
            }
        }
        let mut state: i64 = -3i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:77d080a6932df8eb2afd3ac8b570852a5c9496097162d799036732f9e7437937

// B_CORE_CAPABILITY_BEGIN:52458df072c8c2e2f4c217407db98dc7293adc7a2663d132ff17bb4ef1899c06
mod capability_52458df072c8c2e2 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-005-36c35103-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "52458df072c8c2e2f4c217407db98dc7293adc7a2663d132ff17bb4ef1899c06";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 4i64) == 1i64 {
                stage_value.push((position * 4i64) + -5i64);
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
// B_CORE_CAPABILITY_END:52458df072c8c2e2f4c217407db98dc7293adc7a2663d132ff17bb4ef1899c06

// B_CORE_CAPABILITY_BEGIN:2716441b4b3e09ea5e3f03ffddde10a71c132bd82d00618e9b9532d918142953
mod capability_2716441b4b3e09ea {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-010-9dffdefb-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "2716441b4b3e09ea5e3f03ffddde10a71c132bd82d00618e9b9532d918142953";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 4i64) == 1i64 {
                stage_value.push((position * 4i64) + 7i64);
            }
        }
        let mut state: i64 = 3i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:2716441b4b3e09ea5e3f03ffddde10a71c132bd82d00618e9b9532d918142953

// B_CORE_CAPABILITY_BEGIN:f88d40529aa1b99d112e461640951af63fe8e82e389eaac98d599af7f61e6440
mod capability_f88d40529aa1b99d {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-015-6fb88f8b-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "f88d40529aa1b99d112e461640951af63fe8e82e389eaac98d599af7f61e6440";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 2i64) == 1i64 {
                stage_value.push((position * 2i64) + 4i64);
            }
        }
        let mut state: i64 = -1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 3i64) == 2i64 {
                state = -1i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:f88d40529aa1b99d112e461640951af63fe8e82e389eaac98d599af7f61e6440

// B_CORE_CAPABILITY_BEGIN:d4cc8460340393f78c793e871a76da3177d798ad3d4a8886d05e713167c6a871
mod capability_d4cc8460340393f7 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-003-6e750fcd-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "d4cc8460340393f78c793e871a76da3177d798ad3d4a8886d05e713167c6a871";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 4i64) == 1i64 {
                stage_value.push((position * 4i64) + -6i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 5i64) == 1i64 {
                state = 1i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:d4cc8460340393f78c793e871a76da3177d798ad3d4a8886d05e713167c6a871

// B_CORE_CAPABILITY_BEGIN:d37af674767b2c8c5e16a022ccf0162ac09c900df51af2ee5788ce66b316c623
mod capability_d37af674767b2c8c {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-016-489ed474-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "d37af674767b2c8c5e16a022ccf0162ac09c900df51af2ee5788ce66b316c623";

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
                stage_value.push(item + 5i64);
            }
        }
        let mut state: i64 = -3i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:d37af674767b2c8c5e16a022ccf0162ac09c900df51af2ee5788ce66b316c623

// B_CORE_CAPABILITY_BEGIN:cab5fb604edaace5c7f9d2e39e4656821f42848d947c3e1a1b32f5f46e5394dc
mod capability_cab5fb604edaace5 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-012-6ee6183c-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "cab5fb604edaace5c7f9d2e39e4656821f42848d947c3e1a1b32f5f46e5394dc";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 3i64) == 1i64 {
                stage_value.push((item * 3i64) + 6i64);
            }
        }
        let mut state: i64 = -3i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 4i64) == 1i64 {
                state = -3i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:cab5fb604edaace5c7f9d2e39e4656821f42848d947c3e1a1b32f5f46e5394dc

// B_CORE_CAPABILITY_BEGIN:c7212054c7106572ecb6b991623db126c4792194794389421679b84f17027734
mod capability_c7212054c7106572 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-001-3d9e8f59-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "c7212054c7106572ecb6b991623db126c4792194794389421679b84f17027734";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 2i64) == 1i64 {
                stage_value.push((position * 2i64) + 1i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:c7212054c7106572ecb6b991623db126c4792194794389421679b84f17027734

// B_CORE_CAPABILITY_BEGIN:c70ea137395090da8128ad3f8428225748ef658bb89f63fa6b96289011d13a2a
mod capability_c70ea137395090da {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-016-a5cfac4e-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "c70ea137395090da8128ad3f8428225748ef658bb89f63fa6b96289011d13a2a";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 3i64) == 0i64 {
                stage_value.push((item * 3i64) + 5i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:c70ea137395090da8128ad3f8428225748ef658bb89f63fa6b96289011d13a2a

// B_CORE_CAPABILITY_BEGIN:c0fbd6eb0d0b4aa1d3ea9830ea0af8252f430ceb41d0f6a78881d2360fb579c2
mod capability_c0fbd6eb0d0b4aa1 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-002-4882e33b-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "c0fbd6eb0d0b4aa1d3ea9830ea0af8252f430ceb41d0f6a78881d2360fb579c2";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 4i64) == 1i64 {
                stage_value.push((position * 4i64) + -6i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:c0fbd6eb0d0b4aa1d3ea9830ea0af8252f430ceb41d0f6a78881d2360fb579c2

// B_CORE_CAPABILITY_BEGIN:b7a36f53b68a9b5a56f3894601a95327724e270e996857e9c03548a1c58485b4
mod capability_b7a36f53b68a9b5a {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-017-1f0944f4-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "b7a36f53b68a9b5a56f3894601a95327724e270e996857e9c03548a1c58485b4";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 5i64) == 0i64 {
                stage_value.push(item + 7i64);
            }
        }
        let mut state: i64 = -1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:b7a36f53b68a9b5a56f3894601a95327724e270e996857e9c03548a1c58485b4

// B_CORE_CAPABILITY_BEGIN:b1bf12dd7d3f2d66e2a1c268743993cb652f4e74e186f2628fa2a8792c882e23
mod capability_b1bf12dd7d3f2d66 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-004-40796f0b-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "b1bf12dd7d3f2d66e2a1c268743993cb652f4e74e186f2628fa2a8792c882e23";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 4i64) == 1i64 {
                stage_value.push((position * 4i64) + -7i64);
            }
        }
        let mut state: i64 = 2i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:b1bf12dd7d3f2d66e2a1c268743993cb652f4e74e186f2628fa2a8792c882e23

// B_CORE_CAPABILITY_BEGIN:a50825e3d1d6b25379191dfabccb56398f2e600b5de9befcc75ed378f8f12611
mod capability_a50825e3d1d6b253 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-011-4ba64c20-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "a50825e3d1d6b25379191dfabccb56398f2e600b5de9befcc75ed378f8f12611";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 5i64) == 1i64 {
                stage_value.push(item + -2i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:a50825e3d1d6b25379191dfabccb56398f2e600b5de9befcc75ed378f8f12611

// B_CORE_CAPABILITY_BEGIN:a4a33a1229d25e697860986cdba564b18393ef279598fa5cae49499e5ccaa9d4
mod capability_a4a33a1229d25e69 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-009-e2e9064d-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "a4a33a1229d25e697860986cdba564b18393ef279598fa5cae49499e5ccaa9d4";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 4i64) == 1i64 {
                stage_value.push((position * 4i64) + 3i64);
            }
        }
        let mut state: i64 = 0i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 5i64) == 4i64 {
                state = 0i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:a4a33a1229d25e697860986cdba564b18393ef279598fa5cae49499e5ccaa9d4

// B_CORE_CAPABILITY_BEGIN:9f9b80ecc166241b4275a79ba3575b3b16924211464c6ee6218d5d5a5b55d1fe
mod capability_9f9b80ecc166241b {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-012-5c7e0e83-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "9f9b80ecc166241b4275a79ba3575b3b16924211464c6ee6218d5d5a5b55d1fe";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 2i64) == 1i64 {
                stage_value.push((position * 2i64) + 1i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 3i64) == 2i64 {
                state = 1i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:9f9b80ecc166241b4275a79ba3575b3b16924211464c6ee6218d5d5a5b55d1fe

// B_CORE_CAPABILITY_BEGIN:918974a8e9ff052a546dbc661b40f52738ccb505aa64ca24daf863077ede4e53
mod capability_918974a8e9ff052a {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-012-21215bc5-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "918974a8e9ff052a546dbc661b40f52738ccb505aa64ca24daf863077ede4e53";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 4i64) == 1i64 {
                stage_value.push(position * 4i64);
            }
        }
        let mut state: i64 = 0i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 5i64) == 1i64 {
                state = 0i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:918974a8e9ff052a546dbc661b40f52738ccb505aa64ca24daf863077ede4e53

// B_CORE_CAPABILITY_BEGIN:85165963f057b60bf5d7f8410fc64739dbae3af83234045159a1c2f0b4447cfc
mod capability_85165963f057b60b {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-011-3a286a14-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "85165963f057b60bf5d7f8410fc64739dbae3af83234045159a1c2f0b4447cfc";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 5i64) == 3i64 {
                stage_value.push(item + 4i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:85165963f057b60bf5d7f8410fc64739dbae3af83234045159a1c2f0b4447cfc

// B_CORE_CAPABILITY_BEGIN:83c4dd95a90688644e548c832e5f155a62826021ae8a6d6d192bde74d3ac666f
mod capability_83c4dd95a9068864 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-013-dbfa408b-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "83c4dd95a90688644e548c832e5f155a62826021ae8a6d6d192bde74d3ac666f";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 4i64) == 1i64 {
                stage_value.push((position * 4i64) + 4i64);
            }
        }
        let mut state: i64 = -3i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:83c4dd95a90688644e548c832e5f155a62826021ae8a6d6d192bde74d3ac666f

// B_CORE_CAPABILITY_BEGIN:7eea9036056de0ffb431279efdb1c928074717cddc8d9627f4944bfe255b5d1d
mod capability_7eea9036056de0ff {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-005-4c25dfd0-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "7eea9036056de0ffb431279efdb1c928074717cddc8d9627f4944bfe255b5d1d";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 5i64) == 0i64 {
                stage_value.push(item + 1i64);
            }
        }
        let mut state: i64 = -2i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:7eea9036056de0ffb431279efdb1c928074717cddc8d9627f4944bfe255b5d1d

// B_CORE_CAPABILITY_BEGIN:7eaccc8f70286c7c300138b04b3db54bfb7baf4262a40980f86e571cab08946a
mod capability_7eaccc8f70286c7c {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-016-1e34453c-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "7eaccc8f70286c7c300138b04b3db54bfb7baf4262a40980f86e571cab08946a";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 5i64) == 1i64 {
                stage_value.push(item + -2i64);
            }
        }
        let mut state: i64 = 3i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:7eaccc8f70286c7c300138b04b3db54bfb7baf4262a40980f86e571cab08946a

// B_CORE_CAPABILITY_BEGIN:6c53c25f3b6168063f61d39f352d339ed7e956909f4aefa7881995a491fbfdea
mod capability_6c53c25f3b616806 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-005-8feebe6a-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "6c53c25f3b6168063f61d39f352d339ed7e956909f4aefa7881995a491fbfdea";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 3i64) == 0i64 {
                stage_value.push((item * 3i64) + -1i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:6c53c25f3b6168063f61d39f352d339ed7e956909f4aefa7881995a491fbfdea

// B_CORE_CAPABILITY_BEGIN:6bf380ba0f258057577aee5ea10c905ada451cc6dcacafd60206297061be8db1
mod capability_6bf380ba0f258057 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-008-f8e40e7c-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "6bf380ba0f258057577aee5ea10c905ada451cc6dcacafd60206297061be8db1";

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
                stage_value.push(item + 7i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:6bf380ba0f258057577aee5ea10c905ada451cc6dcacafd60206297061be8db1

// B_CORE_CAPABILITY_BEGIN:649811db59332d0603c370b5042f0658dd6f73cadeba5f4173b5738b4252af18
mod capability_649811db59332d06 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-013-6538322f-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "649811db59332d0603c370b5042f0658dd6f73cadeba5f4173b5738b4252af18";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 4i64) == 1i64 {
                stage_value.push((position * 4i64) + -5i64);
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
// B_CORE_CAPABILITY_END:649811db59332d0603c370b5042f0658dd6f73cadeba5f4173b5738b4252af18

// B_CORE_CAPABILITY_BEGIN:6348d06ec871a16b493b49869224fd4c94995af8dcf5048779d43d9fe148946f
mod capability_6348d06ec871a16b {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-011-213a58b8-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "6348d06ec871a16b493b49869224fd4c94995af8dcf5048779d43d9fe148946f";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 5i64) == 2i64 {
                stage_value.push(item + -6i64);
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
// B_CORE_CAPABILITY_END:6348d06ec871a16b493b49869224fd4c94995af8dcf5048779d43d9fe148946f

// B_CORE_CAPABILITY_BEGIN:5c7ac75d21acf6f4b6cfb87c502edb91a9f367d33d36706c59e9ef174091d2a6
mod capability_5c7ac75d21acf6f4 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-010-17c6ef48-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "5c7ac75d21acf6f4b6cfb87c502edb91a9f367d33d36706c59e9ef174091d2a6";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 5i64) == 3i64 {
                stage_value.push(item + -6i64);
            }
        }
        let mut state: i64 = -2i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:5c7ac75d21acf6f4b6cfb87c502edb91a9f367d33d36706c59e9ef174091d2a6

// B_CORE_CAPABILITY_BEGIN:3b5c3862443c3fdb5cd0afa24a3de3a28a19af38e63cc6dfaf5f611b26faceda
mod capability_3b5c3862443c3fdb {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-003-ffe7762b-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "3b5c3862443c3fdb5cd0afa24a3de3a28a19af38e63cc6dfaf5f611b26faceda";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 2i64) == 1i64 {
                stage_value.push((position * 2i64) + -3i64);
            }
        }
        let mut state: i64 = -1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 3i64) == 1i64 {
                state = -1i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:3b5c3862443c3fdb5cd0afa24a3de3a28a19af38e63cc6dfaf5f611b26faceda

// B_CORE_CAPABILITY_BEGIN:31456763f513deafbd73a358559cb0e3582bd4c600b4c6fbec8b10d0603c2f21
mod capability_31456763f513deaf {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-003-6df1e548-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "31456763f513deafbd73a358559cb0e3582bd4c600b4c6fbec8b10d0603c2f21";

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
                stage_value.push((item * 3i64) + 5i64);
            }
        }
        let mut state: i64 = -2i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 4i64) == 1i64 {
                state = -2i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:31456763f513deafbd73a358559cb0e3582bd4c600b4c6fbec8b10d0603c2f21

// B_CORE_CAPABILITY_BEGIN:2b27fd7fc14fd06d0c806213e040486863dc96ee3c007c58382f04e8b846b86c
mod capability_2b27fd7fc14fd06d {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-007-204cfa21-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "2b27fd7fc14fd06d0c806213e040486863dc96ee3c007c58382f04e8b846b86c";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 2i64) == 1i64 {
                stage_value.push((position * 2i64) + 6i64);
            }
        }
        let mut state: i64 = 1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:2b27fd7fc14fd06d0c806213e040486863dc96ee3c007c58382f04e8b846b86c

// B_CORE_CAPABILITY_BEGIN:262ad8574b3d278199cd9101e773fb85b0574f58b324adeda1330396aa5fc252
mod capability_262ad8574b3d2781 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-010-b4a238a8-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "262ad8574b3d278199cd9101e773fb85b0574f58b324adeda1330396aa5fc252";

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
                stage_value.push(item + 5i64);
            }
        }
        let mut state: i64 = -1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:262ad8574b3d278199cd9101e773fb85b0574f58b324adeda1330396aa5fc252

// B_CORE_CAPABILITY_BEGIN:23fc7ccccbfb1d24078c66c7e294afdc44996b119240189648bd3e12241ed9ee
mod capability_23fc7ccccbfb1d24 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-001-2fe1cd15-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "23fc7ccccbfb1d24078c66c7e294afdc44996b119240189648bd3e12241ed9ee";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 2i64) == 1i64 {
                stage_value.push((position * 2i64) + 4i64);
            }
        }
        let mut state: i64 = -2i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:23fc7ccccbfb1d24078c66c7e294afdc44996b119240189648bd3e12241ed9ee

// B_CORE_CAPABILITY_BEGIN:22cbe17dc4ba11f8aee50941de920c0f99a24bc5045df5c7aa136dbc42f05c8e
mod capability_22cbe17dc4ba11f8 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-010-0501a63f-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "22cbe17dc4ba11f8aee50941de920c0f99a24bc5045df5c7aa136dbc42f05c8e";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 4i64) == 1i64 {
                stage_value.push((position * 4i64) + -5i64);
            }
        }
        let mut state: i64 = -1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:22cbe17dc4ba11f8aee50941de920c0f99a24bc5045df5c7aa136dbc42f05c8e

// B_CORE_CAPABILITY_BEGIN:13813b318fdb898dfc51e90f671d0d8774f7886d883e8567adece44e860ddd47
mod capability_13813b318fdb898d {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-010-a9f906e1-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "13813b318fdb898dfc51e90f671d0d8774f7886d883e8567adece44e860ddd47";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 2i64) == 1i64 {
                stage_value.push(position * 2i64);
            }
        }
        let mut state: i64 = -1i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            state += item;
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:13813b318fdb898dfc51e90f671d0d8774f7886d883e8567adece44e860ddd47

// B_CORE_CAPABILITY_BEGIN:0d85347ab8f6d5a8ca6614ee52d7d77c38b3096465c64d9b9f24d750b9dc0e56
mod capability_0d85347ab8f6d5a8 {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-009-dacea436-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "0d85347ab8f6d5a8ca6614ee52d7d77c38b3096465c64d9b9f24d750b9dc0e56";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 5i64) == 2i64 {
                stage_value.push(item + 5i64);
            }
        }
        let mut state: i64 = 3i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 2i64) == 1i64 {
                state = 3i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:0d85347ab8f6d5a8ca6614ee52d7d77c38b3096465c64d9b9f24d750b9dc0e56

// B_CORE_CAPABILITY_BEGIN:0815a71cd0f718cafb7582572e9eeac88e25df692e96c01f7c6473c59bf436fc
mod capability_0815a71cd0f718ca {
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
    pub const GENERATED_PROGRAM_ID: &str = "P-T-003-46657ac5-FirstPrinciplesD";
    pub const GENERATED_PROGRAM_IR_SHA256: &str =
        "0815a71cd0f718cafb7582572e9eeac88e25df692e96c01f7c6473c59bf436fc";

    pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
        let v0: Vec<i64> = match inputs.get("v0") {
            Some(Value::Sequence(value)) => value.clone(),
            _ => return Err("GENERATED_CAPABILITY_INPUT_TYPE:v0".to_string()),
        };
        let mut stage_value: Vec<i64> = vec![];
        for (raw_index_0, raw_item_0) in v0.clone().into_iter().enumerate() {
            let item: i64 = raw_item_0;
            let position: i64 = raw_index_0 as i64;
            if (item % 4i64) == 1i64 {
                stage_value.push((position * 4i64) + -7i64);
            }
        }
        let mut state: i64 = 0i64;
        for (raw_index_1, raw_item_1) in stage_value.clone().into_iter().enumerate() {
            let item: i64 = raw_item_1;
            let position: i64 = raw_index_1 as i64;
            if (item % 5i64) == 3i64 {
                state = 0i64;
            } else {
                state += item;
            }
        }
        let sem5_result: i64 = state;
        Ok(Value::Int(sem5_result))
    }
}
// B_CORE_CAPABILITY_END:0815a71cd0f718cafb7582572e9eeac88e25df692e96c01f7c6473c59bf436fc

pub fn generated_capability_hashes() -> &'static [&'static str] {
    &[
        "dda8fe9adfe0e2c09c4a0e47496e5ea51dbf929863626df3e0d6cda87b449717",
        "a05aacf33f4962b9092797ef76820657a7a3d80be9ff9fb9c8bf0bb924722f0e",
        "e637fb422e2907369033dcd45f83ffe293d2b5d27ce969ddb2b6babc70f73d86",
        "1b4313ac8f9356bcf981e14f319c07064ffe3ad97f7d5445e8865844ebb65dec",
        "3eb34270273d5a51d5f54d06afa54c512959b605f265d1a5eb4b3cc2803050fb",
        "962f67422edea84953f3ec46d4db7cfba8579a02a0fadb5ea29617cd4b8dab0e",
        "4b7e0f9e0e467c0d00d0867850c30de9b49aedfa2a2cbd89e8e88dd988e525f9",
        "03bdbdbe80842c73f86c1dd82212757788fb4f349df781330fcdc815a3d76550",
        "c91c850c5c09e84795013b89b6ef185f2905c9949bbe22ad203709d8836773a5",
        "574e473e88991d9dfa6087654ff5b59ee3cb124b7abc3355e7737b501a4b80fa",
        "497dd951bbb5547f2817891ad91393b583883101331770303cdc378e0fb5ffec",
        "0429b1c9611371348cb6ed89223a8883a6a065b6f0d8db4fc3e67188f1a7fb99",
        "caef2fe075601913d4878d04ac9faa84b0c4744a40c195db4e73f4f1cf9987a0",
        "4d0caf0c14bd98fcf6ebd9f7d29738d255b7c2f756ea48c6f8c41b0ed451d921",
        "2a976596fabdc1ebb958320d8d54485a015438d1bc9c08ae854e0761abf048a6",
        "230304807723f2a956f7ce81ba9bbfb1dad9be3cf96699d1591793a8c4b1121a",
        "76b955741341723249680016cedea3931861982bb433222a9595ff43da07667c",
        "75f1b3f69dbadbcdd0e63a116efff9ab0f68d388824b9baed0a401535efee58c",
        "33bea1d499b1c41631142977959333b1def70035840d3c86db8817cab0ad5280",
        "247990c6555d7ee136f4a12e507d38e177bbc549e33ca8258159d9678623dc6e",
        "f700111f88ac5f3aa1f47280d8aef76f66fb8e81f8e147d828e56ff1f5b0adc8",
        "c0b52aa436f3ff9cfc2fa574ce08ab2b8095627dc8adb225644ef34b7a1dedcf",
        "5788e14dfcf4302ae6cd55d7e123e4b5beff84a36f06e3a6f9d75d466be90760",
        "0e40e0f291f7734c31a4c32fe5b9fc8f57aa574e6f79ed7038903cbe4afb61a3",
        "e28232536dfc2703b4388d25ce119a8150807c720e28409cc0cf88a8ea940c22",
        "860c4dcb6487e20380db62a122470eab2e246fd485e6edb5c728aa2b2baa5f90",
        "3dd9a903b24d8315cd3eeb97b85c47f4f7ee3fb0987d5e35844104ff0622456e",
        "23f340838e55bc36356521bb94d5e184a3a775ecbdf06935e210215b3bb64721",
        "77d080a6932df8eb2afd3ac8b570852a5c9496097162d799036732f9e7437937",
        "52458df072c8c2e2f4c217407db98dc7293adc7a2663d132ff17bb4ef1899c06",
        "2716441b4b3e09ea5e3f03ffddde10a71c132bd82d00618e9b9532d918142953",
        "f88d40529aa1b99d112e461640951af63fe8e82e389eaac98d599af7f61e6440",
        "d4cc8460340393f78c793e871a76da3177d798ad3d4a8886d05e713167c6a871",
        "d37af674767b2c8c5e16a022ccf0162ac09c900df51af2ee5788ce66b316c623",
        "cab5fb604edaace5c7f9d2e39e4656821f42848d947c3e1a1b32f5f46e5394dc",
        "c7212054c7106572ecb6b991623db126c4792194794389421679b84f17027734",
        "c70ea137395090da8128ad3f8428225748ef658bb89f63fa6b96289011d13a2a",
        "c0fbd6eb0d0b4aa1d3ea9830ea0af8252f430ceb41d0f6a78881d2360fb579c2",
        "b7a36f53b68a9b5a56f3894601a95327724e270e996857e9c03548a1c58485b4",
        "b1bf12dd7d3f2d66e2a1c268743993cb652f4e74e186f2628fa2a8792c882e23",
        "a50825e3d1d6b25379191dfabccb56398f2e600b5de9befcc75ed378f8f12611",
        "a4a33a1229d25e697860986cdba564b18393ef279598fa5cae49499e5ccaa9d4",
        "9f9b80ecc166241b4275a79ba3575b3b16924211464c6ee6218d5d5a5b55d1fe",
        "918974a8e9ff052a546dbc661b40f52738ccb505aa64ca24daf863077ede4e53",
        "85165963f057b60bf5d7f8410fc64739dbae3af83234045159a1c2f0b4447cfc",
        "83c4dd95a90688644e548c832e5f155a62826021ae8a6d6d192bde74d3ac666f",
        "7eea9036056de0ffb431279efdb1c928074717cddc8d9627f4944bfe255b5d1d",
        "7eaccc8f70286c7c300138b04b3db54bfb7baf4262a40980f86e571cab08946a",
        "6c53c25f3b6168063f61d39f352d339ed7e956909f4aefa7881995a491fbfdea",
        "6bf380ba0f258057577aee5ea10c905ada451cc6dcacafd60206297061be8db1",
        "649811db59332d0603c370b5042f0658dd6f73cadeba5f4173b5738b4252af18",
        "6348d06ec871a16b493b49869224fd4c94995af8dcf5048779d43d9fe148946f",
        "5c7ac75d21acf6f4b6cfb87c502edb91a9f367d33d36706c59e9ef174091d2a6",
        "3b5c3862443c3fdb5cd0afa24a3de3a28a19af38e63cc6dfaf5f611b26faceda",
        "31456763f513deafbd73a358559cb0e3582bd4c600b4c6fbec8b10d0603c2f21",
        "2b27fd7fc14fd06d0c806213e040486863dc96ee3c007c58382f04e8b846b86c",
        "262ad8574b3d278199cd9101e773fb85b0574f58b324adeda1330396aa5fc252",
        "23fc7ccccbfb1d24078c66c7e294afdc44996b119240189648bd3e12241ed9ee",
        "22cbe17dc4ba11f8aee50941de920c0f99a24bc5045df5c7aa136dbc42f05c8e",
        "13813b318fdb898dfc51e90f671d0d8774f7886d883e8567adece44e860ddd47",
        "0d85347ab8f6d5a8ca6614ee52d7d77c38b3096465c64d9b9f24d750b9dc0e56",
        "0815a71cd0f718cafb7582572e9eeac88e25df692e96c01f7c6473c59bf436fc",
    ]
}

pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
    capability_0815a71cd0f718ca::run_generated_capability(inputs)
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
        "1b4313ac8f9356bcf981e14f319c07064ffe3ad97f7d5445e8865844ebb65dec" => {
            capability_1b4313ac8f9356bc::run_generated_capability(inputs)
        }
        "3eb34270273d5a51d5f54d06afa54c512959b605f265d1a5eb4b3cc2803050fb" => {
            capability_3eb34270273d5a51::run_generated_capability(inputs)
        }
        "962f67422edea84953f3ec46d4db7cfba8579a02a0fadb5ea29617cd4b8dab0e" => {
            capability_962f67422edea849::run_generated_capability(inputs)
        }
        "4b7e0f9e0e467c0d00d0867850c30de9b49aedfa2a2cbd89e8e88dd988e525f9" => {
            capability_4b7e0f9e0e467c0d::run_generated_capability(inputs)
        }
        "03bdbdbe80842c73f86c1dd82212757788fb4f349df781330fcdc815a3d76550" => {
            capability_03bdbdbe80842c73::run_generated_capability(inputs)
        }
        "c91c850c5c09e84795013b89b6ef185f2905c9949bbe22ad203709d8836773a5" => {
            capability_c91c850c5c09e847::run_generated_capability(inputs)
        }
        "574e473e88991d9dfa6087654ff5b59ee3cb124b7abc3355e7737b501a4b80fa" => {
            capability_574e473e88991d9d::run_generated_capability(inputs)
        }
        "497dd951bbb5547f2817891ad91393b583883101331770303cdc378e0fb5ffec" => {
            capability_497dd951bbb5547f::run_generated_capability(inputs)
        }
        "0429b1c9611371348cb6ed89223a8883a6a065b6f0d8db4fc3e67188f1a7fb99" => {
            capability_0429b1c961137134::run_generated_capability(inputs)
        }
        "caef2fe075601913d4878d04ac9faa84b0c4744a40c195db4e73f4f1cf9987a0" => {
            capability_caef2fe075601913::run_generated_capability(inputs)
        }
        "4d0caf0c14bd98fcf6ebd9f7d29738d255b7c2f756ea48c6f8c41b0ed451d921" => {
            capability_4d0caf0c14bd98fc::run_generated_capability(inputs)
        }
        "2a976596fabdc1ebb958320d8d54485a015438d1bc9c08ae854e0761abf048a6" => {
            capability_2a976596fabdc1eb::run_generated_capability(inputs)
        }
        "230304807723f2a956f7ce81ba9bbfb1dad9be3cf96699d1591793a8c4b1121a" => {
            capability_230304807723f2a9::run_generated_capability(inputs)
        }
        "76b955741341723249680016cedea3931861982bb433222a9595ff43da07667c" => {
            capability_76b9557413417232::run_generated_capability(inputs)
        }
        "75f1b3f69dbadbcdd0e63a116efff9ab0f68d388824b9baed0a401535efee58c" => {
            capability_75f1b3f69dbadbcd::run_generated_capability(inputs)
        }
        "33bea1d499b1c41631142977959333b1def70035840d3c86db8817cab0ad5280" => {
            capability_33bea1d499b1c416::run_generated_capability(inputs)
        }
        "247990c6555d7ee136f4a12e507d38e177bbc549e33ca8258159d9678623dc6e" => {
            capability_247990c6555d7ee1::run_generated_capability(inputs)
        }
        "f700111f88ac5f3aa1f47280d8aef76f66fb8e81f8e147d828e56ff1f5b0adc8" => {
            capability_f700111f88ac5f3a::run_generated_capability(inputs)
        }
        "c0b52aa436f3ff9cfc2fa574ce08ab2b8095627dc8adb225644ef34b7a1dedcf" => {
            capability_c0b52aa436f3ff9c::run_generated_capability(inputs)
        }
        "5788e14dfcf4302ae6cd55d7e123e4b5beff84a36f06e3a6f9d75d466be90760" => {
            capability_5788e14dfcf4302a::run_generated_capability(inputs)
        }
        "0e40e0f291f7734c31a4c32fe5b9fc8f57aa574e6f79ed7038903cbe4afb61a3" => {
            capability_0e40e0f291f7734c::run_generated_capability(inputs)
        }
        "e28232536dfc2703b4388d25ce119a8150807c720e28409cc0cf88a8ea940c22" => {
            capability_e28232536dfc2703::run_generated_capability(inputs)
        }
        "860c4dcb6487e20380db62a122470eab2e246fd485e6edb5c728aa2b2baa5f90" => {
            capability_860c4dcb6487e203::run_generated_capability(inputs)
        }
        "3dd9a903b24d8315cd3eeb97b85c47f4f7ee3fb0987d5e35844104ff0622456e" => {
            capability_3dd9a903b24d8315::run_generated_capability(inputs)
        }
        "23f340838e55bc36356521bb94d5e184a3a775ecbdf06935e210215b3bb64721" => {
            capability_23f340838e55bc36::run_generated_capability(inputs)
        }
        "77d080a6932df8eb2afd3ac8b570852a5c9496097162d799036732f9e7437937" => {
            capability_77d080a6932df8eb::run_generated_capability(inputs)
        }
        "52458df072c8c2e2f4c217407db98dc7293adc7a2663d132ff17bb4ef1899c06" => {
            capability_52458df072c8c2e2::run_generated_capability(inputs)
        }
        "2716441b4b3e09ea5e3f03ffddde10a71c132bd82d00618e9b9532d918142953" => {
            capability_2716441b4b3e09ea::run_generated_capability(inputs)
        }
        "f88d40529aa1b99d112e461640951af63fe8e82e389eaac98d599af7f61e6440" => {
            capability_f88d40529aa1b99d::run_generated_capability(inputs)
        }
        "d4cc8460340393f78c793e871a76da3177d798ad3d4a8886d05e713167c6a871" => {
            capability_d4cc8460340393f7::run_generated_capability(inputs)
        }
        "d37af674767b2c8c5e16a022ccf0162ac09c900df51af2ee5788ce66b316c623" => {
            capability_d37af674767b2c8c::run_generated_capability(inputs)
        }
        "cab5fb604edaace5c7f9d2e39e4656821f42848d947c3e1a1b32f5f46e5394dc" => {
            capability_cab5fb604edaace5::run_generated_capability(inputs)
        }
        "c7212054c7106572ecb6b991623db126c4792194794389421679b84f17027734" => {
            capability_c7212054c7106572::run_generated_capability(inputs)
        }
        "c70ea137395090da8128ad3f8428225748ef658bb89f63fa6b96289011d13a2a" => {
            capability_c70ea137395090da::run_generated_capability(inputs)
        }
        "c0fbd6eb0d0b4aa1d3ea9830ea0af8252f430ceb41d0f6a78881d2360fb579c2" => {
            capability_c0fbd6eb0d0b4aa1::run_generated_capability(inputs)
        }
        "b7a36f53b68a9b5a56f3894601a95327724e270e996857e9c03548a1c58485b4" => {
            capability_b7a36f53b68a9b5a::run_generated_capability(inputs)
        }
        "b1bf12dd7d3f2d66e2a1c268743993cb652f4e74e186f2628fa2a8792c882e23" => {
            capability_b1bf12dd7d3f2d66::run_generated_capability(inputs)
        }
        "a50825e3d1d6b25379191dfabccb56398f2e600b5de9befcc75ed378f8f12611" => {
            capability_a50825e3d1d6b253::run_generated_capability(inputs)
        }
        "a4a33a1229d25e697860986cdba564b18393ef279598fa5cae49499e5ccaa9d4" => {
            capability_a4a33a1229d25e69::run_generated_capability(inputs)
        }
        "9f9b80ecc166241b4275a79ba3575b3b16924211464c6ee6218d5d5a5b55d1fe" => {
            capability_9f9b80ecc166241b::run_generated_capability(inputs)
        }
        "918974a8e9ff052a546dbc661b40f52738ccb505aa64ca24daf863077ede4e53" => {
            capability_918974a8e9ff052a::run_generated_capability(inputs)
        }
        "85165963f057b60bf5d7f8410fc64739dbae3af83234045159a1c2f0b4447cfc" => {
            capability_85165963f057b60b::run_generated_capability(inputs)
        }
        "83c4dd95a90688644e548c832e5f155a62826021ae8a6d6d192bde74d3ac666f" => {
            capability_83c4dd95a9068864::run_generated_capability(inputs)
        }
        "7eea9036056de0ffb431279efdb1c928074717cddc8d9627f4944bfe255b5d1d" => {
            capability_7eea9036056de0ff::run_generated_capability(inputs)
        }
        "7eaccc8f70286c7c300138b04b3db54bfb7baf4262a40980f86e571cab08946a" => {
            capability_7eaccc8f70286c7c::run_generated_capability(inputs)
        }
        "6c53c25f3b6168063f61d39f352d339ed7e956909f4aefa7881995a491fbfdea" => {
            capability_6c53c25f3b616806::run_generated_capability(inputs)
        }
        "6bf380ba0f258057577aee5ea10c905ada451cc6dcacafd60206297061be8db1" => {
            capability_6bf380ba0f258057::run_generated_capability(inputs)
        }
        "649811db59332d0603c370b5042f0658dd6f73cadeba5f4173b5738b4252af18" => {
            capability_649811db59332d06::run_generated_capability(inputs)
        }
        "6348d06ec871a16b493b49869224fd4c94995af8dcf5048779d43d9fe148946f" => {
            capability_6348d06ec871a16b::run_generated_capability(inputs)
        }
        "5c7ac75d21acf6f4b6cfb87c502edb91a9f367d33d36706c59e9ef174091d2a6" => {
            capability_5c7ac75d21acf6f4::run_generated_capability(inputs)
        }
        "3b5c3862443c3fdb5cd0afa24a3de3a28a19af38e63cc6dfaf5f611b26faceda" => {
            capability_3b5c3862443c3fdb::run_generated_capability(inputs)
        }
        "31456763f513deafbd73a358559cb0e3582bd4c600b4c6fbec8b10d0603c2f21" => {
            capability_31456763f513deaf::run_generated_capability(inputs)
        }
        "2b27fd7fc14fd06d0c806213e040486863dc96ee3c007c58382f04e8b846b86c" => {
            capability_2b27fd7fc14fd06d::run_generated_capability(inputs)
        }
        "262ad8574b3d278199cd9101e773fb85b0574f58b324adeda1330396aa5fc252" => {
            capability_262ad8574b3d2781::run_generated_capability(inputs)
        }
        "23fc7ccccbfb1d24078c66c7e294afdc44996b119240189648bd3e12241ed9ee" => {
            capability_23fc7ccccbfb1d24::run_generated_capability(inputs)
        }
        "22cbe17dc4ba11f8aee50941de920c0f99a24bc5045df5c7aa136dbc42f05c8e" => {
            capability_22cbe17dc4ba11f8::run_generated_capability(inputs)
        }
        "13813b318fdb898dfc51e90f671d0d8774f7886d883e8567adece44e860ddd47" => {
            capability_13813b318fdb898d::run_generated_capability(inputs)
        }
        "0d85347ab8f6d5a8ca6614ee52d7d77c38b3096465c64d9b9f24d750b9dc0e56" => {
            capability_0d85347ab8f6d5a8::run_generated_capability(inputs)
        }
        "0815a71cd0f718cafb7582572e9eeac88e25df692e96c01f7c6473c59bf436fc" => {
            capability_0815a71cd0f718ca::run_generated_capability(inputs)
        }
        _ => Err("GENERATED_CAPABILITY_NOT_FOUND".to_string()),
    }
}
