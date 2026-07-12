//! Host-driven Vello rendering for embedded native surfaces.

use std::{
    fmt,
    sync::Arc,
    time::{Duration, Instant},
};

use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use vello::{
    AaConfig, RenderParams, Renderer as VelloRenderer, Scene,
    kurbo::Affine,
    util::{RenderContext, RenderSurface},
    wgpu,
};

use super::generic_runtime::{
    GpuSurfaceInteractionRegion, RetainedSurfaceFrameCache, SceneClipState, SceneTextRunBuffer,
    SurfaceSceneEncodeContext, SurfaceVisibleSuffixScratch, encode_surface_paint_plan_to_scene,
    gpu_surface_requires_compositing, surface_rect_has_visible_region,
};
use super::{NativeTextOptions, NativeTextRenderer, startup_renderer_options};
use crate::{
    gui::types::Vector2,
    runtime::{
        PaintPrimitive, Renderer, RetainedSurfaceCachePolicy, RuntimeBridge, SurfacePaintPlan,
        UiSurface,
    },
    theme::DpiScale,
};

/// Native primitive requiring a host-specific compositing pass unavailable to the embedded
/// single-surface Vello renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmbeddedVelloUnsupportedPrimitive {
    /// Retained GPU surfaces require the full native runtime's GPU-surface compositor.
    GpuSurface,
    /// Host-defined custom surfaces require a host `RuntimeBridge` retained-paint callback.
    CustomSurface,
}

/// Failure produced while creating, resizing, or presenting an embedded Vello surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EmbeddedVelloError {
    /// The paint plan requires a native-runtime compositing facility unavailable in this renderer.
    UnsupportedPrimitive(EmbeddedVelloUnsupportedPrimitive),
    /// WGPU could not create a renderable surface from the supplied host handles.
    CreateSurface(String),
    /// No GPU device compatible with the supplied host surface was available.
    NoCompatibleDevice,
    /// Vello renderer initialization failed.
    CreateRenderer(String),
    /// The host surface could not be configured.
    ConfigureSurface(String),
    /// The current host surface texture could not be acquired.
    AcquireSurface(String),
    /// Vello failed to encode or submit the scene.
    Render(String),
}

impl fmt::Display for EmbeddedVelloError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPrimitive(primitive) => {
                write!(formatter, "embedded Vello does not support {primitive:?}")
            }
            Self::CreateSurface(message) => write!(formatter, "create surface: {message}"),
            Self::NoCompatibleDevice => formatter.write_str("no compatible render device"),
            Self::CreateRenderer(message) => write!(formatter, "create renderer: {message}"),
            Self::ConfigureSurface(message) => write!(formatter, "configure surface: {message}"),
            Self::AcquireSurface(message) => write!(formatter, "acquire surface: {message}"),
            Self::Render(message) => write!(formatter, "render scene: {message}"),
        }
    }
}

impl std::error::Error for EmbeddedVelloError {}

/// Borrow-free raw handle pair for a native surface whose lifecycle is owned by an embedding host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EmbeddedVelloSurfaceHandle {
    display: RawDisplayHandle,
    window: RawWindowHandle,
}

impl EmbeddedVelloSurfaceHandle {
    /// Wrap raw display and window handles supplied by an embedding host.
    ///
    /// # Safety
    ///
    /// Both handles must remain valid until the associated [`EmbeddedVelloRenderer`] is dropped.
    /// The window handle must identify a surface target accepted by WGPU on the current platform.
    pub unsafe fn from_raw(display: RawDisplayHandle, window: RawWindowHandle) -> Self {
        Self { display, window }
    }
}

/// Host-driven Radiant Vello renderer for native views embedded in another application's event
/// loop.
///
/// Unlike `run_native_vello_runtime`, this renderer creates no window and owns no event loop. The
/// embedding host forwards input through its normal Radiant runtime adapter and calls
/// [`Renderer::render`] from its native redraw callback. Scene encoding is shared with Radiant's
/// standalone Vello runtime, so paths, gradients, clips, text, images, and SVGs have identical
/// paint semantics in both environments.
pub struct EmbeddedVelloRenderer {
    render_context: RenderContext,
    render_surface: RenderSurface<'static>,
    renderer: VelloRenderer,
    scene: Scene,
    scaled_scene: Scene,
    text_renderer: NativeTextRenderer,
    bridge: EmbeddedSceneBridge,
    retained_cache: RetainedSurfaceFrameCache,
    text_runs: SceneTextRunBuffer,
    gpu_surface_interaction_regions: Vec<GpuSurfaceInteractionRegion>,
    logical_size: Vector2,
    dpi_scale: DpiScale,
    animation_clock: EmbeddedAnimationClock,
}

