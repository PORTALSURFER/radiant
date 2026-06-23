use crate::application::{PointerShieldBuilder, ViewNode};

/// Declarative pointer target attached to an owning view.
///
/// Pointer targets are transparent input surfaces sized to the owner. They let
/// callers declare drop, move, cancellation, or other pointer routing on the
/// semantic view that owns the behavior instead of manually building local
/// overlay stacks.
pub struct PointerTarget<Message> {
    input: ViewNode<Message>,
}

impl<Message> PointerTarget<Message> {
    fn new(input: ViewNode<Message>) -> Self {
        Self { input }
    }

    /// Assign a stable key to the target input surface.
    pub fn key(mut self, key: impl ToString) -> Self {
        self.input = self.input.key(key);
        self
    }

    pub(in crate::application) fn into_input(self) -> ViewNode<Message> {
        self.input
    }
}

/// Builder for transparent pointer targets attached to an owning view.
pub struct PointerTargetBuilder {
    shield: PointerShieldBuilder,
}

impl PointerTargetBuilder {
    /// Configure whether pointer movement is intercepted.
    pub fn pointer_move(mut self, enabled: bool) -> Self {
        self.shield = self.shield.pointer_move(enabled);
        self
    }

    /// Configure whether pointer press events are intercepted.
    pub fn pointer_press(mut self, enabled: bool) -> Self {
        self.shield = self.shield.pointer_press(enabled);
        self
    }

    /// Configure whether pointer release events are intercepted.
    pub fn pointer_release(mut self, enabled: bool) -> Self {
        self.shield = self.shield.pointer_release(enabled);
        self
    }

    /// Configure whether captured pointer drops are intercepted.
    pub fn pointer_drop(mut self, enabled: bool) -> Self {
        self.shield = self.shield.pointer_drop(enabled);
        self
    }

    /// Configure whether wheel input is intercepted.
    pub fn wheel(mut self, enabled: bool) -> Self {
        self.shield = self.shield.wheel(enabled);
        self
    }

    /// Emit host messages for selected pointer target outputs.
    pub fn filter_map<Message: 'static>(
        self,
        map: impl Fn(crate::widgets::PointerShieldMessage) -> Option<Message> + Send + Sync + 'static,
    ) -> PointerTarget<Message> {
        PointerTarget::new(self.shield.filter_map(map).input_only().fill())
    }

    /// Emit a cloned host message only when the target receives a pointer drop.
    pub fn on_drop<Message>(self, message: Message) -> PointerTarget<Message>
    where
        Message: Clone + Send + Sync + 'static,
    {
        PointerTarget::new(self.shield.on_drop(message).input_only().fill())
    }

    /// Emit a host message from pointer movement positions only.
    pub fn on_pointer_move<Message: 'static>(
        self,
        map: impl Fn(crate::gui::types::Point) -> Message + Send + Sync + 'static,
    ) -> PointerTarget<Message> {
        PointerTarget::new(self.shield.on_pointer_move(map).input_only().fill())
    }
}

/// Build a transparent pointer target with explicit event policy.
pub fn pointer_target(active: bool) -> PointerTargetBuilder {
    PointerTargetBuilder {
        shield: crate::application::pointer_shield(active),
    }
}

/// Build a pointer target that only reports pointer movement.
pub fn pointer_move_target(active: bool) -> PointerTargetBuilder {
    PointerTargetBuilder {
        shield: crate::application::pointer_move_shield(active),
    }
}

