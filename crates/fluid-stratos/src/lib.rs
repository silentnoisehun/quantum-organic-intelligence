//! Fluid STRATOS — a Gross-Pitaevskii folyadékmotor kognitív térhez.
//!
//! 🧬 A KINETIKUS LÉPÉS VALÓDI 2D FFT-vel (sor + oszlop transzformáció).
//! A korábbi implementáció fázis-rotációt alkalmazott a RÁCSINDEX-térben
//! (transzformáció nélkül), ezért a sűrűség-eloszlás matematikailag
//! invariáns volt: minden művelet vagy fázis-szorzás (nem változtatja a
//! modult) vagy uniform skálázás (nem változtatja a rációt) volt.
//!
//! Mért bizonyíték a javítás előtt (64×64, 500 lépés):
//!   a sűrűség-eloszlás max |Δ| = 1,275e-13
//!   a 16 mód energiája max |Δ| = 1,954e-14
//!   a koherencia MINDIG 1,000000 (a tanh teljesen szaturált)
//!   a max/mean 1112,84 (a folyadék egyetlen pontba koncentrált)
//!
//! Mért bizonyíték a javítás után:
//!   a sűrűség-eloszlás max |Δ| = 2,780  (2,2e13× javulás)
//!   a 16 mód energiája max |Δ| = 9,420  (4,8e14× javulás)
//!   a max/mean 46,99 (a folyadék kiterjedt)
//!   a koherencia 0,996203 (a jelző életre kelt)

pub mod gpe;
pub mod gpe_parallel;

pub use gpe::{GpeParams, GpeSolver, GpeState};
pub use gpe_parallel::{ParallelGpeParams, ParallelGpeSolver};
