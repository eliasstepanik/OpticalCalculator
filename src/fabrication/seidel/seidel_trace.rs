//! Paraxial marginal+chief ray trace machinery for the two-mirror Seidel
//! sum evaluation (#7, [[v2-support-seidel]] "Locked formulas" section,
//! S2 VERIFIED). Split out from `seidel.rs` to keep that file under the
//! 300-line limit — this file owns only the trace/summation arithmetic,
//! `seidel.rs` owns the public `compute()`/`SeidelSpec` API and the
//! blur-conversion formulas.
//!
//! **Not a general-purpose N-surface ray tracer** — hardcoded to exactly
//! 2 mirror surfaces (primary + secondary), matching this tool's
//! analytical-only, non-optimization scope
//! ([[architecture-v2]] "kept strictly analytical").
//!
//! **Sign convention (Welford reflection-as-refraction, locked by S2):**
//! light travels +z; `c = 1/R` with R > 0 iff the center of curvature is
//! downstream of the vertex — both Cassegrain mirrors have **R < 0**.
//! Reflection = refraction with `n′ = −n` (n: +1 → −1 → +1). Mirror
//! separation is signed **negative** along travel (`d = −|separation|`).
//! Stop at the primary ⇒ chief ray height at surface 1 is `h̄₁ = 0`.
//!
//! **Paraxial trace** (reduced angle `ω = n·u`, `u` = slope; same
//! equations for the chief ray with bars):
//!
//! ```text
//! φ  = (n′ − n)·c            (surface power)
//! ω′ = ω − h·φ               (refraction)
//! h_next = h + (dz/n′)·ω′    (transfer, dz signed to next surface)
//! ```

/// One surface's Seidel-sum input terms: ray heights/invariants at the
/// surface, plus `Δ(u/n)`/`Δ(1/n)` across it and its own `n, n′, c, K`.
///
/// `A = n(hc+u)` (marginal refraction invariant), `Ā = n(h̄c+ū)` (chief
/// refraction invariant) — Welford ch.7 §7.2 notation, reused verbatim by
/// [[v2-support-seidel]]'s locked sum formulas.
pub(super) struct SurfaceTerms {
    pub h: f64,
    pub hbar: f64,
    pub a: f64,
    pub abar: f64,
    pub delta_u_over_n: f64,
    pub delta_inv_n: f64,
    pub n: f64,
    pub nprime: f64,
    pub c: f64,
    pub k: f64,
}

/// Full 2-surface paraxial trace result: both surfaces' `SurfaceTerms`,
/// the Lagrange invariant `H = n(ūh − uh̄)` (evaluated at surface 1, where
/// the stop-at-primary convention gives `h̄₁ = 0`), and the final image-
/// space `n′` / marginal-ray slope `u′` (needed by the blur-conversion
/// formulas in `seidel.rs`).
pub(super) struct ParaxialTrace {
    pub surfaces: [SurfaceTerms; 2],
    pub lagrange_h: f64,
    pub exit_n_prime: f64,
    pub exit_u_prime: f64,
}

/// One refraction+transfer step for a single ray (marginal or chief) at
/// one surface. Returns `(h_after_this_surface_before_transfer_is_unused,
/// u_after, omega_after)` — actually returns just the post-refraction
/// slope `u'` and reduced angle `ω'`, since height only changes on
/// *transfer* to the next surface, not at the refraction itself.
fn refract(h: f64, u: f64, n: f64, nprime: f64, phi: f64) -> (f64, f64) {
    let omega = n * u;
    let omega_prime = omega - h * phi;
    let u_prime = omega_prime / nprime;
    (u_prime, omega_prime)
}

