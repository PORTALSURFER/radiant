use super::*;

fn browser_drag_model() -> AppModel {
    let mut model = browser_model_with_rows(4, 0);
    let tree_rows = vec![
        crate::compat_app_contract::FolderRowModel::new(
            "Root", "", 0, false, false, true, true, true,
        )
        .with_backing_index(0),
        crate::compat_app_contract::FolderRowModel::new(
            "Group A", "group-a", 1, false, false, false, true, true,
        )
        .with_backing_index(7),
    ];
    model.sources = SourcesPanelModel {
        tree_rows: tree_rows.clone().into(),
        upper_folder_pane: crate::compat_app_contract::FolderPaneModel {
            pane: crate::compat_app_contract::FolderPaneIdModel::Upper,
            title: String::from("Upper"),
            active: true,
            has_item: true,
            tree_rows: tree_rows.clone().into(),
            ..crate::compat_app_contract::FolderPaneModel::default()
        },
        lower_folder_pane: crate::compat_app_contract::FolderPaneModel {
            pane: crate::compat_app_contract::FolderPaneIdModel::Lower,
            title: String::from("Lower"),
            has_item: true,
            tree_rows: tree_rows.into(),
            ..crate::compat_app_contract::FolderPaneModel::default()
        },
        ..SourcesPanelModel::default()
    };
    model
}

fn browser_row_point(layout: &ShellLayout) -> Point {
    let style = StyleTokens::for_viewport_width(layout.root.rect.width());
    Point::new(
        layout.browser_rows.min.x + 24.0,
        layout.browser_rows.min.y + (style.sizing.browser_row_height * 0.5),
    )
}

fn folder_row_point(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
    row_index: usize,
) -> Point {
    let row_rect = shell_state.rendered_folder_row_rects(layout, model)[row_index];
    let seam_stroke = StyleTokens::for_viewport_width(layout.root.rect.width())
        .sizing
        .border_width
        .max(1.0);
    Point::new(
        row_rect.min.x + seam_stroke + 2.0,
        (row_rect.min.y + row_rect.max.y) * 0.5,
    )
}

fn folder_panel_background_point(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
) -> Point {
    for x in layout.sidebar_rows.min.x as i32..=layout.sidebar_rows.max.x as i32 {
        for y in layout.sidebar_rows.min.y as i32..=layout.sidebar_rows.max.y as i32 {
            let point = Point::new(x as f32, y as f32);
            if shell_state.folder_panel_contains_point(layout, model, point)
                && shell_state
                    .folder_row_at_point(layout, model, point)
                    .is_none()
                && shell_state
                    .source_row_at_point(layout, model, point)
                    .is_none()
            {
                return point;
            }
        }
    }
    panic!("expected hittable folder-panel background point");
}

#[derive(Default)]
struct WaveformZoomRefreshBridge {
    actions: Vec<UiAction>,
    model: AppModel,
    project_calls: usize,
}

impl NativeAppBridge for WaveformZoomRefreshBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        self.project_calls = self.project_calls.saturating_add(1);
        Arc::new(self.model.clone())
    }

    fn reduce_action(&mut self, action: UiAction) {
        if matches!(action, UiAction::ZoomWaveform { .. }) {
            self.model.waveform.view_start_micros = 100_000;
            self.model.waveform.view_end_micros = 900_000;
            self.model.waveform.view_start_nanos = 100_000_000;
            self.model.waveform.view_end_nanos = 900_000_000;
        }
        self.actions.push(action);
    }
}

#[derive(Default)]
struct WaveformNoopZoomRefreshBridge {
    actions: Vec<UiAction>,
    model: AppModel,
    project_calls: usize,
}

impl NativeAppBridge for WaveformNoopZoomRefreshBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        self.project_calls = self.project_calls.saturating_add(1);
        Arc::new(self.model.clone())
    }

    fn reduce_action(&mut self, action: UiAction) {
        self.actions.push(action);
    }
}

