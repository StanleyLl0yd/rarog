use rarog_canvas::{CanvasError, CanvasExternalContextLease, CanvasRegistry, CanvasSurfaceId};
use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_MAX_WEBGL_CONTEXTS: usize = 64;
pub const DEFAULT_MAX_WEBGL_RESOURCES_PER_CONTEXT: usize = 1024;
pub const DEFAULT_MAX_WEBGL_RESOURCES: usize = 4096;
pub const DEFAULT_MAX_WEBGL_BUFFER_BYTES: u64 = 64 * 1024 * 1024;
pub const DEFAULT_MAX_TOTAL_WEBGL_BUFFER_BYTES: u64 = 256 * 1024 * 1024;
pub const DEFAULT_MAX_WEBGL_TEXTURE_DIMENSION: u32 = 8192;
pub const DEFAULT_MAX_WEBGL_TEXTURE_PIXELS: u64 = 16_777_216;
pub const DEFAULT_MAX_TOTAL_WEBGL_TEXTURE_PIXELS: u64 = 67_108_864;

static NEXT_WEBGL_REGISTRY_SCOPE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebGlLimits {
    pub max_contexts: usize,
    pub max_resources_per_context: usize,
    pub max_resources: usize,
    pub max_buffer_bytes: u64,
    pub max_total_buffer_bytes: u64,
    pub max_texture_dimension: u32,
    pub max_texture_pixels: u64,
    pub max_total_texture_pixels: u64,
}

impl WebGlLimits {
    pub const fn is_valid(self) -> bool {
        self.max_contexts > 0
            && self.max_resources_per_context > 0
            && self.max_resources > 0
            && self.max_resources_per_context <= self.max_resources
            && self.max_buffer_bytes > 0
            && self.max_total_buffer_bytes >= self.max_buffer_bytes
            && self.max_texture_dimension > 0
            && self.max_texture_pixels > 0
            && self.max_total_texture_pixels >= self.max_texture_pixels
    }
}

impl Default for WebGlLimits {
    fn default() -> Self {
        Self {
            max_contexts: DEFAULT_MAX_WEBGL_CONTEXTS,
            max_resources_per_context: DEFAULT_MAX_WEBGL_RESOURCES_PER_CONTEXT,
            max_resources: DEFAULT_MAX_WEBGL_RESOURCES,
            max_buffer_bytes: DEFAULT_MAX_WEBGL_BUFFER_BYTES,
            max_total_buffer_bytes: DEFAULT_MAX_TOTAL_WEBGL_BUFFER_BYTES,
            max_texture_dimension: DEFAULT_MAX_WEBGL_TEXTURE_DIMENSION,
            max_texture_pixels: DEFAULT_MAX_WEBGL_TEXTURE_PIXELS,
            max_total_texture_pixels: DEFAULT_MAX_TOTAL_WEBGL_TEXTURE_PIXELS,
        }
    }
}

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name {
            scope: NonZeroU64,
            serial: NonZeroU64,
        }
        impl $name {
            pub const fn scope(self) -> u64 {
                self.scope.get()
            }
            pub const fn serial(self) -> u64 {
                self.serial.get()
            }
        }
    };
}

