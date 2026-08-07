use dockable_semantic_core::{
    task::Demonstration, DockableCore, GoalIR, CORE_ABI_VERSION, SEMANTIC_STATE_VERSION,
};

fn main() {
    let core = DockableCore::load_embedded().expect("load embedded semantic state and index");
    let goal = GoalIR {
        request_id: "CORE-X0-DIRECT-CANARY".to_string(),
        core_abi_version: CORE_ABI_VERSION,
        semantic_state_version: SEMANTIC_STATE_VERSION.to_string(),
        target_concept_id: "C000001".to_string(),
        scalar_parameter: 3,
        demonstrations: vec![
            Demonstration {
                input: vec![1, -2, 4],
                observed_output: vec![4, 1, 7],
            },
            Demonstration {
                input: vec![0, 3],
                observed_output: vec![3, 6],
            },
        ],
        query_input: vec![2, -1, 9],
        constraints: vec![
            "FINITE_SEQUENCE".to_string(),
            "CHECKED_ARITHMETIC".to_string(),
        ],
    };
    let result = core.execute_goal(&goal).expect("execute direct GoalIR");
    assert_eq!(result.output, Some(vec![5, 2, 12]));
    assert!(result.verified);
    assert_eq!(result.full_catalog_scans, 0);
    println!("{}", serde_json::to_string(&result).expect("result JSON"));
}
