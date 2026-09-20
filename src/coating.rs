//! Coating struct + `CoatingCatalog` lookup.
//!
//! v2 Chunk V5 per v2-coatings-throughput.md: `Coating{name,
//! reflectivity_pct}`, `CoatingCatalog::get(&str) -> Result<&Coating,
//! OptError>`. Mirrors `glass.rs`'s `Glass`/`GlassCatalog` pattern exactly
//! (same struct shape, same `HashMap<String, T>` catalog, same
//! `from_*`/`get` API). `reflectivity_pct` feeds
//! `fabrication::throughput`'s `system_throughput_pct = (reflectivity_pct /
//! 100)^n_mirrors` (v2-coatings-throughput.md).

use std::collections::HashMap;

use crate::error::OptError;

/// A single mirror coating's representative visual-band (~550 nm)
/// reflectivity per single reflection.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Coating {
    pub name: String,
    /// Representative visual-band reflectivity, percent (0-100).
    pub reflectivity_pct: f64,
}

/// Catalog of named coatings, keyed by `Coating::name` for O(1) lookup.
#[derive(Debug, Clone, Default)]
pub struct CoatingCatalog {
    coatings: HashMap<String, Coating>,
}

impl CoatingCatalog {
    /// Build a catalog from a flat list of coatings, keyed by their `name`.
    pub fn from_coatings(coatings: Vec<Coating>) -> Self {
        Self {
            coatings: coatings.into_iter().map(|c| (c.name.clone(), c)).collect(),
        }
    }

    /// Look up a coating by name, or `OptError::UnknownCoating` if absent.
    pub fn get(&self, name: &str) -> Result<&Coating, OptError> {
        self.coatings
            .get(name)
            .ok_or_else(|| OptError::UnknownCoating(name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap as StdHashMap;

    /// Mirrors `config.rs::load_glasses`'s TOML-loader shape (not yet a
    /// shared `config.rs` loader for coatings — this test parses the real
    /// `configs/coatings.toml` directly to prove the schema round-trips).
    #[derive(Debug, Clone, serde::Deserialize)]
    struct CoatingEntry {
        reflectivity_pct: f64,
    }

    #[derive(Debug, Clone, serde::Deserialize)]
    struct CoatingsFile {
        coatings: StdHashMap<String, CoatingEntry>,
    }

    fn load_real_catalog() -> CoatingCatalog {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("configs/coatings.toml");
        let raw = std::fs::read_to_string(&path).expect("configs/coatings.toml must exist");
        let file: CoatingsFile = toml::from_str(&raw).expect("configs/coatings.toml must parse");
        let coatings: Vec<Coating> = file
            .coatings
            .into_iter()
            .map(|(name, e)| Coating {
                name,
                reflectivity_pct: e.reflectivity_pct,
            })
            .collect();
        CoatingCatalog::from_coatings(coatings)
    }

    #[test]
    fn real_catalog_parses_and_known_coatings_are_sane() {
        let cat = load_real_catalog();

        let enhanced = cat
            .get("enhanced_aluminum")
            .expect("enhanced_aluminum must be in coatings.toml");
        assert!((70.0..=99.9).contains(&enhanced.reflectivity_pct));
        assert!((enhanced.reflectivity_pct - 94.0).abs() < 1e-9);

        let silver = cat
            .get("protected_silver")
            .expect("protected_silver must be in coatings.toml");
        assert!((70.0..=99.9).contains(&silver.reflectivity_pct));
        assert!((silver.reflectivity_pct - 97.5).abs() < 1e-9);
    }

    #[test]
    fn real_catalog_missing_name_returns_unknown_coating_error() {
        let cat = load_real_catalog();
        let err = cat.get("nonexistent").expect_err("nonexistent was not inserted");
        assert!(matches!(err, OptError::UnknownCoating(name) if name == "nonexistent"));
    }

    fn test_catalog() -> CoatingCatalog {
        CoatingCatalog::from_coatings(vec![
            Coating {
                name: "bare_aluminum".to_string(),
                reflectivity_pct: 88.0,
            },
            Coating {
                name: "dielectric".to_string(),
                reflectivity_pct: 99.0,
            },
        ])
    }

    #[test]
    fn get_returns_matching_coating() {
        let cat = test_catalog();
        let c = cat.get("dielectric").expect("dielectric should be in catalog");
        assert_eq!(c.name, "dielectric");
        assert!((c.reflectivity_pct - 99.0).abs() < 1e-9);
    }

    #[test]
    fn get_missing_name_returns_unknown_coating_error() {
        let cat = test_catalog();
        let err = cat.get("gold").expect_err("gold was not inserted");
        assert!(matches!(err, OptError::UnknownCoating(name) if name == "gold"));
    }
}
