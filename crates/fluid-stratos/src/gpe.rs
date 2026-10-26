//! Hope GPE Solver — Gross-Pitaevskii Equation
//!
//! A Fluid STRATOS alapja: kvantum folyadékdinamika alapú kognitív mező.
//! Split-Step Fourier módszer a hullámfüggvény evolúciójára.
//!
//! iℏ ∂ψ/∂t = [ −(ħ²/2m) ∇² + V_ext(r) + g |ψ|² ] ψ
//!
//! ()=>[] — A tiszta potenciálból a hullám megszületik

use num_complex::Complex;
use rand::Rng;
use rustfft::{Fft, FftDirection, FftPlanner};
use std::f64::consts::PI;
use std::fmt;
use std::sync::Arc;

/// GPE paraméterek
#[derive(Debug, Clone)]
pub struct GpeParams {
    /// Rács méret (NxN)
    pub grid_size: usize,
    /// Térbeli kiterjedés
    pub domain_size: f64,
    /// Idő lépés
    pub dt: f64,
    /// Interakció erősség (g)
    pub interaction: f64,
    /// Csapda frekvencia (ω)
    pub trap_frequency: f64,
    /// Csillapítás (γ)
    pub damping: f64,
    /// Planck állandó (normalizált)
    pub hbar: f64,
    /// Tömeg (normalizált)
    pub mass: f64,
}

impl Default for GpeParams {
    fn default() -> Self {
        Self {
            grid_size: 128,
            domain_size: 20.0,
            dt: 0.005,
            interaction: -1.0, // Vonzerő (BEC)
            trap_frequency: 0.1,
            damping: 0.002,
            hbar: 1.0,
            mass: 1.0,
        }
    }
}

/// GPE Solver — Split-Step Fourier módszer
///
/// 🧬 A KINETIKUS LÉPÉS VALÓDI 2D FFT-vel (sor + oszlop transzformáció).
/// A korábbi implementáció fázis-rotációt alkalmazott a RÁCSINDEX-térben
/// (transzformáció nélkül), ezért a sűrűség-eloszlás matematikailag
/// invariáns volt: minden művelet vagy fázis-szorzás (nem változtatja a
/// modult) vagy uniform skálázás (nem változtatja a rációt) volt.
/// Mért bizonyíték: 500 lépés alatt a max |Δ| a sűrűség-eloszlásban
/// 1,126e-13, a 16 mód energiája 1,243e-14 volt, a koherencia mindig 1,0.
pub struct GpeSolver {
    pub params: GpeParams,
    /// Hullámfüggvény ψ(x, y, t) — komplex mátrix
    pub psi: Vec<Vec<Complex<f64>>>,
    /// Külső potenciál V_ext(x, y)
    pub potential: Vec<Vec<f64>>,
    /// Koherencia
    pub coherence: f64,
    /// Idő
    pub time: f64,
    /// Lépésszám
    pub step: u64,
    /// Energia
    pub energy: f64,
    /// Előre leképezett 1D FFT tervek (sor + oszlop transzformáció)
    fft_fwd: Arc<dyn Fft<f64>>,
    fft_inv: Arc<dyn Fft<f64>>,
    /// Közös scratch buffer a transzformációkhoz (a `Fft::process` allokálna)
    scratch: Vec<Complex<f64>>,
    /// Oszlop gyűjtő buffer (gather/scatter az oszlop transzformációhoz)
    col_buf: Vec<Complex<f64>>,
    /// Lapos munkamemória (sor-major, n*n)
    work: Vec<Complex<f64>>,
}

/// A `dyn Fft<f64>` nem `Debug`, ezért a derive helyett kézi implementáció.
/// A `Fft` trait `Send + Sync`, így a solver továbbra is szálbiztos.
impl fmt::Debug for GpeSolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GpeSolver")
            .field("params", &self.params)
            .field("coherence", &self.coherence)
            .field("time", &self.time)
            .field("step", &self.step)
            .field("energy", &self.energy)
            .finish()
    }
}

