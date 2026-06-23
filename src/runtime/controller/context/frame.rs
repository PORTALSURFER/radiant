use super::super::SurfaceRuntime;
use crate::{
    gui::text_layout::{TextWidthEstimate, estimated_text_width_in_range},
    gui::types::{Point, Rect},
    layout::{LayoutOutput, Vector2},
    runtime::{
        PaintPrimitive, RuntimeBridge, SurfaceFrame, SurfacePaintPlan, empty_paint_plan_for_layout,
    },
    theme::ThemeTokens,
    widgets::WidgetId,
};

const TOOLTIP_OVERLAY_ID: WidgetId = u64::MAX - 2_048;
const TOOLTIP_MARGIN: f32 = 6.0;
const TOOLTIP_GAP: f32 = 8.0;
const TOOLTIP_MIN_WIDTH: f32 = 140.0;
const TOOLTIP_MAX_WIDTH: f32 = 360.0;
const TOOLTIP_FONT_SIZE: f32 = 9.0;
const TOOLTIP_LINE_HEIGHT: f32 = 13.0;
const TOOLTIP_BITMAP_GLYPH_HEIGHT: f32 = 7.0;
const TOOLTIP_BITMAP_GLYPH_ADVANCE: f32 = 6.0;
const TOOLTIP_CHAR_ADVANCE_SAFETY: f32 = 1.0;
const TOOLTIP_HORIZONTAL_PADDING: f32 = 16.0;
const TOOLTIP_VERTICAL_PADDING: f32 = 8.0;

/// Borrowed runtime frame for host renderers that do not need owned layout data.
///
/// Unlike [`SurfaceFrame`], this frame borrows the runtime's current layout
/// output while owning the freshly generated paint plan. It is useful for
/// embedded hosts and custom renderers that render immediately and want to
/// avoid cloning potentially large layout maps on every frame.
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSurfaceFrame<'a> {
    /// Current logical viewport rectangle.
    pub viewport: Rect,
    /// Borrowed resolved layout for the runtime's current surface.
    pub layout: &'a LayoutOutput,
    /// Backend-neutral paint plan for the borrowed layout.
    pub paint_plan: SurfacePaintPlan,
}

