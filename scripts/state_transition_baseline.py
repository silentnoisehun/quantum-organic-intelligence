# A VALOS ATMENETI MATRIX — baseline a kvantumhurokhoz
# Forras: a rezonancia log 26 allapot-sorozata (resonance-log-verification-fixed.py:35-38),
# amit a video 1 fps-enkent auditalt (26/26 kek-dominans, 20/26 pontszam >= 0.75).
import json, math

S = ["MAG","TORLES","KERET","TORLES","D_FORMA","KLASZTER","D_FORMA","KLASZTER","KERET",
     "MAG","HASADAS","MAG","HASADAS","MAG","HASADAS","MAG",
     "TORLES","KERET","TORLES","D_FORMA","KLASZTER","D_FORMA","KLASZTER","KERET","MAG","KERET"]

states = sorted(set(S))
idx = {s:i for i,s in enumerate(states)}
N = len(states)
M = [[0]*N for _ in range(N)]

for a,b in zip(S, S[1:]):
    M[idx[a]][idx[b]] += 1

row = [sum(r) for r in M]
print("allapotok (%d): %s" % (N, states))
print("atmenetek: %d" % sum(row))
print()
print("NORMALIZALT ATMENETI MATRIX (P(i->j) | i)")
for i,s in enumerate(states):
    if row[i] == 0: continue
    parts = ["%s=%.2f" % (states[j], M[i][j]/row[i]) for j in range(N) if M[i][j]]
    print("  %-8s -> %s" % (s, "  ".join(parts)))

# Latsasag-eloszlas: minden eltolasi iranyban, hogy ne csak a szomszedos legyen
print()
print("LATASAGO-ELOSZLAS (atmenet 1 vagy 2 pozicioval levo allapot)")
for w in (1,2,3):
    M2 = [[0]*N for _ in range(N)]
    for k in range(len(S)-1):
        a,b = S[k], S[k+1]
        if w==1: M2[idx[a]][idx[b]] += 1
        else:
            for d in (1,2):
                if k+d < len(S): M2[idx[a]][idx[S[k+d]]] += 1
    r2 = [sum(r) for r in M2]
    tot = sum(r2)
    H = 0.0
    for i in range(N):
        if r2[i]==0: continue
        p = r2[i]/tot
        H -= p*math.log(p)
    Hmax = math.log(N)
    print("  window=%d  entr=%0.4f / max %0.4f  -> H/Hmax = %.3f" % (w, H, Hmax, H/Hmax))

# Marginális gyakorisag (hany ciklusban jott elo)
print()
tot = sum(row)
print("MARGINALIS GYAKORISAG (nyertes allapot osszegyult gyakorisaga)")
for i,s in sorted(enumerate(states), key=lambda kv:-row[kv[0]]):
    print("  %-8s %2d  %5.1f%%" % (s, row[i], 100*row[i]/tot))

# Ismetlodes: a 26 allapot ket ciklust alkot?
print()
print("CIKLUS-STRUKTURA:")
print("  0..8   :", " ".join(S[0:9]))
print("  9..15  :", " ".join(S[9:16]))
print("  16..23 :", " ".join(S[16:24]))
print("  24..25 :", " ".join(S[24:26]))
