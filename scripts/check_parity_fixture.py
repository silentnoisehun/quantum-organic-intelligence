#!/usr/bin/env python3
"""Parity check: compare JSON output to fixture with 1e-12 tolerance."""
import json
import sys

def main():
    if len(sys.argv) != 3:
        print("Usage: check_parity_fixture.py <actual.json> <fixture.json>")
        return 1
    actual_path = sys.argv[1]
    fixture_path = sys.argv[2]
    
    with open(actual_path, "r") as f:
        actual = json.load(f)
    with open(fixture_path, "r") as f:
        fixture = json.load(f)
    
    for key in ("max_delta_density", "max_delta_mode_energy", "argmax_initial", "argmax_final"):
        if key not in actual or key not in fixture:
            print(f"FAIL: missing key {key}")
            return 1
        a = actual[key]
        f = fixture[key]
        if isinstance(a, float) and isinstance(f, float):
            diff = abs(a - f)
            print(f"{key}: actual={a:.12e} fixture={f:.12e} diff={diff:.2e}")
            if diff >= 1e-12:
                print(f"FAIL: {key} diff {diff:.2e} >= 1e-12")
                return 1
        else:
            if a != f:
                print(f"FAIL: {key} mismatch: {a} != {f}")
                return 1
            print(f"{key}: {a} == {f} OK")
    
    print("PARITY OK (all diffs < 1e-12)")
    return 0

if __name__ == "__main__":
    sys.exit(main())