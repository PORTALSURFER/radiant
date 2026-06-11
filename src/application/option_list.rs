use super::{
    BoundedScrollColumnParts, LayerHorizontalAnchor, LayerVerticalAnchor, ViewNode, anchored_layer,
    bounded_scroll_column_from_parts, empty, floating_layer_above, pointer_target, row, text,
};
use crate::gui::list::bounded_list_height;
use crate::layout::Vector2;
use crate::widgets::{
    PointerButton, PointerShieldMessage, WidgetProminence, WidgetStyle, WidgetTone,
};

const DEFAULT_COMPACT_OPTION_LIST_MAX_VISIBLE_ROWS: usize = 6;
const DEFAULT_COMPACT_OPTION_LIST_ROW_HEIGHT: f32 = 18.0;
const DEFAULT_COMPACT_OPTION_LIST_VERTICAL_CHROME: f32 = 6.0;
const DEFAULT_COMPACT_OPTION_LIST_PADDING: f32 = 3.0;
const DEFAULT_COMPACT_OPTION_LIST_GAP: f32 = 6.0;

/// One display row in a compact option list.
#[derive(Clone, Debug, PartialEq)]
pub struct CompactOptionListItem {
    /// Main option label.
    pub primary_label: String,
    /// Optional secondary label for group, category, shortcut, or metadata text.
    pub secondary_label: Option<String>,
    /// Whether this row is the active keyboard or current selection.
    pub selected: bool,
}

impl CompactOptionListItem {
    /// Build a compact option-list item.
    pub fn new(primary_label: impl Into<String>) -> Self {
        Self {
            primary_label: primary_label.into(),
            secondary_label: None,
            selected: false,
        }
    }

    /// Set the optional secondary label.
    pub fn secondary_label(mut self, secondary_label: impl Into<String>) -> Self {
        self.secondary_label = Some(secondary_label.into());
        self
    }

    /// Set whether this row is selected.
    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

/// Named construction fields for a compact fixed-row option list.
#[derive(Clone, Debug, PartialEq)]
pub struct CompactOptionListParts {
    /// Ordered option rows.
    pub items: Vec<CompactOptionListItem>,
    /// Maximum number of rows visible before scrolling.
    pub max_visible_rows: usize,
    /// Fixed row height.
    pub row_height: f32,
    /// Vertical chrome included in the capped scroll height.
    pub vertical_chrome: f32,
    /// Fixed width for the primary label column.
    pub primary_label_width: f32,
    /// Gap between primary and secondary columns.
    pub column_gap: f32,
    /// Style applied to the scroll viewport.
    pub style: WidgetStyle,
    /// Padding applied inside the scroll viewport.
    pub padding: f32,
}

impl CompactOptionListParts {
    /// Build compact option-list parts with standard autocomplete/menu metrics.
    pub fn new(items: Vec<CompactOptionListItem>, primary_label_width: f32) -> Self {
        Self {
            items,
            max_visible_rows: DEFAULT_COMPACT_OPTION_LIST_MAX_VISIBLE_ROWS,
            row_height: DEFAULT_COMPACT_OPTION_LIST_ROW_HEIGHT,
            vertical_chrome: DEFAULT_COMPACT_OPTION_LIST_VERTICAL_CHROME,
            primary_label_width,
            column_gap: DEFAULT_COMPACT_OPTION_LIST_GAP,
            style: WidgetStyle::new(WidgetTone::Neutral, WidgetProminence::Subtle),
            padding: DEFAULT_COMPACT_OPTION_LIST_PADDING,
        }
    }

    /// Set the maximum number of visible rows before scrolling.
    pub const fn max_visible_rows(mut self, max_visible_rows: usize) -> Self {
        self.max_visible_rows = max_visible_rows;
        self
    }

    /// Set fixed row height.
    pub const fn row_height(mut self, row_height: f32) -> Self {
        self.row_height = row_height;
        self
    }

    /// Set vertical chrome included in the capped scroll height.
    pub const fn vertical_chrome(mut self, vertical_chrome: f32) -> Self {
        self.vertical_chrome = vertical_chrome;
        self
    }

    /// Set the primary label column width.
    pub const fn primary_label_width(mut self, primary_label_width: f32) -> Self {
        self.primary_label_width = primary_label_width;
        self
    }

    /// Set the gap between primary and secondary columns.
    pub const fn column_gap(mut self, column_gap: f32) -> Self {
        self.column_gap = column_gap;
        self
    }

    /// Set the style applied to the scroll viewport.
    pub const fn style(mut self, style: WidgetStyle) -> Self {
        self.style = style;
        self
    }

