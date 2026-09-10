//! Quaternion / Hopf / gauged-lattice / flux-topograph core.
//!
//! Checked Rust port of `flux_hopf_lib` (classical Hopf, Hurwitz 24) and
//! book labs (gauge, topographs). Fixtures `tests/fixtures/*_v1.json` are
//! the library SoT for the 24 units and Chapter 2 Hopf. Not a third algebra.
//! Model numbers (λ_t, W_g, 350/π) live in `qga-sim`.

mod constants;
mod hopf;
mod lattice;
mod quat;
mod topograph;

pub use constants::*;
pub use hopf::*;
pub use lattice::*;
pub use quat::*;
pub use topograph::*;
