//! Native automation target snapshot export for dev and sidecar automation.

use crate::gui::automation::GuiAutomationTargetSnapshot;
#[cfg(test)]
use crate::gui::automation::{
    AutomationBounds, AutomationNodeId, AutomationNodeSemantics, AutomationRole,
    GuiAutomationSnapshot,
};
use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

const AUTOMATION_TARGET_EXPORT_ENV: &str = "RADIANT_AUTOMATION_TARGET_EXPORT";
const AUTOMATION_TARGET_EXPORT_PRETTY_ENV: &str = "RADIANT_AUTOMATION_TARGET_EXPORT_PRETTY";

pub(super) struct NativeAutomationTargetExporter {
    path: Option<PathBuf>,
    pretty: bool,
    last_payload: Option<Vec<u8>>,
    warned_after_failure: bool,
}

#[derive(Debug)]
pub(super) enum NativeAutomationTargetExportError {
    Serialize(serde_json::Error),
    CreateParent {
        path: PathBuf,
        source: std::io::Error,
    },
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    Rename {
        from: PathBuf,
        to: PathBuf,
        source: std::io::Error,
    },
}

impl NativeAutomationTargetExporter {
    pub(super) fn from_env() -> Self {
        let path = std::env::var_os(AUTOMATION_TARGET_EXPORT_ENV)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let pretty = std::env::var(AUTOMATION_TARGET_EXPORT_PRETTY_ENV)
            .ok()
            .is_some_and(|value| crate::env_flags::is_truthy(&value));
        Self::new(path, pretty)
    }

    pub(super) fn new(path: Option<PathBuf>, pretty: bool) -> Self {
        Self {
            path,
            pretty,
            last_payload: None,
            warned_after_failure: false,
        }
    }

    pub(super) fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub(super) fn has_warned_after_failure(&self) -> bool {
        self.warned_after_failure
    }

    pub(super) fn mark_warned_after_failure(&mut self) {
        self.warned_after_failure = true;
    }

    pub(super) fn reset_failure_warning(&mut self) {
        self.warned_after_failure = false;
    }

    pub(super) fn export(
        &mut self,
        snapshot: &GuiAutomationTargetSnapshot,
    ) -> Result<bool, NativeAutomationTargetExportError> {
        let Some(path) = self.path.clone() else {
            return Ok(false);
        };
        let payload = serialize_snapshot(snapshot, self.pretty)
            .map_err(NativeAutomationTargetExportError::Serialize)?;
        if self.last_payload.as_deref() == Some(payload.as_slice()) {
            return Ok(false);
        }

        write_atomic(&path, &payload)?;
        self.last_payload = Some(payload);
        self.reset_failure_warning();
        Ok(true)
    }
}

impl NativeAutomationTargetExportError {
    pub(super) fn path(&self) -> Option<&Path> {
        match self {
            Self::Serialize(_) => None,
            Self::CreateParent { path, .. } | Self::Write { path, .. } => Some(path),
            Self::Rename { to, .. } => Some(to),
        }
    }
}

impl fmt::Display for NativeAutomationTargetExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialize(err) => write!(f, "serialize automation target snapshot: {err}"),
            Self::CreateParent { path, source } => {
                write!(
                    f,
                    "create parent directory for {}: {source}",
                    path.display()
                )
            }
            Self::Write { path, source } => {
                write!(
                    f,
                    "write automation target snapshot {}: {source}",
                    path.display()
                )
            }
            Self::Rename { from, to, source } => write!(
                f,
                "publish automation target snapshot {} -> {}: {source}",
                from.display(),
                to.display()
            ),
        }
    }
}

impl std::error::Error for NativeAutomationTargetExportError {}

fn serialize_snapshot(
    snapshot: &GuiAutomationTargetSnapshot,
    pretty: bool,
) -> Result<Vec<u8>, serde_json::Error> {
    if pretty {
        serde_json::to_vec_pretty(snapshot)
    } else {
        serde_json::to_vec(snapshot)
    }
}

fn write_atomic(path: &Path, payload: &[u8]) -> Result<(), NativeAutomationTargetExportError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|source| {
            NativeAutomationTargetExportError::CreateParent {
                path: parent.to_path_buf(),
                source,
            }
        })?;
    }

    let tmp_path = temporary_path(path);
    fs::write(&tmp_path, payload).map_err(|source| NativeAutomationTargetExportError::Write {
        path: tmp_path.clone(),
        source,
    })?;
    fs::rename(&tmp_path, path).map_err(|source| {
        let _ = fs::remove_file(&tmp_path);
        NativeAutomationTargetExportError::Rename {
            from: tmp_path,
            to: path.to_path_buf(),
            source,
        }
    })
}

fn temporary_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("automation-targets.json");
    path.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exporter_writes_changed_snapshot_and_skips_identical_payload() {
        let path = test_path("targets");
        let mut exporter = NativeAutomationTargetExporter::new(Some(path.clone()), true);
        let snapshot = target_snapshot("Save");

        assert!(exporter.export(&snapshot).expect("first export"));
        assert!(path.exists());
        let json = fs::read_to_string(&path).expect("target snapshot json");
        assert!(json.contains("\"targets\""));
        assert!(json.contains("\"Save\""));

        assert!(!exporter.export(&snapshot).expect("unchanged export"));

        let changed = target_snapshot("Open");
        assert!(exporter.export(&changed).expect("changed export"));
        let json = fs::read_to_string(&path).expect("changed target snapshot json");
        assert!(json.contains("\"Open\""));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn disabled_exporter_is_noop() {
        let mut exporter = NativeAutomationTargetExporter::new(None, false);

        assert!(
            !exporter
                .export(&target_snapshot("Hidden"))
                .expect("disabled export")
        );
    }

    fn target_snapshot(label: &str) -> GuiAutomationTargetSnapshot {
        let mut semantics = AutomationNodeSemantics::new(AutomationRole::Button).with_label(label);
        semantics.focusable = true;
        GuiAutomationSnapshot {
            schema_version: 2,
            viewport_width: 320,
            viewport_height: 120,
            root: crate::gui::automation::AutomationNodeSnapshot::from_semantics(
                AutomationNodeId::new("10"),
                AutomationBounds {
                    x: 4.0,
                    y: 8.0,
                    width: 80.0,
                    height: 24.0,
                },
                semantics,
            ),
        }
        .target_snapshot()
    }

    fn test_path(name: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "radiant-automation-export-{name}-{}-{stamp}.json",
            std::process::id()
        ))
    }
}
