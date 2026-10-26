# A 10-LOBES PROTOKOLL MATEMATIKAI ELLENORZESE
# Minden szamolo: a scs-quantum/rust/src/psi_quantum.rs decode_formulajaival.
import math
from math import comb, sin, cos, pi, sqrt, asin, acos, log, exp

SHOTS = 10
PHI_TRUE = 1.0

def decode_phase(p):        # psi_quantum.rs:229-232
    return 2.0 * asin(sqrt(min(max(p, 0.0), 1.0)))

def decode_amplitude(p0):   # psi_quantum.rs:223-226
    return 2.0 * acos(sqrt(min(max(p0, 0.0), 1.0)))

def decode_frequency(p1):   # psi_quantum.rs:242-245
    return asin(sqrt(min(max(p1, 0.0), 1.0))) / pi

print("=" * 74)
print("1. AZ IDEALIS FIZIKA: P(q0=1) = sin^2(phi/2)")
print("=" * 74)
p_ideal = sin(PHI_TRUE / 2.0) ** 2
print("  phi = %.3f  ->  P(q0=1) = sin^2(%.4f) = %.6f" % (PHI_TRUE, PHI_TRUE/2, p_ideal))
print()

print("=" * 74)
print("2. A 10 LOBES VISSZAFEJTETT FAZIS (Minden lehetséges k)")
print("=" * 74)
print("   k   P_mert    phi_uj       hiba      |hiba|")
phivals = []
for k in range(SHOTS + 1):
    p = k / SHOTS
    phi = decode_phase(p)
    phivals.append(phi)
    print("  %2d   %.2f    %8.4f   %+8.4f   %7.4f" % (k, p, phi, phi - PHI_TRUE, abs(phi - PHI_TRUE)))
print()

print("=" * 74)
print("3. A BINOMIALIS SULYOK (hardver-zaj nelkul)")
print("=" * 74)
weights = []
for k in range(SHOTS + 1):
    w = comb(SHOTS, k) * (p_ideal ** k) * ((1 - p_ideal) ** (SHOTS - k))
    weights.append(w)
for k, w in enumerate(weights):
    marker = "  <-- LEGVALOSZINUBB" if w == max(weights) else ""
    print("  k=%2d   P=%.4f   phi=%7.4f%s" % (k, w, phivals[k], marker))
print()
mean_phi = sum(w * v for w, v in zip(weights, phivals))
var_phi = sum(w * (v - mean_phi) ** 2 for w, v in zip(weights, phivals))
sd_phi = math.sqrt(var_phi)
print("  E[phi_uj]  = %.4f rad   (bekerult: %.4f)" % (mean_phi, PHI_TRUE))
print("  SD[phi_uj] = %.4f rad" % sd_phi)
print("  relativ hiba: E=%.2f%%   SD=%.2f%%" % (100*(mean_phi-PHI_TRUE)/PHI_TRUE, 100*sd_phi/PHI_TRUE))
print("  elso momentum hiba: %.2f%%" % (100*abs(mean_phi-PHI_TRUE)/PHI_TRUE))
print()

print("=" * 74)
print("4. A DEKODOLASI TARTOMANYOK - A KET REJTETT VESZTESEG")
print("=" * 74)
print("  decode_phase:    phi_uj mindig [0, pi]      = [%.3f, %.3f]" % (0.0, pi))
print("                   a 2*pi-periodicitas miatt a phi [pi, 2pi) NEM invertalható")
print("  decode_frequency: f mindig [0, 0.5]         -> a 2pi-periodikus f nem invertalható")
print("  decode_amplitude: A mindig [0, pi]          -> a valodi A>pi csonkolodik")
print()
# Az amplitude loss: ha a GPE egy A=2.0 erosseget ad
for A in (0.5, 1.0, 1.5, 2.0, 2.5):
    p_true = sin(A/2)**2
    k = round(p_true * SHOTS)
    A_back = decode_amplitude(1.0 - k/SHOTS)
    print("    A=%.1f -> P=%.3f -> k=%d -> A_vissza=%.3f  (veszteseg %+.3f)" % (A, p_true, k, A_back, A_back - A))
print()

print("=" * 74)
print("5. A HARDVER-FAJTA (Bell-egyensuly c) HATASA A FOLYAMATRA")
print("=" * 74)
print("  A hardver c-vel gyengul: p_eff = c*p_true + (1-c)*0.5")
print()
print("   c     p_eff(phi=1.0)   E[phi_uj]   SD      E hiba")
for c in (1.0, 0.993, 0.981, 0.968, 0.949, 0.930, 0.877):
    p_eff = c * p_ideal + (1 - c) * 0.5
    ws = [comb(SHOTS, k) * (p_eff ** k) * ((1 - p_eff) ** (SHOTS - k)) for k in range(SHOTS+1)]
    ps = [decode_phase(k/SHOTS) for k in range(SHOTS+1)]
    m = sum(w*v for w,v in zip(ws,ps))
    s = math.sqrt(sum(w*(v-m)**2 for w,v in zip(ws,ps)))
    print("  %.3f      %.4f          %7.4f   %7.4f  %+7.2f%%" % (c, p_eff, m, s, 100*(m-PHI_TRUE)/PHI_TRUE))
print()
print("  A c=0.930 es c=0.949 a 'jellemzo' futasok (87,7-99,3% tartomany)")
print()

print("=" * 74)
print("6. A PROTOKOLL KOLTSEGA")
print("=" * 74)
print("  1000 iteracio x 3 meres (A, f, phi) = 3000 QPU job")
print("  1000 iteracio x 3 x 10 shot       = 30 000 loves")
print("  a QDS referencia: 1024 shot/job    -> a 10 shot/job 102x kisebb koltaju")
print("  de 3000 job 1024 shottal           = 3 072 000 loves")
print()
print("  A FONTOS KERDES: a 10-shot FELOJEBL a GPE 50-100 lepese alatt")
print("  visszaallitja-e a rendet? -> a GPE deltaert nem szimulalhato")
print("  a 16 mod ARGMAX-avaltozasa a kulcsmeres, nem a phi")
