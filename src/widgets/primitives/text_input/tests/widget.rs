use crate::application::{ApplicationEnvironment, LocaleId, TextScale, WritingDirection};
use crate::gui::types::{Point, Rect, Vector2};
use crate::runtime::{PaintPrimitive, TextClipboardOperation};
use crate::runtime::{ResolvedEnvironment, WindowEnvironment};
use crate::theme::ThemeTokens;
use crate::widgets::interaction::{
    CompositionRange, CompositionSample, PointerButton, TextEditCommand, TextInputMessage,
    TextInputRevision, WidgetInput, WidgetKey,
};
use crate::widgets::{
    SemanticAction, TextAlign, TextEditBoundary, TextEditKind, TextInputEditEvent, TextPrivacy,
    TextSecretPolicy, Widget, WidgetSemantics,
};
use std::sync::Arc;

use super::super::NativeCaretAffinity;
use super::super::{TextInputChrome, TextInputWidget, WidgetSizing};

#[test]
fn text_edit_authority_advances_for_edits_and_focus_loss_but_not_hover() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(100.0, 28.0));
    let mut input =
        TextInputWidget::new(7, "draft", WidgetSizing::fixed(Vector2::new(100.0, 28.0)));
    input.common.state.focused = true;
    let before_edit = input
        .capture_text_edit_authority()
        .expect("live text input issues authority");

    let _ = input.handle_input(bounds, WidgetInput::character('!'));
    assert!(!input.is_current_text_edit_authority(&before_edit));
    let before_hover = input
        .capture_text_edit_authority()
        .expect("live text input issues authority");

    let _ = input.handle_input(bounds, WidgetInput::pointer_move(Point::new(200.0, 0.0)));
    assert!(input.is_current_text_edit_authority(&before_hover));

    let _ = input.handle_input(bounds, WidgetInput::FocusChanged(false));
    assert!(!input.is_current_text_edit_authority(&before_hover));
}

#[test]
fn compatible_reprojection_shares_text_edit_authority_and_newer_revision_cancels_it() {
    let sizing = WidgetSizing::fixed(Vector2::new(100.0, 28.0));
    let mut previous = TextInputWidget::new(7, "draft", sizing);
    previous.props.revision = Some(TextInputRevision::new(3));
    let authority = previous
        .capture_text_edit_authority()
        .expect("live text input issues authority");

    let mut compatible = TextInputWidget::new(7, "draft", sizing);
    compatible.props.revision = Some(TextInputRevision::new(3));
    compatible.synchronize_from_previous(&previous);
    assert!(compatible.is_current_text_edit_authority(&authority));

    let mut newer = TextInputWidget::new(7, "saved", sizing);
    newer.props.revision = Some(TextInputRevision::new(4));
    newer.synchronize_from_previous(&previous);
    assert!(!previous.is_current_text_edit_authority(&authority));
}

#[test]
fn editing_policy_reprojection_revokes_text_edit_authority() {
    let sizing = WidgetSizing::fixed(Vector2::new(100.0, 28.0));
    let mut previous = TextInputWidget::new(7, "draft", sizing);
    previous.props.character_limit = Some(5);
    let authority = previous
        .capture_text_edit_authority()
        .expect("live text input issues authority");

    let mut compatible = TextInputWidget::new(7, "draft", sizing);
    compatible.props.character_limit = Some(5);
    compatible.synchronize_from_previous(&previous);
    assert!(compatible.is_current_text_edit_authority(&authority));

    let mut limited = TextInputWidget::new(7, "draft", sizing);
    limited.props.character_limit = Some(3);
    limited.synchronize_from_previous(&previous);
    assert!(!previous.is_current_text_edit_authority(&authority));

    let submit_previous = TextInputWidget::new(7, "draft", sizing);
    let submit_authority = submit_previous
        .capture_text_edit_authority()
        .expect("live text input issues authority");
    let mut submit_changed = TextInputWidget::new(7, "draft", sizing);
    submit_changed.props.submit_on_enter = false;
    submit_changed.synchronize_from_previous(&submit_previous);
    assert!(!submit_previous.is_current_text_edit_authority(&submit_authority));
}

