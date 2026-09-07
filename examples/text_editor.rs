//! A deterministic, headless controlled multiline-editor fixture.
//!
//! This example intentionally uses a tiny ASCII-only paragraph provider. Native
//! hosts keep their production shaping provider; the receipt here exists only
//! to make the runtime interaction trace reproducible in tests and examples.

use radiant::{
    application::{IntoView, TextEditorDocument, TextEditorEdit, text_editor},
    gui::{
        focus::FocusSurface,
        input::{KeyCode, KeyPress},
        text_layout::{
            editor::{TextEditorGeometryReceipt, TextEditorLayoutRequest},
            paragraph::{
                ClusterCaretOffset, ParagraphBaseDirection, ParagraphGeometry,
                ParagraphGeometryInput, ParagraphGeometryKey, ShapedLogicalCluster,
            },
        },
        types::{Point, Vector2},
    },
    runtime::{Event, RuntimeBridge, SurfaceRuntime, UiSurface},
    widgets::{
        CompositionRange, CompositionSample, TextEditorWidget, TextEditorWidgetParts, WidgetKey,
        WidgetSizing,
    },
};
use std::sync::Arc;

const EDITOR_ID: u64 = 40;
const VIEWPORT: Vector2 = Vector2 { x: 96.0, y: 36.0 };

enum Message {
    Edit(TextEditorEdit),
}

/// The application owns document authority and reprojects after every accepted edit.
struct EditorApp {
    document: TextEditorDocument,
    applied_edits: usize,
}

impl EditorApp {
    fn new() -> Self {
        Self {
            document: TextEditorDocument::new("kick").expect("bounded fixture document"),
            applied_edits: 0,
        }
    }
}

impl RuntimeBridge<Message> for EditorApp {
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        text_editor(self.document.snapshot())
            .id(EDITOR_ID)
            .wrap(true)
            .font_size(14.0)
            .message(Message::Edit)
            .into_surface()
            .into()
    }

    fn reduce_message(&mut self, message: Message) {
        let Message::Edit(edit) = message;
        self.document
            .apply(&edit)
            .expect("runtime only returns the current document's exact edit");
        self.applied_edits += 1;
    }
}

#[derive(Debug, PartialEq, Eq)]
struct FixtureResult {
    text: String,
    applied_edits: usize,
}

fn main() {
    let result = run_fixture();
    println!(
        "{{\"text\":\"{}\",\"applied_edits\":{}}}",
        result.text.replace('\n', "\\n"),
        result.applied_edits,
    );
}

fn run_fixture() -> FixtureResult {
    let mut runtime = SurfaceRuntime::new(EditorApp::new(), VIEWPORT);
    assert!(runtime.focus_widget(EDITOR_ID));

    install_fixture_geometry(&mut runtime);
    dispatch(&mut runtime, Event::key_press(WidgetKey::End));
    dispatch(&mut runtime, Event::character('!'));
    dispatch(&mut runtime, Event::key_press(WidgetKey::Enter));
    for character in "drum".chars() {
        dispatch(&mut runtime, Event::character(character));
    }

    // Metadata-aware focused routing preserves Shift navigation through the real runtime path.
    install_fixture_geometry(&mut runtime);
    assert!(runtime.dispatch_key_press(
        KeyPress::with_shift(KeyCode::ArrowUp),
        Some(WidgetKey::ArrowUp),
        FocusSurface::None,
    ));

    assert_eq!(runtime.bridge().document.snapshot().selection().anchor, 10);
    assert_eq!(runtime.bridge().document.snapshot().selection().caret, 4);

    // Preedit projects into the widget but does not change committed document text.
    install_fixture_geometry(&mut runtime);
    dispatch(&mut runtime, Event::key_press(WidgetKey::Home));
    let empty = CompositionRange::new(0, 0, runtime.bridge().document.text().chars().count())
        .expect("current collapsed composition range");
    assert_eq!(
        runtime.dispatch_composition_sample(
            CompositionSample::start(empty, empty).expect("valid composition start")
        ),
        Some(EDITOR_ID)
    );
    assert_eq!(runtime.bridge().document.text(), "kick!\ndrum");
    let preedit = CompositionRange::new(0, 4, 4).expect("fixture preedit range");
    assert_eq!(
        runtime.dispatch_composition_sample(
            CompositionSample::update("loop", preedit).expect("valid preedit")
        ),
        Some(EDITOR_ID)
    );
    install_fixture_geometry(&mut runtime);
    assert_eq!(
        runtime.dispatch_composition_sample(CompositionSample::commit("loop")),
        Some(EDITOR_ID)
    );

    // A current receipt makes editor-local scrolling and a subsequent reflow deterministic.
    install_fixture_geometry(&mut runtime);
    dispatch(
        &mut runtime,
        Event::scroll(Point::new(12.0, 12.0), Vector2::new(0.0, 400.0)),
    );
    dispatch(&mut runtime, Event::resize(Vector2::new(64.0, 28.0)));
    install_fixture_geometry(&mut runtime);

    FixtureResult {
        text: runtime.bridge().document.text().to_owned(),
        applied_edits: runtime.bridge().applied_edits,
    }
}

