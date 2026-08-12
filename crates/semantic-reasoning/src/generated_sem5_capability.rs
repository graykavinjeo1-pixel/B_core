#![allow(dead_code, unused_imports, unused_parens, unused_variables)]

use crate::sem5::model::Value;
use std::collections::BTreeMap;

pub const GENERATED_CAPABILITY_ACTIVE: bool = true;
pub const GENERATED_SOURCE_SCHEMA_REVISION: u64 = 4;
pub const GENERATED_PROGRAM_ID: &str = "P-T-010-9dffdefb-FirstPrinciplesD";
pub const GENERATED_PROGRAM_IR_SHA256: &str =
    "2716441b4b3e09ea5e3f03ffddde10a71c132bd82d00618e9b9532d918142953";
pub const GENERATED_CAPABILITY_COUNT: usize = 31;

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
    ]
}

pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {
    capability_2716441b4b3e09ea::run_generated_capability(inputs)
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
        _ => Err("GENERATED_CAPABILITY_NOT_FOUND".to_string()),
    }
}
