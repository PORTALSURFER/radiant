use crate::{
    gui::{
        focus::FocusSurface,
        input::KeyPress,
        paint::PaintFrame as GuiPaintFrame,
        repaint::RepaintSignal,
        shortcuts::ShortcutResolution,
        types::Rect,
    },
    widgets::{RetainedSurfaceDescriptor, WidgetId},
};

type RetainedPainter<State> =
    Box<dyn FnMut(&mut State, RetainedSurfaceDescriptor, Rect, Vector2) -> Option<GuiPaintFrame>>;
type AppAnimation<State> = Box<dyn FnMut(&mut State) -> bool>;
type AppFrameMessage<Message> = Box<dyn FnMut() -> Message>;
type AppSubscriptions<State, Message> = Box<dyn FnMut(&mut State) -> Subscription<Message>>;
type AppShortcuts<State, Message> =
    Box<dyn FnMut(&mut State, Option<KeyPress>, KeyPress, FocusSurface) -> ShortcutResolution<Message>>;
type AppStartup<State, Message> = Box<dyn FnMut(&mut State, &mut UpdateContext<Message>)>;
type AppShutdown<State> = Box<dyn FnMut(&mut State) -> Option<serde_json::Value>>;
type AppCloseRequested<State> = Box<dyn FnMut(&mut State) -> bool>;
type AppUpdate<State, Message> = Box<dyn FnMut(&mut State, Message, &mut UpdateContext<Message>)>;
type StateStringCallback<State> = Arc<dyn Fn(&mut State, String) + Send + Sync>;
type StateCallback<State> = Arc<dyn Fn(&mut State) + Send + Sync>;
type StateDragCallback<State> =
    Arc<dyn Fn(&mut State, String, crate::widgets::DragHandleMessage) + Send + Sync>;

include!("runtime/queue.rs");
include!("runtime/update_context.rs");
include!("runtime/subscription.rs");
include!("runtime/bridge.rs");