#[test]
fn secret_text_input_masks_paint_and_restricts_semantics() {
    let secret = "secret-e\u{301}👩‍❤️‍💋‍👩";
    let mut input = TextInputWidget::new(7, secret, WidgetSizing::fixed(Vector2::new(160.0, 28.0)))
        .with_privacy(TextPrivacy::Secret(TextSecretPolicy::new()));
    input.props.completion_suffix = Some("private suffix".into());
    input.state.selection_anchor = 0;
    input.state.caret = input.state.char_len();
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(160.0, 28.0));
    let mut primitives = Vec::new();
    input.append_paint(
        &mut primitives,
        bounds,
        &crate::layout::LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    let paint = primitives
        .into_iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::TextInput(paint) => Some(paint),
            _ => None,
        });
    let paint = paint.expect("text input paint is present");
    assert_eq!(paint.state.value, "•••••••••");
    assert_eq!(paint.completion_suffix, None);
    assert!(!format!("{paint:?}").contains(secret));
    assert_eq!(input.automation_value_text(), None);
    assert_eq!(
        input.automation_metadata().get("text.privacy"),
        Some(&"secret".to_owned())
    );
    assert!(!crate::widgets::WidgetSemanticActions::supports(
        &input,
        &SemanticAction::SetText("updated".into())
    ));

    let exposed = input.with_privacy(TextPrivacy::Secret(
        TextSecretPolicy::new().allow_automation(),
    ));
    assert_eq!(exposed.automation_value_text().as_deref(), Some(secret));
}

#[test]
fn secret_text_input_maps_combining_and_zwj_pointer_boundaries() {
    let text = "e\u{301}👩‍❤️‍💋‍👩";
    let mut input = TextInputWidget::new(7, text, WidgetSizing::fixed(Vector2::new(160.0, 28.0)))
        .with_privacy(TextPrivacy::Secret(TextSecretPolicy::new()));
    input.state.caret = 1;
    input.state.selection_anchor = 1;
    assert_eq!(input.display_state().caret, 0);
    assert!(input.set_native_pointer_display_caret(1, NativeCaretAffinity::Downstream));
    assert_eq!(
        input.take_native_pointer_caret().map(|(caret, _)| caret),
        Some(2)
    );
    assert!(input.set_native_pointer_display_caret(2, NativeCaretAffinity::Downstream));
    assert_eq!(
        input.take_native_pointer_caret().map(|(caret, _)| caret),
        Some(text.chars().count())
    );
}

#[test]
fn text_clipboard_receipts_require_current_exact_state_policy_and_owner() {
    let sizing = WidgetSizing::fixed(Vector2::new(160.0, 28.0));
    let mut input = TextInputWidget::new(7, "secret", sizing);
    input.common.state.focused = true;
    input.state.selection_anchor = 0;
    input.state.caret = input.state.char_len();
    let receipt = Widget::text_clipboard_receipt(&input, TextClipboardOperation::Copy)
        .expect("public selected text is copyable");
    assert!(Widget::accepts_text_clipboard_receipt(&input, &receipt));

    input.state.caret = 1;
    assert!(!Widget::accepts_text_clipboard_receipt(&input, &receipt));

    input.state.caret = input.state.char_len();

    let mut foreign = TextInputWidget::new(7, "secret", sizing);
    foreign.common.state.focused = true;
    foreign.state.selection_anchor = 0;
    foreign.state.caret = foreign.state.char_len();
    let foreign_receipt = Widget::text_clipboard_receipt(&foreign, TextClipboardOperation::Copy)
        .expect("foreign public selected text is copyable");
    assert!(!Widget::accepts_text_clipboard_receipt(
        &input,
        &foreign_receipt
    ));

    let receipt = Widget::text_clipboard_receipt(&input, TextClipboardOperation::Copy)
        .expect("current selected text is copyable");
    let mut limited = TextInputWidget::new(7, "secret", sizing);
    limited.props.character_limit = Some(3);
    limited.synchronize_from_previous(&input);
    assert!(!Widget::accepts_text_clipboard_receipt(&input, &receipt));

    let mut secret = TextInputWidget::new(7, "secret", sizing)
        .with_privacy(TextPrivacy::Secret(TextSecretPolicy::new()));
    secret.common.state.focused = true;
    secret.state.selection_anchor = 0;
    secret.state.caret = secret.state.char_len();
    assert!(Widget::text_clipboard_receipt(&secret, TextClipboardOperation::Copy).is_none());

    let copy_allowed =
        secret.with_privacy(TextPrivacy::Secret(TextSecretPolicy::new().allow_copy()));
    assert!(Widget::text_clipboard_receipt(&copy_allowed, TextClipboardOperation::Copy).is_some());
}

