#!/usr/bin/env python3
"""Parity test: Rust solver vs corrected numpy port.

Same deterministic initial state, 200 steps -> max|Δρ| < 1e-12.
This prevents the silent divergence that caused the review's §1 finding.
"""
import json
import os
import sys
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
TARGET_DIR = Path(r"E:\mm-target-qoi")
RUST_BIN = TARGET_DIR / "debug" / "gpe-parity.exe"
NUMPY_SCRIPT = REPO / "scripts" / "gpe_parity.py"
FIXTURE = REPO / "tests" / "fixtures" / "gpe_parity.json"

def build_rust():
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = str(TARGET_DIR)
    result = subprocess.run(
        ["cargo", "build", "--manifest-path", str(REPO / "crates/fluid-stratos/Cargo.toml"),
         "--example", "gpe_parity"],
        env=env, capture_output=True, text=True
    )
    if result.returncode != 0:
        print("Rust build failed:", result.stderr)
        return False
    return True

def run_rust():
    out = subprocess.run([str(TARGET_DIR / "debug" / "examples" / "gpe_parity.exe")], capture_output=True, text=True)
    if out.returncode != 0:
        print("Rust binary failed:", out.stderr)
        return None
    return json.loads(out.stdout)

def run_numpy():
    out = subprocess.run([sys.executable, str(NUMPY_SCRIPT)], capture_output=True, text=True)
    if out.returncode != 0:
        print("Numpy script failed:", out.stderr)
        return None
    return json.loads(out.stdout)

def main():
    if not build_rust():
        return 1
    rust = run_rust()
    numpy = run_numpy()
    if rust is None or numpy is None:
        return 1

    diff = abs(rust["max_delta_density"] - numpy["max_delta_density"])
    print(f"Rust   max|Δρ| = {rust['max_delta_density']:.12e}")
    print(f"Numpy  max|Δρ| = {numpy['max_delta_density']:.12e}")
    print(f"Diff   = {diff:.2e}")

    if diff < 1e-12:
        print("PARITY OK (diff < 1e-12)")
        return 0
    else:
        print("PARITY FAILED (diff >= 1e-12)")
        return 1

if __name__ == "__main__":
    sys.exit(main())