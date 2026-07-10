//! Source-quality guardrails for focused modules and readable public models.

use std::{collections::BTreeSet, fs, path::PathBuf};

use super::{relative_path, rust_sources_under};

const MAX_PRELUDE_EXPORT_GROUP_LINES: usize = 32;
const MAX_COMMON_PRELUDE_NAMED_EXPORTS: usize = 532;

const ADVANCED_COMMON_PRELUDE_EXCLUSIONS: &[&str] = &[
    "AuxiliaryWindow",
    "BusinessRuntimeDiagnostics",
    "CanvasInvalidation",
    "CanvasSelectionGeometry",
    "DeclarativeSurfaceRuntime",
    "DenseGridLayout",
    "FrameCadenceMonitor",
    "GpuSurfaceContent",
    "GpuSurfaceWidget",
    "HorizontalValueAxis",
    "InvalidationMask",
    "NativeFrameDiagnostics",
    "NativeGenericRunReport",
    "NativeGpuSurfaceDiagnostics",
    "NativeRunOptions",
    "RetainedSegment",
    "RetainedSurfaceCachePolicy",
    "RuntimeRunReport",
    "SurfacePaintPlan",
    "SvgParseError",
    "TimelineViewport",
    "UiSurface",
    "VerticalValueAxis",
    "WidgetPaint",
    "gpu_surface",
    "push_fill_rect",
    "push_sampled_curve_stroke",
    "retained_canvas",
];

fn public_prelude_source(manifest_dir: &std::path::Path) -> String {
    let root = manifest_dir.join("src/prelude.rs");
    let mut source = fs::read_to_string(&root).expect("public prelude module should be readable");
    let prelude_dir = manifest_dir.join("src/prelude");
    append_prelude_source(&mut source, &prelude_dir);
    source
}

fn append_prelude_source(source: &mut String, dir: &std::path::Path) {
    let mut entries = fs::read_dir(dir)
        .unwrap_or_else(|err| {
            panic!(
                "prelude directory {} should be readable: {err}",
                dir.display()
            )
        })
        .map(|entry| {
            entry
                .expect("prelude directory entry should be readable")
                .path()
        })
        .collect::<Vec<_>>();
    entries.sort();

    for path in entries {
        if path.is_dir() {
            append_prelude_source(source, &path);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        source.push('\n');
        source.push_str(&fs::read_to_string(&path).unwrap_or_else(|err| {
            panic!(
                "prelude export group {} should be readable: {err}",
                path.display()
            )
        }));
    }
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

#[test]
fn public_prelude_first_level_groups_stay_as_facades_when_split() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    for module in ["application", "gui", "runtime"] {
        let source_path = manifest_dir
            .join("src/prelude")
            .join(format!("{module}.rs"));
        let source = fs::read_to_string(&source_path).unwrap_or_else(|err| {
            panic!(
                "prelude facade {} should be readable: {err}",
                source_path.display()
            )
        });

        assert!(
            source.contains("mod ") && source.contains("pub use "),
            "split prelude group {module} should stay a facade over focused child export modules"
        );
        assert!(
            !source.contains("pub use crate::"),
            "split prelude group {module} should not rebuild a broad crate export list; add exports to focused child modules under src/prelude/{module}/"
        );
    }
}

#[test]
fn public_prelude_export_groups_stay_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let prelude_dir = manifest_dir.join("src/prelude");

    let oversized = rust_sources_under(&prelude_dir)
        .into_iter()
        .filter(|path| path != &manifest_dir.join("src/prelude.rs"))
        .filter_map(|path| {
            let source = fs::read_to_string(&path).unwrap_or_else(|err| {
                panic!(
                    "prelude export group {} should be readable: {err}",
                    path.display()
                )
            });
            let line_count = source.lines().count();
            (line_count > MAX_PRELUDE_EXPORT_GROUP_LINES).then(|| {
                format!(
                    "{} ({line_count} lines)",
                    relative_path(&manifest_dir, &path)
                )
            })
        })
        .collect::<Vec<_>>();

    assert!(
        oversized.is_empty(),
        "prelude export groups should stay small enough to scan; split broad groups before they rebuild the old giant import list:\n{}",
        oversized.join("\n")
    );
}

#[test]
fn public_prelude_guardrails_scan_root_and_leaf_sources() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let statements = explicit_prelude_export_statements(&manifest_dir);

    assert!(
        statements
            .iter()
            .any(|statement| statement == "pub use application::*;"),
        "prelude export guardrails must scan the src/prelude.rs root facade"
    );
    assert!(
        statements
            .iter()
            .any(|statement| statement.contains("StatefulAppBuilder")),
        "prelude export guardrails must scan focused leaf export groups"
    );
}

#[test]
fn public_prelude_named_surface_stays_bounded_across_leaf_splits() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let statements = explicit_prelude_export_statements(&manifest_dir);
    let named_export_count = statements
        .iter()
        .filter(|statement| !statement.contains("::*"))
        .map(|statement| {
            statement
                .split_once('{')
                .and_then(|(_, tail)| tail.rsplit_once('}').map(|(items, _)| items))
                .map_or(1, |items| {
                    items
                        .split(',')
                        .filter(|item| !item.trim().is_empty())
                        .count()
                })
        })
        .sum::<usize>();

    assert!(
        named_export_count <= MAX_COMMON_PRELUDE_NAMED_EXPORTS,
        "radiant::prelude exposes {named_export_count} named items across its root and leaf modules; \
         keep the reviewed common surface at or below {MAX_COMMON_PRELUDE_NAMED_EXPORTS} and \
         move specialist APIs to their owning public modules"
    );
}

