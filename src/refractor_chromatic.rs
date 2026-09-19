//! General N-element thin-lens chromatic solver.
//!
//! `solve_linear` (Gaussian elimination with partial pivoting) lands in
//! Chunk 9; `solve_powers` (N=2/3/5 chromatic system assembly) lands in
//! Chunk 10 per architecture.md §3.
