//! Slotized text-line geometry for prompt/progress/drag overlays.

use super::{DragOverlayTextLayout, ProgressOverlaySections, ProgressOverlayTextLayout};
use super::{PromptOverlaySections, PromptOverlayTextLayout, shared};
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::native_shell::style::SizingTokens;
use crate::gui::types::{Point, Rect, Vector2};

const PROMPT_TEXT_ROOT_ID: u64 = 980;
const PROMPT_TEXT_TITLE_ID: u64 = 981;
const PROMPT_TEXT_MESSAGE_ID: u64 = 982;
const PROMPT_TEXT_TARGET_ID: u64 = 983;
const PROMPT_TEXT_INPUT_ID: u64 = 984;
const PROMPT_TEXT_INPUT_ERROR_ID: u64 = 985;
const PROMPT_TEXT_CONFIRM_ID: u64 = 986;
const PROMPT_TEXT_CANCEL_ID: u64 = 987;
const PROGRESS_TEXT_ROOT_ID: u64 = 990;
const PROGRESS_TEXT_TITLE_ID: u64 = 991;
const PROGRESS_TEXT_DETAIL_ID: u64 = 992;
const PROGRESS_TEXT_COUNTER_ID: u64 = 993;
const PROGRESS_TEXT_CANCEL_ID: u64 = 994;
const DRAG_TEXT_LABEL_ID: u64 = 995;

struct PromptDialogRows {
    title: Rect,
    message: Rect,
    target: Option<Rect>,
}

struct ProgressDialogRows {
    title: Rect,
    detail: Option<Rect>,
}

/// Compute prompt overlay text-line sections for dialog, input, and action buttons.
pub(super) fn compute_prompt_overlay_text_layout(
    sections: PromptOverlaySections,
    sizing: SizingTokens,
    has_target_label: bool,
    has_input_error: bool,
) -> PromptOverlayTextLayout {
    let rows = compute_prompt_dialog_rows(sections.dialog, sizing, has_target_label);
    let input_text = sections
        .input
        .map(|input| top_line_in_rect(input, sizing, sizing.font_meta, PROMPT_TEXT_INPUT_ID));
    let input_error = compute_prompt_input_error_line(
        sections,
        sizing,
        has_input_error,
        PROMPT_TEXT_INPUT_ERROR_ID,
    );
    PromptOverlayTextLayout {
        title: rows.title,
        message: rows.message,
        target: rows.target,
        input_text,
        input_error,
        confirm_label: centered_line_in_rect(
            sections.confirm_button,
            sizing,
            sizing.font_meta,
            PROMPT_TEXT_CONFIRM_ID,
        ),
        cancel_label: centered_line_in_rect(
            sections.cancel_button,
            sizing,
            sizing.font_meta,
            PROMPT_TEXT_CANCEL_ID,
        ),
    }
}

/// Compute progress overlay text-line sections for title/detail/counter/cancel copy.
pub(super) fn compute_progress_overlay_text_layout(
    sections: ProgressOverlaySections,
    sizing: SizingTokens,
    has_detail: bool,
    has_cancel_button: bool,
) -> ProgressOverlayTextLayout {
    let rows = compute_progress_dialog_rows(sections.dialog, sizing, has_detail);
    ProgressOverlayTextLayout {
        title: rows.title,
        detail: rows.detail,
        counter: compute_progress_counter_line(
            sections.dialog,
            sections.progress_bar,
            if has_cancel_button {
                Some(sections.cancel_button)
            } else {
                None
            },
            sizing,
            PROGRESS_TEXT_COUNTER_ID,
        ),
        cancel_label: if has_cancel_button {
            centered_line_in_rect(
                sections.cancel_button,
                sizing,
                sizing.font_meta,
                PROGRESS_TEXT_CANCEL_ID,
            )
        } else {
            shared::empty_rect(sections.cancel_button)
        },
    }
}

/// Compute drag overlay label text-line bounds.
pub(super) fn compute_drag_overlay_text_layout(
    banner: Rect,
    sizing: SizingTokens,
) -> DragOverlayTextLayout {
    DragOverlayTextLayout {
        label: centered_line_in_rect(banner, sizing, sizing.font_meta, DRAG_TEXT_LABEL_ID),
    }
}