impl EmbeddedVelloRenderer {
    /// Create a Vello surface for a host-owned native view.
    ///
    /// # Safety
    ///
    /// The raw handles wrapped by `handle` must remain valid and renderable until this renderer is
    /// dropped. Creation and all later methods must run on a thread permitted to access the native
    /// surface by the embedding platform.
    pub unsafe fn new(
        handle: EmbeddedVelloSurfaceHandle,
        logical_size: Vector2,
        dpi_scale: DpiScale,
    ) -> Result<Self, EmbeddedVelloError> {
        unsafe {
            Self::new_with_text_options(
                handle,
                logical_size,
                dpi_scale,
                &NativeTextOptions::default(),
            )
        }
    }

    /// Create a Vello surface for a host-owned native view with explicit font policy.
    ///
    /// Use this constructor when an embedded host supplies portable fonts through
    /// [`NativeTextOptions::embedded_fonts`] or preferred font files through
    /// [`NativeTextOptions::font_paths`]. The options are read while creating the renderer;
    /// the host does not need to keep them alive afterward.
    ///
    /// # Safety
    ///
    /// The raw handles wrapped by `handle` must remain valid and renderable until this renderer is
    /// dropped. Creation and all later methods must run on a thread permitted to access the native
    /// surface by the embedding platform.
    pub unsafe fn new_with_text_options(
        handle: EmbeddedVelloSurfaceHandle,
        logical_size: Vector2,
        dpi_scale: DpiScale,
        text_options: &NativeTextOptions,
    ) -> Result<Self, EmbeddedVelloError> {
        let logical_size = sanitized_logical_size(logical_size);
        let (width, height) = physical_size(logical_size, dpi_scale);
        let mut render_context = RenderContext::new();
        let surface: wgpu::Surface<'static> = unsafe {
            render_context
                .instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: handle.display,
                    raw_window_handle: handle.window,
                })
        }
        .map_err(|error| EmbeddedVelloError::CreateSurface(error.to_string()))?;
        let render_surface = pollster::block_on(render_context.create_render_surface(
            surface,
            width,
            height,
            wgpu::PresentMode::AutoVsync,
        ))
        .map_err(|error| match error {
            vello::Error::NoCompatibleDevice => EmbeddedVelloError::NoCompatibleDevice,
            other => EmbeddedVelloError::ConfigureSurface(other.to_string()),
        })?;
        let device = &render_context.devices[render_surface.dev_id].device;
        let renderer = VelloRenderer::new(device, startup_renderer_options())
            .map_err(|error| EmbeddedVelloError::CreateRenderer(error.to_string()))?;

        Ok(Self {
            render_context,
            render_surface,
            renderer,
            scene: Scene::new(),
            scaled_scene: Scene::new(),
            text_renderer: NativeTextRenderer::with_options(text_options),
            bridge: EmbeddedSceneBridge,
            retained_cache: RetainedSurfaceFrameCache::with_policy(
                RetainedSurfaceCachePolicy::default(),
            ),
            text_runs: SceneTextRunBuffer::new(),
            gpu_surface_interaction_regions: Vec::new(),
            logical_size,
            dpi_scale,
            animation_clock: EmbeddedAnimationClock::start(),
        })
    }

    /// Resize the native surface using logical points and a platform DPI scale.
    pub fn resize(&mut self, logical_size: Vector2, dpi_scale: DpiScale) {
        let logical_size = sanitized_logical_size(logical_size);
        let (width, height) = physical_size(logical_size, dpi_scale);
        if self.render_surface.config.width != width || self.render_surface.config.height != height
        {
            self.render_context
                .resize_surface(&mut self.render_surface, width, height);
        }
        self.logical_size = logical_size;
        self.dpi_scale = dpi_scale;
    }

    /// Encode and present one paint plan with an explicit animation time.
    pub fn render_at(
        &mut self,
        plan: &SurfacePaintPlan,
        animation_time: Duration,
    ) -> Result<(), EmbeddedVelloError> {
        validate_plan(plan)?;
        encode_surface_paint_plan_to_scene(
            plan,
            SurfaceSceneEncodeContext {
                scene: &mut self.scene,
                text_renderer: &mut self.text_renderer,
                bridge: &mut self.bridge,
                viewport: self.logical_size,
                retained_cache: &mut self.retained_cache,
                text_runs: &mut self.text_runs,
                gpu_surface_interaction_regions: &mut self.gpu_surface_interaction_regions,
                animation_time,
            },
        );
        self.scaled_scene.reset();
        self.scaled_scene.append(
            &self.scene,
            Some(Affine::scale(self.dpi_scale.factor() as f64)),
        );

        // Acquire first because Lost/Outdated recovery recreates the target view.
        // The scene must render into the post-recovery target presented below.
        let surface_texture = self.acquire_surface_texture()?;
        let dev_id = self.render_surface.dev_id;
        {
            let device = &self.render_context.devices[dev_id].device;
            let queue = &self.render_context.devices[dev_id].queue;
            self.renderer
                .render_to_texture(
                    device,
                    queue,
                    &self.scaled_scene,
                    &self.render_surface.target_view,
                    &RenderParams {
                        base_color: super::color_from_rgba(plan.clear_color),
                        width: self.render_surface.config.width,
                        height: self.render_surface.config.height,
                        antialiasing_method: AaConfig::Area,
                    },
                )
                .map_err(|error| EmbeddedVelloError::Render(error.to_string()))?;
        }

        let device = &self.render_context.devices[dev_id].device;
        let queue = &self.render_context.devices[dev_id].queue;
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("radiant_embedded_vello_present"),
        });
        self.render_surface.blitter.copy(
            device,
            &mut encoder,
            &self.render_surface.target_view,
            &surface_view,
        );
        queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        Ok(())
    }

    fn acquire_surface_texture(&mut self) -> Result<wgpu::SurfaceTexture, EmbeddedVelloError> {
        match self.render_surface.surface.get_current_texture() {
            Ok(texture) => Ok(texture),
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                let width = self.render_surface.config.width;
                let height = self.render_surface.config.height;
                self.render_context
                    .resize_surface(&mut self.render_surface, width, height);
                self.render_surface
                    .surface
                    .get_current_texture()
                    .map_err(|error| EmbeddedVelloError::AcquireSurface(error.to_string()))
            }
            Err(error) => Err(EmbeddedVelloError::AcquireSurface(error.to_string())),
        }
    }
}

