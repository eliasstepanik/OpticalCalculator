//! Shared optics math: focal length, Dawes limit, Airy disk, plate scale,
//! FOV, lensmaker's equation.
//!
//! Filled in Chunk 7 per architecture.md §4 / §7. All angular results in
//! arcseconds/arcminutes; all lengths in millimeters; lens power in mm⁻¹
//! (inverse-mm units throughout, per architecture.md §3 "Solve + convert").

/// System focal length from aperture and focal ratio: F = D · N.
pub fn focal_length_mm(aperture_mm: f64, f_ratio: f64) -> f64 {
    aperture_mm * f_ratio
}

/// Focal ratio from aperture and focal length: N = F / D.
pub fn f_ratio(aperture_mm: f64, focal_length_mm: f64) -> f64 {
    focal_length_mm / aperture_mm
}

/// Dawes limit — standard visual resolving-power formula (W.R. Dawes, 1867):
/// θ(″) = 116 / D(mm).
pub fn dawes_limit_arcsec(aperture_mm: f64) -> f64 {
    116.0 / aperture_mm
}

/// Airy disk diameter — diffraction-limited spot size: d = 2.44 · λ · N.
/// Wavelength given in nm; result in micrometers (λ_um = λ_nm / 1000, and
/// diameter comes out in the same unit as λ since N is dimensionless).
pub fn airy_disk_diameter_um(f_ratio: f64, wavelength_nm: f64) -> f64 {
    let wavelength_um = wavelength_nm / 1000.0;
    2.44 * wavelength_um * f_ratio
}

/// Useful magnification range — standard ATM rule of thumb (not a hard
/// physical limit): min ≈ D(mm)/6, max ≈ D(mm)×2 (≈ 50×/inch of aperture).
/// Returns (min, max).
pub fn useful_magnification_range(aperture_mm: f64) -> (f64, f64) {
    (aperture_mm / 6.0, aperture_mm * 2.0)
}

/// Plate scale — standard formula: 206265 / F(mm), arcsec per mm at focal
/// plane.
pub fn plate_scale_arcsec_per_mm(focal_length_mm: f64) -> f64 {
    206265.0 / focal_length_mm
}

/// True field of view from a field-stop diameter: angle = field_stop / F
/// (radians), converted to arcminutes via 57.2958 (deg/rad) × 60 (arcmin/deg).
pub fn true_fov_arcmin(field_stop_mm: f64, focal_length_mm: f64) -> f64 {
    (field_stop_mm / focal_length_mm) * 57.2958 * 60.0
}

/// Sensor FOV in arcminutes for a given sensor dimension. Thin wrapper over
/// `plate_scale_arcsec_per_mm` kept only because "dimension × plate scale,
/// arcsec→arcmin" is a common enough call site to be worth naming — callers
/// needing the raw plate scale should call `plate_scale_arcsec_per_mm`
/// directly instead of duplicating the /60 here.
pub fn sensor_fov_arcmin(sensor_dimension_mm: f64, focal_length_mm: f64) -> f64 {
    sensor_dimension_mm * plate_scale_arcsec_per_mm(focal_length_mm) / 60.0
}

/// Lensmaker's equation (thin lens, forward form): φ = (n − 1)(1/R1 − 1/R2).
/// Power in mm⁻¹. Only the forward formula lives here — solving radii from a
/// target power via a Coddington shape factor is refractor-specific and
/// belongs in `designs/refractor.rs` (Chunk 13), not duplicated here.
pub fn thin_lens_power(n: f64, r1_mm: f64, r2_mm: f64) -> f64 {
    (n - 1.0) * (1.0 / r1_mm - 1.0 / r2_mm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focal_length_matches_worked_example() {
        // architecture.md §7: focal_length(200, 5.0) == 1000
        assert!((focal_length_mm(200.0, 5.0) - 1000.0).abs() < 1e-9);
        // Newtonian worked example (formula-validation.md A1): D=200, f/6 -> F=1200
        assert!((focal_length_mm(200.0, 6.0) - 1200.0).abs() < 1e-9);
    }

    #[test]
    fn f_ratio_is_inverse_of_focal_length() {
        assert!((f_ratio(200.0, 1200.0) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn dawes_limit_matches_worked_example() {
        // architecture.md §7: Dawes(200mm) ~= 0.58"
        let d = dawes_limit_arcsec(200.0);
        assert!((d - 0.58).abs() < 0.005, "dawes({}) = {}", 200.0, d);
    }

    #[test]
    fn airy_disk_diameter_at_f5_550nm() {
        // 2.44 * 0.55um * 5 = 6.71 um
        let d = airy_disk_diameter_um(5.0, 550.0);
        assert!((d - 6.71).abs() < 1e-6, "airy = {}", d);
    }

    #[test]
    fn useful_magnification_range_matches_rule_of_thumb() {
        let (min, max) = useful_magnification_range(200.0);
        assert!((min - 33.333333333).abs() < 1e-6);
        assert!((max - 400.0).abs() < 1e-9);
    }

    #[test]
    fn plate_scale_matches_known_206265_value() {
        // 206265 / 1000 = 206.265 arcsec/mm at F=1000mm
        let ps = plate_scale_arcsec_per_mm(1000.0);
        assert!((ps - 206.265).abs() < 1e-9);
    }

    #[test]
    fn true_fov_arcmin_matches_hand_computation() {
        // field_stop=27.5mm (typical 2" eyepiece), F=1200mm
        // (27.5/1200) * 57.2958 * 60 = 78.781725 arcmin
        let fov = true_fov_arcmin(27.5, 1200.0);
        let expected = (27.5f64 / 1200.0) * 57.2958 * 60.0;
        assert!((fov - expected).abs() < 1e-9);
        assert!((fov - 78.7817).abs() < 0.001, "fov = {}", fov);
    }

    #[test]
    fn sensor_fov_arcmin_matches_plate_scale_composition() {
        let f = 1000.0;
        let dim = 28.4;
        let via_wrapper = sensor_fov_arcmin(dim, f);
        let via_compose = dim * plate_scale_arcsec_per_mm(f) / 60.0;
        assert!((via_wrapper - via_compose).abs() < 1e-12);
    }

    #[test]
    fn thin_lens_power_matches_doublet_worked_example() {
        // formula-validation.md Chunk 13: BK7 equiconvex, phi=2.3082734e-3,
        // R = 2(n-1)/phi = 447.78mm -> R1=447.78, R2=-447.78 (equiconvex)
        let n = 1.51680; // BK7 nd
        let r = 447.78;
        let phi = thin_lens_power(n, r, -r);
        assert!((phi - 2.3082734e-3).abs() < 1e-6, "phi = {}", phi);
    }
}
