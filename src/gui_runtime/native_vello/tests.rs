use super::*;
pub(super) use crate::app::{
    BrowserPanelModel, BrowserRowModel, ColumnModel, MapPanelModel, MapPointModel,
    SourcesPanelModel, WaveformPanelModel,
};
use crate::gui::types::Vector2;
use std::collections::{HashMap, VecDeque};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
pub(super) use winit::event::{MouseButton, MouseScrollDelta};

fn milli(value: impl Into<i64>) -> u32 {
    value.into() as u32 * 1000
}

#[derive(Default)]
struct RecordingBridge {
    actions: Vec<UiAction>,
}

impl NativeAppBridge for RecordingBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        Arc::new(AppModel::default())
    }

    fn reduce_action(&mut self, action: UiAction) {
        self.actions.push(action);
    }
}

fn browser_model_with_rows(total: usize, focused_visible_row: usize) -> AppModel {
    let mut model = AppModel::default();
    for visible_row in 0..total {
        model.browser.rows.push(BrowserRowModel::new(
            visible_row,
            format!("row_{visible_row:04}"),
            1,
            false,
            visible_row == focused_visible_row,
        ));
    }
    model.browser.visible_count = model.browser.rows.len();
    model.browser.autoscroll = true;
    model.browser.selected_visible_row = Some(focused_visible_row);
    model.browser.anchor_visible_row = Some(focused_visible_row.saturating_sub(2));
    model
}

mod browser_pointer;
mod cursor_drag;
mod drag_highlight;
mod key_bindings;
mod queue_runtime;
mod runtime_core;
mod startup;
mod waveform_drag_core;
mod waveform_drag_finish;
mod waveform_fades;
mod waveform_pointer;
