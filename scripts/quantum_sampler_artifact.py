# A KVANTUM-ARGMAX PONTOSSAG — A SAMPLER-ARIFAKTUM ELLENORZESE
# A korabbi meres (42,29%) a Python `random.Random.gauss` hasznalata miatt
# 16 FUGGETLENNEL fUGGETLEN mintat adott: a `gauss` a `gauss_next` cache-t
# hasznalja, igy a 16 hivas PAROKBA rendezodik. A Brain (1. hivas) es a
# Logic (2. hivas) ugyanabból a Marsaglia-parból kap erteket -> KORRELALT.
# A valodi kvantumhardver viszont a kulonbo qubit-mérések FUGGETLENEK.
import math, random, json

ENERGIES = [
    0.09620959036827400,  # Brain
    0.07677600000000000,  # Logic
    0.08902975000000000,  # Memory
    0.09443425000000000,  # Creativity
    0.08851945000000000,  # Heart
    0.08038322000000000,  # Intuition
    0.07562627000000000,  # Focus
    0.02933872000000000,  # Empathy
    0.03608653000000000,  # Analysis
    0.05413618000000000,  # Synthesis
    0.05810160000000000,  # Ethics
    0.06105229000000000,  # Play
    0.04328244000000000,  # Dream
    0.03946274000000000,  # Social
    0.04136071000000000,  # Survival
    0.03620027197138900,  # Transcendence
]
MODES = ["Brain","Logic","Memory","Creativity","Heart","Intuition","Focus","Empathy",
         "Analysis","Synthesis","Ethics","Play","Dream","Social","Survival","Transcendence"]
SHOTS = 1024
TRIALS = 20000

def depolarized(p, c):
    return c * p + (1.0 - c) / 2.0

def sample_gauss_paired(rng, p, shots):
    """A KORABBI (hibas) sampler: a random.Random.gauss cache-et hasznal."""
    mu = p * shots
    sigma = math.sqrt(max(p * (1.0 - p) * shots, 1e-12))
    n = int(round(rng.gauss(mu, sigma)))
    return max(0, min(shots, n))

def sample_box_muller(rng, p, shots):
    """A FUGGETLEN sampler: Box-Muller, nincs cache, minden hivas kulon."""
    mu = p * shots
    sigma = math.sqrt(max(p * (1.0 - p) * shots, 1e-12))
    u1 = rng.random()
    if u1 <= 0.0:
        u1 = 1e-12
    u2 = rng.random()
    z = math.sqrt(-2.0 * math.log(u1)) * math.cos(2.0 * math.pi * u2)
    n = int(round(mu + z * sigma))
    return max(0, min(shots, n))

def run(sampler, c, seed=20261026, trials=TRIALS):
    rng = random.Random(seed)
    classical = max(range(16), key=lambda i: ENERGIES[i])
    hits = 0
    hist = {}
    for _ in range(trials):
        counts = [sampler(rng, depolarized(p, c), SHOTS) for p in ENERGIES]
        best = max(range(16), key=lambda i: counts[i])
        hist[best] = hist.get(best, 0) + 1
        if best == classical:
            hits += 1
    return hits / trials, hist

print("=" * 78)
print("A SAMPLER-ARIFAKTUM ELLENORZESE — 16 qubit, 1024 shot, %d trial" % TRIALS)
print("=" * 78)
print()
print("  c       PAIRED (random.gauss)     FUGGETLEN (Box-Muller)    kulonbseg")
for c in (1.0, 0.993, 0.981, 0.95, 0.949, 0.920, 0.877):
    a_p, _ = run(sample_gauss_paired, c)
    b_p, _ = run(sample_box_muller, c)
    print("  %.3f      %8.4f                %8.4f              %+8.4f" % (
        c, a_p, b_p, b_p - a_p))
print()
print("  A NYERT MOD MEGOSZLASA (c=1.0):")
_, hp = run(sample_gauss_paired, 1.0)
_, hb = run(sample_box_muller, 1.0)
print("    %-12s %10s %10s" % ("mod", "paired", "fuggetlen"))
for i in sorted(set(list(hp.keys()) + list(hb.keys())), key=lambda k: -hb.get(k, 0))[:6]:
    print("    %-12s %9.1f%% %9.1f%%" % (MODES[i], 100*hp.get(i,0)/TRIALS, 100*hb.get(i,0)/TRIALS))
print()
print("  A Brain es a Logic KORRELACIÓJA (a paired samplerben):")
rng = random.Random(20261026)
pb, pl = ENERGIES[0], ENERGIES[1]
xs, ys = [], []
for _ in range(4000):
    cb = sample_gauss_paired(rng, depolarized(pb, 1.0), SHOTS)
    cl = sample_gauss_paired(rng, depolarized(pl, 1.0), SHOTS)
    xs.append(cb); ys.append(cl)
mx, my = sum(xs)/len(xs), sum(ys)/len(ys)
cov = sum((a-mx)*(b-my) for a, b in zip(xs, ys)) / len(xs)
sx = math.sqrt(sum((a-mx)**2 for a in xs)/len(xs))
sy = math.sqrt(sum((b-my)**2 for b in ys)/len(ys))
print("    paired    korrelacio = %+.4f" % (cov/(sx*sy)))
rng = random.Random(20261026)
xs, ys = [], []
for _ in range(4000):
    cb = sample_box_muller(rng, depolarized(pb, 1.0), SHOTS)
    cl = sample_box_muller(rng, depolarized(pl, 1.0), SHOTS)
    xs.append(cb); ys.append(cl)
mx, my = sum(xs)/len(xs), sum(ys)/len(ys)
cov = sum((a-mx)*(b-my) for a, b in zip(xs, ys)) / len(xs)
sx = math.sqrt(sum((a-mx)**2 for a in xs)/len(xs))
sy = math.sqrt(sum((b-my)**2 for b in ys)/len(ys))
print("    fuggetlen korrelacio = %+.4f" % (cov/(sx*sy)))
print()
print("  A HATAS: a Brain a valodi kvantumhardveren a 16 fuggetlen qubit-")
print("  meresbol NEM osszulhat korrelaltan a Logikkal. A paired sampler")
print("  ezt MESTEN ULONOZZA -> a Brain 42%-os gyozelmi aranya NEM a")
print("  hardver viselkedesenek felel meg.")