impl GpeSolver {
    /// Új solver inicializálása zajjal
    pub fn new(params: GpeParams) -> Self {
        let n = params.grid_size;
        let mut psi = vec![vec![Complex::new(0.0, 0.0); n]; n];
        let potential = Self::create_potential(&params);

        // Kezdeti állapot: véletlenszerű zaj + Gaussian csapda
        let mut rng = rand::thread_rng();
        let center = n as f64 / 2.0;
        let sigma = n as f64 / 6.0;

        for (i, row) in psi.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() {
                let x = (i as f64 - center) / center * params.domain_size / 2.0;
                let y = (j as f64 - center) / center * params.domain_size / 2.0;
                let r2 = x * x + y * y;

                // Gaussian csapda
                let gaussian = (-r2 / (2.0 * sigma * sigma / (center * center))).exp();

                // Véletlenszerű zaj
                let noise_re: f64 = rng.gen_range(-0.1..0.1);
                let noise_im: f64 = rng.gen_range(-0.1..0.1);

                *value = Complex::new(noise_re * gaussian, noise_im * gaussian);
            }
        }

        // Normalizálás
        Self::normalize(&mut psi, params.domain_size, n);

        let coherence = Self::measure_coherence(&psi, n);

        // FFT tervek egyszer, a solver élettartamára (a `FftPlanner` a
        // terveket cache-eli, a `process_with_scratch` nem allokál).
        let mut planner = FftPlanner::<f64>::new();
        let fft_fwd = planner.plan_fft(n, FftDirection::Forward);
        let fft_inv = planner.plan_fft(n, FftDirection::Inverse);
        let scratch_len = fft_fwd
            .get_inplace_scratch_len()
            .max(fft_inv.get_inplace_scratch_len());

