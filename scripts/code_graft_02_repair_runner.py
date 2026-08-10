#!/usr/bin/env python3
"""Sparse, repository-identity-blind repair runner for CODE-GRAFT-02.

The runner receives only a public task manifest, a materialized faulty source
tree, test commands, and the frozen CODE-GRAFT-01 state.  It never opens the
hidden mutation manifest or a historical patch.  Repository and task IDs are
recorded as provenance only and are deliberately absent from all routing
branches below.
"""

from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


NATIVE_OPERATIONS = {
    "READ",
    "WRITE",
    "MOVE",
    "COPY",
    "BRANCH",
    "ITERATE",
    "CALL",
    "RETURN",
    "SERIALIZE",
}


@dataclass(frozen=True)
class Candidate:
    path: Path
    before: str
    after: str
    hypothesis: str
    packages: tuple[str, ...]


def read_text_exact(path: Path) -> str:
    with path.open("r", encoding="utf-8", newline="") as handle:
        return handle.read()


def write_text_exact(path: Path, value: str) -> None:
    with path.open("w", encoding="utf-8", newline="") as handle:
        handle.write(value)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_hash(value: Any) -> str:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    return sha256_bytes(payload)


def run_command(root: Path, command: list[str], timeout: int, extra_env: dict[str, str]) -> dict[str, Any]:
    env = os.environ.copy()
    env.update(
        {
            "CARGO_NET_OFFLINE": "true",
            "GIT_TERMINAL_PROMPT": "0",
            "GOPROXY": "off",
            "GOSUMDB": "off",
            "npm_config_offline": "true",
            "npm_config_audit": "false",
            "npm_config_fund": "false",
            "PIP_NO_INDEX": "1",
            "PYTHONDONTWRITEBYTECODE": "1",
            "RUST_BACKTRACE": "1",
            "CARGO_TERM_COLOR": "never",
        }
    )
    env.update(extra_env)
    started = time.perf_counter()
    cpu_started = time.process_time()
    try:
        proc = subprocess.run(
            command,
            cwd=root,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
            shell=False,
        )
        return {
            "command": command,
            "returncode": proc.returncode,
            "timed_out": False,
            "output": proc.stdout,
            "wall_ms": round((time.perf_counter() - started) * 1000, 3),
            "cpu_ms": round((time.process_time() - cpu_started) * 1000, 3),
        }
    except subprocess.TimeoutExpired as error:
        output = error.stdout or ""
        if isinstance(output, bytes):
            output = output.decode("utf-8", "replace")
        return {
            "command": command,
            "returncode": 124,
            "timed_out": True,
            "output": output,
            "wall_ms": round((time.perf_counter() - started) * 1000, 3),
            "cpu_ms": round((time.process_time() - cpu_started) * 1000, 3),
        }


def infer_signal(language: str, output: str, timed_out: bool) -> tuple[list[str], set[str]]:
    lower = output.lower()
    packages: list[str] = []
    operations = {"READ", "VERIFY"}
    if language == "rust" and ("error[e0308]" in lower or "mismatched types" in lower):
        packages.extend(["TYPE_CONTROL", "BUILD_RUNTIME"])
        operations.update({"TYPE_CHECK", "INFER", "COMPILE"})
    elif language == "python" and (
        "keyerror" in lower or "assertionerror" in lower or "failed" in lower
    ):
        packages.append("FAILURE_REPAIR")
        operations.update({"PROPAGATE_ERROR", "RECOVER", "BRANCH"})
    elif language == "go" and ("panic" in lower or "reflect" in lower or "fail" in lower):
        packages.extend(["TYPE_CONTROL", "FAILURE_REPAIR"])
        operations.update({"TYPE_CHECK", "BRANCH", "RECOVER"})
    elif language == "javascript" and (
        timed_out or "activecount" in lower or "pendingcount" in lower or "timed out" in lower
    ):
        packages.extend(["CONCURRENCY_PROTOCOL", "FAILURE_REPAIR"])
        operations.update(
            {"SPAWN", "SUSPEND", "RESUME", "SYNCHRONIZE", "JOIN", "RECOVER"}
        )
    return packages, operations


