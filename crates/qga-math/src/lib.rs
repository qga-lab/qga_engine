//! Quaternion / Hopf / gauged-lattice / flux-topograph core.
//!
//! Ports of `qga/lib` (classical Hopf, Hurwitz, gauge, topographs) and the
//! maps in `flux_hopf_lib.hopf` / `.constants`. This crate is the engine's
//! source of truth — do not call Python at runtime.

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
