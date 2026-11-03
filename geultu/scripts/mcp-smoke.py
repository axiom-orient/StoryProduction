#!/usr/bin/env python3
import json, os, subprocess, sys, tempfile
from pathlib import Path
root = Path(__file__).resolve().parents[1]
subprocess.run(
    [
        "cargo",
        "build",
        "--manifest-path",
        str(root / "Cargo.toml"),
        "-p",
        "geultu-cli",
        "--bin",
        "geultu",
        "--release",
        "--offline",
        "--target-dir",
        str(root / "target"),
    ],
    cwd=root,
    check=True,
)
binary = root / "target/release/geultu"
if not binary.is_file() or not os.access(binary, os.X_OK):
    raise SystemExit(f"mcp-smoke: release artifact missing: {binary}")
with tempfile.TemporaryDirectory(prefix="geultu-mcp-") as tmp:
    subprocess.run([binary, "--workspace", tmp, "init"], check=True, stdout=subprocess.DEVNULL)
    process = subprocess.Popen([binary, "--workspace", tmp, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    requests = [
        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","clientInfo":{"name":"smoke","version":"1"},"capabilities":{}}},
        {"jsonrpc":"2.0","method":"notifications/initialized"},
        {"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}},
        {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"project.create","arguments":{"project_id":"book","title":"Book","command_id":"mcp-create"}}},
    ]
    for request in requests:
        process.stdin.write(json.dumps(request, ensure_ascii=False) + "\n")
        process.stdin.flush()
        if "id" in request:
            response = json.loads(process.stdout.readline())
            assert response.get("id") == request["id"], response
            assert "error" not in response, response
    process.stdin.close()
    process.wait(timeout=5)
    assert process.returncode == 0, process.stderr.read()
print("mcp-smoke: PASS")
