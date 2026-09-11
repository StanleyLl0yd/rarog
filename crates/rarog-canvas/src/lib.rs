use rarog_types::Color;
use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroU64;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_MAX_CANVAS_SURFACES: usize = 256;
pub const DEFAULT_MAX_CANVAS_CONTEXTS: usize = 256;
pub const DEFAULT_MAX_CANVAS_PIXELS_PER_SURFACE: u64 = 16_777_216;
pub const DEFAULT_MAX_TOTAL_CANVAS_PIXELS: u64 = 67_108_864;
pub const DEFAULT_MAX_CANVAS_STATE_STACK_DEPTH: usize = 64;

static NEXT_CANVAS_REGISTRY_SCOPE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanvasLimits {
    pub max_surfaces: usize,
    pub max_contexts: usize,
    pub max_pixels_per_surface: u64,
    pub max_total_pixels: u64,
    pub max_state_stack_depth: usize,
}

impl CanvasLimits {
    pub const fn is_valid(self) -> bool {
        self.max_surfaces > 0
            && self.max_contexts > 0
            && self.max_contexts <= self.max_surfaces
            && self.max_pixels_per_surface > 0
            && self.max_total_pixels > 0
            && self.max_pixels_per_surface <= self.max_total_pixels
            && self.max_state_stack_depth > 0
    }
}

