//! Shared linear virtualization resolution routines for ScrollView.

mod metrics;
mod state;
mod uniform;
mod validation;

pub(super) use metrics::{build_linear_metrics, known_linear_main_extent};
pub(super) use validation::metrics_is_valid;
