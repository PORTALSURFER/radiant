/// Sort direction displayed by a sortable details-list column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SortDirection {
    /// Ascending sort.
    Ascending,
    /// Descending sort.
    Descending,
}

impl SortDirection {
    fn marker(self) -> &'static str {
        match self {
            Self::Ascending => " ^",
            Self::Descending => " v",
        }
    }

    /// Return the opposite sort direction.
    pub fn toggled(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

/// Current sort state for a details list.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DetailsSort {
    /// Stable sorted column id.
    pub column_id: String,
    /// Current sort direction.
    pub direction: SortDirection,
}

impl DetailsSort {
    /// Build a current sort descriptor.
    pub fn new(column_id: impl ToString, direction: SortDirection) -> Self {
        Self {
            column_id: column_id.to_string(),
            direction,
        }
    }
}

/// One sortable details-list column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailsColumn {
    /// Stable caller-owned column id.
    pub id: String,
    /// Header label.
    pub label: String,
    /// Fixed logical width, or `None` for the flexible primary column.
    pub width: Option<u32>,
}

impl DetailsColumn {
    /// Build a flexible details-list column.
    pub fn flexible(id: impl ToString, label: impl Into<String>) -> Self {
        Self {
            id: id.to_string(),
            label: label.into(),
            width: None,
        }
    }

    /// Build a fixed-width details-list column.
    pub fn fixed(id: impl ToString, label: impl Into<String>, width: u32) -> Self {
        Self {
            id: id.to_string(),
            label: label.into(),
            width: Some(width),
        }
    }
}

/// One compact details-list row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailsRow {
    /// Stable caller-owned row id.
    pub id: String,
    /// Cell text in the same order as the columns.
    pub cells: Vec<String>,
    /// Whether this row is currently selected.
    pub selected: bool,
}

impl DetailsRow {
    /// Build one details-list row.
    pub fn new(
        id: impl ToString,
        cells: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            id: id.to_string(),
            cells: cells.into_iter().map(Into::into).collect(),
            selected: false,
        }
    }

    /// Mark the row as selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

/// Build a compact details list with clickable sort columns.
pub fn sortable_details_list<State: 'static>(
    columns: impl IntoIterator<Item = DetailsColumn>,
    rows: impl IntoIterator<Item = DetailsRow>,
    sort: Option<DetailsSort>,
    on_sort: impl Fn(&mut State, String) + Send + Sync + 'static,
) -> StateView<State> {
    selectable_sortable_details_list(columns, rows, sort, on_sort, None::<fn(&mut State, String)>)
}

/// Build a compact details list with clickable sort columns and selectable rows.
pub fn selectable_sortable_details_list<State: 'static>(
    columns: impl IntoIterator<Item = DetailsColumn>,
    rows: impl IntoIterator<Item = DetailsRow>,
    sort: Option<DetailsSort>,
    on_sort: impl Fn(&mut State, String) + Send + Sync + 'static,
    on_select: Option<impl Fn(&mut State, String) + Send + Sync + 'static>,
) -> StateView<State> {
    let columns = columns.into_iter().collect::<Vec<_>>();
    let on_sort: StateStringCallback<State> = Arc::new(on_sort);
    let on_select: Option<StateStringCallback<State>> =
        on_select.map(|on_select| Arc::new(on_select) as StateStringCallback<State>);

    column([
        details_header(&columns, sort.as_ref(), Arc::clone(&on_sort)),
        scroll(
            column(
                rows.into_iter()
                    .map(|row| details_row(&columns, row, on_select.as_ref().map(Arc::clone)))
                    .collect::<Vec<_>>(),
            )
            .fill_width()
            .spacing(1.0),
        )
        .fill_height(),
    ])
    .fill_width()
    .fill_height()
    .spacing(3.0)
}

fn details_header<State: 'static>(
    columns: &[DetailsColumn],
    sort: Option<&DetailsSort>,
    on_sort: StateStringCallback<State>,
) -> StateView<State> {
    compact_details_row(
        columns
            .iter()
            .map(|column| {
                let id = column.id.clone();
                let on_sort = Arc::clone(&on_sort);
                let marker = sort
                    .filter(|sort| sort.column_id == column.id)
                    .map(|sort| sort.direction.marker())
                    .unwrap_or("");
                let label = format!("{}{}", column.label, marker);
                sized_cell(
                    column,
                    button(label)
                        .on_click(move |state: &mut State| on_sort(state, id.clone()))
                        .key(format!("details-sort-{}", column.id))
                        .subtle(),
                )
            })
            .collect::<Vec<_>>(),
    )
}

fn details_row<State: 'static>(
    columns: &[DetailsColumn],
    row_data: DetailsRow,
    on_select: Option<StateStringCallback<State>>,
) -> StateView<State> {
    let row_id = row_data.id.clone();
    let selectable = on_select.is_some();
    let mut row = compact_details_row(
        columns
            .iter()
            .enumerate()
            .map(|(index, column)| {
                let value = row_data.cells.get(index).cloned().unwrap_or_default();
                let cell = if let Some(on_select) = on_select.as_ref() {
                    let row_id = row_id.clone();
                    let on_select = Arc::clone(on_select);
                    let mut button = button(value)
                        .on_click(move |state: &mut State| on_select(state, row_id.clone()))
                        .key(format!("{}-{index}", row_data.id))
                        .fill_width()
                        .height(20.0);
                    if row_data.selected {
                        button = button.primary();
                    } else {
                        button = button.subtle();
                    };
                    button
                } else {
                    text(value).key(format!("{}-{index}", row_data.id))
                };
                sized_cell(column, cell)
            })
            .collect::<Vec<_>>(),
    )
    .key(format!("details-row-{}", row_data.id))
    .style(if row_data.selected {
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: WidgetProminence::Subtle,
        }
    } else {
        WidgetStyle::default()
    })
    .hoverable();
    if row_data.selected && !selectable {
        row = row.primary();
    }
    row
}

fn sized_cell<State: 'static>(
    column: &DetailsColumn,
    cell: StateView<State>,
) -> StateView<State> {
    match column.width {
        Some(width) => cell.size(width as f32, 20.0),
        None => cell.fill_width().height(20.0),
    }
}

fn compact_details_row<State: 'static>(
    children: impl IntoIterator<Item = StateView<State>>,
) -> StateView<State> {
    row(children)
        .fill_width()
        .height(22.0)
        .padding_x(8.0)
        .padding_y(1.0)
        .spacing(10.0)
}
