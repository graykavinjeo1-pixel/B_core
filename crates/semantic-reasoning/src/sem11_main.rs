use std::{env, path::PathBuf};

use semantic_reasoning::sem11::{freeze_campaign, run_campaign};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    let action = env::args().nth(1).unwrap_or_else(|| "run".to_string());
    let result = match action.as_str() {
        "freeze" => freeze_campaign(&root),
        "run" => run_campaign(&root),
        _ => Err(format!("UNKNOWN_ACTION:{action}")),
    };
    match result {
        Ok(summary) => println!("{summary}"),
        Err(error) => {
            eprintln!("SEM11_STATUS=FAIL\nDISPOSITION={error}");
            std::process::exit(1);
        }
    }
}
