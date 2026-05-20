//! Source-quality guardrails for focused modules and readable public models.

use std::{fs, path::PathBuf};

use super::relative_path;

#[path = "source_quality/api_models.rs"]
mod api_models;
#[path = "source_quality/feedback_and_platform.rs"]
mod feedback_and_platform;
#[path = "source_quality/layout_runtime.rs"]
mod layout_runtime;
#[path = "source_quality/runtime.rs"]
mod runtime;
#[path = "source_quality/widgets.rs"]
mod widgets;
