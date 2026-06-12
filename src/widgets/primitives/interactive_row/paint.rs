//! Dense-row visual state and paint projection helpers.

use super::{InteractiveRowVisualStateParts, InteractiveRowWidget};
use crate::{
    gui::{
        list::{
            DenseRowChromeParts, DenseRowLabelParts, DenseRowPalette, DenseRowVisualState,
            push_dense_row_chrome, push_dense_row_fill, push_dense_row_labeled_chrome,
        },
        types::Rect,
    },
    runtime::PaintPrimitive,
};

impl InteractiveRowWidget {
    /// Project this interactive row into generic dense-list visual state.
    pub fn dense_visual_state(&self, parts: InteractiveRowVisualStateParts) -> DenseRowVisualState {
        DenseRowVisualState {
            selected: parts.selected,
            hovered: self.common.state.hovered,
            pressed: self.common.state.pressed,
            active_target: parts.active_target,
            candidate: parts.candidate,
        }
    }

    /// Return whether hover/pressed interaction fills should be projected.
    ///
    /// Custom-painted row wrappers can use this before attaching hover or
    /// pressed colors to a dense-row palette so their paint follows the same
    /// hover-suppression and active-drag policy as the row input state.
    pub fn paints_interaction_fill(&self) -> bool {
        !self.props.suppress_hover
            && (!self.props.drag_active || self.props.drag_source || self.props.droppable)
    }

    /// Project this row's retained input state and host-owned visual state into
    /// dense-row chrome parts.
    ///
    /// Custom row wrappers can add markers, outlines, or other chrome to the
    /// returned parts before painting while keeping the generic hover, press,
    /// selection, active-target, and candidate state merge in Radiant.
    pub fn dense_chrome_parts(
        &self,
        parts: InteractiveRowVisualStateParts,
        palette: DenseRowPalette,
    ) -> DenseRowChromeParts {
        DenseRowChromeParts::new(self.dense_visual_state(parts), palette)
    }

    /// Push this row's highest-priority dense feedback fill into a custom paint plan.
    ///
    /// Custom row wrappers can use this when they compose an
    /// `InteractiveRowWidget` for retained hover, pressed, drag, and drop state
    /// but paint their own row content.
    pub fn push_dense_fill(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        parts: InteractiveRowVisualStateParts,
        palette: DenseRowPalette,
    ) -> bool {
        push_dense_row_fill(
            primitives,
            self.id(),
            bounds,
            self.dense_visual_state(parts),
            palette,
        )
    }

    /// Push this row's standard dense chrome into a custom paint plan.
    ///
    /// Custom row wrappers can build parts with [`Self::dense_chrome_parts`],
    /// add host-specific markers or outlines, and then use this method so
    /// Radiant supplies the row widget identity and paint-plan guard behavior.
    pub fn push_dense_chrome(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        parts: DenseRowChromeParts,
    ) -> usize {
        push_dense_row_chrome(primitives, self.id(), bounds, parts)
    }

    /// Push this row's standard dense chrome followed by a centered label.
    ///
    /// Custom row wrappers can use this when their visible content is a single
    /// label over Radiant's standard dense-row feedback, markers, and outline.
    pub fn push_dense_labeled_chrome(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        chrome: DenseRowChromeParts,
        label: DenseRowLabelParts,
    ) -> usize {
        push_dense_row_labeled_chrome(primitives, self.id(), bounds, chrome, label)
    }
}
