//! Hope GPE Parallel — Párhuzamos Gross-Pitaevskii Solver
//!
//! A GPE szekvenciális solver Rayon-alapú párhuzamosítása nagy rácsokhoz.
//! A kettős kézi integráció (split-step Fourier) szétszaggatja a rácsot
//! sorokra, és a FFT-t is párhuzamosítja.

use num_complex::Complex;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use rayon::prelude::*;
use rustfft::{Fft, FftDirection, FftPlanner};
use std::cell::RefCell;
use std::f64::consts::PI;
use std::fmt;
use std::sync::Arc;

// A transzformációk scratch buffere szálonként.
//
// 🧬 A `rayon::for_each` `Fn`-t vár (nem `FnMut`-et), ezért egy közös
// `&mut` scratch nem osztható meg a szálak között. A thread-local
// buffer ezt megoldja: minden szálnak saját scratch-e van, és a
// `process_with_scratch` nem allokál.
thread_local! {
    static FFT_SCRATCH: RefCell<Vec<Complex<f64>>> = const { RefCell::new(Vec::new()) };
}

/// A scratch buffer méretezése és a leképezés futtatása a szálon belül
fn with_scratch<R>(need: usize, f: impl FnOnce(&mut [Complex<f64>]) -> R) -> R {
    FFT_SCRATCH.with(|cell| {
        let mut buf = cell.borrow_mut();
        if buf.len() < need {
            buf.resize(need, Complex::new(0.0, 0.0));
        }
        f(&mut buf)
    })
}

/// Párhuzamos GPE paraméterek
#[derive(Debug, Clone)]
pub struct ParallelGpeParams {
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

impl Default for ParallelGpeParams {
    fn default() -> Self {
        Self {
            grid_size: 128,
            domain_size: 20.0,
            dt: 0.005,
            interaction: -1.0,
            trap_frequency: 0.1,
            damping: 0.002,
            hbar: 1.0,
            mass: 1.0,
        }
    }
}

/// Párhuzamos GPE Solver
/// Párhuzamos GPE Solver
///
/// 🧬 A KINETIKUS LÉPÉS VALÓDI 2D FFT-vel (sor + oszlop transzformáció).
/// A korábbi implementáció fázis-rotációt alkalmazott a RÁCSINDEX-térben
/// (transzformáció nélkül) — ezért a sűrűség-eloszlás matematikailag
/// invariáns volt, és a folyadék sosem ébredt fel.
pub struct ParallelGpeSolver {
    pub params: ParallelGpeParams,
    /// Hullámfüggvény ψ — laposított vektor (sor-major)
    pub psi: Vec<Complex<f64>>,
    /// Külső potenciál V_ext
    pub potential: Vec<f64>,
    /// Koherencia
    pub coherence: f64,
    /// Energia
    pub energy: f64,
    /// Idő
    pub time: f64,
    /// Lépésszám
    pub step: u64,
    /// K-tér vektorok (előre kiszámítva)
    k_sq: Vec<f64>,
    /// Előre leképezett 1D FFT tervek
    fft_fwd: Arc<dyn Fft<f64>>,
    fft_inv: Arc<dyn Fft<f64>>,
    /// Oszlop gyűjtő buffer (gather/scatter)
    col_buf: Vec<Complex<f64>>,
}

/// A `dyn Fft<f64>` nem `Debug`, ezért kézi implementáció.
/// A `Fft` trait `Send + Sync` → a solver szálbiztos marad.
impl fmt::Debug for ParallelGpeSolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParallelGpeSolver")
            .field("params", &self.params)
            .field("coherence", &self.coherence)
            .field("time", &self.time)
            .field("step", &self.step)
            .field("energy", &self.energy)
            .finish()
    }
}

