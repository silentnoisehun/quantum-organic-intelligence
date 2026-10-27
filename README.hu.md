# Quantum Organic Intelligence

A Gross-Pitaevskii folyadék, a hardver-igazolt quantumhíd és a mérés, amely dokumentálja a rendszer dinamikai viselkedését. Ez a repó három mérhető komponensen alapul:

* `crates/fluid-stratos`: A 16 kognitív módra képezett GPE-folyadékmotor, valódi split-step Fourier kinetikus lépéssel.
* `crates/quantum-bridge`: A hardver-igazolt quantumhíd: 2 qubit + 3 külön mérés ($A, f, \varphi$).
* `docs/`: Mérési jelentések, módszertan és korlátok.

---

## 1. Architektúra és Módszertan

### A Fluid STRATOS (GPE Folyadékmotor)

A motor a 16 kognitív módot egy két dimenziós Gross-Pitaevskii mezőn valósítja meg, valódi split-step Fourier transzformációs kinetikus lépéssel (`rustfft 6.4.1`) és Strang-féle időléptetéssel.

### A Quantumhíd

A kódolás a valódi $156$ qubites szupravezető QPU mérésein alapul:

* **Kódolási séma (2 qubit + 3 külön mérés):**
  * $A$ (intenzitás): 1 qubit, $\text{RY}(A) \to P(q_0=1) = \sin^2(A/2)$
  * $f$ (frekvencia): 2 qubit, $\text{RY}(2\pi f) + \text{CNOT} \to P(q_0=1) = \sin^2(\pi f)$
  * $\varphi$ (fázis): 2 qubit, $\text{RY}(\varphi) + \text{CNOT} \to P(q_0=1) = \sin^2(\varphi/2)$

* **Visszafejtés:**
  * $A = 2 \cdot \arccos(\sqrt{P(q_0=0)})$
  * $\varphi = 2 \cdot \arcsin(\sqrt{P(q_0=1)})$
  * $f = \arcsin(\sqrt{P(q_0=1)}) / \pi$

---

## 2. A Kvantum–Klasszikus Hurok Szimulációja

A szimulációs protokoll során a CPU-ról kinyert paramétereket $(A, f, \varphi)$ beküldjük a 3 kvantumáramkörbe (10 lövés/mérés), visszafejtjük, majd a mutált értékeket visszatöltjük a folyadékba 50 GPE-lépés erejéig.

---

## 3. Elvárások és Korlátok

* **A perturbáció szerepe:** A kvantumzaj önmagában nem strukturált (közel áll az uniform null-modellhez). A GPE szolgáltatja az attraktort, míg a kvantumzaj a stabilizáló vagy destabilizáló perturbációt.
* **Korlátok:**
  1. A mért adatok egy koherens folyadék-attraktor létezését és a kvantumlökés által generált strukturált mozgást igazolják, de ez nem jelenti önmagában önszerveződő intelligencia meglétét.
  2. A hisztogram-tesztet minden esetben össze kell vetni a klasszikus kontrollal a hamis pozitív eredmények elkerülése érdekében.
  3. A kvantumhíd a hullámcsomag $(A, f, \varphi)$ mérésére alkalmas; a 16 mód közötti verseny kvantum-úton történő eldöntése a jelenlegi hardver-paraméterek mellett nem mutatható ki.

### Mi validálja a teljes állítást

A protokoll 1000 iteráció. A teljes állítást **a mérés végigfutásával** lehet
validálni: a teljes, 1000 iterációs futással, a klasszikus kontrollal együtt,
hogy a hisztogram-tesztnek legyen elegendő statisztikai ereje. A `docs/`
mögötti futások ennél rövidebbek, a
`docs/MEASUREMENTS-HARDWARE-20261027.txt` 6. szakaszában mért okok miatt: az
open csomag 600 s-os kvótája és a Runtime Session hiánya.

---

## 4. Használat

A tesztek és szimulációk futtatása a repón belül:

```bash
# A fluid folyadékmotor regressziós tesztjei
cargo test --manifest-path crates/fluid-stratos/Cargo.toml

# A quantumhíd tesztjei
cargo test --manifest-path crates/quantum-bridge/Cargo.toml

# A folyadék ébredésének mérése (numpy, ~1 perc)
python scripts/gpe_fft_measurement.py

# A kvantum-klasszikus hurok 4 ággal (numpy, ~15 perc)
python scripts/quantum_loop_sim.py

# A 10 lövés matematikájának ellenőrzése
python scripts/quantum_10shot_math.py
```

### A valós hardveres mérés

