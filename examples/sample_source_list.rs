//! Stateful sample-source list with selection-scoped add and remove actions.

use radiant::prelude::*;

#[derive(Clone, Debug)]
struct SampleSource {
    id: u64,
    name: String,
    folder: String,
}

#[derive(Clone, Debug)]
struct SourceListState {
    sources: Vec<SampleSource>,
    selected_id: Option<u64>,
    next_id: u64,
}

impl Default for SourceListState {
    fn default() -> Self {
        Self {
            sources: vec![
                SampleSource {
                    id: 1,
                    name: "Drum Loops".to_string(),
                    folder: "Samples/Drums".to_string(),
                },
                SampleSource {
                    id: 2,
                    name: "Bass One Shots".to_string(),
                    folder: "Samples/Bass".to_string(),
                },
                SampleSource {
                    id: 3,
                    name: "Field Recordings".to_string(),
                    folder: "Library/Field".to_string(),
                },
            ],
            selected_id: Some(1),
            next_id: 4,
        }
    }
}

impl SourceListState {
    fn select(&mut self, id: u64) {
        self.selected_id = Some(id);
    }

    fn add_after(&mut self, after_id: Option<u64>) {
        let id = self.next_id;
        self.next_id += 1;
        let source = SampleSource {
            id,
            name: format!("New Source {id}"),
            folder: "Choose a folder".to_string(),
        };
        let insert_at = after_id
            .and_then(|id| self.sources.iter().position(|source| source.id == id))
            .map_or(self.sources.len(), |index| index + 1);
        self.sources.insert(insert_at, source);
        self.selected_id = Some(id);
    }

    fn remove(&mut self, id: u64) {
        let Some(index) = self.sources.iter().position(|source| source.id == id) else {
            return;
        };
        self.sources.remove(index);
        self.selected_id = self
            .sources
            .get(index)
            .or_else(|| self.sources.last())
            .map(|s| s.id);
    }
}

fn main() -> radiant::Result {
    radiant::app(SourceListState::default())
        .title("Radiant Sample Sources")
        .size(520, 360)
        .min_size(380, 240)
        .view(project_surface)
        .run()
}

fn project_surface(state: &mut SourceListState) -> StateView<SourceListState> {
    column([
        row([
            text("Sample Sources").height(30.0).fill_width(),
            button("+")
                .primary()
                .on_click(|state: &mut SourceListState| state.add_after(state.selected_id))
                .size(32.0, 32.0),
        ])
        .fill_width()
        .spacing(10.0),
        list(state.sources.iter().cloned(), |source| {
            source_row(source, state.selected_id)
        })
        .fill_height(),
        text(selection_summary(state)).height(26.0).fill_width(),
    ])
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

fn source_row(source: SampleSource, selected_id: Option<u64>) -> StateView<SourceListState> {
    let id = source.id;
    let selected = selected_id == Some(id);
    list_row_id(
        id,
        [
            selectable(source.name, selected)
                .on_change(move |state: &mut SourceListState, selected| {
                    if selected {
                        state.select(id);
                    }
                })
                .fill_width(),
            text(source.folder).height(28.0).fill_width(),
            button("+")
                .subtle()
                .on_click(move |state: &mut SourceListState| state.add_after(Some(id)))
                .size(32.0, 32.0),
            selected_remove_button(id, selected),
        ],
    )
}

fn selected_remove_button(id: u64, selected: bool) -> StateView<SourceListState> {
    if selected {
        button("-")
            .danger()
            .on_click(move |state: &mut SourceListState| state.remove(id))
            .size(32.0, 32.0)
    } else {
        text("").size(32.0, 32.0)
    }
}

fn selection_summary(state: &SourceListState) -> String {
    match state
        .selected_id
        .and_then(|id| state.sources.iter().find(|source| source.id == id))
    {
        Some(source) => format!("Selected: {} ({})", source.name, source.folder),
        None => "No source selected".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;

    #[test]
    fn source_list_adds_after_selection_and_removes_selected_source() {
        let mut state = SourceListState::default();

        state.add_after(state.selected_id);
        assert_eq!(state.sources[1].name, "New Source 4");
        assert_eq!(state.selected_id, Some(4));

        state.remove(4);
        assert_eq!(state.sources.len(), 3);
        assert_eq!(state.selected_id, Some(2));
    }

    #[test]
    fn sample_source_list_projects_focusable_row_actions() {
        let mut state = SourceListState::default();
        let surface = project_surface(&mut state).into_surface();

        assert!(surface.keyboard_focus_order().len() >= 8);
    }
}
