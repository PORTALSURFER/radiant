use super::parts::{
    AnchoredLayerParts, CenteredLayerParts, LayerHorizontalAnchor, LayerVerticalAnchor,
};
use crate::application::{ViewNode, column, row, spacer};
use crate::layout::Vector2;

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

fn anchored_column<Message: 'static>(
    row: ViewNode<Message>,
    height: f32,
    anchor: LayerVerticalAnchor,
    inset: f32,
) -> ViewNode<Message> {
    let inset = inset.max(0.0);
    let row = row.fill_width().height(height);
    let top = spacer().fill_height();
    let bottom = spacer().fill_height();
    let inset_spacer = spacer().height(inset);
    let column = match anchor {
        LayerVerticalAnchor::Start => column([inset_spacer, row, bottom]),
        LayerVerticalAnchor::Center => column([top, row, bottom]),
        LayerVerticalAnchor::End => column([top, row, inset_spacer]),
    };
    column.spacing(0.0).fill()
}

fn anchored_row<Message: 'static>(
    child: ViewNode<Message>,
    anchor: LayerHorizontalAnchor,
    inset: f32,
) -> ViewNode<Message> {
    let inset = inset.max(0.0);
    let left = spacer().fill_width();
    let right = spacer().fill_width();
    let inset_spacer = spacer().width(inset).height(1.0);
    let row = match anchor {
        LayerHorizontalAnchor::Start => row([inset_spacer, child, right]),
        LayerHorizontalAnchor::Center => row([left, child, right]),
        LayerHorizontalAnchor::End => row([left, child, inset_spacer]),
    };
    row.spacing(0.0).fill_width()
}
