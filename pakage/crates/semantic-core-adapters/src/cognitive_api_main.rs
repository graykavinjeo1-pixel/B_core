use std::io::{self, BufRead, BufReader, Read, Write};

use semantic_core_adapters::{CognitiveApi, CognitiveApiError, CognitiveApiResponseIR};

fn main() {
    let mut api = match CognitiveApi::new_embedded() {
        Ok(api) => api,
        Err(error) => {
            write_response(&CognitiveApiResponseIR {
                ok: false,
                payload: None,
                error: Some(error),
            });
            return;
        }
    };
    let mut input = BufReader::new(io::stdin().lock());
    let mut prefix = [0_u8; 2];
    match input.read_exact(&mut prefix) {
        Ok(()) => match detect_encoding(prefix) {
            InputEncoding::Utf8 { bom } => run_utf8(&mut api, &mut input, prefix, bom),
            InputEncoding::Utf16Le { bom } => run_utf16(&mut api, &mut input, prefix, bom, true),
            InputEncoding::Utf16Be { bom } => run_utf16(&mut api, &mut input, prefix, bom, false),
        },
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {}
        Err(_) => write_json_input_error(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputEncoding {
    Utf8 { bom: bool },
    Utf16Le { bom: bool },
    Utf16Be { bom: bool },
}

fn detect_encoding(prefix: [u8; 2]) -> InputEncoding {
    match prefix {
        [0xFF, 0xFE] => InputEncoding::Utf16Le { bom: true },
        [0xFE, 0xFF] => InputEncoding::Utf16Be { bom: true },
        [_, 0] => InputEncoding::Utf16Le { bom: false },
        [0, _] => InputEncoding::Utf16Be { bom: false },
        _ => InputEncoding::Utf8 { bom: false },
    }
}

fn run_utf8(api: &mut CognitiveApi, input: &mut impl BufRead, prefix: [u8; 2], bom: bool) {
    let mut line = if bom { Vec::new() } else { prefix.to_vec() };
    loop {
        match input.read_until(b'\n', &mut line) {
            Ok(0) => {
                process_utf8_line(api, &line);
                break;
            }
            Ok(_) => {
                process_utf8_line(api, &line);
                line.clear();
            }
            Err(_) => {
                write_json_input_error();
                break;
            }
        }
    }
}

fn process_utf8_line(api: &mut CognitiveApi, bytes: &[u8]) {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(line) => process_line(api, line),
        Err(_) => write_json_input_error(),
    }
}

fn run_utf16(
    api: &mut CognitiveApi,
    input: &mut impl Read,
    prefix: [u8; 2],
    bom: bool,
    little_endian: bool,
) {
    let mut units = Vec::new();
    if !bom {
        units.push(if little_endian {
            u16::from_le_bytes(prefix)
        } else {
            u16::from_be_bytes(prefix)
        });
    }
    let mut pair = [0_u8; 2];
    loop {
        match input.read_exact(&mut pair) {
            Ok(()) => {
                let unit = if little_endian {
                    u16::from_le_bytes(pair)
                } else {
                    u16::from_be_bytes(pair)
                };
                if unit == u16::from(b'\n') {
                    process_utf16_line(api, &units);
                    units.clear();
                } else {
                    units.push(unit);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                process_utf16_line(api, &units);
                break;
            }
            Err(_) => {
                write_json_input_error();
                break;
            }
        }
    }
}

fn process_utf16_line(api: &mut CognitiveApi, units: &[u16]) {
    match String::from_utf16(units) {
        Ok(line) => process_line(api, &line),
        Err(_) => write_json_input_error(),
    }
}

fn process_line(api: &mut CognitiveApi, line: &str) {
    let line = line.trim_matches(['\r', '\n', '\u{feff}']);
    if line.is_empty() {
        return;
    }
    match api.execute_command_json(line) {
        Ok(response) => {
            println!("{response}");
            let _ = io::stdout().flush();
        }
        Err(error) => write_response(&CognitiveApiResponseIR {
            ok: false,
            payload: None,
            error: Some(error),
        }),
    }
}

fn write_json_input_error() {
    write_response(&CognitiveApiResponseIR {
        ok: false,
        payload: None,
        error: Some(CognitiveApiError::JsonInput),
    });
}

fn write_response(response: &CognitiveApiResponseIR) {
    if let Ok(json) = serde_json::to_string(response) {
        println!("{json}");
        let _ = io::stdout().flush();
    }
}