impl Renderer for EmbeddedVelloRenderer {
    type Error = EmbeddedVelloError;

    fn render(&mut self, plan: &SurfacePaintPlan) -> Result<(), Self::Error> {
        let animation_time = self.animation_clock.elapsed();
        self.render_at(plan, animation_time)
    }
}

struct EmbeddedAnimationClock {
    started_at: Instant,
}

impl EmbeddedAnimationClock {
    fn start() -> Self {
        Self {
            started_at: Instant::now(),
        }
    }

    fn elapsed(&self) -> Duration {
        self.elapsed_at(Instant::now())
    }

    fn elapsed_at(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.started_at)
    }
}

fn validate_plan(plan: &SurfacePaintPlan) -> Result<(), EmbeddedVelloError> {
    let mut clip_state = SceneClipState::default();
    let mut surface_visibility = SurfaceVisibleSuffixScratch::default();
    for (index, primitive) in plan.primitives.iter().enumerate() {
        match primitive {
            PaintPrimitive::ClipStart(clip) => {
                clip_state.begin(clip.rect);
                continue;
            }
            PaintPrimitive::ClipEnd(_) => {
                clip_state.end();
                continue;
            }
            _ if clip_state.is_suppressed() => continue,
            _ => {}
        }
        let unsupported = match primitive {
            PaintPrimitive::GpuSurface(surface)
                if gpu_surface_requires_compositing(
                    surface,
                    plan.primitives.get(..index).unwrap_or_default(),
                    plan.primitives.get(index + 1..).unwrap_or_default(),
                    &mut surface_visibility,
                ) =>
            {
                Some(EmbeddedVelloUnsupportedPrimitive::GpuSurface)
            }
            PaintPrimitive::GpuSurface(_) => None,
            PaintPrimitive::CustomSurface(custom)
                if custom.retained.is_some()
                    && surface_rect_has_visible_region(
                        custom.rect,
                        plan.primitives.get(..index).unwrap_or_default(),
                        plan.primitives.get(index + 1..).unwrap_or_default(),
                        &mut surface_visibility,
                    ) =>
            {
                Some(EmbeddedVelloUnsupportedPrimitive::CustomSurface)
            }
            PaintPrimitive::CustomSurface(_) => None,
            _ => None,
        };
        if let Some(primitive) = unsupported {
            return Err(EmbeddedVelloError::UnsupportedPrimitive(primitive));
        }
    }
    Ok(())
}