#[derive(Default)]
struct DeepZoomClickRefreshBridge {
    actions: Vec<UiAction>,
    model: AppModel,
    project_calls: usize,
}

impl NativeAppBridge for DeepZoomClickRefreshBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        self.project_calls = self.project_calls.saturating_add(1);
        Arc::new(self.model.clone())
    }

    fn reduce_action(&mut self, action: UiAction) {
        if matches!(action, UiAction::ZoomWaveform { .. }) {
            self.model.waveform.view_start_micros = 500_000;
            self.model.waveform.view_end_micros = 500_000;
            self.model.waveform.view_start_nanos = 500_000_000;
            self.model.waveform.view_end_nanos = 500_000_200;
        }
        self.actions.push(action);
    }
}

#[derive(Default)]
struct DeepZoomPanRefreshBridge {
    actions: Vec<UiAction>,
    model: AppModel,
    project_calls: usize,
}

impl NativeAppBridge for DeepZoomPanRefreshBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        self.project_calls = self.project_calls.saturating_add(1);
        Arc::new(self.model.clone())
    }

    fn reduce_action(&mut self, action: UiAction) {
        if let UiAction::SetWaveformViewCenter {
            center_micros,
            center_nanos,
        } = &action
        {
            let span_nanos = self
                .model
                .waveform
                .view_end_nanos
                .saturating_sub(self.model.waveform.view_start_nanos);
            let resolved_center_nanos = center_nanos.unwrap_or(center_micros.saturating_mul(1000));
            let half_span = span_nanos / 2;
            let max_start_nanos = 1_000_000_000u32.saturating_sub(span_nanos);
            let next_start_nanos = resolved_center_nanos
                .saturating_sub(half_span)
                .min(max_start_nanos);
            let next_end_nanos = next_start_nanos
                .saturating_add(span_nanos)
                .min(1_000_000_000);
            self.model.waveform.view_start_nanos = next_start_nanos;
            self.model.waveform.view_end_nanos = next_end_nanos;
            self.model.waveform.view_start_micros = next_start_nanos / 1000;
            self.model.waveform.view_end_micros = next_end_nanos / 1000;
        }
        self.actions.push(action);
    }
}

#[derive(Default)]
struct WaveformScrollbarPressBridge {
    actions: Vec<UiAction>,
    model: AppModel,
    project_calls: usize,
}

impl NativeAppBridge for WaveformScrollbarPressBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        self.project_calls = self.project_calls.saturating_add(1);
        Arc::new(self.model.clone())
    }

    fn reduce_action(&mut self, action: UiAction) {
        if matches!(action, UiAction::ZoomWaveform { .. }) {
            self.model.waveform.view_start_micros = 0;
            self.model.waveform.view_end_micros = 250_000;
            self.model.waveform.view_start_nanos = 0;
            self.model.waveform.view_end_nanos = 250_000_000;
        }
        self.actions.push(action);
    }
}

#[derive(Default)]
struct ImmediateWaveformSelectionBridge {
    actions: Vec<UiAction>,
    model: AppModel,
}

