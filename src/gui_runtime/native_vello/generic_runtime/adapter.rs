//! Event-loop-confined WGPU/Vello adapter ownership for one generic run.

use super::{DeviceLossRegistration, RuntimeUserEvent, device::install_device_loss_callback};
use crate::gui_runtime::{NativeGpuBackend, NativeRunOptions};
use std::{fmt, sync::Arc};
use vello::{
    util::{DeviceHandle, RenderContext, RenderSurface},
    wgpu,
};
use winit::event_loop::EventLoopProxy;

use super::surface::instance_for_options;

/// Monotonic identity for one selected native adapter.
///
/// Unknown and exhausted generations are never valid selection evidence. A
/// failed advancement leaves the caller's previous generation untouched, and
/// a successful advancement can never reuse an earlier serial.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct NativeAdapterGeneration {
    serial: u64,
    status: NativeAdapterGenerationStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeAdapterGenerationStatus {
    Unknown,
    Known,
    Exhausted,
}

impl NativeAdapterGeneration {
    pub(super) const fn unknown() -> Self {
        Self {
            serial: 0,
            status: NativeAdapterGenerationStatus::Unknown,
        }
    }

    /// Advance after a complete adapter selection. Returns `false` once the
    /// serial is exhausted; callers must then remain conservative.
    pub(super) fn advance(&mut self) -> bool {
        if matches!(self.status, NativeAdapterGenerationStatus::Exhausted) {
            return false;
        }
        let Some(serial) = self.serial.checked_add(1) else {
            self.status = NativeAdapterGenerationStatus::Exhausted;
            return false;
        };
        self.serial = serial;
        self.status = NativeAdapterGenerationStatus::Known;
        true
    }

    pub(super) const fn is_known(self) -> bool {
        matches!(self.status, NativeAdapterGenerationStatus::Known)
    }

    #[cfg(test)]
    pub(super) const fn is_exhausted(self) -> bool {
        matches!(self.status, NativeAdapterGenerationStatus::Exhausted)
    }

    #[cfg(test)]
    pub(super) const fn from_test_serial(serial: u64) -> Self {
        Self {
            serial,
            status: NativeAdapterGenerationStatus::Known,
        }
    }
}

impl Default for NativeAdapterGeneration {
    fn default() -> Self {
        Self::unknown()
    }
}

/// Complete selection state published by the shared adapter owner.
///
/// The record is constructed locally and assigned to the owner only after
/// device selection and both callback registrations have completed.
struct SelectedNativeAdapter {
    device_id: usize,
    backend: wgpu::Backend,
    generation: NativeAdapterGeneration,
    device_loss_registration: Arc<DeviceLossRegistration>,
}

/// The one native adapter owner for a generic-native application run.
///
/// The owner is created and used on the event-loop thread. It retains the
/// render context, the one selected device/queue, and the callback witness for
/// that device. Window runners receive borrows of this owner for surface,
/// resize, and presentation work; they never construct another context or
/// device callback pair.
pub(super) struct GenericNativeAdapterOwner {
    render_context: Option<RenderContext>,
    selected: Option<SelectedNativeAdapter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AdapterSurfaceError {
    NoSelectedDevice,
    RenderSurfaceCreation(String),
    DeviceMismatch,
}

impl fmt::Display for AdapterSurfaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSelectedDevice => f.write_str("native adapter has no selected device"),
            Self::RenderSurfaceCreation(message) => {
                write!(f, "native render surface creation failed: {message}")
            }
            Self::DeviceMismatch => f.write_str(
                "native surface resolved to a different device than the selected adapter",
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AuxiliaryAdapterCompatibilityError {
    NoSelectedDevice,
    BackendMismatch,
    SurfaceUnsupported,
}

impl fmt::Display for AuxiliaryAdapterCompatibilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSelectedDevice => f.write_str("native adapter has no selected device"),
            Self::BackendMismatch => f.write_str(
                "auxiliary GPU backend policy is incompatible with the selected native adapter",
            ),
            Self::SurfaceUnsupported => {
                f.write_str("selected native adapter does not support the auxiliary surface")
            }
        }
    }
}

impl GenericNativeAdapterOwner {
    pub(super) fn new(options: &NativeRunOptions) -> Self {
        Self {
            render_context: Some(RenderContext {
                instance: instance_for_options(options),
                devices: Vec::new(),
            }),
            selected: None,
        }
    }

    pub(super) fn instance(&self) -> Option<&wgpu::Instance> {
        self.render_context
            .as_ref()
            .map(|context| &context.instance)
    }

