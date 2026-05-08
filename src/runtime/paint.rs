//! Backend-neutral paint plans emitted from generic Radiant surfaces.

use crate::{
    gui::types::{ImageRgba, Point, Rect, Rgba8},
    layout::{LayoutOutput, NodeId, OverflowPolicy},
    theme::ThemeTokens,
    widgets::{
        PaintBounds, RetainedSurfaceDescriptor, TextInputState, TextWrap, WidgetId, WidgetState,
        WidgetStyle, resolve_widget_visual_tokens,
    },
};
use std::sync::Arc;

/// Horizontal alignment for generic text paint primitives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaintTextAlign {
    /// Align text to the left edge of the assigned text rectangle.
    Left,
    /// Align text to the center of the assigned text rectangle.
    Center,
    /// Align text to the right edge of the assigned text rectangle.
    Right,
}

/// Filled rectangle primitive in logical surface coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintFillRect {
    /// Widget that produced this primitive.
    pub widget_id: WidgetId,
    /// Rectangle to fill.
    pub rect: Rect,
    /// Fill color.
    pub color: Rgba8,
}

/// Stroked rectangle primitive in logical surface coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintStrokeRect {
    /// Widget that produced this primitive.
    pub widget_id: WidgetId,
    /// Rectangle to stroke.
    pub rect: Rect,
    /// Stroke color.
    pub color: Rgba8,
    /// Stroke width in logical pixels.
    pub width: f32,
}

/// Filled polygon primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintFillPolygon {
    /// Widget or node that produced this primitive.
    pub widget_id: WidgetId,
    /// Polygon points in clockwise or counter-clockwise order.
    pub points: Vec<Point>,
    /// Fill color.
    pub color: Rgba8,
}

/// Stroked polygon primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintStrokePolygon {
    /// Widget or node that produced this primitive.
    pub widget_id: WidgetId,
    /// Polygon points in clockwise or counter-clockwise order.
    pub points: Vec<Point>,
    /// Stroke color.
    pub color: Rgba8,
    /// Stroke width in logical pixels.
    pub width: f32,
}

/// Stroked open polyline primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintStrokePolyline {
    /// Widget or node that produced this primitive.
    pub widget_id: WidgetId,
    /// Connected line points in paint order.
    pub points: Vec<Point>,
    /// Stroke color.
    pub color: Rgba8,
    /// Stroke width in logical pixels.
    pub width: f32,
}

/// Single-line text primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintTextRun {
    /// Widget that produced this text run.
    pub widget_id: WidgetId,
    /// Text content to paint.
    pub text: String,
    /// Text layout rectangle.
    pub rect: Rect,
    /// Font size in logical pixels per em.
    pub font_size: f32,
    /// Optional baseline measured from the text rectangle top edge.
    pub baseline: Option<f32>,
    /// Text color.
    pub color: Rgba8,
    /// Horizontal alignment inside the text rectangle.
    pub align: PaintTextAlign,
    /// Wrapping policy requested by the owning widget.
    pub wrap: TextWrap,
}

/// Floating overlay panel used for drag previews and transient popups.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintOverlayPanel {
    /// Stable overlay identifier.
    pub widget_id: WidgetId,
    /// Overlay rectangle in logical surface coordinates.
    pub rect: Rect,
    /// Optional text label to paint inside the panel.
    pub label: Option<String>,
    /// Panel style.
    pub style: WidgetStyle,
}

/// Single-line text-input primitive with native caret and selection state.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintTextInput {
    /// Widget that produced this text field.
    pub widget_id: WidgetId,
    /// Text layout rectangle inside the control chrome.
    pub rect: Rect,
    /// Optional placeholder shown when the value is empty.
    pub placeholder: Option<String>,
    /// Current text input state.
    pub state: TextInputState,
    /// Font size in logical pixels per em.
    pub font_size: f32,
    /// Optional baseline measured from the text rectangle top edge.
    pub baseline: Option<f32>,
    /// Value text color.
    pub color: Rgba8,
    /// Placeholder text color.
    pub placeholder_color: Rgba8,
    /// Selection fill color.
    pub selection_color: Rgba8,
    /// Block caret fill color.
    pub caret_color: Rgba8,
    /// Whether the field currently owns keyboard focus.
    pub focused: bool,
}

