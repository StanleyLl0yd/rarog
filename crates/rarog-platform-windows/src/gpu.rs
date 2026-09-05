use std::fmt;

#[derive(Debug)]
pub enum WindowsGpuError {
    UnsupportedTarget,
    #[cfg(target_os = "windows")]
    RequestAdapter(wgpu::RequestAdapterError),
    #[cfg(target_os = "windows")]
    RequestDevice(wgpu::RequestDeviceError),
    #[cfg(target_os = "windows")]
    CreateSurface(wgpu::CreateSurfaceError),
    #[cfg(target_os = "windows")]
    Surface(wgpu::SurfaceError),
    #[cfg(target_os = "windows")]
    Compositor(rarog_compositor_wgpu::WgpuCompositorError),
    UnsupportedSurface,
    SuspendedSurface,
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
            #[cfg(target_os = "windows")]
            Self::CreateSurface(error) => {
                write!(formatter, "DX12 surface creation failed: {error}")
            }
            #[cfg(target_os = "windows")]
            Self::Surface(error) => write!(formatter, "DX12 surface acquisition failed: {error}"),
            #[cfg(target_os = "windows")]
            Self::Compositor(error) => write!(formatter, "DX12 presentation failed: {error}"),
            Self::UnsupportedSurface => {
                formatter.write_str("DX12 adapter does not support the requested surface")
            }
            Self::SuspendedSurface => formatter.write_str("DX12 surface is suspended"),
        }
    }
}

impl std::error::Error for WindowsGpuError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsSurfaceRecovery {
    Retry,
    Reconfigure,
    Recreate,
    Fatal,
}

impl WindowsGpuError {
    pub fn surface_recovery(&self) -> Option<WindowsSurfaceRecovery> {
        #[cfg(target_os = "windows")]
        {
            return match self {
                Self::Surface(wgpu::SurfaceError::Timeout) => Some(WindowsSurfaceRecovery::Retry),
                Self::Surface(wgpu::SurfaceError::Outdated) => {
                    Some(WindowsSurfaceRecovery::Reconfigure)
                }
                Self::Surface(wgpu::SurfaceError::Lost) => Some(WindowsSurfaceRecovery::Recreate),
                Self::Surface(wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other) => {
                    Some(WindowsSurfaceRecovery::Fatal)
                }
                _ => None,
            };
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    }
}

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

#[cfg(target_os = "windows")]
pub struct WindowsGpuSurface {
    surface: wgpu::Surface<'static>,
    config: Option<wgpu::SurfaceConfiguration>,
    width: u32,
    height: u32,
}

#[cfg(not(target_os = "windows"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowsGpuSurface;

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
    pub fn compositor_backend(&self) -> rarog_compositor_wgpu::WgpuCompositorBackend {
        rarog_compositor_wgpu::WgpuCompositorBackend::new(self.device.clone(), self.queue.clone())
    }

    #[cfg(target_os = "windows")]
    pub fn into_compositor_backend(self) -> rarog_compositor_wgpu::WgpuCompositorBackend {
        rarog_compositor_wgpu::WgpuCompositorBackend::new(self.device, self.queue)
    }

    #[cfg(target_os = "windows")]
    pub fn create_surface<T>(
        &self,
        target: T,
        width: u32,
        height: u32,
    ) -> Result<WindowsGpuSurface, WindowsGpuError>
    where
        T: Into<wgpu::SurfaceTarget<'static>>,
    {
        let surface: wgpu::Surface<'static> = self
            .instance
            .create_surface(target)
            .map_err(WindowsGpuError::CreateSurface)?;
        let mut surface = WindowsGpuSurface {
            surface,
            config: None,
            width,
            height,
        };
        surface.reconfigure(self)?;
        Ok(surface)
    }
}

#[cfg(target_os = "windows")]
impl WindowsGpuSurface {
    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub const fn is_suspended(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn format(&self) -> Option<wgpu::TextureFormat> {
        self.config.as_ref().map(|config| config.format)
    }

    pub const fn surface(&self) -> &wgpu::Surface<'static> {
        &self.surface
    }

    pub fn resize(
        &mut self,
        gpu: &WindowsGpuDevice,
        width: u32,
        height: u32,
    ) -> Result<(), WindowsGpuError> {
        if self.width == width && self.height == height {
            return Ok(());
        }
        self.width = width;
        self.height = height;
        self.reconfigure(gpu)
    }

    pub fn acquire(&self) -> Result<wgpu::SurfaceTexture, WindowsGpuError> {
        if self.config.is_none() {
            return Err(WindowsGpuError::SuspendedSurface);
        }
        self.surface
            .get_current_texture()
            .map_err(WindowsGpuError::Surface)
    }

    pub fn present(
        &self,
        backend: &mut rarog_compositor_wgpu::WgpuCompositorBackend,
    ) -> Result<(), WindowsGpuError> {
        let frame = self.acquire()?;
        let format = self.format().ok_or(WindowsGpuError::SuspendedSurface)?;
        backend
            .present_to_texture(&frame.texture, format)
            .map_err(WindowsGpuError::Compositor)?;
        frame.present();
        Ok(())
    }

    pub fn reconfigure(&mut self, gpu: &WindowsGpuDevice) -> Result<(), WindowsGpuError> {
        if self.is_suspended() {
            self.config = None;
            return Ok(());
        }

        let config = match self.config.take() {
            Some(mut config) => {
                config.width = self.width;
                config.height = self.height;
                config
            }
            None => self
                .surface
                .get_default_config(&gpu.adapter, self.width, self.height)
                .ok_or(WindowsGpuError::UnsupportedSurface)?,
        };
        self.surface.configure(&gpu.device, &config);
        self.config = Some(config);
        Ok(())
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

    #[cfg(target_os = "windows")]
    #[test]
    fn surface_errors_map_to_explicit_recovery_actions() {
        assert_eq!(
            WindowsGpuError::Surface(wgpu::SurfaceError::Timeout).surface_recovery(),
            Some(WindowsSurfaceRecovery::Retry)
        );
        assert_eq!(
            WindowsGpuError::Surface(wgpu::SurfaceError::Outdated).surface_recovery(),
            Some(WindowsSurfaceRecovery::Reconfigure)
        );
        assert_eq!(
            WindowsGpuError::Surface(wgpu::SurfaceError::Lost).surface_recovery(),
            Some(WindowsSurfaceRecovery::Recreate)
        );
        assert_eq!(
            WindowsGpuError::Surface(wgpu::SurfaceError::OutOfMemory).surface_recovery(),
            Some(WindowsSurfaceRecovery::Fatal)
        );
        assert_eq!(
            WindowsGpuError::Surface(wgpu::SurfaceError::Other).surface_recovery(),
            Some(WindowsSurfaceRecovery::Fatal)
        );
    }

    #[test]
    fn surface_state_errors_are_stable() {
        assert_eq!(
            WindowsGpuError::UnsupportedSurface.to_string(),
            "DX12 adapter does not support the requested surface"
        );
        assert_eq!(
            WindowsGpuError::SuspendedSurface.to_string(),
            "DX12 surface is suspended"
        );
    }
}
