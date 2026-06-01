//! # lau-calm-noether
//!
//! CALM = Noether — coordination-free learning rules are exactly
//! permutation-symmetric rules with conserved charges.
//!
//! Implements Opus's Emergent Theorem B: coordination-free (CALM) ⟺
//! permutation-symmetric (Noether) ⟺ conserved charge (semilattice join).

pub mod agent;
pub mod calm;
pub mod noether;
pub mod aci;
pub mod symmetry;
pub mod lyapunov;
pub mod generator;
pub mod partition;
pub mod conservation;
pub mod plato;

pub use agent::*;
pub use calm::*;
pub use noether::*;
pub use aci::*;
pub use symmetry::*;
pub use lyapunov::*;
pub use generator::*;
pub use partition::*;
pub use conservation::*;
pub use plato::*;
