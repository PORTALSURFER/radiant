use super::*;

const FORBIDDEN_PRODUCTION_PATTERNS: &[&str] = &[
    ".unwrap()",
    ".expect(",
    "panic!(",
    "todo!(",
    "unimplemented!(",
    "dbg!(",
];

#[test]
fn production_source_avoids_panic_shortcuts() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();

    for path in rust_sources_under(&manifest_dir.join("src")) {
        if is_test_source(&manifest_dir, &path) {
            continue;
        }
        collect_panic_shortcut_violations(&manifest_dir, &path, &mut violations);
    }

    assert!(
        violations.is_empty(),
        "production source should use explicit error handling instead of panic shortcuts:\n{}",
        violations.join("\n")
    );
}

fn collect_panic_shortcut_violations(
    manifest_dir: &std::path::Path,
    path: &std::path::Path,
    violations: &mut Vec<String>,
) {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("{} should be readable: {err}", path.display()));
    let relative = relative_path(manifest_dir, path);

    for (line_number, line) in production_lines(&source) {
        if let Some(pattern) = FORBIDDEN_PRODUCTION_PATTERNS
            .iter()
            .find(|pattern| line.contains(**pattern))
        {
            violations.push(format!("{relative}:{line_number} contains `{pattern}`"));
        }
    }
}

fn is_test_source(manifest_dir: &std::path::Path, path: &std::path::Path) -> bool {
    let relative = relative_path(manifest_dir, path);
    relative.ends_with("/tests.rs")
        || relative.ends_with("_tests.rs")
        || relative.contains("/tests/")
        || relative.contains("_tests/")
}

fn production_lines(source: &str) -> Vec<(usize, &str)> {
    let mut lines = Vec::new();
    let mut pending_cfg_test = false;
    let mut test_module_depth: Option<isize> = None;

    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        if test_module_depth.is_some() {
            advance_test_module_depth(line, &mut test_module_depth);
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[cfg(test)]") {
            pending_cfg_test = true;
            continue;
        }
        if pending_cfg_test && trimmed.is_empty() {
            continue;
        }
        if pending_cfg_test && starts_inline_test_module(trimmed) {
            test_module_depth = Some(brace_delta(line));
            if test_module_depth == Some(0) {
                test_module_depth = None;
            }
            pending_cfg_test = false;
            continue;
        }
        pending_cfg_test = false;
        lines.push((line_number, line));
    }

    lines
}

fn starts_inline_test_module(trimmed: &str) -> bool {
    trimmed.starts_with("mod tests") && trimmed.contains('{')
}

fn advance_test_module_depth(line: &str, depth: &mut Option<isize>) {
    let current = depth.unwrap_or_default();
    let delta = brace_delta(line);
    let next = current + delta;
    *depth = (next > 0).then_some(next);
}

fn brace_delta(line: &str) -> isize {
    let opens = line.chars().filter(|character| *character == '{').count() as isize;
    let closes = line.chars().filter(|character| *character == '}').count() as isize;
    opens - closes
}
