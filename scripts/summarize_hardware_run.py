#!/usr/bin/env python3
"""
A nyers hardveres mérési adatok összegzése.

A `measurement_raw/` mappában lévő fájlokból olvassa ki az összesített
számokat. Így a jelentés minden állítása visszakövethető a mérési
fájljához — a szkript nem mér, csak összegez.

Minden futás külön szakaszban jelenik meg: a fájlnevekben a futás
címkéje (tag) szerepel, és az összefoglaló csak a saját futását
táplálja.

HASZNÁLAT
---------
    python scripts/summarize_hardware_run.py
    python scripts/summarize_hardware_run.py --tag 20261027T004200Z
"""
import argparse
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent


def load_raw(raw_dir):
    """A per-iterációs nyers fájlok betöltése, futáscímkénként csoportosítva."""
    files = sorted(raw_dir.glob("loop_*iter*.json"))
    runs = {}
    for path in files:
        with open(path, encoding="utf-8") as fh:
            data = json.load(fh)
        data["_file"] = path.name
        # a fájlnév: loop_<tag>_iter<NNNN>_<jobid>.json
        name = path.stem
        parts = name.split("_")
        tag = parts[1] if len(parts) > 2 else "ismeretlen"
        runs.setdefault(tag, []).append(data)

    for tag, rows in runs.items():
        summary_path = raw_dir / f"loop_summary_{tag}.json"
        if not summary_path.exists():
            continue
        with open(summary_path, encoding="utf-8") as fh:
            summary = json.load(fh)
        by_iter = {int(e["iteration"]): e for e in (summary.get("log") or [])}
        for row in rows:
            entry = by_iter.get(int(row.get("iteration", -1)))
            if entry:
                for key in ("ideal", "mode_argmax", "coherence", "c_live"):
                    if key in entry:
                        row[key] = entry[key]
                row["_summary"] = summary_path.name
    return runs


def mean(values):
    return sum(values) / len(values) if values else 0.0


def ideal_decode(encoded):
    """Az IDEÁLIS visszafejtett érték a kódolt értékből.

    🧬 Ugyanaz a képlet, mint a Rust `bridge-cli ideal-wave` parancsa
    (crates/quantum-bridge/src/bin/bridge-cli.rs). Az `A` és a `φ`
    visszafejtése involúciós a [0, π] tartományon, ezért a bázis a kódolt
    érték; az `f` csatorna modul 1 szerint összefügg, ezért abból az
    asin(|sin(π·f)|)/π főérték adódik.
    """
    import math
    a = float(encoded["A"])
    f = float(encoded["f"])
    phi = float(encoded["phi"])
    return {
        "A": a,
        "f": math.asin(abs(math.sin(math.pi * f))) / math.pi,
        "phi": phi,
    }


def report(tag, rows):
    print()
    print("=" * 78)
    print(f"A FUTÁS: {tag}  ({len(rows)} iteráció)")
    print("=" * 78)
    print(f"  backend          : {rows[0].get('backend', '?')}")
    print(f"  lövések          : {rows[0].get('shots', '?')}")

    qpu = [float(r.get("qpu_seconds", 0.0)) for r in rows]
    wall = [float(r.get("wall_seconds", 0.0)) for r in rows]
    print()
    print("  AZ IDŐK")
    print(f"    QPU-idő összesen : {sum(qpu):.2f} mp   átlag {mean(qpu):.2f} mp/iter")
    print(f"    falióra-idő      : {sum(wall):.1f} mp   átlag {mean(wall):.1f} mp/iter "
          f"(min {min(wall):.1f}, max {max(wall):.1f})")

    cs = [float(r["c_live"]) for r in rows if r.get("c_live") is not None]
    if cs:
        print()
        print("  A MÉRT HARDVER-MINŐSÉG (a Bell-anchorból)")
        print(f"    min {min(cs):.4f}   max {max(cs):.4f}   átlag {mean(cs):.4f}   "
              f"drift {max(cs) - min(cs):+.4f}")
        print("    (a referencia a mért 0,949 jellemző érték)")

    print()
    print("  A DEKÓDOLÁS PONTOSSÁGA (a bázis az ideális visszafejtés)")
    for key, label in (("A", "A"), ("f", "f"), ("phi", "φ")):
        for mode, name in (("decoded_raw", "nyers      "),
                           ("decoded_corrected", "torzítástlan")):
            errs = []
            biased = 0
            for r in rows:
                ideal = r.get("ideal")
                if ideal is None and r.get("encoded"):
                    ideal = ideal_decode(r["encoded"])
                    biased += 1
                if not ideal or mode not in r:
                    continue
                errs.append(float(r[mode][key]) - float(ideal[key]))
            if errs:
                abs_errs = [abs(e) for e in errs]
                signed = sum(errs) / len(errs)
                print(f"    {label} {name}: |hiba| átlag {mean(abs_errs):.5f}  "
                      f"max {max(abs_errs):.5f}  átlagos előjel {signed:+.5f}  n={len(errs)}"
                      + (f"  (a bázis {biased} esetben visszaszámítva)" if biased else ""))

    modes = [int(r["mode_argmax"]) for r in rows if "mode_argmax" in r]
    if modes:
        hist = [0] * 16
        for m in modes:
            hist[m] += 1
        print()
        print(f"  A 16 MÓD ARGMAX-ÁNAK SORRENDJE ({len(modes)} iteráció)")
        print(f"    {modes}")
        print(f"    a hisztogram: {hist}")
        print(f"    a meglátott módok száma: {sum(1 for h in hist if h > 0)}/16")
        top = max(range(16), key=lambda i: hist[i])
        print(f"    a leggyakoribb: #{top} ({hist[top]}x = {100 * hist[top] / len(modes):.1f}%)")

    coh = [float(r["coherence"]) for r in rows if "coherence" in r]
    if coh:
        print()
        print(f"  A KOHERENCIA: min {min(coh):.6f}  max {max(coh):.6f}  átlag {mean(coh):.6f}")

    print()
    print(f"  az első job: {rows[0].get('job_id', '?')}")
    print(f"  az utolsó job: {rows[-1].get('job_id', '?')}")
    print(f"  a nyers fájlok: {rows[0]['_file']} … {rows[-1]['_file']}")


def main():
    ap = argparse.ArgumentParser(description="A nyers hardveres mérés összegzése")
    ap.add_argument("--raw-dir", default=str(REPO / "measurement_raw"))
    ap.add_argument("--tag", default=None, help="csak a megadott futás")
    args = ap.parse_args()

    raw_dir = Path(args.raw_dir)
    runs = load_raw(raw_dir)
    if args.tag:
        runs = {k: v for k, v in runs.items() if args.tag in k}
    if not runs:
        print(f"nincs nyers fájl a {raw_dir} mappában — a mérési adatok nincsenek benne")
        return 0

    print("=" * 78)
    print("A NYERS HARDVERES MÉRÉS ÖSSZESÍTÉSE")
    print("=" * 78)
    print(f"  a {raw_dir} mappában {sum(len(v) for v in runs.values())} iterációs "
          f"nyers fájl, {len(runs)} futás")
    for tag in sorted(runs):
        report(tag, runs[tag])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