impl ParallelGpeSolver {
    /// Új párhuzamos solver inicializálása
    pub fn new(params: ParallelGpeParams) -> Self {
        let n = params.grid_size;
        let total = n * n;
        let mut psi = vec![Complex::new(0.0, 0.0); total];
        let potential = Self::create_potential(&params);

        // Kezdeti állapot: véletlenszerű zaj + Gaussian csapda
        let center = n as f64 / 2.0;
        let sigma = n as f64 / 6.0;

        // Párhuzamos inicializálás thread-safe RNG-vel
        psi.par_iter_mut().enumerate().for_each(|(idx, value)| {
            // Thread-local RNG seed from index
            let mut rng = ChaCha12Rng::seed_from_u64(idx as u64);
            let i = idx / n;
            let j = idx % n;
            let x = (i as f64 - center) / center * params.domain_size / 2.0;
            let y = (j as f64 - center) / center * params.domain_size / 2.0;
            let r2 = x * x + y * y;

            let gaussian = (-r2 / (2.0 * sigma * sigma / (center * center))).exp();

            let noise_re: f64 = rng.gen_range(-0.1..0.1);
            let noise_im: f64 = rng.gen_range(-0.1..0.1);

            *value = Complex::new(noise_re * gaussian, noise_im * gaussian);
        });

        // Normalizálás
        Self::normalize_par(&mut psi, params.domain_size, n);

        let coherence = Self::measure_coherence_par(&psi, n);

        // K-tér vektorok előre számítása
        let dk = 2.0 * PI / params.domain_size;
        let mut k_sq = Vec::with_capacity(total);
        for i in 0..n {
            for j in 0..n {
                let kx = if i <= n / 2 {
                    i as f64 * dk
                } else {
                    (i as f64 - n as f64) * dk
                };
                let ky = if j <= n / 2 {
                    j as f64 * dk
                } else {
                    (j as f64 - n as f64) * dk
                };
                k_sq.push(kx * kx + ky * ky);
            }
        }

        // FFT tervek egyszer, a solver élettartamára
        let mut planner = FftPlanner::<f64>::new();
        let fft_fwd = planner.plan_fft(n, FftDirection::Forward);
        let fft_inv = planner.plan_fft(n, FftDirection::Inverse);

        Self {
            params,
            psi,
            potential,
            coherence,
            energy: 0.0,
            time: 0.0,
            step: 0,
            k_sq,
            fft_fwd,
            fft_inv,
            col_buf: vec![Complex::new(0.0, 0.0); n],
        }
    }

    /// Külső potenciál létrehozása (16 kognitív mód)
    fn create_potential(params: &ParallelGpeParams) -> Vec<f64> {
        let n = params.grid_size;
        let total = n * n;
        let mut pot = vec![0.0f64; total];
        let center = n as f64 / 2.0;
        let well_depth = 2.0;

        // 16 kognitív mód pozíciói
        let modes = Self::genome_positions(n);

        // Párhuzamos potenciál számítás
        pot.par_iter_mut().enumerate().for_each(|(idx, value)| {
            let i = idx / n;
            let j = idx % n;
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
                let well = -well_depth * (-dist_sq / (2.0 * 9.0)).exp();
                mode_potential += well;
            }

            *value = harmonic + mode_potential;
        });

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

    /// Koherencia mérés (párhuzamos)
    fn measure_coherence_par(psi: &[Complex<f64>], n: usize) -> f64 {
        let (max_density, total_density) = psi
            .par_iter()
            .map(|v| {
                let d = v.norm_sqr();
                (d, d)
            })
            .reduce(|| (0.0, 0.0), |a, b| (a.0.max(b.0), a.1 + b.1));

        let count = (n * n) as f64;
        let mean_density = total_density / count;
        if mean_density < 1e-10 {
            return 0.0;
        }

        let ratio = max_density / mean_density;
        (ratio / 15.0).tanh()
    }

    /// Hullámfüggvény normalizálás (párhuzamos)
    fn normalize_par(psi: &mut [Complex<f64>], domain_size: f64, n: usize) {
        let dx = domain_size / n as f64;
        let norm_sq = psi.par_iter().map(|v| v.norm_sqr() * dx * dx).sum::<f64>();

        let norm = norm_sq.sqrt();
        if norm < 1e-10 {
            return;
        }

        let factor = 1.0 / norm;
        psi.par_iter_mut().for_each(|v| *v *= factor);
    }

