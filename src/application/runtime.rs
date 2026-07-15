use crate::{
    gui::{
        focus::FocusSurface, input::KeyPress, paint::PaintFrame as GuiPaintFrame,
        shortcuts::ShortcutResolution, types::Rect,
    },
    layout::Vector2,
    runtime::{
        AuxiliaryWindow, NativeFileDrop, NativeFileOpen, NativeFrameDiagnostics, PaintPrimitive,
        RuntimeAnimationActivity, ScrollUpdate, TransientOverlayContext,
    },
    widgets::RetainedSurfaceDescriptor,
};
use std::any::Any;

mod bridge;
mod queue;
mod subscription;
mod task;
mod threading;
mod timer;
mod update_context;

pub(in crate::application) use bridge::{
    AppBridge, AppBridgeLifecycle, FrameMessageActivity, FrameRepaintSource, PendingFrameRepaint,
};
pub(in crate::application) use queue::AppRuntime;
pub use subscription::Subscription;
pub use task::{
    CancellationToken, KeyedLatestTasks, KeyedTaskCompletion, LatestTask, ResourceTaskTicket,
    ResourceTasks, TaskCompletion, TaskTicket,
};
pub use update_context::{BusinessEventSink, BusinessWorkContext, UiUpdateContext};

pub(in crate::application) type RetainedPainter<State> =
    Box<dyn FnMut(&mut State, RetainedSurfaceDescriptor, Rect, Vector2) -> Option<GuiPaintFrame>>;
pub(in crate::application) type TransientOverlayPainter<State> =
    Box<dyn for<'a> FnMut(&mut State, TransientOverlayContext<'a>, &mut Vec<PaintPrimitive>)>;
pub(in crate::application) type TransientOverlayActivity<State> =
    Box<dyn FnMut(&mut State) -> RuntimeAnimationActivity>;
pub(in crate::application) type AppFrameClockActivity<State> =
    Box<dyn FnMut(&mut State) -> RuntimeAnimationActivity>;
pub(in crate::application) type AppAnimation<State> = Box<dyn FnMut(&mut State) -> bool>;
pub(in crate::application) type AppFrameMessage<Message> = Box<dyn FnMut() -> Message>;
pub(in crate::application) type AppSubscriptions<State, Message> =
    Box<dyn FnMut(&mut State) -> Subscription<Message>>;
pub(in crate::application) type AppShortcuts<State, Message> = Box<
    dyn FnMut(&mut State, Option<KeyPress>, KeyPress, FocusSurface) -> ShortcutResolution<Message>,
>;
pub(in crate::application) type AppStartup<State, Message> =
    Box<dyn FnMut(&mut State, &mut UiUpdateContext<Message>)>;
pub(in crate::application) type AppShutdown<State> =
    Box<dyn FnMut(&mut State) -> Option<serde_json::Value>>;
pub(in crate::application) type AppCloseRequested<State> = Box<dyn FnMut(&mut State) -> bool>;
pub(in crate::application) type AppAuxiliaryWindows<State, Message> =
    Box<dyn FnMut(&mut State) -> Vec<AuxiliaryWindow<Message>>>;
pub(in crate::application) type AppUpdate<State, Message> =
    Box<dyn FnMut(&mut State, Message, &mut UiUpdateContext<Message>)>;
pub(in crate::application) type AppScroll<State, Message> =
    Box<dyn FnMut(&mut State, ScrollUpdate, &mut UiUpdateContext<Message>)>;
pub(in crate::application) type AppNativeFileDrop<State, Message> =
    Box<dyn FnMut(&mut State, NativeFileDrop, &mut UiUpdateContext<Message>)>;
pub(in crate::application) type AppNativeFileOpen<State, Message> =
    Box<dyn FnMut(&mut State, NativeFileOpen, &mut UiUpdateContext<Message>)>;
pub(in crate::application) type AppNativeFrameDiagnostics<State> =
    Box<dyn FnMut(&mut State, NativeFrameDiagnostics)>;
pub(in crate::application) trait AppFrameRepaintPolicy<State> {
    fn capture_before_frame(&mut self, state: &mut State) -> Box<dyn Any>;
    fn resolve_after_frame(
        &mut self,
        state: &mut State,
        scope: Box<dyn Any>,
    ) -> crate::runtime::RepaintScope;
}
