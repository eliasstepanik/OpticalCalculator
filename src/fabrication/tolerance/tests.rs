//! `#[cfg(test)] mod tests` body for `fabrication/tolerance/mod.rs`, split
//! out to stay under the 300-line limit — same `mod.rs`/`tests.rs`
//! sibling-file split precedent as `config/` and `request/` (Chunk V1),
//! now also `fabrication/tolerance/` (Chunk VB2, split when the T1
//! conic-tolerance formula/tests pushed the single file over budget).

use super::*;

/// Worked example ([[v2-tolerance-collimation]] /
/// [[task-snapshot-v2-phaseA]] "V7 | ... λ=550 → surface_pv_tol=68.75nm,
/// waves=0.125"): visual default 550nm.
/// surface_pv_tol_nm = 550/8 = 68.75; wavefront_pv_tol_nm = 550/4 =
/// 137.5. In micrometers: 0.06875 and 0.1375 respectively.
#[test]
fn worked_example_550nm_surface_and_wavefront_tolerance() {
    let spec = compute_phase_a(550.0);

    assert!((spec.wavelength_nm - 550.0).abs() < 1e-12);
    assert!((spec.surface_pv_tol_nm - 68.75).abs() < 1e-12);
    assert!((spec.surface_pv_tol_waves - 0.125).abs() < 1e-12);
    assert!((spec.wavefront_pv_tol_nm - 137.5).abs() < 1e-12);
    assert!((spec.wavefront_pv_tol_waves - 0.25).abs() < 1e-12);

    // Micrometer-unit helper functions (same physics, µm units).
    assert!((surface_figure_tolerance_um(550.0) - 0.06875).abs() < 1e-12);
    assert!((wavefront_error_pv_um(550.0) - 0.1375).abs() < 1e-12);

    // compute_phase_a has no conic geometry, so these stay None/0.0.
    assert_eq!(spec.primary_conic_tolerance, 0.0);
    assert!(spec.primary_conic_tolerance_fractional.is_none());
    assert!(spec.secondary_conic_tolerance.is_none());
    assert!(spec.secondary_conic_tolerance_fractional.is_none());
    assert!(spec.conic_mode.is_none());
}

/// Second wavelength (H-alpha, 656nm) confirms the formula generalizes
/// rather than being hardcoded to the 550nm default.
/// surface_pv_tol_nm = 656/8 = 82.0; wavefront_pv_tol_nm = 656/4 = 164.0.
/// In micrometers: 0.082 and 0.164 respectively.
#[test]
fn h_alpha_656nm_surface_and_wavefront_tolerance() {
    let spec = compute_phase_a(656.0);

    assert!((spec.surface_pv_tol_nm - 82.0).abs() < 1e-12);
    assert!((spec.wavefront_pv_tol_nm - 164.0).abs() < 1e-12);
    assert!((spec.surface_pv_tol_waves - 0.125).abs() < 1e-12);
    assert!((spec.wavefront_pv_tol_waves - 0.25).abs() < 1e-12);

    assert!((surface_figure_tolerance_um(656.0) - 0.082).abs() < 1e-12);
    assert!((wavefront_error_pv_um(656.0) - 0.164).abs() < 1e-12);
}

/// The wavefront tolerance is always exactly 2x the surface tolerance
/// (double-pass reflection relationship) regardless of wavelength — a
/// general-shape check, not just the two worked-example points.
#[test]
fn wavefront_tolerance_is_always_double_surface_tolerance() {
    for wavelength_nm in [400.0, 550.0, 656.0, 1000.0] {
        let spec = compute_phase_a(wavelength_nm);
        assert!((spec.wavefront_pv_tol_nm - 2.0 * spec.surface_pv_tol_nm).abs() < 1e-9);
        assert!(
            (wavefront_error_pv_um(wavelength_nm) - 2.0 * surface_figure_tolerance_um(wavelength_nm))
                .abs()
                < 1e-12
        );
    }
}

