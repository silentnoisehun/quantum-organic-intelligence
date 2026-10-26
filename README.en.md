# Quantum Organic Intelligence

**A Gross-Pitaevskii fluid, the hardware-verified quantum bridge, and the measurement that decides whether any of it is emergent.**

This repository contains three things, and all three are measurable:

| | |
|---|---|
| **`crates/fluid-stratos`** | The GPE fluid solver for 16 cognitive modes, with a **real split-step Fourier** kinetic step |
| **`crates/quantum-bridge`** | The hardware-verified quantum bridge: **2 qubits + 3 separate measurements** (A, f, φ) — the hardware-measured scheme |
| **`docs/`** | The measurement reports: **measured numbers, methodology, and the limits** |

There is no marketing here. Every claim is backed by measurement data, and what cannot be proven is stated explicitly in the **Limits** section.

---

## 1. The Fluid STRATOS — what was broken and why

The solver's kinetic step approximated the kinetic operator with a **phase rotation** — in index space, with no transform at all. That is a mathematically dead solution: every step operation was either a phase multiplication (which does not change the modulus) or a uniform scaling (which does not change the ratios), so the density distribution stayed **exactly invariant**.

**Measured evidence before the fix** (64×64 grid, 500 steps, the `gpe.rs` parameters verbatim):

| quantity | value |
|---|---|
| max \|Δ\| in the density distribution | **1.275e-13** |
| max \|Δ\| in the 16 mode energies | **1.954e-14** |
| coherence | **always 1.000000** (the `tanh` fully saturated) |
| max/mean | **1112.84** — the fluid concentrated into a single point |

**The fix:** a real 2D FFT (row **and** column transforms, `rustfft` 6.4.1), keeping the `K/2 → V → K/2` (Strang) ordering, with the nonlinear `g·|ψ|²` term staying in the `V` step. `rustfft` **does not normalize** — an `1/n²` scale factor after the inverse transform is required.

**Measured evidence after the fix:**

| quantity | value | change |
|---|---|---|
| max \|Δ\| in the density distribution | **2.766** | **2.2e13×** |
| max \|Δ\| in the 16 mode energies | **8.444** | **4.3e14×** |
| max/mean | **46.99** | the fluid became spatially extended |
| coherence | **0.996203** | the indicator came alive |

The three regression tests in `crates/fluid-stratos/src/gpe.rs` **fail on the dead code** — they are the guardians of the fix.

### The hidden bug the fix exposed

The parallel solver (`gpe_parallel.rs`) carried the same defect. On top of that, `FluidStratos::tick()` copied only the **scalar** fields across after a parallel step — the wavefunction itself went stale. The 16 mode energies and the dominant frequency would therefore have stayed **static**, even with a live kinetic step. Fixed by `GpeSolver::sync_psi_from_parallel()`.

### A parallelism detail

`rayon::for_each` requires `Fn` (not `FnMut`), so a shared `&mut` scratch buffer **cannot** be used across threads. The solution is a thread-local scratch (`std::thread_local!` + `RefCell`) — `process_with_scratch` then allocates nothing.

---

## 2. The quantum bridge — what the hardware actually verified

The scheme measured on the hardware (2026-10-06, a real 156-qubit superconducting QPU):

| measurement | value |
|---|---|
| Bell-state fidelity (7 runs) | **87.7 – 99.3%** (typical ~94.9%) |
| GHZ-state fidelity | **89.3 – 96.7%** |
| QDS effective T₂ | **89.6 ± 39.2 µs** |
| QDS measurement window | **1024 shots** |

The encoding is **2 qubits + 3 separate measurements**:

| parameter | circuit | probability |
|---|---|---|
| `A` (amplitude) | 1 qubit, `RY(A)` | `P(q0=1) = sin²(A/2)` |
| `f` (frequency) | 2 qubits, `RY(2πf)` + `CNOT` | `P(q0=1) = sin²(πf)` |
| `φ` (phase) | 2 qubits, `RY(φ)` + `CNOT` | `P(q0=1) = sin²(φ/2)` |

Decoding: `A = 2·arccos(√P(q0=0))`, `φ = 2·arcsin(√P(q0=1))`, `f = arcsin(√P(q0=1))/π`.

**The `RZ` gate must not be used** for frequency or phase encoding: it is a global phase factor, which the Z-basis measurement cancels. `γ` is not a gate at all — it is a post-processing quantity.