/// Borrowed runtime frame that reuses host-owned paint-plan storage.
///
/// This is the lowest-allocation runtime frame view for synchronous custom
/// hosts: both the resolved layout and backend-neutral paint plan are borrowed,
/// while the runtime fills the caller-provided paint plan before returning.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuntimeSurfaceFrameRef<'layout, 'paint> {
    /// Current logical viewport rectangle.
    pub viewport: Rect,
    /// Borrowed resolved layout for the runtime's current surface.
    pub layout: &'layout LayoutOutput,
    /// Borrowed backend-neutral paint plan filled for the current layout.
    pub paint_plan: &'paint SurfacePaintPlan,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Project the current surface and layout into backend-neutral paint data.
    pub fn paint_plan(&self, theme: &ThemeTokens) -> SurfacePaintPlan {
        let mut plan = empty_paint_plan_for_layout(&self.layout, theme);
        self.paint_plan_into(theme, &mut plan);
        plan
    }

    /// Project the current runtime paint data into an existing plan buffer.
    ///
    /// This avoids reallocating primitive storage for renderers that rebuild a
    /// paint plan every frame.
    pub fn paint_plan_into(&self, theme: &ThemeTokens, plan: &mut SurfacePaintPlan) {
        self.base_paint_plan_into(theme, plan);
        self.runtime_overlay_paint_into(theme, &mut plan.primitives);
    }

    /// Project the current declarative surface into an existing plan buffer.
    ///
    /// Native retained renderers use this for the cached base scene, then paint
    /// runtime-owned overlays separately so pointer-local affordances can move
    /// without leaving stale copies in the base frame.
    pub fn base_paint_plan_into(&self, theme: &ThemeTokens, plan: &mut SurfacePaintPlan) {
        self.surface.paint_plan_with_hover_into(
            &self.layout,
            theme,
            self.interaction.hover.container,
            self.interaction.hover.scroll_affordance,
            plan,
        );
    }

    /// Append runtime-local overlay primitives for active pointer widgets.
    ///
    /// These primitives are painted over the cached scene by native backends
    /// during paint-only pointer motion, so editor-style cursor and handle
    /// affordances can move without refreshing the declarative surface.
    pub fn runtime_overlay_paint_into(
        &self,
        theme: &ThemeTokens,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        self.append_widget_runtime_overlay(self.interaction.hover.widget, theme, primitives);
        if self.interaction.pointer.capture != self.interaction.hover.widget {
            self.append_widget_runtime_overlay(self.interaction.pointer.capture, theme, primitives);
        }
        self.append_widget_tooltip_overlay(theme, primitives);
        self.append_drag_preview_overlay(theme, primitives);
        self.append_devtools_overlay_paint(theme, primitives);
    }

    /// Return whether runtime-local overlay painting can emit primitives.
    ///
    /// This is intentionally conservative for widget overlays: a hovered or
    /// captured widget may decide not to paint anything, but without calling the
    /// widget we only know that it is a candidate.
    pub fn has_runtime_overlay_paint(&self) -> bool {
        self.interaction.hover.widget.is_some()
            || self.interaction.pointer.capture.is_some()
            || self
                .interaction
                .drag
                .session
                .as_ref()
                .is_some_and(|session| session.visible)
            || self.devtools_overlay.enabled
    }

    /// Package the current runtime viewport, layout, and paint plan for a host renderer.
    ///
    /// Unlike [`UiSurface::frame`](crate::runtime::UiSurface::frame), this uses
    /// the runtime's current event-driven state, including hover-aware container
    /// paint and any layout refreshed by dispatched messages or resize events.
    pub fn frame(&self, theme: &ThemeTokens) -> SurfaceFrame {
        SurfaceFrame {
            viewport: self.viewport,
            layout: self.layout.clone(),
            paint_plan: self.paint_plan(theme),
        }
    }

    /// Package the current runtime frame with Radiant's default theme.
    ///
    /// This is intended for tests, automation, examples, and embedded previews
    /// where custom theme tokens are not part of the behavior under test.
    pub fn frame_with_default_theme(&self) -> SurfaceFrame {
        self.frame(&ThemeTokens::default())
    }

    /// Package the current runtime viewport, borrowed layout, and paint plan.
    ///
    /// This is the lower-allocation counterpart to [`Self::frame`] for hosts
    /// that render synchronously and do not need to retain owned layout output
    /// after borrowing the runtime.
    pub fn borrowed_frame(&self, theme: &ThemeTokens) -> RuntimeSurfaceFrame<'_> {
        RuntimeSurfaceFrame {
            viewport: self.viewport,
            layout: &self.layout,
            paint_plan: self.paint_plan(theme),
        }
    }

    /// Fill a reusable paint plan and package borrowed frame references.
    ///
    /// This is the lower-allocation counterpart to [`Self::borrowed_frame`].
    /// Use it when a host render loop can keep a `SurfacePaintPlan` scratch
    /// buffer and render before mutating the runtime again.
    pub fn borrowed_frame_into<'layout, 'paint>(
        &'layout self,
        theme: &ThemeTokens,
        paint_plan: &'paint mut SurfacePaintPlan,
    ) -> RuntimeSurfaceFrameRef<'layout, 'paint> {
        self.paint_plan_into(theme, paint_plan);
        RuntimeSurfaceFrameRef {
            viewport: self.viewport,
            layout: &self.layout,
            paint_plan,
        }
    }

    fn append_widget_runtime_overlay(
        &self,
        widget_id: Option<WidgetId>,
        theme: &ThemeTokens,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        let Some(widget_id) = widget_id else {
            return;
        };
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return;
        };
        let Some(widget) = self.surface_widget(widget_id) else {
            return;
        };
        if widget.widget_object().common().paint.bounds == crate::widgets::PaintBounds::ClipToRect {
            primitives.push(PaintPrimitive::ClipStart(crate::runtime::PaintClipStart {
                node_id: widget_id,
                rect: bounds,
            }));
        }
        widget.widget_object().append_runtime_overlay_paint(
            primitives,
            bounds,
            &self.layout,
            theme,
        );
        if widget.widget_object().common().paint.bounds == crate::widgets::PaintBounds::ClipToRect {
            primitives.push(PaintPrimitive::ClipEnd(crate::runtime::PaintClipEnd {
                node_id: widget_id,
            }));
        }
    }

    fn append_drag_preview_overlay(
        &self,
        theme: &ThemeTokens,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        let Some(session) = self
            .interaction
            .drag
            .session
            .as_ref()
            .filter(|session| session.visible)
        else {
            return;
        };
        let rect = Rect::from_min_size(
            Point::new(
                session.pointer.x + crate::runtime::drag::DRAG_PREVIEW_OFFSET.x,
                session.pointer.y + crate::runtime::drag::DRAG_PREVIEW_OFFSET.y,
            ),
            session.preview.size,
        );
        crate::runtime::paint::push_overlay_panel(
            primitives,
            u64::MAX - 1024,
            rect,
            Some(session.preview.label.clone()),
            theme,
            crate::widgets::WidgetStyle {
                tone: crate::widgets::WidgetTone::Accent,
                prominence: crate::widgets::WidgetProminence::Strong,
            },
        );
    }

    fn append_widget_tooltip_overlay(
        &self,
        theme: &ThemeTokens,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        if self.interaction.pointer.capture.is_some() {
            return;
        }
        let Some(widget_id) = self.interaction.hover.widget else {
            return;
        };
        let Some(tooltip) = self
            .surface_widget(widget_id)
            .and_then(|widget| widget.tooltip())
            .filter(|tooltip| !tooltip.is_empty())
        else {
            return;
        };
        let Some(anchor) = self.layout.rects.get(&widget_id).copied() else {
            return;
        };
        let layout = tooltip_layout(
            anchor,
            tooltip,
            Vector2::new(self.viewport.width(), self.viewport.height()),
        );
        crate::runtime::paint::push_tooltip_panel(
            primitives,
            TOOLTIP_OVERLAY_ID,
            layout.rect,
            &layout.lines,
            theme,
            TOOLTIP_FONT_SIZE,
            TOOLTIP_LINE_HEIGHT,
        );
    }
}