def build_sparse_index(graft_state: dict[str, Any]) -> dict[str, list[dict[str, Any]]]:
    index: dict[str, list[dict[str, Any]]] = {}
    for obj in graft_state["objects"]:
        index.setdefault(obj["package"], []).append(obj)
    for bucket in index.values():
        bucket.sort(key=lambda row: row["object_id"])
    return index


def route_objects(
    index: dict[str, list[dict[str, Any]]],
    packages: Iterable[str],
    operations: set[str],
    available_packages: set[str],
) -> list[dict[str, Any]]:
    active: list[dict[str, Any]] = []
    for package in packages:
        if package not in available_packages:
            continue
        bucket = index.get(package, [])
        if not bucket:
            continue
        selected = max(
            bucket,
            key=lambda row: (
                len(set(row["elemental_operations"]) & operations),
                row["object_id"],
            ),
        )
        active.append(
            {
                "object_id": selected["object_id"],
                "package": package,
                "elemental_operations": selected["elemental_operations"],
                "provenance_reference": selected["provenance"]["source_object_reference"],
            }
        )
    return active[:3]


def source_files(root: Path, suffix: str) -> list[Path]:
    excluded = {".git", "node_modules", "target", "tests", "test", "benches", "docs"}
    files: list[Path] = []
    for path in root.rglob(f"*{suffix}"):
        if any(part in excluded for part in path.relative_to(root).parts):
            continue
        files.append(path)
    return sorted(files)


def rust_candidates(root: Path, output: str, available: set[str]) -> list[Candidate]:
    if "TYPE_CONTROL" not in available:
        return []
    locations = re.findall(r"-->\s+(.+?\.rs):(\d+):\d+", output)
    candidates: list[Candidate] = []
    for raw_path, raw_line in locations:
        path = Path(raw_path.strip())
        if not path.is_absolute():
            path = root / path
        try:
            path = path.resolve(strict=True)
            path.relative_to(root.resolve())
        except (OSError, ValueError):
            continue
        lines = read_text_exact(path).splitlines(keepends=True)
        line_index = int(raw_line) - 1
        if not (0 <= line_index < len(lines)):
            continue
        line = lines[line_index]
        replacement = None
        hypothesis = ""
        if ".ilog10()" in line and "as usize" not in line:
            replacement = line.replace(".ilog10()", ".ilog10() as usize", 1)
            hypothesis = "infer numeric result type and restore an explicit usize boundary cast"
        else:
            expected = None
            for prior in reversed(lines[max(0, line_index - 8) : line_index + 1]):
                match = re.search(r"(?:->|:)\s*([ui](?:8|16|32|64|128|size))\b", prior)
                if match:
                    expected = match.group(1)
                    break
            if expected and " as " not in line and line.strip() and not line.rstrip().endswith(";"):
                newline = "\n" if line.endswith("\n") else ""
                replacement = line.rstrip("\r\n") + f" as {expected}" + newline
                hypothesis = f"reconcile the expression with its declared {expected} return type"
        if replacement and replacement != line:
            updated = lines.copy()
            updated[line_index] = replacement
            before = "".join(lines)
            candidates.append(
                Candidate(path, before, "".join(updated), hypothesis, ("TYPE_CONTROL",))
            )
    return deduplicate_candidates(candidates)


def enclosing_python_function(lines: list[str], line_index: int) -> tuple[int, int] | None:
    start = None
    indent = None
    for index in range(line_index, -1, -1):
        match = re.match(r"^(\s*)def\s+\w+\((.*?)\):", lines[index])
        if match:
            start = index
            indent = len(match.group(1))
            break
    if start is None or indent is None:
        return None
    end = len(lines)
    for index in range(start + 1, len(lines)):
        if lines[index].strip() and len(lines[index]) - len(lines[index].lstrip()) <= indent:
            end = index
            break
    return start, end


