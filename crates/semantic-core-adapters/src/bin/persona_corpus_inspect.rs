//! Local corpus integrity inspection. Raw text remains outside the executable
//! package; this binary emits only a sealed aggregate receipt.

use semantic_core_adapters::stream_pippa_deduped;
use std::{env, fs::File, io::BufReader, process::ExitCode};

fn main() -> ExitCode {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("usage: persona_corpus_inspect <pippa_deduped.jsonl>");
        return ExitCode::from(2);
    };
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("PIPPA_OPEN:{error}");
            return ExitCode::from(2);
        }
    };
    match stream_pippa_deduped(BufReader::new(file), |_| Ok(())) {
        Ok(report) if report.validate() => match serde_json::to_string_pretty(&report) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("PIPPA_REPORT_SERIALIZE:{error}");
                ExitCode::from(1)
            }
        },
        Ok(_) => {
            eprintln!("PIPPA_REPORT_INVALID");
            ExitCode::from(1)
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}
