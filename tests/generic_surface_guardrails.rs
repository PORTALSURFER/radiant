//! Guardrails for the generic Radiant public surface.
//!
//! Generic modules are allowed to use backend-neutral Radiant primitives only:
//! `radiant::layout`, `radiant::widgets`, `radiant::runtime`, `radiant::theme`,
//! and the shared non-shell `gui` primitives those APIs expose. The current
//! Sempal shell remains a transitional compatibility exception under
//! `compat::sempal_shell`, `gui::native_shell`, and the native Vello
//! compatibility runtime.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const DOMAIN_EXTRACTION_INVENTORY: &str = include_str!("../domain_extraction_inventory.tsv");

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
    "radiant::compat::sempal_shell",
    "radiant::{compat::sempal_shell",
    "compat::sempal_shell",
    "Sempal",
    "sempal",
    "capture_gui_automation_snapshot",
    "capture_native_shell_shot_snapshot",
];

const DOMAIN_SCAN_ROOTS: &[&str] = &["src", "tests", "examples"];

const DOMAIN_SCAN_EXEMPT_FILES: &[&str] = &["tests/generic_surface_guardrails.rs"];

const DOMAIN_TERMS: &[&str] = &[
    "AppModel",
    "UiAction",
    "Sempal",
    "sempal",
    "sample",
    "Sample",
    "browser",
    "Browser",
    "audio",
    "Audio",
    "waveform",
    "Waveform",
    "tag",
    "Tag",
    "collection",
    "Collection",
    "library",
    "Library",
    "source",
    "Source",
    "folder",
    "Folder",
    "BPM",
    "bpm",
    "slice",
    "Slice",
    "loop",
    "Loop",
    "one-shot",
    "One-shot",
    "oneshot",
    "Oneshot",
];

const INVENTORY_DISPOSITIONS: &[&str] = &[
    "move_to_sempal",
    "generalize_in_radiant",
    "remove_compat_export",
    "split_generic_from_compat",
    "generic_wording_cleanup",
];

const EXTRACTION_ISSUES: &[&str] = &[
    "OPT-270", "OPT-271", "OPT-272", "OPT-273", "OPT-274", "OPT-275", "OPT-276",
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

#[test]
fn generic_native_example_stays_non_sempal_and_runtime_backed() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let example_path = manifest_dir.join("examples/generic_native.rs");
    let source = fs::read_to_string(&example_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", example_path.display()));
    let uncommented = strip_rust_comments(&source);

    for forbidden in FORBIDDEN_GENERIC_TEST_TOKENS {
        assert!(
            !uncommented.contains(forbidden),
            "generic_native example must not depend on Sempal compatibility fixtures, found `{forbidden}`"
        );
    }
    for required in [
        "declarative_runtime_bridge",
        "run_native_vello_runtime",
        "UiSurface",
        "WidgetSpec::Button",
        "WidgetSpec::Text",
    ] {
        assert!(
            uncommented.contains(required),
            "generic_native example should exercise the generic runtime/widget API via `{required}`"
        );
    }
}

#[derive(Debug)]
struct ExtractionRule {
    pattern: String,
    disposition: String,
    issue: String,
}

#[test]
fn domain_extraction_inventory_covers_current_domain_bearing_files() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rules = parse_extraction_inventory();
    let files = domain_bearing_rust_files(&manifest_dir);
    let mut matched_rules: BTreeMap<&str, usize> = rules
        .iter()
        .map(|rule| (rule.pattern.as_str(), 0))
        .collect();
    let mut violations = Vec::new();

    for file in &files {
        let matches = rules
            .iter()
            .filter(|rule| rule.matches(file))
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            violations.push(format!(
                "{file} should match exactly one extraction inventory rule, matched {:?}",
                matches
                    .iter()
                    .map(|rule| rule.pattern.as_str())
                    .collect::<Vec<_>>()
            ));
            continue;
        }
        *matched_rules.get_mut(matches[0].pattern.as_str()).unwrap() += 1;
    }

    let unused_rules = matched_rules
        .iter()
        .filter_map(|(pattern, count)| (*count == 0).then_some(*pattern))
        .collect::<Vec<_>>();
    if !unused_rules.is_empty() {
        violations.push(format!(
            "extraction inventory contains rules that match no current domain-bearing Rust files: {unused_rules:?}"
        ));
    }

    assert!(
        violations.is_empty(),
        "every current Radiant file with Sempal-domain terms must have a final extraction disposition:\n{}",
        violations.join("\n")
    );
}

#[test]
fn domain_extraction_inventory_uses_known_dispositions_and_issues() {
    let rules = parse_extraction_inventory();

    for expected_disposition in [
        "move_to_sempal",
        "remove_compat_export",
        "split_generic_from_compat",
        "generic_wording_cleanup",
    ] {
        assert!(
            rules
                .iter()
                .any(|rule| rule.disposition == expected_disposition),
            "domain extraction inventory should include at least one {expected_disposition} rule"
        );
    }

    for expected_issue in ["OPT-270", "OPT-275", "OPT-276"] {
        assert!(
            rules.iter().any(|rule| rule.issue == expected_issue),
            "domain extraction inventory should include at least one {expected_issue} rule"
        );
    }
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

fn parse_extraction_inventory() -> Vec<ExtractionRule> {
    let mut rules = Vec::new();
    for (line_index, line) in DOMAIN_EXTRACTION_INVENTORY.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("pattern\t") {
            continue;
        }
        let columns = line.split('\t').collect::<Vec<_>>();
        assert_eq!(
            columns.len(),
            4,
            "domain extraction inventory line {} should have four tab-separated columns",
            line_index + 1
        );
        let disposition = columns[1].to_owned();
        assert!(
            INVENTORY_DISPOSITIONS.contains(&disposition.as_str()),
            "unknown extraction disposition {disposition:?} on line {}",
            line_index + 1
        );
        let issue = columns[2].to_owned();
        assert!(
            EXTRACTION_ISSUES.contains(&issue.as_str()),
            "unknown extraction issue {issue:?} on line {}",
            line_index + 1
        );
        rules.push(ExtractionRule {
            pattern: columns[0].to_owned(),
            disposition,
            issue,
        });
    }
    assert!(
        !rules.is_empty(),
        "domain extraction inventory should not be empty"
    );
    rules
}

impl ExtractionRule {
    fn matches(&self, file: &str) -> bool {
        if let Some(prefix) = self.pattern.strip_suffix("/**") {
            file.starts_with(&format!("{prefix}/"))
        } else {
            self.pattern == file
        }
    }
}

fn domain_bearing_rust_files(manifest_dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    for root in DOMAIN_SCAN_ROOTS {
        collect_domain_bearing_rust_files(&manifest_dir.join(root), manifest_dir, &mut files);
    }
    files.sort();
    files.dedup();
    files
}

fn collect_domain_bearing_rust_files(path: &Path, manifest_dir: &Path, files: &mut Vec<String>) {
    if !path.exists() {
        return;
    }
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
            collect_domain_bearing_rust_files(&entry, manifest_dir, files);
        }
        return;
    }

    if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
        return;
    }

    let relative = path
        .strip_prefix(manifest_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    if DOMAIN_SCAN_EXEMPT_FILES.contains(&relative.as_str()) {
        return;
    }
    let source = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    if DOMAIN_TERMS.iter().any(|term| source.contains(term)) {
        files.push(relative);
    }
}