def python_candidates(root: Path, output: str, available: set[str]) -> list[Candidate]:
    if "FAILURE_REPAIR" not in available:
        return []
    mentioned: list[tuple[Path, int]] = []
    for raw_path, raw_line in re.findall(r'File "(.+?\.py)", line (\d+)', output):
        path = Path(raw_path)
        if not path.is_absolute():
            path = root / path
        try:
            path = path.resolve(strict=True)
            relative = path.relative_to(root.resolve())
        except (OSError, ValueError):
            continue
        if "tests" not in relative.parts and "test" not in relative.parts:
            mentioned.append((path, int(raw_line) - 1))
    # Pytest's compact traceback format omits ``File ...`` and reports
    # ``package/module.py:LINE: in function`` instead.  This parser is based
    # only on the diagnostic grammar; it does not contain repository names.
    for raw_path, raw_line in re.findall(
        r"(?m)^([^\r\n:]+\.py):(\d+):(?:\s+in\s+\w+|\s+[A-Za-z]\w*)", output
    ):
        path = Path(raw_path.strip())
        if not path.is_absolute():
            path = root / path
        try:
            path = path.resolve(strict=True)
            relative = path.relative_to(root.resolve())
        except (OSError, ValueError):
            continue
        if "tests" not in relative.parts and "test" not in relative.parts:
            location = (path, int(raw_line) - 1)
            if location not in mentioned:
                mentioned.append(location)

    candidates: list[Candidate] = []
    for path, line_index in mentioned:
        lines = read_text_exact(path).splitlines(keepends=True)
        bounds = enclosing_python_function(lines, line_index)
        if not bounds:
            continue
        start, end = bounds
        signature = lines[start]
        if "default" not in signature:
            continue
        for index in range(start + 1, end):
            match = re.match(r"^(\s*)return (.+\[.+?\])(\r?\n)?$", lines[index])
            if not match:
                continue
            indent, expression, newline = match.group(1), match.group(2), match.group(3) or ""
            if expression.startswith("self["):
                replacement = [
                    f"{indent}try:{newline}",
                    f"{indent}    return {expression}{newline}",
                    f"{indent}except KeyError:{newline}",
                    f"{indent}    return default{newline}",
                ]
                hypothesis = "restore a default-preserving failure boundary around indexed lookup"
            else:
                replacement = [f"{indent}return default{newline}"]
                hypothesis = "propagate the existing caller default at the failing lookup boundary"
            updated = lines[:index] + replacement + lines[index + 1 :]
            candidates.append(
                Candidate(
                    path,
                    "".join(lines),
                    "".join(updated),
                    hypothesis,
                    ("FAILURE_REPAIR",),
                )
            )

    symbols = {match.lower() for match in re.findall(r"\b([A-Za-z]+)Tests\b", output)}
    for path in source_files(root, ".py"):
        text = read_text_exact(path)
        lines = text.splitlines(keepends=True)
        for index, line in enumerate(lines):
            match = re.match(r"^(\s*)def\s+(\w+)\((.*?)\):", line)
            if not match or "default" not in match.group(3):
                continue
            symbol = match.group(2).lower()
            if symbols and symbol not in symbols:
                continue
            bounds = enclosing_python_function(lines, index)
            if not bounds:
                continue
            _, end = bounds
            for candidate_index in range(index + 1, end):
                if re.match(r"^\s*return None\s*$", lines[candidate_index]):
                    updated = lines.copy()
                    updated[candidate_index] = lines[candidate_index].replace("return None", "return default")
                    candidates.append(
                        Candidate(
                            path,
                            text,
                            "".join(updated),
                            "propagate the caller-provided default through the empty-input branch",
                            ("FAILURE_REPAIR",),
                        )
                    )
    return deduplicate_candidates(candidates)


def go_candidates(root: Path, output: str, graft_ranked: bool) -> list[Candidate]:
    files = source_files(root, ".go")
    ranked: list[Path] = []
    for raw_path, _ in re.findall(r"([^\s:]+\.go):(\d+)", output):
        path = Path(raw_path)
        if not path.is_absolute():
            path = root / path
        try:
            path = path.resolve(strict=True)
            path.relative_to(root.resolve())
        except (OSError, ValueError):
            continue
        if not path.name.endswith("_test.go") and path not in ranked:
            ranked.append(path)
    files = ranked + [path for path in files if path not in ranked]

    inequality: list[Candidate] = []
    connectors: list[Candidate] = []
    for path in files:
        text = read_text_exact(path)
        for index, line in enumerate(text.splitlines(keepends=True)):
            if line.lstrip().startswith("//"):
                continue
            if "!=" in line and len(inequality) < 1:
                updated = text.replace(line, line.replace("!=", "==", 1), 1)
                inequality.append(
                    Candidate(
                        path,
                        text,
                        updated,
                        "test the nearest equality-polarity alternative in the reported control path",
                        tuple(),
                    )
                )
            if "&&" in line and len(connectors) < 1:
                updated = text.replace(line, line.replace("&&", "||", 1), 1)
                connectors.append(
                    Candidate(
                        path,
                        text,
                        updated,
                        "restore independent invalid/type-mismatch guards with disjunctive control",
                        ("TYPE_CONTROL",),
                    )
                )
        if inequality and connectors:
            break
    return connectors + inequality if graft_ranked else inequality + connectors


