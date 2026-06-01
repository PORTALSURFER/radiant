//! Floating overlay layout builders.

use crate::application::{ViewNode, ViewNodeKind, button, primary_style, stack};
use crate::gui::types::Point;
use crate::layout::Vector2;

const DRAG_PREVIEW_OFFSET_X: f32 = 14.0;
const DRAG_PREVIEW_OFFSET_Y: f32 = 18.0;
const DRAG_PREVIEW_DEFAULT_WIDTH: f32 = 168.0;
const DRAG_PREVIEW_DEFAULT_HEIGHT: f32 = 24.0;

/// Named construction fields for a centered fixed-size child layer.
pub struct CenteredLayerParts<Message> {
    /// Child view to center inside the layer.
    pub child: ViewNode<Message>,
    /// Fixed child size.
    pub size: Vector2,
}

impl<Message> CenteredLayerParts<Message> {
    /// Build centered-layer parts.
    pub fn new(child: ViewNode<Message>, size: Vector2) -> Self {
        Self { child, size }
    }
}

/// Horizontal child placement inside a full-size anchored layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerHorizontalAnchor {
    /// Place the child at the left edge after the configured inset.
    Start,
    /// Place the child centered horizontally.
    Center,
    /// Place the child at the right edge before the configured inset.
    End,
}

/// Vertical child placement inside a full-size anchored layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerVerticalAnchor {
    /// Place the child at the top edge after the configured inset.
    Start,
    /// Place the child centered vertically.
    Center,
    /// Place the child at the bottom edge before the configured inset.
    End,
}

/// Placement policy for a floating layer anchored to a trigger rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatingLayerPlacement {
    /// Place the floating layer above the trigger.
    Above,
    /// Place the floating layer below the trigger.
    Below,
}

/// Named construction fields for an anchored fixed-size child layer.
pub struct AnchoredLayerParts<Message> {
    /// Child view to place inside the layer.
    pub child: ViewNode<Message>,
    /// Fixed child size.
    pub size: Vector2,
    /// Horizontal placement policy.
    pub horizontal: LayerHorizontalAnchor,
    /// Vertical placement policy.
    pub vertical: LayerVerticalAnchor,
    /// Horizontal inset from the chosen edge.
    pub inset_x: f32,
    /// Vertical inset from the chosen edge.
    pub inset_y: f32,
}

impl<Message> AnchoredLayerParts<Message> {
    /// Build anchored-layer parts.
    pub fn new(child: ViewNode<Message>, size: Vector2) -> Self {
        Self {
            child,
            size,
            horizontal: LayerHorizontalAnchor::Center,
            vertical: LayerVerticalAnchor::Center,
            inset_x: 0.0,
            inset_y: 0.0,
        }
    }

    /// Set the horizontal anchor.
    pub fn horizontal(mut self, anchor: LayerHorizontalAnchor) -> Self {
        self.horizontal = anchor;
        self
    }

    /// Set the vertical anchor.
    pub fn vertical(mut self, anchor: LayerVerticalAnchor) -> Self {
        self.vertical = anchor;
        self
    }

    /// Set both edge insets.
    pub fn inset(mut self, x: f32, y: f32) -> Self {
        self.inset_x = x.max(0.0);
        self.inset_y = y.max(0.0);
        self
    }
}

/// Named construction fields for a floating layer anchored to a trigger.
pub struct FloatingLayerAnchorParts<Message> {
    /// Child view to place in the floating layer.
    pub child: ViewNode<Message>,
    /// Fixed floating-layer size.
    pub size: Vector2,
    /// Trigger left edge in the owning stack layer.
    pub x: f32,
    /// Trigger top edge in the owning stack layer.
    pub trigger_y: f32,
    /// Trigger height.
    pub trigger_height: f32,
    /// Gap between the trigger and floating layer.
    pub gap: f32,
    /// Whether to place the layer above or below the trigger.
    pub placement: FloatingLayerPlacement,
    /// Whether child widgets receive input traversal.
    pub interactive: bool,
}

impl<Message> FloatingLayerAnchorParts<Message> {
    /// Build floating-layer anchor parts.
    pub fn new(
        child: ViewNode<Message>,
        size: Vector2,
        x: f32,
        trigger_y: f32,
        trigger_height: f32,
        gap: f32,
        placement: FloatingLayerPlacement,
    ) -> Self {
        Self {
            child,
            size,
            x,
            trigger_y,
            trigger_height,
            gap,
            placement,
            interactive: false,
        }
    }

