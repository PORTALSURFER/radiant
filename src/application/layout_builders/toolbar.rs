use crate::application::{ViewNode, row, spacer, stack};

/// Minimum height of the default toolbar and fixed height of default named parts.
pub const DEFAULT_TOOLBAR_HEIGHT: f32 = 34.0;
/// Default horizontal spacing between toolbar controls.
pub const DEFAULT_TOOLBAR_SPACING: f32 = 4.0;
/// Default horizontal toolbar padding.
pub const DEFAULT_TOOLBAR_PADDING_X: f32 = 0.0;
/// Default vertical toolbar padding.
pub const DEFAULT_TOOLBAR_PADDING_Y: f32 = 3.0;

/// Main control alignment for compact toolbar strips.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolbarAlignment {
    /// Place controls at the leading edge.
    #[default]
    Start,
    /// Center controls inside the toolbar.
    Center,
    /// Place controls at the trailing edge.
    End,
}

/// Named construction fields for compact toolbar/control-strip layout.
///
/// Toolbars are useful when application code owns the actual controls, while
/// Radiant owns the shared row spacing, padding, height, alignment, and optional
/// trailing group mechanics.
pub struct ToolbarParts<Message> {
    /// Main toolbar controls in declaration order.
    pub controls: Vec<ViewNode<Message>>,
    /// Optional trailing controls separated from the main controls by a fill spacer.
    pub trailing: Vec<ViewNode<Message>>,
    /// Fixed toolbar height.
    pub height: f32,
    /// Horizontal spacing between controls and spacer regions.
    pub spacing: f32,
    /// Horizontal toolbar padding.
    pub padding_x: f32,
    /// Vertical toolbar padding.
    pub padding_y: f32,
    /// Alignment for the main controls when no trailing group is present.
    pub alignment: ToolbarAlignment,
}

impl<Message> ToolbarParts<Message> {
    /// Build toolbar parts with Radiant's compact control-strip defaults.
    pub fn new(controls: impl IntoIterator<Item = ViewNode<Message>>) -> Self {
        Self {
            controls: controls.into_iter().collect(),
            trailing: Vec::new(),
            height: DEFAULT_TOOLBAR_HEIGHT,
            spacing: DEFAULT_TOOLBAR_SPACING,
            padding_x: DEFAULT_TOOLBAR_PADDING_X,
            padding_y: DEFAULT_TOOLBAR_PADDING_Y,
            alignment: ToolbarAlignment::Start,
        }
    }

    /// Add one trailing control separated from the main group by a fill spacer.
    pub fn trailing(mut self, control: ViewNode<Message>) -> Self {
        self.trailing.push(control);
        self
    }

    /// Replace trailing controls separated from the main group by a fill spacer.
    pub fn trailing_controls(
        mut self,
        controls: impl IntoIterator<Item = ViewNode<Message>>,
    ) -> Self {
        self.trailing = controls.into_iter().collect();
        self
    }

    /// Override fixed toolbar height.
    pub const fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Override horizontal control spacing.
    pub const fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Override horizontal toolbar padding.
    pub const fn padding_x(mut self, padding: f32) -> Self {
        self.padding_x = padding;
        self
    }

    /// Override vertical toolbar padding.
    pub const fn padding_y(mut self, padding: f32) -> Self {
        self.padding_y = padding;
        self
    }

    /// Override main control alignment when no trailing group is present.
    pub const fn alignment(mut self, alignment: ToolbarAlignment) -> Self {
        self.alignment = alignment;
        self
    }
}

/// Build a compact toolbar/control strip from app-owned controls.
///
/// The default strip grows with its controls' intrinsic heights, including text
/// scale, with a minimum height of 34 logical pixels. Use
/// [`toolbar_from_parts`] or an explicit `.height(...)` for a fixed physical slot.
pub fn toolbar<Message: 'static>(
    controls: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    stack([
        toolbar_from_parts(ToolbarParts::new(controls))
            .intrinsic()
            .fill_width(),
        spacer().width(0.0).height(DEFAULT_TOOLBAR_HEIGHT),
    ])
    .fill_width()
}

/// Build a compact toolbar/control strip from named parts.
pub fn toolbar_from_parts<Message: 'static>(parts: ToolbarParts<Message>) -> ViewNode<Message> {
    let ToolbarParts {
        controls,
        trailing,
        height,
        spacing,
        padding_x,
        padding_y,
        alignment,
    } = parts;
    row(toolbar_children(controls, trailing, alignment))
        .spacing(spacing)
        .padding_x(padding_x)
        .padding_y(padding_y)
        .fill_width()
        .height(height)
}

