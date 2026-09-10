//! World generation and simulation: realm, nebula, flux flywheels, OAM–flux.
//!
//! Model numbers (λ_t, W_g, 350/π, κ, θ_crit) live here. Geometry stays in
//! `qga-math`.

mod clump;
mod flux;
mod gauge;
mod lorenz;
mod model;
mod nbody;
mod oam;
mod world;

pub use clump::*;
pub use flux::*;
pub use gauge::*;
pub use lorenz::*;
pub use model::*;
pub use nbody::*;
pub use oam::*;
pub use world::*;
