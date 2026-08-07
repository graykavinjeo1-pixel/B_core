use dockable_semantic_core::{CapabilityRequest, DockableCore, SemanticValue};
use semantic_core_adapters::DeterministicOffsetCapability;

fn main() {
    let core = DockableCore::load_embedded().expect("core load");
    let mut capability = DeterministicOffsetCapability::new(7);
    let result = core
        .execute_capability(
            &mut capability,
            CapabilityRequest {
                capability_id: "CAPABILITY.DETERMINISTIC_OFFSET.V1".to_string(),
                input: SemanticValue::Integer(5),
            },
        )
        .expect("contract-valid capability execution");
    assert_eq!(result.output, Some(SemanticValue::Integer(12)));
    assert!(result.contract_validated);
    println!("{}", serde_json::to_string(&result).expect("result JSON"));
}
