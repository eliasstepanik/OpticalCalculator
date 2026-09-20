//! Substrate struct + `SubstrateCatalog` lookup (v2 `--fabrication`, #1+#2).
//!
//! Mirrors `glass.rs`'s `Glass`/`GlassCatalog` pattern exactly, but for
//! MIRROR BLANK substrate material (Pyrex, BK7, Fused Silica, ULE,
//! Zerodur) — distinct from v1's optical LENS glass catalog
//! (`glasses.toml`, nd/vd/pgf). See KB doc
//! projects/optical-calculator/v2-substrate-mass.md for the schema and
//! sourced CTE/density values.
//!
//! `SubstrateCatalog::from_substrates` + `get()` are consumed later by
//! `config::load_substrates` (a future chunk, per architecture-v2.md §5)
//! the same way `GlassCatalog::from_glasses` is consumed by
//! `config::load_glasses` today. This module only owns the data type +
//! lookup, mirroring `glass.rs`'s chunk-scoping note.

use std::collections::HashMap;

use crate::error::OptError;

/// A single mirror-blank substrate material's physical properties.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Substrate {
    pub name: String,
    /// Coefficient of thermal expansion, in ppm/K (×1e-6 /K).
    pub cte_ppm_per_k: f64,
    /// Density in g/cm³.
    pub density_g_cm3: f64,
    /// Relative cost tier, 1 (cheap) .. 5 (premium) — not a dollar figure.
    pub cost_tier: u8,
    /// Typical max practical amateur-telescope-maker aperture, in mm.
    pub max_aperture_mm: f64,
}

/// Catalog of named substrates, keyed by `Substrate::name` for O(1) lookup.
#[derive(Debug, Clone, Default)]
pub struct SubstrateCatalog {
    substrates: HashMap<String, Substrate>,
}

impl SubstrateCatalog {
    /// Build a catalog from a flat list of substrates, keyed by their `name`.
    pub fn from_substrates(substrates: Vec<Substrate>) -> Self {
        Self {
            substrates: substrates.into_iter().map(|s| (s.name.clone(), s)).collect(),
        }
    }

    /// Look up a substrate by name, or `OptError::UnknownSubstrate` if absent.
    pub fn get(&self, name: &str) -> Result<&Substrate, OptError> {
        self.substrates
            .get(name)
            .ok_or_else(|| OptError::UnknownSubstrate(name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirrors `config::GlassEntry`/`GlassesFile` (config/mod.rs) — the
    /// `[substrates.<key>]` table body has no `name` field, the TOML key
    /// supplies it. Test-local only: the real `config::load_substrates`
    /// loader (parallel to `load_glasses`) lands in a later chunk per
    /// architecture-v2.md §5; this shape just proves `substrates.toml`
    /// parses into `SubstrateCatalog` end-to-end today.
    #[derive(Debug, Clone, serde::Deserialize)]
    struct SubstrateEntry {
        cte_ppm_per_k: f64,
        density_g_cm3: f64,
        cost_tier: u8,
        max_aperture_mm: f64,
    }

    #[derive(Debug, Clone, serde::Deserialize)]
    struct SubstratesFile {
        substrates: HashMap<String, SubstrateEntry>,
    }

    fn load_real_catalog() -> SubstrateCatalog {
        let raw = include_str!("../configs/substrates.toml");
        let file: SubstratesFile = toml::from_str(raw).expect("substrates.toml must parse");
        let substrates: Vec<Substrate> = file
            .substrates
            .into_iter()
            .map(|(name, e)| Substrate {
                name,
                cte_ppm_per_k: e.cte_ppm_per_k,
                density_g_cm3: e.density_g_cm3,
                cost_tier: e.cost_tier,
                max_aperture_mm: e.max_aperture_mm,
            })
            .collect();
        SubstrateCatalog::from_substrates(substrates)
    }

    #[test]
    fn real_substrates_toml_parses_and_lookups_succeed() {
        let cat = load_real_catalog();

        let pyrex = cat.get("pyrex").expect("pyrex should be in catalog");
        assert_eq!(pyrex.name, "pyrex");
        assert!((pyrex.cte_ppm_per_k - 3.25).abs() < 1e-9);
        assert!((pyrex.density_g_cm3 - 2.23).abs() < 1e-9);
        assert!(pyrex.cte_ppm_per_k >= 0.01 && pyrex.cte_ppm_per_k <= 10.0);
        assert!(pyrex.density_g_cm3 >= 2.0 && pyrex.density_g_cm3 <= 3.0);
        assert!(pyrex.cost_tier >= 1 && pyrex.cost_tier <= 5);

        let ule = cat.get("ule").expect("ule should be in catalog");
        assert_eq!(ule.name, "ule");
        assert!((ule.cte_ppm_per_k - 0.03).abs() < 1e-9);
        assert!((ule.density_g_cm3 - 2.21).abs() < 1e-9);
        assert!(ule.cte_ppm_per_k >= 0.01 && ule.cte_ppm_per_k <= 10.0);
        assert!(ule.density_g_cm3 >= 2.0 && ule.density_g_cm3 <= 3.0);
    }

    #[test]
    fn get_missing_name_returns_unknown_substrate_error() {
        let cat = load_real_catalog();
        let err = cat.get("moon_rock").expect_err("moon_rock was not inserted");
        assert!(matches!(err, OptError::UnknownSubstrate(name) if name == "moon_rock"));
    }
}