fn compute_prompt_dialog_rows(
    dialog: Rect,
    sizing: SizingTokens,
    has_target: bool,
) -> PromptDialogRows {
    let empty = shared::empty_rect(dialog);
    if dialog.width() <= 0.0 || dialog.height() <= 0.0 {
        return PromptDialogRows {
            title: empty,
            message: empty,
            target: if has_target { Some(empty) } else { None },
        };
    }
    let mut children = vec![
        fixed_height_child(PROMPT_TEXT_ROOT_ID + 10, sizing.text_inset_y.max(0.0)),
        fixed_height_child(PROMPT_TEXT_TITLE_ID, sizing.font_title.max(1.0)),
        fixed_height_child(PROMPT_TEXT_ROOT_ID + 11, sizing.text_row_gap.max(0.0)),
        fixed_height_child(PROMPT_TEXT_MESSAGE_ID, sizing.font_meta.max(1.0)),
    ];
    if has_target {
        children.push(fixed_height_child(
            PROMPT_TEXT_ROOT_ID + 12,
            sizing.text_row_gap.max(0.0),
        ));
        children.push(fixed_height_child(
            PROMPT_TEXT_TARGET_ID,
            sizing.font_meta.max(1.0),
        ));
    }
    let output = layout_tree(
        &column_tree(PROMPT_TEXT_ROOT_ID, children),
        shared::inset_horizontal(dialog, sizing.text_inset_x.max(0.0)),
    );
    PromptDialogRows {
        title: shared::rect_for(&output.rects, PROMPT_TEXT_TITLE_ID, empty),
        message: shared::rect_for(&output.rects, PROMPT_TEXT_MESSAGE_ID, empty),
        target: has_target.then_some(shared::rect_for(
            &output.rects,
            PROMPT_TEXT_TARGET_ID,
            empty,
        )),
    }
}

fn compute_progress_dialog_rows(
    dialog: Rect,
    sizing: SizingTokens,
    has_detail: bool,
) -> ProgressDialogRows {
    let empty = shared::empty_rect(dialog);
    if dialog.width() <= 0.0 || dialog.height() <= 0.0 {
        return ProgressDialogRows {
            title: empty,
            detail: if has_detail { Some(empty) } else { None },
        };
    }
    let mut children = vec![
        fixed_height_child(PROGRESS_TEXT_ROOT_ID + 10, sizing.text_inset_y.max(0.0)),
        fixed_height_child(PROGRESS_TEXT_TITLE_ID, sizing.font_header.max(1.0)),
    ];
    if has_detail {
        children.push(fixed_height_child(
            PROGRESS_TEXT_ROOT_ID + 11,
            sizing.text_row_gap.max(0.0),
        ));
        children.push(fixed_height_child(
            PROGRESS_TEXT_DETAIL_ID,
            sizing.font_meta.max(1.0),
        ));
    }
    let output = layout_tree(
        &column_tree(PROGRESS_TEXT_ROOT_ID, children),
        shared::inset_horizontal(dialog, sizing.text_inset_x.max(0.0)),
    );
    ProgressDialogRows {
        title: shared::rect_for(&output.rects, PROGRESS_TEXT_TITLE_ID, empty),
        detail: has_detail.then_some(shared::rect_for(
            &output.rects,
            PROGRESS_TEXT_DETAIL_ID,
            empty,
        )),
    }
}

fn compute_prompt_input_error_line(
    sections: PromptOverlaySections,
    sizing: SizingTokens,
    has_input_error: bool,
    node_id: u64,
) -> Option<Rect> {
    if !has_input_error {
        return None;
    }
    let input = sections.input?;
    let top = input.max.y + sizing.text_row_gap.max(0.0);
    let bottom = sections
        .confirm_button
        .min
        .y
        .min(sections.dialog.max.y - sizing.text_inset_y.max(0.0));
    if bottom <= top {
        return Some(shared::empty_rect(input));
    }
    let bounds = Rect::from_min_max(
        Point::new(input.min.x, top),
        Point::new(input.max.x.max(input.min.x), bottom),
    );
    Some(top_line_in_rect(bounds, sizing, sizing.font_meta, node_id))
}

fn compute_progress_counter_line(
    dialog: Rect,
    progress_bar: Rect,
    cancel_button: Option<Rect>,
    sizing: SizingTokens,
    node_id: u64,
) -> Rect {
    let top = progress_bar.max.y + sizing.text_row_gap.max(0.0);
    let bottom = cancel_button
        .map(|button| button.min.y - sizing.text_row_gap.max(0.0))
        .unwrap_or(dialog.max.y - sizing.text_inset_y.max(0.0));
    if top >= bottom || progress_bar.width() <= 0.0 {
        return shared::empty_rect(progress_bar);
    }
    let bounds = Rect::from_min_max(
        Point::new(progress_bar.min.x, top),
        Point::new(progress_bar.max.x, bottom),
    );
    top_line_in_bounds(bounds, sizing.font_meta, node_id)
}

fn centered_line_in_rect(rect: Rect, sizing: SizingTokens, font_size: f32, node_id: u64) -> Rect {
    let empty = shared::empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    let output = layout_tree(
        &centered_line_tree(node_id, sizing, font_size),
        shared::clamp_rect_to_bounds(rect, rect),
    );
    shared::rect_for(&output.rects, node_id, empty)
}

fn top_line_in_rect(rect: Rect, sizing: SizingTokens, font_size: f32, node_id: u64) -> Rect {
    let empty = shared::empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    let output = layout_tree(
        &top_line_tree(node_id, sizing.text_inset_x.max(0.0), font_size),
        shared::clamp_rect_to_bounds(rect, rect),
    );
    shared::rect_for(&output.rects, node_id, empty)
}

