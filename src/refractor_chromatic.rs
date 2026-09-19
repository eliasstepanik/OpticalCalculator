//! General N-element thin-lens chromatic solver — linear-algebra core.
//!
//! `solve_linear` (Gaussian elimination with partial pivoting, Chunk 9)
//! lives here. `solve_powers` (N=2/3/5 chromatic system assembly, Chunk 10,
//! architecture.md §3) lives in the sibling `refractor_chromatic_solve`
//! module — split out to keep both files under the 300-line limit
//! (constraints.md); the two functions are tightly coupled (`solve_powers`
//! calls `solve_linear` directly) but each is independently substantial
//! with its own worked-example test suite, so a same-crate sibling module
//! (not a directory-nested submodule) keeps the split flat and simple.

use crate::error::OptError;

/// Solve `matrix · x = rhs` for `x` via Gaussian elimination with partial
/// pivoting. `matrix` is N×N (row-major `Vec<Vec<f64>>`), `rhs` is length N.
///
/// ponytail: hand-rolled ~40 LOC per architecture.md §3 — no `nalgebra`, the
/// systems here are ≤5×5 (quintuplet case), a crate buys nothing.
///
/// Returns `Err(OptError::Unsolvable)` if a pivot column is zero within
/// tolerance (singular/ill-conditioned matrix) — never NaN/Inf. The caller
/// (`solve_powers`, Chunk 10) knows the glass names and can build a richer
/// `Unsolvable(vec![...])`; this generic solver has no glass context, so it
/// reports an empty offender list.
const PIVOT_TOLERANCE: f64 = 1e-12;

pub fn solve_linear(matrix: Vec<Vec<f64>>, rhs: Vec<f64>) -> Result<Vec<f64>, OptError> {
    let n = matrix.len();
    if n == 0 || rhs.len() != n || matrix.iter().any(|row| row.len() != n) {
        return Err(OptError::Unsolvable(Vec::new()));
    }

    // Augmented matrix: each row is [a_i0 .. a_i(n-1) | b_i].
    let mut aug: Vec<Vec<f64>> = matrix
        .into_iter()
        .zip(rhs)
        .map(|(mut row, b)| {
            row.push(b);
            row
        })
        .collect();

    // Forward elimination with partial pivoting.
    for col in 0..n {
        // Manual fold (not `.max_by(..).unwrap()`) so a NaN/Inf entry can
        // never panic here — it just fails the tolerance check below instead.
        let mut pivot_row = col;
        let mut pivot_mag = aug[col][col].abs();
        for row in (col + 1)..n {
            let mag = aug[row][col].abs();
            if mag > pivot_mag {
                pivot_row = row;
                pivot_mag = mag;
            }
        }

        if !(pivot_mag >= PIVOT_TOLERANCE) {
            return Err(OptError::Unsolvable(Vec::new()));
        }

        aug.swap(col, pivot_row);

        for row in (col + 1)..n {
            let factor = aug[row][col] / aug[col][col];
            for k in col..=n {
                aug[row][k] -= factor * aug[col][k];
            }
        }
    }

    // Back-substitution.
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let sum: f64 = (row + 1..n).map(|k| aug[row][k] * x[k]).sum();
        x[row] = (aug[row][n] - sum) / aug[row][row];
    }

    Ok(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_known_3x3_system() {
        // formula-validation.md "Worked test values — Chunk 9":
        // M = [[2,1,-1],[-3,-1,2],[-2,1,2]], b = [8,-11,-3] -> x = [2,3,-1]
        let matrix = vec![
            vec![2.0, 1.0, -1.0],
            vec![-3.0, -1.0, 2.0],
            vec![-2.0, 1.0, 2.0],
        ];
        let rhs = vec![8.0, -11.0, -3.0];

        let x = solve_linear(matrix, rhs).expect("well-conditioned system must solve");

        assert!((x[0] - 2.0).abs() < 1e-9, "x0 = {}", x[0]);
        assert!((x[1] - 3.0).abs() < 1e-9, "x1 = {}", x[1]);
        assert!((x[2] - (-1.0)).abs() < 1e-9, "x2 = {}", x[2]);
    }

    #[test]
    fn singular_matrix_returns_err_not_nan() {
        // Duplicate rows -> singular, must error, never produce NaN/panic.
        let matrix = vec![vec![1.0, 2.0], vec![2.0, 4.0]];
        let rhs = vec![3.0, 6.0];

        let result = solve_linear(matrix, rhs);
        match result {
            Err(OptError::Unsolvable(_)) => {}
            other => panic!("expected Unsolvable, got {other:?}"),
        }
    }

    #[test]
    fn all_zero_row_returns_err() {
        let matrix = vec![vec![0.0, 0.0], vec![1.0, 1.0]];
        let rhs = vec![0.0, 2.0];

        let result = solve_linear(matrix, rhs);
        assert!(matches!(result, Err(OptError::Unsolvable(_))));
    }

    #[test]
    fn solves_simple_2x2_system() {
        // x + y = 3, x - y = 1 -> x = 2, y = 1 (hand-verifiable).
        let matrix = vec![vec![1.0, 1.0], vec![1.0, -1.0]];
        let rhs = vec![3.0, 1.0];

        let x = solve_linear(matrix, rhs).expect("2x2 must solve");

        assert!((x[0] - 2.0).abs() < 1e-9, "x0 = {}", x[0]);
        assert!((x[1] - 1.0).abs() < 1e-9, "x1 = {}", x[1]);
    }
}
