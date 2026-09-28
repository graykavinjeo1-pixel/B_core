"""Run a small frozen-Q4 Korean surface naturalness audit.

The judge has no semantic authority and never sees or changes the canonical IR.
It evaluates unique transformed surfaces only.  GPU use is deliberately bounded
to one server slot and a small context.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import socket
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path


MODEL_ROOT = Path(r"C:\Users\Administrator\Documents\자재마켓\onapt-long-term-repair")
BINARY = MODEL_ROOT / "runtime/vendor/llama.cpp-b10872-win-vulkan-x64/llama-server.exe"
MODEL = MODEL_ROOT / "runtime/models/gemma-4-e4b/gemma-4-E4B-it-Q4_0.gguf"


def post(base: str, path: str, body: dict, timeout: int = 120) -> dict:
    req = urllib.request.Request(
        base + path,
        data=json.dumps(body, ensure_ascii=False).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=timeout) as response:
        return json.loads(response.read().decode())


def wait_ready(base: str, process: subprocess.Popen) -> None:
    deadline = time.monotonic() + 180
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"Q4_JUDGE_EXITED:{process.returncode}")
        try:
            with urllib.request.urlopen(base + "/health", timeout=2) as response:
                if response.status == 200:
                    return
        except (OSError, urllib.error.URLError):
            time.sleep(0.25)
    raise TimeoutError("Q4_JUDGE_READINESS_TIMEOUT")


def prompt(surface: str) -> str:
    return f"""다음 한 문장의 사실성이나 의미 내용은 평가하지 말고, 한국어 표면 문법과 관용적 자연스러움만 판정하세요.
조사 결합, 어미 결합, 존대 형태, 문장 종결, 실제 한국어 화자가 쓸 법한 표현을 엄격히 보세요.
조사 오류(예: 받침에 맞지 않는 로/으로)는 반드시 FAIL입니다.
첫 줄에는 PASS 또는 FAIL 중 하나만 쓰고, 둘째 줄에는 짧은 이유만 쓰세요.

문장: {surface}"""


def verdict(text: str) -> str:
    match = re.search(r"\b(PASS|FAIL)\b", text.upper())
    return match.group(1) if match else "UNPARSEABLE"


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--candidates", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    ap.add_argument("--log", type=Path, required=True)
    args = ap.parse_args()
    source = json.loads(args.candidates.read_text(encoding="utf-8"))["rows"]
    surfaces = sorted({row["surface"] for row in source})
    port_socket = socket.socket(); port_socket.bind(("127.0.0.1", 0)); port = port_socket.getsockname()[1]; port_socket.close()
    base = f"http://127.0.0.1:{port}"
    args.log.parent.mkdir(parents=True, exist_ok=True)
    rows = []
    started = time.perf_counter()
    with args.log.open("w", encoding="utf-8") as stream:
        process = subprocess.Popen([
            str(BINARY), "--model", str(MODEL), "--host", "127.0.0.1", "--port", str(port),
            "--alias", "bcore-naturalness-judge", "--threads", "4", "--threads-batch", "4",
            "--parallel", "1", "--ctx-size", "512", "--batch-size", "128", "--ubatch-size", "64",
            "--no-webui", "--jinja", "--no-warmup", "--gpu-layers", "999", "--device", "Vulkan0",
            "--flash-attn", "on", "--no-cache-prompt",
        ], stdout=stream, stderr=subprocess.STDOUT, creationflags=subprocess.CREATE_NO_WINDOW)
        try:
            wait_ready(base, process)
            for index, surface in enumerate(surfaces, 1):
                request_prompt = prompt(surface)
                rendered = post(base, "/apply-template", {
                    "messages": [{"role": "user", "content": request_prompt}],
                    "chat_template_kwargs": {"enable_thinking": False},
                })["prompt"]
                before = time.perf_counter()
                completion = post(base, "/completion", {
                    "prompt": rendered, "temperature": 0, "top_p": 1, "top_k": 1,
                    "seed": 9137, "n_predict": 40, "cache_prompt": False,
                })
                elapsed_ms = (time.perf_counter() - before) * 1000
                raw = completion["content"].strip()
                rows.append({
                    "surface_sha256": hashlib.sha256(surface.encode()).hexdigest(),
                    "surface": surface,
                    "verdict": verdict(raw),
                    "raw_judgement": raw,
                    "latency_ms": elapsed_ms,
                })
                print(json.dumps({"done": index, "total": len(surfaces), "verdict": rows[-1]["verdict"]}, ensure_ascii=False), flush=True)
        finally:
            process.terminate()
            try: process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill(); process.wait(timeout=5)
    artifact = {
        "schema": "BCORE.Q4_KOREAN_NATURALNESS_JUDGE.V1",
        "semantic_authority": False,
        "model": MODEL.name,
        "bf16_loaded": False,
        "parallel": 1,
        "context_size": 512,
        "unique_surface_count": len(rows),
        "elapsed_ms": (time.perf_counter() - started) * 1000,
        "rows": rows,
    }
    args.output.write_text(json.dumps(artifact, ensure_ascii=False, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