    /// Enable or disable input traversal through the floating content.
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }
}

/// Build a floating overlay panel in surface coordinates.
pub fn overlay_panel<Message>(
    label: impl Into<String>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> ViewNode<Message> {
    ViewNode::new(ViewNodeKind::OverlayPanel {
        rect: crate::gui::types::Rect::from_min_size(
            crate::gui::types::Point::new(x, y),
            Vector2::new(width, height),
        ),
        label: Some(label.into()),
    })
}

/// Build a full-size layer that centers a fixed-size child.
///
/// This is useful for modal panels, inspector windows, popovers, and embedded
/// surfaces where application code knows the foreground size but should not
/// manually rebuild spacer rows and columns to center it.
pub fn centered_layer<Message: 'static>(
    child: ViewNode<Message>,
    size: Vector2,
) -> ViewNode<Message> {
    centered_layer_from_parts(CenteredLayerParts::new(child, size))
}

/// Build a full-size centered layer from named parts.
pub fn centered_layer_from_parts<Message: 'static>(
    parts: CenteredLayerParts<Message>,
) -> ViewNode<Message> {
    anchored_layer_from_parts(
        AnchoredLayerParts::new(parts.child, parts.size)
            .horizontal(LayerHorizontalAnchor::Center)
            .vertical(LayerVerticalAnchor::Center),
    )
}

/// Build a full-size layer that anchors a fixed-size child.
pub fn anchored_layer<Message: 'static>(
    child: ViewNode<Message>,
    size: Vector2,
    horizontal: LayerHorizontalAnchor,
    vertical: LayerVerticalAnchor,
    inset_x: f32,
    inset_y: f32,
) -> ViewNode<Message> {
    anchored_layer_from_parts(
        AnchoredLayerParts::new(child, size)
            .horizontal(horizontal)
            .vertical(vertical)
            .inset(inset_x, inset_y),
    )
}

/// Build a full-size anchored layer from named parts.
pub fn anchored_layer_from_parts<Message: 'static>(
    parts: AnchoredLayerParts<Message>,
) -> ViewNode<Message> {
    let row = anchored_row(
        parts.child.size(parts.size.x, parts.size.y),
        parts.horizontal,
        parts.inset_x,
    );
    anchored_column(row, parts.size.y, parts.vertical, parts.inset_y)
}

/// Build a full-size transparent layer that emits a dismiss message when activated.
///
/// Use this behind popovers, menus, dropdowns, and transient panels that should
/// close when the user clicks outside the foreground content.
pub fn dismiss_layer<Message>(message: Message) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    button("")
        .message(message)
        .key("dismiss-layer")
        .input_only()
        .fill()
}

/// Stack base content with a transparent dismiss layer and foreground overlay.
///
/// Use this for transient menus, dropdowns, popovers, and inspectors where the
/// overlay should stay above an outside-click dismissal surface. The base
/// content remains visible underneath, while pointer activation outside the
/// foreground overlay emits `dismiss_message`.
pub fn dismissible_overlay<Message>(
    base: ViewNode<Message>,
    overlay: ViewNode<Message>,
    dismiss_message: Message,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    stack([base, dismiss_layer(dismiss_message), overlay]).fill()
}

/// Layer transparent input over visible content in one stacked view.
///
/// This is useful for composite controls where the application owns the visual
/// row content but wants a generic button, interactive row, drag handle, or
/// other input surface to cover the same bounds without painting its own
/// chrome.
pub fn input_overlay<Message: 'static>(
    content: ViewNode<Message>,
    input: ViewNode<Message>,
) -> ViewNode<Message> {
    stack([content, input.input_only()])
}

/// Layer visible content over an input or feedback surface in one stacked view.
///
/// This is useful for composite rows where the input surface should still paint
/// hover, selection, drag, or drop-target feedback behind custom row content.
pub fn input_underlay<Message: 'static>(
    content: ViewNode<Message>,
    input: ViewNode<Message>,
) -> ViewNode<Message> {
    stack([input, content])
}

