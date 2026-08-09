use std::{env, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("current directory"));
    match semantic_reasoning::sem32_r1::seal_p0(&root) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SEM32_R1_P0_ERROR={error}");
            ExitCode::FAILURE
        }
    }
}
