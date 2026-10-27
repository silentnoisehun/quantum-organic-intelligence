# Quantum Organic Intelligence

**A Gross-Pitaevskii folyadék, a hardver-igazolt kvantumhíd, és a mérés, amely eldönti, hogy ebből valami emergens van-e.**

Ez a repó három dolgot tartalmaz, és mindhárom mérhető:

| | |
|---|---|
| **`crates/fluid-stratos`** | A 16 kognitív módra képezett GPE-folyadékmotor, **valódi split-step Fourier** kinetikus lépéssel |
| **`crates/quantum-bridge`** | A hardver-igazolt kvantumhíd: **2 qubit + 3 külön mérés** (A, f, φ) — a hardverben mért séma |
| **`docs/`** | A mérési jelentések: **mért számok, módszertan, és a korlátok** |

Nincs itt marketing. Minden állítás mögött mérési adat van, és ami nem bizonyítható, azt a **Korlátok** fejezet kimondja.

---

## 1. A Fluid STRATOS — mit javítottunk és miért

A folyadékmotor kinetikus lépése **fázis-rotációval** közelítette a kinetikus operátort — a rácsindex-térben, transzformáció nélkül. Ez matematikailag halott megoldás: minden lépés-művelet vagy fázis-szorzás (nem változtatja a modult) vagy uniform skálázás (nem változtatja a rációt) volt, így a sűrűség-eloszlás **változatlan maradt**.

**Mért bizonyíték a hiba előtt** (64×64 rács, 500 lépés, a `gpe.rs` paraméterei verbatim):

| mennyiség | érték |
|---|---|
| max \|Δ\| a sűrűség-eloszlásban | **1,275e-13** |
| max \|Δ\| a 16 mód energiájában | **1,954e-14** |
| koherencia | **mindig 1,000000** (a `tanh` teljesen szaturált) |
| max/mean | **1112,84** — a folyadék egyetlen pontba koncentrált |

**A javítás:** valódi 2D FFT (sor- **és** oszlop-transzformáció, `rustfft` 6.4.1), a `K/2 → V → K/2` (Strang) sorrend megtartva, a nemlineáris `g·|ψ|²` tag a `V` lépésben. A `rustfft` **nem normalizál** — az inverse transzformáció után `1/n²` skálázás szükséges.

**Mért bizonyíték a javítás után:**

| mennyiség | érték | változás |
|---|---|---|
| max \|Δ\| a sűrűség-eloszlásban | **2,766** | **2,2e13×** |
| max \|Δ\| a 16 mód energiájában | **8,444** | **4,3e14×** |
| max/mean | **46,99** | a folyadék kiterjedt |
| koherencia | **0,996203** | a jelző életre kelt |

A három regressziós teszt a `crates/fluid-stratos/src/gpe.rs` fájlban **a halott kódon elbukik** — ezek a javítás őrzői.

### A rejtett hiba, amit a javítás felszínre hozott

A párhuzamos solver (`gpe_parallel.rs`) ugyanazt a hibát tartalmazta, és ráadásul a `FluidStratos` `tick()` a párhuzamos lépések után **csak a skaláris mezőket** másolta át — a hullámfüggvény elavult maradt. Emiatt a 16 mód energia és a domináns frekvencia **statikus** lett volna, akkor is, ha a kinetikus lépés már él. A javítás: `GpeSolver::sync_psi_from_parallel()`.

### A párhuzamosság technikai része

A `rayon::for_each` `Fn`-t vár (nem `FnMut`-et), ezért egy közös `&mut` scratch buffer **nem osztható meg** a szálak között. A megoldás thread-local scratch (`std::thread_local!` + `RefCell`) — a `process_with_scratch` így sem allokál.

---

## 2. A kvantumhíd — amit a hardver valóban igazolt

A hardverben mért séma (2026.10.06, valódi 156 qubit szupravezető QPU):

| mérés | érték |
|---|---|
| Bell-állapot egyensúly (7 futás) | **87,7 – 99,3%** (jellemző ~94,9%) |
| GHZ-állapot egyensúly | **89,3 – 96,7%** |
| QDS effektív T₂ | **89,6 ± 39,2 µs** |
| QDS mérési ablak | **1024 shot** |

