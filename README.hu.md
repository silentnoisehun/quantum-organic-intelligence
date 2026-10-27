# Quantum Organic Intelligence

A Gross-Pitaevskii folyadĂ©k, a hardver-igazolt quantumhĂ­d Ă©s a mĂ©rĂ©s, amely dokumentĂˇlja a rendszer dinamikai viselkedĂ©sĂ©t. Ez a repĂł hĂˇrom mĂ©rhetĹ‘ komponensen alapul:

* `crates/fluid-stratos`: A 16 kognitĂ­v mĂłdra kĂ©pezett GPE-folyadĂ©kmotor, valĂłdi split-step Fourier kinetikus lĂ©pĂ©ssel.
* `crates/quantum-bridge`: A hardver-igazolt quantumhĂ­d: 2 qubit + 3 kĂĽlĂ¶n mĂ©rĂ©s ($A, f, \varphi$).
* `docs/`: MĂ©rĂ©si jelentĂ©sek, mĂłdszertan Ă©s korlĂˇtok.

---

## 1. ArchitektĂşra Ă©s MĂłdszertan

### A Fluid STRATOS (GPE FolyadĂ©kmotor)

A motor a 16 kognitĂ­v mĂłdot egy kĂ©t dimenziĂłs Gross-Pitaevskii mezĹ‘n valĂłsĂ­tja meg, valĂłdi split-step Fourier transzformĂˇciĂłs kinetikus lĂ©pĂ©ssel (`rustfft 6.4.1`) Ă©s Strang-fĂ©le idĹ‘lĂ©ptetĂ©ssel.

### A QuantumhĂ­d

A kĂłdolĂˇs a valĂłdi $156$ qubites szupravezetĹ‘ QPU mĂ©rĂ©sein alapul:

* **KĂłdolĂˇsi sĂ©ma (2 qubit + 3 kĂĽlĂ¶n mĂ©rĂ©s):**
  * $A$ (intenzitĂˇs): 1 qubit, $\text{RY}(A) \to P(q_0=1) = \sin^2(A/2)$
  * $f$ (frekvencia): 2 qubit, $\text{RY}(2\pi f) + \text{CNOT} \to P(q_0=1) = \sin^2(\pi f)$
  * $\varphi$ (fĂˇzis): 2 qubit, $\text{RY}(\varphi) + \text{CNOT} \to P(q_0=1) = \sin^2(\varphi/2)$

* **VisszafejtĂ©s:**
  * $A = 2 \cdot \arccos(\sqrt{P(q_0=0)})$
  * $\varphi = 2 \cdot \arcsin(\sqrt{P(q_0=1)})$
  * $f = \arcsin(\sqrt{P(q_0=1)}) / \pi$

---

## 2. A Kvantumâ€“Klasszikus Hurok SzimulĂˇciĂłja

A szimulĂˇciĂłs protokoll sorĂˇn a CPU-rĂłl kinyert paramĂ©tereket $(A, f, \varphi)$ bekĂĽldjĂĽk a 3 kvantumĂˇramkĂ¶rbe (10 lĂ¶vĂ©s/mĂ©rĂ©s), visszafejtjĂĽk, majd a mutĂˇlt Ă©rtĂ©keket visszatĂ¶ltjĂĽk a folyadĂ©kba 50 GPE-lĂ©pĂ©s erejĂ©ig.

---

## 3. ElvĂˇrĂˇsok Ă©s KorlĂˇtok

