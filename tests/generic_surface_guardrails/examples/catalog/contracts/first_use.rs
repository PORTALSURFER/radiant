use super::ExampleContract;

pub(super) const CONTRACTS: &[ExampleContract] = &[
    ("counter", &["radiant::app(", ".on_click(", ".primary()"]),
    (
        "todo_list",
        &[
            "use radiant::prelude as ui;",
            "text_input(",
            ".bind_submit(",
            "drag_handle()",
            "drop_marker(",
            "overlay_panel(",
            "scroll(",
        ],
    ),
    ("form", &["text_input(", ".bind(", "toggle(", ".on_change("]),
    (
        "volume_slider",
        &[
            "slider(",
            ".primary()",
            ".on_change(",
            "checkbox(",
            "TextAlign::Right",
        ],
    ),
    (
        "list_actions",
        &[
            "list(",
            "list_row_id(",
            "selectable(",
            "button(\"+\")",
            "button(\"-\")",
            "selected_id",
        ],
    ),
    (
        "toolbar_icons",
        &[
            "use radiant::prelude::*;",
            "SvgIconTintCache::new(",
            "PaintPrimitive::Svg(",
            "custom_widget_mapped(",
            "IconToggleButton::new(",
            "ToolMessage::Toggle(",
        ],
    ),
    (
        "svg",
        &[
            "use radiant::prelude::*;",
            "SvgIcon::from_svg(",
            "icon_button(",
            "PaintPrimitive::Svg(",
            "svg_example_paints_retained_svg_icons",
        ],
    ),
    (
        "floating_overlay",
        &[
            "use radiant::prelude::*;",
            "floating_layer(",
            ".key(\"floating-overlay-layer\")",
            "Point::new(",
            "Vector2::new(",
            "overlay_menu()",
        ],
    ),
    (
        "status_bar",
        &[
            "use radiant::prelude::*;",
            "status_bar(",
            "StatusMessage::ActionPressed",
            "StatusMessage::AutosaveChanged",
            "StatusSegments::primary(",
            "context.spawn(",
            "StatusMessage::WorkerFinished",
            ".animation(",
            ".on_frame(",
            "retained_canvas(",
            "horizontal_progress_fill_rect(",
            "StatusLineLog",
        ],
    ),
    (
        "animation_showcase",
        &[
            ".animation(",
            ".on_frame(",
            "AnimationMessage::Frame",
            "examples/animation_showcase/pulse_meter.rs",
            "examples/animation_showcase/pulse_meter/tests.rs",
        ],
    ),
    (
        "background_loading",
        &[
            ".handle_message(",
            "context.spawn_resource(",
            "ResourceCompletion",
            "LoadingMessage::Loaded",
        ],
    ),
];