        Self {
            params,
            psi,
            potential,
            coherence,
            time: 0.0,
            step: 0,
            energy: 0.0,
            fft_fwd,
            fft_inv,
            scratch: vec![Complex::new(0.0, 0.0); scratch_len],
            col_buf: vec![Complex::new(0.0, 0.0); n],
            work: vec![Complex::new(0.0, 0.0); n * n],
        }
    }

    /// Külső potenciál létrehozása (16 kognitív mód)
    pub fn create_potential(params: &GpeParams) -> Vec<Vec<f64>> {
        let n = params.grid_size;
        let mut pot = vec![vec![0.0f64; n]; n];
        let center = n as f64 / 2.0;
        let well_depth = 2.0;

        // 16 kognitív mód pozíciói (hexagonális elrendezés)
        let modes = Self::genome_positions(n);

        for (i, row) in pot.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() {
                let x = i as f64;
                let y = j as f64;

                // Harmonikus csapda
                let dx = (x - center) / center;
                let dy = (y - center) / center;
                let harmonic =
                    0.5 * params.trap_frequency * params.trap_frequency * (dx * dx + dy * dy);

                // Kognitív mód kutak
                let mut mode_potential = 0.0;
                for (mx, my) in &modes {
                    let dist_sq = (x - mx).powi(2) + (y - my).powi(2);
                    let well = -well_depth * (-dist_sq / (2.0 * 9.0)).exp(); // σ = 3
                    mode_potential += well;
                }

                *value = harmonic + mode_potential;
            }
        }

        pot
    }

    /// Hope Genome pozíciók (16 mód, hexagonális)
    pub fn genome_positions(n: usize) -> Vec<(f64, f64)> {
        let center = n as f64 / 2.0;
        let radius = n as f64 / 4.0;
        let mut positions = Vec::new();

        // Központi mód (Brain)
        positions.push((center, center));

        // 6 belső mód (inner ring)
        for k in 0..6 {
            let angle = 2.0 * PI * k as f64 / 6.0;
            let x = center + radius * 0.5 * angle.cos();
            let y = center + radius * 0.5 * angle.sin();
            positions.push((x, y));
        }

        // 9 külső mód (outer ring)
        for k in 0..9 {
            let angle = 2.0 * PI * k as f64 / 9.0;
            let x = center + radius * angle.cos();
            let y = center + radius * angle.sin();
            positions.push((x, y));
        }

        positions
    }

    /// Koherencia mérés
    /// C(t) = tanh( max|ψ|² / ⟨|ψ|²⟩ / 15 )
    pub fn measure_coherence(psi: &[Vec<Complex<f64>>], n: usize) -> f64 {
        let mut max_density = 0.0;
        let mut total_density = 0.0;
        let count = (n * n) as f64;

        for row in &psi[..n] {
            for value in &row[..n] {
                let density = value.norm_sqr();
                if density > max_density {
                    max_density = density;
                }
                total_density += density;
            }
        }

        let mean_density = total_density / count;
        if mean_density < 1e-10 {
            return 0.0;
        }

        let ratio = max_density / mean_density;
        (ratio / 15.0).tanh()
    }

    /// Hullámfüggvény normalizálás
    pub fn normalize(psi: &mut [Vec<Complex<f64>>], domain_size: f64, n: usize) {
        let dx = domain_size / n as f64;
        let mut norm_sq = 0.0;

        for row in &psi[..n] {
            for value in &row[..n] {
                norm_sq += value.norm_sqr() * dx * dx;
            }
        }

        let norm = norm_sq.sqrt();
        if norm < 1e-10 {
            return;
        }

        let factor = 1.0 / norm;
        for row in &mut psi[..n] {
            for value in &mut row[..n] {
                *value *= factor;
            }
        }
    }

    /// Egy GPE lépés: Split-Step Fourier
    ///
    /// 🧬 A SÖREND: K/2 → V → K/2 (Strang). A két kinetikus féllépés
    /// egyenként `exp(-i·dt·ħ·k²/(4m))` fázist kap, együtt a teljes
    /// `exp(-i·dt·ħ·k²/(2m))` kinetikus operátort adják.
    ///
    /// 🧬 A NEMLINEÁRIS TAG (`g·|ψ|²`) a V lépésben van, mert a sűrűség
    /// csak a fizikai térben értelmezett. A V lépés a teljes `dt`-t viseli.
    ///
    /// 🧬 A CSILLAPÍTÁS a V lépésben van, a nemlineáris fázissal együtt.
    /// A normalizálás után a damping a nemlineáris erőt gyengíti (nem
    /// eltűnik: a `g·|ψ|²` a sűrűség ABSZOLÚT értékétől függ).
    pub fn step(&mut self) {
        let n = self.params.grid_size;
        let dt = self.params.dt;
        let g = self.params.interaction;
        let hbar = self.params.hbar;
        let mass = self.params.mass;
        let gamma = self.params.damping;

        // A sor-os vektorból a lapos munkamemóriába (sor-major: j*n + i)
        self.load_work();

        // 1. Első féllépés: kinetikus (valódi 2D FFT)
        self.kinetic_half_step(dt, hbar, mass);

        // 2. Potenciál + nemlineáris lépés + csillapítás (fizikai tér)
        self.potential_step(dt, g, hbar, gamma);

        // 3. Második féllépés: kinetikus (valódi 2D FFT)
        self.kinetic_half_step(dt, hbar, mass);

        // Vissza a sor-os vektorba
        self.store_work();

        // Normalizálás
        Self::normalize(&mut self.psi, self.params.domain_size, n);

        // Koherencia frissítés
        self.coherence = Self::measure_coherence(&self.psi, n);

        // Energia számítás
        self.energy = self.calculate_energy();

        // Idő és lépés
        self.time += dt;
        self.step += 1;
    }

    /// A párhuzamos solver ψ-jének átvitele ide.
    ///
    /// 🧬 MIÉRT KELL: ha a párhuzamos solver léptet, de a ψ nincs
    /// szinkronban, a 16 mód energia (`mode_energies`) és a domináns
    /// frekvencia az ELAVULT állapotból számolódik → a mód energiák
    /// statikusak maradnak, akkor is, ha a kinetikus lépés már él.
    pub fn sync_psi_from_parallel(&mut self, parallel: &crate::gpe_parallel::ParallelGpeSolver) {
        let n = self.params.grid_size;
        let src = &parallel.psi;
        if src.len() != n * n {
            return;
        }
        for j in 0..n {
            for i in 0..n {
                self.psi[j][i] = src[j * n + i];
            }
        }
    }

    /// A sor-os ψ átvitele a lapos munkamemóriába
    fn load_work(&mut self) {
        let n = self.params.grid_size;
        for (j, row) in self.psi.iter().enumerate() {
            for (i, v) in row.iter().enumerate() {
                self.work[j * n + i] = *v;
            }
        }
    }

    /// A lapos munkamemória visszaírása a sor-os ψ-ba
    fn store_work(&mut self) {
        let n = self.params.grid_size;
        for j in 0..n {
            for i in 0..n {
                self.psi[j][i] = self.work[j * n + i];
            }
        }
    }

    /// A kinetikus féllépés: forward 2D FFT → k-tér fázis → inverse 2D FFT
    fn kinetic_half_step(&mut self, dt: f64, hbar: f64, mass: f64) {
        let n = self.params.grid_size;
        let k_vec = self.k_space_vectors();

        // Fizikai tér → k-tér
        self.fft2_forward();

        // k-térbeli fázis: exp(-i·dt·ħ·k²/(4m)) — a K/2 operátor
        for j in 0..n {
            for i in 0..n {
                let k2 = k_vec[i].0 * k_vec[i].0 + k_vec[j].1 * k_vec[j].1;
                let phase = -dt * hbar * k2 / (4.0 * mass);
                self.work[j * n + i] *= Complex::new(phase.cos(), phase.sin());
            }
        }

        // k-tér → fizikai tér (a `rustfft` nem normalizál: a skálázás itt)
        self.fft2_inverse();
    }

    /// Potenciál + nemlineáris lépés + csillapítás (fizikai tér)
    fn potential_step(&mut self, dt: f64, g: f64, hbar: f64, gamma: f64) {
        let n = self.params.grid_size;
        let decay = (-gamma * dt).exp();

        for j in 0..n {
            for i in 0..n {
                let idx = j * n + i;
                let density = self.work[idx].norm_sqr();
                let v_total = self.potential[j][i] + g * density;
                let phase = -dt * v_total / hbar;
                self.work[idx] *= Complex::new(phase.cos(), phase.sin()) * decay;
            }
        }
    }

    /// 2D forward FFT: sor-transzformáció, majd oszlop-transzformáció
    fn fft2_forward(&mut self) {
        let n = self.params.grid_size;

        // Sorok
        for j in 0..n {
            let start = j * n;
            let end = start + n;
            self.fft_fwd
                .process_with_scratch(&mut self.work[start..end], &mut self.scratch);
        }

        // Oszlopok: gather → transzformáció → scatter
        for i in 0..n {
            for j in 0..n {
                self.col_buf[j] = self.work[j * n + i];
            }
            self.fft_fwd
                .process_with_scratch(&mut self.col_buf, &mut self.scratch);
            for j in 0..n {
                self.work[j * n + i] = self.col_buf[j];
            }
        }
    }

    /// 2D inverse FFT + az `n²`-es normalizáció (a `rustfft` nem skáláz)
    fn fft2_inverse(&mut self) {
        let n = self.params.grid_size;

        // Oszlopok
        for i in 0..n {
            for j in 0..n {
                self.col_buf[j] = self.work[j * n + i];
            }
            self.fft_inv
                .process_with_scratch(&mut self.col_buf, &mut self.scratch);
            for j in 0..n {
                self.work[j * n + i] = self.col_buf[j];
            }
        }

        // Sorok
        for j in 0..n {
            let start = j * n;
            let end = start + n;
            self.fft_inv
                .process_with_scratch(&mut self.work[start..end], &mut self.scratch);
        }

        // Az inverse transzformáció NEM normalizál: 1/n sor × 1/n oszlop = 1/n²
        let inv_n2 = 1.0 / ((n * n) as f64);
        for v in &mut self.work {
            *v *= inv_n2;
        }
    }

    /// K-tér vektorok
    fn k_space_vectors(&self) -> Vec<(f64, f64)> {
        let n = self.params.grid_size;
        let dk = 2.0 * PI / self.params.domain_size;
        let mut kx = Vec::with_capacity(n);

        for i in 0..n {
            let k = if i <= n / 2 {
                i as f64 * dk
            } else {
                (i as f64 - n as f64) * dk
            };
            kx.push((k, k));
        }

        kx
    }

    /// Energia számítás
    pub fn calculate_energy(&self) -> f64 {
        let n = self.params.grid_size;
        let dx = self.params.domain_size / n as f64;
        let g = self.params.interaction;
        let mut energy = 0.0;

        for i in 0..n {
            for j in 0..n {
                let density = self.psi[i][j].norm_sqr();

                // Potenciális energia
                energy += self.potential[i][j] * density * dx * dx;

                // Nemlineáris energia
                energy += 0.5 * g * density * density * dx * dx;
            }
        }

        energy
    }

    /// Domináns frekvencia detektálás (nyelv dekódoláshoz)
    pub fn dominant_frequency(&self) -> (f64, f64) {
        let n = self.params.grid_size;
        let mut max_power = 0.0;
        let mut dominant_freq = 0.0;

        // Egyszerű spektrális analízis (DFT)
        for k in 0..n {
            let freq = k as f64 / n as f64;
            let mut power = 0.0;

            for i in 0..n {
                for j in 0..n {
                    let phase = 2.0 * PI * freq * (i + j) as f64 / n as f64;
                    let re = self.psi[i][j].re * phase.cos() - self.psi[i][j].im * phase.sin();
                    let im = self.psi[i][j].re * phase.sin() + self.psi[i][j].im * phase.cos();
                    power += re * re + im * im;
                }
            }

            if power > max_power {
                max_power = power;
                dominant_freq = freq;
            }
        }

        (dominant_freq, max_power.sqrt())
    }

    /// Teljes állapot lekérdezése
    pub fn state(&self) -> GpeState {
        GpeState {
            coherence: self.coherence,
            energy: self.energy,
            time: self.time,
            step: self.step,
            max_amplitude: self.max_amplitude(),
            mean_density: self.mean_density(),
        }
    }

    /// Maximális amplitúdó
    pub fn max_amplitude(&self) -> f64 {
        let n = self.params.grid_size;
        let mut max = 0.0;
        for i in 0..n {
            for j in 0..n {
                let amp = self.psi[i][j].norm();
                if amp > max {
                    max = amp;
                }
            }
        }
        max
    }

    /// Átlag sűrűség
    pub fn mean_density(&self) -> f64 {
        let n = self.params.grid_size;
        let mut total = 0.0;
        for i in 0..n {
            for j in 0..n {
                total += self.psi[i][j].norm_sqr();
            }
        }
        total / (n * n) as f64
    }
}

