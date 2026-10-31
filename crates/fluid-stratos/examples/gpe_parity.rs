//! Parity binary: same deterministic initial state as the numpy script.
//! Outputs JSON with max|Δρ|, max|ΔE|, argmax, etc. after 200 steps.

use fluid_stratos::{GpeParams, GpeSolver};
use num_complex::Complex;
use rand_chacha::ChaCha8Rng;
use rand::{Rng, SeedableRng};
use serde_json;

fn genome_positions(n: usize) -> Vec<(f64, f64)> {
    let center = n as f64 / 2.0;
    let radius = n as f64 / 4.0;
    let mut pos = vec![(center, center)];
    for k in 0..6 {
        let a = 2.0 * std::f64::consts::PI * k as f64 / 6.0;
        pos.push((center + radius * 0.5 * a.cos(), center + radius * 0.5 * a.sin()));
    }
    for k in 0..9 {
        let a = 2.0 * std::f64::consts::PI * k as f64 / 9.0;
        pos.push((center + radius * a.cos(), center + radius * a.sin()));
    }
    pos
}

fn mode_energies(psi: &[Vec<Complex<f64>>], params: &GpeParams) -> Vec<f64> {
    let n = params.grid_size;
    let positions = genome_positions(n);
    let mut out = Vec::with_capacity(16);
    for (px, py) in positions.iter().take(16) {
        let mut e = 0.0;
        for i in 0..n {
            for j in 0..n {
                let d = ((i as f64 - px).powi(2) + (j as f64 - py).powi(2)).sqrt();
                if d < 5.0 {
                    let weight = (-(d * d) / (2.0 * 9.0)).exp();
                    e += psi[i][j].norm_sqr() * weight;
                }
            }
        }
        out.push(e);
    }
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let params = GpeParams {
        grid_size: 32,
        domain_size: 10.0,
        dt: 0.005,
        interaction: -1.0,
        trap_frequency: 0.1,
        damping: 0.002,
        hbar: 1.0,
        mass: 1.0,
    };

    let mut solver = GpeSolver::new(params);
    let n = solver.params.grid_size;
    let dx = solver.params.domain_size / n as f64;
    let center = n as f64 / 2.0;
    let sigma = n as f64 / 6.0;

    // Deterministic seed matching the numpy port
    let mut rng = ChaCha8Rng::seed_from_u64(20261026);

    // Same initial state as numpy: (noise + i*noise) * gaussian
    for j in 0..n {
        for i in 0..n {
            let x = (i as f64 - center) / center * solver.params.domain_size / 2.0;
            let y = (j as f64 - center) / center * solver.params.domain_size / 2.0;
            let gauss = (-(x * x + y * y) / (2.0 * sigma * sigma / (center * center))).exp();

            let re: f64 = rng.gen_range(-0.1..0.1);
            let im: f64 = rng.gen_range(-0.1..0.1);
            solver.psi[j][i] = Complex::new(
                (re * gauss),
                (im * gauss),
            );
        }
    }

    // Normalize to 1
    let mut norm_sq = 0.0;
    for row in &solver.psi[..n] {
        for val in &row[..n] {
            norm_sq += val.norm_sqr() * dx * dx;
        }
    }
    let norm = norm_sq.sqrt();
    for row in &mut solver.psi[..n] {
        for val in &mut row[..n] {
            *val /= norm;
        }
    }

    // Save initial energies
    let e0 = mode_energies(&solver.psi, &solver.params);
    let rho0: Vec<f64> = solver.psi.iter().map(|row| {
        row.iter().map(|v| v.norm_sqr()).sum()
    }).collect();

    // Run 200 steps
    for _ in 0..200 {
        solver.step();
    }

    let e1 = mode_energies(&solver.psi, &solver.params);
    let rho1: Vec<f64> = solver.psi.iter().map(|row| {
        row.iter().map(|v| v.norm_sqr()).sum()
    }).collect();

    let max_delta_density = rho0.iter().zip(rho1.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    let max_delta_mode_energy = e0.iter().zip(e1.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    let argmax_initial = e0.iter().enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i).unwrap();
    let argmax_final = e1.iter().enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i).unwrap();

    let out = serde_json::json!({
        "max_delta_density": max_delta_density,
        "max_delta_mode_energy": max_delta_mode_energy,
        "argmax_initial": argmax_initial,
        "argmax_final": argmax_final,
        "mode_energy_initial": e0,
        "mode_energy_final": e1,
    });

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}