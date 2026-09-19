//! Chunk 16 acceptance check: every shipped `configs/examples/*.toml`
//! parses into a `Request` and passes `Request::validate` against its
//! matching real `configs/designs.toml` entry — no `MissingField` errors.
//! This is a `#[cfg(test)]`-only module (see `main.rs`'s mod decl) so it
//! never ships in the release binary; it exists purely to catch an example
//! file missing a required field before a real user hits it.

#[cfg(test)]
mod tests {
    use crate::config::load_designs;
    use crate::request::Request;
    use std::path::Path;

    /// Load the real `configs/designs.toml`, parse `example_file` (a path
    /// under `configs/examples/`), and assert `Request::validate` passes
    /// against the design entry named by the file's own `design` key.
    fn assert_example_validates(example_file: &str) {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let designs = load_designs(&manifest_dir.join("configs/designs.toml"))
            .expect("real designs.toml must parse");

        let raw = std::fs::read_to_string(manifest_dir.join("configs/examples").join(example_file))
            .unwrap_or_else(|e| panic!("must read {example_file}: {e}"));
        let req: Request = toml::from_str(&raw).unwrap_or_else(|e| panic!("must parse {example_file}: {e}"));

        let def = designs
            .designs
            .get(&req.design)
            .unwrap_or_else(|| panic!("{example_file}: design '{}' not in designs.toml", req.design));

        req.validate(def)
            .unwrap_or_else(|e| panic!("{example_file} failed validation: {e}"));
    }

    #[test]
    fn newtonian_f5_1000mm_example_validates() {
        assert_example_validates("newtonian-f5-1000mm.toml");
    }

    #[test]
    fn quintuplet_f6_apo_example_validates() {
        assert_example_validates("quintuplet-f6-apo.toml");
    }

    #[test]
    fn rc_f8_example_validates() {
        assert_example_validates("rc-f8.toml");
    }
}
