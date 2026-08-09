use std::{
    io::{self, Read},
    process::ExitCode,
};

use semantic_reasoning::sem34::verifier::{handle, Sem34VerificationRequest};

fn main() -> ExitCode {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("SEM34_TRANSPORT_SCHEMA_ERROR");
        return ExitCode::FAILURE;
    }
    let request = match serde_json::from_str::<Sem34VerificationRequest>(&input) {
        Ok(request) => request,
        Err(_) => {
            eprintln!("SEM34_TRANSPORT_SCHEMA_ERROR");
            return ExitCode::FAILURE;
        }
    };
    match serde_json::to_string(&handle(request)) {
        Ok(response) => {
            println!("{response}");
            ExitCode::SUCCESS
        }
        Err(_) => {
            eprintln!("SEM34_TRANSPORT_SCHEMA_ERROR");
            ExitCode::FAILURE
        }
    }
}