* **A perturbĂˇciĂł szerepe:** A kvantumzaj Ă¶nmagĂˇban nem strukturĂˇlt (kĂ¶zel Ăˇll az uniform null-modellhez). A GPE szolgĂˇltatja az attraktort, mĂ­g a kvantumzaj a stabilizĂˇlĂł vagy destabilizĂˇlĂł perturbĂˇciĂłt.
* **KorlĂˇtok:**
  1. A mĂ©rt adatok egy koherens folyadĂ©k-attraktor lĂ©tezĂ©sĂ©t Ă©s a kvantumlĂ¶kĂ©s Ăˇltal generĂˇlt strukturĂˇlt mozgĂˇst igazoljĂˇk, de ez nem jelenti Ă¶nmagĂˇban Ă¶nszervezĹ‘dĹ‘ intelligencia meglĂ©tĂ©t.
  2. A hisztogram-tesztet minden esetben Ă¶ssze kell vetni a klasszikus kontrollal a hamis pozitĂ­v eredmĂ©nyek elkerĂĽlĂ©se Ă©rdekĂ©ben.
  3. A kvantumhĂ­d a hullĂˇmcsomag $(A, f, \varphi)$ mĂ©rĂ©sĂ©re alkalmas; a 16 mĂłd kĂ¶zĂ¶tti verseny kvantum-Ăşton tĂ¶rtĂ©nĹ‘ eldĂ¶ntĂ©se a jelenlegi hardver-paramĂ©terek mellett nem mutathatĂł ki.

### Mi validĂˇlja a teljes ĂˇllĂ­tĂˇst

A protokoll 1000 iterĂˇciĂł. A teljes ĂˇllĂ­tĂˇst **a mĂ©rĂ©s vĂ©gigfutĂˇsĂˇval** lehet
validĂˇlni: a teljes, 1000 iterĂˇciĂłs futĂˇssal, a klasszikus kontrollal egyĂĽtt,
hogy a hisztogram-tesztnek legyen elegendĹ‘ statisztikai ereje. A `docs/`
mĂ¶gĂ¶tti futĂˇsok ennĂ©l rĂ¶videbbek, a
`docs/MEASUREMENTS-HARDWARE-20261027.txt` 6. szakaszĂˇban mĂ©rt okok miatt: az
open csomag 600 s-os kvĂłtĂˇja Ă©s a Runtime Session hiĂˇnya.

---

## 4. HasznĂˇlat

A tesztek Ă©s szimulĂˇciĂłk futtatĂˇsa a repĂłn belĂĽl:

```bash
# A fluid folyadĂ©kmotor regressziĂłs tesztjei
cargo test --manifest-path crates/fluid-stratos/Cargo.toml

# A quantumhĂ­d tesztjei
cargo test --manifest-path crates/quantum-bridge/Cargo.toml

# A folyadĂ©k Ă©bredĂ©sĂ©nek mĂ©rĂ©se (numpy, ~1 perc)
python scripts/gpe_fft_measurement.py

# A kvantum-klasszikus hurok 4 Ăˇggal (numpy, ~15 perc)
python scripts/quantum_loop_sim.py

# A 10 lĂ¶vĂ©s matematikĂˇjĂˇnak ellenĹ‘rzĂ©se
python scripts/quantum_10shot_math.py
```

### A valĂłs hardveres mĂ©rĂ©s

A quantumhĂ­d valĂłdi QPU-n is futtathatĂł. A token az `IBM_QUANTUM_API_TOKEN`
kĂ¶rnyezetvĂˇltozĂłban kell (a `scs_quantum` sajĂˇt `.env` fĂˇjljĂˇt is betĂ¶lti).

```bash
# A pipeline ellenĹ‘rzĂ©se ideĂˇlis szimulĂˇtoron â€” token NEM kell,
# de ez NEM QPU-bizonyĂ­tĂ©k, csak a kĂłd helyessĂ©gĂ©nek igazolĂˇsa
python scripts/hardware_measure.py --local

# A valĂłdi hardver: a Bell-fidelity, a 16 mĂłd argmax, az (A, f, Ď†) dekĂłdolĂˇs
python scripts/hardware_measure.py --backend ibm_brisbane --shots 1024 --mode-trials 20
```

A mĂ©rĂ©s hĂˇrom dolgot mĂ©r:

