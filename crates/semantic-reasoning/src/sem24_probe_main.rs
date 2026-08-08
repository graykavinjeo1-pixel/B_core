use std::{
    env,
    io::{self, Write},
    process, thread,
    time::Duration,
};

use semantic_reasoning::sem24::engine::{run_verification_probe, VerificationProbeRequest};

fn parse<T: std::str::FromStr>(value: Option<String>, name: &str) -> T {
    value
        .unwrap_or_else(|| panic!("missing {name}"))
        .parse()
        .unwrap_or_else(|_| panic!("invalid {name}"))
}

fn main() {
    let mut args = env::args().skip(1);
    let request = VerificationProbeRequest {
        arm_code: parse(args.next(), "arm_code"),
        object_id: parse(args.next(), "object_id"),
        semantic_hash: parse(args.next(), "semantic_hash"),
        dependency_hash: parse(args.next(), "dependency_hash"),
        certificate_dependency_hash: parse(args.next(), "certificate_dependency_hash"),
        total_claims: parse(args.next(), "total_claims"),
        inherited_claims: parse(args.next(), "inherited_claims"),
        affected_claims: parse(args.next(), "affected_claims"),
        emergent_claims: parse(args.next(), "emergent_claims"),
        verification_law_count: parse(args.next(), "verification_law_count"),
        certificate_depth: parse(args.next(), "certificate_depth"),
        novelty_code: parse(args.next(), "novelty_code"),
        topology_code: parse(args.next(), "topology_code"),
        resource_contract: parse(args.next(), "resource_contract"),
        scale: parse(args.next(), "scale"),
        seed: parse(args.next(), "seed"),
    };
    if args.next().is_some() {
        eprintln!("unexpected argument");
        process::exit(2);
    }
    match run_verification_probe(request) {
        Ok(result) => {
            println!(
                "{}",
                serde_json::to_string(&result).expect("serialize verification probe")
            );
            io::stdout().flush().expect("flush verification probe");
            if let Ok(milliseconds) = env::var("SEM24_MEASUREMENT_HOLD_MS") {
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