#[test]
fn text_input_debug_redacts_active_secret_composition_and_adornments() {
    let original = "original-secret";
    let placeholder = "placeholder-secret";
    let suffix = "suffix-secret";
    let preedit = "preedit-secret";
    let mut input =
        TextInputWidget::new(7, original, WidgetSizing::fixed(Vector2::new(160.0, 28.0)))
            .with_privacy(TextPrivacy::Secret(TextSecretPolicy::new()));
    input.props.placeholder = Some(placeholder.into());
    input.props.completion_suffix = Some(suffix.into());
    input.common.state.focused = true;
    let range = CompositionRange::new(0, input.state.char_len(), input.state.char_len())
        .expect("full composition range is valid");
    assert!(
        Widget::handle_composition_sample(
            &mut input,
            CompositionSample::start(range, range).expect("composition start is valid"),
        )
        .is_none()
    );
    assert!(
        Widget::handle_composition_sample(
            &mut input,
            CompositionSample::update(
                preedit,
                CompositionRange::new(0, preedit.chars().count(), preedit.chars().count())
                    .expect("preedit range is valid"),
            )
            .expect("composition update is valid"),
        )
        .is_none()
    );
    let debug = format!("{input:?}");
    for sentinel in [original, placeholder, suffix, preedit] {
        assert!(!debug.contains(sentinel), "Debug leaked {sentinel:?}");
    }
    assert!(debug.contains("TextInputComposition"));
}

#[test]
fn text_input_grouping_is_opt_in_and_keeps_legacy_messages() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(160.0, 28.0));
    let mut legacy = TextInputWidget::new(7, "", WidgetSizing::fixed(bounds.size()));
    legacy.common.state.focused = true;
    assert_eq!(
        legacy.handle_input(bounds, WidgetInput::character('a')),
        Some(TextInputMessage::Changed { value: "a".into() })
    );

    let mut grouped =
        TextInputWidget::new(7, "", WidgetSizing::fixed(bounds.size())).with_edit_events();
    grouped.common.state.focused = true;
    let first = Widget::handle_input(&mut grouped, bounds, WidgetInput::character('a'))
        .and_then(|output| output.typed_cloned::<TextInputEditEvent>())
        .expect("grouped typing emits an event");
    assert_eq!(
        first.legacy_message,
        Some(TextInputMessage::Changed { value: "a".into() })
    );
    assert_eq!(
        first.grouping.current.map(|event| event.kind),
        Some(TextEditKind::Typing)
    );
    assert_eq!(
        first.grouping.current.map(|event| event.phase),
        Some(crate::widgets::EditPhase::Begin)
    );
    let second = Widget::handle_input(&mut grouped, bounds, WidgetInput::character('b'))
        .and_then(|output| output.typed_cloned::<TextInputEditEvent>())
        .expect("continued typing emits an event");
    assert_eq!(
        second.grouping.current.map(|event| event.phase),
        Some(crate::widgets::EditPhase::Update)
    );
    assert_eq!(
        first.grouping.current.map(|event| event.transaction),
        second.grouping.current.map(|event| event.transaction)
    );
}

#[test]
fn grouped_composition_cancel_and_compatible_reprojection_preserve_boundaries() {
    let sizing = WidgetSizing::fixed(Vector2::new(160.0, 28.0));
    let mut previous = TextInputWidget::new(7, "draft", sizing).with_edit_events();
    previous.common.state.focused = true;
    let range = CompositionRange::new(0, 5, 5).expect("composition range is valid");
    let started = Widget::handle_composition_sample(
        &mut previous,
        CompositionSample::start(range, range).expect("composition start is valid"),
    )
    .and_then(|output| output.typed_cloned::<TextInputEditEvent>())
    .expect("composition start emits a grouping event");
    assert_eq!(started.legacy_message, None);
    assert_eq!(
        started.grouping.current.map(|event| event.kind),
        Some(TextEditKind::Composition)
    );

    let mut successor = TextInputWidget::new(7, "draft", sizing).with_edit_events();
    successor.common.state.focused = true;
    successor.synchronize_from_previous(&previous);
    let canceled = Widget::handle_composition_sample(&mut successor, CompositionSample::cancel())
        .and_then(|output| output.typed_cloned::<TextInputEditEvent>())
        .expect("composition cancel emits a boundary event");
    assert_eq!(canceled.legacy_message, None);
    assert_eq!(
        canceled
            .grouping
            .current
            .map(|event| (event.phase, event.boundary)),
        Some((
            crate::widgets::EditPhase::Cancel,
            Some(TextEditBoundary::Composition)
        ))
    );
}

