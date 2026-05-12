use crate::{
    application::{StateCallback, StateStringCallback, StateView, button, column, row, text},
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};
use std::sync::Arc;

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
    /// Build one property row.
    pub fn new(id: impl ToString, label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            id: id.to_string(),
            label: label.into(),
            value: value.into(),
            selected: false,
        }
    }

    /// Mark the row as selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

/// Build a read-only inspector/property panel.
pub fn property_panel<State: 'static>(
    title: impl Into<String>,
    rows: impl IntoIterator<Item = PropertyRow>,
) -> StateView<State> {
    selectable_property_panel(title, rows, None::<fn(&mut State, String)>)
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
        text(title.into()).height(24.0).fill_width(),
        column(
            rows.into_iter()
                .map(|row| property_row(row, on_select.as_ref().map(Arc::clone))),
        )
        .fill_width()
        .spacing(2.0),
    ])
    .style(WidgetStyle::default())
    .fill_width()
    .padding(10.0)
    .spacing(6.0)
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
    let mut view = row([label.size(128.0, 24.0), value.fill_width().height(24.0)])
        .key(format!("property-row-{}", row_data.id))
        .fill_width()
        .height(28.0)
        .padding_x(8.0)
        .padding_y(2.0)
        .spacing(10.0)
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
            .height(24.0)
    } else {
        text(value).key(key).fill_width().height(24.0)
    }
}
