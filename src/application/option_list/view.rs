use super::model::{CompactOptionListItem, CompactOptionListParts};
use crate::application::{
    BoundedScrollColumnParts, ViewNode, bounded_scroll_column_from_parts, pointer_target, row, text,
};
use crate::widgets::{
    PointerButton, PointerShieldMessage, WidgetProminence, WidgetStyle, WidgetTone,
};

/// Build a compact selected option list from ordered items.
pub fn compact_option_list<Message: 'static>(
    items: Vec<CompactOptionListItem>,
    primary_label_width: f32,
) -> ViewNode<Message> {
    compact_option_list_from_parts(CompactOptionListParts::new(items, primary_label_width))
}

/// Build a compact selected option list from named parts.
pub fn compact_option_list_from_parts<Message: 'static>(
    parts: CompactOptionListParts,
) -> ViewNode<Message> {
    compact_option_list_from_parts_with_activation(parts, |_| None::<Message>)
}

/// Build a compact selected option list and map row activation to host messages.
pub fn compact_option_list_from_parts_with_activation<Message: 'static>(
    parts: CompactOptionListParts,
    activate: impl Fn(usize) -> Option<Message> + Clone + Send + Sync + 'static,
) -> ViewNode<Message> {
    compact_option_list_from_parts_with_interaction_impl(
        parts,
        activate,
        |_| None::<Message>,
        false,
    )
}

/// Build a compact selected option list and map row hover/activation to host messages.
pub fn compact_option_list_from_parts_with_interaction<Message: 'static>(
    parts: CompactOptionListParts,
    activate: impl Fn(usize) -> Option<Message> + Clone + Send + Sync + 'static,
    hover: impl Fn(usize) -> Option<Message> + Clone + Send + Sync + 'static,
) -> ViewNode<Message> {
    compact_option_list_from_parts_with_interaction_impl(parts, activate, hover, true)
}

pub(super) fn compact_option_list_from_parts_with_interaction_impl<Message: 'static>(
    parts: CompactOptionListParts,
    activate: impl Fn(usize) -> Option<Message> + Clone + Send + Sync + 'static,
    hover: impl Fn(usize) -> Option<Message> + Clone + Send + Sync + 'static,
    pointer_move: bool,
) -> ViewNode<Message> {
    let max_visible_rows = parts.max_visible_rows;
    let row_height = parts.row_height;
    let vertical_chrome = parts.vertical_chrome;
    let row_metrics = CompactOptionListRowMetrics {
        height: row_height,
        primary_label_width: parts.primary_label_width,
        column_gap: parts.column_gap,
    };
    let style = parts.style;
    let padding = parts.padding;
    let rows = parts
        .items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            compact_option_list_row(
                index,
                item,
                row_metrics,
                activate.clone(),
                hover.clone(),
                pointer_move,
            )
        })
        .collect::<Vec<_>>();
    bounded_scroll_column_from_parts(
        BoundedScrollColumnParts::new(rows, max_visible_rows, row_height, vertical_chrome)
            .style(style)
            .padding(padding),
    )
}

#[derive(Clone, Copy)]
struct CompactOptionListRowMetrics {
    height: f32,
    primary_label_width: f32,
    column_gap: f32,
}

fn compact_option_list_row<Message: 'static>(
    index: usize,
    item: CompactOptionListItem,
    metrics: CompactOptionListRowMetrics,
    activate: impl Fn(usize) -> Option<Message> + Send + Sync + 'static,
    hover: impl Fn(usize) -> Option<Message> + Send + Sync + 'static,
    pointer_move: bool,
) -> ViewNode<Message> {
    let primary_label = text(item.primary_label)
        .height(metrics.height)
        .width(metrics.primary_label_width.max(0.0))
        .truncate();
    row([
        primary_label,
        text(item.secondary_label.unwrap_or_default())
            .height(metrics.height)
            .fill_width()
            .truncate(),
    ])
    .key(format!("compact-option-list-row-{index}"))
    .style(if item.selected {
        WidgetStyle::new(WidgetTone::Accent, WidgetProminence::Strong)
    } else {
        WidgetStyle::default()
    })
    .height(metrics.height)
    .fill_width()
    .spacing(metrics.column_gap.max(0.0))
    .pointer_target(
        pointer_target(true)
            .pointer_move(pointer_move)
            .pointer_press(false)
            .pointer_release(true)
            .pointer_drop(false)
            .wheel(false)
            .filter_map(move |message| match message {
                PointerShieldMessage::PointerRelease {
                    button: PointerButton::Primary,
                    ..
                } => activate(index),
                PointerShieldMessage::PointerMove { .. } => hover(index),
                _ => None,
            })
            .key(format!("compact-option-list-row-hit-{index}")),
    )
}