/// Build a pointer target that only reports captured pointer drops.
pub fn pointer_drop_target(active: bool) -> PointerTargetBuilder {
    PointerTargetBuilder {
        shield: crate::application::pointer_drop_shield(active),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{IntoView, PointerTarget, button, pointer_drop_target, row, text},
        gui::types::Point,
        layout::Vector2,
        runtime::{DeclarativeOwnedRuntimeBridge, SurfaceRuntime},
        widgets::{PointerButton, PointerModifiers, WidgetInput},
    };

    #[derive(Clone, Debug, PartialEq)]
    enum Message {
        Base,
        Drop,
    }

    fn drop_target() -> PointerTarget<Message> {
        pointer_drop_target(true).on_drop(Message::Drop)
    }

    #[test]
    fn view_pointer_target_routes_drop_above_owner_content() {
        let bridge = DeclarativeOwnedRuntimeBridge::new(
            Vec::<Message>::new(),
            |_| {
                button("Base")
                    .message(Message::Base)
                    .fill()
                    .pointer_target(drop_target())
                    .into_surface()
            },
            |state, message| state.push(message),
        );
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 40.0));

        runtime.dispatch_input_at(
            Point::new(8.0, 8.0),
            WidgetInput::PointerDrop {
                position: Point::new(8.0, 8.0),
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        );

        assert_eq!(runtime.bridge().state(), &[Message::Drop]);
    }

    #[test]
    fn view_pointer_target_is_bounded_to_owner_layout() {
        let bridge = DeclarativeOwnedRuntimeBridge::new(
            Vec::<Message>::new(),
            |_| {
                row([
                    text("Left").width(40.0).height(40.0),
                    button("Right")
                        .message(Message::Base)
                        .width(40.0)
                        .height(40.0)
                        .pointer_target(drop_target()),
                ])
                .spacing(0.0)
                .into_surface()
            },
            |state, message| state.push(message),
        );
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));

        runtime.dispatch_input_at(
            Point::new(8.0, 8.0),
            WidgetInput::PointerDrop {
                position: Point::new(8.0, 8.0),
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        );
        runtime.dispatch_input_at(
            Point::new(48.0, 8.0),
            WidgetInput::PointerDrop {
                position: Point::new(48.0, 8.0),
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        );

        assert_eq!(runtime.bridge().state(), &[Message::Drop]);
    }

    #[test]
    fn view_pointer_target_if_skips_disabled_target() {
        let bridge = DeclarativeOwnedRuntimeBridge::new(
            Vec::<Message>::new(),
            |_| {
                button("Base")
                    .message(Message::Base)
                    .fill()
                    .pointer_target_if(false, drop_target)
                    .into_surface()
            },
            |state, message| state.push(message),
        );
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 40.0));

        runtime.dispatch_input_at(
            Point::new(8.0, 8.0),
            WidgetInput::PointerDrop {
                position: Point::new(8.0, 8.0),
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        );

        assert!(runtime.bridge().state().is_empty());
    }

    #[test]
    fn view_pointer_target_if_routes_enabled_target() {
        let bridge = DeclarativeOwnedRuntimeBridge::new(
            Vec::<Message>::new(),
            |_| {
                button("Base")
                    .message(Message::Base)
                    .fill()
                    .pointer_target_if(true, drop_target)
                    .into_surface()
            },
            |state, message| state.push(message),
        );
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 40.0));

        runtime.dispatch_input_at(
            Point::new(8.0, 8.0),
            WidgetInput::PointerDrop {
                position: Point::new(8.0, 8.0),
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        );

        assert_eq!(runtime.bridge().state(), &[Message::Drop]);
    }

    #[test]
    fn view_pointer_target_preserves_owner_slot_sizing() {
        let bridge = DeclarativeOwnedRuntimeBridge::new(
            Vec::<Message>::new(),
            |_| {
                crate::application::column([
                    text::<Message>("Top").height(10.0),
                    text("Body").id(12_301).fill().pointer_target(drop_target()),
                    text("Bottom").height(10.0),
                ])
                .spacing(0.0)
                .fill()
                .into_surface()
            },
            |state, message| state.push(message),
        );
        let runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 100.0));
        let body_rect = runtime
            .layout()
            .rects
            .get(&12_301)
            .expect("body text should be laid out");

        assert_eq!(body_rect.height(), 80.0);
    }
}
