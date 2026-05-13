use super::super::{ViewNode, ViewNodeKind};
use crate::layout::{CrossAlign, Insets, MainAlign};

impl<Message> ViewNode<Message> {
    /// Apply equal content padding when this node is a container.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Some(Insets::all(padding.max(0.0)));
        self
    }

    /// Apply horizontal content padding when this node is a container.
    pub fn padding_x(mut self, padding: f32) -> Self {
        let padding = padding.max(0.0);
        let mut insets = self.padding.unwrap_or_default();
        insets.left = padding;
        insets.right = padding;
        self.padding = Some(insets);
        self
    }

    /// Apply vertical content padding when this node is a container.
    pub fn padding_y(mut self, padding: f32) -> Self {
        let padding = padding.max(0.0);
        let mut insets = self.padding.unwrap_or_default();
        insets.top = padding;
        insets.bottom = padding;
        self.padding = Some(insets);
        self
    }

    /// Align this container's children along the main axis.
    pub fn align_main(mut self, align: MainAlign) -> Self {
        self.align_main = Some(align);
        self
    }

    /// Align this container's children along the cross axis.
    pub fn align_cross(mut self, align: CrossAlign) -> Self {
        self.align_cross = Some(align);
        self
    }

    /// Set row or column spacing when this node is a container.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.set_spacing(spacing);
        self
    }

    fn set_spacing(&mut self, spacing: f32) {
        match &mut self.kind {
            ViewNodeKind::Row {
                spacing: current, ..
            }
            | ViewNodeKind::Column {
                spacing: current, ..
            } => *current = spacing.max(0.0),
            ViewNodeKind::Scroll { child } | ViewNodeKind::VirtualScroll { child, .. } => {
                child.set_spacing(spacing)
            }
            _ => {}
        }
    }
}