/// Placeholder primitive for widgets whose paint is intentionally host-defined.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintCustomSurface {
    /// Widget that owns this custom surface.
    pub widget_id: WidgetId,
    /// Assigned widget rectangle.
    pub rect: Rect,
    /// Whether the custom paint is clipped to the assigned rectangle.
    pub bounds: PaintBounds,
    /// Optional retained-surface metadata supplied by the host.
    pub retained: Option<RetainedSurfaceDescriptor>,
}

/// Textured RGBA image primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintImage {
    /// Widget that produced this image primitive.
    pub widget_id: WidgetId,
    /// Optional source rectangle in image-pixel coordinates.
    ///
    /// When omitted, the full image is stretched into `rect`.
    pub source_rect: Option<Rect>,
    /// Destination rectangle.
    pub rect: Rect,
    /// Shared RGBA image payload.
    pub image: Arc<ImageRgba>,
}

/// Retained GPU surface drawn by native GPU backends.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintGpuSurface {
    /// Widget that produced this GPU surface.
    pub widget_id: WidgetId,
    /// Stable surface key used to retain GPU resources across frames.
    pub key: u64,
    /// Monotonic content revision for retained GPU resources.
    pub revision: u64,
    /// Destination rectangle in logical surface coordinates.
    pub rect: Rect,
    /// Backend-neutral retained content payload.
    pub content: GpuSurfaceContent,
    /// Optional lightweight overlays composited by the native GPU backend.
    pub overlays: Vec<GpuSurfaceOverlay>,
}

/// Backend-neutral retained GPU surface content.
#[derive(Clone, Debug, PartialEq)]
pub enum GpuSurfaceContent {
    /// Shared RGBA atlas payload sampled from a source rectangle.
    RgbaAtlas {
        /// Source rectangle in atlas-pixel coordinates.
        source_rect: Rect,
        /// Shared RGBA atlas payload uploaded once per key/revision by native backends.
        atlas: Arc<ImageRgba>,
    },
    /// Interleaved floating-point signal bands rendered directly at surface resolution.
    SignalBands {
        /// Total frame count in the retained signal data.
        frames: usize,
        /// Number of interleaved bands per frame.
        band_count: usize,
        /// Visible frame range as start/end frame offsets.
        frame_range: [f32; 2],
        /// Interleaved frame-major band samples.
        samples: Arc<[f32]>,
    },
    /// Interleaved floating-point signal summaries rendered directly at surface resolution.
    SignalSummaryBands {
        /// Total frame count in the retained signal data.
        frames: usize,
        /// Number of interleaved bands per frame.
        band_count: usize,
        /// Visible frame range as start/end frame offsets.
        frame_range: [f32; 2],
        /// Precomputed min/max summary pyramid.
        summary: Arc<GpuSignalSummary>,
    },
}

/// CPU-built min/max pyramid for retained GPU signal surfaces.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuSignalSummary {
    /// Total source frame count represented by the pyramid.
    pub frames: usize,
    /// Number of interleaved bands per frame.
    pub band_count: usize,
    /// Summary levels in ascending bucket size order.
    pub levels: Vec<GpuSignalSummaryLevel>,
}

/// One min/max pyramid level.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuSignalSummaryLevel {
    /// Number of source frames represented by one bucket.
    pub bucket_frames: usize,
    /// Interleaved bucket-major band summaries.
    pub buckets: Arc<[GpuSignalSummaryBucket]>,
}

/// Min/max summary for one band in one bucket.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuSignalSummaryBucket {
    /// Minimum sample value inside the bucket.
    pub min: f32,
    /// Maximum sample value inside the bucket.
    pub max: f32,
}

impl GpuSignalSummary {
    /// Build a retained min/max pyramid from interleaved frame-major band samples.
    pub fn from_interleaved_samples(samples: &[f32], frames: usize, band_count: usize) -> Self {
        let frames = frames.min(samples.len() / band_count.max(1));
        let band_count = band_count.max(1);
        let mut levels = Vec::new();
        let mut bucket_frames = 1usize;
        while bucket_frames <= frames.max(1) {
            levels.push(GpuSignalSummaryLevel {
                bucket_frames,
                buckets: build_signal_summary_level(samples, frames, band_count, bucket_frames),
            });
            if bucket_frames >= frames.max(1) {
                break;
            }
            bucket_frames = bucket_frames.saturating_mul(2).max(bucket_frames + 1);
        }
        Self {
            frames,
            band_count,
            levels,
        }
    }