    /// Set uniform padding inside the scroll viewport.
    pub const fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Return the fixed viewport height implied by these option-list parts.
    pub fn height(&self) -> f32 {
        bounded_list_height(
            self.items.len(),
            self.max_visible_rows,
            self.row_height,
            self.vertical_chrome,
        )
    }
}

/// Named construction fields for a compact option list floating above a trigger.
#[derive(Clone, Debug, PartialEq)]
pub struct CompactOptionListFloatingAboveParts {
    /// Option-list content and row metrics.
    pub list: CompactOptionListParts,
    /// Layer x offset inside the parent stack.
    pub x: f32,
    /// Trigger top y offset inside the parent stack.
    pub trigger_y: f32,
    /// Gap between the trigger and floating option list.
    pub gap: f32,
    /// Floating option-list width.
    pub width: f32,
}

/// Named construction fields for a compact option list in an anchored layer.
#[derive(Clone, Debug, PartialEq)]
pub struct CompactOptionListAnchoredParts {
    /// Option-list content and row metrics.
    pub list: CompactOptionListParts,
    /// Floating option-list width.
    pub width: f32,
    /// Horizontal anchor inside the parent layer.
    pub horizontal_anchor: LayerHorizontalAnchor,
    /// Vertical anchor inside the parent layer.
    pub vertical_anchor: LayerVerticalAnchor,
    /// Horizontal inset from the selected horizontal anchor.
    pub inset_x: f32,
    /// Vertical inset from the selected vertical anchor.
    pub inset_y: f32,
}

impl CompactOptionListFloatingAboveParts {
    /// Build named parts for a compact option list floating above a trigger.
    pub const fn new(
        list: CompactOptionListParts,
        x: f32,
        trigger_y: f32,
        gap: f32,
        width: f32,
    ) -> Self {
        Self {
            list,
            x,
            trigger_y,
            gap,
            width,
        }
    }
}

impl CompactOptionListAnchoredParts {
    /// Build named parts for a compact option list in an anchored layer.
    pub const fn new(
        list: CompactOptionListParts,
        width: f32,
        horizontal_anchor: LayerHorizontalAnchor,
        vertical_anchor: LayerVerticalAnchor,
        inset_x: f32,
        inset_y: f32,
    ) -> Self {
        Self {
            list,
            width,
            horizontal_anchor,
            vertical_anchor,
            inset_x,
            inset_y,
        }
    }
}

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

fn compact_option_list_from_parts_with_interaction_impl<Message: 'static>(
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

/// Build a compact option list in a floating layer above a trigger rectangle.
///
/// This is useful for autocomplete popups and compact editor pickers that should
/// stay in the same stack layer as their trigger while sharing Radiant's capped
/// option-list height and empty-list behavior.
pub fn compact_option_list_floating_above<Message: 'static>(
    parts: CompactOptionListFloatingAboveParts,
) -> ViewNode<Message> {
    let height = parts.list.height();
    if height <= 0.0 {
        return empty().fill_width();
    }
    let width = parts.width.max(1.0);
    let child = compact_option_list_from_parts(parts.list)
        .fill_width()
        .height(height);
    floating_layer_above(
        parts.x,
        parts.trigger_y,
        parts.gap,
        Vector2::new(width, height),
        child,
    )
}

/// Build a compact option list in a parent-anchored layer.
///
/// This is useful for autocomplete popups and compact editor pickers that are
/// projected in a full-surface overlay layer instead of beside their trigger in
/// the local stack.
pub fn compact_option_list_anchored<Message: 'static>(
    parts: CompactOptionListAnchoredParts,
) -> ViewNode<Message> {
    compact_option_list_anchored_with_activation(parts, |_| None::<Message>)
}

/// Build a parent-anchored compact option list and map row activation to host messages.
pub fn compact_option_list_anchored_with_activation<Message: 'static>(
    parts: CompactOptionListAnchoredParts,
    activate: impl Fn(usize) -> Option<Message> + Clone + Send + Sync + 'static,
) -> ViewNode<Message> {
    compact_option_list_anchored_with_interaction_impl(parts, activate, |_| None::<Message>, false)
}

/// Build a parent-anchored compact option list and map row hover/activation to host messages.
pub fn compact_option_list_anchored_with_interaction<Message: 'static>(
    parts: CompactOptionListAnchoredParts,
    activate: impl Fn(usize) -> Option<Message> + Clone + Send + Sync + 'static,
    hover: impl Fn(usize) -> Option<Message> + Clone + Send + Sync + 'static,
) -> ViewNode<Message> {
    compact_option_list_anchored_with_interaction_impl(parts, activate, hover, true)
}

