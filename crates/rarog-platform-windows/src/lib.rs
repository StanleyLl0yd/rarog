use rarog_platform::{
    PlatformCapabilities, PlatformFontError, PlatformFontRequest, PlatformFontService, PlatformHost,
    ResolvedPlatformFont,
};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsPlatformError {
    UnsupportedTarget,
}

impl fmt::Display for WindowsPlatformError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTarget => {
                formatter.write_str("Windows platform host is unavailable on this target")
            }
        }
    }
}

impl std::error::Error for WindowsPlatformError {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowsFontService;

impl WindowsFontService {
    pub const fn new() -> Self {
        Self
    }
}

impl PlatformFontService for WindowsFontService {
    fn resolve(
        &self,
        request: &PlatformFontRequest,
    ) -> Result<ResolvedPlatformFont, PlatformFontError> {
        request.validate()?;
        resolve_system_font(request)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowsPlatformHost {
    fonts: WindowsFontService,
}

impl WindowsPlatformHost {
    pub fn try_new() -> Result<Self, WindowsPlatformError> {
        if cfg!(target_os = "windows") {
            Ok(Self {
                fonts: WindowsFontService::new(),
            })
        } else {
            Err(WindowsPlatformError::UnsupportedTarget)
        }
    }

    pub const fn target_available() -> bool {
        cfg!(target_os = "windows")
    }

    pub const fn fonts(&self) -> &WindowsFontService {
        &self.fonts
    }
}

impl PlatformHost for WindowsPlatformHost {
    fn name(&self) -> &'static str {
        "windows"
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            font_text: true,
            ..PlatformCapabilities::NONE
        }
    }

    fn font_service(&self) -> Option<&dyn PlatformFontService> {
        Some(&self.fonts)
    }
}

#[cfg(target_os = "windows")]
fn resolve_system_font(
    request: &PlatformFontRequest,
) -> Result<ResolvedPlatformFont, PlatformFontError> {
    use font_kit::error::SelectionError;
    use font_kit::family_name::FamilyName;
    use font_kit::handle::Handle;
    use font_kit::loader::Loader;
    use font_kit::properties::{Properties, Stretch, Style, Weight};
    use font_kit::source::SystemSource;
    use rarog_platform::{
        PlatformFontFamily, PlatformFontMetrics, PlatformFontProperties, PlatformFontStyle,
    };
    use std::sync::Arc;

    let families = request
        .families
        .iter()
        .map(|family| match family {
            PlatformFontFamily::Named(name) => FamilyName::Title(name.clone()),
            PlatformFontFamily::Serif => FamilyName::Serif,
            PlatformFontFamily::SansSerif => FamilyName::SansSerif,
            PlatformFontFamily::Monospace => FamilyName::Monospace,
            PlatformFontFamily::Cursive => FamilyName::Cursive,
            PlatformFontFamily::Fantasy => FamilyName::Fantasy,
        })
        .collect::<Vec<_>>();
    let mut properties = Properties::new();
    properties.weight = Weight(request.weight);
    properties.stretch = Stretch(request.stretch);
    properties.style = match request.style {
        PlatformFontStyle::Normal => Style::Normal,
        PlatformFontStyle::Italic => Style::Italic,
        PlatformFontStyle::Oblique => Style::Oblique,
    };

    let handle = SystemSource::new()
        .select_best_match(&families, &properties)
        .map_err(|error| match error {
            SelectionError::NotFound => PlatformFontError::NotFound,
            SelectionError::CannotAccessSource { .. } => PlatformFontError::LoadFailed,
        })?;
    let face_index = match &handle {
        Handle::Path { font_index, .. } | Handle::Memory { font_index, .. } => *font_index,
    };
    let font = handle.load().map_err(|_| PlatformFontError::LoadFailed)?;
    let font_data = font
        .copy_font_data()
        .ok_or(PlatformFontError::DataUnavailable)?;
    if font_data.is_empty() {
        return Err(PlatformFontError::DataUnavailable);
    }

    let native_metrics = font.metrics();
    if native_metrics.units_per_em == 0
        || !native_metrics.ascent.is_finite()
        || !native_metrics.descent.is_finite()
        || !native_metrics.line_gap.is_finite()
    {
        return Err(PlatformFontError::InvalidMetrics);
    }
    let scale = request.size_px / native_metrics.units_per_em as f32;
    let ascent = native_metrics.ascent * scale;
    let descent = -native_metrics.descent * scale;
    let line_gap = native_metrics.line_gap.max(0.0) * scale;
    if !ascent.is_finite()
        || !descent.is_finite()
        || !line_gap.is_finite()
        || ascent <= 0.0
        || descent < 0.0
    {
        return Err(PlatformFontError::InvalidMetrics);
    }

    let selected = font.properties();
    Ok(ResolvedPlatformFont {
        family_name: font.family_name(),
        postscript_name: font.postscript_name(),
        data: Arc::<[u8]>::from(font_data.as_slice()),
        face_index,
        size_px: request.size_px,
        metrics: PlatformFontMetrics {
            ascent,
            descent,
            line_gap,
        },
        properties: PlatformFontProperties {
            weight: selected.weight.0,
            stretch: selected.stretch.0,
            style: match selected.style {
                Style::Normal => PlatformFontStyle::Normal,
                Style::Italic => PlatformFontStyle::Italic,
                Style::Oblique => PlatformFontStyle::Oblique,
            },
        },
    })
}

#[cfg(not(target_os = "windows"))]
fn resolve_system_font(
    _request: &PlatformFontRequest,
) -> Result<ResolvedPlatformFont, PlatformFontError> {
    Err(PlatformFontError::UnsupportedTarget)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_platform::PlatformService;
    #[cfg(target_os = "windows")]
    use rarog_platform::PlatformFontFamily;

    #[test]
    fn construction_matches_compilation_target() {
        let result = WindowsPlatformHost::try_new();
        if cfg!(target_os = "windows") {
            let host = result.expect("Windows CI target should expose the Windows host boundary");
            assert_eq!(host.name(), "windows");
            assert!(host.capabilities().supports(PlatformService::FontText));
            assert!(host.font_service().is_some());
        } else {
            assert_eq!(result, Err(WindowsPlatformError::UnsupportedTarget));
        }
    }

    #[test]
    fn invalid_requests_are_rejected_before_system_lookup() {
        let service = WindowsFontService::new();
        let request = PlatformFontRequest {
            families: Vec::new(),
            ..PlatformFontRequest::default()
        };
        assert!(matches!(
            service.resolve(&request),
            Err(PlatformFontError::EmptyFamilyList)
        ));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn resolves_a_directwrite_system_font_with_bytes_and_scaled_metrics() {
        let service = WindowsFontService::new();
        let request = PlatformFontRequest {
            families: vec![
                PlatformFontFamily::Named("Segoe UI".into()),
                PlatformFontFamily::SansSerif,
            ],
            size_px: 20.0,
            ..PlatformFontRequest::default()
        };
        let resolved = service
            .resolve(&request)
            .expect("Windows CI should expose a default system sans-serif font");

        assert!(!resolved.family_name.trim().is_empty());
        assert!(!resolved.data.is_empty());
        assert_eq!(resolved.size_px, 20.0);
        assert!(resolved.metrics.ascent > 0.0);
        assert!(resolved.metrics.descent >= 0.0);
        assert!(resolved.metrics.line_height() > 0.0);
        assert!((1.0..=1000.0).contains(&resolved.properties.weight));
        assert!((0.5..=2.0).contains(&resolved.properties.stretch));
    }
}
