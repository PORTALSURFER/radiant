use super::super::*;

pub(super) fn render_context_for_options(options: &NativeRunOptions) -> RenderContext {
    let Some(backends) = wgpu_backends(options.gpu.backend) else {
        return RenderContext::new();
    };
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends,
        flags: wgpu::InstanceFlags::from_build_config().with_env(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        backend_options: wgpu::BackendOptions::from_env_or_default(),
    });
    RenderContext {
        instance,
        devices: Vec::new(),
    }
}

fn wgpu_backends(backend: NativeGpuBackend) -> Option<wgpu::Backends> {
    match backend {
        NativeGpuBackend::Auto => None,
        NativeGpuBackend::Primary => Some(wgpu::Backends::PRIMARY),
        NativeGpuBackend::Vulkan => Some(wgpu::Backends::VULKAN),
        NativeGpuBackend::Dx12 => Some(wgpu::Backends::DX12),
        NativeGpuBackend::Metal => Some(wgpu::Backends::METAL),
        NativeGpuBackend::Gl => Some(wgpu::Backends::GL),
        NativeGpuBackend::BrowserWebGpu => Some(wgpu::Backends::BROWSER_WEBGPU),
    }
}

#[cfg(test)]
mod tests {
    use super::{NativeGpuBackend, wgpu, wgpu_backends};

    #[test]
    fn native_gpu_backend_policy_maps_to_wgpu_backends() {
        assert!(wgpu_backends(NativeGpuBackend::Auto).is_none());
        assert_eq!(
            wgpu_backends(NativeGpuBackend::Primary),
            Some(wgpu::Backends::PRIMARY)
        );
        assert_eq!(
            wgpu_backends(NativeGpuBackend::Dx12),
            Some(wgpu::Backends::DX12)
        );
        assert_eq!(
            wgpu_backends(NativeGpuBackend::Vulkan),
            Some(wgpu::Backends::VULKAN)
        );
    }
}