struct TooltipLayout {
    rect: Rect,
    lines: Vec<String>,
}

fn tooltip_layout(anchor: Rect, tooltip: &str, viewport: Vector2) -> TooltipLayout {
    let max_width = (viewport.x - TOOLTIP_MARGIN * 2.0).clamp(1.0, TOOLTIP_MAX_WIDTH);
    let max_line_chars = tooltip_max_line_chars(max_width);
    let lines = tooltip_lines(tooltip, max_line_chars);
    let rect = tooltip_rect_for_lines(anchor, &lines, max_width, viewport);
    TooltipLayout { rect, lines }
}

fn tooltip_rect_for_lines(
    anchor: Rect,
    lines: &[String],
    max_width: f32,
    viewport: Vector2,
) -> Rect {
    let width = tooltip_width_for_lines(lines).min(max_width);
    let height = tooltip_height(lines.len());
    let x = anchor.min.x.clamp(
        TOOLTIP_MARGIN,
        (viewport.x - width - TOOLTIP_MARGIN).max(TOOLTIP_MARGIN),
    );
    let below_y = anchor.max.y + TOOLTIP_GAP;
    let y = if below_y + height <= viewport.y - TOOLTIP_MARGIN {
        below_y
    } else {
        (anchor.min.y - TOOLTIP_GAP - height).max(TOOLTIP_MARGIN)
    };
    Rect::from_min_size(Point::new(x, y), Vector2::new(width, height))
}