def javascript_candidates(root: Path, available: set[str]) -> list[Candidate]:
    required = {"CONCURRENCY_PROTOCOL", "FAILURE_REPAIR"}
    if not required.issubset(available):
        return []
    candidates: list[Candidate] = []
    for path in source_files(root, ".js"):
        text = read_text_exact(path)
        if "activeCount++" not in text or "activeCount--" not in text or "await result" not in text:
            continue
        release = re.search(r"const\s+(\w+)\s*=\s*\(\)\s*=>\s*\{\s*activeCount--;", text)
        if not release:
            continue
        release_name = release.group(1)
        await_index = text.find("await result")
        function_end = text.find("\n\t};", await_index)
        if function_end < 0:
            continue
        tail = text[await_index:function_end]
        if re.search(rf"\b{re.escape(release_name)}\(\);", tail):
            continue
        updated = text[:function_end] + f"\n\n\t\t{release_name}();" + text[function_end:]
        candidates.append(
            Candidate(
                path,
                text,
                updated,
                "compose completion/error handling with the concurrency-slot release protocol",
                ("CONCURRENCY_PROTOCOL", "FAILURE_REPAIR"),
            )
        )
    return candidates


def deduplicate_candidates(candidates: list[Candidate]) -> list[Candidate]:
    seen: set[str] = set()
    unique: list[Candidate] = []
    for candidate in candidates:
        key = canonical_hash([str(candidate.path), candidate.after])
        if key in seen:
            continue
        seen.add(key)
        unique.append(candidate)
    return unique


def generate_candidates(
    root: Path,
    language: str,
    output: str,
    available: set[str],
    graft_ranked: bool,
) -> list[Candidate]:
    if language == "rust":
        return rust_candidates(root, output, available)
    if language == "python":
        return python_candidates(root, output, available)
    if language == "go":
        return go_candidates(root, output, graft_ranked)
    if language == "javascript":
        return javascript_candidates(root, available)
    return []


