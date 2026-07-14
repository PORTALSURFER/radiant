use super::super::*;
use crate::gui::paint::{PaintFrame, Primitive};
use crate::runtime::{RuntimeHostCapabilities, RuntimeRetainedSurfaceHost};

#[derive(Default)]
pub(super) struct RetainedBridge {
    pub(super) render_count: usize,
    pub(super) dirty_mask: u64,
    pub(super) volatile: bool,
}

#[derive(Default)]
pub(super) struct MultiRetainedBridge {
    render_counts: std::collections::BTreeMap<u64, usize>,
}

#[derive(Default)]
pub(super) struct MissingRetainedBridge {
    pub(super) render_count: usize,
}

impl MultiRetainedBridge {
    pub(super) fn render_count_for(&self, key: u64) -> usize {
        self.render_counts.get(&key).copied().unwrap_or_default()
    }
}

impl RuntimeBridge<()> for RetainedBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::retained_canvas_mapped(
            31,
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            crate::widgets::RetainedSurfaceDescriptor {
                key: 7,
                revision: 1,
                dirty_mask: self.dirty_mask,
                volatile: self.volatile,
            },
            |_| (),
        )))
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new().with_retained_surfaces()
    }
}

impl RuntimeRetainedSurfaceHost for RetainedBridge {
    fn render_retained_surface(
        &mut self,
        _descriptor: crate::widgets::RetainedSurfaceDescriptor,
        rect: UiRect,
        _viewport: Vector2,
    ) -> Option<PaintFrame> {
        self.render_count += 1;
        Some(PaintFrame {
            clear_color: Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            primitives: vec![Primitive::Rect(crate::gui::paint::FillRect {
                rect,
                color: Rgba8 {
                    r: 1,
                    g: 2,
                    b: 3,
                    a: 255,
                },
            })],
            text_runs: Vec::new(),
        })
    }
}

impl RuntimeBridge<()> for MultiRetainedBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy {
                kind: ContainerKind::Row,
                spacing: 8.0,
                ..ContainerPolicy::default()
            },
            vec![
                SurfaceChild::new(
                    SlotParams::fill(),
                    SurfaceNode::retained_canvas_mapped(
                        31,
                        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
                        crate::widgets::RetainedSurfaceDescriptor {
                            key: 7,
                            revision: 1,
                            dirty_mask: 0,
                            volatile: false,
                        },
                        |_| (),
                    ),
                ),
                SurfaceChild::new(
                    SlotParams::fill(),
                    SurfaceNode::retained_canvas_mapped(
                        32,
                        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
                        crate::widgets::RetainedSurfaceDescriptor {
                            key: 8,
                            revision: 1,
                            dirty_mask: 0,
                            volatile: false,
                        },
                        |_| (),
                    ),
                ),
            ],
        )))
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new().with_retained_surfaces()
    }
}

impl RuntimeRetainedSurfaceHost for MultiRetainedBridge {
    fn render_retained_surface(
        &mut self,
        descriptor: crate::widgets::RetainedSurfaceDescriptor,
        rect: UiRect,
        _viewport: Vector2,
    ) -> Option<PaintFrame> {
        *self.render_counts.entry(descriptor.key).or_default() += 1;
        Some(PaintFrame {
            clear_color: Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            primitives: vec![Primitive::Rect(crate::gui::paint::FillRect {
                rect,
                color: Rgba8 {
                    r: descriptor.key as u8,
                    g: 2,
                    b: 3,
                    a: 255,
                },
            })],
            text_runs: Vec::new(),
        })
    }
}

impl RuntimeBridge<()> for MissingRetainedBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::retained_canvas_mapped(
            31,
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            crate::widgets::RetainedSurfaceDescriptor {
                key: 7,
                revision: 1,
                dirty_mask: 0,
                volatile: false,
            },
            |_| (),
        )))
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new().with_retained_surfaces()
    }
}

impl RuntimeRetainedSurfaceHost for MissingRetainedBridge {
    fn render_retained_surface(
        &mut self,
        _descriptor: crate::widgets::RetainedSurfaceDescriptor,
        _rect: UiRect,
        _viewport: Vector2,
    ) -> Option<PaintFrame> {
        self.render_count += 1;
        None
    }
}
