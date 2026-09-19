//! Glass struct + `GlassCatalog` lookup.
//!
//! Filled in Chunk 3 per components.md: `Glass{name, nd, vd, pgf}`,
//! `GlassCatalog::get(&str) -> Result<&Glass, OptError>`.
//!
//! `nd`/`vd`/`pgf` feed the N-element thin-lens chromatic solver
//! (architecture.md §3): `vd` (Abbe number) drives the achromatism row,
//! `pgf` (partial dispersion P_g,F) drives the secondary-spectrum row.
//! TOML loading (`configs/glasses.toml` -> `GlassCatalog`) lands in
//! Chunk 5's `config.rs`; this module only owns the data type + lookup.

use std::collections::HashMap;

use crate::error::OptError;

/// A single optical glass's dispersion properties.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Glass {
    pub name: String,
    /// Refractive index at the d-line.
    pub nd: f64,
    /// Abbe number (Vd).
    pub vd: f64,
    /// Partial dispersion P_g,F.
    pub pgf: f64,
}

/// Catalog of named glasses, keyed by `Glass::name` for O(1) lookup.
#[derive(Debug, Clone, Default)]
pub struct GlassCatalog {
    glasses: HashMap<String, Glass>,
}

impl GlassCatalog {
    /// Build a catalog from a flat list of glasses, keyed by their `name`.
    pub fn from_glasses(glasses: Vec<Glass>) -> Self {
        Self {
            glasses: glasses.into_iter().map(|g| (g.name.clone(), g)).collect(),
        }
    }

    /// Look up a glass by name, or `OptError::UnknownGlass` if absent.
    pub fn get(&self, name: &str) -> Result<&Glass, OptError> {
        self.glasses
            .get(name)
            .ok_or_else(|| OptError::UnknownGlass(name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_catalog() -> GlassCatalog {
        GlassCatalog::from_glasses(vec![
            Glass {
                name: "BK7".to_string(),
                nd: 1.51680,
                vd: 64.17,
                pgf: 0.5349,
            },
            Glass {
                name: "F2".to_string(),
                nd: 1.62004,
                vd: 36.37,
                pgf: 0.5828,
            },
            Glass {
                name: "FPL53".to_string(),
                nd: 1.43875,
                vd: 94.66,
                pgf: 0.5340,
            },
        ])
    }

    #[test]
    fn get_returns_matching_glass() {
        let cat = test_catalog();
        let g = cat.get("F2").expect("F2 should be in catalog");
        assert_eq!(g.name, "F2");
        assert!((g.nd - 1.62004).abs() < 1e-9);
        assert!((g.vd - 36.37).abs() < 1e-9);
        assert!((g.pgf - 0.5828).abs() < 1e-9);
    }

    #[test]
    fn get_missing_name_returns_unknown_glass_error() {
        let cat = test_catalog();
        let err = cat.get("SF2").expect_err("SF2 was not inserted");
        assert!(matches!(err, OptError::UnknownGlass(name) if name == "SF2"));
    }
}
