use crate::{
    application::runtime::{
        AppBridgeLifecycle, AppFrameClockActivity, AppFrameMessage, AppFrameRepaintPolicy,
        TransientOverlayActivity, TransientOverlayPainter,
    },
    runtime::{PaintPrimitive, RuntimeAnimationActivity, TransientOverlayContext},
};
use std::any::Any;

/// Declarative application presentation hooks.
///
/// `Presentation` groups app-level frame clocks and transient paint overlays
/// under typed descriptors so the root app configuration can name rendering
/// concerns without spelling out lower-level runtime lifecycle hooks.
pub struct Presentation<State, Message> {
    frame_clock: Option<FrameClock<State, Message>>,
    transient_overlay: Option<TransientOverlay<State>>,
}

/// Build an empty presentation descriptor.
pub fn presentation<State: 'static, Message>() -> Presentation<State, Message> {
    Presentation::new()
}

impl<State: 'static, Message> Presentation<State, Message> {
    /// Build an empty presentation descriptor.
    pub const fn new() -> Self {
        Self {
            frame_clock: None,
            transient_overlay: None,
        }
    }

    /// Declare a frame clock that emits host messages while active.
    pub fn frame_clock(mut self, frame_clock: FrameClock<State, Message>) -> Self {
        self.frame_clock = Some(frame_clock);
        self
    }

    /// Declare a transient paint overlay.
    pub fn transient_overlay(mut self, overlay: TransientOverlay<State>) -> Self {
        self.transient_overlay = Some(overlay);
        self
    }

    pub(in crate::application) fn apply_to_lifecycle(
        self,
        lifecycle: &mut AppBridgeLifecycle<State, Message>,
    ) {
        if let Some(frame_clock) = self.frame_clock {
            let FrameClockParts {
                activity,
                message,
                repaint_policy,
            } = frame_clock.into_parts();
            lifecycle.frame_clock_activity = Some(activity);
            lifecycle.frame_message = Some(message);
            lifecycle.frame_repaint_policy = repaint_policy;
        }

        if let Some(overlay) = self.transient_overlay {
            let TransientOverlayParts { activity, painter } = overlay.into_parts();
            lifecycle.transient_overlay_activity = Some(activity);
            if let Some(painter) = painter {
                lifecycle.transient_overlay = Some(painter);
            }
        }
    }

    pub(in crate::application) fn apply_to_scene_lifecycle(
        self,
        lifecycle: &mut AppBridgeLifecycle<State, Message>,
    ) {
        if let Some(frame_clock) = self.frame_clock {
            let FrameClockParts {
                activity,
                message,
                repaint_policy,
            } = frame_clock.into_parts();
            lifecycle.scene_frame_clock_activity = Some(activity);
            lifecycle.scene_frame_message = Some(message);
            lifecycle.scene_frame_repaint_policy = repaint_policy;
        }

        if let Some(overlay) = self.transient_overlay {
            let TransientOverlayParts { activity, painter } = overlay.into_parts();
            lifecycle.scene_transient_overlay_activity = Some(activity);
            if let Some(painter) = painter {
                lifecycle.scene_transient_overlay = Some(painter);
            }
        }
    }
}

impl<State: 'static, Message> Default for Presentation<State, Message> {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed descriptor for host-state frame messages.
pub struct FrameClock<State, Message> {
    message: AppFrameMessage<Message>,
    when: Box<dyn FnMut(&mut State) -> bool>,
    target_fps: Option<u32>,
    repaint_policy: Option<Box<dyn AppFrameRepaintPolicy<State>>>,
}

struct FrameClockParts<State, Message> {
    activity: AppFrameClockActivity<State>,
    message: AppFrameMessage<Message>,
    repaint_policy: Option<Box<dyn AppFrameRepaintPolicy<State>>>,
}

impl<State: 'static, Message> FrameClock<State, Message>
where
    Message: Clone + 'static,
{
    /// Build a frame clock that emits a cloned message value.
    pub fn message(message: Message) -> Self {
        Self::message_with(move || message.clone())
    }
}

