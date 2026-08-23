//! Event-loop-confined WGPU/Vello adapter ownership for one generic run.

use super::device::DeviceFeatureSelection;
use super::runner_state::NativeWindowAtlasResidencySnapshots;
use super::{DeviceLossRegistration, RuntimeUserEvent, device::install_device_loss_callback};
use super::{
    GpuSurfaceAtlasResidencySnapshot, NativeAdapterAtlasResidencyAccountToken,
    NativeAdapterAtlasResidencyProfile, NativeAtlasResidencyWindowIdentity,
};
use crate::gui_runtime::{NativeGpuBackend, NativeRunOptions};
use std::{collections::HashMap, fmt, sync::Arc};
use vello::{util::RenderSurface, wgpu};
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

    pub(super) const fn known_serial(self) -> Option<u64> {
        if self.is_known() {
            Some(self.serial)
        } else {
            None
        }
    }

    pub(super) const fn is_strictly_newer_than(self, previous: Self) -> bool {
        matches!(
            (self.status, previous.status),
            (
                NativeAdapterGenerationStatus::Known,
                NativeAdapterGenerationStatus::Known
            )
        ) && self.serial > previous.serial
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

/// Private Radiant ownership of one WGPU context and its device candidates.
/// Vello renderers and public render surfaces borrow these handles, but Vello
/// does not own adapter selection or device lifetime for the generic runtime.
pub(super) struct RadiantWgpuContext {
    pub(super) instance: wgpu::Instance,
    devices: Vec<RadiantWgpuDevice>,
}

/// Private Radiant ownership of one selected adapter, device, and queue.
pub(super) struct RadiantWgpuDevice {
    adapter: wgpu::Adapter,
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
}

impl RadiantWgpuContext {
    pub(super) fn new(instance: wgpu::Instance) -> Self {
        Self {
            instance,
            devices: Vec::new(),
        }
    }

    pub(super) async fn device(
        &mut self,
        compatible_surface: Option<&wgpu::Surface<'_>>,
    ) -> Option<usize> {
        let compatible = match compatible_surface {
            Some(surface) => self
                .devices
                .iter()
                .enumerate()
                .find(|(_, device)| device.adapter.is_surface_supported(surface))
                .map(|(index, _)| index),
            None => (!self.devices.is_empty()).then_some(0),
        };
        if compatible.is_none() {
            return self.new_device(compatible_surface).await;
        }
        compatible
    }

    pub(super) fn device_handle(&self, device_id: usize) -> Option<&RadiantWgpuDevice> {
        self.devices.get(device_id)
    }

    async fn new_device(
        &mut self,
        compatible_surface: Option<&wgpu::Surface<'_>>,
    ) -> Option<usize> {
        let adapter =
            wgpu::util::initialize_adapter_from_env_or_default(&self.instance, compatible_surface)
                .await
                .ok()?;
        let selection = DeviceFeatureSelection::for_adapter(adapter.features());
        let device =
            request_device_with_fallback(&self.instance, compatible_surface, adapter, selection)
                .await?;
        self.devices.push(device);
        Some(self.devices.len() - 1)
    }

    pub(super) fn create_render_surface<'surface>(
        &self,
        surface: wgpu::Surface<'surface>,
        width: u32,
        height: u32,
        present_mode: wgpu::PresentMode,
        device_id: usize,
    ) -> Result<RenderSurface<'surface>, &'static str> {
        let Some(device) = self.devices.get(device_id) else {
            return Err("selected device handle is unavailable");
        };
        let capabilities = surface.get_capabilities(&device.adapter);
        let format = capabilities
            .formats
            .into_iter()
            .find(|format| {
                matches!(
                    format,
                    wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm
                )
            })
            .ok_or("selected surface has no supported texture format")?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
        };
        let (target_texture, target_view) = create_targets(width, height, &device.device);
        surface.configure(&device.device, &config);
        Ok(RenderSurface {
            surface,
            config,
            dev_id: device_id,
            format,
            target_texture,
            target_view,
            blitter: wgpu::util::TextureBlitter::new(&device.device, format),
        })
    }

    fn resize_surface(&self, surface: &mut RenderSurface<'_>, width: u32, height: u32) -> bool {
        let Some(device) = self.devices.get(surface.dev_id) else {
            return false;
        };
        let (target_texture, target_view) = create_targets(width, height, &device.device);
        surface.target_texture = target_texture;
        surface.target_view = target_view;
        surface.config.width = width;
        surface.config.height = height;
        surface.surface.configure(&device.device, &surface.config);
        true
    }
}

