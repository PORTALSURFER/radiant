//! Shared widget contracts for the public `radiant::widgets` surface.
//!
//! These types describe what all first-class widgets have in common before the
//! generic runtime/message surface exists. They intentionally define
//! responsibilities and vocabulary rather than locking `radiant` into one
//! retained-tree implementation.

mod hit_test;
mod identity;
mod paint;
mod pointer_motion;
mod revision;
mod semantics;
mod sizing;
mod state;
mod style;
mod widget;

pub use hit_test::{WidgetHitTest, WidgetHitTestResult, WidgetHitTestRevision};
pub use identity::{stable_widget_id, stable_widget_id_u64};
pub use paint::{PaintBounds, PaintContract, WidgetPaintContext};
pub use pointer_motion::{WidgetPointerMotion, WidgetPointerMotionRevision};
pub use revision::WidgetRevision;
pub(crate) use revision::WidgetRevisionComponents;
pub use semantics::{
    WIDGET_CAPABILITIES_CONTRACT_VERSION, WIDGET_CAPABILITIES_V1_CONTRACT_VERSION,
    WIDGET_CAPABILITIES_V2_CONTRACT_VERSION, WidgetCapabilities, WidgetCapabilitiesV2,
    WidgetSemantics, WidgetSemanticsRevision,
};
pub(crate) use semantics::{supports_capabilities_v2_contract, supports_semantics_contract};
pub use sizing::{WidgetId, WidgetSizing, WidgetSizingParts};
pub use state::{FocusBehavior, WidgetState};
pub use style::{WidgetProminence, WidgetStyle, WidgetTone};
pub use widget::{
    FocusLossDecision, FocusedKeyDisposition, PointerCapturePolicy, PointerPressAdmission, Widget,
};