impl<State: 'static, Message> FrameClock<State, Message> {
    /// Build a frame clock from a message factory.
    pub fn message_with(message: impl FnMut() -> Message + 'static) -> Self {
        Self {
            message: Box::new(message),
            when: Box::new(|_| true),
            target_fps: None,
            repaint_policy: None,
        }
    }

    /// Emit frame messages only while the predicate is active.
    pub fn when(mut self, when: impl FnMut(&mut State) -> bool + 'static) -> Self {
        self.when = Box::new(when);
        self
    }

    /// Request a maximum frame-message cadence.
    pub const fn fps(mut self, target_fps: u32) -> Self {
        self.target_fps = Some(target_fps);
        self
    }

    /// Capture state before a queued frame message and decide the repaint
    /// scope after that message has updated host state.
    pub fn repaint_scope<Scope>(
        mut self,
        before_frame: impl FnMut(&mut State) -> Scope + 'static,
        can_use_paint_only: impl FnMut(&mut State, Scope) -> bool + 'static,
    ) -> Self
    where
        Scope: 'static,
    {
        self.repaint_policy = Some(Box::new(TypedFrameRepaintPolicy::new(
            before_frame,
            can_use_paint_only,
        )));
        self
    }

    fn into_parts(self) -> FrameClockParts<State, Message> {
        let Self {
            message,
            mut when,
            target_fps,
            repaint_policy,
        } = self;
        let activity = Box::new(move |state: &mut State| {
            if !when(state) {
                return RuntimeAnimationActivity::idle();
            }
            match target_fps {
                Some(target_fps) => RuntimeAnimationActivity::frame_messages_at(target_fps),
                None => RuntimeAnimationActivity::frame_messages(),
            }
        });
        FrameClockParts {
            activity,
            message,
            repaint_policy,
        }
    }
}

struct TypedFrameRepaintPolicy<Scope, BeforeFrame, CanUsePaintOnly> {
    before_frame: BeforeFrame,
    can_use_paint_only: CanUsePaintOnly,
    _scope: std::marker::PhantomData<Scope>,
}

impl<Scope, BeforeFrame, CanUsePaintOnly>
    TypedFrameRepaintPolicy<Scope, BeforeFrame, CanUsePaintOnly>
{
    fn new(before_frame: BeforeFrame, can_use_paint_only: CanUsePaintOnly) -> Self {
        Self {
            before_frame,
            can_use_paint_only,
            _scope: std::marker::PhantomData,
        }
    }
}

impl<State, Scope, BeforeFrame, CanUsePaintOnly> AppFrameRepaintPolicy<State>
    for TypedFrameRepaintPolicy<Scope, BeforeFrame, CanUsePaintOnly>
where
    Scope: 'static,
    BeforeFrame: FnMut(&mut State) -> Scope,
    CanUsePaintOnly: FnMut(&mut State, Scope) -> bool,
{
    fn capture_before_frame(&mut self, state: &mut State) -> Box<dyn Any> {
        Box::new((self.before_frame)(state))
    }

    fn resolve_after_frame(&mut self, state: &mut State, scope: Box<dyn Any>) -> bool {
        let Ok(scope) = scope.downcast::<Scope>() else {
            return false;
        };
        let scope = *scope;
        (self.can_use_paint_only)(state, scope)
    }
}

/// Typed descriptor for a paint-only transient overlay.
pub struct TransientOverlay<State> {
    key: u64,
    when: Box<dyn FnMut(&mut State) -> bool>,
    target_fps: Option<u32>,
    painter: Option<TransientOverlayPainter<State>>,
}

struct TransientOverlayParts<State> {
    activity: TransientOverlayActivity<State>,
    painter: Option<TransientOverlayPainter<State>>,
}

impl<State: 'static> TransientOverlay<State> {
    /// Build a transient overlay descriptor with a stable host key.
    pub fn new(key: impl Into<u64>) -> Self {
        Self {
            key: key.into(),
            when: Box::new(|_| true),
            target_fps: None,
            painter: None,
        }
    }

    /// Return the stable host key associated with this overlay.
    pub const fn key(&self) -> u64 {
        self.key
    }

    /// Mark this overlay as paint-only.
    ///
    /// Transient overlays are paint-only in this API phase; this builder keeps
    /// call sites explicit and leaves room for future overlay modes.
    pub const fn paint_only(self) -> Self {
        self
    }

    /// Paint this overlay over the current projected surface.
    pub fn paint(
        mut self,
        painter: impl for<'a> FnMut(&mut State, TransientOverlayContext<'a>, &mut Vec<PaintPrimitive>)
        + 'static,
    ) -> Self {
        self.painter = Some(Box::new(painter));
        self
    }

    /// Animate this overlay only while the predicate is active.
    pub fn when(mut self, when: impl FnMut(&mut State) -> bool + 'static) -> Self {
        self.when = Box::new(when);
        self
    }

    /// Request a maximum paint-only overlay cadence.
    pub const fn fps(mut self, target_fps: u32) -> Self {
        self.target_fps = Some(target_fps);
        self
    }

    fn into_parts(self) -> TransientOverlayParts<State> {
        let Self {
            key: _,
            mut when,
            target_fps,
            painter,
        } = self;
        let activity = Box::new(move |state: &mut State| {
            if !when(state) {
                return RuntimeAnimationActivity::idle();
            }
            match target_fps {
                Some(target_fps) => RuntimeAnimationActivity::paint_only_at(target_fps),
                None => RuntimeAnimationActivity::paint_only(),
            }
        });
        TransientOverlayParts { activity, painter }
    }
}