fn tooltip_width_for_lines(lines: &[String]) -> f32 {
    lines
        .iter()
        .map(|line| {
            estimated_text_width_in_range(
                line,
                tooltip_width_estimate(),
                TOOLTIP_MIN_WIDTH,
                TOOLTIP_MAX_WIDTH,
            )
        })
        .fold(TOOLTIP_MIN_WIDTH, f32::max)
}

fn tooltip_height(line_count: usize) -> f32 {
    line_count.max(1) as f32 * TOOLTIP_LINE_HEIGHT + TOOLTIP_VERTICAL_PADDING
}

fn tooltip_max_line_chars(max_width: f32) -> usize {
    ((max_width - TOOLTIP_HORIZONTAL_PADDING).max(1.0) / tooltip_rendered_character_advance())
        .floor()
        .max(12.0) as usize
}

fn tooltip_width_estimate() -> TextWidthEstimate {
    TextWidthEstimate::new(tooltip_character_advance(), TOOLTIP_HORIZONTAL_PADDING)
}

fn tooltip_character_advance() -> f32 {
    tooltip_rendered_character_advance().ceil() + TOOLTIP_CHAR_ADVANCE_SAFETY
}

fn tooltip_rendered_character_advance() -> f32 {
    let scale = (TOOLTIP_FONT_SIZE / TOOLTIP_BITMAP_GLYPH_HEIGHT).clamp(1.0, 3.0);
    TOOLTIP_BITMAP_GLYPH_ADVANCE * scale
}