    /// Return the preferred level for the provided frames-per-pixel ratio.
    pub fn level_for_frames_per_pixel(&self, frames_per_pixel: f32) -> usize {
        let target = frames_per_pixel.max(1.0);
        self.levels
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                let left_delta = (left.bucket_frames as f32 - target).abs();
                let right_delta = (right.bucket_frames as f32 - target).abs();
                left_delta
                    .partial_cmp(&right_delta)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(index, _)| index)
            .unwrap_or_default()
    }
}

fn build_signal_summary_level(
    samples: &[f32],
    frames: usize,
    band_count: usize,
    bucket_frames: usize,
) -> Arc<[GpuSignalSummaryBucket]> {
    let bucket_count = frames.div_ceil(bucket_frames.max(1)).max(1);
    let mut buckets = Vec::with_capacity(bucket_count.saturating_mul(band_count));
    for bucket in 0..bucket_count {
        let start = bucket.saturating_mul(bucket_frames).min(frames);
        let end = ((bucket + 1).saturating_mul(bucket_frames))
            .min(frames)
            .max(start + 1);
        for band in 0..band_count {
            let mut summary = GpuSignalSummaryBucket {
                min: f32::INFINITY,
                max: f32::NEG_INFINITY,
            };
            for frame in start..end {
                let value = samples
                    .get(frame.saturating_mul(band_count).saturating_add(band))
                    .copied()
                    .unwrap_or_default();
                summary.min = summary.min.min(value);
                summary.max = summary.max.max(value);
            }
            if !summary.min.is_finite() || !summary.max.is_finite() {
                summary = GpuSignalSummaryBucket::default();
            }
            buckets.push(summary);
        }
    }
    buckets.into()
}

/// Lightweight GPU-surface overlay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GpuSurfaceOverlay {
    /// Vertical cursor line positioned as a 0..1 ratio inside the destination rect.
    VerticalCursor {
        /// Horizontal cursor position as a 0..1 ratio inside the destination rect.
        ratio: f32,
        /// Cursor color.
        color: Rgba8,
        /// Cursor width in logical pixels.
        width: f32,
    },
    /// Vertical cursor line whose position is owned by the native runtime hover path.
    NativeVerticalHoverCursor {
        /// Cursor color.
        color: Rgba8,
        /// Cursor width in logical pixels.
        width: f32,
    },
}

/// Begin clipping subsequent paint primitives to a rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintClipStart {
    /// Container node that owns this clip.
    pub node_id: NodeId,
    /// Clip rectangle in logical surface coordinates.
    pub rect: Rect,
}

/// End the most recent paint clip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintClipEnd {
    /// Container node that owns the matching clip.
    pub node_id: NodeId,
}

/// One backend-neutral primitive emitted by a generic surface projection.
#[derive(Clone, Debug, PartialEq)]
pub enum PaintPrimitive {
    /// Begin a rectangular clip.
    ClipStart(PaintClipStart),
    /// End the current clip.
    ClipEnd(PaintClipEnd),
    /// Fill a rectangle.
    FillRect(PaintFillRect),
    /// Stroke a rectangle.
    StrokeRect(PaintStrokeRect),
    /// Fill a polygon.
    FillPolygon(PaintFillPolygon),
    /// Stroke a polygon.
    StrokePolygon(PaintStrokePolygon),
    /// Stroke an open polyline.
    StrokePolyline(PaintStrokePolyline),
    /// Paint one text run.
    Text(PaintTextRun),
    /// Paint a floating overlay panel above normal layout content.
    OverlayPanel(PaintOverlayPanel),
    /// Paint a single-line text input value, selection, and caret.
    TextInput(PaintTextInput),
    /// Paint an RGBA image stretched into one destination rectangle.
    Image(PaintImage),
    /// Paint a retained generic GPU surface using native GPU resources when available.
    GpuSurface(PaintGpuSurface),
    /// Reserve a host-painted custom surface.
    CustomSurface(PaintCustomSurface),
}

pub(super) fn push_clip_start(primitives: &mut Vec<PaintPrimitive>, node_id: NodeId, rect: Rect) {
    primitives.push(PaintPrimitive::ClipStart(PaintClipStart { node_id, rect }));
}

pub(super) fn push_clip_end(primitives: &mut Vec<PaintPrimitive>, node_id: NodeId) {
    primitives.push(PaintPrimitive::ClipEnd(PaintClipEnd { node_id }));
}

