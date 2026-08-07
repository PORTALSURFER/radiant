//! macOS-only live acceptance harness for outgoing file drags.
//!
//! Drag the handle out of the window into Finder or another file receiver. The
//! application reports the bounded callback count, terminal effect, accepted
//! state, and whether the completion callback has received its terminal result.

#[cfg(any(target_os = "macos", test))]
use radiant::prelude::*;
#[cfg(any(target_os = "macos", test))]
use radiant::runtime::{
    DragPreview, DragRequest, ExternalDragEffect, ExternalDragOutcome, ExternalDragRequest,
};

#[cfg(any(target_os = "macos", test))]
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::{fs, process, time::SystemTime};

#[cfg(any(target_os = "macos", test))]
const DRAG_HANDLE_ID: u64 = 20;
#[cfg(any(target_os = "macos", test))]
const MAX_CALLBACK_COUNT: u32 = 8;
#[cfg(any(target_os = "macos", test))]
const MAX_STATUS_CHARS: usize = 96;

#[cfg(target_os = "macos")]
struct DisposableSource {
    path: PathBuf,
}

#[cfg(target_os = "macos")]
impl DisposableSource {
    fn create() -> radiant::Result<Self> {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "radiant-macos-external-drag-{}-{nonce}.txt",
            process::id()
        ));
        fs::write(&path, b"Radiant macOS external-drag acceptance source\n")
            .map_err(|error| error.to_string())?;
        Ok(Self { path })
    }
}

#[cfg(target_os = "macos")]
impl Drop for DisposableSource {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(any(target_os = "macos", test))]
struct AcceptanceState {
    source_path: PathBuf,
    #[cfg(target_os = "macos")]
    disposable_source: Option<DisposableSource>,
    callback_count: u32,
    terminal_effect: ExternalDragEffect,
    accepted: bool,
    callback_terminal: bool,
    status: String,
}

#[cfg(any(target_os = "macos", test))]
impl AcceptanceState {
    fn test_state() -> Self {
        Self {
            source_path: PathBuf::from("radiant-external-drag-acceptance.txt"),
            #[cfg(target_os = "macos")]
            disposable_source: None,
            callback_count: 0,
            terminal_effect: ExternalDragEffect::None,
            accepted: false,
            callback_terminal: false,
            status: String::from("Ready to drag the handle into Finder"),
        }
    }

    #[cfg(target_os = "macos")]
    fn live() -> radiant::Result<Self> {
        let source = DisposableSource::create()?;
        let mut state = Self::test_state();
        state.source_path = source.path.clone();
        state.disposable_source = Some(source);
        Ok(state)
    }
}

#[cfg(any(target_os = "macos", test))]
impl Default for AcceptanceState {
    fn default() -> Self {
        Self::test_state()
    }
}

#[cfg(target_os = "macos")]
fn main() -> radiant::Result {
    radiant::app(AcceptanceState::live()?)
        .title("Radiant macOS External Drag Acceptance")
        .size(720, 420)
        .min_size(560, 340)
        .view(project_surface)
        .handle_message(update)
        .run()
}

#[cfg(not(target_os = "macos"))]
fn main() -> radiant::Result {
    Err("macos_external_drag_acceptance is macOS-only".to_owned())
}

#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Debug, PartialEq)]
enum AcceptanceMessage {
    Drag(DragHandleMessage),
    ExternalDragCompleted(std::result::Result<ExternalDragOutcome, String>),
}

#[cfg(any(target_os = "macos", test))]
fn project_surface(state: &AcceptanceState) -> View<AcceptanceMessage> {
    column([
        text("macOS outgoing external-drag acceptance")
            .primary()
            .fill_width(),
        text(
            "Press and drag the handle outside this window. Finder or another file receiver should report the terminal operation back to this same application window.",
        )
        .wrap()
        .fill_width(),
        drag_handle()
            .full_height_rail()
            .mapped(AcceptanceMessage::Drag)
            .id(DRAG_HANDLE_ID)
            .size(360.0, 52.0),
        text(format!(
            "Callback count (bounded at {MAX_CALLBACK_COUNT}): {}",
            state.callback_count
        ))
        .fill_width(),
        text(format!(
            "Terminal effect: {} | Accepted: {} | Callback terminal: {}",
            effect_label(state.terminal_effect),
            state.accepted,
            state.callback_terminal
        ))
        .fill_width(),
        text(format!("Status: {}", state.status))
            .wrap()
            .fill_width(),
    ])
    .padding(24.0)
    .spacing(16.0)
}