fn tooltip_lines(tooltip: &str, max_line_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in tooltip.lines() {
        push_wrapped_tooltip_paragraph(&mut lines, paragraph.trim(), max_line_chars);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn push_wrapped_tooltip_paragraph(lines: &mut Vec<String>, paragraph: &str, max_chars: usize) {
    if paragraph.is_empty() {
        return;
    }
    let mut current = String::new();
    for word in paragraph.split_whitespace() {
        if current.is_empty() {
            push_tooltip_word(lines, &mut current, word, max_chars);
            continue;
        }
        let next_len = current.chars().count() + 1 + word.chars().count();
        if next_len <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            push_tooltip_word(lines, &mut current, word, max_chars);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
}

fn push_tooltip_word(lines: &mut Vec<String>, current: &mut String, word: &str, max_chars: usize) {
    if word.chars().count() <= max_chars {
        current.push_str(word);
        return;
    }
    let mut chunk = String::new();
    for ch in word.chars() {
        if chunk.chars().count() == max_chars {
            lines.push(std::mem::take(&mut chunk));
        }
        chunk.push(ch);
    }
    *current = chunk;
}

#[cfg(test)]
mod tests {
    use super::{
        TOOLTIP_FONT_SIZE, TOOLTIP_HORIZONTAL_PADDING, TOOLTIP_LINE_HEIGHT, TOOLTIP_MAX_WIDTH,
        TOOLTIP_OVERLAY_ID, tooltip_character_advance, tooltip_layout,
    };
    use crate::{
        gui::types::{Point, Rect},
        layout::Vector2,
        prelude::{IntoView, button, text},
        runtime::Event,
        runtime::{DeclarativeOwnedRuntimeBridge, SurfaceRuntime},
        theme::ThemeTokens,
    };

    #[test]
    fn runtime_frame_with_default_theme_projects_paint_plan() {
        let bridge = DeclarativeOwnedRuntimeBridge::new(
            (),
            |_| crate::runtime::UiSurface::new(text("Ready").into_node()),
            |_, _: ()| {},
        );
        let runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 40.0));

        assert!(
            runtime
                .frame_with_default_theme()
                .paint_plan
                .contains_text("Ready")
        );
    }

    #[derive(Default)]
    struct TooltipDemoState {
        clicked: bool,
    }

    #[derive(Clone, Debug, PartialEq)]
    enum TooltipDemoMessage {
        Click,
    }

    #[test]
    fn hovered_widget_tooltip_paints_without_intercepting_activation() {
        let bridge = DeclarativeOwnedRuntimeBridge::new(
            TooltipDemoState::default(),
            |state| {
                crate::runtime::UiSurface::new(
                    button(if state.clicked { "Clicked" } else { "Idle" })
                        .message(TooltipDemoMessage::Click)
                        .tooltip("Audition volume")
                        .id(301)
                        .size(80.0, 24.0)
                        .into_node(),
                )
            },
            |state, message| match message {
                TooltipDemoMessage::Click => state.clicked = true,
            },
        );
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));

        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(8.0, 8.0),
        });

        let frame = runtime.frame_with_default_theme();
        let tooltip = frame
            .paint_plan
            .first_text_run("Audition volume")
            .expect("hover should paint tooltip text");
        assert_eq!(tooltip.font_size, TOOLTIP_FONT_SIZE);

        let tooltip_panel = frame
            .paint_plan
            .visible_fill_rects_for_widget(TOOLTIP_OVERLAY_ID)
            .find(|fill| fill.rect.height() == TOOLTIP_LINE_HEIGHT + 8.0)
            .expect("hover should paint tooltip panel fill");
        assert_ne!(
            tooltip_panel.color,
            ThemeTokens::default().accent_copper,
            "tooltips should not reuse loud accent overlay fills"
        );

        runtime.dispatch_primary_click(Point::new(8.0, 8.0));

        assert!(
            runtime
                .frame_with_default_theme()
                .paint_plan
                .contains_text("Clicked")
        );
    }

    #[test]
    fn tooltip_if_false_skips_hover_tooltip() {
        let bridge = DeclarativeOwnedRuntimeBridge::new(
            (),
            |_| {
                crate::runtime::UiSurface::new(
                    button("Idle")
                        .message(TooltipDemoMessage::Click)
                        .tooltip_if(false, "Audition volume")
                        .id(301)
                        .size(80.0, 24.0)
                        .into_node(),
                )
            },
            |_, _: TooltipDemoMessage| {},
        );
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));

        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(8.0, 8.0),
        });

        assert!(
            !runtime
                .frame_with_default_theme()
                .paint_plan
                .contains_text("Audition volume")
        );
    }

    #[test]
    fn tooltip_rect_allows_long_compact_help_text_to_fit() {
        let layout = tooltip_layout(
            Rect::from_min_size(Point::new(240.0, 300.0), Vector2::new(40.0, 18.0)),
            "Sample row: select, double-click to load, drag to copy, right-click for actions.",
            Vector2::new(1280.0, 720.0),
        );

        assert!(layout.lines.len() > 1);
        assert!(layout.rect.height() > TOOLTIP_LINE_HEIGHT + 8.0);
        assert!(layout.rect.width() <= TOOLTIP_MAX_WIDTH);
    }

    #[test]
    fn tooltip_layout_respects_author_supplied_line_breaks() {
        let layout = tooltip_layout(
            Rect::from_min_size(Point::new(20.0, 20.0), Vector2::new(40.0, 18.0)),
            "Random section playback\nClick: play a random section now.\nCommand-click: make Space use random sections.",
            Vector2::new(360.0, 240.0),
        );

        assert_eq!(
            layout.lines.first().map(String::as_str),
            Some("Random section playback")
        );
        assert!(
            layout
                .lines
                .iter()
                .any(|line| line.contains("Command-click"))
        );
        assert!(layout.rect.height() >= 3.0 * TOOLTIP_LINE_HEIGHT + 8.0);
    }

    #[test]
    fn tooltip_layout_reserves_rendered_bitmap_width_for_short_toolbar_help() {
        let tooltip = "Loop preview playback.";
        let layout = tooltip_layout(
            Rect::from_min_size(Point::new(144.0, 72.0), Vector2::new(28.0, 24.0)),
            tooltip,
            Vector2::new(572.0, 344.0),
        );
        let text_width = layout.rect.width() - TOOLTIP_HORIZONTAL_PADDING;
        let required_width = tooltip.chars().count() as f32 * tooltip_character_advance();

        assert_eq!(layout.lines, vec![String::from(tooltip)]);
        assert!(
            text_width >= required_width,
            "tooltip should reserve enough text width: {text_width} >= {required_width}"
        );
    }
}
