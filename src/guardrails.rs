//! Reusable static guardrails for Radiant host applications.
//!
//! These helpers are intended for tests and CI lanes that scan app-facing
//! update, action, and view paths for obvious blocking work. They are not a
//! proof of non-blocking behavior; they catch known hazards so applications can
//! route work through `UiUpdateContext::business()` or typed platform services.

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

/// Static scan configuration for non-blocking Radiant app paths.
#[derive(Clone, Debug)]
pub struct NonBlockingGuardrail {
    patterns: Vec<ForbiddenBlockingPattern>,
    allowlisted_path_fragments: Vec<AllowedPathFragment>,
}

impl NonBlockingGuardrail {
    /// Build the default app-update guardrail for UI/action/view paths.
    pub fn app_update_paths() -> Self {
        Self {
            patterns: default_blocking_patterns(),
            allowlisted_path_fragments: Vec::new(),
        }
    }

    /// Add one forbidden token to scan for.
    pub fn forbid_token(
        mut self,
        token: &'static str,
        label: &'static str,
        guidance: &'static str,
    ) -> Self {
        self.patterns.push(ForbiddenBlockingPattern {
            token,
            label,
            guidance,
        });
        self
    }

    /// Allow one narrow path fragment with an auditable reason.
    pub fn allow_path_fragment(
        mut self,
        fragment: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        self.allowlisted_path_fragments.push(AllowedPathFragment {
            fragment: normalize_path_fragment(&fragment.into()),
            reason: reason.into(),
        });
        self
    }

    /// Scan one or more source roots.
    pub fn scan_roots<I, P>(&self, roots: I) -> Result<(), NonBlockingGuardrailReport>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut report = NonBlockingGuardrailReport::default();
        for root in roots {
            self.scan_path(root.as_ref(), &mut report);
        }
        report.into_result()
    }

    fn scan_path(&self, path: &Path, report: &mut NonBlockingGuardrailReport) {
        if self.is_allowlisted(path) || !path.exists() {
            return;
        }
        if path.is_dir() {
            self.scan_dir(path, report);
            return;
        }
        if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            self.scan_file(path, report);
        }
    }

    fn scan_dir(&self, dir: &Path, report: &mut NonBlockingGuardrailReport) {
        match fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => self.scan_path(&entry.path(), report),
                        Err(error) => report.read_errors.push(GuardrailReadError {
                            path: dir.to_owned(),
                            error: error.to_string(),
                        }),
                    }
                }
            }
            Err(error) => report.read_errors.push(GuardrailReadError {
                path: dir.to_owned(),
                error: error.to_string(),
            }),
        }
    }

    fn scan_file(&self, path: &Path, report: &mut NonBlockingGuardrailReport) {
        let source = match fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                report.read_errors.push(GuardrailReadError {
                    path: path.to_owned(),
                    error: error.to_string(),
                });
                return;
            }
        };
        for (index, line) in source.lines().enumerate() {
            for pattern in &self.patterns {
                if line.contains(pattern.token) {
                    report.violations.push(NonBlockingGuardrailViolation {
                        path: path.to_owned(),
                        line: index + 1,
                        token: pattern.token,
                        label: pattern.label,
                        guidance: pattern.guidance,
                        source_line: line.trim().to_owned(),
                    });
                    break;
                }
            }
        }
    }

    fn is_allowlisted(&self, path: &Path) -> bool {
        let normalized = normalize_path_fragment(&path.to_string_lossy());
        self.allowlisted_path_fragments
            .iter()
            .any(|allowlist| normalized.contains(&allowlist.fragment))
    }
}

/// A detected non-blocking architecture violation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonBlockingGuardrailViolation {
    /// Source path containing the violation.
    pub path: PathBuf,
    /// One-based line number.
    pub line: usize,
    /// Forbidden token that matched the source line.
    pub token: &'static str,
    /// Human-readable pattern label.
    pub label: &'static str,
    /// Guidance for fixing the violation.
    pub guidance: &'static str,
    /// Trimmed source line that matched.
    pub source_line: String,
}

impl fmt::Display for NonBlockingGuardrailViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}: {} matched `{}`: {} | {}",
            self.path.display(),
            self.line,
            self.label,
            self.token,
            self.guidance,
            self.source_line
        )
    }
}

/// Static guardrail report returned when violations or read errors are found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NonBlockingGuardrailReport {
    violations: Vec<NonBlockingGuardrailViolation>,
    read_errors: Vec<GuardrailReadError>,
}

impl NonBlockingGuardrailReport {
    /// Return detected source violations.
    pub fn violations(&self) -> &[NonBlockingGuardrailViolation] {
        &self.violations
    }

    /// Return source files or directories that could not be read.
    pub fn read_errors(&self) -> &[GuardrailReadError] {
        &self.read_errors
    }

    /// Return whether the report contains no violations and no read errors.
    pub fn is_empty(&self) -> bool {
        self.violations.is_empty() && self.read_errors.is_empty()
    }

