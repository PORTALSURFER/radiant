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
    crate::application::column([
        crate::application::spacer().fill_height(),
        crate::application::row([
            crate::application::spacer().fill_width(),
            parts.child.size(parts.size.x, parts.size.y),
            crate::application::spacer().fill_width(),
        ])
        .spacing(0.0)
        .fill_width()
        .height(parts.size.y),
        crate::application::spacer().fill_height(),
    ])
    .spacing(0.0)
    .fill()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{app, text},
        layout::Vector2,
        runtime::SurfaceRuntime,
        widgets::{PointerButton, PointerModifiers, TextWidget, WidgetInput},
    };

    #[derive(Clone, Debug, PartialEq)]
    enum DemoMessage {
        Activate,
    }

    #[derive(Default)]
    struct DemoState {
        activated: bool,
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
}
