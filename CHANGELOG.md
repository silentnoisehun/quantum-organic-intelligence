# Changelog

## [0.1.0] — 2026-10-27

### Added

- **`crates/fluid-stratos`** — a Gross-Pitaevskii fluid solver for 16 cognitive
  modes with a **real split-step Fourier** kinetic step (row + column
  transforms). The previous implementation used a phase rotation in index
  space with no transform at all, which made the density distribution
  mathematically invariant (measured: max |Δ| = 1.275e-13 over 500 steps).
  After the fix: max |Δ| = 2.766, a 2.2e13× improvement.
- Three regression tests that **fail on the pre-fix code**:
  the density distribution must move, the 16 mode energies must change,
  and the normalization must hold across FFT steps.
- **`crates/quantum-bridge`** — the hardware-verified quantum bridge:
  2 qubits + 3 separate measurements (A, f, φ), with the decoding formulas
  and the prohibition on `RZ` encoding. Includes the measured hardware
  parameters (Bell fidelity 87.7–99.3%, QDS T₂ = 89.6 ± 39.2 µs).
- A measurement of the 16-mode argmax via the quantum path:
  **38.65%** at Bell c = 0.949 versus **100%** for the classical argmax —
  the difference is 0.137σ and therefore not detectable at 1024 shots.
- **Quantum–classical loop simulation** (`scripts/quantum_loop_sim.py`) with
  four branches: quantum loop, classical control, uniform null model, and
  the dead GPE. Measured H/Hmax: 0.4355 / 0.0000 / 0.9962 / 0.0000.
- Measurement reports in `docs/`.

### Fixed

- The parallel solver carried the same phase-rotation defect as the serial
  one.
- The solver synchronization copied only the scalar fields across after a
  parallel step, so the wavefunction went stale and the 16 mode energies
  would have stayed static.
- The binomials sampler used a uniform variate where a normal variate was
  required, biasing the measured accuracy from 38.65% to ~69%.

### Known limitations

See section 4 ("Limits") of the README. In short: no proof of emergent
intelligence, the histogram test requires the classical control to avoid a
false positive, and the 16-mode argmax is not detectable via the quantum
path with the measured hardware parameters.