#[test]
fn generic_pointer_caret_uses_resolved_alignment_and_environment_scale() {
    #[derive(Clone, Copy)]
    enum Placement {
        Left,
        Center,
        Right,
    }

    struct AlignmentCase {
        name: &'static str,
        align: TextAlign,
        ltr: Placement,
        rtl: Placement,
    }

    // Keep this policy table independent from TextAlign::resolve so this test
    // catches a regression in logical-to-physical alignment resolution too.
    let alignments = [
        AlignmentCase {
            name: "left",
            align: TextAlign::Left,
            ltr: Placement::Left,
            rtl: Placement::Left,
        },
        AlignmentCase {
            name: "start",
            align: TextAlign::Start,
            ltr: Placement::Left,
            rtl: Placement::Right,
        },
        AlignmentCase {
            name: "center",
            align: TextAlign::Center,
            ltr: Placement::Center,
            rtl: Placement::Center,
        },
        AlignmentCase {
            name: "right",
            align: TextAlign::Right,
            ltr: Placement::Right,
            rtl: Placement::Right,
        },
        AlignmentCase {
            name: "end",
            align: TextAlign::End,
            ltr: Placement::Right,
            rtl: Placement::Left,
        },
    ];

    fn environment(scale: f32, direction: WritingDirection) -> ResolvedEnvironment {
        ResolvedEnvironment::from_snapshots(
            WindowEnvironment::default(),
            Arc::new(
                ApplicationEnvironment::new(LocaleId::english())
                    .with_text_scale(TextScale::new(scale).expect("valid scale"))
                    .with_writing_direction(direction),
            ),
        )
    }

    for scale in [1.0_f32, 2.0] {
        for direction in [WritingDirection::Ltr, WritingDirection::Rtl] {
            for alignment in &alignments {
                for (field_name, text, field_width) in
                    [("wide", "abcd", 240.0), ("overflow", "abcdefghij", 80.0)]
                {
                    let bounds =
                        Rect::from_min_size(Point::default(), Vector2::new(field_width, 24.0));
                    let sizing = WidgetSizing::fixed(Vector2::new(field_width, 24.0));
                    let font_size = 13.0 * scale;
                    let char_width = (font_size * 0.58).max(1.0);
                    let inset = 8.0 * scale;
                    let text_rect_min = inset;
                    let content_width = (field_width - inset - text_rect_min).max(0.0);
                    let text_width = text.chars().count() as f32 * char_width;
                    let slack = (content_width - text_width).max(0.0);
                    let placement = match direction {
                        WritingDirection::Ltr => alignment.ltr,
                        WritingDirection::Rtl => alignment.rtl,
                    };
                    let alignment_offset = match placement {
                        Placement::Left => 0.0,
                        Placement::Center => slack * 0.5,
                        Placement::Right => slack,
                    };
                    let expected_origin = text_rect_min + alignment_offset;

                    if field_name == "overflow" {
                        assert_eq!(slack, 0.0, "{field_name} {} scale {scale}", alignment.name);
                    }

                    let mut input =
                        TextInputWidget::new(7, text, sizing).with_align(alignment.align);
                    let env = environment(scale, direction);
                    let _ = Widget::handle_input_with_environment(
                        &mut input,
                        bounds,
                        WidgetInput::primary_press(Point::new(expected_origin, 12.0)),
                        &env,
                    );
                    assert_eq!(
                        input.state.caret, 0,
                        "press {field_name} {} {direction:?} scale {scale}",
                        alignment.name
                    );

                    let _ = Widget::handle_input_with_environment(
                        &mut input,
                        bounds,
                        WidgetInput::pointer_move(Point::new(
                            expected_origin + 2.0 * char_width,
                            12.0,
                        )),
                        &env,
                    );
                    assert_eq!(
                        input.state.caret, 2,
                        "drag {field_name} {} {direction:?} scale {scale}",
                        alignment.name
                    );
                    assert_eq!(input.state.selection_range(), (0, 2));
                    assert_eq!(input.selected_text().as_deref(), Some("ab"));
                }
            }
        }
    }
}