fn toolbar_children<Message: 'static>(
    controls: Vec<ViewNode<Message>>,
    trailing: Vec<ViewNode<Message>>,
    alignment: ToolbarAlignment,
) -> Vec<ViewNode<Message>> {
    let fill_spacer = || spacer().fill_width().height(1.0);
    let mut children = Vec::with_capacity(controls.len() + trailing.len() + 2);

    if trailing.is_empty() {
        match alignment {
            ToolbarAlignment::Start => {
                children.extend(controls);
            }
            ToolbarAlignment::Center => {
                children.push(fill_spacer());
                children.extend(controls);
                children.push(fill_spacer());
            }
            ToolbarAlignment::End => {
                children.push(fill_spacer());
                children.extend(controls);
            }
        }
        return children;
    }

    children.extend(controls);
    children.push(fill_spacer());
    children.extend(trailing);
    children
}

#[cfg(test)]
mod tests {
    use super::{ToolbarAlignment, ToolbarParts, toolbar_from_parts};
    use crate::{
        application::{IntoView, button, column, toolbar},
        layout::Vector2,
        widgets::{ButtonMessage, WidgetOutput},
    };

    #[derive(Clone, Debug, PartialEq)]
    enum Message {
        Run,
    }

    #[test]
    fn toolbar_applies_default_spacing_and_honors_control_height() {
        let layout = column([toolbar([
            button("A").message(Message::Run).id(10).width(32.0),
            button("B").message(Message::Run).id(11).width(32.0),
        ])
        .id(1)])
        .view_layout_at_size(Vector2::new(120.0, 50.0));

        assert_eq!(layout.rects[&1].height(), 42.0);
        assert_eq!(layout.rects[&10].height(), 36.0);
        assert!((layout.rects[&11].min.x - layout.rects[&10].max.x - 4.0).abs() < 0.01);
    }

    #[test]
    fn default_toolbar_grows_with_localized_controls_but_explicit_height_stays_physical() {
        use crate::{
            application::{ApplicationEnvironment, LocaleId, TextScale, WritingDirection},
            gui::types::Rect,
            runtime::UiSurface,
        };
        for scale in [1.0, 1.5, 2.0] {
            for direction in [WritingDirection::Ltr, WritingDirection::Rtl] {
                let controls = || [button("حفظ").message(Message::Run).id(10).width(100.0)];
                let environment = ApplicationEnvironment::new(LocaleId::new("ar").unwrap())
                    .with_text_scale(TextScale::new(scale).unwrap())
                    .with_writing_direction(direction);
                for (strip, expected_height) in [
                    (toolbar(controls()), 36.0 * scale + 6.0),
                    (toolbar_from_parts(ToolbarParts::new(controls())), 34.0),
                    (toolbar(controls()).height(40.0), 40.0),
                ] {
                    let surface = UiSurface::new(column([strip.id(1)]).into_node())
                        .with_application_environment(environment.clone());
                    let frame = surface.frame(
                        Rect::from_xy_size(0.0, 0.0, 400.0, 200.0),
                        &Default::default(),
                    );
                    assert_eq!(frame.layout.rects[&1].height(), expected_height);
                    let text = frame.paint_plan.first_text_run("حفظ").unwrap();
                    assert_eq!(text.font_size, 14.0 * scale);
                    assert_eq!(
                        surface
                            .find_widget(10)
                            .unwrap()
                            .widget()
                            .automation_semantics()
                            .label
                            .as_deref(),
                        Some("حفظ")
                    );
                }
            }
        }
    }

    #[test]
    fn toolbar_trailing_controls_are_separated_by_fill_space() {
        let layout = toolbar_from_parts(
            ToolbarParts::new([button("Left").message(Message::Run).id(10).width(40.0)])
                .trailing(button("Right").message(Message::Run).id(11).width(48.0))
                .height(30.0)
                .padding_x(8.0)
                .padding_y(4.0)
                .spacing(6.0),
        )
        .id(1)
        .view_layout_at_size(Vector2::new(180.0, 30.0));

        assert_eq!(layout.rects[&1].height(), 30.0);
        assert_eq!(layout.rects[&10].min.x, 8.0);
        assert!(layout.rects[&11].min.x > layout.rects[&10].max.x + 40.0);
        assert_eq!(layout.rects[&11].max.x, 172.0);
    }

    #[test]
    fn toolbar_can_align_controls_to_the_end() {
        let layout = toolbar_from_parts(
            ToolbarParts::new([button("Run").message(Message::Run).id(10).width(40.0)])
                .alignment(ToolbarAlignment::End),
        )
        .id(1)
        .view_layout_at_size(Vector2::new(140.0, 34.0));

        assert_eq!(layout.rects[&10].max.x, 140.0);
    }

    #[test]
    fn toolbar_routes_child_control_messages() {
        let surface = toolbar([button("Run").message(Message::Run).id(10).width(40.0)])
            .id(1)
            .into_surface();

        assert_eq!(
            surface.dispatch_widget_output(
                10,
                WidgetOutput::typed(ButtonMessage::Activate {
                    provenance: crate::widgets::InteractionProvenance::Programmatic,
                }),
            ),
            Some(Message::Run)
        );
    }
}
