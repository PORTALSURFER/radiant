//! Structural guardrails for Radiant's generic public surface.
//!
//! The boundary is proven through package layout, dependency direction, public
//! exports, standalone examples, and behavior tests. These checks avoid token
//! policing so hosts can choose their own domain language outside Radiant.

#[cfg(test)]
#[path = "generic_surface_guardrails/behavior_suite.rs"]
mod behavior_suite;

#[cfg(test)]
#[path = "generic_surface_guardrails/docs.rs"]
mod docs;

#[cfg(test)]
#[path = "generic_surface_guardrails/dynamic_widgets.rs"]
mod dynamic_widgets;

#[cfg(test)]
#[path = "generic_surface_guardrails/examples.rs"]
mod examples;

#[cfg(test)]
#[path = "generic_surface_guardrails/native_vello.rs"]
mod native_vello;

#[cfg(test)]
#[path = "generic_surface_guardrails/package_structure.rs"]
mod package_structure;

#[cfg(test)]
#[path = "generic_surface_guardrails/public_api.rs"]
mod public_api;

#[cfg(test)]
#[path = "generic_surface_guardrails/source_quality.rs"]
mod source_quality;

#[cfg(test)]
#[path = "generic_surface_guardrails/support.rs"]
mod support;

#[cfg(test)]
pub(crate) use support::*;
