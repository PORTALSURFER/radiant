use crate::{
    gui::{
        focus::FocusSurface, input::KeyPress, paint::PaintFrame as GuiPaintFrame,
        shortcuts::ShortcutResolution, types::Rect,
    },
    layout::Vector2,
    runtime::{PaintPrimitive, RuntimeAnimationActivity, ScrollUpdate, TransientOverlayContext},
    widgets::RetainedSurfaceDescriptor,
};
use std::sync::Arc;

mod bridge;
mod queue;
mod subscription;
mod threading;
mod timer;
mod update_context;

pub(in crate::application) use bridge::{AppBridge, AppBridgeLifecycle};
pub(in crate::application) use queue::AppRuntime;
pub use subscription::Subscription;
pub use update_context::UpdateContext;

pub(in crate::application) type RetainedPainter<State> =
    Box<dyn FnMut(&mut State, RetainedSurfaceDescriptor, Rect, Vector2) -> Option<GuiPaintFrame>>;
pub(in crate::application) type TransientOverlayPainter<State> =
    Box<dyn for<'a> FnMut(&mut State, TransientOverlayContext<'a>, &mut Vec<PaintPrimitive>)>;
pub(in crate::application) type TransientOverlayActivity<State> =
    Box<dyn FnMut(&mut State) -> RuntimeAnimationActivity>;
pub(in crate::application) type AppAnimation<State> = Box<dyn FnMut(&mut State) -> bool>;
pub(in crate::application) type AppFrameMessage<Message> = Box<dyn FnMut() -> Message>;
pub(in crate::application) type AppSubscriptions<State, Message> =
    Box<dyn FnMut(&mut State) -> Subscription<Message>>;
pub(in crate::application) type AppShortcuts<State, Message> = Box<
    dyn FnMut(&mut State, Option<KeyPress>, KeyPress, FocusSurface) -> ShortcutResolution<Message>,
>;
pub(in crate::application) type AppStartup<State, Message> =
    Box<dyn FnMut(&mut State, &mut UpdateContext<Message>)>;
pub(in crate::application) type AppShutdown<State> =
    Box<dyn FnMut(&mut State) -> Option<serde_json::Value>>;
pub(in crate::application) type AppCloseRequested<State> = Box<dyn FnMut(&mut State) -> bool>;
pub(in crate::application) type AppUpdate<State, Message> =
    Box<dyn FnMut(&mut State, Message, &mut UpdateContext<Message>)>;
pub(in crate::application) type AppScroll<State, Message> =
    Box<dyn FnMut(&mut State, ScrollUpdate, &mut UpdateContext<Message>)>;
pub(in crate::application) type StateStringCallback<State> =
    Arc<dyn Fn(&mut State, String) + Send + Sync>;
pub(in crate::application) type StateCallback<State> = Arc<dyn Fn(&mut State) + Send + Sync>;
pub(in crate::application) type StateDragCallback<State> =
    Arc<dyn Fn(&mut State, String, crate::widgets::DragHandleMessage) + Send + Sync>;