A kódolás **2 qubit + 3 külön mérés**:

| paraméter | áramkör | valószínűség |
|---|---|---|
| `A` (intenzitás) | 1 qubit, `RY(A)` | `P(q0=1) = sin²(A/2)` |
| `f` (frekvencia) | 2 qubit, `RY(2πf)` + `CNOT` | `P(q0=1) = sin²(πf)` |
| `φ` (fázis) | 2 qubit, `RY(φ)` + `CNOT` | `P(q0=1) = sin²(φ/2)` |

A visszafejtés: `A = 2·arccos(√P(q0=0))`, `φ = 2·arcsin(√P(q0=1))`, `f = arcsin(√P(q0=1))/π`.

**A `RZ` kapu tilos** a frekvencia- és a fáziskódolásra: globális fázisszorzó, amit a Z-mérés kitöröl. A `γ` nem kapu, hanem posztprocesszálási mennyiség.

### A mért döntés, amit a réteg kimond

A 16 mód argmaxát kvantum-úton mérve (1024 shot, Monte Carlo, a mért Bell-egyensúlyokkal):

| | pontosság |
|---|---|
| **kvantum-út** (c = 0,949) | **38,65%** |
| **klasszikus argmax** (determinista) | **100%** |

Az ok a mért adatokban van: a Brain (0,09620959) és a Creativity (0,09443425) mód energiájának különbsége mindössze **1,85%**, ami 1024 lövés mellett **0,137σ** — a kvantum-mérés nem tudja eldönteni, melyik a nyertes. 3σ-hoz 248 294 shot kellene (242× a QDS-mérés).

**A réteg feladata ezért nem az, hogy a kvantum-út nyerjen, hanem hogy a különbség mérhető és látható legyen.**

---

## 3. A kvantum–klasszikus hurok

A protokoll: a CPU-ről kinyert `(A, f, φ)` → 3 kvantumáramkör, 10 lövés → visszafejtés → a mutált paraméterek visszatöltése a folyadékba → 50 GPE-lépés → újra mérés.

**Mérve, 1000 iteráció** (32×32 rács, 3 mérés × 10 lövés, Bell c = 0,949):

| ág | H/Hmax a 16 módon | megjegyzés |
|---|---|---|
| **kvantum-hurok** | **0,4355** | a leggyakoribb mód 71,4% (az utolsó 500 iterációban) |
| klasszikus kontroll (kvantumzaj nélkül) | 0,0000 | 1000/1000 egyetlen módban |
| uniform null-modell | 0,9962 | majdnem lapos |
| dead GPE (a javítás előtti kód) | 0,0000 | 1000/1000 egyetlen módban |

**Amit ez mond:** a kvantumzaj önmagában **nem strukturált** (az uniform null-modellhez közeli) — a **GPE adja az attraktort**, a kvantumzaj a destabilizáló lökést. A dead GPE és a klasszikus kontroll azonos eredményt ad: a folyadék egyetlen módban áll meg, és a kvantumlökés nélkül nincs semmi, ami kimozdítaná.

**A 10 lövés matematikája** (φ = 1,0 → `P(q0=1) = 0,229849`):

| k | P | φ_vissza | hiba |
|---|---|---|---|
| 2 | 0,20 | 0,9273 | −0,0727 (a **legvalószínűbb**, 0,2942) |
| 3 | 0,30 | 1,1593 | +0,1593 |
| 4 | 0,40 | 1,3694 | +0,3694 |

`E[φ] = 0,9451 rad`, `SD = 0,3812 rad` → a relatív szórás **38,12%**.

**A dekódolási tartományok a rejtett veszteségek:** `φ ∈ [0,π]`, `f ∈ [0,0,5]`, `A ∈ [0,π]` — a 2π-periodicitás miatt ezek nem invertálhatók teljesen.

---

## 4. Korlátok — amit ez a rendszer NEM bizonyít

Ez a fejezet a legfontosabb.

