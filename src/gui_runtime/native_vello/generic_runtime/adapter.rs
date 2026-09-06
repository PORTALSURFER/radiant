//! Event-loop-confined WGPU/Vello adapter ownership for one generic run.

use super::device::DeviceFeatureSelection;
use super::gpu_surface::{
    GpuSurfaceRenderCanvasUploadPlan, GpuSurfaceRenderCanvasUploadPlanContext,
    GpuSurfaceRenderCanvasUploadPlanObservation, GpuSurfaceRenderCanvasUploadStats,
};
use super::native_render_target::{
    NativeRenderTargetReplacementContext, NativeRenderTargetReplacementEvidence,
    NativeRenderTargetReplacementMode, NativeRenderTargetReplacementOutcome,
    NativeRenderTargetReplacementRequest, NativeRenderTargetRetirement, replacement_preflight,
    retirement_identity,
};
use super::runner_state::{
    NativeWindowAtlasResidencySnapshots, NativeWindowCustomShaderResidencySnapshots,
    NativeWindowSignalResidencySnapshots,
};
use super::{DeviceLossRegistration, RuntimeUserEvent, device::install_device_loss_callback};
use super::{
    GpuSurfaceAtlasResidencySnapshot, GpuSurfaceCustomShaderResidencySnapshot,
    GpuSurfaceSignalResidencySnapshot, NativeAdapterAtlasResidencyAccountToken,
    NativeAdapterAtlasResidencyProfile, NativeAdapterCustomShaderResidencyAccountToken,
    NativeAdapterCustomShaderResidencyProfile, NativeAdapterRenderCanvasUploadAccountToken,
    NativeAdapterRenderCanvasUploadProfile, NativeAdapterSignalResidencyAccountToken,
    NativeAdapterSignalResidencyProfile, NativeAtlasResidencyWindowIdentity,
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
        let selection =
            DeviceFeatureSelection::for_adapter(adapter.get_info().backend, adapter.features());
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
            let fallback_features = DeviceFeatureSelection::for_adapter(
                fallback_adapter.get_info().backend,
                fallback_adapter.features(),
            )
            .baseline_request();
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

fn add_optional_count(total: &mut Option<usize>, contribution: Option<usize>) {
    let Some(contribution) = contribution else {
        *total = None;
        return;
    };
    add_count(total, contribution);
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct NativeAdapterSignalResidencyAggregate {
    active_signal_buffer_resident_count: Option<usize>,
    active_signal_buffer_logical_bytes: Option<u64>,
    active_signal_body_texture_resident_count: Option<usize>,
    active_signal_body_texture_logical_rgba_bytes: Option<u64>,
    quarantined_signal_buffer_resident_count: Option<usize>,
    quarantined_signal_buffer_logical_bytes: Option<u64>,
    quarantined_signal_body_texture_resident_count: Option<usize>,
    quarantined_signal_body_texture_logical_rgba_bytes: Option<u64>,
}

struct NativeAdapterSignalResidencyAccount {
    account_generation: u64,
    adapter_generation: NativeAdapterGeneration,
    snapshots: NativeWindowSignalResidencySnapshots,
}

/// Application-scope, crate-private signal residency evidence owned by the
/// selected adapter. Resource lifecycle code updates it at publication,
/// quarantine, rebind, and physical retirement boundaries; profile capture
/// only copies the cached aggregate.
pub(super) struct NativeAdapterSignalResidencyLedger {
    accounts: HashMap<NativeAtlasResidencyWindowIdentity, NativeAdapterSignalResidencyAccount>,
    next_account_generation: Option<u64>,
    /// Generations retained only while they are current or represented by a
    /// live account/incarnation or one of its physical snapshots.
    known_adapter_generations: Vec<NativeAdapterGeneration>,
    current_adapter_generation: NativeAdapterGeneration,
    aggregate: NativeAdapterSignalResidencyAggregate,
}

impl Default for NativeAdapterSignalResidencyLedger {
    fn default() -> Self {
        Self {
            accounts: HashMap::new(),
            next_account_generation: Some(1),
            known_adapter_generations: Vec::new(),
            current_adapter_generation: NativeAdapterGeneration::default(),
            aggregate: NativeAdapterSignalResidencyAggregate {
                active_signal_buffer_resident_count: Some(0),
                active_signal_buffer_logical_bytes: Some(0),
                active_signal_body_texture_resident_count: Some(0),
                active_signal_body_texture_logical_rgba_bytes: Some(0),
                quarantined_signal_buffer_resident_count: Some(0),
                quarantined_signal_buffer_logical_bytes: Some(0),
                quarantined_signal_body_texture_resident_count: Some(0),
                quarantined_signal_body_texture_logical_rgba_bytes: Some(0),
            },
        }
    }
}

impl NativeAdapterSignalResidencyLedger {
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
                || accounts.values().any(|account| {
                    signal_account_references_adapter_generation(account, *generation)
                })
        });
    }

    fn register(
        &mut self,
        window_identity: NativeAtlasResidencyWindowIdentity,
        adapter_generation: NativeAdapterGeneration,
        snapshots: NativeWindowSignalResidencySnapshots,
    ) -> Option<NativeAdapterSignalResidencyAccountToken> {
        if !self.is_known_adapter_generation(adapter_generation)
            || self.accounts.contains_key(&window_identity)
        {
            return None;
        }
        let account_generation = self.allocate_account_generation()?;
        let token = NativeAdapterSignalResidencyAccountToken {
            window_identity: window_identity.clone(),
            account_generation,
            adapter_generation,
        };
        self.accounts.insert(
            window_identity,
            NativeAdapterSignalResidencyAccount {
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
        token: &NativeAdapterSignalResidencyAccountToken,
        snapshots: NativeWindowSignalResidencySnapshots,
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
        token: &NativeAdapterSignalResidencyAccountToken,
        adapter_generation: NativeAdapterGeneration,
        snapshots: NativeWindowSignalResidencySnapshots,
    ) -> Option<NativeAdapterSignalResidencyAccountToken> {
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
        let next = NativeAdapterSignalResidencyAccountToken {
            window_identity: token.window_identity.clone(),
            account_generation: token.account_generation,
            adapter_generation,
        };
        self.recompute_aggregate();
        Some(next)
    }

    fn remove(&mut self, token: &NativeAdapterSignalResidencyAccountToken) -> bool {
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

    fn profile(&self) -> NativeAdapterSignalResidencyProfile {
        NativeAdapterSignalResidencyProfile {
            adapter_generation: self
                .current_adapter_generation
                .is_known()
                .then_some(self.current_adapter_generation),
            active_signal_buffer_resident_count: self.aggregate.active_signal_buffer_resident_count,
            active_signal_buffer_logical_bytes: self.aggregate.active_signal_buffer_logical_bytes,
            active_signal_body_texture_resident_count: self
                .aggregate
                .active_signal_body_texture_resident_count,
            active_signal_body_texture_logical_rgba_bytes: self
                .aggregate
                .active_signal_body_texture_logical_rgba_bytes,
            quarantined_signal_buffer_resident_count: self
                .aggregate
                .quarantined_signal_buffer_resident_count,
            quarantined_signal_buffer_logical_bytes: self
                .aggregate
                .quarantined_signal_buffer_logical_bytes,
            quarantined_signal_body_texture_resident_count: self
                .aggregate
                .quarantined_signal_body_texture_resident_count,
            quarantined_signal_body_texture_logical_rgba_bytes: self
                .aggregate
                .quarantined_signal_body_texture_logical_rgba_bytes,
        }
    }

    fn recompute_aggregate(&mut self) {
        self.prune_known_adapter_generations();
        let mut aggregate = NativeAdapterSignalResidencyAggregate {
            active_signal_buffer_resident_count: Some(0),
            active_signal_buffer_logical_bytes: Some(0),
            active_signal_body_texture_resident_count: Some(0),
            active_signal_body_texture_logical_rgba_bytes: Some(0),
            quarantined_signal_buffer_resident_count: Some(0),
            quarantined_signal_buffer_logical_bytes: Some(0),
            quarantined_signal_body_texture_resident_count: Some(0),
            quarantined_signal_body_texture_logical_rgba_bytes: Some(0),
        };
        for account in self.accounts.values() {
            accumulate_signal_active(
                &mut aggregate,
                account.snapshots.active,
                self.current_adapter_generation,
            );
            accumulate_signal_quarantine(
                &mut aggregate,
                account.snapshots.quarantine_0,
                &self.known_adapter_generations,
            );
            accumulate_signal_quarantine(
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

fn signal_account_references_adapter_generation(
    account: &NativeAdapterSignalResidencyAccount,
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

fn accumulate_signal_active(
    aggregate: &mut NativeAdapterSignalResidencyAggregate,
    snapshot: Option<GpuSurfaceSignalResidencySnapshot>,
    current_adapter_generation: NativeAdapterGeneration,
) {
    let Some(snapshot) = snapshot else {
        return;
    };
    if snapshot.generation != current_adapter_generation || !current_adapter_generation.is_known() {
        aggregate.active_signal_buffer_resident_count = None;
        aggregate.active_signal_buffer_logical_bytes = None;
        aggregate.active_signal_body_texture_resident_count = None;
        aggregate.active_signal_body_texture_logical_rgba_bytes = None;
        return;
    }
    add_count(
        &mut aggregate.active_signal_buffer_resident_count,
        snapshot.signal_buffer_resident_count,
    );
    add_bytes(
        &mut aggregate.active_signal_buffer_logical_bytes,
        snapshot.signal_buffer_logical_bytes,
    );
    add_count(
        &mut aggregate.active_signal_body_texture_resident_count,
        snapshot.signal_body_texture_resident_count,
    );
    add_bytes(
        &mut aggregate.active_signal_body_texture_logical_rgba_bytes,
        snapshot.signal_body_texture_logical_rgba_bytes,
    );
}

fn accumulate_signal_quarantine(
    aggregate: &mut NativeAdapterSignalResidencyAggregate,
    snapshot: Option<GpuSurfaceSignalResidencySnapshot>,
    known_adapter_generations: &[NativeAdapterGeneration],
) {
    let Some(snapshot) = snapshot else {
        return;
    };
    if !snapshot.generation.is_known() || !known_adapter_generations.contains(&snapshot.generation)
    {
        aggregate.quarantined_signal_buffer_resident_count = None;
        aggregate.quarantined_signal_buffer_logical_bytes = None;
        aggregate.quarantined_signal_body_texture_resident_count = None;
        aggregate.quarantined_signal_body_texture_logical_rgba_bytes = None;
        return;
    }
    add_count(
        &mut aggregate.quarantined_signal_buffer_resident_count,
        snapshot.signal_buffer_resident_count,
    );
    add_bytes(
        &mut aggregate.quarantined_signal_buffer_logical_bytes,
        snapshot.signal_buffer_logical_bytes,
    );
    add_count(
        &mut aggregate.quarantined_signal_body_texture_resident_count,
        snapshot.signal_body_texture_resident_count,
    );
    add_bytes(
        &mut aggregate.quarantined_signal_body_texture_logical_rgba_bytes,
        snapshot.signal_body_texture_logical_rgba_bytes,
    );
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct NativeAdapterCustomShaderResidencyAggregate {
    active_pipeline_resident_count: Option<usize>,
    active_binding_resident_count: Option<usize>,
    active_surface_uniform_logical_bytes: Option<u64>,
    active_app_uniform_logical_bytes: Option<u64>,
    active_storage_logical_bytes: Option<u64>,
    active_presentation_uniform_logical_bytes: Option<u64>,
    quarantined_pipeline_resident_count: Option<usize>,
    quarantined_binding_resident_count: Option<usize>,
    quarantined_surface_uniform_logical_bytes: Option<u64>,
    quarantined_app_uniform_logical_bytes: Option<u64>,
    quarantined_storage_logical_bytes: Option<u64>,
    quarantined_presentation_uniform_logical_bytes: Option<u64>,
}

struct NativeAdapterCustomShaderResidencyAccount {
    account_generation: u64,
    adapter_generation: NativeAdapterGeneration,
    snapshots: NativeWindowCustomShaderResidencySnapshots,
}

/// Application-scope, crate-private custom-shader residency evidence owned by
/// the selected adapter. Resource lifecycle code updates it at publication,
/// quarantine, rebind, and physical retirement boundaries; profile capture
/// only copies the cached aggregate.
pub(super) struct NativeAdapterCustomShaderResidencyLedger {
    accounts:
        HashMap<NativeAtlasResidencyWindowIdentity, NativeAdapterCustomShaderResidencyAccount>,
    next_account_generation: Option<u64>,
    /// Generations retained only while they are current or represented by a
    /// live account/incarnation or one of its physical snapshots.
    known_adapter_generations: Vec<NativeAdapterGeneration>,
    current_adapter_generation: NativeAdapterGeneration,
    aggregate: NativeAdapterCustomShaderResidencyAggregate,
}

impl Default for NativeAdapterCustomShaderResidencyLedger {
    fn default() -> Self {
        Self {
            accounts: HashMap::new(),
            next_account_generation: Some(1),
            known_adapter_generations: Vec::new(),
            current_adapter_generation: NativeAdapterGeneration::default(),
            aggregate: NativeAdapterCustomShaderResidencyAggregate {
                active_pipeline_resident_count: Some(0),
                active_binding_resident_count: Some(0),
                active_surface_uniform_logical_bytes: Some(0),
                active_app_uniform_logical_bytes: Some(0),
                active_storage_logical_bytes: Some(0),
                active_presentation_uniform_logical_bytes: Some(0),
                quarantined_pipeline_resident_count: Some(0),
                quarantined_binding_resident_count: Some(0),
                quarantined_surface_uniform_logical_bytes: Some(0),
                quarantined_app_uniform_logical_bytes: Some(0),
                quarantined_storage_logical_bytes: Some(0),
                quarantined_presentation_uniform_logical_bytes: Some(0),
            },
        }
    }
}

impl NativeAdapterCustomShaderResidencyLedger {
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
                || accounts.values().any(|account| {
                    custom_shader_account_references_adapter_generation(account, *generation)
                })
        });
    }

    fn register(
        &mut self,
        window_identity: NativeAtlasResidencyWindowIdentity,
        adapter_generation: NativeAdapterGeneration,
        snapshots: NativeWindowCustomShaderResidencySnapshots,
    ) -> Option<NativeAdapterCustomShaderResidencyAccountToken> {
        if !self.is_known_adapter_generation(adapter_generation)
            || self.accounts.contains_key(&window_identity)
        {
            return None;
        }
        let account_generation = self.allocate_account_generation()?;
        let token = NativeAdapterCustomShaderResidencyAccountToken {
            window_identity: window_identity.clone(),
            account_generation,
            adapter_generation,
        };
        self.accounts.insert(
            window_identity,
            NativeAdapterCustomShaderResidencyAccount {
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
        token: &NativeAdapterCustomShaderResidencyAccountToken,
        snapshots: NativeWindowCustomShaderResidencySnapshots,
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
        token: &NativeAdapterCustomShaderResidencyAccountToken,
        adapter_generation: NativeAdapterGeneration,
        snapshots: NativeWindowCustomShaderResidencySnapshots,
    ) -> Option<NativeAdapterCustomShaderResidencyAccountToken> {
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
        let next = NativeAdapterCustomShaderResidencyAccountToken {
            window_identity: token.window_identity.clone(),
            account_generation: token.account_generation,
            adapter_generation,
        };
        self.recompute_aggregate();
        Some(next)
    }

    fn remove(&mut self, token: &NativeAdapterCustomShaderResidencyAccountToken) -> bool {
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

    fn profile(&self) -> NativeAdapterCustomShaderResidencyProfile {
        NativeAdapterCustomShaderResidencyProfile {
            adapter_generation: self
                .current_adapter_generation
                .is_known()
                .then_some(self.current_adapter_generation),
            active_pipeline_resident_count: self.aggregate.active_pipeline_resident_count,
            active_binding_resident_count: self.aggregate.active_binding_resident_count,
            active_surface_uniform_logical_bytes: self
                .aggregate
                .active_surface_uniform_logical_bytes,
            active_app_uniform_logical_bytes: self.aggregate.active_app_uniform_logical_bytes,
            active_storage_logical_bytes: self.aggregate.active_storage_logical_bytes,
            active_presentation_uniform_logical_bytes: self
                .aggregate
                .active_presentation_uniform_logical_bytes,
            quarantined_pipeline_resident_count: self.aggregate.quarantined_pipeline_resident_count,
            quarantined_binding_resident_count: self.aggregate.quarantined_binding_resident_count,
            quarantined_surface_uniform_logical_bytes: self
                .aggregate
                .quarantined_surface_uniform_logical_bytes,
            quarantined_app_uniform_logical_bytes: self
                .aggregate
                .quarantined_app_uniform_logical_bytes,
            quarantined_storage_logical_bytes: self.aggregate.quarantined_storage_logical_bytes,
            quarantined_presentation_uniform_logical_bytes: self
                .aggregate
                .quarantined_presentation_uniform_logical_bytes,
        }
    }

    fn recompute_aggregate(&mut self) {
        self.prune_known_adapter_generations();
        let mut aggregate = NativeAdapterCustomShaderResidencyAggregate {
            active_pipeline_resident_count: Some(0),
            active_binding_resident_count: Some(0),
            active_surface_uniform_logical_bytes: Some(0),
            active_app_uniform_logical_bytes: Some(0),
            active_storage_logical_bytes: Some(0),
            active_presentation_uniform_logical_bytes: Some(0),
            quarantined_pipeline_resident_count: Some(0),
            quarantined_binding_resident_count: Some(0),
            quarantined_surface_uniform_logical_bytes: Some(0),
            quarantined_app_uniform_logical_bytes: Some(0),
            quarantined_storage_logical_bytes: Some(0),
            quarantined_presentation_uniform_logical_bytes: Some(0),
        };
        for account in self.accounts.values() {
            accumulate_custom_shader_active(
                &mut aggregate,
                account.snapshots.active,
                self.current_adapter_generation,
            );
            accumulate_custom_shader_quarantine(
                &mut aggregate,
                account.snapshots.quarantine_0,
                &self.known_adapter_generations,
            );
            accumulate_custom_shader_quarantine(
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

fn custom_shader_account_references_adapter_generation(
    account: &NativeAdapterCustomShaderResidencyAccount,
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

fn accumulate_custom_shader_active(
    aggregate: &mut NativeAdapterCustomShaderResidencyAggregate,
    snapshot: Option<GpuSurfaceCustomShaderResidencySnapshot>,
    current_adapter_generation: NativeAdapterGeneration,
) {
    let Some(snapshot) = snapshot else {
        return;
    };
    if snapshot.generation != current_adapter_generation || !current_adapter_generation.is_known() {
        aggregate.active_pipeline_resident_count = None;
        aggregate.active_binding_resident_count = None;
        aggregate.active_surface_uniform_logical_bytes = None;
        aggregate.active_app_uniform_logical_bytes = None;
        aggregate.active_storage_logical_bytes = None;
        aggregate.active_presentation_uniform_logical_bytes = None;
        return;
    }
    add_count(
        &mut aggregate.active_pipeline_resident_count,
        snapshot.pipeline_resident_count,
    );
    add_count(
        &mut aggregate.active_binding_resident_count,
        snapshot.binding_resident_count,
    );
    add_bytes(
        &mut aggregate.active_surface_uniform_logical_bytes,
        snapshot.surface_uniform_logical_bytes,
    );
    add_bytes(
        &mut aggregate.active_app_uniform_logical_bytes,
        snapshot.app_uniform_logical_bytes,
    );
    add_bytes(
        &mut aggregate.active_storage_logical_bytes,
        snapshot.storage_logical_bytes,
    );
    add_bytes(
        &mut aggregate.active_presentation_uniform_logical_bytes,
        snapshot.presentation_uniform_logical_bytes,
    );
}

fn accumulate_custom_shader_quarantine(
    aggregate: &mut NativeAdapterCustomShaderResidencyAggregate,
    snapshot: Option<GpuSurfaceCustomShaderResidencySnapshot>,
    known_adapter_generations: &[NativeAdapterGeneration],
) {
    let Some(snapshot) = snapshot else {
        return;
    };
    if !snapshot.generation.is_known() || !known_adapter_generations.contains(&snapshot.generation)
    {
        aggregate.quarantined_pipeline_resident_count = None;
        aggregate.quarantined_binding_resident_count = None;
        aggregate.quarantined_surface_uniform_logical_bytes = None;
        aggregate.quarantined_app_uniform_logical_bytes = None;
        aggregate.quarantined_storage_logical_bytes = None;
        aggregate.quarantined_presentation_uniform_logical_bytes = None;
        return;
    }
    add_count(
        &mut aggregate.quarantined_pipeline_resident_count,
        snapshot.pipeline_resident_count,
    );
    add_count(
        &mut aggregate.quarantined_binding_resident_count,
        snapshot.binding_resident_count,
    );
    add_bytes(
        &mut aggregate.quarantined_surface_uniform_logical_bytes,
        snapshot.surface_uniform_logical_bytes,
    );
    add_bytes(
        &mut aggregate.quarantined_app_uniform_logical_bytes,
        snapshot.app_uniform_logical_bytes,
    );
    add_bytes(
        &mut aggregate.quarantined_storage_logical_bytes,
        snapshot.storage_logical_bytes,
    );
    add_bytes(
        &mut aggregate.quarantined_presentation_uniform_logical_bytes,
        snapshot.presentation_uniform_logical_bytes,
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NativeAdapterRenderCanvasUploadCandidateAggregate {
    observed_candidate_plan_count: Option<usize>,
    observed_candidate_plan_window_count: Option<usize>,
    observed_candidate_no_work_count: Option<usize>,
    observed_candidate_exact_count: Option<usize>,
    observed_candidate_invalid_count: Option<usize>,
    observed_candidate_unsupported_count: Option<usize>,
    observed_candidate_incomplete_count: Option<usize>,
    observed_candidate_overflow_count: Option<usize>,
    observed_candidate_exact_immutable_payload_operations: Option<usize>,
    observed_candidate_exact_immutable_payload_logical_bytes: Option<u64>,
    observed_candidate_exact_volatile_payload_operations: Option<usize>,
    observed_candidate_exact_volatile_payload_logical_bytes: Option<u64>,
    observed_candidate_exact_renderer_parameter_operations: Option<usize>,
    observed_candidate_exact_renderer_parameter_logical_bytes: Option<u64>,
}

impl Default for NativeAdapterRenderCanvasUploadCandidateAggregate {
    fn default() -> Self {
        Self {
            observed_candidate_plan_count: Some(0),
            observed_candidate_plan_window_count: Some(0),
            observed_candidate_no_work_count: Some(0),
            observed_candidate_exact_count: Some(0),
            observed_candidate_invalid_count: Some(0),
            observed_candidate_unsupported_count: Some(0),
            observed_candidate_incomplete_count: Some(0),
            observed_candidate_overflow_count: Some(0),
            observed_candidate_exact_immutable_payload_operations: Some(0),
            observed_candidate_exact_immutable_payload_logical_bytes: Some(0),
            observed_candidate_exact_volatile_payload_operations: Some(0),
            observed_candidate_exact_volatile_payload_logical_bytes: Some(0),
            observed_candidate_exact_renderer_parameter_operations: Some(0),
            observed_candidate_exact_renderer_parameter_logical_bytes: Some(0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NativeAdapterRenderCanvasUploadAggregate {
    candidate: NativeAdapterRenderCanvasUploadCandidateAggregate,
    immutable_payload_operations: Option<usize>,
    immutable_payload_logical_bytes: Option<u64>,
    volatile_payload_operations: Option<usize>,
    volatile_payload_logical_bytes: Option<u64>,
    renderer_parameter_operations: Option<usize>,
    renderer_parameter_logical_bytes: Option<u64>,
}

impl Default for NativeAdapterRenderCanvasUploadAggregate {
    fn default() -> Self {
        Self {
            candidate: NativeAdapterRenderCanvasUploadCandidateAggregate::default(),
            immutable_payload_operations: Some(0),
            immutable_payload_logical_bytes: Some(0),
            volatile_payload_operations: Some(0),
            volatile_payload_logical_bytes: Some(0),
            renderer_parameter_operations: Some(0),
            renderer_parameter_logical_bytes: Some(0),
        }
    }
}

struct NativeAdapterRenderCanvasUploadAccount {
    account_generation: u64,
    adapter_generation: NativeAdapterGeneration,
    totals: NativeAdapterRenderCanvasUploadAggregate,
    has_observed_candidate_plan: bool,
    last_contributed_frame_sequence: Option<u64>,
}

/// Application-scope, crate-private render-canvas upload evidence owned by the
/// selected adapter. Window runners contribute only after a successful native
/// present ticket; lifecycle boundaries own account registration and fencing.
pub(super) struct NativeAdapterRenderCanvasUploadLedger {
    accounts: HashMap<NativeAtlasResidencyWindowIdentity, NativeAdapterRenderCanvasUploadAccount>,
    next_account_generation: Option<u64>,
    current_adapter_generation: NativeAdapterGeneration,
    aggregate: NativeAdapterRenderCanvasUploadAggregate,
}

impl Default for NativeAdapterRenderCanvasUploadLedger {
    fn default() -> Self {
        Self {
            accounts: HashMap::new(),
            next_account_generation: Some(1),
            current_adapter_generation: NativeAdapterGeneration::default(),
            aggregate: NativeAdapterRenderCanvasUploadAggregate::default(),
        }
    }
}

impl NativeAdapterRenderCanvasUploadLedger {
    fn allocate_account_generation(&mut self) -> Option<u64> {
        let generation = self.next_account_generation?;
        self.next_account_generation = generation.checked_add(1);
        Some(generation)
    }

    fn record_adapter_generation(&mut self, generation: NativeAdapterGeneration) {
        self.current_adapter_generation = generation;
        self.recompute_aggregate();
    }

    fn register(
        &mut self,
        window_identity: NativeAtlasResidencyWindowIdentity,
        adapter_generation: NativeAdapterGeneration,
    ) -> Option<NativeAdapterRenderCanvasUploadAccountToken> {
        if !self.is_current_known_adapter_generation(adapter_generation)
            || self.accounts.contains_key(&window_identity)
        {
            return None;
        }
        let account_generation = self.allocate_account_generation()?;
        let token = NativeAdapterRenderCanvasUploadAccountToken {
            window_identity: window_identity.clone(),
            account_generation,
            adapter_generation,
        };
        self.accounts.insert(
            window_identity,
            NativeAdapterRenderCanvasUploadAccount {
                account_generation,
                adapter_generation,
                totals: NativeAdapterRenderCanvasUploadAggregate::default(),
                has_observed_candidate_plan: false,
                last_contributed_frame_sequence: None,
            },
        );
        Some(token)
    }

    fn update(&self, token: &NativeAdapterRenderCanvasUploadAccountToken) -> bool {
        let Some(account) = self.accounts.get(&token.window_identity) else {
            return false;
        };
        self.account_is_current(account, token)
    }

    fn rebind(
        &mut self,
        token: &NativeAdapterRenderCanvasUploadAccountToken,
        adapter_generation: NativeAdapterGeneration,
    ) -> Option<NativeAdapterRenderCanvasUploadAccountToken> {
        if !self.is_current_known_adapter_generation(adapter_generation) {
            return None;
        }
        let account = self.accounts.get_mut(&token.window_identity)?;
        if account.account_generation != token.account_generation
            || account.adapter_generation != token.adapter_generation
        {
            return None;
        }
        account.adapter_generation = adapter_generation;
        account.totals = NativeAdapterRenderCanvasUploadAggregate::default();
        account.has_observed_candidate_plan = false;
        account.last_contributed_frame_sequence = None;
        let next = NativeAdapterRenderCanvasUploadAccountToken {
            window_identity: token.window_identity.clone(),
            account_generation: token.account_generation,
            adapter_generation,
        };
        self.recompute_aggregate();
        Some(next)
    }

    fn remove(&mut self, token: &NativeAdapterRenderCanvasUploadAccountToken) -> bool {
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

    fn contribute(
        &mut self,
        token: &NativeAdapterRenderCanvasUploadAccountToken,
        frame_sequence: u64,
        stats: GpuSurfaceRenderCanvasUploadStats,
        candidate_plan: Option<GpuSurfaceRenderCanvasUploadPlan>,
        current_plan_context: Option<GpuSurfaceRenderCanvasUploadPlanContext>,
    ) -> bool {
        let current_adapter_generation = self.current_adapter_generation;
        let candidate_observation = candidate_plan
            .zip(current_plan_context)
            .and_then(|(plan, current)| plan.matches_context(current).then_some(plan))
            .map(|plan| plan.observation());
        let mut candidate_window_was_new = false;
        {
            let Some(account) = self.accounts.get_mut(&token.window_identity) else {
                return false;
            };
            if account.account_generation != token.account_generation
                || account.adapter_generation != token.adapter_generation
                || !current_adapter_generation.is_known()
                || token.adapter_generation != current_adapter_generation
                || account
                    .last_contributed_frame_sequence
                    .is_some_and(|last| frame_sequence <= last)
            {
                return false;
            }

            account.last_contributed_frame_sequence = Some(frame_sequence);
            accumulate_render_canvas_uploads(&mut account.totals, stats);
            if let Some(observation) = candidate_observation {
                if !account.has_observed_candidate_plan {
                    account.has_observed_candidate_plan = true;
                    candidate_window_was_new = true;
                    add_count(
                        &mut account
                            .totals
                            .candidate
                            .observed_candidate_plan_window_count,
                        1,
                    );
                }
                accumulate_render_canvas_upload_candidate_observation(
                    &mut account.totals.candidate,
                    observation,
                );
            }
        }
        accumulate_render_canvas_uploads(&mut self.aggregate, stats);
        if let Some(observation) = candidate_observation {
            if candidate_window_was_new {
                add_count(
                    &mut self
                        .aggregate
                        .candidate
                        .observed_candidate_plan_window_count,
                    1,
                );
            }
            accumulate_render_canvas_upload_candidate_observation(
                &mut self.aggregate.candidate,
                observation,
            );
        }
        true
    }

    fn is_current_known_adapter_generation(&self, generation: NativeAdapterGeneration) -> bool {
        generation.is_known() && generation == self.current_adapter_generation
    }

    fn account_is_current(
        &self,
        account: &NativeAdapterRenderCanvasUploadAccount,
        token: &NativeAdapterRenderCanvasUploadAccountToken,
    ) -> bool {
        account.account_generation == token.account_generation
            && account.adapter_generation == token.adapter_generation
            && self.is_current_known_adapter_generation(token.adapter_generation)
    }

    fn profile(&self) -> NativeAdapterRenderCanvasUploadProfile {
        let candidate = self.aggregate.candidate;
        NativeAdapterRenderCanvasUploadProfile {
            adapter_generation: self
                .current_adapter_generation
                .is_known()
                .then_some(self.current_adapter_generation),
            observed_candidate_plan_count: candidate.observed_candidate_plan_count,
            observed_candidate_plan_window_count: candidate.observed_candidate_plan_window_count,
            observed_candidate_no_work_count: candidate.observed_candidate_no_work_count,
            observed_candidate_exact_count: candidate.observed_candidate_exact_count,
            observed_candidate_invalid_count: candidate.observed_candidate_invalid_count,
            observed_candidate_unsupported_count: candidate.observed_candidate_unsupported_count,
            observed_candidate_incomplete_count: candidate.observed_candidate_incomplete_count,
            observed_candidate_overflow_count: candidate.observed_candidate_overflow_count,
            observed_candidate_exact_immutable_payload_operations: candidate
                .observed_candidate_exact_immutable_payload_operations,
            observed_candidate_exact_immutable_payload_logical_bytes: candidate
                .observed_candidate_exact_immutable_payload_logical_bytes,
            observed_candidate_exact_volatile_payload_operations: candidate
                .observed_candidate_exact_volatile_payload_operations,
            observed_candidate_exact_volatile_payload_logical_bytes: candidate
                .observed_candidate_exact_volatile_payload_logical_bytes,
            observed_candidate_exact_renderer_parameter_operations: candidate
                .observed_candidate_exact_renderer_parameter_operations,
            observed_candidate_exact_renderer_parameter_logical_bytes: candidate
                .observed_candidate_exact_renderer_parameter_logical_bytes,
            immutable_payload_operations: self.aggregate.immutable_payload_operations,
            immutable_payload_logical_bytes: self.aggregate.immutable_payload_logical_bytes,
            volatile_payload_operations: self.aggregate.volatile_payload_operations,
            volatile_payload_logical_bytes: self.aggregate.volatile_payload_logical_bytes,
            renderer_parameter_operations: self.aggregate.renderer_parameter_operations,
            renderer_parameter_logical_bytes: self.aggregate.renderer_parameter_logical_bytes,
        }
    }

    fn recompute_aggregate(&mut self) {
        let mut aggregate = NativeAdapterRenderCanvasUploadAggregate::default();
        if self.current_adapter_generation.is_known() {
            for account in self.accounts.values() {
                if account.adapter_generation == self.current_adapter_generation {
                    accumulate_render_canvas_upload_aggregate(&mut aggregate, account.totals);
                }
            }
        }
        self.aggregate = aggregate;
    }

    #[cfg(test)]
    fn account_count(&self) -> usize {
        self.accounts.len()
    }
}

fn accumulate_render_canvas_uploads(
    total: &mut NativeAdapterRenderCanvasUploadAggregate,
    contribution: GpuSurfaceRenderCanvasUploadStats,
) {
    add_optional_count(
        &mut total.immutable_payload_operations,
        contribution.immutable_payload.operations,
    );
    add_bytes(
        &mut total.immutable_payload_logical_bytes,
        contribution.immutable_payload.logical_bytes,
    );
    add_optional_count(
        &mut total.volatile_payload_operations,
        contribution.volatile_payload.operations,
    );
    add_bytes(
        &mut total.volatile_payload_logical_bytes,
        contribution.volatile_payload.logical_bytes,
    );
    add_optional_count(
        &mut total.renderer_parameter_operations,
        contribution.renderer_parameter.operations,
    );
    add_bytes(
        &mut total.renderer_parameter_logical_bytes,
        contribution.renderer_parameter.logical_bytes,
    );
}

fn accumulate_render_canvas_upload_candidate_observation(
    total: &mut NativeAdapterRenderCanvasUploadCandidateAggregate,
    contribution: GpuSurfaceRenderCanvasUploadPlanObservation,
) {
    add_count(&mut total.observed_candidate_plan_count, 1);
    match contribution {
        GpuSurfaceRenderCanvasUploadPlanObservation::NoWork => {
            add_count(&mut total.observed_candidate_no_work_count, 1);
        }
        GpuSurfaceRenderCanvasUploadPlanObservation::Exact(stats) => {
            add_count(&mut total.observed_candidate_exact_count, 1);
            let [
                (immutable_payload_operations, immutable_payload_logical_bytes),
                (volatile_payload_operations, volatile_payload_logical_bytes),
                (renderer_parameter_operations, renderer_parameter_logical_bytes),
            ] = stats.values();
            add_count(
                &mut total.observed_candidate_exact_immutable_payload_operations,
                immutable_payload_operations,
            );
            add_bytes(
                &mut total.observed_candidate_exact_immutable_payload_logical_bytes,
                Some(immutable_payload_logical_bytes),
            );
            add_count(
                &mut total.observed_candidate_exact_volatile_payload_operations,
                volatile_payload_operations,
            );
            add_bytes(
                &mut total.observed_candidate_exact_volatile_payload_logical_bytes,
                Some(volatile_payload_logical_bytes),
            );
            add_count(
                &mut total.observed_candidate_exact_renderer_parameter_operations,
                renderer_parameter_operations,
            );
            add_bytes(
                &mut total.observed_candidate_exact_renderer_parameter_logical_bytes,
                Some(renderer_parameter_logical_bytes),
            );
        }
        GpuSurfaceRenderCanvasUploadPlanObservation::Unavailable(reason) => match reason {
            super::gpu_surface::GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid => {
                add_count(&mut total.observed_candidate_invalid_count, 1);
            }
            super::gpu_surface::GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Unsupported => {
                add_count(&mut total.observed_candidate_unsupported_count, 1);
            }
            super::gpu_surface::GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete => {
                add_count(&mut total.observed_candidate_incomplete_count, 1);
            }
            super::gpu_surface::GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Overflow => {
                add_count(&mut total.observed_candidate_overflow_count, 1);
            }
        },
    }
}

fn accumulate_render_canvas_upload_candidate_aggregate(
    total: &mut NativeAdapterRenderCanvasUploadCandidateAggregate,
    contribution: NativeAdapterRenderCanvasUploadCandidateAggregate,
) {
    add_optional_count(
        &mut total.observed_candidate_plan_count,
        contribution.observed_candidate_plan_count,
    );
    add_optional_count(
        &mut total.observed_candidate_plan_window_count,
        contribution.observed_candidate_plan_window_count,
    );
    add_optional_count(
        &mut total.observed_candidate_no_work_count,
        contribution.observed_candidate_no_work_count,
    );
    add_optional_count(
        &mut total.observed_candidate_exact_count,
        contribution.observed_candidate_exact_count,
    );
    add_optional_count(
        &mut total.observed_candidate_invalid_count,
        contribution.observed_candidate_invalid_count,
    );
    add_optional_count(
        &mut total.observed_candidate_unsupported_count,
        contribution.observed_candidate_unsupported_count,
    );
    add_optional_count(
        &mut total.observed_candidate_incomplete_count,
        contribution.observed_candidate_incomplete_count,
    );
    add_optional_count(
        &mut total.observed_candidate_overflow_count,
        contribution.observed_candidate_overflow_count,
    );
    add_optional_count(
        &mut total.observed_candidate_exact_immutable_payload_operations,
        contribution.observed_candidate_exact_immutable_payload_operations,
    );
    add_bytes(
        &mut total.observed_candidate_exact_immutable_payload_logical_bytes,
        contribution.observed_candidate_exact_immutable_payload_logical_bytes,
    );
    add_optional_count(
        &mut total.observed_candidate_exact_volatile_payload_operations,
        contribution.observed_candidate_exact_volatile_payload_operations,
    );
    add_bytes(
        &mut total.observed_candidate_exact_volatile_payload_logical_bytes,
        contribution.observed_candidate_exact_volatile_payload_logical_bytes,
    );
    add_optional_count(
        &mut total.observed_candidate_exact_renderer_parameter_operations,
        contribution.observed_candidate_exact_renderer_parameter_operations,
    );
    add_bytes(
        &mut total.observed_candidate_exact_renderer_parameter_logical_bytes,
        contribution.observed_candidate_exact_renderer_parameter_logical_bytes,
    );
}

fn accumulate_render_canvas_upload_aggregate(
    total: &mut NativeAdapterRenderCanvasUploadAggregate,
    contribution: NativeAdapterRenderCanvasUploadAggregate,
) {
    accumulate_render_canvas_upload_candidate_aggregate(
        &mut total.candidate,
        contribution.candidate,
    );
    add_optional_count(
        &mut total.immutable_payload_operations,
        contribution.immutable_payload_operations,
    );
    add_bytes(
        &mut total.immutable_payload_logical_bytes,
        contribution.immutable_payload_logical_bytes,
    );
    add_optional_count(
        &mut total.volatile_payload_operations,
        contribution.volatile_payload_operations,
    );
    add_bytes(
        &mut total.volatile_payload_logical_bytes,
        contribution.volatile_payload_logical_bytes,
    );
    add_optional_count(
        &mut total.renderer_parameter_operations,
        contribution.renderer_parameter_operations,
    );
    add_bytes(
        &mut total.renderer_parameter_logical_bytes,
        contribution.renderer_parameter_logical_bytes,
    );
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
    signal_residency: NativeAdapterSignalResidencyLedger,
    custom_shader_residency: NativeAdapterCustomShaderResidencyLedger,
    render_canvas_upload_ledger: NativeAdapterRenderCanvasUploadLedger,
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
            signal_residency: NativeAdapterSignalResidencyLedger::default(),
            custom_shader_residency: NativeAdapterCustomShaderResidencyLedger::default(),
            render_canvas_upload_ledger: NativeAdapterRenderCanvasUploadLedger::default(),
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
        self.signal_residency.record_adapter_generation(generation);
        self.custom_shader_residency
            .record_adapter_generation(generation);
        self.render_canvas_upload_ledger
            .record_adapter_generation(generation);
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

    pub(super) fn register_signal_residency_account(
        &mut self,
        window_identity: NativeAtlasResidencyWindowIdentity,
        adapter_generation: NativeAdapterGeneration,
        snapshots: NativeWindowSignalResidencySnapshots,
    ) -> Option<NativeAdapterSignalResidencyAccountToken> {
        self.signal_residency
            .register(window_identity, adapter_generation, snapshots)
    }

    pub(super) fn update_signal_residency_account(
        &mut self,
        token: &NativeAdapterSignalResidencyAccountToken,
        snapshots: NativeWindowSignalResidencySnapshots,
    ) -> bool {
        self.signal_residency.update(token, snapshots)
    }

    pub(super) fn rebind_signal_residency_account(
        &mut self,
        token: &NativeAdapterSignalResidencyAccountToken,
        adapter_generation: NativeAdapterGeneration,
        snapshots: NativeWindowSignalResidencySnapshots,
    ) -> Option<NativeAdapterSignalResidencyAccountToken> {
        self.signal_residency
            .rebind(token, adapter_generation, snapshots)
    }

    pub(super) fn remove_signal_residency_account(
        &mut self,
        token: &NativeAdapterSignalResidencyAccountToken,
    ) -> bool {
        self.signal_residency.remove(token)
    }

    pub(super) fn capture_signal_residency_profile(&self) -> NativeAdapterSignalResidencyProfile {
        self.signal_residency.profile()
    }

    pub(super) fn register_custom_shader_residency_account(
        &mut self,
        window_identity: NativeAtlasResidencyWindowIdentity,
        adapter_generation: NativeAdapterGeneration,
        snapshots: NativeWindowCustomShaderResidencySnapshots,
    ) -> Option<NativeAdapterCustomShaderResidencyAccountToken> {
        self.custom_shader_residency
            .register(window_identity, adapter_generation, snapshots)
    }

    pub(super) fn update_custom_shader_residency_account(
        &mut self,
        token: &NativeAdapterCustomShaderResidencyAccountToken,
        snapshots: NativeWindowCustomShaderResidencySnapshots,
    ) -> bool {
        self.custom_shader_residency.update(token, snapshots)
    }

    pub(super) fn rebind_custom_shader_residency_account(
        &mut self,
        token: &NativeAdapterCustomShaderResidencyAccountToken,
        adapter_generation: NativeAdapterGeneration,
        snapshots: NativeWindowCustomShaderResidencySnapshots,
    ) -> Option<NativeAdapterCustomShaderResidencyAccountToken> {
        self.custom_shader_residency
            .rebind(token, adapter_generation, snapshots)
    }

    pub(super) fn remove_custom_shader_residency_account(
        &mut self,
        token: &NativeAdapterCustomShaderResidencyAccountToken,
    ) -> bool {
        self.custom_shader_residency.remove(token)
    }

    pub(super) fn capture_custom_shader_residency_profile(
        &self,
    ) -> NativeAdapterCustomShaderResidencyProfile {
        self.custom_shader_residency.profile()
    }

    pub(super) fn register_render_canvas_upload_account(
        &mut self,
        window_identity: NativeAtlasResidencyWindowIdentity,
        adapter_generation: NativeAdapterGeneration,
    ) -> Option<NativeAdapterRenderCanvasUploadAccountToken> {
        self.render_canvas_upload_ledger
            .register(window_identity, adapter_generation)
    }

    pub(super) fn update_render_canvas_upload_account(
        &self,
        token: &NativeAdapterRenderCanvasUploadAccountToken,
    ) -> bool {
        self.render_canvas_upload_ledger.update(token)
    }

    pub(super) fn rebind_render_canvas_upload_account(
        &mut self,
        token: &NativeAdapterRenderCanvasUploadAccountToken,
        adapter_generation: NativeAdapterGeneration,
    ) -> Option<NativeAdapterRenderCanvasUploadAccountToken> {
        self.render_canvas_upload_ledger
            .rebind(token, adapter_generation)
    }

    pub(super) fn remove_render_canvas_upload_account(
        &mut self,
        token: &NativeAdapterRenderCanvasUploadAccountToken,
    ) -> bool {
        self.render_canvas_upload_ledger.remove(token)
    }

    pub(super) fn contribute_render_canvas_uploads(
        &mut self,
        token: &NativeAdapterRenderCanvasUploadAccountToken,
        frame_sequence: u64,
        stats: GpuSurfaceRenderCanvasUploadStats,
        candidate_plan: Option<GpuSurfaceRenderCanvasUploadPlan>,
        current_plan_context: Option<GpuSurfaceRenderCanvasUploadPlanContext>,
    ) -> bool {
        self.render_canvas_upload_ledger.contribute(
            token,
            frame_sequence,
            stats,
            candidate_plan,
            current_plan_context,
        )
    }

    pub(super) fn capture_render_canvas_upload_profile(
        &self,
    ) -> NativeAdapterRenderCanvasUploadProfile {
        self.render_canvas_upload_ledger.profile()
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

    pub(super) fn adopt_signal_residency_ledger(
        &mut self,
        previous: &mut GenericNativeAdapterOwner,
    ) {
        let next_generation = self.capture_generation();
        self.signal_residency = std::mem::take(&mut previous.signal_residency);
        if let Some(next_generation) = next_generation {
            self.signal_residency
                .record_adapter_generation(next_generation);
        }
    }

    pub(super) fn adopt_custom_shader_residency_ledger(
        &mut self,
        previous: &mut GenericNativeAdapterOwner,
    ) {
        let next_generation = self.capture_generation();
        self.custom_shader_residency = std::mem::take(&mut previous.custom_shader_residency);
        if let Some(next_generation) = next_generation {
            self.custom_shader_residency
                .record_adapter_generation(next_generation);
        }
    }

    pub(super) fn adopt_render_canvas_upload_ledger(
        &mut self,
        previous: &mut GenericNativeAdapterOwner,
    ) {
        let next_generation = self.capture_generation();
        self.render_canvas_upload_ledger =
            std::mem::take(&mut previous.render_canvas_upload_ledger);
        if let Some(next_generation) = next_generation {
            self.render_canvas_upload_ledger
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

    /// Reconfigure the selected WGPU surface without replacing the target
    /// texture/view owned by its render surface.
    pub(super) fn reconfigure_render_surface_in_place(
        &self,
        surface: &mut RenderSurface<'_>,
    ) -> bool {
        let Some(device) = self.device_handle_for_surface(surface) else {
            return false;
        };
        surface.surface.configure(&device.device, &surface.config);
        true
    }

    /// Resize a recovery candidate before it is published.  The candidate has
    /// no active resource bundle yet, so it intentionally has no retirement
    /// owner or completion evidence to pass to the active replacement path.
    pub(super) fn resize_unpublished_recovery_candidate_surface(
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

    pub(super) fn replace_render_surface_targets(
        &self,
        surface: &mut RenderSurface<'_>,
        predecessor: &mut Option<NativeRenderTargetRetirement>,
        evidence: Option<NativeRenderTargetReplacementEvidence>,
        width: u32,
        height: u32,
        mode: NativeRenderTargetReplacementMode,
    ) -> NativeRenderTargetReplacementOutcome {
        let selected_device_id = self.selected.as_ref().map(|selected| selected.device_id);
        let selected_generation = self.capture_generation();
        let outcome = replacement_preflight(
            NativeRenderTargetReplacementRequest {
                mode,
                current_width: surface.config.width,
                current_height: surface.config.height,
                width,
                height,
                predecessor_occupied: predecessor.is_some(),
                evidence,
            },
            NativeRenderTargetReplacementContext {
                surface_device_id: surface.dev_id,
                selected_device_id,
                selected_generation,
            },
        );
        let NativeRenderTargetReplacementOutcome::Committed {
            next_target_generation,
        } = outcome
        else {
            return outcome;
        };
        let Some(device_id) = selected_device_id else {
            return NativeRenderTargetReplacementOutcome::Deferred;
        };
        let Some(context) = self.render_context.as_ref() else {
            return NativeRenderTargetReplacementOutcome::Deferred;
        };
        let Some(device) = context.device_handle(device_id) else {
            return NativeRenderTargetReplacementOutcome::Deferred;
        };
        let Some(evidence) = evidence else {
            return NativeRenderTargetReplacementOutcome::Deferred;
        };
        let (target_texture, target_view) = create_targets(width, height, &device.device);
        let old_texture = std::mem::replace(&mut surface.target_texture, target_texture);
        let old_view = std::mem::replace(&mut surface.target_view, target_view);
        let old_width = surface.config.width;
        let old_height = surface.config.height;
        *predecessor = Some(NativeRenderTargetRetirement::new(
            old_texture,
            old_view,
            retirement_identity(evidence, old_width, old_height),
        ));
        surface.config.width = width;
        surface.config.height = height;
        surface.surface.configure(&device.device, &surface.config);
        NativeRenderTargetReplacementOutcome::Committed {
            next_target_generation,
        }
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
            signal_residency: NativeAdapterSignalResidencyLedger::default(),
            custom_shader_residency: NativeAdapterCustomShaderResidencyLedger::default(),
            render_canvas_upload_ledger: NativeAdapterRenderCanvasUploadLedger::default(),
        };
        owner.atlas_residency.record_adapter_generation(generation);
        owner.signal_residency.record_adapter_generation(generation);
        owner
            .custom_shader_residency
            .record_adapter_generation(generation);
        owner
            .render_canvas_upload_ledger
            .record_adapter_generation(generation);
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
            signal_residency: NativeAdapterSignalResidencyLedger::default(),
            custom_shader_residency: NativeAdapterCustomShaderResidencyLedger::default(),
            render_canvas_upload_ledger: NativeAdapterRenderCanvasUploadLedger::default(),
        };
        owner.atlas_residency.record_adapter_generation(generation);
        owner.signal_residency.record_adapter_generation(generation);
        owner
            .custom_shader_residency
            .record_adapter_generation(generation);
        owner
            .render_canvas_upload_ledger
            .record_adapter_generation(generation);
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
        GpuSurfaceAtlasResidencySnapshot, GpuSurfaceCustomShaderResidencySnapshot,
        GpuSurfaceRenderCanvasUploadStats, GpuSurfaceSignalResidencySnapshot,
        NativeAdapterAtlasResidencyAccountToken, NativeAdapterAtlasResidencyLedger,
        NativeAdapterCustomShaderResidencyAccountToken, NativeAdapterCustomShaderResidencyLedger,
        NativeAdapterCustomShaderResidencyProfile, NativeAdapterGeneration,
        NativeAdapterRenderCanvasUploadLedger, NativeAdapterSignalResidencyAccountToken,
        NativeAdapterSignalResidencyLedger, NativeAdapterSignalResidencyProfile,
        NativeAtlasResidencyWindowIdentity, NativeWindowAtlasResidencySnapshots,
        NativeWindowCustomShaderResidencySnapshots, NativeWindowSignalResidencySnapshots,
        auxiliary_backend_policy_is_compatible, device_loss_registration_matches,
        render_surface_creation_error,
    };
    use crate::gui_runtime::NativeGpuBackend;
    use crate::gui_runtime::native_vello::generic_runtime::closing::NativeLifecycle;
    use crate::gui_runtime::native_vello::generic_runtime::gpu_surface::{
        GpuSurfaceRenderCanvasUploadPlan, GpuSurfaceRenderCanvasUploadPlanContext,
        GpuSurfaceRenderCanvasUploadPlanUnavailableReason, GpuSurfaceRenderCanvasUploadTarget,
    };
    use crate::gui_runtime::native_vello::generic_runtime::native_encode_present::{
        NativeEncodePresentPath, NativeEncodePresentPlanContext,
    };
    use crate::gui_runtime::native_vello::generic_runtime::native_visual_packet::{
        NativeVisualRequestAdapter, NativeVisualRequestBegin, NativeVisualRequestMailbox,
    };
    use crate::gui_runtime::native_vello::generic_runtime::{
        FrameWork, runner_state::NativeTargetGeneration,
    };
    use std::{num::NonZeroU64, sync::Arc};
    use vello::wgpu;
    use winit::window::WindowId;

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

    fn signal_snapshot(
        generation: NativeAdapterGeneration,
        signal_buffer_resident_count: usize,
        signal_buffer_logical_bytes: Option<u64>,
        signal_body_texture_resident_count: usize,
        signal_body_texture_logical_rgba_bytes: Option<u64>,
    ) -> GpuSurfaceSignalResidencySnapshot {
        GpuSurfaceSignalResidencySnapshot {
            generation,
            signal_buffer_resident_count,
            signal_buffer_logical_bytes,
            signal_body_texture_resident_count,
            signal_body_texture_logical_rgba_bytes,
        }
    }

    fn signal_snapshots(
        active: Option<GpuSurfaceSignalResidencySnapshot>,
        quarantine_0: Option<GpuSurfaceSignalResidencySnapshot>,
        quarantine_1: Option<GpuSurfaceSignalResidencySnapshot>,
    ) -> NativeWindowSignalResidencySnapshots {
        NativeWindowSignalResidencySnapshots {
            active,
            quarantine_0,
            quarantine_1,
        }
    }

    fn custom_shader_snapshot(
        generation: NativeAdapterGeneration,
        pipeline_resident_count: usize,
        binding_resident_count: usize,
        surface_uniform_logical_bytes: Option<u64>,
        app_uniform_logical_bytes: Option<u64>,
        storage_logical_bytes: Option<u64>,
        presentation_uniform_logical_bytes: Option<u64>,
    ) -> GpuSurfaceCustomShaderResidencySnapshot {
        GpuSurfaceCustomShaderResidencySnapshot {
            generation,
            pipeline_resident_count,
            binding_resident_count,
            surface_uniform_logical_bytes,
            app_uniform_logical_bytes,
            storage_logical_bytes,
            presentation_uniform_logical_bytes,
        }
    }

    fn custom_shader_snapshots(
        active: Option<GpuSurfaceCustomShaderResidencySnapshot>,
        quarantine_0: Option<GpuSurfaceCustomShaderResidencySnapshot>,
        quarantine_1: Option<GpuSurfaceCustomShaderResidencySnapshot>,
    ) -> NativeWindowCustomShaderResidencySnapshots {
        NativeWindowCustomShaderResidencySnapshots {
            active,
            quarantine_0,
            quarantine_1,
        }
    }

    fn upload_stats(
        immutable_payload: (Option<usize>, Option<u64>),
        volatile_payload: (Option<usize>, Option<u64>),
        renderer_parameter: (Option<usize>, Option<u64>),
    ) -> GpuSurfaceRenderCanvasUploadStats {
        let mut stats = GpuSurfaceRenderCanvasUploadStats::default();
        stats.immutable_payload.operations = immutable_payload.0;
        stats.immutable_payload.logical_bytes = immutable_payload.1;
        stats.volatile_payload.operations = volatile_payload.0;
        stats.volatile_payload.logical_bytes = volatile_payload.1;
        stats.renderer_parameter.operations = renderer_parameter.0;
        stats.renderer_parameter.logical_bytes = renderer_parameter.1;
        stats
    }

    fn upload_plan_context(
        generation: NativeAdapterGeneration,
        path: NativeEncodePresentPath,
        width: u32,
        height: u32,
    ) -> GpuSurfaceRenderCanvasUploadPlanContext {
        let mut mailbox = NativeVisualRequestMailbox::new();
        let window_id = WindowId::dummy();
        assert!(mailbox.bind_window(window_id));
        let _ = mailbox.enqueue_for_test(FrameWork::None);
        let packet = match NativeVisualRequestAdapter::begin(&mut mailbox, window_id, true) {
            NativeVisualRequestBegin::Requested(packet) => packet.identity(),
            other => panic!("unexpected packet begin: {other:?}"),
        };
        GpuSurfaceRenderCanvasUploadPlanContext::new(
            NativeEncodePresentPlanContext {
                packet,
                adapter_generation: generation,
                target_generation: NativeTargetGeneration::from_test_serial(1),
                lifecycle: NativeLifecycle::default(),
                path,
                snapshot_revision: NonZeroU64::MIN,
            },
            generation,
            GpuSurfaceRenderCanvasUploadTarget::new(
                1,
                wgpu::TextureFormat::Rgba8Unorm,
                width,
                height,
            ),
        )
        .expect("valid upload-plan context")
    }

    fn exact_upload_plan(
        context: GpuSurfaceRenderCanvasUploadPlanContext,
        immutable_payload: usize,
        volatile_payload: usize,
        renderer_parameter: usize,
    ) -> GpuSurfaceRenderCanvasUploadPlan {
        let mut plan = GpuSurfaceRenderCanvasUploadPlan::new(context);
        plan.record_immutable_payload(immutable_payload);
        plan.record_volatile_payload(volatile_payload);
        plan.record_renderer_parameter(renderer_parameter);
        plan
    }

    fn unavailable_upload_plan(
        context: GpuSurfaceRenderCanvasUploadPlanContext,
        reason: GpuSurfaceRenderCanvasUploadPlanUnavailableReason,
    ) -> GpuSurfaceRenderCanvasUploadPlan {
        let mut plan = GpuSurfaceRenderCanvasUploadPlan::new(context);
        plan.mark_unavailable(reason);
        plan
    }

    #[test]
    fn render_canvas_upload_ledger_sums_primary_and_auxiliary_frames_once() {
        let generation = NativeAdapterGeneration::from_test_serial(1);
        let mut ledger = NativeAdapterRenderCanvasUploadLedger::default();
        ledger.record_adapter_generation(generation);
        let primary = ledger
            .register(NativeAtlasResidencyWindowIdentity::Primary, generation)
            .expect("primary upload account should register");
        let auxiliary = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Auxiliary(String::from("inspector")),
                generation,
            )
            .expect("auxiliary upload account should register");

        assert!(ledger.update(&primary));
        assert!(ledger.contribute(
            &primary,
            1,
            upload_stats(
                (Some(2), Some(24)),
                (Some(1), Some(12)),
                (Some(3), Some(48))
            ),
            None,
            None,
        ));
        assert!(ledger.contribute(
            &auxiliary,
            1,
            upload_stats((Some(1), Some(8)), (Some(2), Some(16)), (Some(1), Some(32))),
            None,
            None,
        ));
        assert!(!ledger.contribute(
            &primary,
            1,
            upload_stats((Some(9), Some(9)), (Some(9), Some(9)), (Some(9), Some(9))),
            None,
            None,
        ));
        assert!(!ledger.contribute(
            &primary,
            0,
            GpuSurfaceRenderCanvasUploadStats::default(),
            None,
            None,
        ));
        assert!(ledger.contribute(
            &primary,
            2,
            upload_stats((Some(1), Some(4)), (Some(0), Some(0)), (Some(1), Some(16))),
            None,
            None,
        ));

        let profile = ledger.profile();
        assert_eq!(profile.adapter_generation, Some(generation));
        assert_eq!(profile.immutable_payload_operations, Some(4));
        assert_eq!(profile.immutable_payload_logical_bytes, Some(36));
        assert_eq!(profile.volatile_payload_operations, Some(3));
        assert_eq!(profile.volatile_payload_logical_bytes, Some(28));
        assert_eq!(profile.renderer_parameter_operations, Some(5));
        assert_eq!(profile.renderer_parameter_logical_bytes, Some(96));
    }

    #[test]
    fn render_canvas_upload_ledger_rebind_resets_generation_and_fences_replacements() {
        let first_generation = NativeAdapterGeneration::from_test_serial(1);
        let second_generation = NativeAdapterGeneration::from_test_serial(2);
        let identity = NativeAtlasResidencyWindowIdentity::Auxiliary(String::from("same-key"));
        let mut ledger = NativeAdapterRenderCanvasUploadLedger::default();
        ledger.record_adapter_generation(first_generation);
        let first = ledger
            .register(identity.clone(), first_generation)
            .expect("first upload account should register");
        assert!(ledger.contribute(
            &first,
            1,
            upload_stats((Some(2), Some(8)), (Some(1), Some(4)), (Some(1), Some(16))),
            None,
            None,
        ));

        ledger.record_adapter_generation(second_generation);
        assert_eq!(ledger.profile().immutable_payload_operations, Some(0));
        assert!(!ledger.update(&first));
        assert!(!ledger.contribute(
            &first,
            2,
            GpuSurfaceRenderCanvasUploadStats::default(),
            None,
            None,
        ));

        let rebound = ledger
            .rebind(&first, second_generation)
            .expect("current account should rebind");
        assert_eq!(ledger.profile().immutable_payload_operations, Some(0));
        assert!(ledger.contribute(
            &rebound,
            1,
            upload_stats((Some(1), Some(3)), (Some(0), Some(0)), (Some(0), Some(0))),
            None,
            None,
        ));
        assert!(!ledger.remove(&first));
        assert!(ledger.remove(&rebound));
        assert_eq!(ledger.account_count(), 0);

        let replacement = ledger
            .register(identity, second_generation)
            .expect("replacement upload account should register");
        assert_ne!(first.account_generation, replacement.account_generation);
        assert!(!ledger.contribute(
            &first,
            3,
            GpuSurfaceRenderCanvasUploadStats::default(),
            None,
            None,
        ));
        assert!(ledger.update(&replacement));
    }

    #[test]
    fn render_canvas_upload_ledger_propagates_unavailable_and_checked_overflow() {
        let generation = NativeAdapterGeneration::from_test_serial(1);
        let mut ledger = NativeAdapterRenderCanvasUploadLedger::default();
        ledger.record_adapter_generation(generation);
        let unavailable = ledger
            .register(NativeAtlasResidencyWindowIdentity::Primary, generation)
            .expect("unavailable upload account should register");
        assert!(ledger.contribute(
            &unavailable,
            1,
            upload_stats((None, Some(7)), (Some(1), None), (Some(2), Some(9))),
            None,
            None,
        ));
        let profile = ledger.profile();
        assert_eq!(profile.immutable_payload_operations, None);
        assert_eq!(profile.immutable_payload_logical_bytes, Some(7));
        assert_eq!(profile.volatile_payload_operations, Some(1));
        assert_eq!(profile.volatile_payload_logical_bytes, None);
        assert_eq!(profile.renderer_parameter_operations, Some(2));
        assert_eq!(profile.renderer_parameter_logical_bytes, Some(9));

        let overflow = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Auxiliary(String::from("overflow")),
                generation,
            )
            .expect("overflow upload account should register");
        assert!(ledger.contribute(
            &overflow,
            1,
            upload_stats(
                (Some(usize::MAX), Some(u64::MAX)),
                (Some(0), Some(0)),
                (Some(0), Some(0)),
            ),
            None,
            None,
        ));
        assert!(ledger.contribute(
            &overflow,
            2,
            upload_stats((Some(1), Some(1)), (Some(0), Some(0)), (Some(0), Some(0)),),
            None,
            None,
        ));
        assert_eq!(ledger.profile().immutable_payload_operations, None);
        assert_eq!(ledger.profile().immutable_payload_logical_bytes, None);
        assert!(ledger.remove(&overflow));
        assert_eq!(ledger.profile().immutable_payload_logical_bytes, Some(7));
    }

    #[test]
    fn render_canvas_upload_candidate_plans_sum_primary_and_auxiliary_exact_frames_once() {
        let generation = NativeAdapterGeneration::from_test_serial(1);
        let context = upload_plan_context(generation, NativeEncodePresentPath::Composited, 64, 32);
        let mut ledger = NativeAdapterRenderCanvasUploadLedger::default();
        ledger.record_adapter_generation(generation);
        let primary = ledger
            .register(NativeAtlasResidencyWindowIdentity::Primary, generation)
            .expect("primary upload account should register");
        let auxiliary = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Auxiliary(String::from("inspector")),
                generation,
            )
            .expect("auxiliary upload account should register");
        let primary_plan = exact_upload_plan(context, 16, 8, 32);
        let auxiliary_plan = exact_upload_plan(context, 4, 2, 8);

        assert!(ledger.contribute(
            &primary,
            1,
            upload_stats((Some(1), Some(16)), (Some(1), Some(8)), (Some(1), Some(32))),
            Some(primary_plan),
            Some(context),
        ));
        assert!(ledger.contribute(
            &auxiliary,
            1,
            upload_stats((Some(1), Some(4)), (Some(1), Some(2)), (Some(1), Some(8))),
            Some(auxiliary_plan),
            Some(context),
        ));
        assert!(!ledger.contribute(
            &primary,
            1,
            GpuSurfaceRenderCanvasUploadStats::default(),
            Some(exact_upload_plan(context, 16, 8, 32)),
            Some(context),
        ));
        assert!(!ledger.contribute(
            &primary,
            0,
            GpuSurfaceRenderCanvasUploadStats::default(),
            Some(exact_upload_plan(context, 16, 8, 32)),
            Some(context),
        ));

        let profile = ledger.profile();
        assert_eq!(profile.observed_candidate_plan_count, Some(2));
        assert_eq!(profile.observed_candidate_plan_window_count, Some(2));
        assert_eq!(profile.observed_candidate_no_work_count, Some(0));
        assert_eq!(profile.observed_candidate_exact_count, Some(2));
        assert_eq!(
            profile.observed_candidate_exact_immutable_payload_operations,
            Some(2)
        );
        assert_eq!(
            profile.observed_candidate_exact_immutable_payload_logical_bytes,
            Some(20)
        );
        assert_eq!(
            profile.observed_candidate_exact_volatile_payload_operations,
            Some(2)
        );
        assert_eq!(
            profile.observed_candidate_exact_volatile_payload_logical_bytes,
            Some(10)
        );
        assert_eq!(
            profile.observed_candidate_exact_renderer_parameter_operations,
            Some(2)
        );
        assert_eq!(
            profile.observed_candidate_exact_renderer_parameter_logical_bytes,
            Some(40)
        );
        assert_eq!(profile.immutable_payload_operations, Some(2));
        assert_eq!(profile.immutable_payload_logical_bytes, Some(20));
    }

    #[test]
    fn render_canvas_upload_candidate_plans_keep_no_work_and_unavailable_buckets_separate() {
        let generation = NativeAdapterGeneration::from_test_serial(1);
        let context = upload_plan_context(generation, NativeEncodePresentPath::Composited, 64, 32);
        let mut ledger = NativeAdapterRenderCanvasUploadLedger::default();
        ledger.record_adapter_generation(generation);
        let token = ledger
            .register(NativeAtlasResidencyWindowIdentity::Primary, generation)
            .expect("upload account should register");

        assert!(ledger.contribute(
            &token,
            1,
            GpuSurfaceRenderCanvasUploadStats::default(),
            Some(GpuSurfaceRenderCanvasUploadPlan::new(context)),
            Some(context),
        ));
        assert!(ledger.contribute(
            &token,
            2,
            GpuSurfaceRenderCanvasUploadStats::default(),
            Some(exact_upload_plan(context, 10, 20, 30)),
            Some(context),
        ));
        for (sequence, reason) in [
            (
                3,
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid,
            ),
            (
                4,
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Unsupported,
            ),
            (
                5,
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            ),
            (
                6,
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Overflow,
            ),
        ] {
            assert!(ledger.contribute(
                &token,
                sequence,
                GpuSurfaceRenderCanvasUploadStats::default(),
                Some(unavailable_upload_plan(context, reason)),
                Some(context),
            ));
        }

        let profile = ledger.profile();
        assert_eq!(profile.observed_candidate_plan_count, Some(6));
        assert_eq!(profile.observed_candidate_plan_window_count, Some(1));
        assert_eq!(profile.observed_candidate_no_work_count, Some(1));
        assert_eq!(profile.observed_candidate_exact_count, Some(1));
        assert_eq!(profile.observed_candidate_invalid_count, Some(1));
        assert_eq!(profile.observed_candidate_unsupported_count, Some(1));
        assert_eq!(profile.observed_candidate_incomplete_count, Some(1));
        assert_eq!(profile.observed_candidate_overflow_count, Some(1));
        assert_eq!(
            profile.observed_candidate_exact_immutable_payload_logical_bytes,
            Some(10)
        );
        assert_eq!(
            profile.observed_candidate_exact_volatile_payload_logical_bytes,
            Some(20)
        );
        assert_eq!(
            profile.observed_candidate_exact_renderer_parameter_logical_bytes,
            Some(30)
        );
    }

    #[test]
    fn render_canvas_upload_candidate_rejection_preserves_actual_stats_and_no_plan_is_inert() {
        let generation = NativeAdapterGeneration::from_test_serial(1);
        let context = upload_plan_context(generation, NativeEncodePresentPath::Composited, 64, 32);
        let mismatched_context =
            upload_plan_context(generation, NativeEncodePresentPath::Composited, 128, 32);
        let direct_context =
            upload_plan_context(generation, NativeEncodePresentPath::DirectResize, 64, 32);
        let mut ledger = NativeAdapterRenderCanvasUploadLedger::default();
        ledger.record_adapter_generation(generation);
        let token = ledger
            .register(NativeAtlasResidencyWindowIdentity::Primary, generation)
            .expect("upload account should register");
        let actual = upload_stats((Some(1), Some(4)), (Some(1), Some(8)), (Some(1), Some(16)));

        assert!(ledger.contribute(
            &token,
            1,
            actual,
            Some(exact_upload_plan(mismatched_context, 1, 2, 3)),
            Some(context),
        ));
        assert!(ledger.contribute(
            &token,
            2,
            actual,
            Some(GpuSurfaceRenderCanvasUploadPlan::new(direct_context)),
            Some(direct_context),
        ));
        assert!(ledger.contribute(
            &token,
            3,
            actual,
            Some(exact_upload_plan(context, 1, 2, 3)),
            None,
        ));
        assert!(ledger.contribute(&token, 4, actual, None, None));

        let profile = ledger.profile();
        assert_eq!(profile.observed_candidate_plan_count, Some(0));
        assert_eq!(profile.observed_candidate_plan_window_count, Some(0));
        assert_eq!(profile.observed_candidate_exact_count, Some(0));
        assert_eq!(profile.immutable_payload_operations, Some(4));
        assert_eq!(profile.immutable_payload_logical_bytes, Some(16));
        assert_eq!(profile.volatile_payload_operations, Some(4));
        assert_eq!(profile.volatile_payload_logical_bytes, Some(32));
        assert_eq!(profile.renderer_parameter_operations, Some(4));
        assert_eq!(profile.renderer_parameter_logical_bytes, Some(64));
    }

    #[test]
    fn render_canvas_upload_candidate_rebind_resets_old_generation_evidence() {
        let first_generation = NativeAdapterGeneration::from_test_serial(1);
        let second_generation = NativeAdapterGeneration::from_test_serial(2);
        let first_context = upload_plan_context(
            first_generation,
            NativeEncodePresentPath::Composited,
            64,
            32,
        );
        let second_context = upload_plan_context(
            second_generation,
            NativeEncodePresentPath::Composited,
            64,
            32,
        );
        let identity = NativeAtlasResidencyWindowIdentity::Auxiliary(String::from("same-key"));
        let mut ledger = NativeAdapterRenderCanvasUploadLedger::default();
        ledger.record_adapter_generation(first_generation);
        let first = ledger
            .register(identity, first_generation)
            .expect("first upload account should register");
        let first_plan = exact_upload_plan(first_context, 2, 3, 4);
        assert!(ledger.contribute(
            &first,
            1,
            GpuSurfaceRenderCanvasUploadStats::default(),
            Some(first_plan),
            Some(first_context),
        ));
        assert_eq!(ledger.profile().observed_candidate_exact_count, Some(1));

        ledger.record_adapter_generation(second_generation);
        assert_eq!(ledger.profile().observed_candidate_plan_count, Some(0));
        assert!(!ledger.contribute(
            &first,
            2,
            GpuSurfaceRenderCanvasUploadStats::default(),
            Some(exact_upload_plan(first_context, 2, 3, 4)),
            Some(first_context),
        ));
        let rebound = ledger
            .rebind(&first, second_generation)
            .expect("current account should rebind");
        assert!(ledger.contribute(
            &rebound,
            1,
            GpuSurfaceRenderCanvasUploadStats::default(),
            Some(exact_upload_plan(second_context, 5, 6, 7)),
            Some(second_context),
        ));
        assert_eq!(ledger.profile().observed_candidate_plan_count, Some(1));
        assert_eq!(
            ledger
                .profile()
                .observed_candidate_exact_immutable_payload_logical_bytes,
            Some(5)
        );
    }

    #[test]
    fn render_canvas_upload_candidate_counter_and_bytes_overflow_clear_on_removal() {
        let generation = NativeAdapterGeneration::from_test_serial(1);
        let context = upload_plan_context(generation, NativeEncodePresentPath::Composited, 64, 32);
        let mut ledger = NativeAdapterRenderCanvasUploadLedger::default();
        ledger.record_adapter_generation(generation);
        let token = ledger
            .register(NativeAtlasResidencyWindowIdentity::Primary, generation)
            .expect("upload account should register");
        let account = ledger
            .accounts
            .get_mut(&token.window_identity)
            .expect("registered account should exist");
        account.has_observed_candidate_plan = true;
        account.totals.candidate.observed_candidate_plan_count = Some(usize::MAX);
        account.totals.candidate.observed_candidate_exact_count = Some(usize::MAX);
        account
            .totals
            .candidate
            .observed_candidate_exact_immutable_payload_operations = Some(usize::MAX);
        account
            .totals
            .candidate
            .observed_candidate_exact_immutable_payload_logical_bytes = Some(u64::MAX);
        ledger.aggregate.candidate.observed_candidate_plan_count = Some(usize::MAX);
        ledger.aggregate.candidate.observed_candidate_exact_count = Some(usize::MAX);
        ledger
            .aggregate
            .candidate
            .observed_candidate_exact_immutable_payload_operations = Some(usize::MAX);
        ledger
            .aggregate
            .candidate
            .observed_candidate_exact_immutable_payload_logical_bytes = Some(u64::MAX);

        assert!(ledger.contribute(
            &token,
            1,
            GpuSurfaceRenderCanvasUploadStats::default(),
            Some(exact_upload_plan(context, 1, 2, 3)),
            Some(context),
        ));
        let profile = ledger.profile();
        assert_eq!(profile.observed_candidate_plan_count, None);
        assert_eq!(profile.observed_candidate_exact_count, None);
        assert_eq!(
            profile.observed_candidate_exact_immutable_payload_operations,
            None
        );
        assert_eq!(
            profile.observed_candidate_exact_immutable_payload_logical_bytes,
            None
        );

        assert!(ledger.remove(&token));
        let recovered = ledger.profile();
        assert_eq!(recovered.observed_candidate_plan_count, Some(0));
        assert_eq!(recovered.observed_candidate_exact_count, Some(0));
        assert_eq!(
            recovered.observed_candidate_exact_immutable_payload_operations,
            Some(0)
        );
        assert_eq!(
            recovered.observed_candidate_exact_immutable_payload_logical_bytes,
            Some(0)
        );
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
    fn signal_ledger_aggregates_primary_and_auxiliaries_once() {
        let generation = NativeAdapterGeneration::from_test_serial(1);
        let mut ledger = NativeAdapterSignalResidencyLedger::default();
        ledger.record_adapter_generation(generation);

        let primary = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Primary,
                generation,
                signal_snapshots(
                    Some(signal_snapshot(generation, 3, Some(12), 2, Some(8))),
                    None,
                    None,
                ),
            )
            .expect("primary signal account should register");
        let auxiliary = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Auxiliary(String::from("inspector")),
                generation,
                signal_snapshots(
                    Some(signal_snapshot(generation, 2, Some(8), 1, Some(4))),
                    Some(signal_snapshot(generation, 1, Some(4), 1, Some(4))),
                    None,
                ),
            )
            .expect("auxiliary signal account should register");

        assert_eq!(ledger.account_count(), 2);
        let profile = ledger.profile();
        assert_eq!(profile.active_signal_buffer_resident_count, Some(5));
        assert_eq!(profile.active_signal_buffer_logical_bytes, Some(20));
        assert_eq!(profile.active_signal_body_texture_resident_count, Some(3));
        assert_eq!(
            profile.active_signal_body_texture_logical_rgba_bytes,
            Some(12)
        );
        assert_eq!(profile.quarantined_signal_buffer_resident_count, Some(1));
        assert_eq!(profile.quarantined_signal_buffer_logical_bytes, Some(4));
        assert_eq!(
            profile.quarantined_signal_body_texture_resident_count,
            Some(1)
        );
        assert_eq!(
            profile.quarantined_signal_body_texture_logical_rgba_bytes,
            Some(4)
        );

        assert!(ledger.update(
            &primary,
            signal_snapshots(
                Some(signal_snapshot(generation, 4, Some(16), 3, Some(12))),
                None,
                None,
            ),
        ));
        let profile = ledger.profile();
        assert_eq!(profile.active_signal_buffer_resident_count, Some(6));
        assert_eq!(profile.active_signal_buffer_logical_bytes, Some(24));
        assert_eq!(profile.active_signal_body_texture_resident_count, Some(4));
        assert_eq!(
            profile.active_signal_body_texture_logical_rgba_bytes,
            Some(16)
        );

        assert!(ledger.remove(&auxiliary));
        let profile = ledger.profile();
        assert_eq!(profile.active_signal_buffer_resident_count, Some(4));
        assert_eq!(profile.active_signal_body_texture_resident_count, Some(3));
        assert_eq!(profile.quarantined_signal_buffer_resident_count, Some(0));
        assert_eq!(
            profile.quarantined_signal_body_texture_resident_count,
            Some(0)
        );
        assert!(!ledger.remove(&auxiliary));
    }

    #[test]
    fn signal_ledger_keeps_physical_quarantine_across_recovery_and_rebinds_active() {
        let old_generation = NativeAdapterGeneration::from_test_serial(1);
        let new_generation = NativeAdapterGeneration::from_test_serial(2);
        let mut ledger = NativeAdapterSignalResidencyLedger::default();
        ledger.record_adapter_generation(old_generation);
        let token = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Primary,
                old_generation,
                signal_snapshots(
                    Some(signal_snapshot(old_generation, 3, Some(12), 2, Some(8))),
                    None,
                    None,
                ),
            )
            .expect("primary signal account should register");

        ledger.record_adapter_generation(new_generation);
        let profile = ledger.profile();
        assert_eq!(profile.active_signal_buffer_resident_count, None);
        assert_eq!(profile.active_signal_body_texture_resident_count, None);

        let rebound = ledger
            .rebind(
                &token,
                new_generation,
                signal_snapshots(
                    Some(signal_snapshot(new_generation, 2, Some(8), 1, Some(4))),
                    Some(signal_snapshot(old_generation, 3, Some(12), 2, Some(8))),
                    None,
                ),
            )
            .expect("current signal account should rebind");
        let profile = ledger.profile();
        assert_eq!(profile.active_signal_buffer_resident_count, Some(2));
        assert_eq!(profile.active_signal_buffer_logical_bytes, Some(8));
        assert_eq!(profile.active_signal_body_texture_resident_count, Some(1));
        assert_eq!(
            profile.active_signal_body_texture_logical_rgba_bytes,
            Some(4)
        );
        assert_eq!(profile.quarantined_signal_buffer_resident_count, Some(3));
        assert_eq!(profile.quarantined_signal_buffer_logical_bytes, Some(12));
        assert_eq!(
            profile.quarantined_signal_body_texture_resident_count,
            Some(2)
        );
        assert_eq!(
            profile.quarantined_signal_body_texture_logical_rgba_bytes,
            Some(8)
        );

        assert!(!ledger.update(
            &token,
            signal_snapshots(
                Some(signal_snapshot(
                    old_generation,
                    99,
                    Some(396),
                    98,
                    Some(392)
                )),
                None,
                None,
            ),
        ));
        assert!(ledger.update(
            &rebound,
            signal_snapshots(
                Some(signal_snapshot(new_generation, 4, Some(16), 3, Some(12))),
                Some(signal_snapshot(old_generation, 3, Some(12), 2, Some(8))),
                None,
            ),
        ));
        let profile = ledger.profile();
        assert_eq!(profile.active_signal_buffer_resident_count, Some(4));
        assert_eq!(profile.active_signal_body_texture_resident_count, Some(3));
        assert_eq!(profile.quarantined_signal_buffer_resident_count, Some(3));
        assert_eq!(
            profile.quarantined_signal_body_texture_resident_count,
            Some(2)
        );

        assert!(ledger.update(
            &rebound,
            signal_snapshots(
                Some(signal_snapshot(new_generation, 4, Some(16), 3, Some(12))),
                None,
                None,
            ),
        ));
        assert_eq!(ledger.known_adapter_generations.len(), 1);
        let profile = ledger.profile();
        assert_eq!(profile.quarantined_signal_buffer_resident_count, Some(0));
        assert_eq!(profile.quarantined_signal_buffer_logical_bytes, Some(0));
        assert_eq!(
            profile.quarantined_signal_body_texture_resident_count,
            Some(0)
        );
        assert_eq!(
            profile.quarantined_signal_body_texture_logical_rgba_bytes,
            Some(0)
        );
    }

    #[test]
    fn signal_ledger_fences_wrong_generation_and_same_key_incarnations() {
        let first_generation = NativeAdapterGeneration::from_test_serial(1);
        let second_generation = NativeAdapterGeneration::from_test_serial(2);
        let identity = NativeAtlasResidencyWindowIdentity::Auxiliary(String::from("same-key"));
        let mut ledger = NativeAdapterSignalResidencyLedger::default();
        ledger.record_adapter_generation(first_generation);
        let first = ledger
            .register(
                identity.clone(),
                first_generation,
                signal_snapshots(
                    Some(signal_snapshot(first_generation, 1, Some(4), 2, Some(8))),
                    None,
                    None,
                ),
            )
            .expect("first signal incarnation should register");
        ledger.record_adapter_generation(second_generation);
        let wrong_generation = NativeAdapterSignalResidencyAccountToken {
            adapter_generation: second_generation,
            ..first.clone()
        };
        assert!(!ledger.update(
            &wrong_generation,
            signal_snapshots(
                Some(signal_snapshot(first_generation, 7, Some(28), 8, Some(32))),
                None,
                None,
            ),
        ));
        assert!(ledger.remove(&first));
        assert!(!ledger.remove(&first));

        let second = ledger
            .register(
                identity,
                second_generation,
                signal_snapshots(
                    Some(signal_snapshot(second_generation, 2, Some(8), 3, Some(12))),
                    None,
                    None,
                ),
            )
            .expect("replacement signal incarnation should register");
        assert_ne!(first.account_generation, second.account_generation);
        assert!(!ledger.update(
            &first,
            signal_snapshots(
                Some(signal_snapshot(
                    second_generation,
                    9,
                    Some(36),
                    10,
                    Some(40)
                )),
                None,
                None,
            ),
        ));
        assert!(!ledger.remove(&first));
        let profile = ledger.profile();
        assert_eq!(profile.active_signal_buffer_resident_count, Some(2));
        assert_eq!(profile.active_signal_body_texture_resident_count, Some(3));
    }

    #[test]
    fn signal_ledger_keeps_unknown_bytes_independent_and_recovers() {
        let generation = NativeAdapterGeneration::from_test_serial(1);
        let mut ledger = NativeAdapterSignalResidencyLedger::default();
        ledger.record_adapter_generation(generation);
        let token = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Primary,
                generation,
                signal_snapshots(
                    Some(signal_snapshot(generation, 2, None, 3, Some(12))),
                    Some(signal_snapshot(generation, 1, Some(4), 2, None)),
                    None,
                ),
            )
            .expect("signal account with unknown bytes should register");
        let profile = ledger.profile();
        assert_eq!(profile.active_signal_buffer_resident_count, Some(2));
        assert_eq!(profile.active_signal_buffer_logical_bytes, None);
        assert_eq!(profile.active_signal_body_texture_resident_count, Some(3));
        assert_eq!(
            profile.active_signal_body_texture_logical_rgba_bytes,
            Some(12)
        );
        assert_eq!(profile.quarantined_signal_buffer_resident_count, Some(1));
        assert_eq!(profile.quarantined_signal_buffer_logical_bytes, Some(4));
        assert_eq!(
            profile.quarantined_signal_body_texture_resident_count,
            Some(2)
        );
        assert_eq!(
            profile.quarantined_signal_body_texture_logical_rgba_bytes,
            None
        );

        assert!(ledger.update(
            &token,
            signal_snapshots(
                Some(signal_snapshot(generation, 2, Some(8), 3, Some(12))),
                Some(signal_snapshot(generation, 1, Some(4), 2, Some(8))),
                None,
            ),
        ));
        let profile = ledger.profile();
        assert_eq!(profile.active_signal_buffer_logical_bytes, Some(8));
        assert_eq!(
            profile.quarantined_signal_body_texture_logical_rgba_bytes,
            Some(8)
        );
    }

    #[test]
    fn signal_ledger_recovers_after_count_and_byte_overflow() {
        let generation = NativeAdapterGeneration::from_test_serial(1);
        let mut ledger = NativeAdapterSignalResidencyLedger::default();
        ledger.record_adapter_generation(generation);
        let max = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Primary,
                generation,
                signal_snapshots(
                    Some(signal_snapshot(
                        generation,
                        usize::MAX,
                        Some(u64::MAX),
                        usize::MAX,
                        Some(u64::MAX),
                    )),
                    None,
                    None,
                ),
            )
            .expect("maximum signal contribution should register");
        let one = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Auxiliary(String::from("overflow")),
                generation,
                signal_snapshots(
                    Some(signal_snapshot(generation, 1, Some(1), 1, Some(1))),
                    None,
                    None,
                ),
            )
            .expect("overflow signal contribution should register");

        let profile = ledger.profile();
        assert_eq!(profile.active_signal_buffer_resident_count, None);
        assert_eq!(profile.active_signal_buffer_logical_bytes, None);
        assert_eq!(profile.active_signal_body_texture_resident_count, None);
        assert_eq!(profile.active_signal_body_texture_logical_rgba_bytes, None);
        assert!(ledger.remove(&one));
        let profile = ledger.profile();
        assert_eq!(
            profile.active_signal_buffer_resident_count,
            Some(usize::MAX)
        );
        assert_eq!(profile.active_signal_buffer_logical_bytes, Some(u64::MAX));
        assert_eq!(
            profile.active_signal_body_texture_resident_count,
            Some(usize::MAX)
        );
        assert_eq!(
            profile.active_signal_body_texture_logical_rgba_bytes,
            Some(u64::MAX)
        );
        assert!(ledger.remove(&max));
        let profile = ledger.profile();
        assert_eq!(profile.active_signal_buffer_resident_count, Some(0));
        assert_eq!(profile.active_signal_buffer_logical_bytes, Some(0));
        assert_eq!(profile.active_signal_body_texture_resident_count, Some(0));
        assert_eq!(
            profile.active_signal_body_texture_logical_rgba_bytes,
            Some(0)
        );
    }

    #[test]
    fn signal_profile_default_is_absent_while_empty_ledger_is_zero() {
        let default_profile = NativeAdapterSignalResidencyProfile::default();
        assert_eq!(default_profile.adapter_generation, None);
        assert_eq!(default_profile.active_signal_buffer_resident_count, None);
        assert_eq!(default_profile.active_signal_buffer_logical_bytes, None);
        assert_eq!(
            default_profile.active_signal_body_texture_resident_count,
            None
        );
        assert_eq!(
            default_profile.active_signal_body_texture_logical_rgba_bytes,
            None
        );
        assert_eq!(
            default_profile.quarantined_signal_buffer_resident_count,
            None
        );
        assert_eq!(
            default_profile.quarantined_signal_buffer_logical_bytes,
            None
        );
        assert_eq!(
            default_profile.quarantined_signal_body_texture_resident_count,
            None
        );
        assert_eq!(
            default_profile.quarantined_signal_body_texture_logical_rgba_bytes,
            None
        );

        let empty_ledger = NativeAdapterSignalResidencyLedger::default();
        let empty_profile = empty_ledger.profile();
        assert_eq!(empty_profile.adapter_generation, None);
        assert_eq!(empty_profile.active_signal_buffer_resident_count, Some(0));
        assert_eq!(empty_profile.active_signal_buffer_logical_bytes, Some(0));
        assert_eq!(
            empty_profile.active_signal_body_texture_resident_count,
            Some(0)
        );
        assert_eq!(
            empty_profile.active_signal_body_texture_logical_rgba_bytes,
            Some(0)
        );
        assert_eq!(
            empty_profile.quarantined_signal_buffer_resident_count,
            Some(0)
        );
        assert_eq!(
            empty_profile.quarantined_signal_buffer_logical_bytes,
            Some(0)
        );
        assert_eq!(
            empty_profile.quarantined_signal_body_texture_resident_count,
            Some(0)
        );
        assert_eq!(
            empty_profile.quarantined_signal_body_texture_logical_rgba_bytes,
            Some(0)
        );
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
            signal_residency: NativeAdapterSignalResidencyLedger::default(),
            custom_shader_residency: NativeAdapterCustomShaderResidencyLedger::default(),
            render_canvas_upload_ledger: NativeAdapterRenderCanvasUploadLedger::default(),
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
    fn custom_shader_ledger_aggregates_primary_auxiliary_and_q0_q1_once() {
        let generation = NativeAdapterGeneration::from_test_serial(1);
        let mut ledger = NativeAdapterCustomShaderResidencyLedger::default();
        ledger.record_adapter_generation(generation);
        let primary = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Primary,
                generation,
                custom_shader_snapshots(
                    Some(custom_shader_snapshot(
                        generation,
                        2,
                        3,
                        Some(10),
                        Some(20),
                        Some(30),
                        Some(40),
                    )),
                    None,
                    None,
                ),
            )
            .expect("primary custom-shader account should register");
        let auxiliary = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Auxiliary(String::from("inspector")),
                generation,
                custom_shader_snapshots(
                    Some(custom_shader_snapshot(
                        generation,
                        4,
                        5,
                        Some(11),
                        Some(21),
                        Some(31),
                        Some(41),
                    )),
                    Some(custom_shader_snapshot(
                        generation,
                        1,
                        2,
                        Some(12),
                        Some(22),
                        Some(32),
                        Some(42),
                    )),
                    Some(custom_shader_snapshot(
                        generation,
                        3,
                        4,
                        Some(13),
                        Some(23),
                        Some(33),
                        Some(43),
                    )),
                ),
            )
            .expect("auxiliary custom-shader account should register");

        assert_eq!(ledger.account_count(), 2);
        let profile = ledger.profile();
        assert_eq!(profile.active_pipeline_resident_count, Some(6));
        assert_eq!(profile.active_binding_resident_count, Some(8));
        assert_eq!(profile.active_surface_uniform_logical_bytes, Some(21));
        assert_eq!(profile.active_app_uniform_logical_bytes, Some(41));
        assert_eq!(profile.active_storage_logical_bytes, Some(61));
        assert_eq!(profile.active_presentation_uniform_logical_bytes, Some(81));
        assert_eq!(profile.quarantined_pipeline_resident_count, Some(4));
        assert_eq!(profile.quarantined_binding_resident_count, Some(6));
        assert_eq!(profile.quarantined_surface_uniform_logical_bytes, Some(25));
        assert_eq!(profile.quarantined_app_uniform_logical_bytes, Some(45));
        assert_eq!(profile.quarantined_storage_logical_bytes, Some(65));
        assert_eq!(
            profile.quarantined_presentation_uniform_logical_bytes,
            Some(85)
        );

        assert!(ledger.update(
            &primary,
            custom_shader_snapshots(
                Some(custom_shader_snapshot(
                    generation,
                    5,
                    6,
                    Some(15),
                    Some(25),
                    Some(35),
                    Some(45),
                )),
                None,
                None,
            ),
        ));
        assert_eq!(ledger.profile().active_pipeline_resident_count, Some(9));
        assert!(ledger.remove(&auxiliary));
        let profile = ledger.profile();
        assert_eq!(profile.active_pipeline_resident_count, Some(5));
        assert_eq!(profile.quarantined_pipeline_resident_count, Some(0));
    }

    #[test]
    fn custom_shader_ledger_keeps_recovery_quarantine_and_prunes_history() {
        let first_generation = NativeAdapterGeneration::from_test_serial(1);
        let mut ledger = NativeAdapterCustomShaderResidencyLedger::default();
        ledger.record_adapter_generation(first_generation);
        let mut token = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Primary,
                first_generation,
                custom_shader_snapshots(
                    Some(custom_shader_snapshot(
                        first_generation,
                        3,
                        4,
                        Some(12),
                        Some(16),
                        Some(20),
                        Some(24),
                    )),
                    None,
                    None,
                ),
            )
            .expect("recovery account should register");

        for serial in 2..=32 {
            let generation = NativeAdapterGeneration::from_test_serial(serial);
            ledger.record_adapter_generation(generation);
            token = ledger
                .rebind(
                    &token,
                    generation,
                    custom_shader_snapshots(
                        Some(custom_shader_snapshot(
                            generation,
                            2,
                            3,
                            Some(8),
                            Some(12),
                            Some(16),
                            Some(20),
                        )),
                        Some(custom_shader_snapshot(
                            first_generation,
                            3,
                            4,
                            Some(12),
                            Some(16),
                            Some(20),
                            Some(24),
                        )),
                        None,
                    ),
                )
                .expect("current account should rebind");
        }

        assert_eq!(ledger.known_adapter_generations.len(), 2);
        let profile = ledger.profile();
        assert_eq!(
            profile.adapter_generation,
            Some(NativeAdapterGeneration::from_test_serial(32))
        );
        assert_eq!(profile.active_pipeline_resident_count, Some(2));
        assert_eq!(profile.quarantined_pipeline_resident_count, Some(3));
        assert_eq!(profile.quarantined_storage_logical_bytes, Some(20));

        assert!(ledger.update(
            &token,
            custom_shader_snapshots(
                Some(custom_shader_snapshot(
                    NativeAdapterGeneration::from_test_serial(32),
                    2,
                    3,
                    Some(8),
                    Some(12),
                    Some(16),
                    Some(20),
                )),
                None,
                None,
            ),
        ));
        assert_eq!(ledger.known_adapter_generations.len(), 1);
        assert_eq!(
            ledger.profile().quarantined_pipeline_resident_count,
            Some(0)
        );
    }

    #[test]
    fn custom_shader_ledger_fences_stale_wrong_and_same_key_tokens() {
        let first_generation = NativeAdapterGeneration::from_test_serial(1);
        let second_generation = NativeAdapterGeneration::from_test_serial(2);
        let identity = NativeAtlasResidencyWindowIdentity::Auxiliary(String::from("same-key"));
        let mut ledger = NativeAdapterCustomShaderResidencyLedger::default();
        ledger.record_adapter_generation(first_generation);
        let first = ledger
            .register(
                identity.clone(),
                first_generation,
                custom_shader_snapshots(
                    Some(custom_shader_snapshot(
                        first_generation,
                        1,
                        2,
                        Some(4),
                        Some(8),
                        Some(12),
                        Some(16),
                    )),
                    None,
                    None,
                ),
            )
            .expect("first incarnation should register");
        ledger.record_adapter_generation(second_generation);
        let wrong_generation = NativeAdapterCustomShaderResidencyAccountToken {
            adapter_generation: second_generation,
            ..first.clone()
        };
        assert!(!ledger.update(
            &wrong_generation,
            custom_shader_snapshots(
                Some(custom_shader_snapshot(
                    first_generation,
                    9,
                    9,
                    Some(36),
                    Some(36),
                    Some(36),
                    Some(36),
                )),
                None,
                None,
            ),
        ));
        assert!(ledger.remove(&first));
        let second = ledger
            .register(
                identity,
                second_generation,
                custom_shader_snapshots(
                    Some(custom_shader_snapshot(
                        second_generation,
                        2,
                        3,
                        Some(8),
                        Some(12),
                        Some(16),
                        Some(20),
                    )),
                    None,
                    None,
                ),
            )
            .expect("same-key replacement should register");
        assert_ne!(first.account_generation, second.account_generation);
        assert!(!ledger.update(
            &first,
            custom_shader_snapshots(
                Some(custom_shader_snapshot(
                    second_generation,
                    99,
                    99,
                    Some(396),
                    Some(396),
                    Some(396),
                    Some(396),
                )),
                None,
                None,
            ),
        ));
        assert!(!ledger.remove(&first));
        assert_eq!(ledger.profile().active_pipeline_resident_count, Some(2));
    }

    #[test]
    fn custom_shader_ledger_propagates_unknown_and_overflow_independently_and_recovers() {
        let generation = NativeAdapterGeneration::from_test_serial(1);
        let mut ledger = NativeAdapterCustomShaderResidencyLedger::default();
        ledger.record_adapter_generation(generation);
        let unknown = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Primary,
                generation,
                custom_shader_snapshots(
                    Some(custom_shader_snapshot(
                        generation,
                        2,
                        3,
                        None,
                        Some(8),
                        Some(9),
                        Some(10),
                    )),
                    Some(custom_shader_snapshot(
                        generation,
                        1,
                        2,
                        Some(4),
                        None,
                        Some(6),
                        Some(7),
                    )),
                    None,
                ),
            )
            .expect("unknown byte families should not reject account registration");
        let profile = ledger.profile();
        assert_eq!(profile.active_pipeline_resident_count, Some(2));
        assert_eq!(profile.active_binding_resident_count, Some(3));
        assert_eq!(profile.active_surface_uniform_logical_bytes, None);
        assert_eq!(profile.active_app_uniform_logical_bytes, Some(8));
        assert_eq!(profile.quarantined_pipeline_resident_count, Some(1));
        assert_eq!(profile.quarantined_app_uniform_logical_bytes, None);
        assert_eq!(profile.quarantined_storage_logical_bytes, Some(6));

        assert!(ledger.update(
            &unknown,
            custom_shader_snapshots(
                Some(custom_shader_snapshot(
                    generation,
                    2,
                    3,
                    Some(5),
                    Some(8),
                    Some(9),
                    Some(10),
                )),
                Some(custom_shader_snapshot(
                    generation,
                    1,
                    2,
                    Some(4),
                    Some(5),
                    Some(6),
                    Some(7),
                )),
                None,
            ),
        ));
        assert_eq!(
            ledger.profile().active_surface_uniform_logical_bytes,
            Some(5)
        );
        assert_eq!(
            ledger.profile().quarantined_app_uniform_logical_bytes,
            Some(5)
        );
        assert!(ledger.remove(&unknown));

        let max = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Auxiliary(String::from("max")),
                generation,
                custom_shader_snapshots(
                    Some(custom_shader_snapshot(
                        generation,
                        usize::MAX,
                        1,
                        Some(u64::MAX),
                        Some(1),
                        Some(2),
                        Some(3),
                    )),
                    None,
                    None,
                ),
            )
            .expect("maximum contribution should register");
        let one = ledger
            .register(
                NativeAtlasResidencyWindowIdentity::Auxiliary(String::from("overflow")),
                generation,
                custom_shader_snapshots(
                    Some(custom_shader_snapshot(
                        generation,
                        1,
                        1,
                        Some(1),
                        Some(2),
                        Some(3),
                        Some(4),
                    )),
                    None,
                    None,
                ),
            )
            .expect("overflow contribution should register");
        let profile = ledger.profile();
        assert_eq!(profile.active_pipeline_resident_count, None);
        assert_eq!(profile.active_surface_uniform_logical_bytes, None);
        assert_eq!(profile.active_binding_resident_count, Some(2));
        assert_eq!(profile.active_app_uniform_logical_bytes, Some(3));
        assert_eq!(profile.active_storage_logical_bytes, Some(5));
        assert_eq!(profile.active_presentation_uniform_logical_bytes, Some(7));
        assert!(ledger.remove(&one));
        let profile = ledger.profile();
        assert_eq!(profile.active_pipeline_resident_count, Some(usize::MAX));
        assert_eq!(profile.active_surface_uniform_logical_bytes, Some(u64::MAX));
        assert_eq!(profile.active_binding_resident_count, Some(1));
        assert_eq!(profile.active_app_uniform_logical_bytes, Some(1));
        assert_eq!(profile.active_storage_logical_bytes, Some(2));
        assert_eq!(profile.active_presentation_uniform_logical_bytes, Some(3));
        assert!(ledger.remove(&max));
    }

    #[test]
    fn custom_shader_ledger_fences_account_generation_exhaustion_and_reports_empty_vs_absent() {
        let generation = NativeAdapterGeneration::from_test_serial(1);
        assert_eq!(
            NativeAdapterCustomShaderResidencyProfile::default().adapter_generation,
            None
        );
        assert_eq!(
            NativeAdapterCustomShaderResidencyLedger::default()
                .profile()
                .active_pipeline_resident_count,
            Some(0)
        );

        let mut ledger = NativeAdapterCustomShaderResidencyLedger::default();
        ledger.record_adapter_generation(generation);
        ledger.next_account_generation = Some(u64::MAX);
        assert!(
            ledger
                .register(
                    NativeAtlasResidencyWindowIdentity::Primary,
                    generation,
                    NativeWindowCustomShaderResidencySnapshots::default(),
                )
                .is_some()
        );
        assert!(
            ledger
                .register(
                    NativeAtlasResidencyWindowIdentity::Auxiliary(String::from("exhausted")),
                    generation,
                    NativeWindowCustomShaderResidencySnapshots::default(),
                )
                .is_none()
        );
    }

    #[test]
    fn custom_shader_ledger_adoption_preserves_old_generation_quarantine_before_rebind() {
        let old_generation = NativeAdapterGeneration::from_test_serial(1);
        let new_generation = NativeAdapterGeneration::from_test_serial(2);
        let registration = Arc::new(DeviceLossRegistration::new());
        let mut previous = GenericNativeAdapterOwner::with_test_registration(
            old_generation,
            Arc::clone(&registration),
        );
        let token = previous
            .register_custom_shader_residency_account(
                NativeAtlasResidencyWindowIdentity::Primary,
                old_generation,
                custom_shader_snapshots(
                    Some(custom_shader_snapshot(
                        old_generation,
                        3,
                        4,
                        Some(12),
                        Some(16),
                        Some(20),
                        Some(24),
                    )),
                    None,
                    None,
                ),
            )
            .expect("previous custom-shader account should register");
        let mut current =
            GenericNativeAdapterOwner::with_test_registration(new_generation, registration);
        current.adopt_custom_shader_residency_ledger(&mut previous);
        assert_eq!(
            current
                .capture_custom_shader_residency_profile()
                .active_pipeline_resident_count,
            None
        );
        let rebound = current
            .rebind_custom_shader_residency_account(
                &token,
                new_generation,
                custom_shader_snapshots(
                    Some(custom_shader_snapshot(
                        new_generation,
                        2,
                        3,
                        Some(8),
                        Some(12),
                        Some(16),
                        Some(20),
                    )),
                    Some(custom_shader_snapshot(
                        old_generation,
                        3,
                        4,
                        Some(12),
                        Some(16),
                        Some(20),
                        Some(24),
                    )),
                    None,
                ),
            )
            .expect("adopted account should rebind");
        let profile = current.capture_custom_shader_residency_profile();
        assert_eq!(profile.adapter_generation, Some(new_generation));
        assert_eq!(profile.active_pipeline_resident_count, Some(2));
        assert_eq!(profile.quarantined_pipeline_resident_count, Some(3));
        assert!(current.remove_custom_shader_residency_account(&rebound));
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