1. a **bit-sorrend-ellenĹ‘rzĂ©s** â€” a mĂ©rĂ©s elĹ‘feltĂ©tele; ha nem megy Ăˇt, a mĂ©rĂ©s elutasĂ­tĂˇsra kerĂĽl
2. a **hardver-minĹ‘sĂ©get** (a Bell-referencia egyensĂşlya) â€” ez a `c`, amit a szimulĂˇciĂł hasznĂˇl
3. a **16 mĂłd argmax pontossĂˇgĂˇt** a klasszikus argmaxhoz kĂ©pest, Ă©s az **$(A, f, \varphi)$ dekĂłdolĂˇs** pontossĂˇgĂˇt

A nyers mĂ©rĂ©si adatok a `measurement_raw/` mappĂˇba kerĂĽlnek (a job ID-vel, a
backenddel Ă©s az idĹ‘bĂ©lyeggel) â€” ez **verziĂłkĂ¶vetett**, Ă­gy a `docs/`-ban
idĂ©zett minden szĂˇm visszakĂ¶vethetĹ‘ a mĂ©rĂ©si fĂˇjljĂˇhoz.

### A kvantumâ€“klasszikus hurok valĂłdi hardveren

A `scripts/quantum_loop_hardware.py` a hurkot futtatja egy QPU-n. Az iterĂˇciĂł
hĂˇrom ĂˇramkĂ¶rĂ©t (A, f, Ď†) egy jobba kĂ¶tegeli, minden N. iterĂˇciĂłban
Bell-anchort ad hozzĂˇ, Ă©s a mĂ©rt hardver-minĹ‘sĂ©get (`c`) visszaadja a
dekĂłdolĂłba a Rust bridge-en keresztĂĽl
(`crates/quantum-bridge/src/bin/bridge-cli.rs`) â€” Ă­gy a dekĂłdolĂˇs
matematikĂˇja pontosan egy helyen van.

```bash
# A hurok: 100 iterĂˇciĂł, 1024 lĂ¶vĂ©s, az anchor minden 5. iterĂˇciĂłban
python scripts/quantum_loop_hardware.py --backend ibm_fez --iterations 100 --shots 1024 --anchor-every 5
```

> **Az open csomagban nincs session.** Az IBM megtagadja a Runtime Session-t az
> open csomagon ("not authorized to run a session when using the open plan"),
> Ă­gy minden iterĂˇciĂł Ă¶nĂˇllĂł jobkĂ©nt Ăˇll sorba. A szkript ezĂ©rt a
> sor-vĂˇrakozĂˇst **mĂ©ri**, nem feltĂ©telezi, Ă©s a QPU-mp-eket meg a faliĂłra-idĹ‘t
> kĂĽlĂ¶n kiĂ­rja. A fenntartott ablakhoz mĂˇsik plan instance kell.

> **A hardveres futĂˇs mĂ©rĂ©si jelentĂ©se** â€”
> `docs/MEASUREMENTS-HARDWARE-20261027.txt`, a futĂˇsnaplĂłkkal egyĂĽtt
> (`docs/RUNLOG-*.txt`). Minden szĂˇm a `measurement_raw/` nyers fĂˇjljaibĂłl
> ĂşjraszĂˇmĂ­thatĂł a `scripts/summarize_hardware_run.py` szkripttel.

> **KĂ¶rnyezet:** a mĂ©rĂ©shez tiszta virtuĂˇlis kĂ¶rnyezet ajĂˇnlott. A Qiskit 1.x Ă©s
> 2.x nem Ă©l egyĂĽtt ugyanabban a Pythonban:
> ```bash
> python -m venv .venv-quantum
> .venv-quantum/Scripts/pip install "qiskit>=2.0" "qiskit-ibm-runtime>=0.50" python-dotenv numpy
> .venv-quantum/Scripts/python scripts/hardware_measure.py --local
> ```
> A `scs_quantum` csomag telepĂ­tett pĂ©ldĂˇny hiĂˇnyĂˇban a `SCS_QUANTUM_PATH`
> kĂ¶rnyezetvĂˇltozĂł mutatja a `python/` mappĂˇt.

---

## 5. Licenc

MIT licenc. A hivatkozĂˇsi adatok a `CITATION.cff` fĂˇjlban talĂˇlhatĂłk.

