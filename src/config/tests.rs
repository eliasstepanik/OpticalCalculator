use super::*;

/// Minimal inline schema matching architecture.md §2a — real
/// `configs/designs.toml` lands in Chunk 15; this round-trips the type
/// shape only, it is not the production config.
const DESIGNS_TOML: &str = r#"
    [designs.newtonian]
    pipeline = "newtonian"
    elements = 2
    kind = "reflector"
    required = ["aperture_mm", "f_ratio", "focuser_backfocus_mm", "field_diam_mm"]
    optional = ["sensor_diag_mm", "eyepiece_fieldstop_mm"]
    outputs = ["basics", "newt_secondary", "illumination", "diffraction", "plate_scale", "fov", "conic"]
    [designs.newtonian.conic]
    primary = -1.0

    [designs.quintuplet_apo]
    pipeline = "refractor"
    elements = 5
    kind = "refractor"
    required = ["aperture_mm", "f_ratio"]
    outputs = ["basics", "diffraction", "plate_scale", "fov", "prescription"]
    glasses = ["FPL53", "FPL53", "BK7", "F2", "BK7"]
    power_split = [0.4, 0.1, -0.2, -0.2, 0.1]
"#;

#[test]
fn designs_file_round_trips_inline_schema() {
    let file: DesignsFile = toml::from_str(DESIGNS_TOML).expect("valid designs TOML");
    assert_eq!(file.designs.len(), 2);

    let newt = &file.designs["newtonian"];
    assert_eq!(newt.pipeline, "newtonian");
    assert_eq!(newt.elements, 2);
    assert_eq!(newt.kind, "reflector");
    assert_eq!(newt.required, vec!["aperture_mm", "f_ratio", "focuser_backfocus_mm", "field_diam_mm"]);
    assert_eq!(newt.optional, vec!["sensor_diag_mm", "eyepiece_fieldstop_mm"]);
    assert!(newt.outputs.contains(&"conic".to_string()));
    let conic = newt.conic.as_ref().expect("newtonian has a fixed conic");
    assert!((conic.primary - (-1.0)).abs() < 1e-12);
    assert!(newt.conic_mode.is_none());
    assert!(newt.glasses.is_none());

    let quint = &file.designs["quintuplet_apo"];
    assert_eq!(quint.pipeline, "refractor");
    assert_eq!(quint.elements, 5);
    assert!(quint.optional.is_empty()); // no `optional` key -> serde default
    assert_eq!(
        quint.glasses,
        Some(vec!["FPL53".to_string(), "FPL53".to_string(), "BK7".to_string(), "F2".to_string(), "BK7".to_string()])
    );
    let split = quint.power_split.as_ref().expect("quintuplet needs power_split");
    assert_eq!(split.len(), 5);

    // v2 Chunk V1: absent `n_mirrors`/`fab_applicable` in a v1-shaped
    // config must default to `None`, not error or default to `Some(...)`.
    assert!(newt.n_mirrors.is_none());
    assert!(newt.fab_applicable.is_none());
}

/// v2 Chunk V1: `n_mirrors`/`fab_applicable` parse correctly when present
/// (architecture-v2.md §2a `[designs.rc]` example), and stay additive — a
/// design entry with them still round-trips every v1 field unchanged.
#[test]
fn design_def_parses_v2_n_mirrors_and_fab_applicable() {
    const RC_TOML: &str = r#"
        [designs.rc]
        pipeline = "cassegrain"
        elements = 2
        kind = "reflector"
        required = ["aperture_mm", "f_ratio", "primary_f_ratio", "backfocus_mm"]
        outputs = ["basics"]
        conic_mode = "rc"
        n_mirrors = 2
        fab_applicable = ["substrate", "tolerance", "throughput"]
    "#;
    let file: DesignsFile = toml::from_str(RC_TOML).expect("valid v2-shaped designs TOML");
    let rc = &file.designs["rc"];
    assert_eq!(rc.pipeline, "cassegrain");
    assert_eq!(rc.conic_mode.as_deref(), Some("rc"));
    assert_eq!(rc.n_mirrors, Some(2));
    assert_eq!(
        rc.fab_applicable,
        Some(vec!["substrate".to_string(), "tolerance".to_string(), "throughput".to_string()])
    );
}

#[test]
fn load_designs_on_missing_path_returns_err_not_panic() {
    let err = load_designs(Path::new("does/not/exist-designs.toml"))
        .expect_err("nonexistent path must error");
    assert!(matches!(err, OptError::Io { .. }));
}

