//! User request struct + per-design field validation.
//!
//! Deserialized from a user-supplied TOML file (architecture.md §2c), e.g.
//! `configs/examples/newtonian-f5-1000mm.toml` (Chunk 16). Field names match
//! architecture.md §2c and `designs.toml`'s `required`/`optional` lists
//! (config.rs) exactly, since `validate` matches strings from `DesignDef`
//! against this struct's field presence.
//!
//! Two fields were added beyond architecture.md §2c per
//! formula-validation.md finding F2 (a resolved blocker): `primary_f_ratio`
//! and `backfocus_mm`, required by the rc/cassegrain pipeline. Architecture
//! §2a/§2c never listed them; `designs.toml`'s `rc`/`cassegrain` entries add
//! them to `required` (Chunk 15) and this struct must accept them (here).

use crate::config::DesignDef;
use crate::error::OptError;

/// The user's telescope spec request — one field per possible design input.
/// Only `design`, `aperture_mm`, `f_ratio` are universally required; every
/// other field is `Option` because different design pipelines need
/// different subsets (`DesignDef.required`/`optional` drive that, not this
/// struct's types). `Request::validate` is the actual gate.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Request {
    /// Key into `designs.toml`'s `[designs.<name>]` map, e.g. `"newtonian"`,
    /// `"rc"`, `"cassegrain"`, `"quintuplet"`.
    pub design: String,
    /// Primary aperture / mirror or objective diameter, mm.
    pub aperture_mm: f64,
    /// System focal ratio (F/D of the whole system, not the primary alone —
    /// see `primary_f_ratio` below for the RC/Cassegrain primary's own N).
    pub f_ratio: f64,

    /// Newtonian only. **Semantic (formula-validation.md A1, common
    /// misunderstanding):** this is the FULL secondary-center-to-focal-plane
    /// distance L — tube radius + wall thickness + focuser stack + desired
    /// in-travel, i.e. exactly what an ATM diagonal-mirror calculator asks
    /// for as "diagonal to focal plane distance". It is NOT drawtube-only
    /// travel; treating it as such undersizes the secondary by 2-4x. Sanity:
    /// L < aperture/2 is physically impossible (the light path alone needs
    /// more clearance than the primary's radius to exit the tube) —
    /// `designs/newtonian.rs` (Chunk 11) is responsible for that warning.
    #[serde(default)]
    pub focuser_backfocus_mm: Option<f64>,

    /// Newtonian only. Fully-illuminated field diameter, mm (drives
    /// secondary sizing and illumination falloff). No serde default value —
    /// if unset, `designs/newtonian.rs` applies the documented ATM default
    /// of 20.0 mm; this struct only records what the user actually gave.
    #[serde(default)]
    pub field_diam_mm: Option<f64>,

    /// RC/Cassegrain only, per formula-validation.md F2/A2. The PRIMARY
    /// mirror's own focal ratio (f1/D), distinct from `f_ratio` (system
    /// F/D). Needed to recover f1 = aperture_mm * primary_f_ratio and hence
    /// the secondary magnification m = f_ratio / primary_f_ratio.
    #[serde(default)]
    pub primary_f_ratio: Option<f64>,

    /// RC/Cassegrain only, per formula-validation.md F2/A2. Backfocus
    /// measured from the PRIMARY MIRROR VERTEX to the focal plane
    /// (Schroeder §6.2 convention `b`), positive behind the primary. Not
    /// measured from the secondary or the tube's rear cell.
    #[serde(default)]
    pub backfocus_mm: Option<f64>,

    /// Optional (any design with a `fov`/`plate_scale` output). Eyepiece
    /// field-stop diameter, mm, for true-FOV calculation.
    #[serde(default)]
    pub eyepiece_fieldstop_mm: Option<f64>,

    /// Optional (any design with a `fov`/`plate_scale` output). Sensor
    /// diagonal, mm, for imaging FOV/plate-scale calculation.
    #[serde(default)]
    pub sensor_diag_mm: Option<f64>,

    /// Optional (any design with a `diffraction` output). Design
    /// wavelength, nm. No serde default value — `common.rs`'s diffraction
    /// calls apply the documented ATM default of 550.0 nm (visual green)
    /// when unset.
    #[serde(default)]
    pub wavelength_nm: Option<f64>,

    /// Optional file export path, wired into `--export` handling in
    /// Chunk 17's CLI layer. Not read by any pipeline module.
    #[serde(default)]
    pub output_path: Option<String>,
}

impl Request {
    /// Check every field `def.required` names is present on this request.
    /// Returns the FIRST missing field as `OptError::MissingField` (matches
    /// `error.rs`'s single-field variant shape, not a `Vec<String>`).
    ///
    /// Field-name -> presence is a plain match (ponytail: no reflection/derive
    /// macro for ~10 known names — a match is simpler and more explicit).
    pub fn validate(&self, def: &DesignDef) -> Result<(), OptError> {
        for field in &def.required {
            if !self.has_field(field) {
                return Err(OptError::MissingField {
                    design: self.design.clone(),
                    field: field.clone(),
                });
            }
        }
        crate::request_validate::validate_values(self)
    }