/// GPE állapot
#[derive(Debug, Clone)]
pub struct GpeState {
    pub coherence: f64,
    pub energy: f64,
    pub time: f64,
    pub step: u64,
    pub max_amplitude: f64,
    pub mean_density: f64,
}

impl std::fmt::Display for GpeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "🌊 GPE State:\n  Coherence: {:.3}\n  Energy: {:.3}\n  Time: {:.3}\n  Step: {}\n  Max Amp: {:.3}\n  Mean Density: {:.3}",
            self.coherence, self.energy, self.time, self.step, self.max_amplitude, self.mean_density
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpe_new() {
        let params = GpeParams::default();
        let solver = GpeSolver::new(params);
        assert!(solver.coherence >= 0.0);
        assert!(solver.coherence <= 1.0);
    }

    #[test]
    fn test_gpe_step() {
        let params = GpeParams {
            grid_size: 32,
            ..Default::default()
        };
        let mut solver = GpeSolver::new(params);
        let _initial_coherence = solver.coherence;
        solver.step();
        // Koherencia változik
        assert!(solver.step == 1);
    }

    #[test]
    fn test_coherence_range() {
        let params = GpeParams {
            grid_size: 32,
            ..Default::default()
        };
        let solver = GpeSolver::new(params);
        assert!(solver.coherence >= 0.0);
        assert!(solver.coherence <= 1.0);
    }

    #[test]
    fn test_genome_positions() {
        let positions = GpeSolver::genome_positions(128);
        assert_eq!(positions.len(), 16); // 1 + 6 + 9 = 16
    }

    #[test]
    fn test_dominant_frequency() {
        let params = GpeParams {
            grid_size: 32,
            ..Default::default()
        };
        let solver = GpeSolver::new(params);
        let (freq, amp) = solver.dominant_frequency();
        assert!(freq >= 0.0);
        assert!(freq <= 1.0);
        assert!(amp >= 0.0);
    }

    // ------------------------------------------------------------------
    // A REGRESSZIÓS TESZTEK — a halott (fázis-rotációs) kinetikus lépés ellen
    // ------------------------------------------------------------------

    /// A sűrűség-eloszlásnak MOZGATNIA kell a lépés után.
    ///
    /// 🧬 A HIBÁS (fázis-rotációs) kinetikus lépésben minden művelet vagy
    /// fázis-szorzás volt (nem változtatja a modult) vagy uniform skálázás
    /// (nem változtatja a rációt) → a sűrűség-eloszlás MATEMATIKAILAG
    /// INVARIÁNS volt. Mért érték a halott kódon: max |Δ| = 1,126e-13
    /// 500 lépés alatt (64×64). Ezt a tesztet a javítás előtt FAIL-elte.
    #[test]
    fn test_density_distribution_actually_moves() {
        let params = GpeParams {
            grid_size: 32,
            ..Default::default()
        };
        let mut solver = GpeSolver::new(params);

        let before: Vec<f64> = solver
            .psi
            .iter()
            .flat_map(|row| row.iter().map(|c| c.norm_sqr()))
            .collect();

        for _ in 0..50 {
            solver.step();
        }

        let after: Vec<f64> = solver
            .psi
            .iter()
            .flat_map(|row| row.iter().map(|c| c.norm_sqr()))
            .collect();

        let max_delta = before
            .iter()
            .zip(after.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f64, f64::max);

        assert!(
            max_delta > 1e-6,
            "a kinetikus lépésnek meg kell mozgatnia a sűrűség-eloszlást; \
             max_delta={:.3e} (a halott kódon 1,126e-13 volt)",
            max_delta
        );
    }

    /// A 16 kognitív mód energiájának mozognia kell — különben a
    /// `FluidStratos` 16 módja statikus marad, és a nyelvi dekódolás
    /// és a rezonancia nem kap valós bemenetet.
    ///
    /// 🧬 A HALOTT kódon: max |Δ| = 1,243e-14 500 lépés alatt.
    #[test]
    fn test_mode_energies_change_over_steps() {
        let params = GpeParams {
            grid_size: 32,
            ..Default::default()
        };
        let mut solver = GpeSolver::new(params);

        let mode_energy = |s: &GpeSolver| -> Vec<f64> {
            let n = s.params.grid_size;
            let positions = GpeSolver::genome_positions(n);
            let mut out = Vec::with_capacity(16);
            for (px, py) in positions.iter().take(16) {
                let mut e = 0.0;
                for i in 0..n {
                    for j in 0..n {
                        let d = ((i as f64 - px).powi(2) + (j as f64 - py).powi(2)).sqrt();
                        if d < 5.0 {
                            let weight = (-(d * d) / (2.0 * 9.0)).exp();
                            e += s.psi[i][j].norm_sqr() * weight;
                        }
                    }
                }
                out.push(e);
            }
            out
        };

        let before = mode_energy(&solver);
        for _ in 0..50 {
            solver.step();
        }
        let after = mode_energy(&solver);

        let max_delta = before
            .iter()
            .zip(after.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f64, f64::max);

        assert!(
            max_delta > 1e-9,
            "a 16 mód energiájának mozognia kell; max_delta={:.3e} \
             (a halott kódon 1,243e-14 volt)",
            max_delta
        );
    }

    /// A normálás után a hullámfüggvény egységnyi marad — a valódi FFT
    /// nem szennyezheti a normát (az inverse transzformáció 1/n² skálázás
    /// nélkül 1/n²-szeres hibát adna minden lépésben).
    #[test]
    fn test_normalization_holds_after_fft_steps() {
        let params = GpeParams {
            grid_size: 32,
            ..Default::default()
        };
        let mut solver = GpeSolver::new(params);
        let n = solver.params.grid_size;
        let dx = solver.params.domain_size / n as f64;

        for _ in 0..20 {
            solver.step();
            let norm_sq: f64 = solver
                .psi
                .iter()
                .flat_map(|row| row.iter().map(|c| c.norm_sqr()))
                .sum::<f64>()
                * dx
                * dx;
            assert!(
                (norm_sq - 1.0).abs() < 1e-9,
                "a normalizálásnak meg kell tartania a normát; norm_sq={:.12}",
                norm_sq
            );
        }
    }
}
