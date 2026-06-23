use crate::{
    application::{ViewNode, row},
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};

const DEFAULT_FORM_ROW_HEIGHT: f32 = 24.0;
const DEFAULT_FORM_ROW_LABEL_WIDTH: f32 = 112.0;
const DEFAULT_FORM_ROW_CELL_HEIGHT: f32 = 20.0;
const DEFAULT_FORM_ROW_HORIZONTAL_PADDING: f32 = 6.0;
const DEFAULT_FORM_ROW_VERTICAL_PADDING: f32 = 1.0;
const DEFAULT_FORM_ROW_SPACING: f32 = 6.0;

/// Named construction fields for a compact horizontal label/control row.
pub struct FormRowParts<Message> {
    /// Stable caller-owned row id used to derive the row key.
    pub id: String,
    /// Leading label or label-like view.
    pub label: ViewNode<Message>,
    /// Trailing control or value editor.
    pub control: ViewNode<Message>,
    /// Fixed row height.
    pub height: f32,
    /// Fixed leading label width.
    pub label_width: f32,
    /// Fixed label and control cell height.
    pub cell_height: f32,
    /// Horizontal row padding.
    pub padding_x: f32,
    /// Vertical row padding.
    pub padding_y: f32,
    /// Gap between label and control.
    pub spacing: f32,
    /// Visual styling applied to the row.
    pub style: WidgetStyle,
    /// Whether the row should receive hover styling.
    pub hoverable: bool,
}

impl<Message> FormRowParts<Message> {
    /// Build form-row parts with compact inspector/editor defaults.
    pub fn new(id: impl ToString, label: ViewNode<Message>, control: ViewNode<Message>) -> Self {
        Self {
            id: id.to_string(),
            label,
            control,
            height: DEFAULT_FORM_ROW_HEIGHT,
            label_width: DEFAULT_FORM_ROW_LABEL_WIDTH,
            cell_height: DEFAULT_FORM_ROW_CELL_HEIGHT,
            padding_x: DEFAULT_FORM_ROW_HORIZONTAL_PADDING,
            padding_y: DEFAULT_FORM_ROW_VERTICAL_PADDING,
            spacing: DEFAULT_FORM_ROW_SPACING,
            style: WidgetStyle::default(),
            hoverable: true,
        }
    }

    /// Build dense form-row parts with no outer padding or hover chrome.
    ///
    /// Use this for sidebar filters, compact inspectors, popover editors, and
    /// other dense control groups where the surrounding panel already provides
    /// hover/selection feedback and outer spacing.
    pub fn dense(id: impl ToString, label: ViewNode<Message>, control: ViewNode<Message>) -> Self {
        Self::new(id, label, control)
            .padding_x(0.0)
            .padding_y(0.0)
            .hoverable(false)
    }

    /// Override fixed row height.
    pub const fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Override fixed leading label width.
    pub const fn label_width(mut self, width: f32) -> Self {
        self.label_width = width;
        self
    }

    /// Override fixed label/control cell height.
    pub const fn cell_height(mut self, height: f32) -> Self {
        self.cell_height = height;
        self
    }

    /// Override horizontal row padding.
    pub const fn padding_x(mut self, padding: f32) -> Self {
        self.padding_x = padding;
        self
    }

    /// Override vertical row padding.
    pub const fn padding_y(mut self, padding: f32) -> Self {
        self.padding_y = padding;
        self
    }

    /// Override label/control spacing.
    pub const fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Override row style.
    pub const fn style(mut self, style: WidgetStyle) -> Self {
        self.style = style;
        self
    }

    /// Mark the row as selected with Radiant's standard subtle accent style.
    pub const fn selected(mut self, selected: bool) -> Self {
        if selected {
            self.style = WidgetStyle {
                tone: WidgetTone::Accent,
                prominence: WidgetProminence::Subtle,
            };
        }
        self
    }

    /// Override whether the row should receive hover styling.
    pub const fn hoverable(mut self, hoverable: bool) -> Self {
        self.hoverable = hoverable;
        self
    }
}

/// Build a compact horizontal label/control row.
pub fn form_row<Message: 'static>(
    id: impl ToString,
    label: ViewNode<Message>,
    control: ViewNode<Message>,
) -> ViewNode<Message> {
    form_row_from_parts(FormRowParts::new(id, label, control))
}

/// Build a compact horizontal label/control row from named parts.
pub fn form_row_from_parts<Message: 'static>(parts: FormRowParts<Message>) -> ViewNode<Message> {
    let mut view = row([
        parts
            .label
            .width(parts.label_width)
            .height(parts.cell_height),
        parts.control.fill_width().height(parts.cell_height),
    ])
    .key(format!("form-row-{}", parts.id))
    .fill_width()
    .height(parts.height)
    .padding_x(parts.padding_x)
    .padding_y(parts.padding_y)
    .spacing(parts.spacing)
    .style(parts.style);

    if parts.hoverable {
        view = view.hoverable();
    }
    view
}

#[cfg(test)]
mod tests {
    use super::{FormRowParts, form_row_from_parts};
    use crate::application::{IntoView, spacer};
    use crate::{
        layout::Vector2,
        widgets::{WidgetStyle, WidgetTone},
    };

    type TestMessage = ();
    const LABEL_ID: u64 = 0x464f_524d_0000_0001;
    const CONTROL_ID: u64 = 0x464f_524d_0000_0002;

    #[test]
    fn form_row_applies_compact_default_geometry() {
        let layout = form_row_from_parts(FormRowParts::<TestMessage>::new(
            "filter",
            spacer().id(LABEL_ID),
            spacer().id(CONTROL_ID),
        ))
        .view_layout_at_size(Vector2::new(240.0, 32.0));

        let label = layout.rects.get(&LABEL_ID).expect("label rect");
        let control = layout.rects.get(&CONTROL_ID).expect("control rect");

        assert_eq!(label.width(), 112.0);
        assert!(control.min.x >= label.max.x + 6.0);
    }

    #[test]
    fn form_row_accepts_custom_metrics_and_style() {
        let layout = form_row_from_parts(
            FormRowParts::<TestMessage>::new(
                "short",
                spacer().id(LABEL_ID),
                spacer().id(CONTROL_ID),
            )
            .label_width(48.0)
            .cell_height(18.0)
            .spacing(3.0)
            .style(WidgetStyle {
                tone: WidgetTone::Accent,
                ..WidgetStyle::default()
            })
            .hoverable(false),
        )
        .view_layout_at_size(Vector2::new(160.0, 32.0));

        let label = layout.rects.get(&LABEL_ID).expect("label rect");
        let control = layout.rects.get(&CONTROL_ID).expect("control rect");

        assert_eq!(label.width(), 48.0);
        assert!(control.min.x >= label.max.x + 3.0);
    }

    #[test]
    fn dense_form_row_removes_outer_chrome_for_embedded_control_rows() {
        let parts = FormRowParts::<TestMessage>::dense(
            "filter",
            spacer().id(LABEL_ID),
            spacer().id(CONTROL_ID),
        )
        .label_width(38.0);

        assert_eq!(parts.padding_x, 0.0);
        assert_eq!(parts.padding_y, 0.0);
        assert!(!parts.hoverable);

        let layout = form_row_from_parts(parts).view_layout_at_size(Vector2::new(160.0, 24.0));
        let label = layout.rects.get(&LABEL_ID).expect("label rect");

        assert_eq!(label.width(), 38.0);
    }
}
