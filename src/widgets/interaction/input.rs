//! Backend-neutral widget input contracts.

mod composition;
mod event;
mod keyboard;
mod pointer;
mod text_edit;
mod wheel;

pub(crate) use composition::CompositionSelectionState;
pub use composition::{
    CompositionPhase, CompositionRange, CompositionRangeError, CompositionSample,
    CompositionSampleError, CompositionStartContext,
};
pub use event::WidgetInput;
pub use keyboard::{KeyboardModifiers, WidgetKey, is_scroll_fallback_key};
pub use pointer::{PointerButton, PointerModifiers};
pub use text_edit::TextEditCommand;
pub use wheel::{
    WHEEL_LINE_EQUIVALENCE_PIXELS, WheelDelta, WheelDeltaError, WheelPhase, WheelSample,
    WheelSampleError,
};