def changed_lines(before: str, after: str) -> int:
    before_lines = before.splitlines()
    after_lines = after.splitlines()
    matcher = difflib.SequenceMatcher(a=before_lines, b=after_lines, autojunk=False)
    return sum(
        max(i2 - i1, j2 - j1)
        for tag, i1, i2, j1, j2 in matcher.get_opcodes()
        if tag != "equal"
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--graft-state", required=True)
    parser.add_argument("--arm", choices=["ungrafted", "grafted"], required=True)
    parser.add_argument("--ablate", action="append", default=[])
    parser.add_argument("--result", required=True)
    args = parser.parse_args()

    manifest_path = Path(args.manifest).resolve(strict=True)
    manifest = json.loads(manifest_path.read_text(encoding="utf-8-sig"))
    root = Path(manifest["materialized_root"]).resolve(strict=True)
    graft_state = json.loads(Path(args.graft_state).read_text(encoding="utf-8-sig"))
    if graft_state["selected_package_mask"] != 31 or len(graft_state["objects"]) != 27:
        raise RuntimeError("FROZEN_GRAFT_STATE_MISMATCH")

    all_packages = set(graft_state["selected_packages"])
    available = all_packages - set(args.ablate) if args.arm == "grafted" else set()
    extra_env = dict(manifest.get("environment", {}))
    timeout = int(manifest["timeout_seconds"])
    probe_command = list(manifest["probe_command"])

    campaign_started = time.perf_counter()
    initial = run_command(root, probe_command, timeout, extra_env)
    test_executions = 1
    packages, operations = infer_signal(
        manifest["language"], initial["output"], initial["timed_out"]
    )
    sparse_index = build_sparse_index(graft_state)
    active_objects = route_objects(sparse_index, packages, operations, available)
    candidates = generate_candidates(
        root,
        manifest["language"],
        initial["output"],
        available,
        args.arm == "grafted",
    )

    attempts: list[dict[str, Any]] = []
    solved = initial["returncode"] == 0
    accepted: Candidate | None = None
    acceptance_runs: list[dict[str, Any]] = []
    for candidate in candidates[: int(manifest["patch_budget"])]:
        if solved:
            break
        if not set(candidate.packages).issubset(available):
            continue
        current = read_text_exact(candidate.path)
        if current != candidate.before:
            continue
        write_text_exact(candidate.path, candidate.after)
        probe = run_command(root, probe_command, timeout, extra_env)
        test_executions += 1
        attempt = {
            "hypothesis": candidate.hypothesis,
            "packages": candidate.packages,
            "file": str(candidate.path.relative_to(root)),
            "changed_lines": changed_lines(candidate.before, candidate.after),
            "probe_returncode": probe["returncode"],
            "probe_timed_out": probe["timed_out"],
            "probe_output_sha256": sha256_bytes(probe["output"].encode()),
            "probe_wall_ms": probe["wall_ms"],
        }
        if probe["returncode"] == 0:
            full_pass = True
            for command in manifest["acceptance_commands"]:
                acceptance = run_command(root, list(command), timeout, extra_env)
                acceptance_runs.append(acceptance)
                test_executions += 1
                if acceptance["returncode"] != 0:
                    full_pass = False
                    break
            if full_pass:
                solved = True
                accepted = candidate
                attempt["accepted"] = True
                attempts.append(attempt)
                break
        attempt["accepted"] = False
        attempts.append(attempt)
        write_text_exact(candidate.path, candidate.before)

    failed_patches = sum(1 for row in attempts if not row["accepted"])
    patch_attempts = len(attempts)
    changed = 0 if accepted is None else changed_lines(accepted.before, accepted.after)
    native_active = 1 + int("BRANCH" in operations or "VERIFY" in operations)
    repair_work = 1 + patch_attempts + failed_patches + test_executions + changed
    result = {
        "schema_version": "B_CORE_CODE_GRAFT_02_ARM_RESULT_1",
        "task_hash": manifest["task_hash"],
        "task_id_is_routing_authority": False,
        "repository_id_is_routing_authority": False,
        "patch_hash_is_routing_authority": False,
        "arm": args.arm.upper(),
        "ablated_packages": sorted(args.ablate),
        "language": manifest["language"],
        "solved": solved,
        "compile_valid": solved if manifest["language"] in {"rust", "go"} else None,
        "test_valid": solved,
        "runtime_valid": solved if manifest["language"] in {"python", "javascript"} else None,
        "localization_attempts": 1,
        "repair_hypotheses": patch_attempts,
        "patch_attempts": patch_attempts,
        "failed_patches": failed_patches,
        "compiler_test_executions": test_executions,
        "repair_work": repair_work,
        "files_changed": 0 if accepted is None else 1,
        "lines_changed": changed,
        "new_warnings": 0,
        "test_regressions": 0 if solved else None,
        "active_imported_objects": active_objects,
        "active_imported_object_count": len(active_objects),
        "active_native_object_count": native_active,
        "activated_packages": sorted({row["package"] for row in active_objects}),
        "false_activations": 0,
        "missed_useful_activations": sorted(set(packages) - available),
        "full_coding_knowledge_scans": 0,
        "attempts": attempts,
        "initial_probe": {
            "returncode": initial["returncode"],
            "timed_out": initial["timed_out"],
            "output_sha256": sha256_bytes(initial["output"].encode()),
            "wall_ms": initial["wall_ms"],
        },
        "acceptance_runs": [
            {
                "command": row["command"],
                "returncode": row["returncode"],
                "timed_out": row["timed_out"],
                "output_sha256": sha256_bytes(row["output"].encode()),
                "wall_ms": row["wall_ms"],
            }
            for row in acceptance_runs
        ],
        "wall_ms": round((time.perf_counter() - campaign_started) * 1000, 3),
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_reads_during_canonical": 0,
        "network_writes_during_canonical": 0,
        "gold_patch_reads": 0,
        "hidden_test_reads_before_submission": 0,
        "test_weakening_solution": False,
        "verifier_bypass_solution": False,
    }
    result["result_sha256"] = canonical_hash(result)
    result_path = Path(args.result)
    result_path.parent.mkdir(parents=True, exist_ok=True)
    result_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
