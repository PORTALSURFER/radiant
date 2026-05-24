use super::*;

#[test]
fn public_module_tree_exposes_one_progressive_api_surface() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("Radiant lib.rs should be readable");
    let public_modules = public_module_names(&lib);
    let expected = BTreeSet::from([
        "gui".to_owned(),
        "gui_runtime".to_owned(),
        "layout".to_owned(),
        "prelude".to_owned(),
        "runtime".to_owned(),
        "theme".to_owned(),
        "widgets".to_owned(),
    ]);

    assert_eq!(
        public_modules, expected,
        "Radiant's crate root should expose only generic public modules"
    );
    assert!(
        !manifest_dir.join("src/compat.rs").exists()
            && rust_sources_under(&manifest_dir.join("src/compat")).is_empty(),
        "compatibility adapter source files belong outside the generic Radiant crate"
    );
}

fn public_module_names(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub mod "))
        .filter_map(|tail| tail.split([';', '{']).next())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}
