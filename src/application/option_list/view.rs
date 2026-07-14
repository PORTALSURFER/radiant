use std::sync::Arc;

use super::{
    model::{CompactOptionListItem, CompactOptionListParts},
    placement::{
        CompactOptionListAnchor, CompactOptionListFloatingAbove, CompactOptionListPlacement,
        place_compact_option_list,
    },
};
use crate::application::{
    BoundedScrollColumnParts, ViewNode, bounded_scroll_column_from_parts, pointer_target, row, text,
};
use crate::widgets::{
    PointerButton, PointerShieldMessage, WidgetProminence, WidgetStyle, WidgetTone,
};

type OptionListMessageMap<Message> = Arc<dyn Fn(usize) -> Option<Message> + Send + Sync + 'static>;

/// Fluent builder for compact option-list content, interaction, and placement.
pub struct CompactOptionListBuilder<Message> {
    parts: CompactOptionListParts,
    activate: Option<OptionListMessageMap<Message>>,
    hover: Option<OptionListMessageMap<Message>>,
    placement: CompactOptionListPlacement,
}

/// Start a compact option list from its reusable content and row configuration.
pub fn compact_option_list<Message>(
    parts: CompactOptionListParts,
) -> CompactOptionListBuilder<Message> {
    CompactOptionListBuilder {
        parts,
        activate: None,
        hover: None,
        placement: CompactOptionListPlacement::Inline,
    }
}

impl<Message> CompactOptionListBuilder<Message> {
    /// Emit a host message when an option row is activated.
    pub fn on_activate(
        mut self,
        activate: impl Fn(usize) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Message: 'static,
    {
        self.activate = Some(Arc::new(move |index| Some(activate(index))));
        self
    }

    /// Conditionally emit a host message when an option row is activated.
    pub fn filter_map_activate(
        mut self,
        activate: impl Fn(usize) -> Option<Message> + Send + Sync + 'static,
    ) -> Self
    where
        Message: 'static,
    {
        self.activate = Some(Arc::new(activate));
        self
    }

    /// Emit a host message when the pointer hovers an option row.
    pub fn on_hover(mut self, hover: impl Fn(usize) -> Message + Send + Sync + 'static) -> Self
    where
        Message: 'static,
    {
        self.hover = Some(Arc::new(move |index| Some(hover(index))));
        self
    }

    /// Conditionally emit a host message when the pointer hovers an option row.
    pub fn filter_map_hover(
        mut self,
        hover: impl Fn(usize) -> Option<Message> + Send + Sync + 'static,
    ) -> Self
    where
        Message: 'static,
    {
        self.hover = Some(Arc::new(hover));
        self
    }

    /// Place the option list in a fixed-size layer anchored to its parent.
    pub fn anchored(mut self, anchor: CompactOptionListAnchor) -> Self {
        self.placement = CompactOptionListPlacement::Anchored(anchor);
        self
    }

    /// Place the option list in a local floating layer above a trigger.
    pub fn floating_above(mut self, placement: CompactOptionListFloatingAbove) -> Self {
        self.placement = CompactOptionListPlacement::FloatingAbove(placement);
        self
    }

    /// Build the configured compact option-list view.
    pub fn view(self) -> ViewNode<Message>
    where
        Message: 'static,
    {
        let height = self.parts.height();
        let pointer_move = self.hover.is_some();
        let interactive = self.activate.is_some() || pointer_move;
        let child = compact_option_list_view(self.parts, self.activate, self.hover, pointer_move);
        place_compact_option_list(self.placement, child, height, interactive)
    }
}

fn compact_option_list_view<Message: 'static>(
    parts: CompactOptionListParts,
    activate: Option<OptionListMessageMap<Message>>,
    hover: Option<OptionListMessageMap<Message>>,
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
                activate.as_ref().map(Arc::clone),
                hover.as_ref().map(Arc::clone),
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
    activate: Option<OptionListMessageMap<Message>>,
    hover: Option<OptionListMessageMap<Message>>,
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
                } => activate.as_ref().and_then(|activate| activate(index)),
                PointerShieldMessage::PointerMove { .. } => {
                    hover.as_ref().and_then(|hover| hover(index))
                }
                _ => None,
            })
            .key(format!("compact-option-list-row-hit-{index}")),
    )
}