    /// Egy GPE lépés (párhuzamos)
    ///
    /// 🧬 A SÖREND: K/2 → V → K/2 (Strang). A kinetikus féllépés valódi
    /// 2D FFT-vel (sor + oszlop transzformáció) történik; a nemlineáris
    /// `g·|ψ|²` tag a V lépésben marad (a sűrűség a fizikai térben értelmezett).
    pub fn step(&mut self) {
        let n = self.params.grid_size;
        let dt = self.params.dt;
        let g = self.params.interaction;
        let hbar = self.params.hbar;
        let mass = self.params.mass;
        let gamma = self.params.damping;

        // 1. Első féllépés: kinetikus (valódi 2D FFT)
        self.kinetic_half_step(dt, hbar, mass);

        // 2. Potenciál + nemlineáris lépés + csillapítás
        self.potential_step(dt, g, hbar, gamma);

        // 3. Második féllépés: kinetikus (valódi 2D FFT)
        self.kinetic_half_step(dt, hbar, mass);

        // 4. Normalizálás
        Self::normalize_par(&mut self.psi, self.params.domain_size, n);

        // 5. Koherencia frissítés
        self.coherence = Self::measure_coherence_par(&self.psi, n);

        // 6. Energia számítás
        self.energy = self.calculate_energy_par();

        // 7. Idő és lépés
        self.time += dt;
        self.step += 1;
    }

    /// A kinetikus féllépés: forward 2D FFT → k-tér fázis → inverse 2D FFT
    fn kinetic_half_step(&mut self, dt: f64, hbar: f64, mass: f64) {
        // Fizikai tér → k-tér (sor- és oszlop-transzformáció)
        self.fft2_forward();

        // k-térbeli fázis: exp(-i·dt·ħ·k²/(4m)) — a K/2 operátor
        let phase_scale = -dt * hbar / (4.0 * mass);
        self.psi
            .par_iter_mut()
            .zip(self.k_sq.par_iter())
            .for_each(|(v, &k2)| {
                let phase = phase_scale * k2;
                *v *= Complex::new(phase.cos(), phase.sin());
            });

        // k-tér → fizikai tér (a `rustfft` nem normalizál: a skálázás itt)
        self.fft2_inverse();
    }

    /// Potenciál + nemlineáris lépés + csillapítás (fizikai tér)
    fn potential_step(&mut self, dt: f64, g: f64, hbar: f64, gamma: f64) {
        let decay = (-gamma * dt).exp();

        self.psi
            .par_iter_mut()
            .zip(self.potential.par_iter())
            .for_each(|(v, &v_pot)| {
                let density = v.norm_sqr();
                let v_total = v_pot + g * density;
                let phase = -dt * v_total / hbar;
                *v *= Complex::new(phase.cos(), phase.sin()) * decay;
            });
    }

    /// 2D forward FFT: sor-transzformáció, majd oszlop-transzformáció
    fn fft2_forward(&mut self) {
        let n = self.params.grid_size;

        // Sorok (párhuzamosan: minden sor független, saját scratch-csel)
        let fft = &self.fft_fwd;
        let need = fft.get_inplace_scratch_len();
        self.psi.par_chunks_mut(n).for_each(|row| {
            with_scratch(need, |scratch| {
                fft.process_with_scratch(row, scratch);
            });
        });

        // Oszlopok: gather → transzformáció → scatter
        for i in 0..n {
            for j in 0..n {
                self.col_buf[j] = self.psi[j * n + i];
            }
            let need = self.fft_fwd.get_inplace_scratch_len();
            with_scratch(need, |scratch| {
                self.fft_fwd
                    .process_with_scratch(&mut self.col_buf, scratch);
            });
            for j in 0..n {
                self.psi[j * n + i] = self.col_buf[j];
            }
        }
    }