    fn into_result(self) -> Result<(), Self> {
        if self.is_empty() { Ok(()) } else { Err(self) }
    }
}

impl fmt::Display for NonBlockingGuardrailReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "app-facing Radiant code must not perform obvious blocking work; use UiUpdateContext::business() or typed platform services"
        )?;
        for violation in &self.violations {
            writeln!(formatter, "{violation}")?;
        }
        for error in &self.read_errors {
            writeln!(formatter, "{error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for NonBlockingGuardrailReport {}

/// Source file or directory that could not be scanned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuardrailReadError {
    /// Source path that could not be read.
    pub path: PathBuf,
    /// Read error text.
    pub error: String,
}

impl fmt::Display for GuardrailReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: failed to read guarded source: {}",
            self.path.display(),
            self.error
        )
    }
}

#[derive(Clone, Debug)]
struct ForbiddenBlockingPattern {
    token: &'static str,
    label: &'static str,
    guidance: &'static str,
}

#[derive(Clone, Debug)]
struct AllowedPathFragment {
    fragment: String,
    #[allow(dead_code)]
    reason: String,
}

fn default_blocking_patterns() -> Vec<ForbiddenBlockingPattern> {
    vec![
        blocking("std::fs::", "filesystem API"),
        blocking("fs::", "filesystem API"),
        blocking(".exists()", "filesystem metadata check"),
        blocking(".metadata()", "filesystem metadata check"),
        blocking(".canonicalize()", "filesystem path resolution"),
        blocking("std::thread::sleep", "thread sleep"),
        blocking("thread::sleep", "thread sleep"),
        blocking("std::thread::spawn", "manual thread spawn"),
        blocking("thread::spawn", "manual thread spawn"),
        blocking(".join()", "blocking join"),
        blocking(".recv()", "blocking channel receive"),
        blocking("blocking_recv", "blocking channel receive"),
        blocking("SourceDatabase::open", "database open"),
        blocking("SourceDatabase::open_fast", "database open"),
        blocking("SourceDatabase::open_with_role", "database open"),
        blocking("FileDialog::new", "direct file dialog"),
        blocking("MessageDialog::new", "direct message dialog"),
        blocking("open::that", "direct shell open"),
        blocking("arboard::Clipboard", "direct clipboard access"),
        blocking("std::process::Command", "direct process launch"),
    ]
}

fn blocking(token: &'static str, label: &'static str) -> ForbiddenBlockingPattern {
    ForbiddenBlockingPattern {
        token,
        label,
        guidance: "route work through context.business() or a typed platform service",
    }
}

fn normalize_path_fragment(fragment: &str) -> String {
    fragment.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guardrail_reports_file_line_and_guidance_for_blocking_tokens() {
        let root = temp_guardrail_dir("reports_blocking");
        let source = root.join("ui_action.rs");
        fs::create_dir_all(&root).expect("create guardrail fixture dir");
        fs::write(
            &source,
            "fn update() {\n    std::thread::sleep(std::time::Duration::from_millis(1));\n}\n",
        )
        .expect("write guardrail fixture");

        let report = NonBlockingGuardrail::app_update_paths()
            .scan_roots([&root])
            .expect_err("blocking sleep should be reported");

        assert_eq!(report.violations().len(), 1);
        assert!(report.to_string().contains("ui_action.rs:2"));
        assert!(report.to_string().contains("context.business()"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn guardrail_allows_narrow_worker_or_platform_boundaries() {
        let root = temp_guardrail_dir("allows_worker_boundary");
        let worker = root.join("worker").join("job.rs");
        let ui = root.join("ui.rs");
        fs::create_dir_all(worker.parent().expect("worker parent")).expect("create worker dir");
        fs::write(
            &worker,
            "fn run() { std::fs::read(\"sample.wav\").ok(); }\n",
        )
        .expect("write worker fixture");
        fs::write(&ui, "fn update() { let message = 1; }\n").expect("write ui fixture");

        NonBlockingGuardrail::app_update_paths()
            .allow_path_fragment("/worker/", "business worker boundary")
            .scan_roots([&root])
            .expect("allowlisted worker path should be skipped");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn guardrail_accepts_custom_forbidden_tokens_for_host_domains() {
        let root = temp_guardrail_dir("custom_tokens");
        let source = root.join("controller.rs");
        fs::create_dir_all(&root).expect("create guardrail fixture dir");
        fs::write(&source, "fn update() { decode_audio_now(); }\n").expect("write fixture");

        let report = NonBlockingGuardrail::app_update_paths()
            .forbid_token(
                "decode_audio_now",
                "host decode",
                "schedule decode work through the business runtime",
            )
            .scan_roots([&root])
            .expect_err("custom decode token should be reported");

        assert!(report.to_string().contains("host decode"));

        let _ = fs::remove_dir_all(root);
    }

    fn temp_guardrail_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "radiant_non_blocking_guardrail_{name}_{}",
            std::process::id()
        ))
    }
}