typed_id!(WebGlContextId);
typed_id!(WebGlBufferId);
typed_id!(WebGlTextureId);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebGlContextState {
    Active,
    Lost(WebGlContextLossReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebGlContextLossReason {
    Explicit,
    ResourcePressure,
    BackendUnavailable,
    DeviceReset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebGlContextView {
    id: WebGlContextId,
    surface: CanvasSurfaceId,
    state: WebGlContextState,
    resource_count: usize,
    buffer_bytes: u64,
    texture_pixels: u64,
}

impl WebGlContextView {
    pub const fn id(self) -> WebGlContextId {
        self.id
    }
    pub const fn surface(self) -> CanvasSurfaceId {
        self.surface
    }
    pub const fn state(self) -> WebGlContextState {
        self.state
    }
    pub const fn resource_count(self) -> usize {
        self.resource_count
    }
    pub const fn buffer_bytes(self) -> u64 {
        self.buffer_bytes
    }
    pub const fn texture_pixels(self) -> u64 {
        self.texture_pixels
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebGlBufferView {
    id: WebGlBufferId,
    context: WebGlContextId,
    bytes: u64,
}

impl WebGlBufferView {
    pub const fn id(self) -> WebGlBufferId {
        self.id
    }
    pub const fn context(self) -> WebGlContextId {
        self.context
    }
    pub const fn bytes(self) -> u64 {
        self.bytes
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebGlTextureView {
    id: WebGlTextureId,
    context: WebGlContextId,
    width: u32,
    height: u32,
    pixels: u64,
}

impl WebGlTextureView {
    pub const fn id(self) -> WebGlTextureId {
        self.id
    }
    pub const fn context(self) -> WebGlContextId {
        self.context
    }
    pub const fn width(self) -> u32 {
        self.width
    }
    pub const fn height(self) -> u32 {
        self.height
    }
    pub const fn pixels(self) -> u64 {
        self.pixels
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WebGlError {
    InvalidLimits,
    RegistryScopeExhausted,
    ContextIdentitySpaceExhausted,
    BufferIdentitySpaceExhausted,
    TextureIdentitySpaceExhausted,
    ContextLimitExceeded {
        contexts: usize,
        limit: usize,
    },
    ResourceLimitExceeded {
        resources: usize,
        limit: usize,
    },
    ContextResourceLimitExceeded {
        resources: usize,
        limit: usize,
    },
    InvalidBufferSize,
    BufferByteLimitExceeded {
        bytes: u64,
        limit: u64,
    },
    TotalBufferByteOverflow,
    TotalBufferByteLimitExceeded {
        bytes: u64,
        limit: u64,
    },
    InvalidTextureDimensions,
    TextureDimensionLimitExceeded {
        dimension: u32,
        limit: u32,
    },
    TexturePixelLimitExceeded {
        pixels: u64,
        limit: u64,
    },
    TotalTexturePixelOverflow,
    TotalTexturePixelLimitExceeded {
        pixels: u64,
        limit: u64,
    },
    UnknownContext(WebGlContextId),
    UnknownBuffer(WebGlBufferId),
    UnknownTexture(WebGlTextureId),
    ContextLost(WebGlContextId),
    ForeignBuffer {
        buffer: WebGlBufferId,
        context: WebGlContextId,
    },
    ForeignTexture {
        texture: WebGlTextureId,
        context: WebGlContextId,
    },
    Canvas(CanvasError),
    InconsistentState,
}

impl fmt::Display for WebGlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WebGL semantic ownership error: {self:?}")
    }
}
impl std::error::Error for WebGlError {}
impl From<CanvasError> for WebGlError {
    fn from(value: CanvasError) -> Self {
        Self::Canvas(value)
    }
}

#[derive(Clone, Debug)]
struct IdAllocator {
    scope: NonZeroU64,
    next_context: u64,
    next_buffer: u64,
    next_texture: u64,
}

impl IdAllocator {
    const fn new(scope: NonZeroU64) -> Self {
        Self {
            scope,
            next_context: 1,
            next_buffer: 1,
            next_texture: 1,
        }
    }
    fn context(&mut self) -> Result<WebGlContextId, WebGlError> {
        let serial =
            NonZeroU64::new(self.next_context).ok_or(WebGlError::ContextIdentitySpaceExhausted)?;
        self.next_context = self.next_context.checked_add(1).unwrap_or(0);
        Ok(WebGlContextId {
            scope: self.scope,
            serial,
        })
    }
    fn buffer(&mut self) -> Result<WebGlBufferId, WebGlError> {
        let serial =
            NonZeroU64::new(self.next_buffer).ok_or(WebGlError::BufferIdentitySpaceExhausted)?;
        self.next_buffer = self.next_buffer.checked_add(1).unwrap_or(0);
        Ok(WebGlBufferId {
            scope: self.scope,
            serial,
        })
    }
    fn texture(&mut self) -> Result<WebGlTextureId, WebGlError> {
        let serial =
            NonZeroU64::new(self.next_texture).ok_or(WebGlError::TextureIdentitySpaceExhausted)?;
        self.next_texture = self.next_texture.checked_add(1).unwrap_or(0);
        Ok(WebGlTextureId {
            scope: self.scope,
            serial,
        })
    }
}

#[derive(Clone, Debug)]
struct ContextRecord {
    surface: CanvasSurfaceId,
    lease: CanvasExternalContextLease,
    state: WebGlContextState,
    resource_count: usize,
    buffer_bytes: u64,
    texture_pixels: u64,
}

#[derive(Clone, Copy, Debug)]
struct BufferRecord {
    context: WebGlContextId,
    bytes: u64,
}
#[derive(Clone, Copy, Debug)]
struct TextureRecord {
    context: WebGlContextId,
    width: u32,
    height: u32,
    pixels: u64,
}

#[derive(Debug)]
pub struct WebGlRegistry {
    limits: WebGlLimits,
    ids: IdAllocator,
    contexts: BTreeMap<WebGlContextId, ContextRecord>,
    buffers: BTreeMap<WebGlBufferId, BufferRecord>,
    textures: BTreeMap<WebGlTextureId, TextureRecord>,
    total_buffer_bytes: u64,
    total_texture_pixels: u64,
}

impl WebGlRegistry {
    pub fn try_new(limits: WebGlLimits) -> Result<Self, WebGlError> {
        if !limits.is_valid() {
            return Err(WebGlError::InvalidLimits);
        }
        let scope = allocate_scope()?;
        Ok(Self {
            limits,
            ids: IdAllocator::new(scope),
            contexts: BTreeMap::new(),
            buffers: BTreeMap::new(),
            textures: BTreeMap::new(),
            total_buffer_bytes: 0,
            total_texture_pixels: 0,
        })
    }

    pub const fn limits(&self) -> WebGlLimits {
        self.limits
    }
    pub fn context_count(&self) -> usize {
        self.contexts.len()
    }
    pub fn resource_count(&self) -> usize {
        self.buffers.len() + self.textures.len()
    }
    pub const fn total_buffer_bytes(&self) -> u64 {
        self.total_buffer_bytes
    }
    pub const fn total_texture_pixels(&self) -> u64 {
        self.total_texture_pixels
    }

    pub fn context(&self, id: WebGlContextId) -> Option<WebGlContextView> {
        self.contexts.get(&id).map(|c| WebGlContextView {
            id,
            surface: c.surface,
            state: c.state,
            resource_count: c.resource_count,
            buffer_bytes: c.buffer_bytes,
            texture_pixels: c.texture_pixels,
        })
    }

    pub fn buffer(&self, id: WebGlBufferId) -> Option<WebGlBufferView> {
        self.buffers.get(&id).map(|b| WebGlBufferView {
            id,
            context: b.context,
            bytes: b.bytes,
        })
    }

    pub fn texture(&self, id: WebGlTextureId) -> Option<WebGlTextureView> {
        self.textures.get(&id).map(|t| WebGlTextureView {
            id,
            context: t.context,
            width: t.width,
            height: t.height,
            pixels: t.pixels,
        })
    }

    pub fn create_context(
        &mut self,
        canvas: &mut CanvasRegistry,
        surface: CanvasSurfaceId,
    ) -> Result<WebGlContextId, WebGlError> {
        let next = self
            .contexts
            .len()
            .checked_add(1)
            .ok_or(WebGlError::ContextLimitExceeded {
                contexts: usize::MAX,
                limit: self.limits.max_contexts,
            })?;
        if next > self.limits.max_contexts {
            return Err(WebGlError::ContextLimitExceeded {
                contexts: next,
                limit: self.limits.max_contexts,
            });
        }
        let lease = canvas.acquire_external_context(surface)?;
        let id = match self.ids.context() {
            Ok(id) => id,
            Err(error) => {
                canvas.release_external_context(lease)?;
                return Err(error);
            }
        };
        let old = self.contexts.insert(
            id,
            ContextRecord {
                surface,
                lease,
                state: WebGlContextState::Active,
                resource_count: 0,
                buffer_bytes: 0,
                texture_pixels: 0,
            },
        );
        debug_assert!(old.is_none());
        Ok(id)
    }

    pub fn create_buffer(
        &mut self,
        context: WebGlContextId,
        bytes: u64,
    ) -> Result<WebGlBufferId, WebGlError> {
        if bytes == 0 {
            return Err(WebGlError::InvalidBufferSize);
        }
        if bytes > self.limits.max_buffer_bytes {
            return Err(WebGlError::BufferByteLimitExceeded {
                bytes,
                limit: self.limits.max_buffer_bytes,
            });
        }
        self.require_active_context(context)?;
        self.check_resource_capacity(context)?;
        let next_total = self
            .total_buffer_bytes
            .checked_add(bytes)
            .ok_or(WebGlError::TotalBufferByteOverflow)?;
        if next_total > self.limits.max_total_buffer_bytes {
            return Err(WebGlError::TotalBufferByteLimitExceeded {
                bytes: next_total,
                limit: self.limits.max_total_buffer_bytes,
            });
        }
        let owner = self
            .contexts
            .get(&context)
            .ok_or(WebGlError::InconsistentState)?;
        let next_owner_resources = owner
            .resource_count
            .checked_add(1)
            .ok_or(WebGlError::InconsistentState)?;
        let next_owner_bytes = owner
            .buffer_bytes
            .checked_add(bytes)
            .ok_or(WebGlError::InconsistentState)?;
        let id = self.ids.buffer()?;
        let old = self.buffers.insert(id, BufferRecord { context, bytes });
        debug_assert!(old.is_none());
        let owner = self
            .contexts
            .get_mut(&context)
            .ok_or(WebGlError::InconsistentState)?;
        owner.resource_count = next_owner_resources;
        owner.buffer_bytes = next_owner_bytes;
        self.total_buffer_bytes = next_total;
        Ok(id)
    }

    pub fn create_texture(
        &mut self,
        context: WebGlContextId,
        width: u32,
        height: u32,
    ) -> Result<WebGlTextureId, WebGlError> {
        if width == 0 || height == 0 {
            return Err(WebGlError::InvalidTextureDimensions);
        }
        let dimension = width.max(height);
        if dimension > self.limits.max_texture_dimension {
            return Err(WebGlError::TextureDimensionLimitExceeded {
                dimension,
                limit: self.limits.max_texture_dimension,
            });
        }
        let pixels = u64::from(width) * u64::from(height);
        if pixels > self.limits.max_texture_pixels {
            return Err(WebGlError::TexturePixelLimitExceeded {
                pixels,
                limit: self.limits.max_texture_pixels,
            });
        }
        self.require_active_context(context)?;
        self.check_resource_capacity(context)?;
        let next_total = self
            .total_texture_pixels
            .checked_add(pixels)
            .ok_or(WebGlError::TotalTexturePixelOverflow)?;
        if next_total > self.limits.max_total_texture_pixels {
            return Err(WebGlError::TotalTexturePixelLimitExceeded {
                pixels: next_total,
                limit: self.limits.max_total_texture_pixels,
            });
        }
        let owner = self
            .contexts
            .get(&context)
            .ok_or(WebGlError::InconsistentState)?;
        let next_owner_resources = owner
            .resource_count
            .checked_add(1)
            .ok_or(WebGlError::InconsistentState)?;
        let next_owner_pixels = owner
            .texture_pixels
            .checked_add(pixels)
            .ok_or(WebGlError::InconsistentState)?;
        let id = self.ids.texture()?;
        let old = self.textures.insert(
            id,
            TextureRecord {
                context,
                width,
                height,
                pixels,
            },
        );
        debug_assert!(old.is_none());
        let owner = self
            .contexts
            .get_mut(&context)
            .ok_or(WebGlError::InconsistentState)?;
        owner.resource_count = next_owner_resources;
        owner.texture_pixels = next_owner_pixels;
        self.total_texture_pixels = next_total;
        Ok(id)
    }

    pub fn destroy_buffer(
        &mut self,
        context: WebGlContextId,
        buffer: WebGlBufferId,
    ) -> Result<(), WebGlError> {
        let record = *self
            .buffers
            .get(&buffer)
            .ok_or(WebGlError::UnknownBuffer(buffer))?;
        if record.context != context {
            return Err(WebGlError::ForeignBuffer { buffer, context });
        }
        self.buffers.remove(&buffer);
        let owner = self
            .contexts
            .get_mut(&context)
            .ok_or(WebGlError::InconsistentState)?;
        owner.resource_count = owner
            .resource_count
            .checked_sub(1)
            .ok_or(WebGlError::InconsistentState)?;
        owner.buffer_bytes = owner
            .buffer_bytes
            .checked_sub(record.bytes)
            .ok_or(WebGlError::InconsistentState)?;
        self.total_buffer_bytes = self
            .total_buffer_bytes
            .checked_sub(record.bytes)
            .ok_or(WebGlError::InconsistentState)?;
        Ok(())
    }

    pub fn destroy_texture(
        &mut self,
        context: WebGlContextId,
        texture: WebGlTextureId,
    ) -> Result<(), WebGlError> {
        let record = *self
            .textures
            .get(&texture)
            .ok_or(WebGlError::UnknownTexture(texture))?;
        if record.context != context {
            return Err(WebGlError::ForeignTexture { texture, context });
        }
        self.textures.remove(&texture);
        let owner = self
            .contexts
            .get_mut(&context)
            .ok_or(WebGlError::InconsistentState)?;
        owner.resource_count = owner
            .resource_count
            .checked_sub(1)
            .ok_or(WebGlError::InconsistentState)?;
        owner.texture_pixels = owner
            .texture_pixels
            .checked_sub(record.pixels)
            .ok_or(WebGlError::InconsistentState)?;
        self.total_texture_pixels = self
            .total_texture_pixels
            .checked_sub(record.pixels)
            .ok_or(WebGlError::InconsistentState)?;
        Ok(())
    }

    pub fn lose_context(
        &mut self,
        context: WebGlContextId,
        reason: WebGlContextLossReason,
    ) -> Result<bool, WebGlError> {
        let state = self
            .contexts
            .get(&context)
            .ok_or(WebGlError::UnknownContext(context))?
            .state;
        if matches!(state, WebGlContextState::Lost(_)) {
            return Ok(false);
        }
        self.retire_resources(context)?;
        self.contexts
            .get_mut(&context)
            .ok_or(WebGlError::InconsistentState)?
            .state = WebGlContextState::Lost(reason);
        Ok(true)
    }

    pub fn destroy_context(
        &mut self,
        canvas: &mut CanvasRegistry,
        context: WebGlContextId,
    ) -> Result<(), WebGlError> {
        let record = self
            .contexts
            .get(&context)
            .ok_or(WebGlError::UnknownContext(context))?;
        let lease = record.lease;
        if canvas
            .surface(record.surface)
            .and_then(|surface| surface.external_context())
            != Some(lease)
        {
            return Err(WebGlError::InconsistentState);
        }
        self.retire_resources(context)?;
        canvas.release_external_context(lease)?;
        self.contexts.remove(&context);
        Ok(())
    }

    fn require_active_context(&self, context: WebGlContextId) -> Result<(), WebGlError> {
        match self
            .contexts
            .get(&context)
            .ok_or(WebGlError::UnknownContext(context))?
            .state
        {
            WebGlContextState::Active => Ok(()),
            WebGlContextState::Lost(_) => Err(WebGlError::ContextLost(context)),
        }
    }

    fn check_resource_capacity(&self, context: WebGlContextId) -> Result<(), WebGlError> {
        let owner = self
            .contexts
            .get(&context)
            .ok_or(WebGlError::UnknownContext(context))?;
        let context_next = owner.resource_count.checked_add(1).ok_or(
            WebGlError::ContextResourceLimitExceeded {
                resources: usize::MAX,
                limit: self.limits.max_resources_per_context,
            },
        )?;
        if context_next > self.limits.max_resources_per_context {
            return Err(WebGlError::ContextResourceLimitExceeded {
                resources: context_next,
                limit: self.limits.max_resources_per_context,
            });
        }
        let current_total = self.buffers.len().checked_add(self.textures.len()).ok_or(
            WebGlError::ResourceLimitExceeded {
                resources: usize::MAX,
                limit: self.limits.max_resources,
            },
        )?;
        let total_next = current_total
            .checked_add(1)
            .ok_or(WebGlError::ResourceLimitExceeded {
                resources: usize::MAX,
                limit: self.limits.max_resources,
            })?;
        if total_next > self.limits.max_resources {
            return Err(WebGlError::ResourceLimitExceeded {
                resources: total_next,
                limit: self.limits.max_resources,
            });
        }
        Ok(())
    }

    fn retire_resources(&mut self, context: WebGlContextId) -> Result<(), WebGlError> {
        let (buffer_bytes, texture_pixels) = {
            let owner = self
                .contexts
                .get(&context)
                .ok_or(WebGlError::UnknownContext(context))?;
            (owner.buffer_bytes, owner.texture_pixels)
        };
        self.buffers.retain(|_, record| record.context != context);
        self.textures.retain(|_, record| record.context != context);
        self.total_buffer_bytes = self
            .total_buffer_bytes
            .checked_sub(buffer_bytes)
            .ok_or(WebGlError::InconsistentState)?;
        self.total_texture_pixels = self
            .total_texture_pixels
            .checked_sub(texture_pixels)
            .ok_or(WebGlError::InconsistentState)?;
        let owner = self
            .contexts
            .get_mut(&context)
            .ok_or(WebGlError::InconsistentState)?;
        owner.resource_count = 0;
        owner.buffer_bytes = 0;
        owner.texture_pixels = 0;
        Ok(())
    }
}

fn allocate_scope() -> Result<NonZeroU64, WebGlError> {
    let scope = NEXT_WEBGL_REGISTRY_SCOPE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| WebGlError::RegistryScopeExhausted)?;
    NonZeroU64::new(scope).ok_or(WebGlError::RegistryScopeExhausted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_canvas::CanvasLimits;

    fn canvas() -> CanvasRegistry {
        CanvasRegistry::try_new(CanvasLimits {
            max_surfaces: 4,
            max_contexts: 4,
            max_pixels_per_surface: 64,
            max_total_pixels: 128,
            max_state_stack_depth: 4,
        })
        .unwrap()
    }

    fn limits() -> WebGlLimits {
        WebGlLimits {
            max_contexts: 2,
            max_resources_per_context: 2,
            max_resources: 3,
            max_buffer_bytes: 8,
            max_total_buffer_bytes: 12,
            max_texture_dimension: 4,
            max_texture_pixels: 8,
            max_total_texture_pixels: 12,
        }
    }

    #[test]
    fn context_lease_excludes_2d_and_is_released_on_destroy() {
        let mut canvas = canvas();
        let surface = canvas.create_surface(2, 2).unwrap();
        let mut webgl = WebGlRegistry::try_new(limits()).unwrap();
        let context = webgl.create_context(&mut canvas, surface).unwrap();
        assert!(canvas.create_2d_context(surface).is_err());
        webgl.destroy_context(&mut canvas, context).unwrap();
        assert!(canvas.create_2d_context(surface).is_ok());
    }

    #[test]
    fn resources_are_bounded_and_context_owned() {
        let mut canvas = canvas();
        let a = canvas.create_surface(2, 2).unwrap();
        let b = canvas.create_surface(2, 2).unwrap();
        let mut webgl = WebGlRegistry::try_new(limits()).unwrap();
        let ca = webgl.create_context(&mut canvas, a).unwrap();
        let cb = webgl.create_context(&mut canvas, b).unwrap();
        let buffer = webgl.create_buffer(ca, 8).unwrap();
        let texture = webgl.create_texture(ca, 2, 2).unwrap();
        assert!(matches!(
            webgl.create_buffer(ca, 1),
            Err(WebGlError::ContextResourceLimitExceeded { .. })
        ));
        assert!(matches!(
            webgl.destroy_buffer(cb, buffer),
            Err(WebGlError::ForeignBuffer { .. })
        ));
        assert!(matches!(
            webgl.destroy_texture(cb, texture),
            Err(WebGlError::ForeignTexture { .. })
        ));
    }

    #[test]
    fn loss_retires_resources_without_reusing_authority() {
        let mut canvas = canvas();
        let surface = canvas.create_surface(2, 2).unwrap();
        let mut webgl = WebGlRegistry::try_new(limits()).unwrap();
        let context = webgl.create_context(&mut canvas, surface).unwrap();
        let buffer = webgl.create_buffer(context, 8).unwrap();
        let texture = webgl.create_texture(context, 2, 2).unwrap();
        assert!(
            webgl
                .lose_context(context, WebGlContextLossReason::Explicit)
                .unwrap()
        );
        assert!(
            !webgl
                .lose_context(context, WebGlContextLossReason::DeviceReset)
                .unwrap()
        );
        assert!(webgl.buffer(buffer).is_none());
        assert!(webgl.texture(texture).is_none());
        assert_eq!(webgl.total_buffer_bytes(), 0);
        assert_eq!(webgl.total_texture_pixels(), 0);
        assert!(
            matches!(webgl.create_buffer(context, 1), Err(WebGlError::ContextLost(id)) if id == context)
        );
        webgl.destroy_context(&mut canvas, context).unwrap();
    }

    #[test]
    fn scoped_ids_do_not_alias_across_registries() {
        let first = WebGlRegistry::try_new(limits()).unwrap();
        let second = WebGlRegistry::try_new(limits()).unwrap();
        assert_ne!(first.ids.scope, second.ids.scope);
    }
}