pub(super) fn push_scroll_affordance(
    primitives: &mut Vec<PaintPrimitive>,
    node_id: NodeId,
    content_id: NodeId,
    layout: &LayoutOutput,
    theme: &ThemeTokens,
) {
    let Some(viewport) = layout.rects.get(&node_id).copied() else {
        return;
    };
    let Some(content) = layout.rects.get(&content_id).copied() else {
        return;
    };
    let Some(overflow) = layout.overflow_flags.get(&node_id) else {
        return;
    };
    if overflow.policy != OverflowPolicy::Scroll || !overflow.y {
        return;
    }

    let viewport_h = viewport.height().max(0.0);
    let content_h = content.height().max(viewport_h);
    if viewport_h <= 0.0 || content_h <= viewport_h {
        return;
    }

    let track_w = 3.0;
    let y_inset = 6.0;
    let track_x = viewport.max.x - track_w;
    let track = Rect::from_min_max(
        Point::new(track_x, viewport.min.y + y_inset),
        Point::new(track_x + track_w, viewport.max.y - y_inset),
    );
    let max_scroll = (content_h - viewport_h).max(1.0);
    let scroll_y = (viewport.min.y - content.min.y).clamp(0.0, max_scroll);
    let thumb_h = ((viewport_h / content_h) * track.height()).clamp(24.0, track.height());
    let thumb_y = track.min.y + ((track.height() - thumb_h) * (scroll_y / max_scroll));
    let thumb = Rect::from_min_size(
        Point::new(track.min.x, thumb_y),
        crate::gui::types::Vector2::new(track.width(), thumb_h),
    );

    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: node_id,
        rect: thumb,
        color: theme.grid_strong,
    }));
}

pub(super) fn scroll_content_clip_rect(
    node_id: NodeId,
    layout: &LayoutOutput,
    viewport: Rect,
) -> Rect {
    let Some(overflow) = layout.overflow_flags.get(&node_id) else {
        return viewport;
    };
    if overflow.policy != OverflowPolicy::Scroll || !overflow.y {
        return viewport;
    }
    viewport
}

pub(super) fn push_container_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    node_id: NodeId,
    layout: &LayoutOutput,
    theme: &ThemeTokens,
    style: WidgetStyle,
    state: WidgetState,
) {
    let Some(bounds) = layout.rects.get(&node_id).copied() else {
        return;
    };
    let base_tokens = resolve_widget_visual_tokens(theme, style, WidgetState::default());
    let tokens = if state.hovered {
        base_tokens
    } else {
        resolve_widget_visual_tokens(theme, style, state)
    };
    let fill = if state.hovered {
        blend_color(
            base_tokens.fill,
            theme.surface_overlay,
            theme.state_hover_strong,
        )
    } else {
        tokens.fill
    };
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: node_id,
        rect: bounds,
        color: fill,
    }));
    if state.hovered {
        let marker_height = (bounds.height() - 16.0).max(8.0).min(bounds.height());
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: node_id,
            rect: Rect::from_min_size(
                Point::new(
                    bounds.min.x + 1.0,
                    bounds.min.y + (bounds.height() - marker_height) * 0.5,
                ),
                crate::gui::types::Vector2::new(3.0, marker_height),
            ),
            color: theme.accent_danger,
        }));
    }
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id: node_id,
        rect: bounds,
        color: tokens.border,
        width: 1.0,
    }));
}

pub(super) fn push_overlay_panel(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    rect: Rect,
    label: Option<String>,
    theme: &ThemeTokens,
    style: WidgetStyle,
) {
    let mut state = WidgetState {
        active: true,
        ..WidgetState::default()
    };
    if label.is_none() {
        state.selected = true;
    }
    let tokens = resolve_widget_visual_tokens(theme, style, state);
    if label.is_some() {
        let shadow = Rect::from_min_max(
            Point::new(rect.min.x + 4.0, rect.min.y + 6.0),
            Point::new(rect.max.x + 4.0, rect.max.y + 6.0),
        );
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id,
            rect: shadow,
            color: Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 96,
            },
        }));
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id,
            rect,
            color: blend_color(tokens.fill, theme.surface_overlay, 0.30),
        }));
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id,
            rect,
            color: tokens.emphasis,
            width: 1.0,
        }));
        if let Some(label) = label {
            push_text_run(
                primitives,
                widget_id,
                label,
                inset_rect(rect, 48.0, 4.0),
                optical_centered_baseline(inset_rect(rect, 48.0, 4.0), text_font_size(rect)),
                theme.text_primary,
                PaintTextAlign::Left,
                TextWrap::None,
                text_font_size(rect),
            );
        }
    } else {
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id,
            rect,
            color: tokens.emphasis,
        }));
    }
}

