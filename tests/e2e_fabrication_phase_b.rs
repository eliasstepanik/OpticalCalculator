//! Black-box end-to-end test for the v2 `--fabrication` pass's Phase B
//! subsystems (baffle #3, support #6, conic-tolerance #4 — V-8 dispatch
//! wiring, [[task-snapshot-v2-phaseB]] VB4 row).
//!
//! Same real-binary-invocation pattern as `tests/e2e_fabrication_phase_a.rs`.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_optical-calculator"))
}

/// Real binary run against `configs/examples/rc-f8.toml --fabrication`:
/// confirms all three Phase B sections (baffle, support, conic tolerance)
/// are populated alongside Phase A's substrate/throughput/surface-tolerance
/// sections — the VB4 acceptance check.
#[test]
fn fabrication_flag_shows_all_phase_a_and_phase_b_sections() {
    let output = bin()
        .arg("configs/examples/rc-f8.toml")
        .arg("--fabrication")
        .output()
        .expect("failed to run binary");

    assert!(
        output.status.success(),
        "expected exit 0, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Phase A sections still present (no regression from V-8's dispatch change).
    assert!(stdout.contains("-- Substrate / Mirror Blank --"), "{stdout}");
    assert!(stdout.contains("-- Optical Throughput --"), "{stdout}");
    assert!(stdout.contains("Surface P-V tol: 68.75 nm (lambda/8)"), "{stdout}");

    // Phase B: conic tolerance (T1 VERIFIED, A2 RC worked example).
    assert!(stdout.contains("Primary conic tol (dK): 0.03802"), "{stdout}");
    assert!(stdout.contains("Secondary conic tol (dK): 0.45676"), "{stdout}");

    // Phase B: baffle diameters (B1 VERIFIED, A2 RC worked example).
    assert!(stdout.contains("-- Baffle / Anti-Vignetting (cassegrain) --"), "{stdout}");
    assert!(stdout.contains("Primary baffle hole:   36.9 mm"), "{stdout}");
    assert!(stdout.contains("Secondary baffle tube: 81.4 mm"), "{stdout}");

    // Phase B: mirror support sag (S1 VERIFIED).
    assert!(stdout.contains("-- Mirror Support (9_point) --"), "{stdout}");
    assert!(stdout.contains("Support points:  9"), "{stdout}");

    // V-9 fix: the stale "T1 pending" note text must be gone.
    assert!(
        !stdout.contains("conic tolerance (T1) pending Phase B validation"),
        "stale T1-pending note must be removed: {stdout}"
    );
}

/// V-9 regression guard: the exact stale note string from the old
/// dispatch must never reappear anywhere in stdout.
#[test]
fn stale_t1_pending_note_never_appears() {
    let output = bin()
        .arg("configs/examples/rc-f8.toml")
        .arg("--fabrication")
        .output()
        .expect("failed to run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("pending Phase B validation"), "{stdout}");
}
