//! Compatibility shell snapshot capture used by host-owned GUI fixtures.

use super::AppModel;
use crate::gui::{
    native_shell::{NativeShellState, ShellLayout, ShellLayoutRuntime, StyleTokens},
    paint::PaintFrame as NativeViewFrame,
    snapshot::{VisualSnapshot, visual_snapshot_from_paint_frame},
    types::Vector2,
};

/// Compatibility alias for generic visual snapshots captured from the legacy shell.
pub type NativeShellShotSnapshot = VisualSnapshot;

/// Capture a deterministic native-shell visual snapshot without launching a window.
pub fn capture_native_shell_shot_snapshot(
    name: impl Into<String>,
    viewport: [f32; 2],
    model: &AppModel,
) -> NativeShellShotSnapshot {
    let viewport = Vector2::new(viewport[0].max(1.0), viewport[1].max(1.0));
    let style = StyleTokens::for_viewport_width(viewport.x);
    let mut runtime = ShellLayoutRuntime::default();
    let layout = ShellLayout::build_with_style_and_runtime(viewport, &style, &mut runtime);
    let mut state = NativeShellState::new();
    state.sync_from_model(model);
    let mut frame = NativeViewFrame {
        clear_color: style.clear_color,
        primitives: Vec::new(),
        text_runs: Vec::new(),
    };
    state.build_frame_with_style_into_static(&layout, &style, model, &mut frame);
    visual_snapshot_from_paint_frame(
        name,
        [layout.root.rect.width(), layout.root.rect.height()],
        &frame,
    )
}
