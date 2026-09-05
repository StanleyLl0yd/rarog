use std::fmt;

#[derive(Debug)]
pub enum WindowsGpuError {
    UnsupportedTarget,
    #[cfg(target_os = "windows")]
    RequestAdapter(wgpu::RequestAdapterError),
    #[cfg(target_os = "windows")]
    RequestDevice(wgpu::RequestDeviceError),
}

impl fmt::Display for WindowsGpuError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTarget => {
                formatter.write_str("Windows GPU service is unavailable on this target")
            }
            #[cfg(target_os = "windows")]
            Self::RequestAdapter(error) => {
                write!(formatter, "DX12 adapter request failed: {error}")
            }
            #[cfg(target_os = "windows")]
            Self::RequestDevice(error) => write!(formatter, "DX12 device request failed: {error}"),
        }
    }
}

impl std::error::Error for WindowsGpuError {}

#[cfg(target_os = "windows")]
pub struct WindowsGpuDevice {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

#[cfg(not(target_os = "windows"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowsGpuDevice;

impl WindowsGpuDevice {
    pub const fn target_available() -> bool {
        cfg!(target_os = "windows")
    }

    #[cfg(target_os = "windows")]
    pub async fn request() -> Result<Self, WindowsGpuError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .map_err(WindowsGpuError::RequestAdapter)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("rarog-windows-device"),
                ..Default::default()
            })
            .await
            .map_err(WindowsGpuError::RequestDevice)?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }

    #[cfg(not(target_os = "windows"))]
    pub async fn request() -> Result<Self, WindowsGpuError> {
        Err(WindowsGpuError::UnsupportedTarget)
    }

    #[cfg(target_os = "windows")]
    pub const fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    #[cfg(target_os = "windows")]
    pub const fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    #[cfg(target_os = "windows")]
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }

    #[cfg(target_os = "windows")]
    pub const fn device(&self) -> &wgpu::Device {
        &self.device
    }

    #[cfg(target_os = "windows")]
    pub const fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    #[cfg(target_os = "windows")]
    pub fn into_compositor_backend(self) -> rarog_compositor_wgpu::WgpuCompositorBackend {
        rarog_compositor_wgpu::WgpuCompositorBackend::new(self.device, self.queue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_availability_matches_compilation_target() {
        assert_eq!(
            WindowsGpuDevice::target_available(),
            cfg!(target_os = "windows")
        );
    }

    #[test]
    fn unsupported_target_error_is_stable() {
        assert_eq!(
            WindowsGpuError::UnsupportedTarget.to_string(),
            "Windows GPU service is unavailable on this target"
        );
    }
}