### The measured decision the layer states out loud

Measuring the argmax over the 16 modes via the quantum path (1024 shots, Monte Carlo, with the measured Bell fidelities):

| | accuracy |
|---|---|
| **quantum path** (c = 0.949) | **38.65%** |
| **classical argmax** (deterministic) | **100%** |

The reason is in the measured data: the energy difference between Brain (0.09620959) and Creativity (0.09443425) is only **1.85%**, which at 1024 shots is **0.137σ** — the quantum measurement cannot decide which mode wins. Reaching 3σ would require 248 294 shots (242× the QDS measurement).

**The purpose of this layer is therefore not for the quantum path to win, but for the difference to be measurable and visible.**

---

## 3. The quantum–classical loop

The protocol: extract `(A, f, φ)` on the CPU → 3 quantum circuits, 10 shots → decode → load the mutated parameters back into the fluid → 50 GPE steps → measure again.

**Measured over 1000 iterations** (32×32 grid, 3 measurements × 10 shots, Bell c = 0.949):

| branch | H/Hmax over the 16 modes | note |
|---|---|---|
| **quantum loop** | **0.4355** | most frequent mode 71.4% (over the last 500 iterations) |
| classical control (no quantum noise) | 0.0000 | 1000/1000 in a single mode |
| uniform null model | 0.9962 | nearly flat |
| dead GPE (the pre-fix code) | 0.0000 | 1000/1000 in a single mode |

**What this says:** the quantum noise alone is **not** structured (it sits close to the uniform null model) — the **GPE supplies the attractor**, the quantum shot supplies the destabilizing kick. The dead GPE and the classical control give the same result: the fluid settles into a single mode, and without the quantum kick there is nothing to dislodge it.

**The mathematics of 10 shots** (φ = 1.0 → `P(q0=1) = 0.229849`):

| k | P | φ_decoded | error |
|---|---|---|---|
| 2 | 0.20 | 0.9273 | −0.0727 (the **most likely**, 0.2942) |
| 3 | 0.30 | 1.1593 | +0.1593 |
| 4 | 0.40 | 1.3694 | +0,3694 |

`E[φ] = 0.9451 rad`, `SD = 0.3812 rad` → relative spread **38.12%**.

**The decoding ranges are the hidden losses:** `φ ∈ [0,π]`, `f ∈ [0,0.5]`, `A ∈ [0,π]` — because of the 2π periodicity these cannot be fully inverted.

---

## 4. Limits — what this system does NOT prove

This is the most important section.

1. **There is no proof of emergent intelligence.** The measurements show that there *is* a mathematically coherent fluid attractor, and that a quantum kick produces structured motion. That does **not** prove self-organizing intelligence.
2. **The histogram test alone is a false positive.** The dead GPE would also produce an "asymmetric" result (a single attractor at 100%) — just not emergent, but static. A histogram test must always be accompanied by the classical control.
3. **Coherence is not a standalone metric.** The `tanh(max/mean/15)` formula saturates completely after normalization (max/mean 996–2726), so in the dead code it is always 1.0. The 0.9962 value in the fixed code is an artifact of normalization, not an independent state of the fluid.
4. **The 16-mode argmax is not detectable via the quantum path** with the measured hardware parameters (0.137σ). The quantum bridge works for measuring the wavepacket `(A, f, φ)`, not for a 16-way competition.

---

## 5. Usage

```bash
# the fluid solver: the regression tests
cargo test --manifest-path crates/fluid-stratos/Cargo.toml

# the quantum bridge: reproducing the measured accuracy
cargo test --manifest-path crates/quantum-bridge/Cargo.toml

# measuring the fluid's awakening (numpy, ~1 min)
python scripts/gpe_fft_measurement.py

# the quantum–classical loop with 4 branches (numpy, ~15 min)
python scripts/quantum_loop_sim.py

# the mathematics of 10 shots
python scripts/quantum_10shot_math.py
```

All measurements run with **deterministic seeds**, so the numbers are reproducible.

---

## 6. License and citation

MIT license. Citation metadata is in `CITATION.cff`.

The measured hardware parameters: 2026-10-06, a real 156-qubit superconducting QPU. Source of the GPE parameters: the Fluid STRATOS paper, Appendix A.2.