    pub(super) fn select_primary_device(
        &mut self,
        surface: &wgpu::Surface<'_>,
        proxy: EventLoopProxy<RuntimeUserEvent>,
    ) -> Result<(), &'static str> {
        if self.render_context.is_none() {
            return Err("native adapter render context is unavailable");
        }
        let generation = self.next_generation()?;
        let selected = {
            let Some(context) = self.render_context.as_mut() else {
                return Err("native adapter render context is unavailable");
            };
            let Some(device_id) = pollster::block_on(context.device(Some(surface))) else {
                return Err("no compatible render device found");
            };
            let Some(device_handle) = context.devices.get(device_id) else {
                return Err("native adapter selected device handle is unavailable");
            };
            let backend = device_handle.adapter().get_info().backend;
            let device_loss_registration =
                install_device_loss_callback(&device_handle.device, proxy, generation);
            SelectedNativeAdapter {
                device_id,
                backend,
                generation,
                device_loss_registration,
            }
        };
        self.selected = Some(selected);
        Ok(())
    }

    pub(super) fn validate_auxiliary_surface(
        &self,
        requested_backend: NativeGpuBackend,
        surface: &wgpu::Surface<'_>,
    ) -> Result<(), AuxiliaryAdapterCompatibilityError> {
        let selected_backend = self
            .selected
            .as_ref()
            .map(|selected| selected.backend)
            .ok_or(AuxiliaryAdapterCompatibilityError::NoSelectedDevice)?;
        if !auxiliary_backend_policy_is_compatible(requested_backend, selected_backend) {
            return Err(AuxiliaryAdapterCompatibilityError::BackendMismatch);
        }
        if !self
            .selected_adapter()
            .is_some_and(|adapter| adapter.is_surface_supported(surface))
        {
            return Err(AuxiliaryAdapterCompatibilityError::SurfaceUnsupported);
        }
        Ok(())
    }

    pub(super) fn create_render_surface<'surface>(
        &mut self,
        surface: wgpu::Surface<'surface>,
        width: u32,
        height: u32,
        present_mode: wgpu::PresentMode,
    ) -> Result<RenderSurface<'surface>, AdapterSurfaceError> {
        let selected_device_id = self
            .selected
            .as_ref()
            .map(|selected| selected.device_id)
            .ok_or(AdapterSurfaceError::NoSelectedDevice)?;
        let Some(context) = self.render_context.as_mut() else {
            return Err(AdapterSurfaceError::NoSelectedDevice);
        };
        let render_surface =
            pollster::block_on(context.create_render_surface(surface, width, height, present_mode))
                .map_err(render_surface_creation_error)?;
        if render_surface.dev_id != selected_device_id {
            return Err(AdapterSurfaceError::DeviceMismatch);
        }
        Ok(render_surface)
    }

    pub(super) fn selected_device_handle(&self) -> Option<&DeviceHandle> {
        let context = self.render_context.as_ref()?;
        let device_id = self.selected.as_ref()?.device_id;
        context.devices.get(device_id)
    }

    pub(super) fn device_handle_for_surface(
        &self,
        surface: &RenderSurface<'_>,
    ) -> Option<&DeviceHandle> {
        let device_id = self.selected.as_ref()?.device_id;
        (surface.dev_id == device_id)
            .then(|| self.selected_device_handle())
            .flatten()
    }

    pub(super) fn resize_surface(
        &self,
        surface: &mut RenderSurface<'_>,
        width: u32,
        height: u32,
    ) -> bool {
        let Some(device_id) = self.selected.as_ref().map(|selected| selected.device_id) else {
            return false;
        };
        if surface.dev_id != device_id {
            return false;
        }
        let Some(context) = self.render_context.as_ref() else {
            return false;
        };
        context.resize_surface(surface, width, height);
        true
    }

    pub(super) fn accepts_device_loss(
        &self,
        generation: NativeAdapterGeneration,
        registration: &Arc<DeviceLossRegistration>,
    ) -> bool {
        self.selected.as_ref().is_some_and(|selected| {
            generation.is_known()
                && selected.generation == generation
                && device_loss_registration_matches(
                    Some(&selected.device_loss_registration),
                    registration,
                )
        })
    }

    #[cfg(test)]
    pub(super) fn with_test_registration(
        generation: NativeAdapterGeneration,
        registration: Arc<DeviceLossRegistration>,
    ) -> Self {
        Self {
            render_context: None,
            selected: Some(SelectedNativeAdapter {
                device_id: 0,
                backend: wgpu::Backend::Noop,
                generation,
                device_loss_registration: registration,
            }),
        }
    }

    fn next_generation(&self) -> Result<NativeAdapterGeneration, &'static str> {
        let mut generation = self
            .selected
            .as_ref()
            .map_or_else(NativeAdapterGeneration::unknown, |selected| {
                selected.generation
            });
        if !generation.advance() {
            return Err("native adapter generation is exhausted");
        }
        Ok(generation)
    }

    fn selected_adapter(&self) -> Option<&wgpu::Adapter> {
        self.selected_device_handle().map(DeviceHandle::adapter)
    }
}

fn render_surface_creation_error(error: impl fmt::Display) -> AdapterSurfaceError {
    AdapterSurfaceError::RenderSurfaceCreation(error.to_string())
}