impl RadiantWgpuDevice {
    pub(super) fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }
}

async fn request_device_with_fallback(
    instance: &wgpu::Instance,
    compatible_surface: Option<&wgpu::Surface<'_>>,
    adapter: wgpu::Adapter,
    selection: DeviceFeatureSelection,
) -> Option<RadiantWgpuDevice> {
    match adapter
        .request_device(&device_descriptor(selection.initial_request()))
        .await
    {
        Ok((device, queue)) => Some(RadiantWgpuDevice {
            adapter,
            device,
            queue,
        }),
        Err(_) => {
            selection.retry_after_failure()?;
            // WGPU permits only one request_device call per adapter. Drop the
            // failed adapter before selecting the one permitted fallback.
            drop(adapter);
            let fallback_adapter =
                wgpu::util::initialize_adapter_from_env_or_default(instance, compatible_surface)
                    .await
                    .ok()?;
            let fallback_features =
                DeviceFeatureSelection::for_adapter(fallback_adapter.features()).baseline_request();
            let (device, queue) = fallback_adapter
                .request_device(&device_descriptor(fallback_features))
                .await
                .ok()?;
            Some(RadiantWgpuDevice {
                adapter: fallback_adapter,
                device,
                queue,
            })
        }
    }
}

fn device_descriptor(required_features: wgpu::Features) -> wgpu::DeviceDescriptor<'static> {
    wgpu::DeviceDescriptor {
        label: None,
        required_features,
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }
}

fn create_targets(
    width: u32,
    height: u32,
    device: &wgpu::Device,
) -> (wgpu::Texture, wgpu::TextureView) {
    let target_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
        format: wgpu::TextureFormat::Rgba8Unorm,
        view_formats: &[],
    });
    let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
    (target_texture, target_view)
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct NativeAdapterAtlasResidencyAggregate {
    active_resident_count: Option<usize>,
    active_logical_rgba_texel_bytes: Option<u64>,
    quarantined_resident_count: Option<usize>,
    quarantined_logical_rgba_texel_bytes: Option<u64>,
}

struct NativeAdapterAtlasResidencyAccount {
    account_generation: u64,
    adapter_generation: NativeAdapterGeneration,
    snapshots: NativeWindowAtlasResidencySnapshots,
}

/// Application-scope, crate-private atlas residency evidence owned by the
/// selected adapter. Resource lifecycle code updates it at publication,
/// quarantine, rebind, and physical retirement boundaries; profile capture
/// only copies the cached aggregate.
pub(super) struct NativeAdapterAtlasResidencyLedger {
    accounts: HashMap<NativeAtlasResidencyWindowIdentity, NativeAdapterAtlasResidencyAccount>,
    next_account_generation: Option<u64>,
    /// Generations retained only while they are current or represented by a
    /// live account/incarnation or one of its physical snapshots.
    known_adapter_generations: Vec<NativeAdapterGeneration>,
    current_adapter_generation: NativeAdapterGeneration,
    aggregate: NativeAdapterAtlasResidencyAggregate,
}

impl Default for NativeAdapterAtlasResidencyLedger {
    fn default() -> Self {
        Self {
            accounts: HashMap::new(),
            next_account_generation: Some(1),
            known_adapter_generations: Vec::new(),
            current_adapter_generation: NativeAdapterGeneration::default(),
            aggregate: NativeAdapterAtlasResidencyAggregate {
                active_resident_count: Some(0),
                active_logical_rgba_texel_bytes: Some(0),
                quarantined_resident_count: Some(0),
                quarantined_logical_rgba_texel_bytes: Some(0),
            },
        }
    }
}

impl NativeAdapterAtlasResidencyLedger {
    fn allocate_account_generation(&mut self) -> Option<u64> {
        let generation = self.next_account_generation?;
        self.next_account_generation = generation.checked_add(1);
        Some(generation)
    }

    fn record_adapter_generation(&mut self, generation: NativeAdapterGeneration) {
        if generation.is_known() && !self.known_adapter_generations.contains(&generation) {
            self.known_adapter_generations.push(generation);
        }
        self.current_adapter_generation = generation;
        self.recompute_aggregate();
    }

    fn prune_known_adapter_generations(&mut self) {
        let current_adapter_generation = self.current_adapter_generation;
        let accounts = &self.accounts;
        self.known_adapter_generations.retain(|generation| {
            *generation == current_adapter_generation
                || accounts
                    .values()
                    .any(|account| account_references_adapter_generation(account, *generation))
        });
    }