/// Trace the marginal ray (`h₁=D/2, u₁=0`) and chief ray (`h̄₁=0, ū₁=θ`,
/// stop at primary) through the two mirrors and assemble both surfaces'
/// `SurfaceTerms` plus the Lagrange invariant and exit `(n′, u′)`.
///
/// `r1_mm`/`r2_mm` are radius-of-curvature **magnitudes** (v1's existing
/// positive-magnitude convention, e.g. `r1_mm=1200.0`); this function
/// applies the locked `c = 1/R, R<0 for both mirrors` sign internally
/// (`c = −1/|R|`). `mirror_separation_d_mm` is likewise a positive
/// magnitude; the locked `d = −|separation|` (negative along travel) sign
/// is applied internally.
pub(super) fn trace_two_mirror(
    aperture_d_mm: f64,
    r1_mm: f64,
    r2_mm: f64,
    mirror_separation_d_mm: f64,
    k1: f64,
    k2: f64,
    theta_rad: f64,
) -> ParaxialTrace {
    // Surface 1 (primary): n=+1 -> n'=-1, c1 = -1/|R1|.
    let n1 = 1.0_f64;
    let n1p = -1.0_f64;
    let c1 = -1.0 / r1_mm;

    let h1 = aperture_d_mm / 2.0;
    let u1 = 0.0_f64;
    let hbar1 = 0.0_f64; // stop at primary
    let ubar1 = theta_rad;

    let phi1 = (n1p - n1) * c1;
    let a1 = n1 * (h1 * c1 + u1);
    let abar1 = n1 * (hbar1 * c1 + ubar1);

    let (u1p, omega1p) = refract(h1, u1, n1, n1p, phi1);
    let (ubar1p, omegabar1p) = refract(hbar1, ubar1, n1, n1p, phi1);

    let delta_u_over_n_1 = u1p / n1p - u1 / n1;
    let delta_inv_n_1 = 1.0 / n1p - 1.0 / n1;

    // Transfer to surface 2. Separation signed negative along travel.
    let d_signed = -mirror_separation_d_mm;
    let h2 = h1 + (d_signed / n1p) * omega1p;
    let hbar2 = hbar1 + (d_signed / n1p) * omegabar1p;
    let u2 = u1p;
    let ubar2 = ubar1p;

    // Surface 2 (secondary): n=-1 -> n'=+1, c2 = -1/|R2|.
    let n2 = n1p;
    let n2p = 1.0_f64;
    let c2 = -1.0 / r2_mm;

    let phi2 = (n2p - n2) * c2;
    let a2 = n2 * (h2 * c2 + u2);
    let abar2 = n2 * (hbar2 * c2 + ubar2);

    let (u2p, _omega2p) = refract(h2, u2, n2, n2p, phi2);
    let (_ubar2p, _omegabar2p) = refract(hbar2, ubar2, n2, n2p, phi2);

    let delta_u_over_n_2 = u2p / n2p - u2 / n2;
    let delta_inv_n_2 = 1.0 / n2p - 1.0 / n2;

    // Lagrange invariant H = n(uh - uh_bar); invariant everywhere, so
    // evaluate at surface 1 where h_bar1 = 0 (stop at primary).
    let lagrange_h = n1 * (ubar1 * h1 - u1 * hbar1);

    ParaxialTrace {
        surfaces: [
            SurfaceTerms {
                h: h1,
                hbar: hbar1,
                a: a1,
                abar: abar1,
                delta_u_over_n: delta_u_over_n_1,
                delta_inv_n: delta_inv_n_1,
                n: n1,
                nprime: n1p,
                c: c1,
                k: k1,
            },
            SurfaceTerms {
                h: h2,
                hbar: hbar2,
                a: a2,
                abar: abar2,
                delta_u_over_n: delta_u_over_n_2,
                delta_inv_n: delta_inv_n_2,
                n: n2,
                nprime: n2p,
                c: c2,
                k: k2,
            },
        ],
        lagrange_h,
        exit_n_prime: n2p,
        exit_u_prime: u2p,
    }
}

/// Sum the four Seidel sums (S_I..S_IV) across both surfaces of a
/// `ParaxialTrace`, per [[v2-support-seidel]]'s locked formulas
/// (Welford ch.7 §7.2 sphere terms + ch.8 conic terms):
///
/// ```text
/// S_I   = -Sum A^2*h*D(u/n)   + Sum (n'-n)*K*c^3*h^4
/// S_II  = -Sum A*Abar*h*D(u/n)+ Sum (n'-n)*K*c^3*hbar*h^3
/// S_III = -Sum Abar^2*h*D(u/n)+ Sum (n'-n)*K*c^3*hbar^2*h^2
/// S_IV  = -Sum H^2*c*D(1/n)                      (no conic term)
/// ```
///
/// Returns `(S_I, S_II, S_III, S_IV)`, all in mm.
pub(super) fn seidel_sums(trace: &ParaxialTrace) -> (f64, f64, f64, f64) {
    let mut s1 = 0.0;
    let mut s2 = 0.0;
    let mut s3 = 0.0;
    let mut s4 = 0.0;

    for s in &trace.surfaces {
        let sphere1 = -(s.a * s.a) * s.h * s.delta_u_over_n;
        let sphere2 = -(s.a * s.abar) * s.h * s.delta_u_over_n;
        let sphere3 = -(s.abar * s.abar) * s.h * s.delta_u_over_n;

        let c_cubed = s.c * s.c * s.c;
        let conic1 = (s.nprime - s.n) * s.k * c_cubed * s.h.powi(4);
        let conic2 = (s.nprime - s.n) * s.k * c_cubed * s.hbar * s.h.powi(3);
        let conic3 = (s.nprime - s.n) * s.k * c_cubed * s.hbar * s.hbar * s.h * s.h;

        s1 += sphere1 + conic1;
        s2 += sphere2 + conic2;
        s3 += sphere3 + conic3;

        s4 += -(trace.lagrange_h * trace.lagrange_h) * s.c * s.delta_inv_n;
    }

    (s1, s2, s3, s4)
}
