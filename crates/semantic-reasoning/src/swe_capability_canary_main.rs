use std::path::{Path, PathBuf};

use semantic_reasoning::benchmark_capability_canary::{
    run_benchmark_capability_canary, write_benchmark_capability_report,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn executable_candidates(name: &str) -> Vec<String> {
    if cfg!(windows) {
        vec![
            format!("{name}.exe"),
            format!("{name}.cmd"),
            format!("{name}.bat"),
            name.to_string(),
        ]
    } else {
        vec![name.to_string()]
    }
}

fn resolve_tool(environment_key: &str, name: &str, fallback: &Path) -> Result<PathBuf, String> {
    if let Some(configured) = std::env::var_os(environment_key) {
        let path = PathBuf::from(configured);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "CANARY_TOOL_OVERRIDE_NOT_FOUND:{environment_key}:{}",
            path.display()
        ));
    }
    if let Some(search_path) = std::env::var_os("PATH") {
        let candidates = executable_candidates(name);
        for directory in std::env::split_paths(&search_path) {
            for candidate in &candidates {
                let path = directory.join(candidate);
                if path.is_file() {
                    return Ok(path);
                }
            }
        }
    }
    if fallback.is_file() {
        return Ok(fallback.to_path_buf());
    }
    Err(format!("CANARY_TOOL_NOT_FOUND:{name}"))
}

fn run() -> Result<(), String> {
    let root = std::env::current_dir().map_err(|error| format!("CANARY_ROOT:{error}"))?;
    let node = resolve_tool(
        "B_CORE_NODE",
        "node",
        Path::new(r"C:\Program Files\nodejs\node.exe"),
    )?;
    let tsc = resolve_tool(
        "B_CORE_TSC",
        "tsc",
        Path::new(r"C:\Users\Administrator\AppData\Roaming\npm\tsc.cmd"),
    )?;
    let go = resolve_tool(
        "B_CORE_GO",
        "go",
        Path::new(r"C:\Program Files\Go\bin\go.exe"),
    )?;
    let report = run_benchmark_capability_canary(&node, &tsc, &go);
    let markdown = write_benchmark_capability_report(&root, &report)?;
    println!("REPORT={}", markdown.display());
    println!("DISPOSITION={}", report.disposition);
    if !report.pass {
        return Err(format!(
            "CANARY_FAILED:{}",
            report.failed_boundaries.join(",")
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_candidates_cover_windows_command_shims() {
        let candidates = executable_candidates("tsc");
        if cfg!(windows) {
            assert_eq!(candidates[1], "tsc.cmd");
        } else {
            assert_eq!(candidates, vec!["tsc"]);
        }
        assert!(!candidates[0].is_empty());
    }
}
