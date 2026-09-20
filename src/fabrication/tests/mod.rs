//! `#[cfg(test)] mod tests` body for `fabrication/mod.rs`.
//!
//! Split into a directory module (`fixtures.rs` + `phase_a.rs` +
//! `phase_b.rs`, same `mod.rs`/sibling-file split precedent as `config/`
//! and `request/`, Chunk V1) because adding V-8's Phase B dispatch tests
//! (baffle/support/conic-tolerance) pushed the original single
//! `tests.rs` well past the 300-line limit.

mod fixtures;
mod phase_a;
mod phase_b;
