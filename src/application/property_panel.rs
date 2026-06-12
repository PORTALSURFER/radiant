use crate::{
    application::{
        StateCallback, StateStringCallback, ViewNode, button, column, compatibility::StateView,
        row, text,
    },
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};
use std::sync::Arc;

/// Named construction inputs for a generic inspector/property row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyRowParts {
    /// Stable caller-owned row id.
    pub id: String,
    /// Property label shown in the leading column.
    pub label: String,
    /// Property value shown in the trailing column.
    pub value: String,
}

/// One row in a generic inspector/property panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyRow {
    /// Stable caller-owned row id.
    pub id: String,
    /// Property label shown in the leading column.
    pub label: String,
    /// Property value shown in the trailing column.
    pub value: String,
    /// Whether this row is currently selected.
    pub selected: bool,
}

impl PropertyRow {
    /// Build one property row from named construction inputs.
    pub fn from_parts(parts: PropertyRowParts) -> Self {
        Self {
            id: parts.id,
            label: parts.label,
            value: parts.value,
            selected: false,
        }
    }

    /// Build one property row.
    pub fn new(id: impl ToString, label: impl Into<String>, value: impl Into<String>) -> Self {
        Self::from_parts(PropertyRowParts {
            id: id.to_string(),
            label: label.into(),
            value: value.into(),
        })
    }

    /// Mark the row as selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

/// Build a read-only inspector/property panel.
pub fn property_panel<Message: 'static>(
    title: impl Into<String>,
    rows: impl IntoIterator<Item = PropertyRow>,
) -> ViewNode<Message> {
    column([
        text(title.into()).height(20.0).fill_width(),
        property_rows(rows),
    ])
    .style(WidgetStyle::default())
    .fill_width()
    .padding(6.0)
    .spacing(4.0)
}

/// Build read-only inspector/property rows without adding a titled panel shell.
pub fn property_rows<Message: 'static>(
    rows: impl IntoIterator<Item = PropertyRow>,
) -> ViewNode<Message> {
    column(rows.into_iter().map(read_only_property_row))
        .fill_width()
        .spacing(1.0)
}

/// Build an inspector/property panel with selectable rows.
pub fn selectable_property_panel<State: 'static>(
    title: impl Into<String>,
    rows: impl IntoIterator<Item = PropertyRow>,
    on_select: Option<impl Fn(&mut State, String) + Send + Sync + 'static>,
) -> StateView<State> {
    let on_select: Option<StateStringCallback<State>> =
        on_select.map(|on_select| Arc::new(on_select) as StateStringCallback<State>);
    column([
        text(title.into()).height(20.0).fill_width(),
        column(
            rows.into_iter()
                .map(|row| property_row(row, on_select.as_ref().map(Arc::clone))),
        )
        .fill_width()
        .spacing(1.0),
    ])
    .style(WidgetStyle::default())
    .fill_width()
    .padding(6.0)
    .spacing(4.0)
}

/// Build an inspector/property panel whose selectable rows emit host messages.
pub fn message_selectable_property_panel<Message: 'static>(
    title: impl Into<String>,
    rows: impl IntoIterator<Item = PropertyRow>,
    select_message: Option<impl Fn(String) -> Message + Send + Sync + 'static>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let select_message = select_message.map(|select_message| {
        Arc::new(select_message) as Arc<dyn Fn(String) -> Message + Send + Sync>
    });
    column([
        text(title.into()).height(20.0).fill_width(),
        column(
            rows.into_iter()
                .map(|row| message_property_row(row, select_message.as_ref().map(Arc::clone))),
        )
        .fill_width()
        .spacing(1.0),
    ])
    .style(WidgetStyle::default())
    .fill_width()
    .padding(6.0)
    .spacing(4.0)
}

fn read_only_property_row<Message: 'static>(row_data: PropertyRow) -> ViewNode<Message> {
    let selected = row_data.selected;
    let mut view = row([
        text(row_data.label)
            .key(format!("property-{}-label", row_data.id))
            .size(112.0, 20.0),
        text(row_data.value)
            .key(format!("property-{}-value", row_data.id))
            .fill_width()
            .height(20.0),
    ])
    .key(format!("property-row-{}", row_data.id))
    .fill_width()
    .height(24.0)
    .padding_x(6.0)
    .padding_y(1.0)
    .spacing(6.0)
    .style(if selected {
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: WidgetProminence::Subtle,
        }
    } else {
        WidgetStyle::default()
    })
    .hoverable();
    if selected {
        view = view.primary();
    }
    view
}

fn property_row<State: 'static>(
    row_data: PropertyRow,
    on_select: Option<StateStringCallback<State>>,
) -> StateView<State> {
    let row_id = row_data.id.clone();
    let selected = row_data.selected;
    let label = property_cell(
        row_data.label,
        format!("property-{}-label", row_data.id),
        None,
    );
    let value = property_cell(
        row_data.value,
        format!("property-{}-value", row_data.id),
        on_select.map(|on_select| {
            let row_id = row_id.clone();
            Arc::new(move |state: &mut State| on_select(state, row_id.clone()))
                as StateCallback<State>
        }),
    );
    let mut view = row([label.size(112.0, 20.0), value.fill_width().height(20.0)])
        .key(format!("property-row-{}", row_data.id))
        .fill_width()
        .height(24.0)
        .padding_x(6.0)
        .padding_y(1.0)
        .spacing(6.0)
        .style(if selected {
            WidgetStyle {
                tone: WidgetTone::Accent,
                prominence: WidgetProminence::Subtle,
            }
        } else {
            WidgetStyle::default()
        })
        .hoverable();
    if selected {
        view = view.primary();
    }
    view
}

fn message_property_row<Message>(
    row_data: PropertyRow,
    select_message: Option<Arc<dyn Fn(String) -> Message + Send + Sync>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let row_id = row_data.id.clone();
    let selected = row_data.selected;
    let label = message_property_cell(
        row_data.label,
        format!("property-{}-label", row_data.id),
        None,
    );
    let value = message_property_cell(
        row_data.value,
        format!("property-{}-value", row_data.id),
        select_message.map(|select_message| select_message(row_id)),
    );
    let mut view = row([label.size(112.0, 20.0), value.fill_width().height(20.0)])
        .key(format!("property-row-{}", row_data.id))
        .fill_width()
        .height(24.0)
        .padding_x(6.0)
        .padding_y(1.0)
        .spacing(6.0)
        .style(if selected {
            WidgetStyle {
                tone: WidgetTone::Accent,
                prominence: WidgetProminence::Subtle,
            }
        } else {
            WidgetStyle::default()
        })
        .hoverable();
    if selected {
        view = view.primary();
    }
    view
}

fn property_cell<State: 'static>(
    value: String,
    key: String,
    on_select: Option<StateCallback<State>>,
) -> StateView<State> {
    if let Some(on_select) = on_select {
        button(value)
            .on_click(move |state: &mut State| on_select(state))
            .key(key)
            .subtle()
            .fill_width()
            .height(20.0)
    } else {
        text(value).key(key).fill_width().height(20.0)
    }
}

fn message_property_cell<Message>(
    value: String,
    key: String,
    message: Option<Message>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    if let Some(message) = message {
        button(value)
            .message(message)
            .key(key)
            .subtle()
            .fill_width()
            .height(20.0)
    } else {
        text(value).key(key).fill_width().height(20.0)
    }
}
