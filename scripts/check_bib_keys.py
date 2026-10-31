#!/usr/bin/env python3
"""Ellenorzi, hogy a LaTeX cite-kulcsok es a references.bib bejegyzesei egyeznek.

A bibtex nem jelzi a nem hasznalt bejegyzeseket, ezert ez a CI-step fogja meg
(https://github.com/silentnoisehun/quantum-organic-intelligence review, §6).
Kilepesi kod: 0 = egyezik, 1 = hiba.
"""
from __future__ import annotations

import io
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
TEX = REPO / "paper" / "aisc2027_unified_paper.tex"
BIB = REPO / "paper" / "references.bib"


def collect_cite_keys(text: str) -> set[str]:
    keys: set[str] = set()
    for match in re.finditer(r"\\cite\{([^}]*)\}", text):
        for key in match.group(1).split(","):
            key = key.strip()
            if key:
                keys.add(key)
    return keys


def collect_bib_keys(text: str) -> set[str]:
    return set(re.findall(r"@\w+\{([^,]+),", text))


def main() -> int:
    if not TEX.exists() or not BIB.exists():
        print("ERROR: paper sources not found", file=sys.stderr)
        return 1

    tex = io.open(TEX, "r", encoding="utf-8").read()
    bib = io.open(BIB, "r", encoding="utf-8").read()

    cited = collect_cite_keys(tex)
    defined = collect_bib_keys(bib)

    undefined = sorted(cited - defined)
    unused = sorted(defined - cited)

    print("cite keys : %d" % len(cited))
    print("bib keys  : %d" % len(defined))

    ok = True
    if undefined:
        print("UNDEFINED citations (no bib entry): %s" % ", ".join(undefined))
        ok = False
    if unused:
        print("UNUSED bib entries (never cited): %s" % ", ".join(unused))
        ok = False
    if ok:
        print("OK: every citation resolves and every bib entry is cited")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