#[test]
fn public_prelude_rejects_crate_owner_wildcards() {
    assert!(prelude_owner_wildcard_export("pub use crate::runtime::*;"));
    assert!(prelude_owner_wildcard_export("pub use crate::widgets::*;"));
    assert!(prelude_owner_wildcard_export(
        "pub use crate::runtime::{self, *};"
    ));
    assert!(!prelude_owner_wildcard_export("pub use runtime::*;"));

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let offenders = explicit_prelude_export_statements(&manifest_dir)
        .into_iter()
        .filter(|statement| prelude_owner_wildcard_export(statement))
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "prelude roots, facades, and leaves must not wildcard-export owning crate modules; \
         keep local facade globs focused and name crate-owned exports explicitly:\n{}",
        offenders.join("\n")
    );
}

fn prelude_owner_wildcard_export(statement: &str) -> bool {
    statement.starts_with("pub use crate::") && statement.contains('*')
}

#[test]
fn public_prelude_excludes_advanced_host_paint_and_visualization_apis() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let statements = explicit_prelude_export_statements(&manifest_dir);
    let offenders = ADVANCED_COMMON_PRELUDE_EXCLUSIONS
        .iter()
        .filter(|name| {
            statements.iter().any(|statement| {
                statement
                    .split(|character: char| {
                        !(character.is_ascii_alphanumeric() || character == '_')
                    })
                    .any(|token| token == **name)
            })
        })
        .copied()
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "advanced APIs must require imports from radiant::runtime, radiant::gui, or \
         radiant::widgets instead of leaking through radiant::prelude: {}",
        offenders.join(", ")
    );
}

fn explicit_prelude_export_statements(manifest_dir: &std::path::Path) -> Vec<String> {
    std::iter::once(manifest_dir.join("src/prelude.rs"))
        .chain(rust_sources_under(&manifest_dir.join("src/prelude")))
        .flat_map(|path| {
            let source = fs::read_to_string(&path).unwrap_or_else(|err| {
                panic!(
                    "prelude export group {} should be readable: {err}",
                    path.display()
                )
            });
            let mut statements = Vec::new();
            let mut current = None::<String>;
            for line in source.lines() {
                let trimmed = line.trim();
                if current.is_none() && trimmed.starts_with("pub use ") {
                    current = Some(trimmed.to_owned());
                } else if let Some(statement) = current.as_mut() {
                    statement.push(' ');
                    statement.push_str(trimmed);
                }
                if trimmed.ends_with(';')
                    && let Some(statement) = current.take()
                {
                    statements.push(statement);
                }
            }
            statements
        })
        .collect()
}

#[test]
fn public_prelude_leaf_groups_do_not_wildcard_export_owning_subsystems() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let prelude_dir = manifest_dir.join("src/prelude");
    let facade_files = BTreeSet::from([
        manifest_dir.join("src/prelude.rs"),
        prelude_dir.join("application.rs"),
        prelude_dir.join("gui.rs"),
        prelude_dir.join("runtime.rs"),
    ]);

    let offenders = rust_sources_under(&prelude_dir)
        .into_iter()
        .filter(|path| !facade_files.contains(path))
        .flat_map(|path| {
            let source = fs::read_to_string(&path).unwrap_or_else(|err| {
                panic!(
                    "prelude export group {} should be readable: {err}",
                    path.display()
                )
            });
            let relative = relative_path(&manifest_dir, &path);
            source
                .lines()
                .filter(|line| prelude_leaf_wildcard_export(line))
                .map(move |line| format!("{relative}: {}", line.trim()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "prelude leaf modules must name exported items instead of wildcard-exporting \
         owning subsystems; use a first-level facade for child modules only:\n{}",
        offenders.join("\n")
    );
}

fn prelude_leaf_wildcard_export(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("pub use ") && trimmed.contains("::*")
}

#[test]
fn public_prelude_stays_backend_neutral() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = public_prelude_source(&manifest_dir);

    let forbidden = [
        "macroquad",
        "native_vello",
        "vello",
        "wgpu",
        "winit",
        "windows::",
        "windows_core",
        "windows_sys",
        "windows-sys",
    ];
    let offenders = forbidden
        .into_iter()
        .filter(|token| source.contains(token))
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "radiant::prelude should stay backend-neutral; renderer, windowing, and platform-specific APIs belong on explicit modules, not common imports:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn direct_state_callback_api_stays_out_of_common_surface() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let offenders = rust_sources_under(&manifest_dir.join("src"))
        .into_iter()
        .filter_map(|path| {
            let source = fs::read_to_string(&path).unwrap_or_else(|err| {
                panic!("source file {} should be readable: {err}", path.display())
            });
            let relative = relative_path(&manifest_dir, &path);
            direct_state_callback_offense(&relative, &source)
        })
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "direct state-callback APIs should not be exported or exposed through \
         common Radiant builders:\n{}",
        offenders.join("\n")
    );
}

fn direct_state_callback_offense(relative: &str, source: &str) -> Option<String> {
    let state_action = ["State", "Action"].concat();
    let state_view = ["State", "View"].concat();
    let application_compatibility = ["application::", "compatibility"].concat();
    let compatibility_state_action = ["compatibility::", &state_action].concat();

    if source.contains(&state_action)
        || source.contains(&state_view)
        || source.contains(&application_compatibility)
        || source.contains(&compatibility_state_action)
    {
        return Some(relative.to_owned());
    }
    None
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
