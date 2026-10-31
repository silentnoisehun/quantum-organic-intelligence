# A GPE KINETIKUS LEPES JAVITASANAK MERESE
# dead = a gpe.rs FAZIS-ROTACIOJA (index-térben, transzformacio nelkul)
# live = a JAVITOTT split-step Fourier (valodi 2D FFT)
# Forras: gpe.rs:228-283 (dead) + a javított kinetic_half_step (live)
import numpy as np, math, json

N = 64
DOMAIN = 20.0
DT = 0.005
G = -1.0
TRAP = 0.1
DAMPING = 0.002
HBAR = 1.0
MASS = 1.0
WELL_DEPTH = 2.0
WELL_SIGMA = 3.0
STEPS = 500

def genome_positions(n):
    center = n/2.0; radius = n/4.0
    pos = [(center, center)]
    for k in range(6):
        a = 2*math.pi*k/6
        pos.append((center+radius*0.5*math.cos(a), center+radius*0.5*math.sin(a)))
    for k in range(9):
        a = 2*math.pi*k/9
        pos.append((center+radius*math.cos(a), center+radius*math.sin(a)))
    return pos

MODES = genome_positions(N)

def create_potential(n):
    pot = np.zeros((n, n)); center = n/2.0
    for i in range(n):
        for j in range(n):
            dx = (i-center)/center; dy = (j-center)/center
            v = 0.5*TRAP*TRAP*(dx*dx+dy*dy)
            for (mx, my) in MODES:
                d2 = (i-mx)**2 + (j-my)**2
                v += -WELL_DEPTH*math.exp(-d2/(2.0*WELL_SIGMA**2))
            pot[i, j] = v
    return pot

K1 = np.array([i*(2*math.pi/DOMAIN) if i <= N//2 else (i-N)*(2*math.pi/DOMAIN) for i in range(N)])
KX, KY = np.meshgrid(K1, K1, indexing="ij")
K2 = KX**2 + KY**2
KPH = np.exp(-1j*DT*HBAR*K2/(4.0*MASS))
POT = create_potential(N)
DX = DOMAIN/N
DECAY = math.exp(-DAMPING*DT)

def normalize(psi):
    ns = (np.abs(psi)**2).sum()*DX*DX
    return psi/math.sqrt(ns) if ns > 1e-10 else psi

def init_psi(seed):
    r = np.random.default_rng(seed)
    c = N/2.0; sigma = N/6.0
    i = np.arange(N)[:, None]; j = np.arange(N)[None, :]
    x = (i-c)/c*DOMAIN/2.0; y = (j-c)/c*DOMAIN/2.0
    gauss = np.exp(-(x*x+y*y)/(2.0*sigma*sigma/(c*c)))
    return normalize((r.uniform(-0.1,0.1,(N,N)) + 1j*r.uniform(-0.1,0.1,(N,N)))*gauss)

def step(psi, live):
    if live:
        psi = np.fft.ifft2(np.fft.fft2(psi) * KPH)
    else:
        psi = psi*np.exp(-1j*DT*HBAR*K2/(4.0*MASS))
    dens = np.abs(psi)**2
    psi = psi*np.exp(-1j*DT*(POT+G*dens)/HBAR) * np.exp(-DAMPING * DT * dens)
    if live:
        psi = np.fft.ifft2(np.fft.fft2(psi) * KPH)
    else:
        psi = psi*np.exp(-1j*DT*HBAR*K2/(4.0*MASS))
    return normalize(psi)

def mode_energies(psi):
    dens = np.abs(psi)**2
    out = np.zeros(16)
    i = np.arange(N)[:, None]; j = np.arange(N)[None, :]
    for m, (px, py) in enumerate(MODES):
        d = np.hypot(i-px, j-py); mask = d < 5.0
        w = np.exp(-(d*d)/(2.0*9.0))*mask
        out[m] = (dens*w).sum()
    return out

def coherence(psi):
    dens = np.abs(psi)**2
    mx = dens.max(); mean = dens.mean()
    return math.tanh((mx/mean)/15.0) if mean > 1e-10 else 0.0

def run(live, seed=20261026, steps=STEPS):
    psi = init_psi(seed)
    d0 = (np.abs(psi)**2).copy()
    e0 = mode_energies(psi)
    c0 = coherence(psi)
    m0 = (np.abs(psi)**2).max()/(np.abs(psi)**2).mean()
    for _ in range(steps):
        psi = step(psi, live)
    d1 = (np.abs(psi)**2)
    e1 = mode_energies(psi)
    return {
        "max_delta_density": float(np.abs(d1-d0).max()),
        "max_delta_mode_energy": float(np.abs(e1-e0).max()),
        "coherence_initial": c0, "coherence_final": coherence(psi),
        "max_over_mean_initial": float(m0),
        "max_over_mean_final": float((np.abs(psi)**2).max()/(np.abs(psi)**2).mean()),
        "argmax_initial": int(np.argmax(e0)), "argmax_final": int(np.argmax(e1)),
        "mode_energy_initial": e0.tolist(), "mode_energy_final": e1.tolist(),
    }

print("="*76)
print("A KINETIKUS LEPES JAVITASA — %dx%d, %d lepes" % (N, N, STEPS))
print("="*76)
res = {}
for label, live in (("DEAD (fazis-rotacio, index-tér)", False), ("LIVE (valodi 2D FFT)", True)):
    r = run(live)
    res[label] = r
    print()
    print("--- %s ---" % label)
    print("  max |Δ| suruseg-eloszlas : %.6e" % r["max_delta_density"])
    print("  max |Δ| 16 mód energia  : %.6e" % r["max_delta_mode_energy"])
    print("  koherencia  kezdet/vég  : %.6f / %.6f" % (r["coherence_initial"], r["coherence_final"]))
    print("  max/mean    kezdet/vég  : %.2f / %.2f" % (r["max_over_mean_initial"], r["max_over_mean_final"]))
    print("  argmax mód  kezdet/vég  : %d / %d" % (r["argmax_initial"], r["argmax_final"]))

print()
print("="*76)
print("EREDMENY")
print("="*76)
d = res["DEAD (fazis-rotacio, index-tér)"]
l = res["LIVE (valodi 2D FFT)"]
print("  a suruseg-eloszlas mozgása:  dead %.3e  ->  live %.3e   (javulas: %.1e x)" % (
    d["max_delta_density"], l["max_delta_density"], l["max_delta_density"]/max(d["max_delta_density"],1e-30)))
print("  a 16 mód energia mozgása:  dead %.3e  ->  live %.3e   (javulas: %.1e x)" % (
    d["max_delta_mode_energy"], l["max_delta_mode_energy"], l["max_delta_mode_energy"]/max(d["max_delta_mode_energy"],1e-30)))
with open("gpe-fft-fix-measurement.json","w") as fh:
    json.dump(res, fh, indent=1)
print("  kiirva: gpe-fft-fix-measurement.json")
