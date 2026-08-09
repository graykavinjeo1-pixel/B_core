use std::{io::Read, process::ExitCode};

use semantic_reasoning::sem33::verifier::{handle, Sem33VerificationRequest};

fn main() -> ExitCode {
    let mut input = Vec::new();
    if let Err(error) = std::io::stdin().read_to_end(&mut input) {
        eprintln!("READ_SEM33_VERIFICATION_REQUEST:{error}");
        return ExitCode::FAILURE;
    }
    let request: Sem33VerificationRequest = match serde_json::from_slice(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("PARSE_SEM33_VERIFICATION_REQUEST:{error}");
            return ExitCode::FAILURE;
        }
    };
    match serde_json::to_string(&handle(request)) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SERIALIZE_SEM33_VERIFICATION_RESPONSE:{error}");
            ExitCode::FAILURE
        }
    }
}