    /// 2D inverse FFT + az `n²`-es normalizáció (a `rustfft` nem skáláz)
    fn fft2_inverse(&mut self) {
        let n = self.params.grid_size;

        // Oszlopok
        for i in 0..n {
            for j in 0..n {
                self.col_buf[j] = self.psi[j * n + i];
            }
            let need = self.fft_inv.get_inplace_scratch_len();
            with_scratch(need, |scratch| {
                self.fft_inv
                    .process_with_scratch(&mut self.col_buf, scratch);
            });
            for j in 0..n {
                self.psi[j * n + i] = self.col_buf[j];
            }
        }

        // Sorok
        let fft = &self.fft_inv;
        let need = fft.get_inplace_scratch_len();
        self.psi.par_chunks_mut(n).for_each(|row| {
            with_scratch(need, |scratch| {
                fft.process_with_scratch(row, scratch);
            });
        });

        // Az inverse transzformáció NEM normalizál: 1/n sor × 1/n oszlop = 1/n²
        let inv_n2 = 1.0 / ((n * n) as f64);
        self.psi.par_iter_mut().for_each(|v| *v *= inv_n2);
    }

    /// Energia számítás (párhuzamos)
    fn calculate_energy_par(&self) -> f64 {
        let n = self.params.grid_size;
        let dx = self.params.domain_size / n as f64;
        let g = self.params.interaction;

        self.psi
            .par_iter()
            .zip(self.potential.par_iter())
            .map(|(v, &v_pot)| {
                let density = v.norm_sqr();
                let pot_energy = v_pot * density * dx * dx;
                let nonlin_energy = 0.5 * g * density * density * dx * dx;
                pot_energy + nonlin_energy
            })
            .sum()
    }

    /// Domináns frekvencia detektálás (párhuzamos)
    pub fn dominant_frequency_parallel(&self) -> (f64, f64) {
        let n = self.params.grid_size;
        let _total = n * n;

        // Egyszerű DFT (párhuzamos a frekvenciák felett)
        let (dominant_freq, max_power) = (0..n)
            .into_par_iter()
            .map(|k| {
                let freq = k as f64 / n as f64;
                let mut power = 0.0;

                for i in 0..n {
                    for j in 0..n {
                        let idx = i * n + j;
                        let phase = 2.0 * PI * freq * (i + j) as f64 / n as f64;
                        let re = self.psi[idx].re * phase.cos() - self.psi[idx].im * phase.sin();
                        let im = self.psi[idx].re * phase.sin() + self.psi[idx].im * phase.cos();
                        power += re * re + im * im;
                    }
                }

                (freq, power)
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap_or((0.0, 0.0));

        (dominant_freq, max_power.sqrt())
    }

    /// Teljes állapot
    pub fn state(&self) -> super::gpe::GpeState {
        super::gpe::GpeState {
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
        self.psi
            .par_iter()
            .map(|v| v.norm())
            .reduce(|| 0.0, |a, b| a.max(b))
    }

    /// Átlag sűrűség
    pub fn mean_density(&self) -> f64 {
        let total: f64 = self.psi.par_iter().map(|v| v.norm_sqr()).sum();
        let n = self.params.grid_size;
        total / (n * n) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_gpe_new() {
        let params = ParallelGpeParams {
            grid_size: 32,
            ..Default::default()
        };
        let solver = ParallelGpeSolver::new(params);
        assert!(solver.coherence >= 0.0);
        assert!(solver.coherence <= 1.0);
    }

    #[test]
    fn test_parallel_step() {
        let params = ParallelGpeParams {
            grid_size: 32,
            ..Default::default()
        };
        let mut solver = ParallelGpeSolver::new(params);
        solver.step();
        assert_eq!(solver.step, 1);
    }

    #[test]
    fn test_parallel_coherence() {
        let params = ParallelGpeParams {
            grid_size: 32,
            ..Default::default()
        };
        let solver = ParallelGpeSolver::new(params);
        assert!(solver.coherence >= 0.0);
        assert!(solver.coherence <= 1.0);
    }

    #[test]
    fn test_dominant_frequency_parallel() {
        let params = ParallelGpeParams {
            grid_size: 32,
            ..Default::default()
        };
        let solver = ParallelGpeSolver::new(params);
        let (freq, amp) = solver.dominant_frequency_parallel();
        assert!(freq >= 0.0);
        assert!(freq <= 1.0);
        assert!(amp >= 0.0);
    }
}
