//! System reflective throughput physics (#8, [[v2-coatings-throughput]]
//! `## System throughput (throughput.rs)`) — Chunk V6,
//! [[task-snapshot-v2-phaseA]].
//!
//! **Chunk V6 scope:** standalone module + `compute()` free function only,
//! mirroring Chunk V4's (`substrate_mass.rs`) precedent exactly. Wiring
//! into `fabrication::compute`'s dispatch match (`fabrication/mod.rs`,
//! Chunk V2) is Chunk V9's job per [[task-snapshot-v2-phaseA]]'s chunk
//! table ("V9 ... registered in fabrication::compute's dispatch match") —
//! deliberately left untouched here, staying scoped to this file (and the
//! one-line `pub mod throughput;` registration in `fabrication/mod.rs`
//! needed to make the file part of the crate tree, same minimal-only
//! pattern Chunk V4 used for `substrate_mass`).
//!
//! Formula ([[v2-coatings-throughput]] `## System throughput`):
//! `T_system = (reflectivity_pct / 100)^n_mirrors`, reported as a percent
//! (`system_throughput_pct = 100 * T_system`). Each reflective surface
//! multiplies the transmitted fraction by its single-reflection
//! reflectivity — a plain product-of-N-factors closed form, no iteration
//! needed since all mirrors share one coating in v2 (`Request.coating`
//! names one coating for the whole system, per the KB doc).
//!
//! **Refractor case (n_mirrors = 0):** the formula's own exponent-0
//! identity handles this without a special-cased branch —
//! `base.powi(0) == 1.0` for any `base`, so `system_throughput_pct` is
//! naturally `100.0`. `applicable` is set to `false` for `n_mirrors == 0`
//! per `ThroughputSpec`'s doc comment ("false for refractors") so callers
//! know the reflective-throughput number isn't meaningful for a refractor
//! (its own glass-transmission/AR-coating losses are out of scope for this
//! chunk per the KB doc's explicit limitation note). `compute()` is
//! infallible — it never errors on any `n_mirrors` value, including 0.

use crate::coating::Coating;
use crate::fabrication::ThroughputSpec;

/// Compute the system-level reflective throughput for an `n_mirrors`-mirror
/// design, all mirrors assumed to share `coating`.
///
/// Formula ([[v2-coatings-throughput]]):
/// `system_throughput_pct = (coating.reflectivity_pct / 100)^n_mirrors * 100`.
///
/// Infallible: `n_mirrors` comes from `DesignDef.n_mirrors`
/// ([[architecture-v2]] §2a), which is not user-supplied free-form input,
/// so there is no invalid-range case to reject here.
pub fn compute(coating: &Coating, n_mirrors: u32) -> ThroughputSpec {
    let base = coating.reflectivity_pct / 100.0;
    let system_throughput_pct = base.powi(n_mirrors as i32) * 100.0;

    ThroughputSpec {
        coating: coating.name.clone(),
        reflectivity_pct: coating.reflectivity_pct,
        n_mirrors,
        system_throughput_pct,
        applicable: n_mirrors > 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enhanced_aluminum() -> Coating {
        Coating {
            name: "enhanced_aluminum".to_string(),
            reflectivity_pct: 94.0,
        }
    }

    fn protected_silver() -> Coating {
        Coating {
            name: "protected_silver".to_string(),
            reflectivity_pct: 97.5,
        }
    }

    /// Newtonian (2 mirrors), enhanced aluminum: 0.94^2 * 100 = 88.36%
    /// ([[v2-coatings-throughput]] worked example, [[task-snapshot-v2-phaseA]]
    /// acceptance highlight "N=2 enhanced Al(94.0)=88.36%").
    #[test]
    fn two_mirror_enhanced_aluminum_is_88_36_pct() {
        let spec = compute(&enhanced_aluminum(), 2);
        assert_eq!(spec.coating, "enhanced_aluminum");
        assert!((spec.reflectivity_pct - 94.0).abs() < 1e-9);
        assert_eq!(spec.n_mirrors, 2);
        assert!((spec.system_throughput_pct - 88.36).abs() < 1e-9);
        assert!(spec.applicable);
    }

    /// RC/Cassegrain (2 mirrors), protected silver: 0.975^2 * 100 = 95.0625%
    /// ([[task-snapshot-v2-phaseA]] acceptance highlight
    /// "protected Ag(97.5)=95.06%" — exact value is 95.0625, the doc's
    /// "95.06" is the rounded display form).
    #[test]
    fn two_mirror_protected_silver_is_95_0625_pct() {
        let spec = compute(&protected_silver(), 2);
        assert!((spec.system_throughput_pct - 95.0625).abs() < 1e-9);
        assert!(spec.applicable);
    }

    /// Refractor (n_mirrors = 0): exponent-0 identity naturally yields
    /// 100.0% with no special-cased branch, and `applicable` is false
    /// ([[task-snapshot-v2-phaseA]] "refractor N=0 -> 100.0%,
    /// applicable=false").
    #[test]
    fn refractor_zero_mirrors_is_100_pct_and_not_applicable() {
        let spec = compute(&enhanced_aluminum(), 0);
        assert!((spec.system_throughput_pct - 100.0).abs() < 1e-9);
        assert!(!spec.applicable);
    }

    /// General N-mirror edge case (not a shipped design, but the formula
    /// must handle it generally): 3 mirrors, 90% coating -> 0.9^3 * 100 =
    /// 72.9%.
    #[test]
    fn three_mirror_ninety_pct_coating_is_72_9_pct() {
        let coating = Coating {
            name: "test_90pct".to_string(),
            reflectivity_pct: 90.0,
        };
        let spec = compute(&coating, 3);
        assert!((spec.system_throughput_pct - 72.9).abs() < 1e-9);
        assert!(spec.applicable);
    }

    /// One mirror is the degenerate N=1 case: throughput equals the raw
    /// reflectivity, and it's still `applicable` (only N=0 is not).
    #[test]
    fn one_mirror_equals_raw_reflectivity_and_is_applicable() {
        let spec = compute(&enhanced_aluminum(), 1);
        assert!((spec.system_throughput_pct - 94.0).abs() < 1e-9);
        assert!(spec.applicable);
    }
}