#[test]
fn native_pointer_affinity_resets_for_keyboard_input() {
    let mut input = TextInputWidget::new(
        7,
        "ab",
        WidgetSizing::new(Vector2::new(100.0, 28.0), Vector2::new(160.0, 28.0)),
    );
    input.set_native_pointer_caret(1, NativeCaretAffinity::Upstream);
    assert_eq!(input.native_caret_affinity, NativeCaretAffinity::Upstream);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 28.0));
    let _ = input.handle_input(bounds, WidgetInput::FocusChanged(true));
    assert_eq!(input.native_caret_affinity, NativeCaretAffinity::Downstream);
}

#[test]
fn text_input_editing_emits_changed_and_submitted_messages() {
    let mut input = TextInputWidget::new(
        7,
        "ab",
        WidgetSizing::new(Vector2::new(100.0, 28.0), Vector2::new(160.0, 28.0)),
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 28.0));
    let _ = input.handle_input(bounds, WidgetInput::FocusChanged(true));
    input.state.caret = 1;
    input.state.selection_anchor = 1;

    assert_eq!(
        input.handle_input(bounds, WidgetInput::character('z')),
        Some(TextInputMessage::Changed {
            value: String::from("azb"),
        })
    );
    assert_eq!(input.state.caret, 2);

    assert_eq!(
        input.handle_input(bounds, WidgetInput::key_press(WidgetKey::Backspace)),
        Some(TextInputMessage::Changed {
            value: String::from("ab"),
        })
    );

    assert_eq!(
        input.handle_input(bounds, WidgetInput::key_press(WidgetKey::Enter)),
        Some(TextInputMessage::Submitted {
            value: String::from("ab"),
        })
    );
    assert_eq!(
        input.handle_input(bounds, WidgetInput::key_press(WidgetKey::Tab)),
        Some(TextInputMessage::CompletionRequested {
            value: String::from("ab"),
        })
    );
}

#[test]
fn text_input_selection_replaces_cuts_and_pastes_text() {
    let mut input = TextInputWidget::new(
        7,
        "alpha beta",
        WidgetSizing::new(Vector2::new(100.0, 28.0), Vector2::new(160.0, 28.0)),
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 28.0));
    let _ = input.handle_input(bounds, WidgetInput::FocusChanged(true));

    let _ = input.handle_input(
        bounds,
        WidgetInput::text_edit(TextEditCommand::MoveHome {
            extend_selection: false,
        }),
    );
    for _ in 0..5 {
        let _ = input.handle_input(
            bounds,
            WidgetInput::text_edit(TextEditCommand::MoveRight {
                extend_selection: true,
            }),
        );
    }

    assert_eq!(input.selected_text().as_deref(), Some("alpha"));
    assert_eq!(
        input.handle_input(
            bounds,
            WidgetInput::text_edit(TextEditCommand::InsertText(String::from("one\ntwo"))),
        ),
        Some(TextInputMessage::Changed {
            value: String::from("onetwo beta"),
        })
    );

    let _ = input.handle_input(bounds, WidgetInput::text_edit(TextEditCommand::SelectAll));
    assert_eq!(input.selected_text().as_deref(), Some("onetwo beta"));
    assert_eq!(
        input.handle_input(
            bounds,
            WidgetInput::text_edit(TextEditCommand::CutSelection)
        ),
        Some(TextInputMessage::Changed {
            value: String::new(),
        })
    );
}

