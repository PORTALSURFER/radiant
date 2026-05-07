//! Runtime state and event routing for the generic native Vello runner.

use crate::gui::{
    focus::FocusSurface,
    input::KeyPress,
    types::{Point, Vector2},
};
use crate::runtime::{RuntimeBridge, SurfaceRuntime};
use crate::theme::ThemeTokens;
use crate::widgets::{PointerButton, TextEditCommand, WidgetInput, WidgetKey};

pub(in crate::gui_runtime::native_vello) struct GenericNativeRuntimeCore<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::gui_runtime::native_vello) runtime: SurfaceRuntime<Bridge, Message>,
    theme: ThemeTokens,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct GenericRouteOutcome {
    pub(in crate::gui_runtime::native_vello) routed: bool,
    pub(in crate::gui_runtime::native_vello) repaint_requested: bool,
}

impl GenericRouteOutcome {
    pub(super) fn needs_redraw(self) -> bool {
        self.routed || self.repaint_requested
    }
}

impl<Bridge, Message> GenericNativeRuntimeCore<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::gui_runtime::native_vello) fn new(bridge: Bridge, viewport: Vector2) -> Self {
        Self {
            runtime: SurfaceRuntime::new(bridge, viewport),
            theme: ThemeTokens::default(),
        }
    }

    pub(super) fn set_viewport(&mut self, viewport: Vector2) {
        let _ = self
            .runtime
            .dispatch_event(crate::runtime::Event::Resize { viewport });
    }

    pub(super) fn paint_plan(&self) -> crate::runtime::SurfacePaintPlan {
        self.runtime.paint_plan(&self.theme)
    }

    pub(super) fn refresh_surface(&mut self) {
        self.runtime.refresh();
    }

    pub(super) fn needs_animation(&mut self) -> bool {
        self.runtime.bridge_mut().needs_animation()
    }

    fn route_outcome(&mut self, routed: bool) -> GenericRouteOutcome {
        GenericRouteOutcome {
            routed,
            repaint_requested: self.runtime.take_repaint_requested(),
        }
    }

    pub(in crate::gui_runtime::native_vello) fn route_pointer_move(
        &mut self,
        position: Point,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(crate::runtime::Event::PointerMove { position })
            .is_some();
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_pointer_press(
        &mut self,
        position: Point,
        button: PointerButton,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(crate::runtime::Event::PointerPress { position, button })
            .is_some();
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_pointer_release(
        &mut self,
        position: Point,
        button: PointerButton,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(crate::runtime::Event::PointerRelease { position, button })
            .is_some();
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_scroll(
        &mut self,
        position: Point,
        delta: Vector2,
    ) -> GenericRouteOutcome {
        let routed = self.runtime.wheel_or_scroll_at(position, delta);
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_key_press(
        &mut self,
        press: KeyPress,
        widget_key: Option<WidgetKey>,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_key_press(press, widget_key, FocusSurface::None);
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_widget_key(
        &mut self,
        key: WidgetKey,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(crate::runtime::Event::KeyPress(key))
            .is_some();
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn route_text_edit(
        &mut self,
        command: TextEditCommand,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_focused_input(WidgetInput::TextEdit(command))
            .is_some();
        self.route_outcome(routed)
    }

    pub(in crate::gui_runtime::native_vello) fn focused_text_selection(&self) -> Option<String> {
        self.runtime.focused_text_selection()
    }

    pub(in crate::gui_runtime::native_vello) fn has_focused_text_input(&self) -> bool {
        self.runtime.focused_text_input_id().is_some()
    }

    pub(in crate::gui_runtime::native_vello) fn route_character(
        &mut self,
        character: char,
    ) -> GenericRouteOutcome {
        let routed = self
            .runtime
            .dispatch_event(crate::runtime::Event::Character(character))
            .is_some();
        self.route_outcome(routed)
    }
}
