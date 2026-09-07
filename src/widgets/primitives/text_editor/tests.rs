use super::*;
use crate::{
    gui::{
        text_layout::{
            editor::TextEditorGeometryReceipt,
            paragraph::{
                ClusterCaretOffset, ParagraphBaseDirection, ParagraphGeometry,
                ParagraphGeometryInput, ParagraphGeometryKey, ShapedLogicalCluster,
            },
        },
        types::{Point, Rect, Vector2},
    },
    runtime::ResolvedEnvironment,
    widgets::{
        CompositionRange, CompositionSample, KeyboardModifiers, PointerModifiers, TextEditCommand,
        WheelDelta, WheelSample, Widget, WidgetInput, WidgetKey, WidgetSizing,
    },
};
use std::{sync::Arc, time::Instant};

fn bounds(width: f32, height: f32) -> Rect {
    Rect::from_xy_size(0.0, 0.0, width, height)
}

fn editor(document: &TextEditorDocument) -> TextEditorWidget {
    TextEditorWidget::from_parts(TextEditorWidgetParts {
        id: 91,
        snapshot: document.snapshot(),
        sizing: WidgetSizing::fixed(Vector2::new(180.0, 80.0)),
    })
}

/// A deliberately simple, deterministic host-shaping fixture. Each non-break
/// ASCII scalar is a complete one-cell logical cluster; production shaping is
/// intentionally not used by these widget-contract tests.
fn geometry(
    request: &crate::gui::text_layout::editor::TextEditorLayoutRequest,
) -> Arc<ParagraphGeometry> {
    let clusters = request
        .text
        .char_indices()
        .filter(|(_, character)| !matches!(character, '\n' | '\r'))
        .map(|(start, character)| {
            let length = character.len_utf8();
            ShapedLogicalCluster {
                bytes: start..start + length,
                advance: 10.0,
                bidi_level: 0,
                safe_break_after: true,
                carets: vec![
                    ClusterCaretOffset {
                        byte_offset: 0,
                        x: 0.0,
                    },
                    ClusterCaretOffset {
                        byte_offset: length as u32,
                        x: 10.0,
                    },
                ],
            }
        })
        .collect();
    Arc::new(
        ParagraphGeometry::build(ParagraphGeometryInput {
            key: ParagraphGeometryKey(4),
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
        .expect("deterministic editor geometry"),
    )
}

fn install(widget: &mut TextEditorWidget, bounds: Rect, environment: &ResolvedEnvironment) {
    let request = widget.layout_request(bounds, environment);
    let receipt = TextEditorGeometryReceipt::new(
        request,
        geometry(&widget.layout_request(bounds, environment)),
    )
    .expect("exact deterministic receipt");
    assert!(Widget::install_text_editor_geometry(
        widget,
        receipt,
        bounds,
        environment
    ));
}

fn input(
    widget: &mut TextEditorWidget,
    bounds: Rect,
    environment: &ResolvedEnvironment,
    event: WidgetInput,
) -> Option<crate::widgets::WidgetOutput> {
    Widget::handle_input_with_environment(widget, bounds, event, environment)
}

fn environment_with_text_scale(scale: f32) -> ResolvedEnvironment {
    let application = crate::application::ApplicationEnvironment::default()
        .with_text_scale(crate::application::TextScale::new(scale).expect("valid test text scale"));
    ResolvedEnvironment::from_snapshots(
        crate::runtime::WindowEnvironment::default(),
        Arc::new(application),
    )
}

fn wheel(delta: Vector2) -> WheelSample {
    WheelSample::discrete(
        WheelDelta::pixels(delta).expect("finite test wheel delta"),
        PointerModifiers::default(),
    )
    .expect("valid test wheel sample")
}

#[test]
fn controlled_edits_emit_exact_deltas_and_reprojection_fences_stale_or_equal_authority() {
    let mut document = TextEditorDocument::new("a").unwrap();
    let environment = ResolvedEnvironment::default();
    let viewport = bounds(120.0, 48.0);
    let stale_snapshot = document.snapshot();
    let mut current = editor(&document);
    input(
        &mut current,
        viewport,
        &environment,
        WidgetInput::FocusChanged(true),
    );
    let output = input(
        &mut current,
        viewport,
        &environment,
        WidgetInput::character('!'),
    )
    .expect("focused editor emits an edit");
    let edit = output.typed_cloned::<TextEditorEdit>().expect("typed edit");
    assert_eq!(edit.expected_revision(), document.revision());
    assert_eq!(current.text(), "!a");

    let mut stale = TextEditorWidget::from_parts(TextEditorWidgetParts {
        id: 91,
        snapshot: stale_snapshot,
        sizing: WidgetSizing::fixed(Vector2::new(180.0, 80.0)),
    });
    Widget::synchronize_from_previous(&mut stale, &current);
    assert_eq!(
        stale.text(),
        "!a",
        "older controlled projection preserves optimism"
    );

    document.apply(&edit).unwrap();
    let mut equal = editor(&document);
    Widget::synchronize_from_previous(&mut equal, &current);
    assert_eq!(
        equal.text(),
        "!a",
        "equal controlled projection preserves optimism"
    );

    document.set_text("server").unwrap();
    let mut newer = editor(&document);
    Widget::synchronize_from_previous(&mut newer, &current);
    assert_eq!(newer.text(), "server", "newer application authority wins");
    assert_eq!(document.apply(&edit), Err(TextEditorError::StaleRevision));
}

#[test]
fn geometry_receipts_require_exact_owner_revision_and_viewport_width() {
    let document = TextEditorDocument::new("abcd").unwrap();
    let environment = ResolvedEnvironment::default();
    let viewport = bounds(80.0, 42.0);
    let mut widget = editor(&document);
    let request = widget.layout_request(viewport, &environment);
    let shape = geometry(&request);
    let receipt = TextEditorGeometryReceipt::new(request.clone(), Arc::clone(&shape)).unwrap();
    assert!(widget.install_geometry(receipt, viewport, &environment));

    let mut wrong_owner = request.clone();
    wrong_owner.owner = wrong_owner.owner.saturating_add(1);
    let wrong_owner = TextEditorGeometryReceipt::new(wrong_owner, Arc::clone(&shape)).unwrap();
    assert!(!widget.install_geometry(wrong_owner, viewport, &environment));

    let mut wrong_revision = request.clone();
    wrong_revision.revision = wrong_revision.revision.saturating_add(1);
    let wrong_revision =
        TextEditorGeometryReceipt::new(wrong_revision, Arc::clone(&shape)).unwrap();
    assert!(!widget.install_geometry(wrong_revision, viewport, &environment));

    let mut wrong_width = request;
    wrong_width.rect = Rect::from_xy_size(
        wrong_width.rect.min.x,
        wrong_width.rect.min.y,
        wrong_width.rect.width() + 1.0,
        wrong_width.rect.height(),
    );
    assert!(TextEditorGeometryReceipt::new(wrong_width, shape).is_none());
}

#[test]
fn focus_transition_preserves_a_current_nondefault_environment_receipt() {
    let document = TextEditorDocument::new("abcd").unwrap();
    let environment = environment_with_text_scale(1.5);
    let viewport = bounds(80.0, 42.0);
    let mut widget = editor(&document);
    install(&mut widget, viewport, &environment);

    Widget::handle_focus_changed_at(&mut widget, viewport, true, Instant::now());
    let output = input(
        &mut widget,
        viewport,
        &environment,
        WidgetInput::primary_press(Point::new(74.0, 10.0)),
    );

    assert!(output.is_some(), "focus must not discard the accepted receipt");
    assert_eq!(widget.selection().caret, 4);
}

#[test]
fn vertical_navigation_preserves_shift_anchor_and_drag_selection_uses_exact_geometry() {
    let document = TextEditorDocument::new("aa\nbb\ncc").unwrap();
    let environment = ResolvedEnvironment::default();
    let viewport = bounds(80.0, 64.0);
    let mut widget = editor(&document);
    input(
        &mut widget,
        viewport,
        &environment,
        WidgetInput::FocusChanged(true),
    );
    install(&mut widget, viewport, &environment);
    input(
        &mut widget,
        viewport,
        &environment,
        WidgetInput::text_edit(TextEditCommand::MoveEnd {
            extend_selection: false,
        }),
    );
    install(&mut widget, viewport, &environment);
    input(
        &mut widget,
        viewport,
        &environment,
        WidgetInput::KeyPress {
            key: WidgetKey::ArrowDown,
            modifiers: KeyboardModifiers {
                shift: true,
                ..KeyboardModifiers::default()
            },
            repeat: false,
            timestamp: None,
        },
    );
    assert_eq!(widget.selection().anchor, 2);
    assert_eq!(widget.selection().caret, 5);

    install(&mut widget, viewport, &environment);
    input(
        &mut widget,
        viewport,
        &environment,
        WidgetInput::primary_press(Point::new(6.0, 6.0)),
    );
    install(&mut widget, viewport, &environment);
    input(
        &mut widget,
        viewport,
        &environment,
        WidgetInput::pointer_move(Point::new(22.0, 6.0)),
    );
    input(
        &mut widget,
        viewport,
        &environment,
        WidgetInput::primary_release(Point::new(22.0, 6.0)),
    );
    assert_eq!(widget.selection().range(), 0..2);
}

#[test]
fn reflowed_geometry_reveals_offscreen_selection_and_clamps_scroll() {
    let document = TextEditorDocument::new("a\nb\nc\nd").unwrap();
    let environment = ResolvedEnvironment::default();
    let viewport = bounds(80.0, 28.0);
    let mut widget = editor(&document);
    input(
        &mut widget,
        viewport,
        &environment,
        WidgetInput::FocusChanged(true),
    );
    install(&mut widget, viewport, &environment);
    for _ in 0..3 {
        input(
            &mut widget,
            viewport,
            &environment,
            WidgetInput::key_press(WidgetKey::ArrowDown),
        );
        install(&mut widget, viewport, &environment);
    }
    assert!(widget.scroll_offset().y > 0.0);

    let wider = bounds(160.0, 28.0);
    install(&mut widget, wider, &environment);
    assert!(widget.scroll_offset().x >= 0.0 && widget.scroll_offset().y >= 0.0);
}

#[test]
fn wheel_rejects_a_stale_receipt_after_text_scale_and_wrap_change_at_stable_bounds() {
    let document = TextEditorDocument::new("a ".repeat(200)).unwrap();
    let initial_environment = ResolvedEnvironment::default();
    let scaled_environment = environment_with_text_scale(1.5);
    let viewport = bounds(80.0, 48.0);
    let mut widget = editor(&document);
    install(&mut widget, viewport, &initial_environment);

    widget.wrap = false;
    assert!(
        Widget::handle_wheel_sample_with_environment(
            &mut widget,
            viewport,
            Point::new(20.0, 20.0),
            wheel(Vector2::new(0.0, 5_000.0)),
            &scaled_environment,
        )
        .is_none()
    );
    assert_eq!(
        widget.scroll_offset().y,
        0.0,
        "a stale receipt must not mutate scroll before current geometry is installed"
    );

    install(&mut widget, viewport, &scaled_environment);
    assert_eq!(
        widget.scroll_offset().y,
        0.0,
        "the current unwrapped receipt owns the scroll clamp"
    );
}

#[test]
fn wheel_scrolls_with_a_current_nondefault_environment_receipt() {
    let document = TextEditorDocument::new("a ".repeat(200)).unwrap();
    let environment = environment_with_text_scale(1.5);
    let viewport = bounds(80.0, 48.0);
    let mut widget = editor(&document);
    install(&mut widget, viewport, &environment);

    assert!(
        Widget::handle_wheel_sample_with_environment(
            &mut widget,
            viewport,
            Point::new(20.0, 20.0),
            wheel(Vector2::new(0.0, 5_000.0)),
            &environment,
        )
        .is_some()
    );
    assert!(widget.scroll_offset().y > 0.0);
    assert!(widget.scroll_offset().y < 5_000.0);
}

#[test]
fn composition_normalizes_scalar_preedit_selection_and_commit_grapheme_seams() {
    let document = TextEditorDocument::new("a").unwrap();
    let environment = ResolvedEnvironment::default();
    let viewport = bounds(100.0, 48.0);
    let mut widget = editor(&document);
    input(
        &mut widget,
        viewport,
        &environment,
        WidgetInput::FocusChanged(true),
    );
    let at_end = CompositionRange::new(1, 1, 1).unwrap();
    assert!(
        Widget::handle_composition_sample(
            &mut widget,
            CompositionSample::start(at_end, at_end).unwrap(),
        )
        .is_some()
    );
    let raw_preedit_selection = TextEditorSelection::caret(1);
    let direct_update = widget
        .snapshot
        .edit(
            TextEditorDelta::Composition(TextEditorCompositionDelta::Update {
                text: Arc::from("\u{301}"),
            }),
            raw_preedit_selection,
        )
        .expect("document transition normalizes the scalar preedit caret");
    assert_eq!(
        direct_update.selection(),
        TextEditorSelection::caret("a\u{301}".len())
    );
    assert!(
        Widget::handle_composition_sample(
            &mut widget,
            CompositionSample::update("\u{301}", CompositionRange::new(0, 0, 1).unwrap()).unwrap(),
        )
        .is_some()
    );
    assert_eq!(
        widget.selection(),
        TextEditorSelection::caret("a\u{301}".len())
    );
    assert!(
        Widget::handle_composition_sample(&mut widget, CompositionSample::commit("\u{301}"))
            .is_some()
    );
    assert_eq!(widget.text(), "a\u{301}");

    // CR creates a valid hard grapheme boundary before a following combining
    // mark. Replacing it with a base scalar joins that suffix, so the raw
    // post-commit byte position lies inside the resulting grapheme.
    let mut seam_document = TextEditorDocument::new("\r\u{301}").unwrap();
    let start = seam_document
        .snapshot()
        .edit(
            TextEditorDelta::Composition(TextEditorCompositionDelta::Start { range: 0..1 }),
            TextEditorSelection::caret(0),
        )
        .unwrap();
    seam_document.apply(&start).unwrap();
    let commit = seam_document
        .snapshot()
        .edit(
            TextEditorDelta::Composition(TextEditorCompositionDelta::Commit {
                text: Arc::from("a"),
            }),
            TextEditorSelection::caret(1),
        )
        .expect("commit seam selection is normalized instead of rejected");
    assert_eq!(
        commit.selection(),
        TextEditorSelection::caret("a\u{301}".len())
    );
    seam_document.apply(&commit).unwrap();
    assert_eq!(seam_document.text(), "a\u{301}");
}

#[test]
fn composition_trace_commits_cancels_on_focus_loss_and_uncontrolled_state_survives_sync() {
    let document = TextEditorDocument::new("ab").unwrap();
    let environment = ResolvedEnvironment::default();
    let viewport = bounds(100.0, 48.0);
    let mut widget = editor(&document);
    input(
        &mut widget,
        viewport,
        &environment,
        WidgetInput::FocusChanged(true),
    );
    let committed_range = CompositionRange::new(0, 1, 2).unwrap();
    assert!(
        Widget::handle_composition_sample(
            &mut widget,
            CompositionSample::start(committed_range, committed_range).unwrap(),
        )
        .is_some()
    );
    assert!(
        Widget::handle_composition_sample(
            &mut widget,
            CompositionSample::update("漢", CompositionRange::new(0, 1, 1).unwrap()).unwrap(),
        )
        .is_some()
    );
    assert!(Widget::retains_managed_composition(&widget));
    assert_eq!(widget.text(), "ab", "preedit stays outside committed text");
    assert!(
        Widget::handle_composition_sample(&mut widget, CompositionSample::commit("z")).is_some()
    );
    assert_eq!(widget.text(), "zb");

    let current = CompositionRange::new(0, 1, 2).unwrap();
    Widget::handle_composition_sample(
        &mut widget,
        CompositionSample::start(current, current).unwrap(),
    );
    Widget::handle_composition_sample(
        &mut widget,
        CompositionSample::update("候", CompositionRange::new(0, 1, 1).unwrap()).unwrap(),
    );
    input(
        &mut widget,
        viewport,
        &environment,
        WidgetInput::FocusChanged(false),
    );
    assert!(!Widget::retains_managed_composition(&widget));
    assert_eq!(widget.text(), "zb");

    let mut uncontrolled =
        TextEditorWidget::uncontrolled(92, "own", WidgetSizing::fixed(Vector2::new(120.0, 40.0)))
            .unwrap();
    input(
        &mut uncontrolled,
        viewport,
        &environment,
        WidgetInput::FocusChanged(true),
    );
    input(
        &mut uncontrolled,
        viewport,
        &environment,
        WidgetInput::character('!'),
    );
    let mut reprojection =
        TextEditorWidget::uncontrolled(92, "stale", WidgetSizing::fixed(Vector2::new(120.0, 40.0)))
            .unwrap();
    Widget::synchronize_from_previous(&mut reprojection, &uncontrolled);
    assert_eq!(reprojection.text(), "!own");
}