/// T1 VERIFIED worked example ([[formula-validation-v2]] T1, A2 RC:
/// D=200mm, primary f/3 → R1=1200mm, y1=100mm, K1=-1.1454741; secondary
/// R2=654.5455mm, y2=34mm, K2=-6.5878621; λ=550nm). Primary
/// ΔK_tol=0.03802 (3.319% fractional), secondary ΔK_tol=0.46166 (7.008%
/// fractional) — the corrected `4·λ·R³/y⁴` form, not the stale
/// `2·λ·R³/y⁴` provisional constant (which would give half these values).
#[test]
fn t1_verified_worked_example_a2_rc_conic_tolerance() {
    let primary_tol = allowable_conic_tolerance(550.0, 1200.0, 100.0);
    assert!(
        (primary_tol - 0.03802).abs() < 1e-4,
        "primary ΔK_tol = {primary_tol}, expected 0.03802"
    );

    let secondary_tol = allowable_conic_tolerance(550.0, 654.5455, 34.0);
    assert!(
        (secondary_tol - 0.46166).abs() < 1e-3,
        "secondary ΔK_tol = {secondary_tol}, expected 0.46166"
    );

    // Sanity: the stale provisional 2λR³/y⁴ constant would give exactly
    // half of primary_tol — confirm we are NOT reproducing that form.
    let stale_provisional = 2.0 * 550.0e-6 * 1200.0_f64.powi(3) / 100.0_f64.powi(4);
    assert!((primary_tol - 2.0 * stale_provisional).abs() < 1e-9);

    let spec = compute_phase_b_conic(
        550.0,
        -1.1454741,
        1200.0,
        100.0,
        Some(-6.5878621),
        Some(654.5455),
        Some(34.0),
        Some("rc".to_string()),
    );
    assert_eq!(spec.conic_mode.as_deref(), Some("rc"));

    assert!((spec.primary_conic_tolerance - 0.03802).abs() < 1e-4);
    assert!((spec.secondary_conic_tolerance.unwrap() - 0.46166).abs() < 1e-3);

    // Fractional tolerances: primary 3.319%, secondary 7.008%.
    let primary_frac_pct = spec.primary_conic_tolerance_fractional.unwrap() * 100.0;
    assert!(
        (primary_frac_pct - 3.319).abs() < 1e-2,
        "primary fractional = {primary_frac_pct}%, expected 3.319%"
    );
    let secondary_frac_pct = spec.secondary_conic_tolerance_fractional.unwrap() * 100.0;
    assert!(
        (secondary_frac_pct - 7.008).abs() < 1e-2,
        "secondary fractional = {secondary_frac_pct}%, expected 7.008%"
    );

    // Phase-A fields are still populated alongside the new conic fields.
    assert!((spec.surface_pv_tol_nm - 68.75).abs() < 1e-12);
    assert!((spec.wavefront_pv_tol_nm - 137.5).abs() < 1e-12);
}

/// Single-mirror case (no secondary conic data supplied): secondary
/// fields stay `None`, primary fields are still computed.
#[test]
fn compute_phase_b_conic_no_secondary_leaves_secondary_fields_none() {
    let spec = compute_phase_b_conic(550.0, -1.0, 1200.0, 100.0, None, None, None, None);

    assert!(spec.primary_conic_tolerance > 0.0);
    assert!(spec.secondary_conic_tolerance.is_none());
    assert!(spec.secondary_conic_tolerance_fractional.is_none());
    assert!(spec.conic_mode.is_none());
}

/// A spherical primary (`k1 == 0.0`) has an undefined fractional
/// tolerance — `primary_conic_tolerance_fractional` must be `None`, not
/// a divide-by-zero `inf`/`NaN`.
#[test]
fn compute_phase_b_conic_zero_k1_gives_none_fractional() {
    let spec = compute_phase_b_conic(550.0, 0.0, 1200.0, 100.0, None, None, None, None);

    assert!(spec.primary_conic_tolerance > 0.0);
    assert!(spec.primary_conic_tolerance_fractional.is_none());
}
