#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlatformService {
    WindowEvents,
    FontText,
    InputIme,
    Accessibility,
    SandboxProcess,
    GpuCompositor,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlatformCapabilities {
    pub window_events: bool,
    pub font_text: bool,
    pub input_ime: bool,
    pub accessibility: bool,
    pub sandbox_process: bool,
    pub gpu_compositor: bool,
}

impl PlatformCapabilities {
    pub const NONE: Self = Self {
        window_events: false,
        font_text: false,
        input_ime: false,
        accessibility: false,
        sandbox_process: false,
        gpu_compositor: false,
    };

    pub const fn supports(self, service: PlatformService) -> bool {
        match service {
            PlatformService::WindowEvents => self.window_events,
            PlatformService::FontText => self.font_text,
            PlatformService::InputIme => self.input_ime,
            PlatformService::Accessibility => self.accessibility,
            PlatformService::SandboxProcess => self.sandbox_process,
            PlatformService::GpuCompositor => self.gpu_compositor,
        }
    }
}

pub trait PlatformHost: Send + Sync {
    fn name(&self) -> &'static str;

    fn capabilities(&self) -> PlatformCapabilities;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NullPlatformHost;

impl PlatformHost for NullPlatformHost {
    fn name(&self) -> &'static str {
        "none"
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities::NONE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_host_exposes_no_platform_services() {
        let host = NullPlatformHost;
        assert_eq!(host.name(), "none");
        assert_eq!(host.capabilities(), PlatformCapabilities::NONE);
        for service in [
            PlatformService::WindowEvents,
            PlatformService::FontText,
            PlatformService::InputIme,
            PlatformService::Accessibility,
            PlatformService::SandboxProcess,
            PlatformService::GpuCompositor,
        ] {
            assert!(!host.capabilities().supports(service));
        }
    }
}