fn compact_option_list_anchored_with_interaction_impl<Message: 'static>(
    parts: CompactOptionListAnchoredParts,
    activate: impl Fn(usize) -> Option<Message> + Clone + Send + Sync + 'static,
    hover: impl Fn(usize) -> Option<Message> + Clone + Send + Sync + 'static,
    pointer_move: bool,
) -> ViewNode<Message> {
    let height = parts.list.height();
    if height <= 0.0 {
        return empty().fill_width();
    }
    let width = parts.width.max(1.0);
    let child = compact_option_list_from_parts_with_interaction_impl(
        parts.list,
        activate,
        hover,
        pointer_move,
    )
    .fill_width()
    .height(height);
    anchored_layer(
        child,
        Vector2::new(width, height),
        parts.horizontal_anchor,
        parts.vertical_anchor,
        parts.inset_x,
        parts.inset_y,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{IntoView, column, stack},
        gui::types::{Point, Rect},
        layout::{LayoutNode, SizeModeMain, Vector2},
        runtime::{PaintPrimitive, UiSurface},
        widgets::WidgetInput,
    };

    #[test]
    fn compact_option_list_caps_height_and_keeps_empty_lists_hidden() {
        let empty_parts = CompactOptionListParts::new(Vec::new(), 80.0);
        assert_eq!(empty_parts.height(), 0.0);
        let empty_frame = compact_option_list::<()>(Vec::new(), 80.0)
            .view_frame_at_size_with_default_theme(Vector2::new(120.0, 80.0));
        assert!(empty_frame.paint_plan.text_runs().next().is_none());

        let items = (0..12)
            .map(|index| {
                CompactOptionListItem::new(format!("Item {index}"))
                    .secondary_label("Group")
                    .selected(index == 1)
            })
            .collect::<Vec<_>>();
        let view = compact_option_list::<()>(items, 80.0);
        let layout = column([view]).into_surface().layout_node();
        let LayoutNode::Container(parent_column) = layout else {
            panic!("parent should lower to a column container");
        };
        assert!(matches!(
            parent_column.children[0].slot.size_main,
            SizeModeMain::Fixed(height) if (height - 114.0).abs() < 0.01
        ));
    }

    #[test]
    fn compact_option_list_parts_exposes_capped_height() {
        let items = (0..12)
            .map(|index| CompactOptionListItem::new(format!("Item {index}")))
            .collect::<Vec<_>>();
        let parts = CompactOptionListParts::new(items, 80.0)
            .max_visible_rows(4)
            .row_height(20.0)
            .vertical_chrome(8.0);

        assert_eq!(parts.height(), 88.0);
    }

    #[test]
    fn compact_option_list_floating_above_positions_popup_before_trigger() {
        let items = vec![CompactOptionListItem::new("Kick").secondary_label("Drum")];
        let list = CompactOptionListParts::new(items, 80.0)
            .row_height(18.0)
            .vertical_chrome(6.0);
        let popup = compact_option_list_floating_above::<()>(
            CompactOptionListFloatingAboveParts::new(list, 10.0, 64.0, 4.0, 160.0),
        );

        let frame = UiSurface::new(stack([text("").size(220.0, 120.0), popup]).into_node()).frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 120.0)),
            &Default::default(),
        );

        let text_rect = frame
            .paint_plan
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Text(text) if text.text.as_str() == "Kick" => Some(text.rect),
                _ => None,
            })
            .expect("floating option list should paint item text");

        assert!((text_rect.min.x - 17.0).abs() < 0.01, "{text_rect:?}");
        assert!((text_rect.min.y - 43.0).abs() < 0.01, "{text_rect:?}");
    }

    #[test]
    fn compact_option_list_anchored_positions_popup_from_parent_edges() {
        let items = vec![CompactOptionListItem::new("Kick").secondary_label("Drum")];
        let list = CompactOptionListParts::new(items, 80.0)
            .row_height(18.0)
            .vertical_chrome(6.0);
        let popup = compact_option_list_anchored::<()>(CompactOptionListAnchoredParts::new(
            list,
            160.0,
            LayerHorizontalAnchor::Start,
            LayerVerticalAnchor::End,
            12.0,
            24.0,
        ));

        let frame = UiSurface::new(stack([text("").size(220.0, 120.0), popup]).into_node()).frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 120.0)),
            &Default::default(),
        );

        let text_rect = frame
            .paint_plan
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Text(text) if text.text.as_str() == "Kick" => Some(text.rect),
                _ => None,
            })
            .expect("anchored option list should paint item text");

        assert!((text_rect.min.x - 19.0).abs() < 0.01, "{text_rect:?}");
        assert!((text_rect.min.y - 79.0).abs() < 0.01, "{text_rect:?}");
    }

    #[test]
    fn compact_option_list_activation_maps_clicked_row_index() {
        let bridge = crate::runtime::DeclarativeOwnedRuntimeBridge::new(
            Vec::<usize>::new(),
            |_| {
                let items = vec![
                    CompactOptionListItem::new("Kick"),
                    CompactOptionListItem::new("Snare").selected(true),
                ];
                let list = CompactOptionListParts::new(items, 80.0);
                compact_option_list_from_parts_with_activation(list, Some).into_surface()
            },
            |state, message| state.push(message),
        );
        let mut runtime = crate::runtime::SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));
        let click_rect = runtime
            .frame_with_default_theme()
            .paint_plan
            .first_text_rect("Snare")
            .expect("second option should paint");

        runtime.dispatch_primary_click(click_rect.center());

        assert_eq!(runtime.bridge().state(), &[1]);
    }

    #[test]
    fn compact_option_list_interaction_maps_hovered_row_index() {
        let bridge = crate::runtime::DeclarativeOwnedRuntimeBridge::new(
            Vec::<usize>::new(),
            |_| {
                let items = vec![
                    CompactOptionListItem::new("Kick"),
                    CompactOptionListItem::new("Snare").selected(true),
                ];
                let list = CompactOptionListParts::new(items, 80.0);
                compact_option_list_from_parts_with_interaction(list, |_| None, Some).into_surface()
            },
            |state, message| state.push(message),
        );
        let mut runtime = crate::runtime::SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));
        let hover_rect = runtime
            .frame_with_default_theme()
            .paint_plan
            .first_text_rect("Snare")
            .expect("second option should paint");

        runtime.dispatch_input_at(
            hover_rect.center(),
            WidgetInput::PointerMove {
                position: hover_rect.center(),
            },
        );

        assert_eq!(runtime.bridge().state(), &[1]);
    }

    #[test]
    fn compact_option_list_interaction_maps_hover_across_full_row_width() {
        let bridge = crate::runtime::DeclarativeOwnedRuntimeBridge::new(
            Vec::<usize>::new(),
            |_| {
                let items = vec![
                    CompactOptionListItem::new("Kick").secondary_label("Drum"),
                    CompactOptionListItem::new("Snare")
                        .secondary_label("Drum")
                        .selected(true),
                ];
                let list = CompactOptionListParts::new(items, 80.0);
                compact_option_list_from_parts_with_interaction(list, |_| None, Some)
                    .width(180.0)
                    .into_surface()
            },
            |state, message| state.push(message),
        );
        let mut runtime = crate::runtime::SurfaceRuntime::new(bridge, Vector2::new(180.0, 80.0));
        let snare_rect = runtime
            .frame_with_default_theme()
            .paint_plan
            .first_text_rect("Snare")
            .expect("second option should paint");
        let right_side = Point::new(168.0, snare_rect.center().y);

        runtime.dispatch_input_at(
            right_side,
            WidgetInput::PointerMove {
                position: right_side,
            },
        );

        assert_eq!(runtime.bridge().state(), &[1]);
    }

    #[test]
    fn compact_option_list_anchored_activation_maps_clicked_row_index() {
        let bridge = crate::runtime::DeclarativeOwnedRuntimeBridge::new(
            Vec::<usize>::new(),
            |_| {
                let items = vec![
                    CompactOptionListItem::new("Kick"),
                    CompactOptionListItem::new("Snare").selected(true),
                ];
                let list = CompactOptionListParts::new(items, 80.0);
                let popup = compact_option_list_anchored_with_activation(
                    CompactOptionListAnchoredParts::new(
                        list,
                        120.0,
                        LayerHorizontalAnchor::Start,
                        LayerVerticalAnchor::End,
                        8.0,
                        8.0,
                    ),
                    Some,
                );
                stack([text("").size(160.0, 100.0), popup]).into_surface()
            },
            |state, message| state.push(message),
        );
        let mut runtime = crate::runtime::SurfaceRuntime::new(bridge, Vector2::new(160.0, 100.0));
        let click_rect = runtime
            .frame_with_default_theme()
            .paint_plan
            .first_text_rect("Snare")
            .expect("second anchored option should paint");

        runtime.dispatch_primary_click(click_rect.center());

        assert_eq!(runtime.bridge().state(), &[1]);
    }
}
