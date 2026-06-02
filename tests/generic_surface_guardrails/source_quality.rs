//! Source-quality guardrails for focused modules and readable public models.

use std::{fs, path::PathBuf};

use super::{relative_path, rust_sources_under};

fn public_prelude_source(manifest_dir: &std::path::Path) -> String {
    let root = manifest_dir.join("src/prelude.rs");
    let mut source = fs::read_to_string(&root).expect("public prelude module should be readable");
    let prelude_dir = manifest_dir.join("src/prelude");
    for group in [
        "application.rs",
        "gui.rs",
        "layout.rs",
        "runtime.rs",
        "theme.rs",
        "widgets.rs",
    ] {
        source.push('\n');
        source.push_str(
            &fs::read_to_string(prelude_dir.join(group)).unwrap_or_else(|err| {
                panic!("prelude export group {group} should be readable: {err}")
            }),
        );
    }
    source
}

#[test]
fn public_prelude_stays_grouped_by_subsystem() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/prelude.rs"))
        .expect("public prelude module should be readable");

    for module in [
        "application",
        "gui",
        "layout",
        "runtime",
        "theme",
        "widgets",
    ] {
        assert!(
            root.contains(&format!("mod {module};"))
                && root.contains(&format!("pub use {module}::*;")),
            "src/prelude.rs should stay a small facade over the {module} export group"
        );
        assert!(
            manifest_dir
                .join("src/prelude")
                .join(format!("{module}.rs"))
                .is_file(),
            "prelude export group should live at src/prelude/{module}.rs"
        );
    }

    assert!(
        !root.contains("pub use crate::application::{")
            && !root.contains("pub use crate::runtime::{")
            && !root.contains("pub use crate::widgets::{"),
        "large subsystem export lists belong in src/prelude/<group>.rs, not src/prelude.rs"
    );
}

#[path = "source_quality/api_models.rs"]
mod api_models;
#[path = "source_quality/error_handling.rs"]
mod error_handling;
#[path = "source_quality/feedback_and_platform.rs"]
mod feedback_and_platform;
#[path = "source_quality/layout_runtime.rs"]
mod layout_runtime;
#[path = "source_quality/runtime.rs"]
mod runtime;
#[path = "source_quality/widgets.rs"]
mod widgets;
