use std::{
    env,
    io::{self, Write},
    process, thread,
    time::Duration,
};

use semantic_reasoning::sem27::engine::{run_post_scaffold_epoch, PostScaffoldEpochRequest};

fn main() {
    let request_json = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("missing request JSON");
        process::exit(2);
    });
    if env::args().nth(2).is_some() {
        eprintln!("unexpected argument");
        process::exit(2);
    }
    let request: PostScaffoldEpochRequest =
        serde_json::from_str(&request_json).unwrap_or_else(|error| {
            eprintln!("invalid request JSON: {error}");
            process::exit(2);
        });
    match run_post_scaffold_epoch(request) {
        Ok(result) => {
            println!(
                "{}",
                serde_json::to_string(&result).expect("serialize post scaffold epoch")
            );
            io::stdout().flush().expect("flush post scaffold epoch");
            if let Ok(milliseconds) = env::var("SEM27_MEASUREMENT_HOLD_MS") {
                if let Ok(milliseconds) = milliseconds.parse::<u64>() {
                    thread::sleep(Duration::from_millis(milliseconds.min(2_000)));
                }
            }
        }
        Err(error) => {
            eprintln!("PROBE_ERROR={error}");
            process::exit(1);
        }
    }
}