#[test]
fn load_glasses_on_missing_path_returns_err_not_panic() {
    let err = load_glasses(Path::new("does/not/exist-glasses.toml"))
        .expect_err("nonexistent path must error");
    assert!(matches!(err, OptError::Io { .. }));
}

/// End-to-end: the real `configs/glasses.toml` (built in Chunk 4) loads
/// through the actual filesystem loader into a working `GlassCatalog`.
#[test]
fn load_glasses_loads_real_catalog_end_to_end() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("configs/glasses.toml");
    let catalog = load_glasses(&path).expect("real glasses.toml must parse and convert");

    let bk7 = catalog.get("BK7").expect("BK7 must be in the shipped catalog");
    assert_eq!(bk7.name, "BK7");
    assert!((bk7.nd - 1.51680).abs() < 1e-6);
    assert!((bk7.vd - 64.17).abs() < 1e-6);

    let fpl53 = catalog.get("FPL53").expect("FPL53 must be in the shipped catalog");
    assert_eq!(fpl53.name, "FPL53");
    assert!((fpl53.nd - 1.43875).abs() < 1e-6);
    assert!((fpl53.vd - 94.66).abs() < 1e-6);

    // Sanity: catalog isn't accidentally single-entry.
    assert!(catalog.get("F2").is_ok());
}

/// End-to-end (Chunk 15): the real `configs/designs.toml` parses into
/// `DesignsFile` via the actual filesystem loader, all 6 named entries
/// (newtonian/rc/cassegrain/achromat/triplet/quintuplet -- 5 design
/// types, rc+cassegrain sharing one pipeline) have the right
/// `pipeline`/`conic_mode`/`glasses`, and every glass name any entry
/// references actually exists in the real `configs/glasses.toml` (a
/// typo'd glass name here would otherwise only surface as a runtime
/// `UnknownGlass` error).
#[test]
fn load_designs_loads_real_config_end_to_end() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let designs_path = manifest_dir.join("configs/designs.toml");
    let glasses_path = manifest_dir.join("configs/glasses.toml");

    let designs = load_designs(&designs_path).expect("real designs.toml must parse");
    let catalog = load_glasses(&glasses_path).expect("real glasses.toml must parse");

    assert_eq!(designs.designs.len(), 6, "expected 6 named design entries");
    for name in ["newtonian", "rc", "cassegrain", "achromat", "triplet", "quintuplet"] {
        assert!(designs.designs.contains_key(name), "missing design entry: {name}");
    }

    let newt = &designs.designs["newtonian"];
    assert_eq!(newt.pipeline, "newtonian");
    assert_eq!(newt.conic.as_ref().expect("newtonian has conic").primary, -1.0);

    let rc = &designs.designs["rc"];
    assert_eq!(rc.pipeline, "cassegrain");
    assert_eq!(rc.conic_mode.as_deref(), Some("rc"));

    let cass = &designs.designs["cassegrain"];
    assert_eq!(cass.pipeline, "cassegrain");
    assert_eq!(cass.conic_mode.as_deref(), Some("classical"));

    let achromat = &designs.designs["achromat"];
    assert_eq!(achromat.pipeline, "refractor");
    assert_eq!(achromat.glasses.as_deref(), Some(&["BK7".to_string(), "F2".to_string()][..]));
    assert_eq!(achromat.elements, achromat.glasses.as_ref().unwrap().len());

    let triplet = &designs.designs["triplet"];
    assert_eq!(triplet.pipeline, "refractor");
    assert_eq!(
        triplet.glasses.as_deref(),
        Some(&["FPL53".to_string(), "BK7".to_string(), "F2".to_string()][..])
    );
    assert_eq!(triplet.elements, triplet.glasses.as_ref().unwrap().len());

    let quint = &designs.designs["quintuplet"];
    assert_eq!(quint.pipeline, "refractor");
    assert_eq!(
        quint.glasses.as_deref(),
        Some(
            &[
                "FPL53".to_string(),
                "FPL53".to_string(),
                "BK7".to_string(),
                "F2".to_string(),
                "BK7".to_string(),
            ][..]
        )
    );
    assert_eq!(quint.elements, quint.glasses.as_ref().unwrap().len());

    // Cross-check: every glass name any design references must exist in
    // the real glass catalog -- catches a typo'd name that would
    // otherwise only surface as a runtime UnknownGlass error.
    for (design_name, def) in &designs.designs {
        if let Some(glasses) = &def.glasses {
            for glass_name in glasses {
                assert!(
                    catalog.get(glass_name).is_ok(),
                    "design '{design_name}' references unknown glass '{glass_name}'"
                );
            }
        }
    }
}
