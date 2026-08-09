use std::{io, process};

use semantic_core_adapters::{ExternalProblemAdapter, ExternalProblemInput};

fn main() {
    let input: ExternalProblemInput =
        serde_json::from_reader(io::stdin()).unwrap_or_else(|error| {
            eprintln!("INVALID_EXTERNAL_PROBLEM_INPUT:{error}");
            process::exit(2);
        });
    let output = ExternalProblemAdapter
        .compile(input)
        .unwrap_or_else(|error| {
            eprintln!("EXTERNAL_PROBLEM_ADAPTER_ERROR:{error:?}");
            process::exit(3);
        });
    serde_json::to_writer_pretty(io::stdout(), &output).expect("write external goal IR");
}
