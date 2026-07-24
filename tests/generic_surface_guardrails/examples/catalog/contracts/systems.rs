use super::ExampleContract;

pub(super) const CONTRACTS: &[ExampleContract] = &[
    (
        "styling",
        &[
            ".primary()",
            ".danger()",
            ".subtle()",
            ".hoverable()",
            "toggle(",
        ],
    ),
    (
        "theme_playground",
        &[
            "ThemeTokens::dark_for_viewport_width(",
            "effective_ui_scale(",
            "resolve_widget_visual_tokens(",
            "radiant_theme_playground",
        ],
    ),
    (
        "dpi_scaling",
        &[
            "use radiant::theme::DpiScale;",
            "DpiScale::new(",
            "context.set_dpi_scale(",
            "context.set_window_logical_size(",
            "physical_to_logical(",
            "DpiScaleChoice::Two",
            "dpi_scaling_example_projects_metrics_for_selected_scale",
        ],
    ),
    ("scroll", &["scroll_column(", ".fill_height()"]),
    (
        "sizing",
        &[".size(", ".min_size(", ".preferred_size(", ".fill_width()"],
    ),
    (
        "message_routing",
        &[".handle_message", "context.emit", "context.request_repaint"],
    ),
    ("keys", &[".key(", "list_row(", ".reverse()"]),
    (
        "focus_controls",
        &[
            ".shortcuts(",
            "ShortcutResolution::action",
            ".handle_message(",
            "context.focus(",
            "text_input(",
            ".message(",
        ],
    ),
    (
        "plugin_panel",
        &[
            "grid_with_gaps(",
            "toggle(",
            "parameter_tile(",
            "WidgetProminence::Subtle",
        ],
    ),
    (
        "eq_editor",
        &[
            "Graphical EQ",
            "custom_widget_mapped(",
            "EqEditorMessage::MoveBand",
            "examples/eq_editor/widget/geometry.rs",
            "examples/eq_editor/widget/paint.rs",
            "examples/eq_editor/widget/paint/grid.rs",
            "examples/eq_editor/widget/paint/primitives.rs",
            "examples/eq_editor/widget/response.rs",
            "PaintPrimitive::StrokePolyline",
            "prefers_pointer_move_paint_only",
            "append_runtime_overlay_paint",
            "without_dsp",
        ],
    ),
    (
        "custom_widget",
        &[
            "use radiant::prelude::*;",
            "fn append_paint(",
            "impl Widget for StatusChip",
            "WidgetOutput::custom(",
            "dispatch_input(",
        ],
    ),
    (
        "render_canvas",
        &[
            "render_canvas_input(",
            "RenderCanvasContent::RgbaAtlas",
            "OnceLock<Arc<ImageRgba>>",
            "render_canvas_example_lowers_to_retained_gpu_primitive",
            "render_canvas_example_routes_input_to_state",
        ],
    ),
    (
        "custom_shader_surface",
        &[
            "render_canvas(",
            "GpuShaderSurfaceDescriptor::new",
            "RenderCanvasContent::CustomShader",
            "custom_shader_surface_example_lowers_to_render_canvas_primitive",
            "custom_shader_surface_example_descriptor_is_valid",
        ],
    ),
    (
        "render_canvas_stack_overlay",
        &[
            "render_canvas(",
            "custom_widget_mapped(",
            ".animated_transient_overlay_at(",
            "SurfacePaintPlan",
            "SelectionOverlay",
            "examples/render_canvas_stack_overlay/tests.rs",
        ],
    ),
];