impl NativeAppBridge for ImmediateWaveformSelectionBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        Arc::new(self.model.clone())
    }

    fn resolve_hotkey_press(
        &mut self,
        pending_chord: Option<crate::compat_app_contract::KeyPress>,
        press: crate::compat_app_contract::KeyPress,
        focus: crate::compat_app_contract::FocusContextModel,
    ) -> crate::compat_app_contract::HotkeyResolution {
        super::key_bindings::default_hotkey_resolver(pending_chord, press, focus)
    }

    fn reduce_action(&mut self, action: UiAction) {
        match &action {
            UiAction::BeginWaveformSelectionAt { .. } => {
                self.model.focus_context = crate::compat_app_contract::FocusContextModel::Timeline;
            }
            UiAction::BeginWaveformSelectionAtPrecise { .. } => {
                self.model.focus_context = crate::compat_app_contract::FocusContextModel::Timeline;
            }
            UiAction::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                ..
            } => {
                self.model.focus_context = crate::compat_app_contract::FocusContextModel::Timeline;
                self.model.waveform.selection_milli = Some(
                    crate::compat_app_contract::NormalizedRangeModel::from_micros(
                        *start_micros,
                        *end_micros,
                    ),
                );
            }
            UiAction::SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                ..
            } => {
                self.model.focus_context = crate::compat_app_contract::FocusContextModel::Timeline;
                self.model.waveform.selection_milli = Some(
                    crate::compat_app_contract::NormalizedRangeModel::from_nanos(
                        *start_nanos,
                        *end_nanos,
                    ),
                );
            }
            UiAction::FinishWaveformSelectionDrag => {
                self.model.focus_context = crate::compat_app_contract::FocusContextModel::Timeline;
            }
            UiAction::FinishWaveformSelectionRangeDrag => {
                self.model.focus_context = crate::compat_app_contract::FocusContextModel::Timeline;
            }
            _ => {}
        }
        self.actions.push(action);
    }
}

#[derive(Default)]
struct QueuedWaveformClickBridge {
    actions: Vec<UiAction>,
    model: AppModel,
    queued_actions: Vec<UiAction>,
    project_calls: usize,
}

impl NativeAppBridge for QueuedWaveformClickBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        self.project_calls = self.project_calls.saturating_add(1);
        for action in self.queued_actions.drain(..) {
            match &action {
                UiAction::ClearWaveformSelection => {
                    self.model.waveform.selection_milli = None;
                }
                UiAction::SeekWaveformPrecise { position_nanos }
                | UiAction::SetWaveformCursorPrecise { position_nanos } => {
                    self.model.waveform.cursor_milli = Some((position_nanos / 1_000_000) as u16);
                }
                UiAction::PlayWaveformAtPrecise { position_nanos } => {
                    self.model.waveform.cursor_milli = Some((position_nanos / 1_000_000) as u16);
                    self.model.transport_running = true;
                }
                UiAction::PlayFromCurrentPlayhead | UiAction::PlayFromWaveformCursor => {
                    self.model.transport_running = true;
                }
                _ => {}
            }
            self.actions.push(action);
        }
        Arc::new(self.model.clone())
    }

    fn reduce_action(&mut self, action: UiAction) {
        match &action {
            UiAction::BeginWaveformSelectionAt { .. } => {
                self.model.focus_context = crate::compat_app_contract::FocusContextModel::Timeline;
                self.actions.push(action);
            }
            UiAction::ClearWaveformSelection | UiAction::SeekWaveformPrecise { .. } => {
                self.queued_actions.push(action);
            }
            UiAction::SetWaveformCursorPrecise { position_nanos } => {
                self.model.waveform.cursor_milli = Some((position_nanos / 1_000_000) as u16);
                self.actions.push(action);
            }
            UiAction::PlayWaveformAtPrecise { position_nanos } => {
                self.model.waveform.cursor_milli = Some((position_nanos / 1_000_000) as u16);
                self.model.transport_running = true;
                self.actions.push(action);
            }
            UiAction::PlayFromCurrentPlayhead | UiAction::PlayFromWaveformCursor => {
                self.model.transport_running = true;
                self.actions.push(action);
            }
            _ => {
                self.actions.push(action);
            }
        }
    }
}

#[derive(Default)]
struct FolderCreateCancelBridge {
    actions: Vec<UiAction>,
    model: AppModel,
}

impl NativeAppBridge for FolderCreateCancelBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        Arc::new(self.model.clone())
    }

    fn reduce_action(&mut self, action: UiAction) {
        self.actions.push(action);
    }
}

mod browser;
mod coalesced_actions;
mod waveform_motion;
mod waveform_refresh;
mod waveform_selection;
