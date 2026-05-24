use super::{DemoMessage, DemoState, widget_ref};
use radiant::{
    layout::Vector2,
    runtime::{Command, PaintFillRect, PaintPrimitive, RuntimeBridge, SurfaceRuntime},
    widgets::{ButtonMessage, TextWidget},
};
use std::{
    thread,
    time::{Duration, Instant},
};

#[path = "runtime_behavior/animation.rs"]
mod animation;
#[path = "runtime_behavior/background.rs"]
mod background;
#[path = "runtime_behavior/focus.rs"]
mod focus;
#[path = "runtime_behavior/state_updates.rs"]
mod state_updates;
#[path = "runtime_behavior/support.rs"]
mod support;

pub(crate) use support::{FocusMessage, LoadingMessage, wait_for_runtime_message};
