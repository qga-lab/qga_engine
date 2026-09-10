//! Quaternion / Hopf / gauged-lattice / flux-topograph core.
//!
//! Checked Rust port of `flux_hopf_lib` (Hopf, Hurwitz 24) and `qga/lib`
//! book labs (gauge, topographs). Not a third algebra. Do not call Python
//! at runtime. Golden vectors: `tests/fixtures/*_v1.json`.

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
