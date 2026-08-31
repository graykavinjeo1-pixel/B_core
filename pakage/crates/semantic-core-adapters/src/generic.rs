use dockable_semantic_core::{
    interface::{
        CapabilityEffect, CapabilityPermission, CapabilityResources, SemanticType,
        CAPABILITY_CONTRACT_VERSION, CORE_ABI_VERSION,
    },
    Capability, CapabilityContract, CapabilityRequest, CapabilityResult, SemanticValue,
};

pub struct DeterministicOffsetCapability {
    offset: i64,
}

impl DeterministicOffsetCapability {
    pub fn new(offset: i64) -> Self {
        Self { offset }
    }
}

impl Capability for DeterministicOffsetCapability {
    fn contract(&self) -> CapabilityContract {
        CapabilityContract {
            capability_id: "CAPABILITY.DETERMINISTIC_OFFSET.V1".to_string(),
            core_abi_version: CORE_ABI_VERSION,
            contract_version: CAPABILITY_CONTRACT_VERSION,
            input_type: SemanticType::Integer,
            output_type: SemanticType::Integer,
            preconditions: vec!["checked addition is defined".to_string()],
            postconditions: vec!["output equals input plus declared offset".to_string()],
            effects: vec![CapabilityEffect::Pure],
            mutates_state: false,
            resources: CapabilityResources {
                max_wall_time_ms: 10,
                max_memory_bytes: 4096,
                deterministic: true,
            },
            failure_modes: vec!["ARITHMETIC_OVERFLOW".to_string()],
            permissions: vec![CapabilityPermission::None],
            semantic_relations: vec!["CHECKED_INTEGER_TRANSLATION".to_string()],
        }
    }

    fn execute(&mut self, request: CapabilityRequest) -> CapabilityResult {
        let output = match request.input {
            SemanticValue::Integer(value) => {
                value.checked_add(self.offset).map(SemanticValue::Integer)
            }
            _ => None,
        };
        CapabilityResult {
            capability_id: request.capability_id,
            failure: if output.is_none() {
                Some("ARITHMETIC_OVERFLOW_OR_TYPE_MISMATCH".to_string())
            } else {
                None
            },
            output,
            contract_validated: true,
        }
    }
}