/// Build a non-interactive floating child tree positioned relative to its parent.
///
/// The layer paints regular view content without contributing intrinsic size and
/// does not register its child widgets for pointer or wheel input.
pub fn floating_layer<Message>(
    offset: Point,
    size: Vector2,
    child: ViewNode<Message>,
) -> ViewNode<Message> {
    floating_layer_with_input(offset, size, child, false)
}

/// Build a floating child tree positioned relative to its parent.
///
/// Set `interactive` when the floating content should receive pointer, wheel,
/// focus, and state synchronization traversal like normal content.
pub fn floating_layer_with_input<Message>(
    offset: Point,
    size: Vector2,
    child: ViewNode<Message>,
    interactive: bool,
) -> ViewNode<Message> {
    let has_reserved_descendant_identity = child.has_reserved_identity_in_subtree();
    ViewNode::new(ViewNodeKind::FloatingLayer {
        offset,
        size,
        child: Box::new(child),
        interactive,
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build a floating child tree above a trigger rectangle.
///
/// This is useful for autocompletion, tooltips, and compact editor popups that
/// should stay in the same stack layer as their trigger without app-local
/// offset arithmetic.
pub fn floating_layer_above<Message>(
    x: f32,
    trigger_y: f32,
    gap: f32,
    size: Vector2,
    child: ViewNode<Message>,
) -> ViewNode<Message> {
    floating_layer_around_from_parts(FloatingLayerAnchorParts::new(
        child,
        size,
        x,
        trigger_y,
        0.0,
        gap,
        FloatingLayerPlacement::Above,
    ))
}

/// Build a floating child tree below a trigger rectangle.
pub fn floating_layer_below<Message>(
    x: f32,
    trigger_y: f32,
    trigger_height: f32,
    gap: f32,
    size: Vector2,
    child: ViewNode<Message>,
) -> ViewNode<Message> {
    floating_layer_around_from_parts(FloatingLayerAnchorParts::new(
        child,
        size,
        x,
        trigger_y,
        trigger_height,
        gap,
        FloatingLayerPlacement::Below,
    ))
}

/// Build a floating child tree around a trigger from named parts.
pub fn floating_layer_around_from_parts<Message>(
    parts: FloatingLayerAnchorParts<Message>,
) -> ViewNode<Message> {
    let size = Vector2::new(parts.size.x.max(1.0), parts.size.y.max(1.0));
    let offset = floating_layer_anchor_offset(
        parts.x,
        parts.trigger_y,
        parts.trigger_height,
        parts.gap,
        size,
        parts.placement,
    );
    floating_layer_with_input(offset, size, parts.child, parts.interactive)
}

fn floating_layer_anchor_offset(
    x: f32,
    trigger_y: f32,
    trigger_height: f32,
    gap: f32,
    size: Vector2,
    placement: FloatingLayerPlacement,
) -> Point {
    let x = x.max(0.0);
    let trigger_y = trigger_y.max(0.0);
    let gap = gap.max(0.0);
    let y = match placement {
        FloatingLayerPlacement::Above => (trigger_y - gap - size.y).max(0.0),
        FloatingLayerPlacement::Below => trigger_y + trigger_height.max(0.0) + gap,
    };
    Point::new(x, y)
}

/// Build a floating drop marker in surface coordinates.
pub fn drop_marker<Message>(x: f32, y: f32, width: f32, height: f32) -> ViewNode<Message> {
    ViewNode::new(ViewNodeKind::OverlayPanel {
        rect: crate::gui::types::Rect::from_min_size(
            crate::gui::types::Point::new(x, y),
            Vector2::new(width, height),
        ),
        label: None,
    })
    .style(primary_style())
}

/// Build a non-interactive drag preview that follows the pointer.
///
/// The preview is offset from the pointer so it reads like a carried item
/// without covering the exact drop target under the cursor.
pub fn drag_preview<Message>(label: impl Into<String>, pointer: Point) -> ViewNode<Message> {
    drag_preview_sized(
        label,
        pointer,
        Vector2::new(DRAG_PREVIEW_DEFAULT_WIDTH, DRAG_PREVIEW_DEFAULT_HEIGHT),
    )
}

/// Build a non-interactive drag preview with an explicit preview size.
pub fn drag_preview_sized<Message>(
    label: impl Into<String>,
    pointer: Point,
    size: Vector2,
) -> ViewNode<Message> {
    overlay_panel(
        label,
        pointer.x + DRAG_PREVIEW_OFFSET_X,
        pointer.y + DRAG_PREVIEW_OFFSET_Y,
        size.x.max(1.0),
        size.y.max(1.0),
    )
    .style(primary_style())
}

fn anchored_column<Message: 'static>(
    row: ViewNode<Message>,
    height: f32,
    anchor: LayerVerticalAnchor,
    inset: f32,
) -> ViewNode<Message> {
    let inset = inset.max(0.0);
    let row = row.fill_width().height(height);
    let top = crate::application::spacer().fill_height();
    let bottom = crate::application::spacer().fill_height();
    let inset_spacer = crate::application::spacer().height(inset);
    let column = match anchor {
        LayerVerticalAnchor::Start => crate::application::column([inset_spacer, row, bottom]),
        LayerVerticalAnchor::Center => crate::application::column([top, row, bottom]),
        LayerVerticalAnchor::End => crate::application::column([top, row, inset_spacer]),
    };
    column.spacing(0.0).fill()
}

fn anchored_row<Message: 'static>(
    child: ViewNode<Message>,
    anchor: LayerHorizontalAnchor,
    inset: f32,
) -> ViewNode<Message> {
    let inset = inset.max(0.0);
    let left = crate::application::spacer().fill_width();
    let right = crate::application::spacer().fill_width();
    let inset_spacer = crate::application::spacer().width(inset).height(1.0);
    let row = match anchor {
        LayerHorizontalAnchor::Start => crate::application::row([inset_spacer, child, right]),
        LayerHorizontalAnchor::Center => crate::application::row([left, child, right]),
        LayerHorizontalAnchor::End => crate::application::row([left, child, inset_spacer]),
    };
    row.spacing(0.0).fill_width()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{IntoView, app, text},
        gui::types::Rect,
        layout::Vector2,
        runtime::{Event, PaintPrimitive, SurfaceRuntime, UiSurface},
        widgets::{PointerButton, PointerModifiers, TextWidget, WidgetInput},
    };

    #[derive(Clone, Debug, PartialEq)]
    enum DemoMessage {
        Activate,
        Dismiss,
    }

    #[derive(Default)]
    struct DemoState {
        activated: bool,
        dismissed: bool,
    }

    #[test]
    fn input_overlay_routes_transparent_input_above_content() {
        let bridge = app(DemoState::default())
            .view(|state| {
                input_overlay(
                    text(if state.activated { "activated" } else { "idle" })
                        .id(90)
                        .fill_width()
                        .height(22.0),
                    button("").message(DemoMessage::Activate).fill(),
                )
                .fill()
            })
            .update(|state, message| match message {
                DemoMessage::Activate => state.activated = true,
                DemoMessage::Dismiss => state.dismissed = true,
            })
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 22.0));
        let position = Point::new(8.0, 8.0);

        runtime.dispatch_input_at(
            position,
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        );
        runtime.dispatch_input_at(
            position,
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        );

        assert_eq!(
            runtime
                .surface()
                .find_widget(90)
                .and_then(|widget| widget.widget_object().as_any().downcast_ref::<TextWidget>())
                .map(|widget| widget.text.as_str()),
            Some("activated")
        );
    }

    #[test]
    fn input_underlay_routes_input_below_visible_content() {
        let bridge = app(DemoState::default())
            .view(|state| {
                input_underlay(
                    text(if state.activated { "activated" } else { "idle" })
                        .id(91)
                        .fill_width()
                        .height(22.0),
                    button("").message(DemoMessage::Activate).fill(),
                )
                .fill()
            })
            .update(|state, message| match message {
                DemoMessage::Activate => state.activated = true,
                DemoMessage::Dismiss => state.dismissed = true,
            })
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 22.0));
        let position = Point::new(8.0, 8.0);

        runtime.dispatch_input_at(
            position,
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        );
        runtime.dispatch_input_at(
            position,
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        );

        assert_eq!(
            runtime
                .surface()
                .find_widget(91)
                .and_then(|widget| widget.widget_object().as_any().downcast_ref::<TextWidget>())
                .map(|widget| widget.text.as_str()),
            Some("activated")
        );
    }

    #[test]
    fn dismissible_overlay_routes_outside_activation_to_dismiss_layer() {
        let bridge = app(DemoState::default())
            .view(|state| {
                let status = if state.dismissed {
                    "dismissed"
                } else if state.activated {
                    "activated"
                } else {
                    "open"
                };
                dismissible_overlay(
                    text(status).id(92).fill(),
                    floating_layer_with_input(
                        Point::new(0.0, 0.0),
                        Vector2::new(60.0, 24.0),
                        button("menu").message(DemoMessage::Activate).fill(),
                        true,
                    ),
                    DemoMessage::Dismiss,
                )
            })
            .update(|state, message| match message {
                DemoMessage::Activate => state.activated = true,
                DemoMessage::Dismiss => state.dismissed = true,
            })
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 80.0));
        let outside_overlay = Point::new(90.0, 8.0);

        runtime.dispatch_event(Event::PointerPress {
            position: outside_overlay,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        });
        runtime.dispatch_event(Event::PointerRelease {
            position: outside_overlay,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        });

        assert_eq!(
            runtime
                .surface()
                .find_widget(92)
                .and_then(|widget| widget.widget_object().as_any().downcast_ref::<TextWidget>())
                .map(|widget| widget.text.as_str()),
            Some("dismissed")
        );
    }

    #[test]
    fn anchored_layer_places_child_at_configured_edges() {
        let frame = UiSurface::new(
            anchored_layer::<()>(
                text("details").id(90).size(80.0, 20.0),
                Vector2::new(80.0, 20.0),
                LayerHorizontalAnchor::End,
                LayerVerticalAnchor::End,
                12.0,
                8.0,
            )
            .into_node(),
        )
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 100.0)),
            &Default::default(),
        );

        let text_rect = frame
            .paint_plan
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Text(text) if text.widget_id == 90 => Some(text.rect),
                _ => None,
            })
            .expect("anchored layer child should paint");

        assert!((text_rect.min.x - 108.0).abs() < 0.01, "{text_rect:?}");
        assert!((text_rect.min.y - 72.0).abs() < 0.01, "{text_rect:?}");
    }

    #[test]
    fn centered_layer_uses_anchored_layer_center_policy() {
        let frame = UiSurface::new(
            centered_layer::<()>(text("center").id(91), Vector2::new(80.0, 20.0)).into_node(),
        )
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 100.0)),
            &Default::default(),
        );

        let text_rect = frame
            .paint_plan
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Text(text) if text.widget_id == 91 => Some(text.rect),
                _ => None,
            })
            .expect("centered layer child should paint");

        assert!((text_rect.min.x - 60.0).abs() < 0.01, "{text_rect:?}");
        assert!((text_rect.min.y - 40.0).abs() < 0.01, "{text_rect:?}");
    }

    #[test]
    fn floating_layer_above_positions_child_before_trigger_gap() {
        let frame = UiSurface::new(
            stack([
                text("").size(200.0, 100.0),
                floating_layer_above::<()>(
                    12.0,
                    60.0,
                    4.0,
                    Vector2::new(80.0, 20.0),
                    text("popup").id(92),
                ),
            ])
            .into_node(),
        )
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 100.0)),
            &Default::default(),
        );

        let text_rect = frame
            .paint_plan
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Text(text) if text.widget_id == 92 => Some(text.rect),
                _ => None,
            })
            .expect("floating layer child should paint");

        assert!((text_rect.min.x - 12.0).abs() < 0.01, "{text_rect:?}");
        assert!((text_rect.min.y - 36.0).abs() < 0.01, "{text_rect:?}");
    }

    #[test]
    fn floating_layer_below_positions_child_after_trigger_gap() {
        let frame = UiSurface::new(
            stack([
                text("").size(200.0, 100.0),
                floating_layer_below::<()>(
                    12.0,
                    20.0,
                    18.0,
                    4.0,
                    Vector2::new(80.0, 20.0),
                    text("popup").id(93),
                ),
            ])
            .into_node(),
        )
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 100.0)),
            &Default::default(),
        );

        let text_rect = frame
            .paint_plan
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Text(text) if text.widget_id == 93 => Some(text.rect),
                _ => None,
            })
            .expect("floating layer child should paint");

        assert!((text_rect.min.x - 12.0).abs() < 0.01, "{text_rect:?}");
        assert!((text_rect.min.y - 42.0).abs() < 0.01, "{text_rect:?}");
    }
}
