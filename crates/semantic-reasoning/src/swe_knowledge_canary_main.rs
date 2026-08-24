use std::env;
use std::fs;
use std::path::PathBuf;

use semantic_reasoning::repository_coding_knowledge::run_repository_knowledge_canary;

fn main() -> Result<(), String> {
    let receipt = run_repository_knowledge_canary();
    let payload = serde_json::to_string_pretty(&receipt)
        .map_err(|error| format!("CANARY_SERIALIZE:{error}"))?;
    let mut args = env::args_os().skip(1);
    match (args.next(), args.next(), args.next()) {
        (None, None, None) => println!("{payload}"),
        (Some(flag), Some(path), None) if flag == "--output" => {
            let path = PathBuf::from(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("CANARY_OUTPUT_DIRECTORY:{error}"))?;
            }
            fs::write(&path, format!("{payload}\n"))
                .map_err(|error| format!("CANARY_OUTPUT_WRITE:{error}"))?;
            println!("{}", path.display());
        }
        _ => return Err("USAGE: swe-knowledge-canary [--output PATH]".to_string()),
    }
    if receipt.pass {
        Ok(())
    } else {
        Err("REPOSITORY_KNOWLEDGE_CANARY_FAILED".to_string())
    }
}
