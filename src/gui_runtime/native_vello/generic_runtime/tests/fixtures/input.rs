use super::super::*;
use crate::application::{IntoView, ViewNode};
use crate::gui::list::{
    TreeGuideRow, TreeGuideStyle, VirtualListWindow, VirtualListWindowChange,
    VirtualListWindowRequest, resolve_virtual_list_window,
};
use crate::layout::{SizeModeCross, SizeModeMain};
use crate::runtime::{
    NativeFrameDiagnostics, RepaintScope, RuntimeFrameDiagnosticsHost, RuntimeHostCapabilities,
};
use std::rc::Rc;

#[derive(Default)]
pub(in super::super) struct CanvasBridge {
    pub(in super::super) text: String,
}

#[derive(Default)]
pub(in super::super) struct ScrollbarBridge {
    pub(in super::super) offset: f32,
}

#[derive(Default)]
pub(in super::super) struct WheelRefreshBridge {
    pub(in super::super) wheel_count: usize,
    pub(in super::super) project_count: usize,
    pub(in super::super) repaint_scope: Option<RepaintScope>,
}

#[derive(Default)]
pub(in super::super) struct ScrollRefreshBridge {
    pub(in super::super) scroll_count: usize,
    pub(in super::super) project_count: usize,
}

pub(in super::super) struct AppVirtualListBridge {
    pub(in super::super) window: VirtualListWindow,
    pub(in super::super) scroll_count: usize,
    pub(in super::super) project_count: usize,
    retain_materialized_window: bool,
    include_coalescing_gpu_surface: bool,
}

/// Long virtualized list whose rows retain the normal tree-row hover chrome.
///
/// This fixture is intentionally app-owned: a wheel crossing the materialized
/// window must project a new row set before the native runner re-evaluates the
/// pointer hover target.
pub(in super::super) struct HoverVirtualListBridge {
    pub(in super::super) window: VirtualListWindow,
}

impl Default for HoverVirtualListBridge {
    fn default() -> Self {
        Self {
            window: resolve_virtual_list_window(VirtualListWindowRequest {
                total_items: 100,
                viewport_len: 4,
                requested_start: 0,
                overscan: 1,
                focused_index: None,
                previous_start: None,
                guard_band: 0,
            }),
        }
    }
}

impl Default for AppVirtualListBridge {
    fn default() -> Self {
        Self {
            window: resolve_virtual_list_window(VirtualListWindowRequest {
                total_items: 100,
                viewport_len: 4,
                requested_start: 0,
                overscan: 1,
                focused_index: None,
                previous_start: None,
                guard_band: 0,
            }),
            scroll_count: 0,
            project_count: 0,
            retain_materialized_window: false,
            include_coalescing_gpu_surface: false,
        }
    }
}

impl AppVirtualListBridge {
    pub(in super::super) fn retaining_materialized_window() -> Self {
        Self {
            retain_materialized_window: true,
            ..Self::default()
        }
    }

    pub(in super::super) fn with_coalescing_gpu_surface(mut self) -> Self {
        self.include_coalescing_gpu_surface = true;
        self
    }
}

impl RuntimeBridge<String> for CanvasBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<String>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::canvas_mapped(
            21,
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            |message| match message {
                CanvasMessage::Input {
                    input: WidgetInput::Character { character, .. },
                } => character.to_string(),
                CanvasMessage::Input {
                    input: WidgetInput::Wheel { .. },
                } => String::from("wheel"),
                CanvasMessage::Input {
                    input: WidgetInput::PointerDoubleClick { .. },
                } => String::from("double"),
                _ => String::new(),
            },
        )))
    }

    fn update(&mut self, message: String) -> Command<String> {
        self.text.push_str(&message);
        Command::none()
    }
}

impl RuntimeBridge<f32> for ScrollbarBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<f32>> {
        let mut scrollbar = ScrollbarWidget::new(
            41,
            ScrollbarAxis::Horizontal,
            WidgetSizing::fixed(Vector2::new(220.0, 14.0)),
        );
        scrollbar.props.viewport_fraction = 0.25;
        scrollbar.state.offset_fraction = self.offset;
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            scrollbar,
            WidgetMessageMapper::scrollbar(|message| match message {
                ScrollbarMessage::OffsetChanged { offset_fraction } => offset_fraction,
            }),
        )))
    }

    fn reduce_message(&mut self, message: f32) {
        self.offset = message;
    }
}

