#!/usr/bin/env python3
"""Parity script: same deterministic initial state as Rust, 200 steps.

Outputs JSON matching the Rust binary format.
"""
import json
import numpy as np
import math

# Same constants as Rust
N = 32
DOMAIN = 10.0
DT = 0.005
G = -1.0
TRAP = 0.1
DAMPING = 0.002
HBAR = 1.0
MASS = 1.0
WELL_DEPTH = 2.0
WELL_SIGMA = 3.0

DX = DOMAIN / N
center = N / 2.0
sigma = N / 6.0

# Modes for mode energies
center = N / 2.0
radius = N / 4.0
MODES = [(center, center)]
for k in range(6):
    a = 2 * math.pi * k / 6
    MODES.append((center + radius * 0.5 * math.cos(a), center + radius * 0.5 * math.sin(a)))
for k in range(9):
    a = 2 * math.pi * k / 9
    MODES.append((center + radius * math.cos(a), center + radius * math.sin(a)))

# Potential
pot = np.zeros((N, N))
for i in range(N):
    for j in range(N):
        dx_ = (i - center) / center
        dy_ = (j - center) / center
        v = 0.5 * TRAP * TRAP * (dx_ * dx_ + dy_ * dy_)
        for (mx, my) in MODES:
            d2 = (i - mx) ** 2 + (j - my) ** 2
            v += -WELL_DEPTH * math.exp(-d2 / (2.0 * WELL_SIGMA ** 2))
        pot[i, j] = v

# K-space
k = np.fft.fftfreq(N, d=DX) * 2 * np.pi
k2 = k[:, None] ** 2 + k[None, :] ** 2
KPH = np.exp(-1j * DT * HBAR * k2 / (4.0 * MASS))

def normalize(psi):
    ns = (np.abs(psi) ** 2).sum() * DX * DX
    if ns < 1e-10:
        return psi
    return psi / math.sqrt(ns)

def init_psi(seed):
    r = np.random.default_rng(seed)
    i = np.arange(N)[:, None]
    j = np.arange(N)[None, :]
    x = (i - center) / center * DOMAIN / 2.0
    y = (j - center) / center * DOMAIN / 2.0
    gauss = np.exp(-(x * x + y * y) / (2.0 * sigma * sigma / (center * center)))
    noise = (r.uniform(-0.1, 0.1, (N, N)) + 1j * r.uniform(-0.1, 0.1, (N, N)))
    return normalize(noise * gauss)

def step(psi, live=True):
    if live:
        psi = np.fft.ifft2(np.fft.fft2(psi) * KPH)
    else:
        psi = psi * np.exp(-1j * DT * HBAR * k2 / (4.0 * MASS))
    dens = np.abs(psi) ** 2
    v = pot + G * dens
    psi = psi * np.exp(-1j * DT * v / HBAR) * np.exp(-DAMPING * DT * dens)
    if live:
        psi = np.fft.ifft2(np.fft.fft2(psi) * KPH)
    else:
        psi = psi * np.exp(-1j * DT * HBAR * k2 / (4.0 * MASS))
    return normalize(psi)

def mode_energies(psi):
    dens = np.abs(psi) ** 2
    out = np.zeros(16)
    i = np.arange(N)[:, None]
    j = np.arange(N)[None, :]
    for m, (px, py) in enumerate(MODES):
        d = np.hypot(i - px, j - py)
        mask = d < 5.0
        w = np.exp(-(d * d) / (2.0 * 9.0)) * mask
        out[m] = (dens * w).sum()
    return out

# Run
psi = init_psi(20261026)
e0 = mode_energies(psi)
rho0 = np.abs(psi) ** 2

for _ in range(200):
    psi = step(psi, live=True)

e1 = mode_energies(psi)
rho1 = np.abs(psi) ** 2

max_delta_density = float(np.abs(rho1 - rho0).max())
max_delta_mode_energy = float(np.abs(e1 - e0).max())
argmax_initial = int(np.argmax(e0))
argmax_final = int(np.argmax(e1))

out = {
    "max_delta_density": max_delta_density,
    "max_delta_mode_energy": max_delta_mode_energy,
    "argmax_initial": argmax_initial,
    "argmax_final": argmax_final,
    "mode_energy_initial": e0.tolist(),
    "mode_energy_final": e1.tolist(),
}

print(json.dumps(out, indent=1))