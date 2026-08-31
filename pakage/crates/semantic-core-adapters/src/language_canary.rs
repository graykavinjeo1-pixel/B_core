use dockable_semantic_core::DockableCore;
use semantic_core_adapters::LanguageAdapter;

fn main() {
    let adapter = LanguageAdapter;
    let goal = adapter
        .compile(
            "CORE-X0-LANGUAGE-CANARY",
            "각 값에 3을 더해",
            vec![2, -1, 9],
        )
        .expect("language grounding");
    let core = DockableCore::load_embedded().expect("core load");
    let result = core.execute_goal(&goal).expect("semantic execution");
    assert_eq!(result.output, Some(vec![5, 2, 12]));
    assert!(result.verified);
    println!("{}", serde_json::to_string(&result).expect("result JSON"));
}