1. **Nincs bizonyíték emergens intelligenciára.** A mért eredmények azt mutatják, hogy *van* egy matematikailag koherens folyadék-attraktor, és hogy a kvantumlökés strukturált mozgást kelt. Ez **nem** bizonyít önszerveződő intelligenciát.
2. **A hisztogram-teszt önmagában fals pozitív.** A dead GPE is „aszimmetrikus" eredményt adna (egyetlen attraktor 100%-ban) — csak nem emergens, hanem statikus. A hisztogram-teszttel **mindig együtt** kell adni a klasszikus kontrollt.
3. **A koherencia nem önálló mérőszám.** A `tanh(max/mean/15)` képlet a normalizálás után teljesen szaturál (a max/mean 996–2726), ezért a dead kódban mindig 1,0. A 0,9962-es érték a javított kódban a normalizálás artefaktuma, nem a folyadék önálló állapota.
4. **A 16 mód argmax kvantum-úton nem kimutatható** a mért hardver-paraméterekkel (0,137σ). A kvantumhíd a hullámcsomag `(A, f, φ)` mérésére működik, nem a 16 módos versenyre.

---

## 5. A repó használata

```bash
# a fluid folyadékmotor: a regressziós tesztek
cargo test --manifest-path crates/fluid-stratos/Cargo.toml

# a kvantumhíd: a mért pontosság reprodukciója
cargo test --manifest-path crates/quantum-bridge/Cargo.toml

# a folyadék ébredésének mérése (numpy, ~1 perc)
python scripts/gpe_fft_measurement.py

# a kvantum-klasszikus hurok 4 ággal (numpy, ~15 perc)
python scripts/quantum_loop_sim.py

# a 10 lövés matematikája
python scripts/quantum_10shot_math.py
```

### A valós hardveres mérés

A kvantumhíd **valódi QPU-n** is futtatható. A token az `IBM_QUANTUM_API_TOKEN`
környezetváltozóban kell (a `scs_quantum` saját `.env` fájlját is betölti) —
**soha ne a parancssorban, soha ne fájlba írva a repóban**.

```bash
# a pipeline ellenőrzése ideális szimulátoron — token NEM kell,
# de ez NEM QPU-bizonyíték, csak a kód helyességének igazolása
python scripts/hardware_measure.py --local

# a valódi hardver: a Bell-fidelity, a 16 mód argmax, az (A, f, φ) dekódolás
python scripts/hardware_measure.py --backend ibm_brisbane --shots 1024 --mode-trials 20
```

A mérés három dolgot mér:
1. a **bit-sorrend-ellenőrzés** — a mérés előfeltétele; ha nem megy át, a mérés elutasításra kerül
2. a **hardver-minőséget** (a Bell-referencia egyensúlya) → ez a szimulációban használt `c`
3. a **16 mód argmax pontosságát** a klasszikus argmaxhoz képest, és az **(A, f, φ) dekódolás** pontosságát

A nyers mérési adatok a `measurement_raw/` mappába kerülnek (a job ID-vel, a
backenddel és az időbélyeggel) — ez **nincs verziókövetve**.

> **Környezet:** a méréshez tiszta virtuális környezet ajánlott. A Qiskit 1.x és
> 2.x nem él együtt ugyanabban a Pythonban:
> ```bash
> python -m venv .venv-quantum
> .venv-quantum/Scripts/pip install "qiskit>=2.0" "qiskit-ibm-runtime>=0.50" python-dotenv numpy
> .venv-quantum/Scripts/python scripts/hardware_measure.py --local
> ```
> A `scs_quantum` csomag telepített példány hiányában a `SCS_QUANTUM_PATH`
> környezetváltozó mutatja a `python/` mappát.

A mérések mind **determinisztikus magokkal** futnak, így a számok reprodukálhatók.

---

## 6. Licenc és hivatkozás

MIT licenc. A hivatkozási adatok a `CITATION.cff` fájlban.

A mért hardver-paraméterek: 2026.10.06, valódi 156 qubit szupravezető QPU. A GPE-paraméterek forrása: a Fluid STRATOS paper A.2 függeléke.
