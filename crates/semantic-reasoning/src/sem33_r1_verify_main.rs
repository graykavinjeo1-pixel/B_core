use std::{io::Read, process::ExitCode};

use semantic_reasoning::sem33_r1::verifier::{handle, Sem33VerificationRequest};

fn main() -> ExitCode {
    let mut input = Vec::new();
    if let Err(error) = std::io::stdin().read_to_end(&mut input) {
        eprintln!("SEM33_R1_TRANSPORT_READ_ERROR:{error}");
        return ExitCode::FAILURE;
    }
    let request: Sem33VerificationRequest = match serde_json::from_slice(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("SEM33_R1_TRANSPORT_SCHEMA_ERROR:{error}");
            return ExitCode::FAILURE;
        }
    };
    match serde_json::to_string(&handle(request)) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SEM33_R1_TRANSPORT_SERIALIZATION_ERROR:{error}");
            ExitCode::FAILURE
        }
    }
}
