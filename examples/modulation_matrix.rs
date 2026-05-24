//! Modulation matrix sandbox for DAW-style routing GUI interaction.

use radiant::prelude::*;
use radiant::{
    runtime::{PaintFillRect, PaintStrokeRect},
    widgets::PaintBounds,
};

const MATRIX_WIDGET_ID: u64 = 94;
const STATUS_WIDGET_ID: u64 = 95;
const SOURCE_COUNT: usize = 6;
const DESTINATION_COUNT: usize = 8;
const DATA_SOURCE_NOTE: &str = "without_synth_or_dsp";

#[derive(Clone, Debug)]
struct ModulationMatrixState {
    running: bool,
    frame: u64,
    activity_phase: f32,
    selected: MatrixCell,
    amounts: [[f32; DESTINATION_COUNT]; SOURCE_COUNT],
}

impl Default for ModulationMatrixState {
    fn default() -> Self {
        Self {
            running: true,
            frame: 0,
            activity_phase: 0.0,
            selected: MatrixCell {
                source: 0,
                destination: 2,
            },
            amounts: seeded_amounts(),
        }
    }
}

impl ModulationMatrixState {
    fn tick(&mut self) {
        if !self.running {
            return;
        }
        self.frame = self.frame.saturating_add(1);
        self.activity_phase = (self.activity_phase + 0.035) % 1.0;
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn selected_amount(&self) -> f32 {
        self.amounts[self.selected.source][self.selected.destination]
    }

    fn status(&self) -> String {
        let transport = if self.running { "running" } else { "paused" };
        format!(
            "{transport} | frame {} | {} -> {} | {:+.0}% | synthetic GUI routing",
            self.frame,
            SOURCES[self.selected.source],
            DESTINATIONS[self.selected.destination],
            self.selected_amount() * 100.0
        )
    }

    fn apply_matrix_message(&mut self, message: MatrixMessage) {
        match message {
            MatrixMessage::SetAmount { cell, amount } => {
                let cell = cell.clamped();
                self.amounts[cell.source][cell.destination] = amount.clamp(-1.0, 1.0);
                self.selected = cell;
            }
            MatrixMessage::ClearSelected => {
                self.amounts[self.selected.source][self.selected.destination] = 0.0;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MatrixCell {
    source: usize,
    destination: usize,
}

impl MatrixCell {
    fn clamped(self) -> Self {
        Self {
            source: self.source.min(SOURCE_COUNT - 1),
            destination: self.destination.min(DESTINATION_COUNT - 1),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum AppMessage {
    Frame,
    ToggleRun,
    Reset,
    Matrix(MatrixMessage),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum MatrixMessage {
    SetAmount { cell: MatrixCell, amount: f32 },
    ClearSelected,
}

const SOURCES: [&str; SOURCE_COUNT] = ["LFO 1", "LFO 2", "Env 1", "Env 2", "Vel", "Mod"];
const DESTINATIONS: [&str; DESTINATION_COUNT] = [
    "Cutoff", "Res", "Drive", "Pan", "Pitch", "PWM", "Mix", "Level",
];

fn seeded_amounts() -> [[f32; DESTINATION_COUNT]; SOURCE_COUNT] {
    let mut amounts = [[0.0; DESTINATION_COUNT]; SOURCE_COUNT];
    amounts[0][0] = 0.64;
    amounts[0][3] = 0.28;
    amounts[1][4] = -0.32;
    amounts[2][0] = 0.78;
    amounts[2][6] = 0.36;
    amounts[3][2] = -0.46;
    amounts[4][7] = 0.52;
    amounts[5][1] = 0.24;
    amounts
}

fn main() -> radiant::Result {
    radiant::app(ModulationMatrixState::default())
        .title("Radiant Modulation Matrix")
        .size(1040, 620)
        .min_size(820, 500)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| AppMessage::Frame)
        .update(update)
        .run()
}

fn project_surface(state: &mut ModulationMatrixState) -> View<AppMessage> {
    column([
        row([
            text("Modulation Matrix").height(30.0).fill_width(),
            button(if state.running { "Pause" } else { "Run" })
                .primary()
                .message(AppMessage::ToggleRun)
                .size(88.0, 30.0),
            button("Clear")
                .subtle()
                .message(AppMessage::Matrix(MatrixMessage::ClearSelected))
                .size(82.0, 30.0),
            button("Reset")
                .subtle()
                .message(AppMessage::Reset)
                .size(82.0, 30.0),
        ])
        .fill_width()
        .spacing(10.0),
        custom_widget_mapped(
            ModulationMatrixWidget::new(state.amounts, state.selected, state.activity_phase),
            AppMessage::Matrix,
        )
        .id(MATRIX_WIDGET_ID)
        .height(390.0)
        .fill_width(),
        row([
            stat_tile("Sources", SOURCE_COUNT.to_string()),
            stat_tile("Destinations", DESTINATION_COUNT.to_string()),
            stat_tile(
                "Selected",
                format!("{}%", (state.selected_amount() * 100.0).round()),
            ),
            stat_tile("Source", DATA_SOURCE_NOTE),
            text(state.status())
                .id(STATUS_WIDGET_ID)
                .height(68.0)
                .fill_width(),
        ])
        .fill_width()
        .spacing(10.0),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

fn stat_tile(label: impl Into<String>, value: impl Into<String>) -> View<AppMessage> {
    column([
        text(label.into()).height(22.0).fill_width(),
        text(value.into()).height(24.0).fill_width(),
    ])
    .style(WidgetStyle {
        tone: WidgetTone::Neutral,
        prominence: WidgetProminence::Subtle,
    })
    .padding(10.0)
    .spacing(4.0)
    .height(68.0)
    .fill_width()
}

fn update(state: &mut ModulationMatrixState, message: AppMessage) {
    match message {
        AppMessage::Frame => state.tick(),
        AppMessage::ToggleRun => {
            state.running = !state.running;
        }
        AppMessage::Reset => state.reset(),
        AppMessage::Matrix(message) => state.apply_matrix_message(message),
    }
}

#[derive(Clone, Debug)]
struct ModulationMatrixWidget {
    common: WidgetCommon,
    amounts: [[f32; DESTINATION_COUNT]; SOURCE_COUNT],
    selected: MatrixCell,
    activity_phase: f32,
    hover_cell: Option<MatrixCell>,
    hover_position: Option<Point>,
    drag_cell: Option<MatrixCell>,
}

impl ModulationMatrixWidget {
    fn new(
        amounts: [[f32; DESTINATION_COUNT]; SOURCE_COUNT],
        selected: MatrixCell,
        activity_phase: f32,
    ) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::new(Vector2::new(760.0, 340.0), Vector2::new(1000.0, 390.0)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            amounts,
            selected: selected.clamped(),
            activity_phase,
            hover_cell: None,
            hover_position: None,
            drag_cell: None,
        }
    }

    fn matrix_rect(&self, bounds: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(bounds.min.x + 112.0, bounds.min.y + 54.0),
            Point::new(bounds.max.x - 18.0, bounds.max.y - 22.0),
        )
    }

    fn source_label_rect(&self, bounds: Rect, source: usize) -> Rect {
        let matrix = self.matrix_rect(bounds);
        let cell = self.cell_rect(
            matrix,
            MatrixCell {
                source,
                destination: 0,
            },
        );
        Rect::from_min_max(
            Point::new(bounds.min.x + 12.0, cell.min.y),
            Point::new(matrix.min.x - 8.0, cell.max.y),
        )
    }

    fn destination_label_rect(&self, matrix: Rect, destination: usize) -> Rect {
        let cell = self.cell_rect(
            matrix,
            MatrixCell {
                source: 0,
                destination,
            },
        );
        Rect::from_min_max(
            Point::new(cell.min.x, matrix.min.y - 38.0),
            Point::new(cell.max.x, matrix.min.y - 6.0),
        )
    }

    fn cell_rect(&self, matrix: Rect, cell: MatrixCell) -> Rect {
        let width = matrix.width() / DESTINATION_COUNT as f32;
        let height = matrix.height() / SOURCE_COUNT as f32;
        let x = matrix.min.x + cell.destination as f32 * width;
        let y = matrix.min.y + cell.source as f32 * height;
        Rect::from_min_size(Point::new(x, y), Vector2::new(width, height))
    }

    fn cell_at_position(&self, matrix: Rect, position: Point) -> Option<MatrixCell> {
        if !matrix.contains(position) {
            return None;
        }
        let destination =
            ((position.x - matrix.min.x) / (matrix.width() / DESTINATION_COUNT as f32)) as usize;
        let source =
            ((position.y - matrix.min.y) / (matrix.height() / SOURCE_COUNT as f32)) as usize;
        Some(
            MatrixCell {
                source,
                destination,
            }
            .clamped(),
        )
    }

    fn amount_for_position(&self, rect: Rect, position: Point) -> f32 {
        let ratio = ((rect.max.y - position.y) / rect.height().max(1.0)).clamp(0.0, 1.0);
        ratio * 2.0 - 1.0
    }
}

impl Widget for ModulationMatrixWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        let matrix = self.matrix_rect(bounds);
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                self.hover_position = matrix.contains(position).then_some(position);
                self.hover_cell = self.cell_at_position(matrix, position);
                if let Some(cell) = self.drag_cell {
                    let rect = self.cell_rect(matrix, cell);
                    return Some(WidgetOutput::custom(MatrixMessage::SetAmount {
                        cell,
                        amount: self.amount_for_position(rect, position),
                    }));
                }
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if matrix.contains(position) => {
                let cell = self.cell_at_position(matrix, position)?;
                let rect = self.cell_rect(matrix, cell);
                self.selected = cell;
                self.hover_cell = Some(cell);
                self.drag_cell = Some(cell);
                Some(WidgetOutput::custom(MatrixMessage::SetAmount {
                    cell,
                    amount: self.amount_for_position(rect, position),
                }))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            }
            | WidgetInput::PointerDrop {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                let drag = self.drag_cell.take();
                self.hover_cell = self.cell_at_position(matrix, position);
                drag.map(|cell| {
                    let rect = self.cell_rect(matrix, cell);
                    WidgetOutput::custom(MatrixMessage::SetAmount {
                        cell,
                        amount: self.amount_for_position(rect, position),
                    })
                })
            }
            WidgetInput::KeyPress(WidgetKey::Delete | WidgetKey::Backspace)
                if self.common.state.focused =>
            {
                Some(WidgetOutput::custom(MatrixMessage::ClearSelected))
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            _ => None,
        }
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        self.drag_cell.is_none()
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            self.hover_cell = previous.hover_cell;
            self.hover_position = previous.hover_position;
            self.drag_cell = previous.drag_cell;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let matrix = self.matrix_rect(bounds);
        push_rect(primitives, self.common.id, bounds, theme.bg_secondary);
        for (source, label) in SOURCES.iter().enumerate() {
            push_text(
                primitives,
                self.common.id,
                *label,
                self.source_label_rect(bounds, source),
                theme.text_muted,
                PaintTextAlign::Right,
            );
        }
        for (destination, label) in DESTINATIONS.iter().enumerate() {
            push_text(
                primitives,
                self.common.id,
                *label,
                self.destination_label_rect(matrix, destination),
                theme.text_muted,
                PaintTextAlign::Center,
            );
        }
        for source in 0..SOURCE_COUNT {
            for destination in 0..DESTINATION_COUNT {
                self.append_cell(
                    primitives,
                    matrix,
                    MatrixCell {
                        source,
                        destination,
                    },
                    theme,
                );
            }
        }
        push_stroke(
            primitives,
            self.common.id,
            matrix,
            theme.border_emphasis,
            1.0,
        );
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let matrix = self.matrix_rect(bounds);
        if let Some(position) = self.hover_position
            && matrix.contains(position)
        {
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(
                    Point::new(position.x, matrix.min.y),
                    Point::new(position.x + 1.0, matrix.max.y),
                ),
                translucent(theme.text_muted, 70),
            );
        }
        if let Some(cell) = self.hover_cell {
            push_stroke(
                primitives,
                self.common.id,
                self.cell_rect(matrix, cell),
                translucent(theme.highlight_cyan, 190),
                2.0,
            );
        }
        self.append_activity_pulses(primitives, matrix, theme);
    }
}

impl ModulationMatrixWidget {
    fn append_cell(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        matrix: Rect,
        cell: MatrixCell,
        theme: &ThemeTokens,
    ) {
        let rect = self.cell_rect(matrix, cell);
        let amount = self.amounts[cell.source][cell.destination];
        let selected = self.selected == cell;
        push_rect(
            primitives,
            self.common.id,
            rect,
            if selected {
                blend_color(theme.surface_raised, theme.highlight_blue, 0.18)
            } else {
                theme.surface_base
            },
        );
        push_stroke(
            primitives,
            self.common.id,
            rect,
            translucent(theme.border, 130),
            1.0,
        );
        let center_y = rect.center().y;
        push_rect(
            primitives,
            self.common.id,
            Rect::from_min_max(
                Point::new(rect.min.x + 8.0, center_y),
                Point::new(rect.max.x - 8.0, center_y + 1.0),
            ),
            translucent(theme.grid_soft, 140),
        );
        if amount.abs() > 0.01 {
            let bar = amount_bar_rect(rect, amount);
            push_rect(
                primitives,
                self.common.id,
                bar,
                if amount >= 0.0 {
                    theme.highlight_cyan
                } else {
                    theme.highlight_orange
                },
            );
            push_text(
                primitives,
                self.common.id,
                format!("{:+.0}", amount * 100.0),
                rect,
                theme.text_primary,
                PaintTextAlign::Center,
            );
        }
    }

    fn append_activity_pulses(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        matrix: Rect,
        theme: &ThemeTokens,
    ) {
        for source in 0..SOURCE_COUNT {
            for destination in 0..DESTINATION_COUNT {
                let amount = self.amounts[source][destination];
                if amount.abs() < 0.20 {
                    continue;
                }
                let cell = MatrixCell {
                    source,
                    destination,
                };
                let rect = self.cell_rect(matrix, cell);
                let phase =
                    (self.activity_phase + source as f32 * 0.11 + destination as f32 * 0.07)
                        .fract();
                let x = rect.min.x + 8.0 + (rect.width() - 16.0) * phase;
                push_rect(
                    primitives,
                    self.common.id,
                    Rect::from_min_size(
                        Point::new(x, rect.min.y + 7.0),
                        Vector2::new(4.0, rect.height() - 14.0),
                    ),
                    translucent(theme.text_primary, 70),
                );
            }
        }
    }
}

fn amount_bar_rect(rect: Rect, amount: f32) -> Rect {
    let center = rect.center().y;
    let available = rect.height() * 0.44;
    if amount >= 0.0 {
        Rect::from_min_max(
            Point::new(rect.min.x + 12.0, center - available * amount),
            Point::new(rect.max.x - 12.0, center),
        )
    } else {
        Rect::from_min_max(
            Point::new(rect.min.x + 12.0, center),
            Point::new(rect.max.x - 12.0, center + available * amount.abs()),
        )
    }
}

fn blend_color(a: Rgba8, b: Rgba8, t: f32) -> Rgba8 {
    let t = t.clamp(0.0, 1.0);
    rgba(
        (a.r as f32 + (b.r as f32 - a.r as f32) * t).round() as u8,
        (a.g as f32 + (b.g as f32 - a.g as f32) * t).round() as u8,
        (a.b as f32 + (b.b as f32 - a.b as f32) * t).round() as u8,
        255,
    )
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

fn translucent(mut color: Rgba8, alpha: u8) -> Rgba8 {
    color.a = alpha;
    color
}

fn push_rect(primitives: &mut Vec<PaintPrimitive>, widget_id: u64, rect: Rect, color: Rgba8) {
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color,
    }));
}

fn push_stroke(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    rect: Rect,
    color: Rgba8,
    width: f32,
) {
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect,
        color,
        width,
    }));
}

fn push_text(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    text: impl Into<String>,
    rect: Rect,
    color: Rgba8,
    align: PaintTextAlign,
) {
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text: text.into().into(),
        rect,
        font_size: 12.0,
        baseline: Some(16.0),
        color,
        align,
        wrap: TextWrap::None,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::runtime::{RuntimeBridge, SurfaceRuntime};

    #[test]
    fn modulation_matrix_tick_advances_synthetic_activity_without_synth_or_dsp() {
        let mut state = ModulationMatrixState::default();
        let initial = state.activity_phase;

        state.tick();

        assert_eq!(state.frame, 1);
        assert!(state.activity_phase > initial);
        assert_eq!(DATA_SOURCE_NOTE, "without_synth_or_dsp");
    }

    #[test]
    fn modulation_matrix_widget_paints_sources_destinations_and_amounts() {
        let state = ModulationMatrixState::default();
        let widget =
            ModulationMatrixWidget::new(state.amounts, state.selected, state.activity_phase);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        assert!(
            primitives
                .iter()
                .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(_)))
                .count()
                >= SOURCE_COUNT * DESTINATION_COUNT
        );
        assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str() == "LFO 1"))
        );
        assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str() == "Cutoff"))
        );
    }

    #[test]
    fn modulation_matrix_drag_routes_bipolar_amount_change() {
        let state = ModulationMatrixState::default();
        let mut widget =
            ModulationMatrixWidget::new(state.amounts, state.selected, state.activity_phase);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
        let matrix = widget.matrix_rect(bounds);
        let cell = MatrixCell {
            source: 2,
            destination: 4,
        };
        let rect = widget.cell_rect(matrix, cell);

        let output = widget.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(rect.center().x, rect.min.y + 1.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );

        assert_eq!(
            output.and_then(|output| output.typed_ref::<MatrixMessage>().copied()),
            Some(MatrixMessage::SetAmount {
                cell,
                amount: widget
                    .amount_for_position(rect, Point::new(rect.center().x, rect.min.y + 1.0))
            })
        );
        assert!(!widget.prefers_pointer_move_paint_only());
    }

    #[test]
    fn modulation_matrix_hover_uses_paint_only_runtime_overlay() {
        let state = ModulationMatrixState::default();
        let mut widget =
            ModulationMatrixWidget::new(state.amounts, state.selected, state.activity_phase);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
        let matrix = widget.matrix_rect(bounds);
        let cell = MatrixCell {
            source: 1,
            destination: 3,
        };

        let output = widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: widget.cell_rect(matrix, cell).center(),
            },
        );

        assert!(output.is_none());
        assert_eq!(widget.hover_cell, Some(cell));
        assert!(widget.prefers_pointer_move_paint_only());
        let mut overlay = Vec::new();
        widget.append_runtime_overlay_paint(
            &mut overlay,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        assert!(
            overlay
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::StrokeRect(_))),
            "hovered route should paint as a lightweight runtime overlay"
        );
    }

    #[test]
    fn modulation_matrix_runtime_hover_does_not_refresh_surface() {
        let bridge = modulation_matrix_test_bridge(ModulationMatrixState::default());
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
        let bounds = runtime.layout().rects[&MATRIX_WIDGET_ID];
        let first = runtime.dispatch_pointer_move_with_outcome(Point::new(
            bounds.min.x + 180.0,
            bounds.center().y,
        ));
        let second = runtime.dispatch_pointer_move_with_outcome(Point::new(
            bounds.min.x + 280.0,
            bounds.center().y,
        ));

        assert!(first.needs_scene_rebuild());
        assert!(second.paint_only_requested);
        assert!(
            !second.needs_scene_rebuild(),
            "stable modulation-matrix hover should avoid reprojection and full scene rebuilds"
        );
    }

    #[test]
    fn modulation_matrix_runtime_frame_messages_advance_status() {
        let bridge = modulation_matrix_test_bridge(ModulationMatrixState::default());
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
        let initial_status = status_text(&runtime);

        assert!(runtime.bridge_mut().needs_animation());
        assert!(runtime.bridge_mut().queue_animation_frame());
        let outcome = runtime.drain_runtime_messages();

        assert_eq!(outcome.messages_dispatched, 1);
        assert_ne!(status_text(&runtime), initial_status);
    }

    fn modulation_matrix_test_bridge(
        state: ModulationMatrixState,
    ) -> impl RuntimeBridge<AppMessage> {
        radiant::app(state)
            .view(project_surface)
            .animation(|state| state.running)
            .on_frame(|| AppMessage::Frame)
            .update(update)
            .into_bridge()
    }

    fn status_text<Bridge>(runtime: &SurfaceRuntime<Bridge, AppMessage>) -> String
    where
        Bridge: RuntimeBridge<AppMessage>,
    {
        runtime
            .paint_plan(&ThemeTokens::default())
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Text(text) if text.widget_id == STATUS_WIDGET_ID => {
                    Some(text.text.as_str().to_string())
                }
                _ => None,
            })
            .expect("status text should be painted")
    }
}
