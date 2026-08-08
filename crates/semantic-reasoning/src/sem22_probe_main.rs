use std::{
    env,
    io::{self, Write},
    process, thread,
    time::Duration,
};

use semantic_reasoning::sem22::engine::{run_probe, ReactionRequest};

fn parse<T: std::str::FromStr>(value: Option<String>, name: &str) -> T {
    value
        .unwrap_or_else(|| panic!("missing {name}"))
        .parse()
        .unwrap_or_else(|_| panic!("invalid {name}"))
}

fn main() {
    let mut args = env::args().skip(1);
    let request = ReactionRequest {
        representation_mode: parse(args.next(), "representation_mode"),
        reactant_mask: parse(args.next(), "reactant_mask"),
        topology_code: parse(args.next(), "topology_code"),
        role_binding_mask: parse(args.next(), "role_binding_mask"),
        required_role_mask: parse(args.next(), "required_role_mask"),
        catalyst_mask: parse(args.next(), "catalyst_mask"),
        mediator_present: parse::<u8>(args.next(), "mediator_present") != 0,
        scale: parse(args.next(), "scale"),
        seed: parse(args.next(), "seed"),
        required_assumptions: parse(args.next(), "required_assumptions"),
        local_codebook: parse::<u8>(args.next(), "local_codebook") != 0,
    };
    if args.next().is_some() {
        eprintln!("unexpected argument");
        process::exit(2);
    }
    match run_probe(request) {
        Ok(result) => {
            println!(
                "{}",
                serde_json::to_string(&result).expect("serialize reaction probe")
            );
            io::stdout().flush().expect("flush reaction probe");
            if let Ok(milliseconds) = env::var("SEM22_MEASUREMENT_HOLD_MS") {
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