A quantumhíd valódi QPU-n is futtatható. A token az `IBM_QUANTUM_API_TOKEN`
környezetváltozóban kell (a `scs_quantum` saját `.env` fájlját is betölti).

```bash
# A pipeline ellenőrzése ideális szimulátoron — token NEM kell,
# de ez NEM QPU-bizonyíték, csak a kód helyességének igazolása
python scripts/hardware_measure.py --local

# A valódi hardver: a Bell-fidelity, a 16 mód argmax, az (A, f, φ) dekódolás
python scripts/hardware_measure.py --backend ibm_brisbane --shots 1024 --mode-trials 20
```

A mérés három dolgot mér:

1. a **bit-sorrend-ellenőrzés** — a mérés előfeltétele; ha nem megy át, a mérés elutasításra kerül
2. a **hardver-minőséget** (a Bell-referencia egyensúlya) — ez a `c`, amit a szimuláció használ
3. a **16 mód argmax pontosságát** a klasszikus argmaxhoz képest, és az **$(A, f, \varphi)$ dekódolás** pontosságát

A nyers mérési adatok a `measurement_raw/` mappába kerülnek (a job ID-vel, a
backenddel és az időbélyeggel) — ez **verziókövetett**, így a `docs/`-ban
idézett minden szám visszakövethető a mérési fájljához.

### A kvantum–klasszikus hurok valódi hardveren

A `scripts/quantum_loop_hardware.py` a hurkot futtatja egy QPU-n. Az iteráció
három áramkörét (A, f, φ) egy jobba kötegeli, minden N. iterációban
Bell-anchort ad hozzá, és a mért hardver-minőséget (`c`) visszaadja a
dekódolóba a Rust bridge-en keresztül
(`crates/quantum-bridge/src/bin/bridge-cli.rs`) — így a dekódolás
matematikája pontosan egy helyen van.

```bash
# A hurok: 100 iteráció, 1024 lövés, az anchor minden 5. iterációban
python scripts/quantum_loop_hardware.py --backend ibm_fez --iterations 100 --shots 1024 --anchor-every 5
```

> **Az open csomagban nincs session.** Az IBM megtagadja a Runtime Session-t az
> open csomagon ("not authorized to run a session when using the open plan"),
> így minden iteráció önálló jobként áll sorba. A szkript ezért a
> sor-várakozást **méri**, nem feltételezi, és a QPU-mp-eket meg a falióra-időt
> külön kiírja. A fenntartott ablakhoz másik plan instance kell.

> **A hardveres futás mérési jelentése** —
> `docs/MEASUREMENTS-HARDWARE-20261027.txt`, a futásnaplókkal együtt
> (`docs/RUNLOG-*.txt`). Minden szám a `measurement_raw/` nyers fájljaiból
> újraszámítható a `scripts/summarize_hardware_run.py` szkripttel.

> **Környezet:** a méréshez tiszta virtuális környezet ajánlott. A Qiskit 1.x és
> 2.x nem él együtt ugyanabban a Pythonban:
> ```bash
> python -m venv .venv-quantum
> .venv-quantum/Scripts/pip install "qiskit>=2.0" "qiskit-ibm-runtime>=0.50" python-dotenv numpy
> .venv-quantum/Scripts/python scripts/hardware_measure.py --local
> ```
> A `scs_quantum` csomag telepített példány hiányában a `SCS_QUANTUM_PATH`
> környezetváltozó mutatja a `python/` mappát.

---

## Nyílt Kutatás

Ez a munka nyitott a együttműködésre, szakmai ellenőrzésre és független validációra.
Ha bármelyik alábbi területen dolgozol, szívesen beszélek veled:

* **Quantum error mitigation** — az aszimmetrikus mérésizaj és a `c`-indikátor megközelítés
* **Neuromorph memóriamodellek** — a rekonzolidációs dinamika és a 16-módos folyadék-attraktor
* **Agentikus AI architektúrák** — a kognitív gradiens tervezés és a kvantum-klasszikus visszacsatolási hurok
* **Cross-validáció** — különösen a rekonzolidációs modell biológiai plauszibilitására
* **Előadások** — a pószt-akadémiai tudományról és az ipar-to-kutató útról

### Kapcsolat

**ORCID:** 0009-0003-3986-6039  
**Email:** materobi86@gmail.com  
**Sponsor:** github.com/sponsors/silentnoisehun  
**GitHub:** github.com/silentnoisehun/quantum-organic-intelligence

---

## 5. Licenc

MIT licenc. A hivatkozási adatok a `CITATION.cff` fájlban találhatók.