fn dispatch(runtime: &mut SurfaceRuntime<EditorApp, Message>, event: Event) {
    runtime.dispatch_event(event);
}

fn install_fixture_geometry(runtime: &mut SurfaceRuntime<EditorApp, Message>) {
    let bounds = runtime.layout().rects[&EDITOR_ID];
    let snapshot = runtime.bridge().document.snapshot();
    let widget = TextEditorWidget::from_parts(TextEditorWidgetParts {
        id: EDITOR_ID,
        snapshot,
        sizing: WidgetSizing::fixed(VIEWPORT),
    });
    let request = widget.layout_request(bounds, &Default::default());
    let receipt = TextEditorGeometryReceipt::new(request.clone(), fixture_geometry(&request))
        .expect("exact ASCII fixture receipt");
    assert!(runtime.install_text_editor_geometry(receipt));
}

fn fixture_geometry(request: &TextEditorLayoutRequest) -> Arc<ParagraphGeometry> {
    assert!(request.text.is_ascii(), "fixture shapes ASCII only");
    let clusters = request
        .text
        .char_indices()
        .filter(|(_, character)| !matches!(character, '\n' | '\r'))
        .map(|(start, character)| ShapedLogicalCluster {
            bytes: start..start + character.len_utf8(),
            advance: 8.0,
            bidi_level: 0,
            safe_break_after: true,
            carets: vec![
                ClusterCaretOffset {
                    byte_offset: 0,
                    x: 0.0,
                },
                ClusterCaretOffset {
                    byte_offset: character.len_utf8() as u32,
                    x: 8.0,
                },
            ],
        })
        .collect();
    Arc::new(
        ParagraphGeometry::build(ParagraphGeometryInput {
            key: ParagraphGeometryKey(EDITOR_ID),
            source: request.text.clone(),
            clusters,
            base_direction: ParagraphBaseDirection::Ltr,
            wrap_width: if request.wrap {
                request.rect.width()
            } else {
                f32::MAX
            },
            line_height: request.line_height,
        })
        .expect("deterministic fixture geometry"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::{application::WritingDirection, gui::types::Rect};

    #[test]
    fn controlled_editor_trace_applies_commits_and_reflows_headlessly() {
        assert_eq!(
            run_fixture(),
            FixtureResult {
                text: "loopkick!\ndrum".into(),
                applied_edits: 12,
            }
        );
    }

    #[test]
    fn fixture_geometry_declares_its_ascii_boundary() {
        let request = TextEditorLayoutRequest {
            widget_id: EDITOR_ID,
            owner: 1,
            revision: 1,
            text: Arc::from("ascii"),
            rect: Rect::from_size(64.0, 24.0),
            font_size: 14.0,
            line_height: 19.6,
            wrap: true,
            direction: WritingDirection::Ltr,
            locale: None,
        };
        assert_eq!(fixture_geometry(&request).source(), "ascii");
    }
}
