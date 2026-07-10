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
fn prelude_views_can_prepare_test_frames_directly() {
    use radiant::prelude::*;

    fn ready_view() -> impl IntoView<()> {
        text("Ready")
    }

    let frame = ready_view().view_frame_at_size_with_default_theme(Vector2::new(120.0, 40.0));

    assert!(frame.paint_plan.contains_text("Ready"));
}

#[test]
fn prelude_views_can_resolve_layout_directly() {
    use radiant::prelude::*;

    fn ready_view() -> impl IntoView<()> {
        text("Ready")
    }

    let layout = ready_view().view_layout_at_size(Vector2::new(120.0, 40.0));

    assert!(layout.rects.contains_key(&1));
}

#[test]
fn view_node_overlay_helpers_are_available_from_prelude_views() {
    use radiant::prelude::*;

    let view: View<()> = text("Owner").overlays(
        overlays()
            .floating(text("Floating"))
            .popover(text("Popover"))
            .modal(text("Modal"))
            .context_menu(text("Menu"))
            .tooltip(text("Tooltip"))
            .drag_preview(text("Drag"))
            .layer_opt(None)
            .floating_opt(None)
            .popover_opt(None)
            .modal_opt(None)
            .blocking_modal_opt(None)
            .context_menu_opt(None)
            .dismissible_context_menu_opt(None, ())
            .tooltip_opt(None)
            .drag_preview_opt(None),
    );

    let frame = view.view_frame_at_size_with_default_theme(Vector2::new(120.0, 40.0));

    assert!(frame.paint_plan.contains_text("Owner"));
    assert!(!frame.paint_plan.contains_text("Floating"));
}

#[test]
fn view_node_tooltip_opt_is_available_from_prelude_views() {
    use radiant::prelude::*;

    let with_tooltip = button("Help").message(()).tooltip_opt(Some("Helpful"));
    let without_tooltip = button("Plain")
        .message(())
        .tooltip_opt(None::<&'static str>);

    assert_eq!(
        with_tooltip
            .into_surface()
            .find_widget(1)
            .and_then(|widget| widget.widget_object().common().tooltip.as_deref()),
        Some("Helpful")
    );
    assert_eq!(
        without_tooltip
            .into_surface()
            .find_widget(1)
            .and_then(|widget| widget.widget_object().common().tooltip.as_deref()),
        None
    );
}

#[test]
fn prelude_exports_children_builder_for_optional_container_children() {
    use radiant::prelude::*;

    let view: View<()> = row(children()
        .push(text_line("Left", 20.0))
        .push_opt(Some(text_line("Middle", 20.0)))
        .push_opt(None)
        .push_if(true, || text_line("Right", 20.0))
        .push_if(false, || text_line("Hidden", 20.0)))
    .fill();

    let frame = view.view_frame_at_size_with_default_theme(Vector2::new(240.0, 40.0));

    assert!(frame.paint_plan.contains_text("Left"));
    assert!(frame.paint_plan.contains_text("Middle"));
    assert!(frame.paint_plan.contains_text("Right"));
    assert!(!frame.paint_plan.contains_text("Hidden"));
}

#[test]
fn prelude_views_can_dispatch_widget_outputs_directly() {
    use radiant::prelude::*;

    #[derive(Clone)]
    struct OutputProbe {
        common: WidgetCommon,
    }

    impl Widget for OutputProbe {
        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }

        fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
            None
        }

        fn append_paint(
            &self,
            _primitives: &mut Vec<PaintPrimitive>,
            _bounds: Rect,
            _layout: &LayoutOutput,
            _theme: &ThemeTokens,
        ) {
        }
    }

    fn action_view() -> impl IntoView<&'static str> {
        widget(DynamicWidget::new(
            OutputProbe {
                common: WidgetCommon::new(7, WidgetSizing::fixed(Vector2::new(80.0, 24.0))),
            },
            |output| output.typed_copied::<u8>().map(|_| "run"),
        ))
        .id(7)
    }

    let message = action_view().view_dispatch_widget_output(7, WidgetOutput::typed(1_u8));

    assert_eq!(message, Some("run"));
}