#[cfg(any(target_os = "macos", test))]
fn update(
    state: &mut AcceptanceState,
    message: AcceptanceMessage,
    context: &mut UiUpdateContext<AcceptanceMessage>,
) {
    match message {
        AcceptanceMessage::Drag(DragHandleMessage::Started { position, .. }) => {
            state.status = String::from("Native drag armed; move the pointer outside the window");
            context.begin_drag_with_external(
                DragRequest::new(
                    DragPreview::sized("Radiant file", Vector2::new(180.0, 36.0)),
                    position,
                ),
                ExternalDragRequest::files([state.source_path.clone()], "Radiant external file"),
                AcceptanceMessage::ExternalDragCompleted,
            );
        }
        AcceptanceMessage::Drag(DragHandleMessage::Ended { .. }) => {
            state.status = String::from("Drag ended inside the application");
            context.end_drag_session();
        }
        AcceptanceMessage::Drag(DragHandleMessage::Cancelled { .. }) => {
            state.status =
                String::from("Pointer drag cancelled; awaiting any native terminal result");
        }
        AcceptanceMessage::Drag(DragHandleMessage::Moved { .. })
        | AcceptanceMessage::Drag(DragHandleMessage::DoubleActivate { .. }) => {}
        AcceptanceMessage::ExternalDragCompleted(result) => {
            state.callback_count = state
                .callback_count
                .saturating_add(1)
                .min(MAX_CALLBACK_COUNT);
            state.callback_terminal = true;
            match result {
                Ok(outcome) => {
                    state.terminal_effect = outcome.effect;
                    state.accepted = outcome.accepted();
                    state.status = format!(
                        "Native drag completed with {}",
                        effect_label(state.terminal_effect)
                    );
                }
                Err(error) => {
                    state.terminal_effect = ExternalDragEffect::None;
                    state.accepted = false;
                    state.status = format!("Native drag failed: {}", bounded_status(&error));
                }
            }
        }
    }
}

#[cfg(any(target_os = "macos", test))]
fn effect_label(effect: ExternalDragEffect) -> &'static str {
    match effect {
        ExternalDragEffect::None => "none",
        ExternalDragEffect::Copy => "copy",
        ExternalDragEffect::Move => "move",
        ExternalDragEffect::Link => "link",
    }
}

#[cfg(any(target_os = "macos", test))]
fn bounded_status(value: &str) -> String {
    value.chars().take(MAX_STATUS_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::{layout::Vector2, runtime::SurfaceRuntime};

    #[test]
    fn acceptance_surface_arms_external_drag_through_update_context() {
        let bridge = radiant::app(AcceptanceState::default())
            .view(project_surface)
            .handle_message(update)
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(720.0, 420.0));

        runtime.dispatch_message(AcceptanceMessage::Drag(DragHandleMessage::started(
            Point::new(120.0, 80.0),
        )));

        assert!(runtime.external_drag_armed());
        assert!(runtime.drag_session_active());
        assert!(runtime.surface().find_widget(DRAG_HANDLE_ID).is_some());
    }

    #[test]
    fn acceptance_state_reports_terminal_copy_and_bounds_callback_count() {
        let mut state = AcceptanceState::default();
        let terminal = Ok(ExternalDragOutcome {
            effect: ExternalDragEffect::Copy,
        });

        for _ in 0..(MAX_CALLBACK_COUNT + 3) {
            update(
                &mut state,
                AcceptanceMessage::ExternalDragCompleted(terminal.clone()),
                &mut UiUpdateContext::default(),
            );
        }

        assert_eq!(state.callback_count, MAX_CALLBACK_COUNT);
        assert_eq!(state.terminal_effect, ExternalDragEffect::Copy);
        assert!(state.accepted);
        assert!(state.callback_terminal);
    }

    #[test]
    fn acceptance_state_reports_immediate_error_as_terminal_unaccepted() {
        let mut state = AcceptanceState::default();
        update(
            &mut state,
            AcceptanceMessage::ExternalDragCompleted(Err(String::from("native start failed"))),
            &mut UiUpdateContext::default(),
        );

        assert_eq!(state.callback_count, 1);
        assert_eq!(state.terminal_effect, ExternalDragEffect::None);
        assert!(!state.accepted);
        assert!(state.callback_terminal);
        assert!(state.status.contains("native start failed"));
    }

    #[test]
    fn acceptance_state_bounds_terminal_error_status() {
        let mut state = AcceptanceState::default();
        let error = "x".repeat(MAX_STATUS_CHARS + 20);

        update(
            &mut state,
            AcceptanceMessage::ExternalDragCompleted(Err(error)),
            &mut UiUpdateContext::default(),
        );

        assert_eq!(
            state.status.chars().count(),
            "Native drag failed: ".chars().count() + MAX_STATUS_CHARS
        );
    }
}
