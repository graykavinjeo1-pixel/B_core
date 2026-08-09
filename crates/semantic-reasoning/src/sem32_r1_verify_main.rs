use std::{io::Read, process::ExitCode};

use semantic_reasoning::sem32_r1::verifier::{handle, R1VerificationRequest};

fn main() -> ExitCode {
    let mut input = Vec::new();
    if let Err(error) = std::io::stdin().read_to_end(&mut input) {
        eprintln!("READ_R1_VERIFICATION_REQUEST:{error}");
        return ExitCode::FAILURE;
    }
    let request: R1VerificationRequest = match serde_json::from_slice(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("PARSE_R1_VERIFICATION_REQUEST:{error}");
            return ExitCode::FAILURE;
        }
    };
    match serde_json::to_string(&handle(request)) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SERIALIZE_R1_VERIFICATION_RESPONSE:{error}");
            ExitCode::FAILURE
        }
    }
}