    fn register(
        &mut self,
        window_identity: NativeAtlasResidencyWindowIdentity,
        adapter_generation: NativeAdapterGeneration,
        snapshots: NativeWindowAtlasResidencySnapshots,
    ) -> Option<NativeAdapterAtlasResidencyAccountToken> {
        if !self.is_known_adapter_generation(adapter_generation)
            || self.accounts.contains_key(&window_identity)
        {
            return None;
        }
        let account_generation = self.allocate_account_generation()?;
        let token = NativeAdapterAtlasResidencyAccountToken {
            window_identity: window_identity.clone(),
            account_generation,
            adapter_generation,
        };
        self.accounts.insert(
            window_identity,
            NativeAdapterAtlasResidencyAccount {
                account_generation,
                adapter_generation,
                snapshots,
            },
        );
        self.recompute_aggregate();
        Some(token)
    }

    fn update(
        &mut self,
        token: &NativeAdapterAtlasResidencyAccountToken,
        snapshots: NativeWindowAtlasResidencySnapshots,
    ) -> bool {
        let Some(account) = self.accounts.get_mut(&token.window_identity) else {
            return false;
        };
        if account.account_generation != token.account_generation
            || account.adapter_generation != token.adapter_generation
        {
            return false;
        }
        account.snapshots = snapshots;
        self.recompute_aggregate();
        true
    }

    fn rebind(
        &mut self,
        token: &NativeAdapterAtlasResidencyAccountToken,
        adapter_generation: NativeAdapterGeneration,
        snapshots: NativeWindowAtlasResidencySnapshots,
    ) -> Option<NativeAdapterAtlasResidencyAccountToken> {
        if !self.is_known_adapter_generation(adapter_generation) {
            return None;
        }
        let account = self.accounts.get_mut(&token.window_identity)?;
        if account.account_generation != token.account_generation
            || account.adapter_generation != token.adapter_generation
        {
            return None;
        }
        account.adapter_generation = adapter_generation;
        account.snapshots = snapshots;
        let next = NativeAdapterAtlasResidencyAccountToken {
            window_identity: token.window_identity.clone(),
            account_generation: token.account_generation,
            adapter_generation,
        };
        self.recompute_aggregate();
        Some(next)
    }

    fn remove(&mut self, token: &NativeAdapterAtlasResidencyAccountToken) -> bool {
        let Some(account) = self.accounts.get(&token.window_identity) else {
            return false;
        };
        if account.account_generation != token.account_generation
            || account.adapter_generation != token.adapter_generation
        {
            return false;
        }
        let removed = self.accounts.remove(&token.window_identity).is_some();
        if removed {
            self.recompute_aggregate();
        }
        removed
    }

    fn is_known_adapter_generation(&self, generation: NativeAdapterGeneration) -> bool {
        generation.is_known() && self.known_adapter_generations.contains(&generation)
    }

    fn profile(&self) -> NativeAdapterAtlasResidencyProfile {
        NativeAdapterAtlasResidencyProfile {
            adapter_generation: self
                .current_adapter_generation
                .is_known()
                .then_some(self.current_adapter_generation),
            active_resident_count: self.aggregate.active_resident_count,
            active_logical_rgba_texel_bytes: self.aggregate.active_logical_rgba_texel_bytes,
            quarantined_resident_count: self.aggregate.quarantined_resident_count,
            quarantined_logical_rgba_texel_bytes: self
                .aggregate
                .quarantined_logical_rgba_texel_bytes,
        }
    }

    fn recompute_aggregate(&mut self) {
        self.prune_known_adapter_generations();
        let mut aggregate = NativeAdapterAtlasResidencyAggregate {
            active_resident_count: Some(0),
            active_logical_rgba_texel_bytes: Some(0),
            quarantined_resident_count: Some(0),
            quarantined_logical_rgba_texel_bytes: Some(0),
        };
        for account in self.accounts.values() {
            accumulate_active(
                &mut aggregate,
                account.snapshots.active,
                self.current_adapter_generation,
            );
            accumulate_quarantine(
                &mut aggregate,
                account.snapshots.quarantine_0,
                &self.known_adapter_generations,
            );
            accumulate_quarantine(
                &mut aggregate,
                account.snapshots.quarantine_1,
                &self.known_adapter_generations,
            );
        }
        self.aggregate = aggregate;
    }

    #[cfg(test)]
    fn account_count(&self) -> usize {
        self.accounts.len()
    }
}