impl RuntimeBridge<String> for WheelRefreshBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<String>> {
        self.project_count += 1;
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::canvas_mapped(
            51,
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            |message| match message {
                CanvasMessage::Input {
                    input: WidgetInput::Wheel { .. },
                } => String::from("wheel"),
                _ => String::new(),
            },
        )))
    }

    fn update(&mut self, message: String) -> Command<String> {
        if message == "wheel" {
            self.wheel_count += 1;
        }
        self.repaint_scope
            .map_or_else(Command::none, Command::repaint)
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, String> {
        RuntimeHostCapabilities::new().with_frame_diagnostics()
    }
}

impl RuntimeFrameDiagnosticsHost for WheelRefreshBridge {
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {}
}

impl RuntimeBridge<String> for ScrollRefreshBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<String>> {
        self.project_count += 1;
        crate::runtime::test_arc_surface(UiSurface::new(
            SurfaceNode::scroll_area(
                61,
                SurfaceNode::column(
                    62,
                    0.0,
                    (0..10)
                        .map(|index| {
                            SurfaceChild::new(
                                SlotParams {
                                    size_main: SizeModeMain::Fixed(20.0),
                                    size_cross: SizeModeCross::Fill,
                                    constraints: crate::layout::Constraints::unconstrained(),
                                    margin: Default::default(),
                                    align_cross_override: None,
                                    allow_fixed_compress: false,
                                },
                                SurfaceNode::text(
                                    70 + index,
                                    format!("Row {index}"),
                                    WidgetSizing::fixed(Vector2::new(120.0, 20.0)),
                                ),
                            )
                        })
                        .collect(),
                ),
            )
            .with_scroll_message_local(Rc::new(|_| Some(String::from("scroll")))),
        ))
    }

    fn reduce_message(&mut self, message: String) {
        if message == "scroll" {
            self.scroll_count += 1;
        }
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, String> {
        RuntimeHostCapabilities::new().with_frame_diagnostics()
    }
}

impl RuntimeFrameDiagnosticsHost for ScrollRefreshBridge {
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {}
}

impl RuntimeBridge<VirtualListWindowChange> for AppVirtualListBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<VirtualListWindowChange>> {
        self.project_count += 1;
        let window = self.window;
        let include_coalescing_gpu_surface = self.include_coalescing_gpu_surface;
        let list = crate::application::virtual_list_windowed(|index| {
            if include_coalescing_gpu_surface && index == 0 {
                ViewNode::from(SurfaceNode::custom_widget(
                    super::gpu_wheel::PassiveGpuWheelWidget::new(),
                    WidgetMessageMapper::none(),
                ))
                .height(20.0)
                .fill_width()
            } else {
                crate::application::text(format!("Row {index}"))
                    .height(20.0)
                    .fill_width()
            }
        })
        .row_height(20.0)
        .window(window)
        .overscan_px(20.0);
        let list = if self.retain_materialized_window {
            list.retain_materialized_window()
        } else {
            list
        };
        crate::runtime::test_arc_surface(
            list.on_window_changed(|change| change)
                .view()
                .id(81)
                .fill()
                .scroll_policy(
                    crate::layout::ScrollPolicy::default()
                        .scrollbar_visibility(crate::layout::ScrollbarVisibility::Always),
                )
                .into_surface(),
        )
    }

    fn reduce_message(&mut self, message: VirtualListWindowChange) {
        self.scroll_count += 1;
        self.window = message.window;
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, VirtualListWindowChange> {
        RuntimeHostCapabilities::new().with_frame_diagnostics()
    }
}

impl RuntimeFrameDiagnosticsHost for AppVirtualListBridge {
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {}
}

impl RuntimeBridge<VirtualListWindowChange> for HoverVirtualListBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<VirtualListWindowChange>> {
        use crate::{
            application::{row_actions, tree_row, virtual_tree_list_windowed},
            gui::types::Rgba8,
        };

        let window = self.window;
        let guide_rows = vec![TreeGuideRow::new(0, false); window.total_items];
        crate::runtime::test_arc_surface(
            virtual_tree_list_windowed(
                window,
                20.0,
                &guide_rows,
                TreeGuideStyle::new(12.0, 20.0, Rgba8::new(90, 120, 160, 255)),
                |index| {
                    tree_row(format!("Row {index}"))
                        .row_key(format!("hover-row-{index}"))
                        .hit_key(format!("hover-row-hit-{index}"))
                        .row_height(20.0)
                        .interactive_actions(row_actions())
                },
            )
            .overscan_px(20.0)
            .on_window_changed(|change| change)
            .view()
            .id(181)
            .fill()
            .into_surface(),
        )
    }

    fn reduce_message(&mut self, message: VirtualListWindowChange) {
        self.window = message.window;
    }
}

impl RuntimeFrameDiagnosticsHost for HoverVirtualListBridge {
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {}
}
