//! Guardrails for the generic Radiant public surface.
//!
//! Generic modules are allowed to use backend-neutral Radiant primitives only:
//! `radiant::layout`, `radiant::widgets`, `radiant::runtime`, `radiant::theme`,
//! and the shared non-shell `gui` primitives those APIs expose. The current
//! Sempal shell remains a transitional compatibility exception under
//! `compat::sempal_shell`, `app`, `gui::native_shell`, and the native Vello
//! compatibility runtime.

use std::{
    fs,
    path::{Path, PathBuf},
};

const GENERIC_SOURCE_ROOTS: &[&str] = &[
    "src/runtime",
    "src/widgets",
    "src/theme.rs",
    "src/gui/layout_core",
];

const COMPAT_INTEGRATION_TESTS: &[&str] = &[
    "compat_sempal_shell_public_api.rs",
    "compat_status_bar_pilot.rs",
];

const FORBIDDEN_GENERIC_TOKENS: &[&str] = &[
    "crate::app",
    "crate::{app",
    "crate::sempal_app",
    "crate::{sempal_app",
    "crate::compat::sempal_shell",
    "crate::{compat::sempal_shell",
    "compat::sempal_shell",
    "crate::gui::native_shell",
    "crate::{gui::native_shell",
    "gui::native_shell",
    "crate::gui_runtime::native_vello",
    "crate::{gui_runtime::native_vello",
    "gui_runtime::native_vello",
    "native_shell",
    "AppModel",
    "UiAction",
];

const FORBIDDEN_GENERIC_TEST_TOKENS: &[&str] = &[
    "radiant::app",
    "radiant::{app",
    "radiant::compat::sempal_shell",
    "radiant::{compat::sempal_shell",
    "compat::sempal_shell",
    "Sempal",
    "sempal",
    "capture_gui_automation_snapshot",
    "capture_native_shell_shot_snapshot",
];

#[test]
fn generic_sources_do_not_import_sempal_shell_contracts() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();

    for root in GENERIC_SOURCE_ROOTS {
        collect_violations(&manifest_dir.join(root), &manifest_dir, &mut violations);
    }

    assert!(
        violations.is_empty(),
        "generic Radiant modules must stay independent from Sempal compatibility contracts; \
         move transitional shell code under app, compat::sempal_shell, gui::native_shell, or gui_runtime/native_vello:\n{}",
        violations.join("\n")
    );
}

#[test]
fn generic_integration_tests_do_not_reintroduce_sempal_shell_fixtures() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tests_dir = manifest_dir.join("tests");
    let mut violations = Vec::new();

    assert!(
        !tests_dir.join("shots").exists(),
        "Sempal visual snapshot fixtures belong in the host app test tree, not Radiant tests/shots"
    );

    let entries = fs::read_dir(&tests_dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", tests_dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("failed to read entry in {}: {err}", tests_dir.display()))
            .path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        if path
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .is_some_and(|file_name| {
                file_name == "generic_surface_guardrails.rs"
                    || COMPAT_INTEGRATION_TESTS.contains(&file_name)
            })
        {
            continue;
        }
        collect_token_violations(
            &path,
            &manifest_dir,
            FORBIDDEN_GENERIC_TEST_TOKENS,
            &mut violations,
        );
    }

    assert!(
        violations.is_empty(),
        "generic Radiant integration tests must stay neutral; keep Sempal shell coverage in \
         host-owned tests or the explicit compat tests:\n{}",
        violations.join("\n")
    );
}

fn collect_violations(path: &Path, manifest_dir: &Path, violations: &mut Vec<String>) {
    if path.is_dir() {
        let mut entries = fs::read_dir(path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
            .map(|entry| {
                entry
                    .unwrap_or_else(|err| {
                        panic!("failed to read entry in {}: {err}", path.display())
                    })
                    .path()
            })
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            collect_violations(&entry, manifest_dir, violations);
        }
        return;
    }

    if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
        return;
    }

    collect_token_violations(path, manifest_dir, FORBIDDEN_GENERIC_TOKENS, violations);
}

fn collect_token_violations(
    path: &Path,
    manifest_dir: &Path,
    forbidden_tokens: &[&str],
    violations: &mut Vec<String>,
) {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    let uncommented = strip_rust_comments(&source);
    for (line_index, line) in uncommented.lines().enumerate() {
        let normalized = line
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>();
        for token in forbidden_tokens {
            if normalized.contains(token) {
                let relative = path.strip_prefix(manifest_dir).unwrap_or(path);
                violations.push(format!(
                    "{}:{} imports or names `{}`",
                    relative.display(),
                    line_index + 1,
                    token
                ));
            }
        }
    }
}

fn strip_rust_comments(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut block_depth = 0usize;

    while let Some(ch) = chars.next() {
        if block_depth > 0 {
            if ch == '/' && chars.peek() == Some(&'*') {
                chars.next();
                block_depth += 1;
            } else if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                block_depth -= 1;
            } else if ch == '\n' {
                output.push('\n');
            }
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'/') {
            for next in chars.by_ref() {
                if next == '\n' {
                    output.push('\n');
                    break;
                }
            }
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            block_depth = 1;
            continue;
        }

        output.push(ch);
    }

    output
}
