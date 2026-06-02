//! Shared widget contracts for the public `radiant::widgets` surface.
//!
//! These types describe what all first-class widgets have in common before the
//! generic runtime/message surface exists. They intentionally define
//! responsibilities and vocabulary rather than locking `radiant` into one
//! retained-tree implementation.

mod identity;
mod paint;
mod sizing;
mod state;
mod style;
mod widget;

pub use identity::{stable_widget_id, stable_widget_id_u64};
pub use paint::{PaintBounds, PaintContract};
pub use sizing::{WidgetId, WidgetSizing, WidgetSizingParts};
pub use state::{FocusBehavior, WidgetState};
pub use style::{WidgetProminence, WidgetStyle, WidgetTone};
pub use widget::Widget;