#[test]
fn text_input_pointer_drag_extends_selection_including_caret_character() {
    let mut input = TextInputWidget::new(
        7,
        "abcdef",
        WidgetSizing::new(Vector2::new(100.0, 42.0), Vector2::new(180.0, 42.0)),
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 42.0));

    assert_eq!(
        input.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(26.0, 20.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        ),
        None
    );
    assert_eq!(input.state.caret, 1);
    assert_eq!(
        input.handle_input(bounds, WidgetInput::pointer_move(Point::new(43.0, 20.0)),),
        None
    );
    assert_eq!(input.state.caret, 3);
    assert_eq!(input.selected_text().as_deref(), Some("bc"));
    assert_eq!(
        input.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(43.0, 20.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        ),
        None
    );
    assert!(!input.common.state.pressed);
}

#[test]
fn text_input_double_click_selects_word_under_pointer() {
    let mut input = TextInputWidget::new(
        7,
        "alpha  beta_gamma.日文",
        WidgetSizing::new(Vector2::new(160.0, 42.0), Vector2::new(240.0, 42.0)),
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 42.0));

    assert_eq!(
        input.handle_input(
            bounds,
            WidgetInput::PointerDoubleClick {
                position: Point::new(82.0, 20.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        ),
        None
    );

    assert!(input.common.state.focused);
    assert_eq!(input.selected_text().as_deref(), Some("beta_gamma"));
}

#[test]
fn text_input_double_click_selects_complete_unicode_word_graphemes() {
    let mut input = TextInputWidget::new(
        7,
        "e\u{301} क्\u{200d}ष \u{10400}\u{301}",
        WidgetSizing::new(Vector2::new(180.0, 42.0), Vector2::new(260.0, 42.0)),
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 42.0));

    let double_click = |input: &mut TextInputWidget, position| {
        input.handle_input(
            bounds,
            WidgetInput::PointerDoubleClick {
                position,
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        )
    };

    assert_eq!(double_click(&mut input, Point::new(20.0, 20.0)), None);
    assert_eq!(input.selected_text().as_deref(), Some("e\u{301}"));

    assert_eq!(double_click(&mut input, Point::new(47.0, 20.0)), None);
    assert_eq!(input.selected_text().as_deref(), Some("क्\u{200d}ष"));

    assert_eq!(double_click(&mut input, Point::new(90.0, 20.0)), None);
    assert_eq!(input.selected_text().as_deref(), Some("\u{10400}\u{301}"));
}

#[test]
fn text_input_selection_range_clamps_stale_public_state() {
    let mut input = TextInputWidget::new(
        7,
        "abc",
        WidgetSizing::new(Vector2::new(100.0, 28.0), Vector2::new(160.0, 28.0)),
    );
    input.state.selection_anchor = usize::MAX;
    input.state.caret = 1;

    assert_eq!(input.selection_range(), (1, 3));
    assert_eq!(input.selected_text().as_deref(), Some("bc"));

    input.state.selection_anchor = 9;
    input.state.caret = 7;

    assert_eq!(input.selection_range(), (3, 3));
    assert_eq!(input.selected_text(), None);
}

#[test]
fn underline_text_input_paints_baseline_without_box_chrome() {
    let mut input = TextInputWidget::new(
        7,
        "",
        WidgetSizing::new(Vector2::new(100.0, 18.0), Vector2::new(160.0, 18.0)),
    );
    input.props.chrome = TextInputChrome::Underline;
    input.props.placeholder = Some("add tag".into());
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 18.0));
    let mut primitives = Vec::new();

    input.append_paint(
        &mut primitives,
        bounds,
        &crate::layout::LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        !primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::FillRect(_)))
    );
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        PaintPrimitive::StrokeRect(stroke) if (stroke.rect.height() - 1.0).abs() < 0.01
    )));
    assert!(
        primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::TextInput(_)))
    );
}

#[test]
fn text_input_paint_carries_inline_completion_suffix() {
    let mut input = TextInputWidget::new(
        7,
        "ki",
        WidgetSizing::new(Vector2::new(100.0, 18.0), Vector2::new(160.0, 18.0)),
    );
    input.props.completion_suffix = Some("ck".into());
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 18.0));
    let mut primitives = Vec::new();

    input.append_paint(
        &mut primitives,
        bounds,
        &crate::layout::LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        PaintPrimitive::TextInput(text_input)
            if text_input.completion_suffix.as_deref() == Some("ck")
    )));
}

