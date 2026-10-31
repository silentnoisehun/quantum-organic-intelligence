#!/usr/bin/env python3
"""Parity check: compare JSON output to fixture with 1e-12 tolerance."""
import json
import sys

def compare_vectors(a_vec, f_vec, name, tol=1e-12):
    if len(a_vec) != len(f_vec):
        print(f"FAIL: {name} length mismatch: {len(a_vec)} vs {len(f_vec)}")
        return False
    max_diff = max(abs(a - f) for a, f in zip(a_vec, f_vec))
    print(f"{name}: max_diff={max_diff:.2e}")
    if max_diff >= tol:
        print(f"FAIL: {name} max_diff {max_diff:.2e} >= {tol}")
        return False
    return True

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
    
    all_ok = True
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
                all_ok = False
        else:
            if a != f:
                print(f"FAIL: {key} mismatch: {a} != {f}")
                all_ok = False
            else:
                print(f"{key}: {a} == {f} OK")
    
    # Compare mode energy vectors
    for vec_key in ("mode_energy_initial", "mode_energy_final"):
        if vec_key in actual and vec_key in fixture:
            if not compare_vectors(actual[vec_key], fixture[vec_key], vec_key):
                all_ok = False
        elif vec_key in actual or vec_key in fixture:
            print(f"FAIL: missing {vec_key}")
            all_ok = False
    
    if all_ok:
        print("PARITY OK (all diffs < 1e-12)")
        return 0
    else:
        return 1

if __name__ == "__main__":
    sys.exit(main())