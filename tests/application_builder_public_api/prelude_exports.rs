use radiant::{gui::types::Rgba8, layout::LayoutOutput, prelude as ui, theme::ThemeTokens};

#[test]
fn prelude_supports_hello_world_imports() {
    use radiant::prelude::*;

    fn hello_body() -> impl IntoView<()> {
        text("Hello, world!")
    }

    let surface = hello_body().into_surface();

    assert!(surface.find_widget(1).is_some());
}

#[test]
fn prelude_exports_application_chrome_models() {
    let status = ui::StatusSegments::from_parts(ui::StatusSegmentsParts {
        left: "Ready".to_owned(),
        center: "Autosave on".to_owned(),
        right: "Idle".to_owned(),
    });
    let mut log = ui::StatusLineLog::new(2);
    let entry = ui::StatusLineEntry::from_parts(ui::StatusLineEntryParts {
        source: "worker".to_owned(),
        message: "finished".to_owned(),
    });
    let chrome = ui::ContentViewChrome::default();

    log.publish(entry.source(), entry.message());
    assert_eq!(status.left, "Ready");
    assert_eq!(status.center, "Autosave on");
    assert_eq!(status.right, "Idle");
    assert_eq!(log.latest_line(), "worker: finished");
    assert_eq!(log.latest(), "worker: finished");
    assert_eq!(entry.line(), "worker: finished");
    assert_eq!(chrome.tabs.item_column_label, "Item");
    assert_eq!(
        ui::ContentViewFooterChrome::default().item_count_label,
        "0 items"
    );
}

#[test]
fn prelude_exports_list_selection_controller() {
    let column = ui::ColumnSummary::from_parts(ui::ColumnSummaryParts {
        title: "Inbox".to_owned(),
        item_count: 42,
    });
    let metrics = ui::VirtualListStackMetrics::from_parts(ui::VirtualListStackMetricsParts {
        item_extent: 24.0,
        item_gap: 4.0,
        max_viewport_len: Some(6),
    });
    let mut selection = ui::ListSelectionController::new();

    selection.select(1, 4, ui::ListSelectionModifiers::new());
    selection.select(3, 4, ui::ListSelectionModifiers::extend());

    assert_eq!(column.title, "Inbox");
    assert_eq!(column.item_count, 42);
    assert_eq!(metrics.stride(), 28.0);
    assert_eq!(ui::list_index_after_delta(1, 1, 4), Some(2));
    assert_eq!(
        ui::virtual_list_view_start_for_scroll_offset(48.0, 24.0, 4),
        2
    );
    assert_eq!(metrics.max_viewport_len, Some(6));
    assert_eq!(selection.selected_indices(), &[1, 2, 3]);
}

#[test]
fn prelude_exports_revision_counter() {
    let mut revision = ui::RevisionCounter::default();

    assert_eq!(revision.get(), 0);
    assert_eq!(revision.bump(), 1);
    assert_eq!(revision.bump_if(false), 1);
    assert_eq!(revision.bump_if(true), 2);
}

#[test]
fn prelude_exports_custom_widget_authoring_contract() {
    use radiant::prelude::IntoView;

    #[derive(Clone)]
    struct AuthorWidget {
        common: ui::WidgetCommon,
    }

    impl ui::Widget for AuthorWidget {
        fn common(&self) -> &ui::WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut ui::WidgetCommon {
            &mut self.common
        }

        fn handle_input(
            &mut self,
            _bounds: ui::Rect,
            input: ui::WidgetInput,
        ) -> Option<ui::WidgetOutput> {
            match input {
                ui::WidgetInput::KeyPress(ui::WidgetKey::Enter) => {
                    Some(ui::WidgetOutput::custom(()))
                }
                ui::WidgetInput::PointerRelease {
                    button: ui::PointerButton::Primary,
                    ..
                } => Some(ui::WidgetOutput::custom(())),
                _ => None,
            }
        }

        fn append_paint(
            &self,
            primitives: &mut Vec<ui::PaintPrimitive>,
            bounds: ui::Rect,
            _layout: &LayoutOutput,
            _theme: &ThemeTokens,
        ) {
            primitives.push(ui::PaintPrimitive::FillRect(ui::PaintFillRect {
                widget_id: self.common.id,
                rect: bounds,
                color: Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            }));
        }
    }

    let mut common =
        ui::WidgetCommon::new(77, ui::WidgetSizing::fixed(ui::Vector2::new(120.0, 28.0)));
    common.focus = ui::FocusBehavior::Keyboard;
    common.state = ui::WidgetState {
        hovered: true,
        ..ui::WidgetState::default()
    };
    let widget = AuthorWidget { common };
    let surface: ui::View<()> = ui::widget(widget);
    let surface = surface.into_surface();

    assert!(surface.find_widget(1).is_some());
}