fn account_references_adapter_generation(
    account: &NativeAdapterAtlasResidencyAccount,
    generation: NativeAdapterGeneration,
) -> bool {
    account.adapter_generation == generation
        || account
            .snapshots
            .active
            .is_some_and(|snapshot| snapshot.generation == generation)
        || account
            .snapshots
            .quarantine_0
            .is_some_and(|snapshot| snapshot.generation == generation)
        || account
            .snapshots
            .quarantine_1
            .is_some_and(|snapshot| snapshot.generation == generation)
}

fn accumulate_active(
    aggregate: &mut NativeAdapterAtlasResidencyAggregate,
    snapshot: Option<GpuSurfaceAtlasResidencySnapshot>,
    current_adapter_generation: NativeAdapterGeneration,
) {
    let Some(snapshot) = snapshot else {
        return;
    };
    if snapshot.generation != current_adapter_generation || !current_adapter_generation.is_known() {
        aggregate.active_resident_count = None;
        aggregate.active_logical_rgba_texel_bytes = None;
        return;
    }
    add_count(
        &mut aggregate.active_resident_count,
        snapshot.resident_count,
    );
    add_bytes(
        &mut aggregate.active_logical_rgba_texel_bytes,
        snapshot.logical_rgba_texel_bytes,
    );
}

fn accumulate_quarantine(
    aggregate: &mut NativeAdapterAtlasResidencyAggregate,
    snapshot: Option<GpuSurfaceAtlasResidencySnapshot>,
    known_adapter_generations: &[NativeAdapterGeneration],
) {
    let Some(snapshot) = snapshot else {
        return;
    };
    if !snapshot.generation.is_known() || !known_adapter_generations.contains(&snapshot.generation)
    {
        aggregate.quarantined_resident_count = None;
        aggregate.quarantined_logical_rgba_texel_bytes = None;
        return;
    }
    add_count(
        &mut aggregate.quarantined_resident_count,
        snapshot.resident_count,
    );
    add_bytes(
        &mut aggregate.quarantined_logical_rgba_texel_bytes,
        snapshot.logical_rgba_texel_bytes,
    );
}

fn add_count(total: &mut Option<usize>, contribution: usize) {
    let Some(total_value) = *total else {
        return;
    };
    *total = total_value.checked_add(contribution);
}

fn add_bytes(total: &mut Option<u64>, contribution: Option<u64>) {
    let Some(total_value) = *total else {
        return;
    };
    let Some(contribution) = contribution else {
        *total = None;
        return;
    };
    *total = total_value.checked_add(contribution);
}

/// The one native adapter owner for a generic-native application run.
///
/// The owner is created and used on the event-loop thread. It retains the
/// render context, the one selected device/queue, and the callback witness for
/// that device. Window runners receive borrows of this owner for surface,
/// resize, and presentation work; they never construct another context or
/// device callback pair.
pub(super) struct GenericNativeAdapterOwner {
    render_context: Option<RadiantWgpuContext>,
    selected: Option<SelectedNativeAdapter>,
    atlas_residency: NativeAdapterAtlasResidencyLedger,
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
            render_context: Some(RadiantWgpuContext::new(instance_for_options(options))),
            selected: None,
            atlas_residency: NativeAdapterAtlasResidencyLedger::default(),
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
            let Some(device_handle) = context.device_handle(device_id) else {
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
        self.atlas_residency.record_adapter_generation(generation);
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
        let render_surface = context
            .create_render_surface(surface, width, height, present_mode, selected_device_id)
            .map_err(render_surface_creation_error)?;
        if render_surface.dev_id != selected_device_id {
            return Err(AdapterSurfaceError::DeviceMismatch);
        }
        Ok(render_surface)
    }

