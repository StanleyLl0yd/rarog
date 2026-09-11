#[path = "implementation.rs"]
mod implementation;

use rarog_types::Color;
use std::collections::BTreeMap;
use std::num::NonZeroU64;

pub use implementation::{
    Canvas2dContext, Canvas2dState, CanvasContentRevision, CanvasContextId, CanvasError,
    CanvasLimits, CanvasSurface, CanvasSurfaceId, CanvasSurfaceSnapshot, CanvasTransform,
    DEFAULT_MAX_CANVAS_CONTEXTS, DEFAULT_MAX_CANVAS_PIXELS_PER_SURFACE,
    DEFAULT_MAX_CANVAS_STATE_STACK_DEPTH, DEFAULT_MAX_CANVAS_SURFACES,
    DEFAULT_MAX_TOTAL_CANVAS_PIXELS,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanvasExternalContextLease {
    surface: CanvasSurfaceId,
    serial: NonZeroU64,
}

impl CanvasExternalContextLease {
    pub const fn surface(self) -> CanvasSurfaceId {
        self.surface
    }

    pub const fn serial(self) -> u64 {
        self.serial.get()
    }
}

#[derive(Debug)]
pub struct CanvasRegistry {
    inner: implementation::CanvasRegistry,
    external_contexts: BTreeMap<CanvasSurfaceId, CanvasExternalContextLease>,
    next_external_context_serial: u64,
}

impl CanvasRegistry {
    pub fn try_new(limits: CanvasLimits) -> Result<Self, CanvasError> {
        Ok(Self {
            inner: implementation::CanvasRegistry::try_new(limits)?,
            external_contexts: BTreeMap::new(),
            next_external_context_serial: 1,
        })
    }

    pub fn limits(&self) -> CanvasLimits {
        self.inner.limits()
    }

    pub fn surface_count(&self) -> usize {
        self.inner.surface_count()
    }

    pub fn context_count(&self) -> usize {
        self.inner.context_count()
    }

    pub const fn total_pixels(&self) -> u64 {
        self.inner.total_pixels()
    }

    pub fn surface(&self, id: CanvasSurfaceId) -> Option<&CanvasSurface> {
        self.inner.surface(id)
    }

    pub fn context(&self, id: CanvasContextId) -> Option<&Canvas2dContext> {
        self.inner.context(id)
    }

    pub fn external_context(&self, surface: CanvasSurfaceId) -> Option<CanvasExternalContextLease> {
        self.external_contexts.get(&surface).copied()
    }

    pub fn create_surface(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<CanvasSurfaceId, CanvasError> {
        self.inner.create_surface(width, height)
    }

    pub fn retire_surface(&mut self, id: CanvasSurfaceId) -> Result<(), CanvasError> {
        if self.external_contexts.contains_key(&id) {
            return Err(CanvasError::SurfaceHasLiveContext(id));
        }
        self.inner.retire_surface(id)
    }

    pub fn surface_snapshot(
        &self,
        id: CanvasSurfaceId,
    ) -> Result<CanvasSurfaceSnapshot, CanvasError> {
        self.inner.surface_snapshot(id)
    }

    pub fn fill_surface(
        &mut self,
        id: CanvasSurfaceId,
        color: Color,
    ) -> Result<CanvasContentRevision, CanvasError> {
        self.inner.fill_surface(id, color)
    }

    pub fn clear_surface(
        &mut self,
        id: CanvasSurfaceId,
    ) -> Result<CanvasContentRevision, CanvasError> {
        self.inner.clear_surface(id)
    }

    pub fn create_2d_context(
        &mut self,
        surface: CanvasSurfaceId,
    ) -> Result<CanvasContextId, CanvasError> {
        if self.external_contexts.contains_key(&surface) {
            return Err(CanvasError::SurfaceAlreadyHasContext(surface));
        }
        self.inner.create_2d_context(surface)
    }

    pub fn retire_context(&mut self, id: CanvasContextId) -> Result<(), CanvasError> {
        self.inner.retire_context(id)
    }

    pub fn acquire_external_context(
        &mut self,
        surface: CanvasSurfaceId,
    ) -> Result<CanvasExternalContextLease, CanvasError> {
        let owner = self
            .inner
            .surface(surface)
            .ok_or(CanvasError::UnknownSurface(surface))?;
        if owner.context().is_some() || self.external_contexts.contains_key(&surface) {
            return Err(CanvasError::SurfaceAlreadyHasContext(surface));
        }
        let serial = NonZeroU64::new(self.next_external_context_serial)
            .ok_or(CanvasError::ContextIdentitySpaceExhausted)?;
        self.next_external_context_serial = self
            .next_external_context_serial
            .checked_add(1)
            .unwrap_or(0);
        let lease = CanvasExternalContextLease { surface, serial };
        let previous = self.external_contexts.insert(surface, lease);
        debug_assert!(previous.is_none());
        Ok(lease)
    }

    pub fn release_external_context(
        &mut self,
        lease: CanvasExternalContextLease,
    ) -> Result<(), CanvasError> {
        let current = self
            .external_contexts
            .get(&lease.surface)
            .copied()
            .ok_or(CanvasError::InconsistentState)?;
        if current != lease || self.inner.surface(lease.surface).is_none() {
            return Err(CanvasError::InconsistentState);
        }
        self.external_contexts.remove(&lease.surface);
        Ok(())
    }

    pub fn set_fill_color(
        &mut self,
        context: CanvasContextId,
        color: Color,
    ) -> Result<(), CanvasError> {
        self.inner.set_fill_color(context, color)
    }

    pub fn set_stroke_color(
        &mut self,
        context: CanvasContextId,
        color: Color,
    ) -> Result<(), CanvasError> {
        self.inner.set_stroke_color(context, color)
    }

    pub fn set_global_alpha(
        &mut self,
        context: CanvasContextId,
        alpha: f32,
    ) -> Result<(), CanvasError> {
        self.inner.set_global_alpha(context, alpha)
    }

    pub fn set_line_width(
        &mut self,
        context: CanvasContextId,
        width: f32,
    ) -> Result<(), CanvasError> {
        self.inner.set_line_width(context, width)
    }

    pub fn set_transform(
        &mut self,
        context: CanvasContextId,
        transform: CanvasTransform,
    ) -> Result<(), CanvasError> {
        self.inner.set_transform(context, transform)
    }

    pub fn reset_transform(&mut self, context: CanvasContextId) -> Result<(), CanvasError> {
        self.inner.reset_transform(context)
    }

    pub fn save(&mut self, context: CanvasContextId) -> Result<(), CanvasError> {
        self.inner.save(context)
    }

    pub fn restore(&mut self, context: CanvasContextId) -> Result<(), CanvasError> {
        self.inner.restore(context)
    }
}

#[cfg(test)]
mod external_context_tests {
    use super::*;

    fn limits() -> CanvasLimits {
        CanvasLimits {
            max_surfaces: 3,
            max_contexts: 3,
            max_pixels_per_surface: 16,
            max_total_pixels: 32,
            max_state_stack_depth: 2,
        }
    }

    #[test]
    fn external_context_is_exclusive_with_2d_and_blocks_surface_retirement() {
        let mut registry = CanvasRegistry::try_new(limits()).unwrap();
        let surface = registry.create_surface(2, 2).unwrap();
        let lease = registry.acquire_external_context(surface).unwrap();
        assert_eq!(registry.external_context(surface), Some(lease));
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
    fn two_d_context_blocks_external_context_and_stale_leases_fail_closed() {
        let mut registry = CanvasRegistry::try_new(limits()).unwrap();
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
            Err(CanvasError::InconsistentState)
        );
        assert_eq!(registry.external_context(surface), Some(second));
    }

    #[test]
    fn foreign_surface_and_lease_authority_fail_closed() {
        let mut first = CanvasRegistry::try_new(limits()).unwrap();
        let mut second = CanvasRegistry::try_new(limits()).unwrap();
        let surface = first.create_surface(1, 1).unwrap();
        let lease = first.acquire_external_context(surface).unwrap();
        assert_eq!(
            second.acquire_external_context(surface),
            Err(CanvasError::UnknownSurface(surface))
        );
        assert_eq!(
            second.release_external_context(lease),
            Err(CanvasError::InconsistentState)
        );
    }
}
