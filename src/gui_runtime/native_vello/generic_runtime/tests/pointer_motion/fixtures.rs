use crate::{
    application::IntoView,
    gui::list::{TreeGuideRow, TreeGuideStyle, VirtualListWindow},
    layout::{Point, Rect, Vector2},
    runtime::{
        Command, PaintFillRect, PaintPrimitive, RepaintScope, RuntimeBridge, SurfaceNode,
        UiSurface, WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{FocusBehavior, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing},
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PointerMoveMessage;

#[derive(Clone, Debug)]
pub(super) struct PointerMoveWidget {
    common: WidgetCommon,
}

impl PointerMoveWidget {
    fn new() -> Self {
        Self {
            common: WidgetCommon::new(71, WidgetSizing::fixed(Vector2::new(80.0, 24.0))),
        }
    }
}

impl Widget for PointerMoveWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        matches!(input, WidgetInput::PointerMove { .. })
            .then(|| WidgetOutput::typed(PointerMoveMessage))
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &crate::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

#[derive(Default)]
pub(super) struct PointerMoveBridge {
    pub(super) moves: usize,
    pub(super) project_count: usize,
    pub(super) request_repaint_on_update: bool,
    pub(super) repaint_scope: Option<RepaintScope>,
}

impl RuntimeBridge<PointerMoveMessage> for PointerMoveBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<PointerMoveMessage>> {
        self.project_count += 1;
        Arc::new(UiSurface::new(SurfaceNode::custom_widget(
            PointerMoveWidget::new(),
            WidgetMessageMapper::typed(|message: PointerMoveMessage| message),
        )))
    }

    fn update(&mut self, _message: PointerMoveMessage) -> Command<PointerMoveMessage> {
        self.moves += 1;
        if let Some(scope) = self.repaint_scope {
            Command::repaint(scope)
        } else if self.request_repaint_on_update {
            Command::request_repaint()
        } else {
            Command::none()
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct LocalPointerMoveWidget {
    common: WidgetCommon,
    last_position: Option<Point>,
}

impl LocalPointerMoveWidget {
    fn new() -> Self {
        let mut common = WidgetCommon::new(72, WidgetSizing::fixed(Vector2::new(80.0, 24.0)));
        common.focus = FocusBehavior::Pointer;
        Self {
            common,
            last_position: None,
        }
    }
}

impl Widget for LocalPointerMoveWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        if let WidgetInput::PointerMove { position } = input {
            self.last_position = Some(position);
        }
        None
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &crate::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

pub(super) struct LocalPointerMoveBridge;

impl RuntimeBridge<()> for LocalPointerMoveBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::custom_widget(
            LocalPointerMoveWidget::new(),
            WidgetMessageMapper::none(),
        )))
    }

    fn reduce_message(&mut self, _message: ()) {}
}

#[derive(Clone, Debug)]
pub(super) struct PaintOnlyPointerMoveWidget {
    common: WidgetCommon,
    last_position: Option<Point>,
}

impl PaintOnlyPointerMoveWidget {
    fn new() -> Self {
        let mut common = WidgetCommon::new(73, WidgetSizing::fixed(Vector2::new(80.0, 24.0)));
        common.focus = FocusBehavior::Pointer;
        Self {
            common,
            last_position: None,
        }
    }
}

impl Widget for PaintOnlyPointerMoveWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        true
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        if let WidgetInput::PointerMove { position } = input {
            self.last_position = Some(position);
        }
        None
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &crate::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &crate::layout::LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let Some(position) = self.last_position else {
            return;
        };
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: Rect::from_min_size(Point::new(position.x - 1.0, 0.0), Vector2::new(2.0, 24.0)),
            color: theme.highlight_orange,
        }));
    }
}

pub(super) struct PaintOnlyPointerMoveBridge;

impl RuntimeBridge<()> for PaintOnlyPointerMoveBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::custom_widget(
            PaintOnlyPointerMoveWidget::new(),
            WidgetMessageMapper::none(),
        )))
    }

    fn reduce_message(&mut self, _message: ()) {}
}

pub(super) struct AdjacentTreeRowsBridge;

impl RuntimeBridge<()> for AdjacentTreeRowsBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        use crate::application::{column, row_actions, tree_row};

        Arc::new(
            column([
                tree_row("One")
                    .input_id(81)
                    .row_height(22.0)
                    .interactive_actions(row_actions()),
                tree_row("Two")
                    .input_id(82)
                    .row_height(22.0)
                    .interactive_actions(row_actions()),
            ])
            .spacing(0.0)
            .into_surface(),
        )
    }

    fn reduce_message(&mut self, _message: ()) {}
}

#[derive(Default)]
pub(super) struct QuietInteractiveRowBridge {
    pub(super) project_count: usize,
    pub(super) update_count: usize,
}

impl RuntimeBridge<()> for QuietInteractiveRowBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        use crate::application::{row_actions, tree_row};

        self.project_count += 1;
        Arc::new(
            tree_row("Quiet row")
                .input_id(85)
                .row_height(22.0)
                .interactive_actions(row_actions())
                .into_surface(),
        )
    }

    fn update(&mut self, _message: ()) -> Command<()> {
        self.update_count += 1;
        Command::none()
    }
}

pub(super) struct DisclosureAndTreeRowBridge;

impl RuntimeBridge<()> for DisclosureAndTreeRowBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        use crate::application::{disclosure_button, row, row_actions, tree_row};

        Arc::new(
            row([
                disclosure_button(false)
                    .subtle()
                    .mapped(|_| ())
                    .id(83)
                    .size(28.0, 22.0),
                tree_row("Folder")
                    .input_id(84)
                    .row_height(22.0)
                    .interactive_actions(row_actions()),
            ])
            .spacing(1.0)
            .into_surface(),
        )
    }

    fn reduce_message(&mut self, _message: ()) {}
}

pub(super) struct VirtualTreeRowsBridge;

impl RuntimeBridge<()> for VirtualTreeRowsBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        use crate::{
            application::{row_actions, tree_row, virtual_tree_list_window},
            gui::types::Rgba8,
        };

        let window = VirtualListWindow {
            total_items: 3,
            viewport_start: 0,
            viewport_end: 3,
            window_start: 0,
            window_end: 3,
        };
        let guide_rows = [
            TreeGuideRow::new(0, true),
            TreeGuideRow::new(1, false),
            TreeGuideRow::new(1, false),
        ];
        let labels = ["Root", "One", "Two"];

        Arc::new(
            virtual_tree_list_window(
                window,
                22.0,
                &guide_rows,
                TreeGuideStyle::new(12.0, 22.0, Rgba8::new(90, 120, 160, 255)),
                |index| {
                    tree_row(labels[index])
                        .row_key(format!("folder-row-{index}"))
                        .hit_key(format!("folder-row-hit-{index}"))
                        .depth(guide_rows[index].depth)
                        .has_children(guide_rows[index].starts_descendant_group)
                        .expanded(true)
                        .row_height(22.0)
                        .interactive_actions(row_actions())
                },
                22.0,
            )
            .into_surface(),
        )
    }

    fn reduce_message(&mut self, _message: ()) {}
}