    /// Create a surface bundle using the already selected device without
    /// running Vello's device-selection future again. Recovery and lazy
    /// auxiliary rebuilds use this event-loop-local path after a fresh device
    /// candidate has been selected elsewhere.
    pub(super) fn create_render_surface_for_selected<'surface>(
        &self,
        surface: wgpu::Surface<'surface>,
        width: u32,
        height: u32,
        present_mode: wgpu::PresentMode,
    ) -> Result<RenderSurface<'surface>, AdapterSurfaceError> {
        let selected = self
            .selected
            .as_ref()
            .ok_or(AdapterSurfaceError::NoSelectedDevice)?;
        let Some(context) = self.render_context.as_ref() else {
            return Err(AdapterSurfaceError::NoSelectedDevice);
        };
        context
            .create_render_surface(surface, width, height, present_mode, selected.device_id)
            .map_err(render_surface_creation_error)
    }

    pub(super) fn selected_device_handle(&self) -> Option<&RadiantWgpuDevice> {
        let context = self.render_context.as_ref()?;
        let device_id = self.selected.as_ref()?.device_id;
        context.device_handle(device_id)
    }

    pub(super) fn selected_device_identity(&self) -> Option<usize> {
        self.selected_device_handle()
            .map(|handle| super::device::wgpu_device_id(&handle.device))
    }

    /// Capture the owner's current known generation for a window resource
    /// bundle. Callers cannot manufacture or advance this evidence.
    pub(super) fn capture_generation(&self) -> Option<NativeAdapterGeneration> {
        let generation = self.selected.as_ref()?.generation;
        generation.is_known().then_some(generation)
    }

    pub(super) fn capture_device_loss_registration(&self) -> Option<Arc<DeviceLossRegistration>> {
        self.selected
            .as_ref()
            .map(|selected| Arc::clone(&selected.device_loss_registration))
    }

    /// Admit a window resource bundle only when it still names the exact
    /// current known generation owned by this adapter.
    pub(super) fn admit_generation(&self, generation: NativeAdapterGeneration) -> bool {
        self.capture_generation() == Some(generation)
    }

    pub(super) fn register_atlas_residency_account(
        &mut self,
        window_identity: NativeAtlasResidencyWindowIdentity,
        adapter_generation: NativeAdapterGeneration,
        snapshots: NativeWindowAtlasResidencySnapshots,
    ) -> Option<NativeAdapterAtlasResidencyAccountToken> {
        self.atlas_residency
            .register(window_identity, adapter_generation, snapshots)
    }

    pub(super) fn update_atlas_residency_account(
        &mut self,
        token: &NativeAdapterAtlasResidencyAccountToken,
        snapshots: NativeWindowAtlasResidencySnapshots,
    ) -> bool {
        self.atlas_residency.update(token, snapshots)
    }

    pub(super) fn rebind_atlas_residency_account(
        &mut self,
        token: &NativeAdapterAtlasResidencyAccountToken,
        adapter_generation: NativeAdapterGeneration,
        snapshots: NativeWindowAtlasResidencySnapshots,
    ) -> Option<NativeAdapterAtlasResidencyAccountToken> {
        self.atlas_residency
            .rebind(token, adapter_generation, snapshots)
    }

    pub(super) fn remove_atlas_residency_account(
        &mut self,
        token: &NativeAdapterAtlasResidencyAccountToken,
    ) -> bool {
        self.atlas_residency.remove(token)
    }

    pub(super) fn capture_atlas_residency_profile(&self) -> NativeAdapterAtlasResidencyProfile {
        self.atlas_residency.profile()
    }

    pub(super) fn adopt_atlas_residency_ledger(
        &mut self,
        previous: &mut GenericNativeAdapterOwner,
    ) {
        let next_generation = self.capture_generation();
        self.atlas_residency = std::mem::take(&mut previous.atlas_residency);
        if let Some(next_generation) = next_generation {
            self.atlas_residency
                .record_adapter_generation(next_generation);
        }
    }

    pub(super) fn device_handle_for_surface(
        &self,
        surface: &RenderSurface<'_>,
    ) -> Option<&RadiantWgpuDevice> {
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
        context.resize_surface(surface, width, height)
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

    pub(super) fn next_recovery_generation(&self) -> Option<NativeAdapterGeneration> {
        self.next_generation().ok()
    }

    pub(super) fn from_fresh_recovery_context(
        render_context: RadiantWgpuContext,
        device_id: usize,
        generation: NativeAdapterGeneration,
        device_loss_registration: Arc<DeviceLossRegistration>,
    ) -> Result<Self, &'static str> {
        let Some(backend) = render_context
            .device_handle(device_id)
            .map(|device| device.adapter().get_info().backend)
        else {
            return Err("fresh recovery context did not retain its selected device");
        };
        if !generation.is_known() {
            return Err("fresh recovery context requires a known generation");
        }
        let mut owner = Self {
            render_context: Some(render_context),
            selected: Some(SelectedNativeAdapter {
                device_id,
                backend,
                generation,
                device_loss_registration,
            }),
            atlas_residency: NativeAdapterAtlasResidencyLedger::default(),
        };
        owner.atlas_residency.record_adapter_generation(generation);
        Ok(owner)
    }

    #[cfg(test)]
    pub(super) fn with_test_registration(
        generation: NativeAdapterGeneration,
        registration: Arc<DeviceLossRegistration>,
    ) -> Self {
        let mut owner = Self {
            render_context: None,
            selected: Some(SelectedNativeAdapter {
                device_id: 0,
                backend: wgpu::Backend::Noop,
                generation,
                device_loss_registration: registration,
            }),
            atlas_residency: NativeAdapterAtlasResidencyLedger::default(),
        };
        owner.atlas_residency.record_adapter_generation(generation);
        owner
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
        self.selected_device_handle()
            .map(RadiantWgpuDevice::adapter)
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
        GpuSurfaceAtlasResidencySnapshot, NativeAdapterAtlasResidencyAccountToken,
        NativeAdapterAtlasResidencyLedger, NativeAdapterGeneration,
        NativeAtlasResidencyWindowIdentity, NativeWindowAtlasResidencySnapshots,
        auxiliary_backend_policy_is_compatible, device_loss_registration_matches,
        render_surface_creation_error,
    };
    use crate::gui_runtime::NativeGpuBackend;
    use std::sync::Arc;
    use vello::wgpu;

    fn atlas_snapshot(
        generation: NativeAdapterGeneration,
        resident_count: usize,
        logical_rgba_texel_bytes: Option<u64>,
    ) -> GpuSurfaceAtlasResidencySnapshot {
        GpuSurfaceAtlasResidencySnapshot {
            generation,
            resident_count,
            logical_rgba_texel_bytes,
        }
    }

    fn snapshots(
        active: Option<GpuSurfaceAtlasResidencySnapshot>,
        quarantine_0: Option<GpuSurfaceAtlasResidencySnapshot>,
        quarantine_1: Option<GpuSurfaceAtlasResidencySnapshot>,
    ) -> NativeWindowAtlasResidencySnapshots {
        NativeWindowAtlasResidencySnapshots {
            active,
            quarantine_0,
            quarantine_1,
        }
    }

    #[test]
    fn atlas_ledger_aggregates_primary_and_auxiliaries_once() {
        let generation = NativeAdapterGeneration::from_test_serial(1);
        let mut ledger = NativeAdapterAtlasResidencyLedger::default();
        ledger.record_adapter_generation(generation);

        let primary = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Primary,
                generation,
                snapshots(Some(atlas_snapshot(generation, 3, Some(12))), None, None),
            )
            .expect("primary account should register");
        let auxiliary = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Auxiliary(String::from("inspector")),
                generation,
                snapshots(
                    Some(atlas_snapshot(generation, 2, Some(8))),
                    Some(atlas_snapshot(generation, 1, Some(4))),
                    None,
                ),
            )
            .expect("auxiliary account should register");

        assert_eq!(ledger.account_count(), 2);
        assert_eq!(ledger.profile().active_resident_count, Some(5));
        assert_eq!(ledger.profile().active_logical_rgba_texel_bytes, Some(20));
        assert_eq!(ledger.profile().quarantined_resident_count, Some(1));
        assert_eq!(
            ledger.profile().quarantined_logical_rgba_texel_bytes,
            Some(4)
        );

        assert!(ledger.update(
            &primary,
            snapshots(Some(atlas_snapshot(generation, 4, Some(16))), None, None),
        ));
        assert_eq!(ledger.profile().active_resident_count, Some(6));
        assert_eq!(ledger.profile().active_logical_rgba_texel_bytes, Some(24));
        assert!(ledger.remove(&auxiliary));
        assert_eq!(ledger.profile().active_resident_count, Some(4));
        assert_eq!(ledger.profile().quarantined_resident_count, Some(0));
        assert!(!ledger.remove(&auxiliary));
    }

    #[test]
    fn atlas_ledger_keeps_physical_quarantine_across_recovery_and_rebinds_active() {
        let old_generation = NativeAdapterGeneration::from_test_serial(1);
        let new_generation = NativeAdapterGeneration::from_test_serial(2);
        let mut ledger = NativeAdapterAtlasResidencyLedger::default();
        ledger.record_adapter_generation(old_generation);
        let token = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Primary,
                old_generation,
                snapshots(
                    Some(atlas_snapshot(old_generation, 3, Some(12))),
                    None,
                    None,
                ),
            )
            .expect("primary account should register");

        ledger.record_adapter_generation(new_generation);
        assert_eq!(ledger.profile().active_resident_count, None);
        assert_eq!(ledger.profile().active_logical_rgba_texel_bytes, None);

        let rebound = ledger
            .rebind(
                &token,
                new_generation,
                snapshots(
                    Some(atlas_snapshot(new_generation, 2, Some(8))),
                    Some(atlas_snapshot(old_generation, 3, Some(12))),
                    None,
                ),
            )
            .expect("current account should rebind");
        assert_eq!(ledger.profile().active_resident_count, Some(2));
        assert_eq!(ledger.profile().active_logical_rgba_texel_bytes, Some(8));
        assert_eq!(ledger.profile().quarantined_resident_count, Some(3));
        assert_eq!(
            ledger.profile().quarantined_logical_rgba_texel_bytes,
            Some(12)
        );
        assert!(!ledger.update(
            &token,
            snapshots(
                Some(atlas_snapshot(old_generation, 99, Some(396))),
                None,
                None
            ),
        ));
        assert!(ledger.update(
            &rebound,
            snapshots(
                Some(atlas_snapshot(new_generation, 4, Some(16))),
                Some(atlas_snapshot(old_generation, 3, Some(12))),
                None,
            ),
        ));
        assert_eq!(ledger.profile().active_resident_count, Some(4));
        assert_eq!(ledger.profile().quarantined_resident_count, Some(3));
    }

    #[test]
    fn atlas_ledger_prunes_recovery_history_but_keeps_live_quarantine_generation() {
        let first_generation = NativeAdapterGeneration::from_test_serial(1);
        let mut ledger = NativeAdapterAtlasResidencyLedger::default();
        ledger.record_adapter_generation(first_generation);
        let mut token = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Primary,
                first_generation,
                snapshots(
                    Some(atlas_snapshot(first_generation, 1, Some(4))),
                    None,
                    None,
                ),
            )
            .expect("primary account should register");

        for serial in 2..=32 {
            let generation = NativeAdapterGeneration::from_test_serial(serial);
            ledger.record_adapter_generation(generation);
            token = ledger
                .rebind(
                    &token,
                    generation,
                    snapshots(
                        Some(atlas_snapshot(generation, 1, Some(4))),
                        Some(atlas_snapshot(first_generation, 2, Some(8))),
                        None,
                    ),
                )
                .expect("current account should rebind");
        }

        let current_generation = NativeAdapterGeneration::from_test_serial(32);
        assert_eq!(ledger.known_adapter_generations.len(), 2);
        assert!(ledger.known_adapter_generations.contains(&first_generation));
        assert!(
            ledger
                .known_adapter_generations
                .contains(&current_generation)
        );
        assert_eq!(ledger.profile().active_resident_count, Some(1));
        assert_eq!(ledger.profile().quarantined_resident_count, Some(2));
        assert_eq!(
            ledger.profile().quarantined_logical_rgba_texel_bytes,
            Some(8)
        );

        assert!(ledger.update(
            &token,
            snapshots(
                Some(atlas_snapshot(current_generation, 1, Some(4))),
                None,
                None,
            ),
        ));
        assert_eq!(ledger.known_adapter_generations.len(), 1);
        assert!(!ledger.known_adapter_generations.contains(&first_generation));
        assert_eq!(ledger.profile().active_resident_count, Some(1));
        assert_eq!(ledger.profile().quarantined_resident_count, Some(0));
        assert_eq!(
            ledger.profile().quarantined_logical_rgba_texel_bytes,
            Some(0)
        );
    }

    #[test]
    fn atlas_ledger_fences_wrong_generation_and_same_key_incarnations() {
        let first_generation = NativeAdapterGeneration::from_test_serial(1);
        let second_generation = NativeAdapterGeneration::from_test_serial(2);
        let identity = NativeAtlasResidencyWindowIdentity::Auxiliary(String::from("same-key"));
        let mut ledger = NativeAdapterAtlasResidencyLedger::default();
        ledger.record_adapter_generation(first_generation);
        let first = ledger
            .register(
                identity.clone(),
                first_generation,
                snapshots(
                    Some(atlas_snapshot(first_generation, 1, Some(4))),
                    None,
                    None,
                ),
            )
            .expect("first incarnation should register");
        ledger.record_adapter_generation(second_generation);
        let wrong_generation = NativeAdapterAtlasResidencyAccountToken {
            adapter_generation: second_generation,
            ..first.clone()
        };
        assert!(!ledger.update(
            &wrong_generation,
            snapshots(
                Some(atlas_snapshot(first_generation, 7, Some(28))),
                None,
                None
            ),
        ));
        assert!(ledger.remove(&first));
        assert!(!ledger.remove(&first));

        let second = ledger
            .register(
                identity,
                second_generation,
                snapshots(
                    Some(atlas_snapshot(second_generation, 2, Some(8))),
                    None,
                    None,
                ),
            )
            .expect("replacement incarnation should register");
        assert_ne!(first.account_generation, second.account_generation);
        assert!(!ledger.update(
            &first,
            snapshots(
                Some(atlas_snapshot(second_generation, 9, Some(36))),
                None,
                None
            ),
        ));
        assert!(!ledger.remove(&first));
        assert_eq!(ledger.profile().active_resident_count, Some(2));
        assert_eq!(ledger.profile().active_logical_rgba_texel_bytes, Some(8));
    }

    #[test]
    fn atlas_ledger_marks_unknown_generation_or_bytes_unavailable_and_recovers() {
        let generation = NativeAdapterGeneration::from_test_serial(1);
        let unknown_generation = NativeAdapterGeneration::unknown();
        let mut ledger = NativeAdapterAtlasResidencyLedger::default();
        ledger.record_adapter_generation(generation);
        let token = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Primary,
                generation,
                snapshots(
                    Some(atlas_snapshot(unknown_generation, 2, Some(8))),
                    Some(atlas_snapshot(unknown_generation, 1, Some(4))),
                    None,
                ),
            )
            .expect("account generation is known even when a slot is not");
        assert_eq!(ledger.profile().active_resident_count, None);
        assert_eq!(ledger.profile().quarantined_resident_count, None);

        assert!(ledger.update(
            &token,
            snapshots(
                Some(atlas_snapshot(generation, 2, None)),
                Some(atlas_snapshot(generation, 1, Some(4))),
                None,
            ),
        ));
        assert_eq!(ledger.profile().active_resident_count, Some(2));
        assert_eq!(ledger.profile().active_logical_rgba_texel_bytes, None);
        assert_eq!(ledger.profile().quarantined_resident_count, Some(1));

        assert!(ledger.update(
            &token,
            snapshots(
                Some(atlas_snapshot(generation, 2, Some(8))),
                Some(atlas_snapshot(generation, 1, Some(4))),
                None,
            ),
        ));
        assert_eq!(ledger.profile().active_logical_rgba_texel_bytes, Some(8));
    }

    #[test]
    fn atlas_ledger_recovers_after_count_and_byte_overflow() {
        let generation = NativeAdapterGeneration::from_test_serial(1);
        let mut ledger = NativeAdapterAtlasResidencyLedger::default();
        ledger.record_adapter_generation(generation);
        let max = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Primary,
                generation,
                snapshots(
                    Some(atlas_snapshot(generation, usize::MAX, Some(u64::MAX))),
                    None,
                    None,
                ),
            )
            .expect("maximum contribution should register");
        let one = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Auxiliary(String::from("overflow")),
                generation,
                snapshots(Some(atlas_snapshot(generation, 1, Some(1))), None, None),
            )
            .expect("overflow contribution should register");

        assert_eq!(ledger.profile().active_resident_count, None);
        assert_eq!(ledger.profile().active_logical_rgba_texel_bytes, None);
        assert!(ledger.remove(&one));
        assert_eq!(ledger.profile().active_resident_count, Some(usize::MAX));
        assert_eq!(
            ledger.profile().active_logical_rgba_texel_bytes,
            Some(u64::MAX)
        );
        assert!(ledger.remove(&max));
        assert_eq!(ledger.profile().active_resident_count, Some(0));
        assert_eq!(ledger.profile().active_logical_rgba_texel_bytes, Some(0));
    }

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
    fn generation_capture_and_admission_require_the_exact_known_owner_value() {
        let generation = NativeAdapterGeneration::from_test_serial(7);
        let owner = GenericNativeAdapterOwner::with_test_registration(
            generation,
            Arc::new(DeviceLossRegistration::new()),
        );

        assert_eq!(owner.capture_generation(), Some(generation));
        assert!(owner.admit_generation(generation));
        assert!(!owner.admit_generation(NativeAdapterGeneration::from_test_serial(8)));
        assert!(!owner.admit_generation(NativeAdapterGeneration::unknown()));
    }

    #[test]
    fn generation_capture_rejects_an_unselected_owner() {
        let owner = GenericNativeAdapterOwner {
            render_context: None,
            selected: None,
            atlas_residency: NativeAdapterAtlasResidencyLedger::default(),
        };

        assert_eq!(owner.capture_generation(), None);
        assert!(!owner.admit_generation(NativeAdapterGeneration::from_test_serial(1)));
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
    fn recovery_generation_must_be_strictly_newer_and_known() {
        let previous = NativeAdapterGeneration::from_test_serial(4);
        let newer = NativeAdapterGeneration::from_test_serial(5);

        assert!(newer.is_strictly_newer_than(previous));
        assert!(!previous.is_strictly_newer_than(previous));
        assert!(!NativeAdapterGeneration::unknown().is_strictly_newer_than(previous));
        assert!(!newer.is_strictly_newer_than(NativeAdapterGeneration::unknown()));
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