#[test]
fn prelude_exports_direct_custom_widget_builder() {
    use radiant::prelude::*;

    #[derive(Clone)]
    struct DirectProbe {
        common: WidgetCommon,
    }

    impl Widget for DirectProbe {
        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }

        fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
            None
        }

        fn append_paint(
            &self,
            _primitives: &mut Vec<PaintPrimitive>,
            _bounds: Rect,
            _layout: &LayoutOutput,
            _theme: &ThemeTokens,
        ) {
        }
    }

    let message = custom_widget_direct(DirectProbe {
        common: WidgetCommon::new(9, WidgetSizing::fixed(Vector2::new(80.0, 24.0))),
    })
    .id(9)
    .view_dispatch_widget_output(9, WidgetOutput::typed("ready"));

    assert_eq!(message, Some("ready"));
}

#[test]
fn prelude_views_can_dispatch_widget_inputs_directly() {
    use radiant::prelude::*;

    #[derive(Clone)]
    struct InputProbe {
        common: WidgetCommon,
    }

    impl Widget for InputProbe {
        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }

        fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
            matches!(input, WidgetInput::KeyPress(WidgetKey::Enter))
                .then(|| WidgetOutput::typed("enter"))
        }

        fn append_paint(
            &self,
            _primitives: &mut Vec<PaintPrimitive>,
            _bounds: Rect,
            _layout: &LayoutOutput,
            _theme: &ThemeTokens,
        ) {
        }
    }

    fn action_view() -> impl IntoView<()> {
        widget(InputProbe {
            common: WidgetCommon::new(7, WidgetSizing::fixed(Vector2::new(80.0, 24.0))),
        })
        .id(7)
    }

    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 24.0));
    let output = action_view().view_dispatch_widget_input(
        7,
        bounds,
        WidgetInput::KeyPress(WidgetKey::Enter),
    );

    assert_eq!(
        output.and_then(|output| output.typed_copied::<&'static str>()),
        Some("enter")
    );
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
    let mut cycle = ui::CyclicListSelectionCycle::new();

    selection.select(1, 4, ui::ListSelectionModifiers::new());
    selection.select(3, 4, ui::ListSelectionModifiers::extend());
    cycle.move_selection("search", 1, 4);

    assert_eq!(column.title, "Inbox");
    assert_eq!(column.item_count, 42);
    assert_eq!(metrics.stride(), 28.0);
    assert_eq!(ui::list_index_after_delta(1, 1, 4), Some(2));
    assert_eq!(cycle.selected_index("search", 4), Some(1));
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
    let cursor =
        radiant::gui::feedback::horizontal_value_cursor_rect(rect, 0.5, 2.0).expect("cursor rect");
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
fn svg_parse_errors_require_an_explicit_runtime_import() {
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
    let _: radiant::runtime::SvgParseError = icon_error;
}

#[test]
fn native_run_reports_require_an_explicit_runtime_import() {
    let report: radiant::runtime::RuntimeRunReport<(), radiant::runtime::NativeGenericRunError> =
        radiant::runtime::RuntimeRunReport {
            artifacts: (),
            result: Err(radiant::runtime::NativeGenericRunError::EventLoopRun(
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
fn prelude_exports_native_file_drop_callback_payloads() {
    let event = ui::NativeFileDrop::cancel(None, None);

    assert_eq!(event.phase, ui::NativeFileDropPhase::Cancel);
}

#[test]
fn advanced_apis_remain_public_through_their_owning_modules() {
    fn assert_public<T>() {}

    assert_public::<radiant::runtime::NativeFrameDiagnostics>();
    assert_public::<radiant::runtime::SurfacePaintPlan>();
    assert_public::<radiant::runtime::GpuSurfaceContent>();
    assert_public::<radiant::gui::visualization::TimelineViewport>();
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