impl Default for CanvasLimits {
    fn default() -> Self {
        Self {
            max_surfaces: DEFAULT_MAX_CANVAS_SURFACES,
            max_contexts: DEFAULT_MAX_CANVAS_CONTEXTS,
            max_pixels_per_surface: DEFAULT_MAX_CANVAS_PIXELS_PER_SURFACE,
            max_total_pixels: DEFAULT_MAX_TOTAL_CANVAS_PIXELS,
            max_state_stack_depth: DEFAULT_MAX_CANVAS_STATE_STACK_DEPTH,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanvasSurfaceId {
    scope: NonZeroU64,
    serial: NonZeroU64,
}

impl CanvasSurfaceId {
    pub const fn scope(self) -> u64 {
        self.scope.get()
    }

    pub const fn serial(self) -> u64 {
        self.serial.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanvasContextId {
    scope: NonZeroU64,
    serial: NonZeroU64,
}

impl CanvasContextId {
    pub const fn scope(self) -> u64 {
        self.scope.get()
    }

    pub const fn serial(self) -> u64 {
        self.serial.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanvasExternalContextLease {
    scope: NonZeroU64,
    serial: NonZeroU64,
    surface: CanvasSurfaceId,
}

impl CanvasExternalContextLease {
    pub const fn scope(self) -> u64 {
        self.scope.get()
    }

    pub const fn serial(self) -> u64 {
        self.serial.get()
    }

    pub const fn surface(self) -> CanvasSurfaceId {
        self.surface
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanvasContentRevision(u64);

impl CanvasContentRevision {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasSurfaceSnapshot {
    id: CanvasSurfaceId,
    width: u32,
    height: u32,
    content_revision: CanvasContentRevision,
    pixels: Arc<[Color]>,
}

impl CanvasSurfaceSnapshot {
    pub const fn id(&self) -> CanvasSurfaceId {
        self.id
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub const fn content_revision(&self) -> CanvasContentRevision {
        self.content_revision
    }

    pub fn pixels(&self) -> &[Color] {
        &self.pixels
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasTransform {
    pub m11: f32,
    pub m12: f32,
    pub m21: f32,
    pub m22: f32,
    pub tx: f32,
    pub ty: f32,
}

impl CanvasTransform {
    pub const IDENTITY: Self = Self {
        m11: 1.0,
        m12: 0.0,
        m21: 0.0,
        m22: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    pub fn is_finite(self) -> bool {
        [self.m11, self.m12, self.m21, self.m22, self.tx, self.ty]
            .into_iter()
            .all(f32::is_finite)
    }
}

impl Default for CanvasTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Canvas2dState {
    fill_color: Color,
    stroke_color: Color,
    global_alpha: f32,
    line_width: f32,
    transform: CanvasTransform,
}

impl Canvas2dState {
    pub const fn fill_color(self) -> Color {
        self.fill_color
    }

    pub const fn stroke_color(self) -> Color {
        self.stroke_color
    }

    pub const fn global_alpha(self) -> f32 {
        self.global_alpha
    }

    pub const fn line_width(self) -> f32 {
        self.line_width
    }

    pub const fn transform(self) -> CanvasTransform {
        self.transform
    }
}

impl Default for Canvas2dState {
    fn default() -> Self {
        Self {
            fill_color: Color::BLACK,
            stroke_color: Color::BLACK,
            global_alpha: 1.0,
            line_width: 1.0,
            transform: CanvasTransform::IDENTITY,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasSurface {
    id: CanvasSurfaceId,
    width: u32,
    height: u32,
    pixels: u64,
    content_revision: CanvasContentRevision,
    output: Arc<[Color]>,
    context: Option<CanvasContextId>,
    external_context: Option<CanvasExternalContextLease>,
}

impl CanvasSurface {
    pub const fn id(&self) -> CanvasSurfaceId {
        self.id
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub const fn pixel_count(&self) -> u64 {
        self.pixels
    }

    pub const fn content_revision(&self) -> CanvasContentRevision {
        self.content_revision
    }

    pub const fn context(&self) -> Option<CanvasContextId> {
        self.context
    }

    pub const fn external_context(&self) -> Option<CanvasExternalContextLease> {
        self.external_context
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Canvas2dContext {
    id: CanvasContextId,
    surface: CanvasSurfaceId,
    state: Canvas2dState,
    saved_states: Vec<Canvas2dState>,
}

impl Canvas2dContext {
    pub const fn id(&self) -> CanvasContextId {
        self.id
    }

    pub const fn surface(&self) -> CanvasSurfaceId {
        self.surface
    }

    pub const fn state(&self) -> Canvas2dState {
        self.state
    }

    pub fn saved_state_depth(&self) -> usize {
        self.saved_states.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CanvasError {
    InvalidLimits,
    RegistryScopeExhausted,
    InvalidDimensions,
    SurfaceLimitExceeded { surfaces: usize, limit: usize },
    ContextLimitExceeded { contexts: usize, limit: usize },
    SurfacePixelLimitExceeded { pixels: u64, limit: u64 },
    TotalPixelOverflow,
    TotalPixelLimitExceeded { pixels: u64, limit: u64 },
    PixelAllocationFailed { pixels: u64 },
    ContentRevisionExhausted(CanvasSurfaceId),
    SurfaceIdentitySpaceExhausted,
    ContextIdentitySpaceExhausted,
    ExternalContextIdentitySpaceExhausted,
    UnknownSurface(CanvasSurfaceId),
    UnknownContext(CanvasContextId),
    UnknownExternalContextLease(CanvasExternalContextLease),
    SurfaceAlreadyHasContext(CanvasSurfaceId),
    SurfaceHasLiveContext(CanvasSurfaceId),
    StateStackLimitExceeded { depth: usize, limit: usize },
    InvalidGlobalAlpha(f32),
    InvalidLineWidth(f32),
    InvalidTransform,
    InconsistentState,
}

impl fmt::Display for CanvasError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => formatter.write_str("Canvas limits are invalid"),
            Self::RegistryScopeExhausted => {
                formatter.write_str("Canvas registry scope space is exhausted")
            }
            Self::InvalidDimensions => {
                formatter.write_str("Canvas surface dimensions must be non-zero")
            }
            Self::SurfaceLimitExceeded { surfaces, limit } => write!(
                formatter,
                "Canvas registry would contain {surfaces} surfaces; limit is {limit}"
            ),
            Self::ContextLimitExceeded { contexts, limit } => write!(
                formatter,
                "Canvas registry would contain {contexts} contexts; limit is {limit}"
            ),
            Self::SurfacePixelLimitExceeded { pixels, limit } => write!(
                formatter,
                "Canvas surface requires {pixels} pixels; per-surface limit is {limit}"
            ),
            Self::TotalPixelOverflow => {
                formatter.write_str("Canvas aggregate pixel accounting overflowed")
            }
            Self::TotalPixelLimitExceeded { pixels, limit } => write!(
                formatter,
                "Canvas registry would retain {pixels} pixels; total limit is {limit}"
            ),
            Self::PixelAllocationFailed { pixels } => write!(
                formatter,
                "Canvas surface could not allocate a bounded output buffer for {pixels} pixels"
            ),
            Self::ContentRevisionExhausted(id) => write!(
                formatter,
                "Canvas surface {}:{} content revision space is exhausted",
                id.scope(),
                id.serial()
            ),
            Self::SurfaceIdentitySpaceExhausted => {
                formatter.write_str("Canvas surface identity space is exhausted")
            }
            Self::ContextIdentitySpaceExhausted => {
                formatter.write_str("Canvas context identity space is exhausted")
            }
            Self::ExternalContextIdentitySpaceExhausted => {
                formatter.write_str("Canvas external-context lease identity space is exhausted")
            }
            Self::UnknownSurface(id) => write!(
                formatter,
                "unknown Canvas surface {}:{}",
                id.scope(),
                id.serial()
            ),
            Self::UnknownContext(id) => write!(
                formatter,
                "unknown Canvas context {}:{}",
                id.scope(),
                id.serial()
            ),
            Self::UnknownExternalContextLease(lease) => write!(
                formatter,
                "unknown Canvas external-context lease {}:{} for surface {}:{}",
                lease.scope(),
                lease.serial(),
                lease.surface().scope(),
                lease.surface().serial()
            ),
            Self::SurfaceAlreadyHasContext(id) => write!(
                formatter,
                "Canvas surface {}:{} already has a rendering context",
                id.scope(),
                id.serial()
            ),
            Self::SurfaceHasLiveContext(id) => write!(
                formatter,
                "Canvas surface {}:{} cannot retire while its rendering context is live",
                id.scope(),
                id.serial()
            ),
            Self::StateStackLimitExceeded { depth, limit } => write!(
                formatter,
                "Canvas saved-state stack would reach depth {depth}; limit is {limit}"
            ),
            Self::InvalidGlobalAlpha(value) => {
                write!(formatter, "invalid Canvas global alpha {value}")
            }
            Self::InvalidLineWidth(value) => write!(formatter, "invalid Canvas line width {value}"),
            Self::InvalidTransform => formatter.write_str("Canvas transform must be finite"),
            Self::InconsistentState => {
                formatter.write_str("Canvas ownership state is inconsistent")
            }
        }
    }
}

impl std::error::Error for CanvasError {}

#[derive(Clone, Debug)]
struct IdentityAllocator {
    scope: NonZeroU64,
    next_serial: u64,
}

impl IdentityAllocator {
    const fn new(scope: NonZeroU64) -> Self {
        Self {
            scope,
            next_serial: 1,
        }
    }

    fn allocate_surface(&mut self) -> Result<CanvasSurfaceId, CanvasError> {
        let serial =
            NonZeroU64::new(self.next_serial).ok_or(CanvasError::SurfaceIdentitySpaceExhausted)?;
        self.next_serial = self.next_serial.checked_add(1).unwrap_or(0);
        Ok(CanvasSurfaceId {
            scope: self.scope,
            serial,
        })
    }

    fn allocate_context(&mut self) -> Result<CanvasContextId, CanvasError> {
        let serial =
            NonZeroU64::new(self.next_serial).ok_or(CanvasError::ContextIdentitySpaceExhausted)?;
        self.next_serial = self.next_serial.checked_add(1).unwrap_or(0);
        Ok(CanvasContextId {
            scope: self.scope,
            serial,
        })
    }

    fn allocate_external_context(
        &mut self,
        surface: CanvasSurfaceId,
    ) -> Result<CanvasExternalContextLease, CanvasError> {
        let serial = NonZeroU64::new(self.next_serial)
            .ok_or(CanvasError::ExternalContextIdentitySpaceExhausted)?;
        self.next_serial = self.next_serial.checked_add(1).unwrap_or(0);
        Ok(CanvasExternalContextLease {
            scope: self.scope,
            serial,
            surface,
        })
    }
}

#[derive(Debug)]
pub struct CanvasRegistry {
    limits: CanvasLimits,
    surface_ids: IdentityAllocator,
    context_ids: IdentityAllocator,
    external_context_ids: IdentityAllocator,
    surfaces: BTreeMap<CanvasSurfaceId, CanvasSurface>,
    contexts: BTreeMap<CanvasContextId, Canvas2dContext>,
    total_pixels: u64,
}

impl CanvasRegistry {
    pub fn try_new(limits: CanvasLimits) -> Result<Self, CanvasError> {
        if !limits.is_valid() {
            return Err(CanvasError::InvalidLimits);
        }
        let scope = allocate_registry_scope()?;
        Ok(Self {
            limits,
            surface_ids: IdentityAllocator::new(scope),
            context_ids: IdentityAllocator::new(scope),
            external_context_ids: IdentityAllocator::new(scope),
            surfaces: BTreeMap::new(),
            contexts: BTreeMap::new(),
            total_pixels: 0,
        })
    }

    pub fn limits(&self) -> CanvasLimits {
        self.limits
    }

    pub fn surface_count(&self) -> usize {
        self.surfaces.len()
    }

    pub fn context_count(&self) -> usize {
        self.contexts.len()
    }

    pub const fn total_pixels(&self) -> u64 {
        self.total_pixels
    }

    pub fn surface(&self, id: CanvasSurfaceId) -> Option<&CanvasSurface> {
        self.surfaces.get(&id)
    }

    pub fn context(&self, id: CanvasContextId) -> Option<&Canvas2dContext> {
        self.contexts.get(&id)
    }

    pub fn create_surface(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<CanvasSurfaceId, CanvasError> {
        if width == 0 || height == 0 {
            return Err(CanvasError::InvalidDimensions);
        }
        let next_count =
            self.surfaces
                .len()
                .checked_add(1)
                .ok_or(CanvasError::SurfaceLimitExceeded {
                    surfaces: usize::MAX,
                    limit: self.limits.max_surfaces,
                })?;
        if next_count > self.limits.max_surfaces {
            return Err(CanvasError::SurfaceLimitExceeded {
                surfaces: next_count,
                limit: self.limits.max_surfaces,
            });
        }

        let pixels = u64::from(width) * u64::from(height);
        if pixels > self.limits.max_pixels_per_surface {
            return Err(CanvasError::SurfacePixelLimitExceeded {
                pixels,
                limit: self.limits.max_pixels_per_surface,
            });
        }
        let next_total = self
            .total_pixels
            .checked_add(pixels)
            .ok_or(CanvasError::TotalPixelOverflow)?;
        if next_total > self.limits.max_total_pixels {
            return Err(CanvasError::TotalPixelLimitExceeded {
                pixels: next_total,
                limit: self.limits.max_total_pixels,
            });
        }

        let output = allocate_canvas_pixels(pixels, Color::TRANSPARENT)?;
        let id = self.surface_ids.allocate_surface()?;
        let previous = self.surfaces.insert(
            id,
            CanvasSurface {
                id,
                width,
                height,
                pixels,
                content_revision: CanvasContentRevision::default(),
                output,
                context: None,
                external_context: None,
            },
        );
        debug_assert!(previous.is_none());
        self.total_pixels = next_total;
        Ok(id)
    }

    pub fn retire_surface(&mut self, id: CanvasSurfaceId) -> Result<(), CanvasError> {
        let surface = self
            .surfaces
            .get(&id)
            .ok_or(CanvasError::UnknownSurface(id))?;
        if surface.context.is_some() || surface.external_context.is_some() {
            return Err(CanvasError::SurfaceHasLiveContext(id));
        }
        let next_total = self
            .total_pixels
            .checked_sub(surface.pixels)
            .ok_or(CanvasError::InconsistentState)?;
        self.surfaces.remove(&id);
        self.total_pixels = next_total;
        Ok(())
    }

    pub fn surface_snapshot(
        &self,
        id: CanvasSurfaceId,
    ) -> Result<CanvasSurfaceSnapshot, CanvasError> {
        let surface = self
            .surfaces
            .get(&id)
            .ok_or(CanvasError::UnknownSurface(id))?;
        Ok(CanvasSurfaceSnapshot {
            id,
            width: surface.width,
            height: surface.height,
            content_revision: surface.content_revision,
            pixels: Arc::clone(&surface.output),
        })
    }

    pub fn fill_surface(
        &mut self,
        id: CanvasSurfaceId,
        color: Color,
    ) -> Result<CanvasContentRevision, CanvasError> {
        let (pixels, current_revision) = {
            let surface = self
                .surfaces
                .get(&id)
                .ok_or(CanvasError::UnknownSurface(id))?;
            (surface.pixels, surface.content_revision)
        };
        let next_revision = current_revision
            .0
            .checked_add(1)
            .ok_or(CanvasError::ContentRevisionExhausted(id))?;
        let output = allocate_canvas_pixels(pixels, color)?;
        let surface = self
            .surfaces
            .get_mut(&id)
            .ok_or(CanvasError::InconsistentState)?;
        surface.output = output;
        surface.content_revision = CanvasContentRevision(next_revision);
        Ok(surface.content_revision)
    }

    pub fn clear_surface(
        &mut self,
        id: CanvasSurfaceId,
    ) -> Result<CanvasContentRevision, CanvasError> {
        self.fill_surface(id, Color::TRANSPARENT)
    }

    pub fn create_2d_context(
        &mut self,
        surface: CanvasSurfaceId,
    ) -> Result<CanvasContextId, CanvasError> {
        let owner = self
            .surfaces
            .get(&surface)
            .ok_or(CanvasError::UnknownSurface(surface))?;
        if owner.context.is_some() || owner.external_context.is_some() {
            return Err(CanvasError::SurfaceAlreadyHasContext(surface));
        }
        let next_count =
            self.contexts
                .len()
                .checked_add(1)
                .ok_or(CanvasError::ContextLimitExceeded {
                    contexts: usize::MAX,
                    limit: self.limits.max_contexts,
                })?;
        if next_count > self.limits.max_contexts {
            return Err(CanvasError::ContextLimitExceeded {
                contexts: next_count,
                limit: self.limits.max_contexts,
            });
        }

        let id = self.context_ids.allocate_context()?;
        let previous = self.contexts.insert(
            id,
            Canvas2dContext {
                id,
                surface,
                state: Canvas2dState::default(),
                saved_states: Vec::new(),
            },
        );
        debug_assert!(previous.is_none());
        self.surfaces
            .get_mut(&surface)
            .ok_or(CanvasError::InconsistentState)?
            .context = Some(id);
        Ok(id)
    }

    pub fn retire_context(&mut self, id: CanvasContextId) -> Result<(), CanvasError> {
        let surface = self
            .contexts
            .get(&id)
            .ok_or(CanvasError::UnknownContext(id))?
            .surface;
        let owner = self
            .surfaces
            .get(&surface)
            .ok_or(CanvasError::InconsistentState)?;
        if owner.context != Some(id) {
            return Err(CanvasError::InconsistentState);
        }
        self.contexts.remove(&id);
        self.surfaces
            .get_mut(&surface)
            .ok_or(CanvasError::InconsistentState)?
            .context = None;
        Ok(())
    }

    pub fn acquire_external_context(
        &mut self,
        surface: CanvasSurfaceId,
    ) -> Result<CanvasExternalContextLease, CanvasError> {
        let owner = self
            .surfaces
            .get(&surface)
            .ok_or(CanvasError::UnknownSurface(surface))?;
        if owner.context.is_some() || owner.external_context.is_some() {
            return Err(CanvasError::SurfaceAlreadyHasContext(surface));
        }
        let lease = self
            .external_context_ids
            .allocate_external_context(surface)?;
        self.surfaces
            .get_mut(&surface)
            .ok_or(CanvasError::InconsistentState)?
            .external_context = Some(lease);
        Ok(lease)
    }

    pub fn release_external_context(
        &mut self,
        lease: CanvasExternalContextLease,
    ) -> Result<(), CanvasError> {
        let owner = self
            .surfaces
            .get_mut(&lease.surface)
            .ok_or(CanvasError::UnknownExternalContextLease(lease))?;
        if owner.external_context != Some(lease) {
            return Err(CanvasError::UnknownExternalContextLease(lease));
        }
        owner.external_context = None;
        Ok(())
    }

    pub fn set_fill_color(
        &mut self,
        context: CanvasContextId,
        color: Color,
    ) -> Result<(), CanvasError> {
        self.context_mut(context)?.state.fill_color = color;
        Ok(())
    }

    pub fn set_stroke_color(
        &mut self,
        context: CanvasContextId,
        color: Color,
    ) -> Result<(), CanvasError> {
        self.context_mut(context)?.state.stroke_color = color;
        Ok(())
    }

    pub fn set_global_alpha(
        &mut self,
        context: CanvasContextId,
        alpha: f32,
    ) -> Result<(), CanvasError> {
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return Err(CanvasError::InvalidGlobalAlpha(alpha));
        }
        self.context_mut(context)?.state.global_alpha = alpha;
        Ok(())
    }

    pub fn set_line_width(
        &mut self,
        context: CanvasContextId,
        width: f32,
    ) -> Result<(), CanvasError> {
        if !width.is_finite() || width <= 0.0 {
            return Err(CanvasError::InvalidLineWidth(width));
        }
        self.context_mut(context)?.state.line_width = width;
        Ok(())
    }

    pub fn set_transform(
        &mut self,
        context: CanvasContextId,
        transform: CanvasTransform,
    ) -> Result<(), CanvasError> {
        if !transform.is_finite() {
            return Err(CanvasError::InvalidTransform);
        }
        self.context_mut(context)?.state.transform = transform;
        Ok(())
    }

    pub fn reset_transform(&mut self, context: CanvasContextId) -> Result<(), CanvasError> {
        self.context_mut(context)?.state.transform = CanvasTransform::IDENTITY;
        Ok(())
    }

    pub fn save(&mut self, context: CanvasContextId) -> Result<(), CanvasError> {
        let limit = self.limits.max_state_stack_depth;
        let owner = self.context_mut(context)?;
        let next_depth = owner.saved_states.len().checked_add(1).ok_or(
            CanvasError::StateStackLimitExceeded {
                depth: usize::MAX,
                limit,
            },
        )?;
        if next_depth > limit {
            return Err(CanvasError::StateStackLimitExceeded {
                depth: next_depth,
                limit,
            });
        }
        owner.saved_states.push(owner.state);
        Ok(())
    }

    pub fn restore(&mut self, context: CanvasContextId) -> Result<(), CanvasError> {
        let owner = self.context_mut(context)?;
        if let Some(state) = owner.saved_states.pop() {
            owner.state = state;
        }
        Ok(())
    }

    fn context_mut(&mut self, id: CanvasContextId) -> Result<&mut Canvas2dContext, CanvasError> {
        self.contexts
            .get_mut(&id)
            .ok_or(CanvasError::UnknownContext(id))
    }
}

fn allocate_canvas_pixels(pixels: u64, color: Color) -> Result<Arc<[Color]>, CanvasError> {
    let length =
        usize::try_from(pixels).map_err(|_| CanvasError::PixelAllocationFailed { pixels })?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(length)
        .map_err(|_| CanvasError::PixelAllocationFailed { pixels })?;
    output.resize(length, color);
    Ok(output.into())
}

fn allocate_registry_scope() -> Result<NonZeroU64, CanvasError> {
    let scope = NEXT_CANVAS_REGISTRY_SCOPE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| CanvasError::RegistryScopeExhausted)?;
    NonZeroU64::new(scope).ok_or(CanvasError::RegistryScopeExhausted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_limits() -> CanvasLimits {
        CanvasLimits {
            max_surfaces: 2,
            max_contexts: 2,
            max_pixels_per_surface: 16,
            max_total_pixels: 20,
            max_state_stack_depth: 2,
        }
    }

    #[test]
    fn limits_must_be_non_zero_and_internally_consistent() {
        let limits = CanvasLimits {
            max_surfaces: 0,
            ..CanvasLimits::default()
        };
        assert_eq!(
            CanvasRegistry::try_new(limits).unwrap_err(),
            CanvasError::InvalidLimits
        );

        let defaults = CanvasLimits::default();
        let limits = CanvasLimits {
            max_contexts: defaults.max_surfaces + 1,
            ..defaults
        };
        assert_eq!(
            CanvasRegistry::try_new(limits).unwrap_err(),
            CanvasError::InvalidLimits
        );

        let defaults = CanvasLimits::default();
        let limits = CanvasLimits {
            max_pixels_per_surface: defaults.max_total_pixels + 1,
            ..defaults
        };
        assert_eq!(
            CanvasRegistry::try_new(limits).unwrap_err(),
            CanvasError::InvalidLimits
        );
    }

    #[test]
    fn surface_admission_is_bounded_and_rejected_admission_is_atomic() {
        let mut registry = CanvasRegistry::try_new(tiny_limits()).unwrap();
        assert_eq!(
            registry.create_surface(0, 1),
            Err(CanvasError::InvalidDimensions)
        );
        assert_eq!(registry.surface_count(), 0);
        assert_eq!(registry.total_pixels(), 0);

        let first = registry.create_surface(4, 4).unwrap();
        assert_eq!(registry.total_pixels(), 16);
        assert_eq!(
            registry.create_surface(5, 4),
            Err(CanvasError::SurfacePixelLimitExceeded {
                pixels: 20,
                limit: 16,
            })
        );
        assert_eq!(registry.surface_count(), 1);
        assert_eq!(registry.total_pixels(), 16);

        let second = registry.create_surface(2, 2).unwrap();
        assert_eq!(registry.total_pixels(), 20);
        assert_eq!(
            registry.create_surface(1, 1),
            Err(CanvasError::SurfaceLimitExceeded {
                surfaces: 3,
                limit: 2,
            })
        );
        assert_eq!(registry.surface_count(), 2);
        assert!(registry.surface(first).is_some());
        assert!(registry.surface(second).is_some());
    }

    #[test]
    fn aggregate_pixel_budget_recovers_after_surface_retirement() {
        let mut registry = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let first = registry.create_surface(4, 4).unwrap();
        assert_eq!(
            registry.create_surface(3, 2),
            Err(CanvasError::TotalPixelLimitExceeded {
                pixels: 22,
                limit: 20,
            })
        );
        registry.retire_surface(first).unwrap();
        let replacement = registry.create_surface(4, 4).unwrap();
        assert_ne!(replacement, first);
        assert!(replacement.serial() > first.serial());
        assert_eq!(registry.total_pixels(), 16);
    }

    #[test]
    fn registries_use_distinct_scopes_and_reject_foreign_identities() {
        let mut first = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let mut second = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let first_surface = first.create_surface(1, 1).unwrap();
        let second_surface = second.create_surface(1, 1).unwrap();
        assert_ne!(first_surface.scope(), second_surface.scope());
        assert_eq!(
            second.create_2d_context(first_surface),
            Err(CanvasError::UnknownSurface(first_surface))
        );
        assert!(second.surface(second_surface).is_some());
    }

    #[test]
    fn one_context_owns_each_surface_and_retirement_order_is_explicit() {
        let mut registry = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let surface = registry.create_surface(2, 2).unwrap();
        let context = registry.create_2d_context(surface).unwrap();
        assert_eq!(registry.surface(surface).unwrap().context(), Some(context));
        assert_eq!(registry.context(context).unwrap().surface(), surface);
        assert_eq!(
            registry.create_2d_context(surface),
            Err(CanvasError::SurfaceAlreadyHasContext(surface))
        );
        assert_eq!(
            registry.retire_surface(surface),
            Err(CanvasError::SurfaceHasLiveContext(surface))
        );
        registry.retire_context(context).unwrap();
        assert_eq!(registry.context_count(), 0);
        assert_eq!(registry.surface(surface).unwrap().context(), None);
        registry.retire_surface(surface).unwrap();
        assert_eq!(registry.surface_count(), 0);
    }

    #[test]
    fn retired_context_identity_is_not_reused() {
        let mut registry = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let first_surface = registry.create_surface(1, 1).unwrap();
        let first = registry.create_2d_context(first_surface).unwrap();
        registry.retire_context(first).unwrap();
        registry.retire_surface(first_surface).unwrap();
        let second_surface = registry.create_surface(1, 1).unwrap();
        let second = registry.create_2d_context(second_surface).unwrap();
        assert_ne!(first, second);
        assert!(second.serial() > first.serial());
        assert_eq!(
            registry.set_fill_color(first, Color::WHITE),
            Err(CanvasError::UnknownContext(first))
        );
    }

    #[test]
    fn default_state_is_deterministic() {
        let mut registry = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let surface = registry.create_surface(1, 1).unwrap();
        let context = registry.create_2d_context(surface).unwrap();
        let state = registry.context(context).unwrap().state();
        assert_eq!(state.fill_color(), Color::BLACK);
        assert_eq!(state.stroke_color(), Color::BLACK);
        assert_eq!(state.global_alpha(), 1.0);
        assert_eq!(state.line_width(), 1.0);
        assert_eq!(state.transform(), CanvasTransform::IDENTITY);
    }

    #[test]
    fn invalid_state_setters_are_atomic() {
        let mut registry = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let surface = registry.create_surface(1, 1).unwrap();
        let context = registry.create_2d_context(surface).unwrap();
        let original = registry.context(context).unwrap().state();

        assert!(matches!(
            registry.set_global_alpha(context, f32::NAN),
            Err(CanvasError::InvalidGlobalAlpha(value)) if value.is_nan()
        ));
        assert_eq!(registry.context(context).unwrap().state(), original);
        assert_eq!(
            registry.set_global_alpha(context, 1.1),
            Err(CanvasError::InvalidGlobalAlpha(1.1))
        );
        assert_eq!(registry.context(context).unwrap().state(), original);
        assert_eq!(
            registry.set_line_width(context, 0.0),
            Err(CanvasError::InvalidLineWidth(0.0))
        );
        assert_eq!(registry.context(context).unwrap().state(), original);
        assert_eq!(
            registry.set_transform(
                context,
                CanvasTransform {
                    tx: f32::INFINITY,
                    ..CanvasTransform::IDENTITY
                }
            ),
            Err(CanvasError::InvalidTransform)
        );
        assert_eq!(registry.context(context).unwrap().state(), original);
    }

    #[test]
    fn state_updates_are_portable_and_isolated_per_context() {
        let mut registry = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let first_surface = registry.create_surface(1, 1).unwrap();
        let second_surface = registry.create_surface(1, 1).unwrap();
        let first = registry.create_2d_context(first_surface).unwrap();
        let second = registry.create_2d_context(second_surface).unwrap();

        registry.set_fill_color(first, Color::WHITE).unwrap();
        registry
            .set_stroke_color(first, Color::TRANSPARENT)
            .unwrap();
        registry.set_global_alpha(first, 0.5).unwrap();
        registry.set_line_width(first, 3.0).unwrap();
        registry
            .set_transform(
                first,
                CanvasTransform {
                    tx: 10.0,
                    ty: 20.0,
                    ..CanvasTransform::IDENTITY
                },
            )
            .unwrap();

        let first_state = registry.context(first).unwrap().state();
        assert_eq!(first_state.fill_color(), Color::WHITE);
        assert_eq!(first_state.stroke_color(), Color::TRANSPARENT);
        assert_eq!(first_state.global_alpha(), 0.5);
        assert_eq!(first_state.line_width(), 3.0);
        assert_eq!(first_state.transform().tx, 10.0);
        assert_eq!(
            registry.context(second).unwrap().state(),
            Canvas2dState::default()
        );
    }

    #[test]
    fn save_restore_is_bounded_and_overflow_does_not_mutate_stack() {
        let mut registry = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let surface = registry.create_surface(1, 1).unwrap();
        let context = registry.create_2d_context(surface).unwrap();

        registry.set_fill_color(context, Color::WHITE).unwrap();
        registry.save(context).unwrap();
        registry.set_global_alpha(context, 0.5).unwrap();
        registry.save(context).unwrap();
        let before = registry.context(context).unwrap().clone();
        assert_eq!(
            registry.save(context),
            Err(CanvasError::StateStackLimitExceeded { depth: 3, limit: 2 })
        );
        assert_eq!(registry.context(context).unwrap(), &before);

        registry.set_line_width(context, 4.0).unwrap();
        registry.restore(context).unwrap();
        assert_eq!(
            registry.context(context).unwrap().state().global_alpha(),
            0.5
        );
        assert_eq!(registry.context(context).unwrap().state().line_width(), 1.0);
        registry.restore(context).unwrap();
        assert_eq!(
            registry.context(context).unwrap().state().fill_color(),
            Color::WHITE
        );
        assert_eq!(
            registry.context(context).unwrap().state().global_alpha(),
            1.0
        );
        assert_eq!(registry.context(context).unwrap().saved_state_depth(), 0);
    }

    #[test]
    fn restoring_empty_stack_is_a_no_op() {
        let mut registry = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let surface = registry.create_surface(1, 1).unwrap();
        let context = registry.create_2d_context(surface).unwrap();
        registry.set_fill_color(context, Color::WHITE).unwrap();
        let before = registry.context(context).unwrap().clone();
        registry.restore(context).unwrap();
        assert_eq!(registry.context(context).unwrap(), &before);
    }

    #[test]
    fn context_capacity_recovers_without_reusing_identity() {
        let limits = CanvasLimits {
            max_surfaces: 2,
            max_contexts: 1,
            max_pixels_per_surface: 4,
            max_total_pixels: 8,
            max_state_stack_depth: 1,
        };
        let mut registry = CanvasRegistry::try_new(limits).unwrap();
        let first_surface = registry.create_surface(1, 1).unwrap();
        let second_surface = registry.create_surface(1, 1).unwrap();
        let first = registry.create_2d_context(first_surface).unwrap();
        assert_eq!(
            registry.create_2d_context(second_surface),
            Err(CanvasError::ContextLimitExceeded {
                contexts: 2,
                limit: 1,
            })
        );
        assert_eq!(registry.context_count(), 1);
        registry.retire_context(first).unwrap();
        let second = registry.create_2d_context(second_surface).unwrap();
        assert_ne!(first, second);
        assert!(second.serial() > first.serial());
    }

    #[test]
    fn surface_output_starts_transparent_and_snapshots_are_immutable() {
        let mut registry = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let surface = registry.create_surface(2, 2).unwrap();
        let initial = registry.surface_snapshot(surface).unwrap();
        assert_eq!(initial.id(), surface);
        assert_eq!(initial.width(), 2);
        assert_eq!(initial.height(), 2);
        assert_eq!(initial.content_revision().get(), 0);
        assert_eq!(initial.pixels(), &[Color::TRANSPARENT; 4]);

        let revision = registry.fill_surface(surface, Color::WHITE).unwrap();
        assert_eq!(revision.get(), 1);
        let changed = registry.surface_snapshot(surface).unwrap();
        assert_eq!(changed.content_revision(), revision);
        assert_eq!(changed.pixels(), &[Color::WHITE; 4]);
        assert_eq!(initial.pixels(), &[Color::TRANSPARENT; 4]);

        let cleared = registry.clear_surface(surface).unwrap();
        assert_eq!(cleared.get(), 2);
        assert_eq!(
            registry.surface_snapshot(surface).unwrap().pixels(),
            &[Color::TRANSPARENT; 4]
        );
    }

    #[test]
    fn output_mutation_rejects_foreign_and_retired_surface_identities() {
        let mut first = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let mut second = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let foreign = first.create_surface(1, 1).unwrap();
        let retired = second.create_surface(1, 1).unwrap();
        second.retire_surface(retired).unwrap();

        assert_eq!(
            second.fill_surface(foreign, Color::WHITE),
            Err(CanvasError::UnknownSurface(foreign))
        );
        assert_eq!(
            second.surface_snapshot(retired),
            Err(CanvasError::UnknownSurface(retired))
        );
    }

    #[test]
    fn content_revision_exhaustion_is_atomic() {
        let mut registry = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let surface = registry.create_surface(1, 1).unwrap();
        registry
            .surfaces
            .get_mut(&surface)
            .unwrap()
            .content_revision = CanvasContentRevision(u64::MAX);
        let before = registry.surface_snapshot(surface).unwrap();

        assert_eq!(
            registry.fill_surface(surface, Color::WHITE),
            Err(CanvasError::ContentRevisionExhausted(surface))
        );
        assert_eq!(registry.surface_snapshot(surface).unwrap(), before);
    }

    #[test]
    fn external_context_is_exclusive_with_2d_and_surface_retirement() {
        let mut registry = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let surface = registry.create_surface(2, 2).unwrap();
        let lease = registry.acquire_external_context(surface).unwrap();
        assert_eq!(
            registry.surface(surface).unwrap().external_context(),
            Some(lease)
        );
        assert_eq!(
            registry.create_2d_context(surface),
            Err(CanvasError::SurfaceAlreadyHasContext(surface))
        );
        assert_eq!(
            registry.retire_surface(surface),
            Err(CanvasError::SurfaceHasLiveContext(surface))
        );
        registry.release_external_context(lease).unwrap();
        assert!(registry.create_2d_context(surface).is_ok());
    }

    #[test]
    fn two_d_context_blocks_external_context_and_stale_lease_fails_closed() {
        let mut registry = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let surface = registry.create_surface(1, 1).unwrap();
        let context = registry.create_2d_context(surface).unwrap();
        assert_eq!(
            registry.acquire_external_context(surface),
            Err(CanvasError::SurfaceAlreadyHasContext(surface))
        );
        registry.retire_context(context).unwrap();
        let first = registry.acquire_external_context(surface).unwrap();
        registry.release_external_context(first).unwrap();
        let second = registry.acquire_external_context(surface).unwrap();
        assert_ne!(first, second);
        assert!(second.serial() > first.serial());
        assert_eq!(
            registry.release_external_context(first),
            Err(CanvasError::UnknownExternalContextLease(first))
        );
        assert_eq!(
            registry.surface(surface).unwrap().external_context(),
            Some(second)
        );
    }

    #[test]
    fn foreign_external_context_authority_is_rejected() {
        let mut first = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let mut second = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let surface = first.create_surface(1, 1).unwrap();
        let lease = first.acquire_external_context(surface).unwrap();
        assert_eq!(
            second.acquire_external_context(surface),
            Err(CanvasError::UnknownSurface(surface))
        );
        assert_eq!(
            second.release_external_context(lease),
            Err(CanvasError::UnknownExternalContextLease(lease))
        );
    }
}
