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
        substrate: None,
        lightweight_fraction: None,
        coating: None,
        thickness_ratio: None,
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
        substrate: None,
        lightweight_fraction: None,
        coating: None,
        thickness_ratio: None,
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
        substrate: None,
        lightweight_fraction: None,
        coating: None,
        thickness_ratio: None,
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

/// v2 Chunk V1 acceptance check (2): the 4 new v2 `Option` fields
/// (`substrate`, `lightweight_fraction`, `coating`, `thickness_ratio`)
/// parse correctly from a TOML literal via `toml::from_str`, alongside
/// every v1 field, and an entirely v1-shaped request (no v2 keys at all)
/// still parses with all 4 defaulting to `None`.
#[test]
fn request_parses_v2_fabrication_fields_from_toml() {
    const TOML_WITH_V2: &str = r#"
        design = "rc"
        aperture_mm = 200.0
        f_ratio = 8.0
        primary_f_ratio = 3.0
        backfocus_mm = 150.0
        substrate = "pyrex"
        lightweight_fraction = 0.3
        coating = "enhanced_aluminum"
        thickness_ratio = 6.0
    "#;
    let req: Request = toml::from_str(TOML_WITH_V2).expect("valid v2-shaped request TOML");
    assert_eq!(req.substrate.as_deref(), Some("pyrex"));
    assert_eq!(req.lightweight_fraction, Some(0.3));
    assert_eq!(req.coating.as_deref(), Some("enhanced_aluminum"));
    assert_eq!(req.thickness_ratio, Some(6.0));

    const TOML_V1_ONLY: &str = r#"
        design = "newtonian"
        aperture_mm = 200.0
        f_ratio = 6.0
        focuser_backfocus_mm = 220.0
        field_diam_mm = 20.0
    "#;
    let req_v1: Request = toml::from_str(TOML_V1_ONLY).expect("valid v1-shaped request TOML");
    assert!(req_v1.substrate.is_none());
    assert!(req_v1.lightweight_fraction.is_none());
    assert!(req_v1.coating.is_none());
    assert!(req_v1.thickness_ratio.is_none());
}