/// Deterministic backend-neutral paint output for a generic [`crate::runtime::UiSurface`].
#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePaintPlan {
    /// Clear color a backend may use before replaying primitives.
    pub clear_color: Rgba8,
    /// Primitives in declarative surface tree order.
    pub primitives: Vec<PaintPrimitive>,
}

/// Backend-neutral renderer contract for generic Radiant paint plans.
///
/// A renderer consumes the deterministic [`SurfacePaintPlan`] emitted by a
/// [`crate::runtime::View`] or [`crate::runtime::SurfaceRuntime`]. Renderer
/// implementations own backend resources and frame submission policy; Radiant
/// only defines the replayable paint-plan boundary.
pub trait Renderer {
    /// Backend-specific error type.
    type Error;

    /// Render one backend-neutral paint plan.
    fn render(&mut self, plan: &SurfacePaintPlan) -> Result<(), Self::Error>;
}

impl SurfacePaintPlan {
    /// Build an empty paint plan for the provided theme.
    pub fn empty(theme: &ThemeTokens) -> Self {
        Self {
            clear_color: theme.clear_color,
            primitives: Vec::new(),
        }
    }
}

pub(crate) fn blend_color(from: Rgba8, to: Rgba8, amount: f32) -> Rgba8 {
    let amount = amount.clamp(0.0, 1.0);
    Rgba8 {
        r: blend_channel(from.r, to.r, amount),
        g: blend_channel(from.g, to.g, amount),
        b: blend_channel(from.b, to.b, amount),
        a: blend_channel(from.a, to.a, amount),
    }
}

fn blend_channel(from: u8, to: u8, amount: f32) -> u8 {
    ((from as f32) + (((to as f32) - (from as f32)) * amount)).round() as u8
}

pub(crate) fn diagonal_cut_rect_points(rect: Rect) -> Vec<Point> {
    let cut = (rect.height().min(rect.width()) * 0.18).clamp(4.0, 8.0);
    vec![
        Point::new(rect.min.x, rect.min.y),
        Point::new(rect.max.x, rect.min.y),
        Point::new(rect.max.x, rect.max.y - cut),
        Point::new(rect.max.x - cut, rect.max.y),
        Point::new(rect.min.x, rect.max.y),
    ]
}

pub(crate) fn push_text_run(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    text: String,
    rect: Rect,
    baseline: Option<f32>,
    color: Rgba8,
    align: PaintTextAlign,
    wrap: TextWrap,
    font_size: f32,
) {
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text,
        rect,
        font_size,
        baseline,
        color,
        align,
        wrap,
    }));
}

pub(crate) fn text_font_size(rect: Rect) -> f32 {
    if rect.height() >= 38.0 {
        18.0
    } else if rect.height() >= 28.0 {
        14.0
    } else {
        13.0
    }
}

pub(crate) fn button_font_size(rect: Rect) -> f32 {
    if rect.height() >= 48.0 {
        16.0
    } else if rect.height() >= 36.0 {
        14.0
    } else {
        13.0
    }
}

pub(crate) fn input_font_size(rect: Rect) -> f32 {
    if rect.height() >= 42.0 { 15.0 } else { 13.0 }
}

pub(crate) fn optical_centered_baseline(rect: Rect, font_size: f32) -> Option<f32> {
    Some((rect.height() * 0.5 + font_size * 0.35).max(0.0))
}

pub(crate) fn push_axis_stroke(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    color: Rgba8,
    horizontal: bool,
) {
    let rect = if horizontal {
        Rect::from_min_max(
            Point::new(bounds.min.x, bounds.min.y + bounds.height() * 0.5),
            Point::new(bounds.max.x, bounds.min.y + bounds.height() * 0.5),
        )
    } else {
        Rect::from_min_max(
            Point::new(bounds.min.x + bounds.width() * 0.5, bounds.min.y),
            Point::new(bounds.min.x + bounds.width() * 0.5, bounds.max.y),
        )
    };
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect,
        color,
        width: 1.0,
    }));
}

pub(crate) fn inset_rect(rect: Rect, x: f32, y: f32) -> Rect {
    Rect::from_min_max(
        Point::new(rect.min.x + x, rect.min.y + y),
        Point::new(rect.max.x - x, rect.max.y - y),
    )
}