pub(super) fn auxiliary_backend_policy_is_compatible(
    requested_backend: NativeGpuBackend,
    selected_backend: wgpu::Backend,
) -> bool {
    match requested_backend {
        NativeGpuBackend::Auto => true,
        NativeGpuBackend::Primary => matches!(
            selected_backend,
            wgpu::Backend::Vulkan
                | wgpu::Backend::Metal
                | wgpu::Backend::Dx12
                | wgpu::Backend::BrowserWebGpu
        ),
        NativeGpuBackend::Vulkan => selected_backend == wgpu::Backend::Vulkan,
        NativeGpuBackend::Dx12 => selected_backend == wgpu::Backend::Dx12,
        NativeGpuBackend::Metal => selected_backend == wgpu::Backend::Metal,
        NativeGpuBackend::Gl => selected_backend == wgpu::Backend::Gl,
        NativeGpuBackend::BrowserWebGpu => selected_backend == wgpu::Backend::BrowserWebGpu,
    }
}

pub(super) fn device_loss_registration_matches(
    current: Option<&Arc<DeviceLossRegistration>>,
    registration: &Arc<DeviceLossRegistration>,
) -> bool {
    current.is_some_and(|current| Arc::ptr_eq(current, registration))
}

#[cfg(test)]
mod tests {
    use super::{
        AdapterSurfaceError, DeviceLossRegistration, GenericNativeAdapterOwner,
        NativeAdapterGeneration, auxiliary_backend_policy_is_compatible,
        device_loss_registration_matches, render_surface_creation_error,
    };
    use crate::gui_runtime::NativeGpuBackend;
    use std::sync::Arc;
    use vello::wgpu;

    #[test]
    fn auxiliary_auto_inherits_every_selected_backend() {
        for backend in wgpu::Backend::ALL {
            assert!(auxiliary_backend_policy_is_compatible(
                NativeGpuBackend::Auto,
                backend
            ));
        }
    }

    #[test]
    fn explicit_auxiliary_backend_requires_exact_or_primary_compatibility() {
        assert!(auxiliary_backend_policy_is_compatible(
            NativeGpuBackend::Metal,
            wgpu::Backend::Metal
        ));
        assert!(!auxiliary_backend_policy_is_compatible(
            NativeGpuBackend::Metal,
            wgpu::Backend::Vulkan
        ));
        assert!(auxiliary_backend_policy_is_compatible(
            NativeGpuBackend::Primary,
            wgpu::Backend::Dx12
        ));
        assert!(!auxiliary_backend_policy_is_compatible(
            NativeGpuBackend::Primary,
            wgpu::Backend::Gl
        ));
    }

    #[test]
    fn one_owner_witness_admits_current_and_rejects_stale_or_missing_events() {
        let current = Arc::new(DeviceLossRegistration::new());
        let stale = Arc::new(DeviceLossRegistration::new());
        let generation = NativeAdapterGeneration::from_test_serial(1);

        assert!(device_loss_registration_matches(Some(&current), &current));
        assert!(!device_loss_registration_matches(Some(&current), &stale));
        assert!(!device_loss_registration_matches(None, &current));
        let owner =
            GenericNativeAdapterOwner::with_test_registration(generation, Arc::clone(&current));
        assert!(owner.accepts_device_loss(generation, &current));
        assert!(
            !owner.accepts_device_loss(NativeAdapterGeneration::from_test_serial(2), &current,)
        );
    }

    #[test]
    fn missing_render_context_is_reported_as_absent() {
        let owner = GenericNativeAdapterOwner::with_test_registration(
            NativeAdapterGeneration::from_test_serial(1),
            Arc::new(DeviceLossRegistration::new()),
        );

        assert!(owner.instance().is_none());
    }

    #[test]
    fn adapter_generation_advances_from_unknown_without_reuse() {
        let mut generation = NativeAdapterGeneration::default();
        assert!(!generation.is_known());
        assert!(!generation.is_exhausted());
        assert!(generation.advance());
        assert!(generation.is_known());
        let previous = generation;
        assert!(generation.advance());
        assert_ne!(generation, previous);
    }

    #[test]
    fn adapter_generation_does_not_wrap_after_exhaustion() {
        let mut generation = NativeAdapterGeneration::from_test_serial(u64::MAX);
        assert!(!generation.advance());
        assert!(!generation.is_known());
        assert!(generation.is_exhausted());
        assert!(!generation.advance());
    }

    #[test]
    fn failed_generation_candidate_leaves_selected_record_unchanged() {
        let registration = Arc::new(DeviceLossRegistration::new());
        let generation = NativeAdapterGeneration::from_test_serial(u64::MAX);
        let owner = GenericNativeAdapterOwner::with_test_registration(
            generation,
            Arc::clone(&registration),
        );

        assert_eq!(
            owner.next_generation(),
            Err("native adapter generation is exhausted")
        );
        assert!(owner.accepts_device_loss(generation, &registration));
    }

    #[test]
    fn render_surface_creation_error_retains_backend_message() {
        let backend_error = String::from("unsupported surface format");
        let error = render_surface_creation_error(backend_error.as_str());
        drop(backend_error);

        assert_eq!(
            error,
            AdapterSurfaceError::RenderSurfaceCreation(String::from("unsupported surface format",))
        );
        assert_eq!(
            error.to_string(),
            "native render surface creation failed: unsupported surface format"
        );
    }
}
