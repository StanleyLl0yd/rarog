use std::fmt;
use std::sync::Arc;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlatformFontFamily {
    Named(String),
    Serif,
    SansSerif,
    Monospace,
    Cursive,
    Fantasy,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlatformFontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlatformFontRequest {
    pub families: Vec<PlatformFontFamily>,
    pub weight: f32,
    pub stretch: f32,
    pub style: PlatformFontStyle,
    pub size_px: f32,
}

impl Default for PlatformFontRequest {
    fn default() -> Self {
        Self {
            families: vec![PlatformFontFamily::SansSerif],
            weight: 400.0,
            stretch: 1.0,
            style: PlatformFontStyle::Normal,
            size_px: 16.0,
        }
    }
}

impl PlatformFontRequest {
    pub fn validate(&self) -> Result<(), PlatformFontError> {
        if self.families.is_empty() {
            return Err(PlatformFontError::EmptyFamilyList);
        }
        if !self.weight.is_finite() || !(1.0..=1000.0).contains(&self.weight) {
            return Err(PlatformFontError::InvalidWeight(self.weight));
        }
        if !self.stretch.is_finite() || !(0.5..=2.0).contains(&self.stretch) {
            return Err(PlatformFontError::InvalidStretch(self.stretch));
        }
        if !self.size_px.is_finite() || self.size_px <= 0.0 {
            return Err(PlatformFontError::InvalidSize(self.size_px));
        }
        if self.families.iter().any(|family| {
            matches!(family, PlatformFontFamily::Named(name) if name.trim().is_empty())
        }) {
            return Err(PlatformFontError::EmptyNamedFamily);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlatformFontMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
}

impl PlatformFontMetrics {
    pub fn line_height(self) -> f32 {
        self.ascent + self.descent + self.line_gap
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlatformFontProperties {
    pub weight: f32,
    pub stretch: f32,
    pub style: PlatformFontStyle,
}

#[derive(Clone, Debug)]
pub struct ResolvedPlatformFont {
    pub family_name: String,
    pub postscript_name: Option<String>,
    pub data: Arc<[u8]>,
    pub face_index: u32,
    pub size_px: f32,
    pub metrics: PlatformFontMetrics,
    pub properties: PlatformFontProperties,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlatformFontError {
    UnsupportedTarget,
    EmptyFamilyList,
    EmptyNamedFamily,
    InvalidWeight(f32),
    InvalidStretch(f32),
    InvalidSize(f32),
    NotFound,
    LoadFailed,
    DataUnavailable,
    InvalidMetrics,
}

impl fmt::Display for PlatformFontError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTarget => formatter.write_str("font platform is unavailable on this target"),
            Self::EmptyFamilyList => formatter.write_str("font family list must not be empty"),
            Self::EmptyNamedFamily => formatter.write_str("named font family must not be empty"),
            Self::InvalidWeight(value) => write!(formatter, "invalid font weight {value}"),
            Self::InvalidStretch(value) => write!(formatter, "invalid font stretch {value}"),
            Self::InvalidSize(value) => write!(formatter, "invalid font size {value}"),
            Self::NotFound => formatter.write_str("no matching system font was found"),
            Self::LoadFailed => formatter.write_str("selected system font could not be loaded"),
            Self::DataUnavailable => formatter.write_str("selected system font data is unavailable"),
            Self::InvalidMetrics => formatter.write_str("selected system font exposed invalid metrics"),
        }
    }
}

impl std::error::Error for PlatformFontError {}

pub trait PlatformFontService: Send + Sync {
    fn resolve(&self, request: &PlatformFontRequest) -> Result<ResolvedPlatformFont, PlatformFontError>;
}

pub trait PlatformHost: Send + Sync {
    fn name(&self) -> &'static str;

    fn capabilities(&self) -> PlatformCapabilities;

    fn font_service(&self) -> Option<&dyn PlatformFontService> {
        None
    }
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
        assert!(host.font_service().is_none());
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

    #[test]
    fn default_font_request_is_valid() {
        assert_eq!(PlatformFontRequest::default().validate(), Ok(()));
    }

    #[test]
    fn invalid_font_requests_are_rejected_before_platform_calls() {
        let mut request = PlatformFontRequest::default();
        request.families.clear();
        assert_eq!(request.validate(), Err(PlatformFontError::EmptyFamilyList));

        let mut request = PlatformFontRequest::default();
        request.weight = f32::NAN;
        assert!(matches!(
            request.validate(),
            Err(PlatformFontError::InvalidWeight(value)) if value.is_nan()
        ));

        let mut request = PlatformFontRequest::default();
        request.stretch = 2.1;
        assert_eq!(request.validate(), Err(PlatformFontError::InvalidStretch(2.1)));

        let mut request = PlatformFontRequest::default();
        request.size_px = 0.0;
        assert_eq!(request.validate(), Err(PlatformFontError::InvalidSize(0.0)));
    }
}