#[test]
fn prelude_exports_custom_widget_signature_types() {
    let rect = ui::Rect::from_min_size(ui::Point::new(0.0, 0.0), ui::Vector2::new(8.0, 4.0));
    let layout = ui::LayoutOutput::default();
    let theme = ui::ThemeTokens::default();
    let color = ui::Rgba8 {
        r: 1,
        g: 2,
        b: 3,
        a: 255,
    };
    let image = ui::ImageRgba::new(1, 1, vec![255, 255, 255, 255]).expect("valid image");
    let image_error = ui::ImageRgba::try_new(1, 1, vec![255]).expect_err("invalid image");
    let cursor = ui::horizontal_value_cursor_rect(rect, 0.5, 2.0).expect("cursor rect");
    let text_line = ui::centered_text_line(rect, 13.0, ui::TextLineInsets::horizontal(2.0), 0.0);
    let baseline = ui::centered_text_baseline(text_line, 13.0).expect("text baseline");

    assert_eq!(rect.width(), 8.0);
    assert_eq!(cursor.width(), 2.0);
    assert!(baseline > 0.0);
    assert_eq!(
        ui::top_text_line(rect, 13.0, ui::TextLineInsets::horizontal(2.0))
            .min
            .y,
        rect.min.y
    );
    assert!(layout.rects.is_empty());
    assert_eq!(theme.text_primary.a, 255);
    assert_eq!(color.g, 2);
    assert_eq!(image.width, 1);
    let _: ui::ImageRgbaError = image_error;
}

#[test]
fn prelude_exports_svg_icon_vector_painting() {
    let icon = ui::SvgIcon::from_svg(
        r#"<svg viewBox="0 0 4 4"><rect x="0" y="0" width="4" height="4"/></svg>"#,
    )
    .expect("filled SVG rect should parse");
    let icon_error = ui::SvgIcon::try_from_svg("<svg><").expect_err("invalid svg");
    let mut primitives = Vec::new();
    icon.append_paint(
        &mut primitives,
        1,
        ui::Rect::from_min_size(ui::Point::new(2.0, 3.0), ui::Vector2::new(4.0, 4.0)),
    );

    assert!(matches!(
        primitives.as_slice(),
        [ui::PaintPrimitive::Svg(_)]
    ));
    let _: ui::SvgParseError = icon_error;
}

#[test]
fn prelude_exports_native_run_report_error_boundary() {
    let report: ui::RuntimeRunReport<(), ui::NativeGenericRunError> = ui::RuntimeRunReport {
        artifacts: (),
        result: Err(ui::NativeGenericRunError::EventLoopRun(
            "stopped".to_string(),
        )),
    };

    assert_eq!(
        report
            .result
            .expect_err("report should carry typed error")
            .to_string(),
        "native event loop failed: stopped"
    );
}

#[test]
fn hello_world_example_stays_on_application_builders() {
    let source = include_str!("../../examples/hello_world.rs");

    assert!(source.contains("use radiant::prelude::*;"));
    assert!(source.contains("radiant::window(\"Radiant Hello World\")"));
    assert!(source.contains(".run(text(\"Hello, world!\"))"));
    assert!(!source.contains("NativeRunOptions"));
    assert!(!source.contains("RuntimeBridge"));
    assert!(!source.contains("SurfaceChild"));
    assert!(!source.contains("WidgetSizing"));
    assert!(!source.contains("declarative_command_runtime_bridge"));
}