    /// Whether `field` (a `DesignDef.required`/`optional` entry name) has a
    /// value on this request. `design`/`aperture_mm`/`f_ratio` are
    /// non-optional Rust fields so they always count as present once the
    /// struct deserialized at all. An unrecognized name (a `designs.toml`
    /// typo or a field this struct hasn't been taught yet) counts as
    /// missing rather than silently passing — a config bug should surface
    /// as a validation error, not be ignored.
    fn has_field(&self, field: &str) -> bool {
        match field {
            "design" => !self.design.is_empty(),
            "aperture_mm" => true,
            "f_ratio" => true,
            "focuser_backfocus_mm" => self.focuser_backfocus_mm.is_some(),
            "field_diam_mm" => self.field_diam_mm.is_some(),
            "primary_f_ratio" => self.primary_f_ratio.is_some(),
            "backfocus_mm" => self.backfocus_mm.is_some(),
            "eyepiece_fieldstop_mm" => self.eyepiece_fieldstop_mm.is_some(),
            "sensor_diag_mm" => self.sensor_diag_mm.is_some(),
            "wavelength_nm" => self.wavelength_nm.is_some(),
            "output_path" => self.output_path.is_some(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal inline `DesignDef` matching a Newtonian's required list
    /// (architecture.md §2a / config.rs's inline test schema) — no TOML
    /// parse needed here, just the struct `Request::validate` consumes.
    fn newtonian_def() -> DesignDef {
        toml::from_str(
            r#"
                pipeline = "newtonian"
                elements = 2
                kind = "reflector"
                required = ["aperture_mm", "f_ratio", "focuser_backfocus_mm", "field_diam_mm"]
                optional = ["sensor_diag_mm", "eyepiece_fieldstop_mm"]
                outputs = ["basics"]
            "#,
        )
        .expect("valid inline DesignDef")
    }

    /// Minimal inline `DesignDef` matching formula-validation.md F2's
    /// rc/cassegrain resolution — required list includes the two new fields.
    fn rc_def() -> DesignDef {
        toml::from_str(
            r#"
                pipeline = "cassegrain"
                elements = 2
                kind = "reflector"
                required = ["aperture_mm", "f_ratio", "primary_f_ratio", "backfocus_mm"]
                outputs = ["basics"]
            "#,
        )
        .expect("valid inline DesignDef")
    }

    fn newtonian_request(focuser_backfocus_mm: Option<f64>, field_diam_mm: Option<f64>) -> Request {
        Request {
            design: "newtonian".to_string(),
            aperture_mm: 200.0,
            f_ratio: 6.0,
            focuser_backfocus_mm,
            field_diam_mm,
            primary_f_ratio: None,
            backfocus_mm: None,
            eyepiece_fieldstop_mm: None,
            sensor_diag_mm: None,
            wavelength_nm: None,
            output_path: None,
        }
    }

    #[test]
    fn validate_ok_when_all_required_fields_present() {
        let req = newtonian_request(Some(220.0), Some(20.0));
        assert!(req.validate(&newtonian_def()).is_ok());
    }

    #[test]
    fn validate_errs_on_first_missing_required_field() {
        // field_diam_mm missing -> MissingField naming it, not focuser_backfocus_mm
        let req = newtonian_request(Some(220.0), None);
        let err = req.validate(&newtonian_def()).expect_err("must fail");
        match err {
            OptError::MissingField { design, field } => {
                assert_eq!(design, "newtonian");
                assert_eq!(field, "field_diam_mm");
            }
            other => panic!("expected MissingField, got {other:?}"),
        }
    }

    #[test]
    fn validate_errs_when_backfocus_missing_for_newtonian() {
        let req = newtonian_request(None, Some(20.0));
        let err = req.validate(&newtonian_def()).expect_err("must fail");
        match err {
            OptError::MissingField { field, .. } => assert_eq!(field, "focuser_backfocus_mm"),
            other => panic!("expected MissingField, got {other:?}"),
        }
    }

    #[test]
    fn rc_request_with_f2_fields_validates_ok() {
        // Confirms primary_f_ratio/backfocus_mm exist and satisfy an
        // RC-style DesignDef's required list (formula-validation.md F2).
        let req = Request {
            design: "rc".to_string(),
            aperture_mm: 200.0,
            f_ratio: 8.0,
            focuser_backfocus_mm: None,
            field_diam_mm: None,
            primary_f_ratio: Some(3.0),
            backfocus_mm: Some(150.0),
            eyepiece_fieldstop_mm: None,
            sensor_diag_mm: None,
            wavelength_nm: None,
            output_path: None,
        };
        assert!(req.validate(&rc_def()).is_ok());
    }

    #[test]
    fn rc_request_missing_backfocus_errs_naming_it() {
        let req = Request {
            design: "rc".to_string(),
            aperture_mm: 200.0,
            f_ratio: 8.0,
            focuser_backfocus_mm: None,
            field_diam_mm: None,
            primary_f_ratio: Some(3.0),
            backfocus_mm: None,
            eyepiece_fieldstop_mm: None,
            sensor_diag_mm: None,
            wavelength_nm: None,
            output_path: None,
        };
        let err = req.validate(&rc_def()).expect_err("must fail");
        match err {
            OptError::MissingField { design, field } => {
                assert_eq!(design, "rc");
                assert_eq!(field, "backfocus_mm");
            }
            other => panic!("expected MissingField, got {other:?}"),
        }
    }

    #[test]
    fn unknown_required_field_name_counts_as_missing() {
        // A designs.toml typo/unknown field must fail validation, not be
        // silently ignored.
        let def: DesignDef = toml::from_str(
            r#"
                pipeline = "newtonian"
                elements = 2
                kind = "reflector"
                required = ["totally_unknown_field"]
                outputs = ["basics"]
            "#,
        )
        .expect("valid inline DesignDef");
        let req = newtonian_request(Some(220.0), Some(20.0));
        let err = req.validate(&def).expect_err("must fail on unknown field");
        assert!(matches!(err, OptError::MissingField { field, .. } if field == "totally_unknown_field"));
    }
}
