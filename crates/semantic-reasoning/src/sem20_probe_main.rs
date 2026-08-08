use std::{
    env,
    io::{self, Write},
    process, thread,
    time::Duration,
};

use semantic_reasoning::sem20::engine::{run_probe, ProbeRequest};

fn parse<T: std::str::FromStr>(value: Option<String>, name: &str) -> T {
    value
        .unwrap_or_else(|| panic!("missing {name}"))
        .parse()
        .unwrap_or_else(|_| panic!("invalid {name}"))
}

fn main() {
    let mut args = env::args().skip(1);
    let request = ProbeRequest {
        representation_mode: parse(args.next(), "representation_mode"),
        family_code: parse(args.next(), "family_code"),
        scale: parse(args.next(), "scale"),
        seed: parse(args.next(), "seed"),
        active_feature_mask: parse(args.next(), "active_feature_mask"),
        use_local_codebook: parse::<u8>(args.next(), "use_local_codebook") != 0,
    };
    if args.next().is_some() {
        eprintln!("unexpected argument");
        process::exit(2);
    }
    match run_probe(request) {
        Ok(result) => {
            println!(
                "{}",
                serde_json::to_string(&result).expect("serialize probe")
            );
            io::stdout().flush().expect("flush probe result");
            if let Ok(milliseconds) = env::var("SEM20_MEASUREMENT_HOLD_MS") {
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
