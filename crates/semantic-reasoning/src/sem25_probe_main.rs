use std::{
    env,
    io::{self, Write},
    process, thread,
    time::Duration,
};

use semantic_reasoning::sem25::engine::{run_growth_probe, GrowthProbeRequest};

fn main() {
    let mut args = env::args().skip(1);
    let request_json = args.next().unwrap_or_else(|| {
        eprintln!("missing request JSON");
        process::exit(2);
    });
    if args.next().is_some() {
        eprintln!("unexpected argument");
        process::exit(2);
    }
    let request: GrowthProbeRequest = serde_json::from_str(&request_json).unwrap_or_else(|error| {
        eprintln!("invalid request JSON: {error}");
        process::exit(2);
    });
    match run_growth_probe(request) {
        Ok(result) => {
            println!(
                "{}",
                serde_json::to_string(&result).expect("serialize growth probe")
            );
            io::stdout().flush().expect("flush growth probe");
            if let Ok(milliseconds) = env::var("SEM25_MEASUREMENT_HOLD_MS") {
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
