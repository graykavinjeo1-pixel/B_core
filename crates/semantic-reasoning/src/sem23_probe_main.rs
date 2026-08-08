use std::{
    env,
    io::{self, Write},
    process, thread,
    time::Duration,
};

use semantic_reasoning::sem23::engine::{run_probe, GenerativeRequest};

fn parse<T: std::str::FromStr>(value: Option<String>, name: &str) -> T {
    value
        .unwrap_or_else(|| panic!("missing {name}"))
        .parse()
        .unwrap_or_else(|_| panic!("invalid {name}"))
}

fn main() {
    let mut args = env::args().skip(1);
    let request = GenerativeRequest {
        representation_mode: parse(args.next(), "representation_mode"),
        mechanism_mask: parse(args.next(), "mechanism_mask"),
        reactant_property_mask: parse(args.next(), "reactant_property_mask"),
        reactant_count: parse(args.next(), "reactant_count"),
        composite_reactant_count: parse(args.next(), "composite_reactant_count"),
        topology_code: parse(args.next(), "topology_code"),
        stoichiometry_code: parse(args.next(), "stoichiometry_code"),
        desired_property_mask: parse(args.next(), "desired_property_mask"),
        predicted_property_mask: parse(args.next(), "predicted_property_mask"),
        family_prior_mask: parse(args.next(), "family_prior_mask"),
        reaction_law_mask: parse(args.next(), "reaction_law_mask"),
        new_element_property_mask: parse(args.next(), "new_element_property_mask"),
        recursive_depth: parse(args.next(), "recursive_depth"),
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
                serde_json::to_string(&result).expect("serialize generative probe")
            );
            io::stdout().flush().expect("flush generative probe");
            if let Ok(milliseconds) = env::var("SEM23_MEASUREMENT_HOLD_MS") {
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
