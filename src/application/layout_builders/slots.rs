//! Optional fixed-size layout slot builders.

use crate::application::{ViewNode, spacer};

/// Build a fixed-size reserved slot from optional content.
///
/// When `content` exists, it is sized to `width` by `height`. When it does not,
/// Radiant returns a non-painting spacer with the same fixed slot dimensions so
/// adjacent controls do not shift.
pub fn fixed_slot_opt<Message: 'static>(
    content: Option<ViewNode<Message>>,
    width: f32,
    height: f32,
) -> ViewNode<Message> {
    match content {
        Some(content) => fixed_slot(content, width, height),
        None => fixed_slot(spacer(), width, height),
    }
}

/// Lazily build a fixed-size reserved slot when `condition` is true.
///
/// This avoids constructing interactive content for absent controls while still
/// preserving the same layout slot.
pub fn fixed_slot_if<Message: 'static>(
    condition: bool,
    content: impl FnOnce() -> ViewNode<Message>,
    width: f32,
    height: f32,
) -> ViewNode<Message> {
    fixed_slot_opt(condition.then(content), width, height)
}

fn fixed_slot<Message>(view: ViewNode<Message>, width: f32, height: f32) -> ViewNode<Message> {
    view.size(width, height).width(width).height(height)
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{IntoView, fixed_slot_if, fixed_slot_opt, row, text},
        layout::{LayoutNode, SizeModeCross, SizeModeMain, Vector2},
    };

    #[test]
    fn fixed_slot_opt_reserves_same_slot_when_content_is_absent() {
        let present = fixed_slot_opt(Some(text::<()>("Clear")), 20.0, 16.0);
        let absent = fixed_slot_opt::<()>(None, 20.0, 16.0);

        assert_fixed_child_slot(row([present]).into_surface().layout_node(), 20.0, 16.0);
        assert_fixed_child_slot(row([absent]).into_surface().layout_node(), 20.0, 16.0);
    }

    #[test]
    fn fixed_slot_if_lazily_skips_absent_content() {
        let mut built = false;
        let absent = fixed_slot_if(
            false,
            || {
                built = true;
                text::<()>("Clear")
            },
            20.0,
            16.0,
        );

        assert!(!built);
        assert_fixed_child_slot(row([absent]).into_surface().layout_node(), 20.0, 16.0);
    }

    #[test]
    fn fixed_slot_absent_branch_uses_non_painting_content() {
        let layout = fixed_slot_opt::<()>(None, 20.0, 16.0)
            .into_surface()
            .layout_node();

        let LayoutNode::Container(container) = layout else {
            panic!("absent fixed slot should lower to spacer container content");
        };
        assert!(container.children.is_empty());
    }

    fn assert_fixed_child_slot(layout: LayoutNode, width: f32, height: f32) {
        let LayoutNode::Container(container) = layout else {
            panic!("row should lower to a layout container");
        };
        assert_eq!(container.children.len(), 1);
        let child = &container.children[0];
        assert_eq!(child.slot.size_main, SizeModeMain::Fixed(width));
        assert_eq!(child.slot.size_cross, SizeModeCross::Fixed(height));

        let LayoutNode::Widget(widget) = &child.child else {
            return;
        };
        assert_eq!(widget.intrinsic, Vector2::new(width, height));
    }
}
