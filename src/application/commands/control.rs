//! Command-backed controls reuse one presentation and defer mapping until activation.
use super::{CommandActivation, CommandPresentation, CommandSource};
use crate::{
    application::{MappedWidget, ViewNode, default_button_sizing, view_node_from_widget},
    gui::types::Rect,
    layout::{LayoutNode, LayoutOutput},
    runtime::{AutomationRole, PaintPrimitive, ResolvedEnvironment, WidgetMessageMapper},
    theme::ThemeTokens,
    widgets::{
        ButtonWidget, FocusedKeyDisposition, TextScaleParticipation, Widget, WidgetCapabilities,
        WidgetCapabilitiesV2, WidgetCommon, WidgetInput, WidgetKey, WidgetOutput,
        WidgetPaintContext, WidgetSemantics, WidgetSemanticsRevision,
    },
};

impl CommandPresentation {
    /// Capture an opaque activation for a control or native adapter.
    pub fn activation(&self, source: CommandSource) -> Option<CommandActivation> {
        self.enabled
            .then(|| self.target.clone())
            .flatten()
            .map(|target| CommandActivation { target, source })
    }

    /// Build a toolbar button without storing a domain message or duplicating metadata.
    pub fn toolbar_button<Message: 'static>(self) -> ViewNode<Message> {
        self.control(CommandSource::Toolbar)
    }

    /// Build a command menu row from this same metadata and target.
    pub fn menu_item<Message: 'static>(self) -> ViewNode<Message> {
        self.control(CommandSource::Menu).fill_width()
    }

    /// Build a command-palette row from this same metadata and target.
    pub fn palette_item<Message: 'static>(self) -> ViewNode<Message> {
        self.control(CommandSource::Palette).fill_width()
    }

    /// Build passive shortcut help using the registry label and expanded platform key names.
    pub fn shortcut_help<Message: 'static>(self) -> ViewNode<Message> {
        let mut rows = vec![crate::application::text(self.label)];
        rows.extend(
            self.shortcuts
                .into_iter()
                .map(|shortcut| crate::application::text(shortcut.spoken)),
        );
        crate::application::column(rows)
    }

    fn control<Message: 'static>(self, source: CommandSource) -> ViewNode<Message> {
        let activation = self.activation(source);
        let sizing = default_button_sizing(&self.label);
        let mut button = ButtonWidget::new(0, self.label, sizing);
        button.common.state.disabled = activation.is_none();
        button.common.state.selected = self.checked.unwrap_or(false);
        if let Some(shortcut) = self.shortcuts.first() {
            button = button.with_trailing_label(shortcut.compact.clone());
        }
        let retained_activation = activation.clone();
        let messages = activation
            .map(WidgetMessageMapper::command)
            .unwrap_or_else(WidgetMessageMapper::none);
        view_node_from_widget(MappedWidget::new(
            CommandControlWidget {
                button,
                activation: retained_activation,
                accessibility: self.accessibility,
                description: self.description,
                shortcut: self
                    .shortcuts
                    .first()
                    .map(|shortcut| shortcut.spoken.clone()),
            },
            messages,
        ))
    }
}

#[derive(Clone)]
struct CommandControlWidget {
    button: ButtonWidget,
    activation: Option<CommandActivation>,
    accessibility: String,
    description: Option<String>,
    shortcut: Option<String>,
}
impl WidgetSemantics for CommandControlWidget {
    fn revision(&self) -> WidgetSemanticsRevision {
        WidgetSemanticsRevision::exact((
            self.accessibility.clone(),
            self.description.clone(),
            self.shortcut.clone(),
            self.button.common.state.disabled,
            self.button.common.state.selected,
        ))
    }
    fn automation_role(&self) -> AutomationRole {
        AutomationRole::Button
    }
    fn automation_label(&self) -> Option<String> {
        Some(self.accessibility.clone())
    }
    fn automation_description(&self) -> Option<String> {
        match (&self.description, &self.shortcut) {
            (Some(description), Some(shortcut)) => Some(format!("{description}. {shortcut}")),
            (Some(text), None) | (None, Some(text)) => Some(text.clone()),
            (None, None) => None,
        }
    }
}
impl Widget for CommandControlWidget {
    fn common(&self) -> &WidgetCommon {
        &self.button.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.button.common
    }
    fn focused_key_disposition(&self, key: WidgetKey) -> FocusedKeyDisposition {
        self.button.focused_key_disposition(key)
    }
    fn text_scale_participation(&self) -> TextScaleParticipation {
        self.button.text_scale_participation()
    }
    fn layout_node_with_environment(&self, environment: &ResolvedEnvironment) -> LayoutNode {
        self.button.layout_node_with_environment(environment)
    }
    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        Widget::handle_input(&mut self.button, bounds, input)
    }
    fn handle_pointer_capture_cancelled(&mut self, bounds: Rect) -> Option<WidgetOutput> {
        self.button.handle_pointer_capture_cancelled(bounds)
    }
    fn needs_state_synchronization(&self) -> bool {
        true
    }
    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.button.synchronize_from_previous(&previous.button);
            let same_target = match (&self.activation, &previous.activation) {
                (Some(current), Some(previous)) => {
                    current.source == previous.source
                        && std::sync::Arc::ptr_eq(
                            &current.target.registry,
                            &previous.target.registry,
                        )
                        && std::rc::Rc::ptr_eq(
                            &current.target.scope_identity,
                            &previous.target.scope_identity,
                        )
                        && current.target.scope == previous.target.scope
                        && current.target.command == previous.target.command
                }
                (None, None) => true,
                _ => false,
            };
            if !same_target {
                self.button.state = Default::default();
                self.button.common.state.pressed = false;
            }
        }
    }
    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new().semantics(self)
    }
    fn capabilities_v2(&self) -> WidgetCapabilitiesV2<'_> {
        self.button.capabilities_v2()
    }
    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.button.append_paint(primitives, bounds, layout, theme);
    }
    fn append_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
        self.button.append_paint_with_context(context);
    }
}
