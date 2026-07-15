use super::super::*;
use crate::application::IntoView;
use crate::gui::list::{
    VirtualListWindow, VirtualListWindowChange, VirtualListWindowRequest,
    resolve_virtual_list_window,
};
use crate::layout::{SizeModeCross, SizeModeMain};
use crate::runtime::{
    NativeFrameDiagnostics, RepaintScope, RuntimeFrameDiagnosticsHost, RuntimeHostCapabilities,
};

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
        }
    }
}

impl RuntimeBridge<String> for CanvasBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<String>> {
        Arc::new(UiSurface::new(SurfaceNode::canvas_mapped(
            21,
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            |message| match message {
                CanvasMessage::Input {
                    input: WidgetInput::Character(character),
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
        Arc::new(UiSurface::new(SurfaceNode::widget(
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
        Arc::new(UiSurface::new(SurfaceNode::canvas_mapped(
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
        Arc::new(UiSurface::new(
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
            .with_scroll_message(Arc::new(|_| Some(String::from("scroll")))),
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
        Arc::new(
            crate::application::virtual_list_windowed(|index| {
                crate::application::text(format!("Row {index}"))
                    .height(20.0)
                    .fill_width()
            })
            .row_height(20.0)
            .window(window)
            .overscan_px(20.0)
            .on_window_changed(|change| change)
            .view()
            .id(81)
            .fill()
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