fn sanitized_logical_size(size: Vector2) -> Vector2 {
    Vector2::new(
        if size.x.is_finite() {
            size.x.max(1.0)
        } else {
            1.0
        },
        if size.y.is_finite() {
            size.y.max(1.0)
        } else {
            1.0
        },
    )
}

fn physical_size(size: Vector2, dpi_scale: DpiScale) -> (u32, u32) {
    (
        dpi_scale.logical_to_physical(size.x).ceil().max(1.0) as u32,
        dpi_scale.logical_to_physical(size.y).ceil().max(1.0) as u32,
    )
}

struct EmbeddedSceneBridge;

impl RuntimeBridge<()> for EmbeddedSceneBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        unreachable!("embedded paint-plan rendering never projects a surface")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::{ImageRgba, Rect};
    use crate::runtime::{
        GpuShaderSurfaceDescriptor, GpuSurfaceCapabilities, GpuSurfaceContent, PaintClipEnd,
        PaintClipStart, PaintCustomSurface, PaintFillPath, PaintFillRect, PaintGpuSurface,
        PaintPath, PaintPathCommand,
    };
    use crate::theme::ThemeTokens;
    use crate::widgets::{PaintBounds, RetainedSurfaceDescriptor};

    #[test]
    fn embedded_vello_accepts_gradient_fill_paths() {
        let mut plan = SurfacePaintPlan::empty(&ThemeTokens::default());
        plan.primitives
            .push(PaintPrimitive::FillPath(PaintFillPath::new(
                1,
                PaintPath::from([
                    PaintPathCommand::MoveTo(crate::gui::types::Point::new(0.0, 10.0)),
                    PaintPathCommand::LineTo(crate::gui::types::Point::new(5.0, 0.0)),
                    PaintPathCommand::LineTo(crate::gui::types::Point::new(10.0, 10.0)),
                    PaintPathCommand::Close,
                ]),
                crate::runtime::PaintBrush::linear_gradient(
                    crate::runtime::PaintLinearGradient::vertical(
                        crate::gui::types::Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
                        crate::gui::types::Rgba8::new(255, 0, 0, 96),
                        crate::gui::types::Rgba8::new(255, 0, 0, 0),
                    ),
                ),
            )));

        assert_eq!(validate_plan(&plan), Ok(()));
    }

    #[test]
    fn embedded_vello_rejects_retained_host_custom_surfaces_before_rendering() {
        let mut plan = SurfacePaintPlan::empty(&ThemeTokens::default());
        plan.primitives
            .push(PaintPrimitive::CustomSurface(PaintCustomSurface {
                widget_id: 2,
                rect: crate::gui::types::Rect::from_xy_size(0.0, 0.0, 1.0, 1.0),
                bounds: PaintBounds::ClipToRect,
                retained: Some(RetainedSurfaceDescriptor {
                    key: 1,
                    revision: 1,
                    dirty_mask: 0,
                    volatile: false,
                }),
            }));

        assert_eq!(
            validate_plan(&plan),
            Err(EmbeddedVelloError::UnsupportedPrimitive(
                EmbeddedVelloUnsupportedPrimitive::CustomSurface
            ))
        );
    }

    #[test]
    fn embedded_vello_accepts_non_retained_custom_surface_fallbacks() {
        let mut plan = SurfacePaintPlan::empty(&ThemeTokens::default());
        plan.primitives
            .push(PaintPrimitive::CustomSurface(PaintCustomSurface {
                widget_id: 2,
                rect: Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
                bounds: PaintBounds::ClipToRect,
                retained: None,
            }));

        assert_eq!(validate_plan(&plan), Ok(()));
    }

    #[test]
    fn embedded_vello_ignores_retained_custom_surfaces_covered_by_opaque_suffixes() {
        let mut plan = SurfacePaintPlan::empty(&ThemeTokens::default());
        plan.primitives
            .push(PaintPrimitive::CustomSurface(PaintCustomSurface {
                widget_id: 2,
                rect: Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
                bounds: PaintBounds::ClipToRect,
                retained: Some(RetainedSurfaceDescriptor {
                    key: 1,
                    revision: 1,
                    dirty_mask: 0,
                    volatile: false,
                }),
            }));
        plan.primitives
            .push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: 3,
                rect: Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
                color: crate::gui::types::Rgba8::new(20, 30, 40, 255),
            }));

        assert_eq!(validate_plan(&plan), Ok(()));
    }

    #[test]
    fn embedded_vello_ignores_custom_surfaces_without_renderable_geometry() {
        let mut plan = SurfacePaintPlan::empty(&ThemeTokens::default());
        plan.primitives
            .push(PaintPrimitive::CustomSurface(PaintCustomSurface {
                widget_id: 2,
                rect: Rect::from_xy_size(0.0, 0.0, 0.0, 0.0),
                bounds: PaintBounds::ClipToRect,
                retained: None,
            }));

        assert_eq!(validate_plan(&plan), Ok(()));
    }

    #[test]
    fn embedded_vello_rejects_renderable_gpu_surfaces_before_rendering() {
        let mut plan = SurfacePaintPlan::empty(&ThemeTokens::default());
        plan.primitives
            .push(PaintPrimitive::GpuSurface(test_gpu_surface(
                Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
            )));

        assert_eq!(
            validate_plan(&plan),
            Err(EmbeddedVelloError::UnsupportedPrimitive(
                EmbeddedVelloUnsupportedPrimitive::GpuSurface
            ))
        );
    }

    #[test]
    fn embedded_vello_ignores_gpu_surfaces_without_renderable_geometry() {
        let mut plan = SurfacePaintPlan::empty(&ThemeTokens::default());
        plan.primitives
            .push(PaintPrimitive::GpuSurface(test_gpu_surface(
                Rect::from_xy_size(0.0, 0.0, 0.0, 10.0),
            )));

        assert_eq!(validate_plan(&plan), Ok(()));
    }

    #[test]
    fn embedded_vello_ignores_gpu_surfaces_without_renderable_content() {
        let mut surface = test_gpu_surface(Rect::from_xy_size(0.0, 0.0, 10.0, 10.0));
        surface.content = GpuSurfaceContent::RgbaAtlas {
            source_rect: Rect::from_xy_size(0.0, 0.0, 2.0, 2.0),
            atlas: Arc::new(
                ImageRgba::new(1, 1, vec![255, 255, 255, 255]).expect("valid test atlas"),
            ),
        };
        let mut plan = SurfacePaintPlan::empty(&ThemeTokens::default());
        plan.primitives.push(PaintPrimitive::GpuSurface(surface));

        assert_eq!(validate_plan(&plan), Ok(()));
    }

    #[test]
    fn embedded_vello_ignores_gpu_surfaces_fully_covered_by_opaque_suffixes() {
        let mut plan = SurfacePaintPlan::empty(&ThemeTokens::default());
        plan.primitives
            .push(PaintPrimitive::GpuSurface(test_gpu_surface(
                Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
            )));
        plan.primitives
            .push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: 4,
                rect: Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
                color: crate::gui::types::Rgba8::new(20, 30, 40, 255),
            }));

        assert_eq!(validate_plan(&plan), Ok(()));
    }

    #[test]
    fn embedded_vello_rejects_gpu_surfaces_with_visible_uncovered_regions() {
        let mut plan = SurfacePaintPlan::empty(&ThemeTokens::default());
        plan.primitives
            .push(PaintPrimitive::GpuSurface(test_gpu_surface(
                Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
            )));
        plan.primitives
            .push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: 4,
                rect: Rect::from_xy_size(0.0, 0.0, 5.0, 10.0),
                color: crate::gui::types::Rgba8::new(20, 30, 40, 255),
            }));

        assert_eq!(
            validate_plan(&plan),
            Err(EmbeddedVelloError::UnsupportedPrimitive(
                EmbeddedVelloUnsupportedPrimitive::GpuSurface
            ))
        );
    }

    #[test]
    fn embedded_vello_rejects_gpu_surface_partly_covered_by_clipped_opaque_fill() {
        let mut plan = SurfacePaintPlan::empty(&ThemeTokens::default());
        plan.primitives
            .push(PaintPrimitive::GpuSurface(test_gpu_surface(
                Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
            )));
        plan.primitives
            .push(PaintPrimitive::ClipStart(PaintClipStart {
                node_id: 4,
                rect: Rect::from_xy_size(0.0, 0.0, 5.0, 10.0),
            }));
        plan.primitives
            .push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: 5,
                rect: Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
                color: crate::gui::types::Rgba8::new(20, 30, 40, 255),
            }));
        plan.primitives
            .push(PaintPrimitive::ClipEnd(PaintClipEnd { node_id: 4 }));

        assert_eq!(
            validate_plan(&plan),
            Err(EmbeddedVelloError::UnsupportedPrimitive(
                EmbeddedVelloUnsupportedPrimitive::GpuSurface
            ))
        );
    }

    #[test]
    fn embedded_vello_ignores_custom_shaders_without_native_pipeline_inputs() {
        let mut surface = test_gpu_surface(Rect::from_xy_size(0.0, 0.0, 10.0, 10.0));
        surface.content = GpuSurfaceContent::CustomShader {
            descriptor: Arc::new(GpuShaderSurfaceDescriptor::new("test/backend-owned")),
        };
        let mut plan = SurfacePaintPlan::empty(&ThemeTokens::default());
        plan.primitives.push(PaintPrimitive::GpuSurface(surface));

        assert_eq!(validate_plan(&plan), Ok(()));
    }

    #[test]
    fn embedded_vello_rejects_visible_custom_shaders_with_native_pipeline_inputs() {
        let mut surface = test_gpu_surface(Rect::from_xy_size(0.0, 0.0, 10.0, 10.0));
        surface.content = GpuSurfaceContent::CustomShader {
            descriptor: Arc::new(
                GpuShaderSurfaceDescriptor::new("test/native")
                    .wgsl_source("native shader source")
                    .fragment_entry_point("fragment_main"),
            ),
        };
        let mut plan = SurfacePaintPlan::empty(&ThemeTokens::default());
        plan.primitives.push(PaintPrimitive::GpuSurface(surface));

        assert_eq!(
            validate_plan(&plan),
            Err(EmbeddedVelloError::UnsupportedPrimitive(
                EmbeddedVelloUnsupportedPrimitive::GpuSurface
            ))
        );
    }

    #[test]
    fn embedded_vello_ignores_unsupported_surfaces_inside_suppressed_clips() {
        let mut plan = SurfacePaintPlan::empty(&ThemeTokens::default());
        plan.primitives
            .push(PaintPrimitive::ClipStart(PaintClipStart {
                node_id: 1,
                rect: crate::gui::types::Rect::from_xy_size(0.0, 0.0, 0.0, 10.0),
            }));
        plan.primitives
            .push(PaintPrimitive::CustomSurface(PaintCustomSurface {
                widget_id: 2,
                rect: crate::gui::types::Rect::from_xy_size(0.0, 0.0, 1.0, 1.0),
                bounds: PaintBounds::ClipToRect,
                retained: None,
            }));
        plan.primitives
            .push(PaintPrimitive::ClipEnd(PaintClipEnd { node_id: 1 }));

        assert_eq!(validate_plan(&plan), Ok(()));
    }

    #[test]
    fn embedded_vello_physical_size_respects_dpi_scale() {
        assert_eq!(
            physical_size(Vector2::new(420.0, 282.0), DpiScale::new(2.0)),
            (840, 564)
        );
    }

    #[test]
    fn embedded_animation_clock_reports_elapsed_monotonic_time() {
        let started_at = Instant::now();
        let clock = EmbeddedAnimationClock { started_at };

        assert_eq!(clock.elapsed_at(started_at), Duration::ZERO);
        assert_eq!(
            clock.elapsed_at(started_at + Duration::from_millis(750)),
            Duration::from_millis(750)
        );
    }

    fn test_gpu_surface(rect: Rect) -> PaintGpuSurface {
        PaintGpuSurface {
            widget_id: 3,
            key: 1,
            revision: 1,
            rect,
            content: GpuSurfaceContent::RgbaAtlas {
                source_rect: Rect::from_xy_size(0.0, 0.0, 1.0, 1.0),
                atlas: Arc::new(
                    ImageRgba::new(1, 1, vec![255, 255, 255, 255]).expect("valid test atlas"),
                ),
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        }
    }
}