#[test]
fn newer_text_input_revision_applies_projected_value_and_selection() {
    let sizing = WidgetSizing::new(Vector2::new(100.0, 28.0), Vector2::new(160.0, 28.0));
    let mut previous = TextInputWidget::new(7, "draft", sizing);
    previous.props.revision = Some(TextInputRevision::new(3));
    previous.state.caret = 2;
    previous.state.selection_anchor = 2;

    let mut current = TextInputWidget::new(7, "saved", sizing);
    current.props.revision = Some(TextInputRevision::new(4));
    current.state.selection_anchor = 1;
    current.state.caret = 3;

    current.synchronize_from_previous(&previous);

    assert_eq!(current.state.value, "saved");
    assert_eq!(current.state.selection_anchor, 1);
    assert_eq!(current.state.caret, 3);
}

#[test]
fn newer_equal_value_text_input_revision_applies_projected_selection() {
    let sizing = WidgetSizing::new(Vector2::new(100.0, 28.0), Vector2::new(160.0, 28.0));
    let mut previous = TextInputWidget::new(7, "same", sizing);
    previous.props.revision = Some(TextInputRevision::new(3));
    previous.state.caret = 1;
    previous.state.selection_anchor = 1;

    let mut current = TextInputWidget::new(7, "same", sizing);
    current.props.revision = Some(TextInputRevision::new(4));
    current.state.selection_anchor = 0;
    current.state.caret = 2;

    current.synchronize_from_previous(&previous);

    assert_eq!(current.state.value, "same");
    assert_eq!(current.state.selection_anchor, 0);
    assert_eq!(current.state.caret, 2);
}

#[test]
fn equal_or_older_text_input_revision_preserves_retained_editing_state() {
    let sizing = WidgetSizing::new(Vector2::new(100.0, 28.0), Vector2::new(160.0, 28.0));
    for current_revision in [3, 2] {
        let mut previous = TextInputWidget::new(7, "draft", sizing);
        previous.props.revision = Some(TextInputRevision::new(3));
        previous.state.caret = 2;
        previous.state.selection_anchor = 1;

        let mut current = TextInputWidget::new(7, "saved", sizing);
        current.props.revision = Some(TextInputRevision::new(current_revision));

        current.synchronize_from_previous(&previous);

        assert_eq!(current.state.value, "draft");
        assert_eq!(current.state.selection_anchor, 1);
        assert_eq!(current.state.caret, 2);
    }
}

#[test]
fn text_input_revision_mode_changes_are_explicit_reset_boundaries() {
    let sizing = WidgetSizing::new(Vector2::new(100.0, 28.0), Vector2::new(160.0, 28.0));

    let mut previous_revisioned = TextInputWidget::new(7, "draft", sizing);
    previous_revisioned.props.revision = Some(TextInputRevision::new(3));
    previous_revisioned.state.caret = 2;
    previous_revisioned.state.selection_anchor = 1;
    let mut current_unrevisioned = TextInputWidget::new(7, "saved", sizing);
    current_unrevisioned.synchronize_from_previous(&previous_revisioned);
    assert_eq!(current_unrevisioned.state.value, "saved");
    assert_eq!(current_unrevisioned.state.selection_range(), (5, 5));

    let previous_unrevisioned = TextInputWidget::new(7, "draft", sizing);
    let mut current_revisioned = TextInputWidget::new(7, "saved", sizing);
    current_revisioned.props.revision = Some(TextInputRevision::new(1));
    current_revisioned.synchronize_from_previous(&previous_unrevisioned);
    assert_eq!(current_revisioned.state.value, "saved");
}

#[test]
fn text_input_revision_requires_matching_identity_and_unrevisioned_inputs_keep_legacy_sync() {
    let sizing = WidgetSizing::new(Vector2::new(100.0, 28.0), Vector2::new(160.0, 28.0));
    let mut previous = TextInputWidget::new(7, "same", sizing);
    previous.state.caret = 1;
    previous.state.selection_anchor = 0;

    let mut current = TextInputWidget::new(7, "same", sizing);
    current.synchronize_from_previous(&previous);
    assert_eq!(current.state.selection_anchor, 0);
    assert_eq!(current.state.caret, 1);

    let mut different_identity = TextInputWidget::new(8, "same", sizing);
    different_identity.synchronize_from_previous(&previous);
    assert_eq!(different_identity.state.selection_range(), (4, 4));
}