fn top_line_in_bounds(bounds: Rect, font_size: f32, node_id: u64) -> Rect {
    let empty = shared::empty_rect(bounds);
    if bounds.width() <= 0.0 || bounds.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    let output = layout_tree(&top_line_tree(node_id, 0.0, font_size), bounds);
    shared::rect_for(&output.rects, node_id, empty)
}

fn column_tree(root_id: u64, children: Vec<SlotChild>) -> LayoutNode {
    LayoutNode::container(
        root_id,
        ContainerPolicy {
            kind: ContainerKind::Column,
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        children,
    )
}

fn centered_line_tree(node_id: u64, sizing: SizingTokens, font_size: f32) -> LayoutNode {
    LayoutNode::container(
        node_id + 100,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: sizing.text_inset_x.max(0.0),
                right: sizing.text_inset_x.max(0.0),
                top: sizing.text_inset_y.max(0.0),
                bottom: sizing.text_inset_y.max(0.0),
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                node_id + 101,
                ContainerPolicy {
                    kind: ContainerKind::AlignBox,
                    align_main: MainAlign::Center,
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![fixed_height_child(node_id, font_size.max(1.0))],
            ),
        }],
    )
}

fn top_line_tree(node_id: u64, horizontal_inset: f32, font_size: f32) -> LayoutNode {
    let horizontal = Insets {
        left: horizontal_inset.max(0.0),
        right: horizontal_inset.max(0.0),
        ..Insets::default()
    };
    LayoutNode::container(
        node_id + 200,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: horizontal,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: column_tree(
                node_id + 201,
                vec![
                    fixed_height_child(node_id, font_size.max(1.0)),
                    SlotChild {
                        slot: SlotParams::fill(),
                        child: LayoutNode::widget(node_id + 202, Vector2::new(1.0, 1.0)),
                    },
                ],
            ),
        }],
    )
}

fn fixed_height_child(node_id: u64, height: f32) -> SlotChild {
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(height.max(0.0)),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(0.0, f32::INFINITY, height.max(0.0), height.max(0.0)),
            margin: Insets::default(),
            align_cross_override: Some(CrossAlign::Stretch),
            allow_fixed_compress: true,
        },
        child: LayoutNode::widget(node_id, Vector2::new(1.0, height.max(1.0))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::native_shell::style::StyleTokens;

    fn assert_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    #[test]
    fn prompt_text_layout_stays_inside_sections() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let sections = PromptOverlaySections {
            dialog: Rect::from_min_max(Point::new(400.0, 180.0), Point::new(920.0, 460.0)),
            confirm_button: Rect::from_min_max(Point::new(740.0, 418.0), Point::new(824.0, 438.0)),
            cancel_button: Rect::from_min_max(Point::new(836.0, 418.0), Point::new(920.0, 438.0)),
            input: Some(Rect::from_min_max(
                Point::new(420.0, 320.0),
                Point::new(900.0, 342.0),
            )),
        };
        let layout = compute_prompt_overlay_text_layout(sections, style.sizing, true, true);
        assert_inside(sections.dialog, layout.title);
        assert_inside(sections.dialog, layout.message);
        assert_inside(sections.dialog, layout.target.expect("target row"));
        assert_inside(
            sections.input.expect("input"),
            layout.input_text.expect("input text"),
        );
        assert_inside(sections.confirm_button, layout.confirm_label);
        assert_inside(sections.cancel_button, layout.cancel_label);
        assert!(
            layout.input_error.expect("input error").min.y >= sections.input.expect("input").max.y
        );
    }

    #[test]
    fn progress_text_layout_stays_inside_sections() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let sections = ProgressOverlaySections {
            dialog: Rect::from_min_max(Point::new(560.0, 110.0), Point::new(980.0, 320.0)),
            progress_bar: Rect::from_min_max(Point::new(580.0, 190.0), Point::new(960.0, 200.0)),
            cancel_button: Rect::from_min_max(Point::new(876.0, 282.0), Point::new(960.0, 302.0)),
        };
        let layout = compute_progress_overlay_text_layout(sections, style.sizing, true, true);
        assert_inside(sections.dialog, layout.title);
        assert_inside(sections.dialog, layout.detail.expect("detail row"));
        assert_inside(sections.dialog, layout.counter);
        assert_inside(sections.cancel_button, layout.cancel_label);
        assert!(layout.counter.min.y >= sections.progress_bar.max.y);
    }

    #[test]
    fn drag_text_layout_stays_inside_banner() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let banner = Rect::from_min_max(Point::new(420.0, 620.0), Point::new(900.0, 656.0));
        let layout = compute_drag_overlay_text_layout(banner, style.sizing);
        assert_inside(banner, layout.label);
    }
}
