// Stage S0 quarantine: inherited recursive-improvement modules remain in the
// repository for inspection, but are deliberately absent from the compiled
// crate surface. Re-enabling them requires an explicit post-SEM-0 source
// change authorized as a constitutional amendment or later-stage gate.
pub mod quarantine;

pub use synapse_core;
