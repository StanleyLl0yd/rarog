use rarog_canvas::CanvasRegistry;
use rarog_webgl::{
    WebGlBufferId, WebGlBufferView, WebGlContextId, WebGlContextLossReason, WebGlContextState,
    WebGlContextView, WebGlError, WebGlRegistry, WebGlTextureId, WebGlTextureView,
};
use std::collections::BTreeMap;
use std::fmt;

pub const DEFAULT_MAX_GRAPHICS_CONTEXT_BINDINGS: usize = 64;
pub const DEFAULT_MAX_GRAPHICS_RESOURCE_BINDINGS: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphicsAdapterLimits {
    pub max_context_bindings: usize,
    pub max_resource_bindings: usize,
}

impl GraphicsAdapterLimits {
    pub const fn is_valid(self) -> bool {
        self.max_context_bindings > 0 && self.max_resource_bindings > 0
    }
}

impl Default for GraphicsAdapterLimits {
    fn default() -> Self {
        Self {
            max_context_bindings: DEFAULT_MAX_GRAPHICS_CONTEXT_BINDINGS,
            max_resource_bindings: DEFAULT_MAX_GRAPHICS_RESOURCE_BINDINGS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicsBackendErrorKind {
    Unsupported,
    ResourcePressure,
    Unavailable,
    DeviceReset,
    InvalidState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphicsBackendError {
    kind: GraphicsBackendErrorKind,
}

impl GraphicsBackendError {
    pub const fn new(kind: GraphicsBackendErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> GraphicsBackendErrorKind {
        self.kind
    }
}

impl fmt::Display for GraphicsBackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            GraphicsBackendErrorKind::Unsupported => "graphics backend operation is unsupported",
            GraphicsBackendErrorKind::ResourcePressure => {
                "graphics backend operation exceeded backend resources"
            }
            GraphicsBackendErrorKind::Unavailable => "graphics backend is unavailable",
            GraphicsBackendErrorKind::DeviceReset => "graphics backend device was reset",
            GraphicsBackendErrorKind::InvalidState => {
                "graphics backend operation is invalid in the current state"
            }
        })
    }
}

impl std::error::Error for GraphicsBackendError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicsBackendLoss {
    ResourcePressure,
    Unavailable,
    DeviceReset,
}

impl GraphicsBackendLoss {
    pub const fn webgl_reason(self) -> WebGlContextLossReason {
        match self {
            Self::ResourcePressure => WebGlContextLossReason::ResourcePressure,
            Self::Unavailable => WebGlContextLossReason::BackendUnavailable,
            Self::DeviceReset => WebGlContextLossReason::DeviceReset,
        }
    }
}

pub trait GraphicsBackend {
    type ContextHandle;
    type BufferHandle;
    type TextureHandle;

    fn create_context(
        &mut self,
        context: WebGlContextView,
    ) -> Result<Self::ContextHandle, GraphicsBackendError>;

    fn create_buffer(
        &mut self,
        context: &mut Self::ContextHandle,
        buffer: WebGlBufferView,
    ) -> Result<Self::BufferHandle, GraphicsBackendError>;

    fn create_texture(
        &mut self,
        context: &mut Self::ContextHandle,
        texture: WebGlTextureView,
    ) -> Result<Self::TextureHandle, GraphicsBackendError>;

    fn destroy_buffer(
        &mut self,
        context: &mut Self::ContextHandle,
        buffer: &mut Self::BufferHandle,
    ) -> Result<(), GraphicsBackendError>;

    fn destroy_texture(
        &mut self,
        context: &mut Self::ContextHandle,
        texture: &mut Self::TextureHandle,
    ) -> Result<(), GraphicsBackendError>;

    fn destroy_context(
        &mut self,
        context: &mut Self::ContextHandle,
    ) -> Result<(), GraphicsBackendError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicsContextBindingState {
    Active,
    Lost,
    CleanupPending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphicsContextBindingView {
    context: WebGlContextId,
    state: GraphicsContextBindingState,
}

impl GraphicsContextBindingView {
    pub const fn context(self) -> WebGlContextId {
        self.context
    }

    pub const fn state(self) -> GraphicsContextBindingState {
        self.state
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GraphicsAdapterError {
    InvalidLimits,
    ContextBindingLimitExceeded { bindings: usize, limit: usize },
    ResourceBindingLimitExceeded { bindings: usize, limit: usize },
    ContextAlreadyBound(WebGlContextId),
    BufferAlreadyBound(WebGlBufferId),
    TextureAlreadyBound(WebGlTextureId),
    ContextNotBound(WebGlContextId),
    BufferNotBound(WebGlBufferId),
    TextureNotBound(WebGlTextureId),
    ContextNotActive(WebGlContextId),
    ContextSurfaceDrift(WebGlContextId),
    BufferOwnerDrift(WebGlBufferId),
    TextureOwnerDrift(WebGlTextureId),
    Backend(GraphicsBackendErrorKind),
    BackendCleanupPending(GraphicsBackendErrorKind),
    WebGl(WebGlError),
    InconsistentState,
}

impl fmt::Display for GraphicsAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "graphics adapter error: {self:?}")
    }
}

impl std::error::Error for GraphicsAdapterError {}

impl From<WebGlError> for GraphicsAdapterError {
    fn from(value: WebGlError) -> Self {
        Self::WebGl(value)
    }
}

struct ContextBinding<Handle> {
    surface: rarog_canvas::CanvasSurfaceId,
    state: GraphicsContextBindingState,
    handle: Handle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResourceBindingState {
    Active,
    CleanupPending,
}

struct BufferBinding<Handle> {
    context: WebGlContextId,
    state: ResourceBindingState,
    handle: Handle,
}

struct TextureBinding<Handle> {
    context: WebGlContextId,
    state: ResourceBindingState,
    handle: Handle,
}

pub struct GraphicsAdapter<B: GraphicsBackend> {
    limits: GraphicsAdapterLimits,
    backend: B,
    contexts: BTreeMap<WebGlContextId, ContextBinding<B::ContextHandle>>,
    buffers: BTreeMap<WebGlBufferId, BufferBinding<B::BufferHandle>>,
    textures: BTreeMap<WebGlTextureId, TextureBinding<B::TextureHandle>>,
}

impl<B: GraphicsBackend> GraphicsAdapter<B> {
    pub fn try_new(
        limits: GraphicsAdapterLimits,
        backend: B,
    ) -> Result<Self, GraphicsAdapterError> {
        if !limits.is_valid() {
            return Err(GraphicsAdapterError::InvalidLimits);
        }
        Ok(Self {
            limits,
            backend,
            contexts: BTreeMap::new(),
            buffers: BTreeMap::new(),
            textures: BTreeMap::new(),
        })
    }

    pub const fn limits(&self) -> GraphicsAdapterLimits {
        self.limits
    }

    pub fn context_binding_count(&self) -> usize {
        self.contexts.len()
    }

    pub fn resource_binding_count(&self) -> usize {
        self.buffers.len() + self.textures.len()
    }

    pub fn context_binding(&self, context: WebGlContextId) -> Option<GraphicsContextBindingView> {
        self.contexts
            .get(&context)
            .map(|binding| GraphicsContextBindingView {
                context,
                state: binding.state,
            })
    }

    pub fn buffer_is_bound(&self, buffer: WebGlBufferId) -> bool {
        self.buffers.contains_key(&buffer)
    }

    pub fn texture_is_bound(&self, texture: WebGlTextureId) -> bool {
        self.textures.contains_key(&texture)
    }

    pub fn bind_context(
        &mut self,
        webgl: &WebGlRegistry,
        context: WebGlContextId,
    ) -> Result<(), GraphicsAdapterError> {
        if self.contexts.contains_key(&context) {
            return Err(GraphicsAdapterError::ContextAlreadyBound(context));
        }
        let view = webgl.context(context).ok_or(GraphicsAdapterError::WebGl(
            WebGlError::UnknownContext(context),
        ))?;
        if view.state() != WebGlContextState::Active {
            return Err(GraphicsAdapterError::ContextNotActive(context));
        }
        self.check_context_capacity()?;
        let handle = self
            .backend
            .create_context(view)
            .map_err(|error| GraphicsAdapterError::Backend(error.kind()))?;
        let previous = self.contexts.insert(
            context,
            ContextBinding {
                surface: view.surface(),
                state: GraphicsContextBindingState::Active,
                handle,
            },
        );
        debug_assert!(previous.is_none());
        Ok(())
    }

    pub fn bind_buffer(
        &mut self,
        webgl: &WebGlRegistry,
        context: WebGlContextId,
        buffer: WebGlBufferId,
    ) -> Result<(), GraphicsAdapterError> {
        if self.buffers.contains_key(&buffer) {
            return Err(GraphicsAdapterError::BufferAlreadyBound(buffer));
        }
        self.require_active_bound_context(webgl, context)?;
        let view =
            webgl
                .buffer(buffer)
                .ok_or(GraphicsAdapterError::WebGl(WebGlError::UnknownBuffer(
                    buffer,
                )))?;
        if view.context() != context {
            return Err(GraphicsAdapterError::BufferOwnerDrift(buffer));
        }
        self.check_resource_capacity()?;
        let handle = {
            let backend = &mut self.backend;
            let binding = self
                .contexts
                .get_mut(&context)
                .ok_or(GraphicsAdapterError::InconsistentState)?;
            backend
                .create_buffer(&mut binding.handle, view)
                .map_err(|error| GraphicsAdapterError::Backend(error.kind()))?
        };
        let previous = self.buffers.insert(
            buffer,
            BufferBinding {
                context,
                state: ResourceBindingState::Active,
                handle,
            },
        );
        debug_assert!(previous.is_none());
        Ok(())
    }

    pub fn bind_texture(
        &mut self,
        webgl: &WebGlRegistry,
        context: WebGlContextId,
        texture: WebGlTextureId,
    ) -> Result<(), GraphicsAdapterError> {
        if self.textures.contains_key(&texture) {
            return Err(GraphicsAdapterError::TextureAlreadyBound(texture));
        }
        self.require_active_bound_context(webgl, context)?;
        let view = webgl.texture(texture).ok_or(GraphicsAdapterError::WebGl(
            WebGlError::UnknownTexture(texture),
        ))?;
        if view.context() != context {
            return Err(GraphicsAdapterError::TextureOwnerDrift(texture));
        }
        self.check_resource_capacity()?;
        let handle = {
            let backend = &mut self.backend;
            let binding = self
                .contexts
                .get_mut(&context)
                .ok_or(GraphicsAdapterError::InconsistentState)?;
            backend
                .create_texture(&mut binding.handle, view)
                .map_err(|error| GraphicsAdapterError::Backend(error.kind()))?
        };
        let previous = self.textures.insert(
            texture,
            TextureBinding {
                context,
                state: ResourceBindingState::Active,
                handle,
            },
        );
        debug_assert!(previous.is_none());
        Ok(())
    }

    pub fn destroy_buffer(
        &mut self,
        webgl: &mut WebGlRegistry,
        context: WebGlContextId,
        buffer: WebGlBufferId,
    ) -> Result<(), GraphicsAdapterError> {
        self.require_exact_buffer_binding(webgl, context, buffer)?;
        webgl.destroy_buffer(context, buffer)?;
        match self.cleanup_buffer_binding(context, buffer) {
            Ok(()) => Ok(()),
            Err(kind) => Err(GraphicsAdapterError::BackendCleanupPending(kind)),
        }
    }

    pub fn destroy_texture(
        &mut self,
        webgl: &mut WebGlRegistry,
        context: WebGlContextId,
        texture: WebGlTextureId,
    ) -> Result<(), GraphicsAdapterError> {
        self.require_exact_texture_binding(webgl, context, texture)?;
        webgl.destroy_texture(context, texture)?;
        match self.cleanup_texture_binding(context, texture) {
            Ok(()) => Ok(()),
            Err(kind) => Err(GraphicsAdapterError::BackendCleanupPending(kind)),
        }
    }

    pub fn propagate_backend_loss(
        &mut self,
        webgl: &mut WebGlRegistry,
        context: WebGlContextId,
        loss: GraphicsBackendLoss,
    ) -> Result<bool, GraphicsAdapterError> {
        let view = webgl.context(context).ok_or(GraphicsAdapterError::WebGl(
            WebGlError::UnknownContext(context),
        ))?;
        let binding = self
            .contexts
            .get(&context)
            .ok_or(GraphicsAdapterError::ContextNotBound(context))?;
        if binding.surface != view.surface() {
            return Err(GraphicsAdapterError::ContextSurfaceDrift(context));
        }
        if binding.state == GraphicsContextBindingState::CleanupPending {
            return Err(GraphicsAdapterError::ContextNotActive(context));
        }

        let transitioned = webgl.lose_context(context, loss.webgl_reason())?;
        let binding = self
            .contexts
            .get_mut(&context)
            .ok_or(GraphicsAdapterError::InconsistentState)?;
        binding.state = GraphicsContextBindingState::Lost;
        self.mark_context_resources_cleanup_pending(context);
        self.retry_cleanup(context)?;
        Ok(transitioned)
    }

    pub fn destroy_context(
        &mut self,
        canvas: &mut CanvasRegistry,
        webgl: &mut WebGlRegistry,
        context: WebGlContextId,
    ) -> Result<(), GraphicsAdapterError> {
        let view = webgl.context(context).ok_or(GraphicsAdapterError::WebGl(
            WebGlError::UnknownContext(context),
        ))?;
        let binding = self
            .contexts
            .get(&context)
            .ok_or(GraphicsAdapterError::ContextNotBound(context))?;
        if binding.surface != view.surface() {
            return Err(GraphicsAdapterError::ContextSurfaceDrift(context));
        }
        if binding.state == GraphicsContextBindingState::CleanupPending {
            return Err(GraphicsAdapterError::ContextNotActive(context));
        }

        webgl.destroy_context(canvas, context)?;
        let binding = self
            .contexts
            .get_mut(&context)
            .ok_or(GraphicsAdapterError::InconsistentState)?;
        binding.state = GraphicsContextBindingState::CleanupPending;
        self.mark_context_resources_cleanup_pending(context);
        self.retry_cleanup(context)
    }

    pub fn retry_cleanup(&mut self, context: WebGlContextId) -> Result<(), GraphicsAdapterError> {
        if !self.contexts.contains_key(&context) {
            return Err(GraphicsAdapterError::ContextNotBound(context));
        }
        let buffer_ids: Vec<_> = self
            .buffers
            .iter()
            .filter_map(|(id, binding)| {
                (binding.context == context
                    && binding.state == ResourceBindingState::CleanupPending)
                    .then_some(*id)
            })
            .collect();
        let texture_ids: Vec<_> = self
            .textures
            .iter()
            .filter_map(|(id, binding)| {
                (binding.context == context
                    && binding.state == ResourceBindingState::CleanupPending)
                    .then_some(*id)
            })
            .collect();

        let mut first_failure = None;
        for buffer in buffer_ids {
            if let Err(kind) = self.cleanup_buffer_binding(context, buffer) {
                first_failure.get_or_insert(kind);
            }
        }
        for texture in texture_ids {
            if let Err(kind) = self.cleanup_texture_binding(context, texture) {
                first_failure.get_or_insert(kind);
            }
        }

        let state = self
            .contexts
            .get(&context)
            .ok_or(GraphicsAdapterError::InconsistentState)?
            .state;
        let resources_remain = self
            .buffers
            .values()
            .any(|binding| binding.context == context)
            || self
                .textures
                .values()
                .any(|binding| binding.context == context);
        if state == GraphicsContextBindingState::CleanupPending && !resources_remain {
            let result = {
                let backend = &mut self.backend;
                let binding = self
                    .contexts
                    .get_mut(&context)
                    .ok_or(GraphicsAdapterError::InconsistentState)?;
                backend.destroy_context(&mut binding.handle)
            };
            match result {
                Ok(()) => {
                    self.contexts.remove(&context);
                }
                Err(error) => {
                    first_failure.get_or_insert(error.kind());
                }
            }
        }

        match first_failure {
            Some(kind) => Err(GraphicsAdapterError::BackendCleanupPending(kind)),
            None => Ok(()),
        }
    }

    fn require_active_bound_context(
        &self,
        webgl: &WebGlRegistry,
        context: WebGlContextId,
    ) -> Result<WebGlContextView, GraphicsAdapterError> {
        let view = webgl.context(context).ok_or(GraphicsAdapterError::WebGl(
            WebGlError::UnknownContext(context),
        ))?;
        if view.state() != WebGlContextState::Active {
            return Err(GraphicsAdapterError::ContextNotActive(context));
        }
        let binding = self
            .contexts
            .get(&context)
            .ok_or(GraphicsAdapterError::ContextNotBound(context))?;
        if binding.state != GraphicsContextBindingState::Active {
            return Err(GraphicsAdapterError::ContextNotActive(context));
        }
        if binding.surface != view.surface() {
            return Err(GraphicsAdapterError::ContextSurfaceDrift(context));
        }
        Ok(view)
    }

    fn require_exact_buffer_binding(
        &self,
        webgl: &WebGlRegistry,
        context: WebGlContextId,
        buffer: WebGlBufferId,
    ) -> Result<(), GraphicsAdapterError> {
        self.require_active_bound_context(webgl, context)?;
        let view =
            webgl
                .buffer(buffer)
                .ok_or(GraphicsAdapterError::WebGl(WebGlError::UnknownBuffer(
                    buffer,
                )))?;
        if view.context() != context {
            return Err(GraphicsAdapterError::BufferOwnerDrift(buffer));
        }
        let binding = self
            .buffers
            .get(&buffer)
            .ok_or(GraphicsAdapterError::BufferNotBound(buffer))?;
        if binding.context != context || binding.state != ResourceBindingState::Active {
            return Err(GraphicsAdapterError::BufferOwnerDrift(buffer));
        }
        Ok(())
    }

    fn require_exact_texture_binding(
        &self,
        webgl: &WebGlRegistry,
        context: WebGlContextId,
        texture: WebGlTextureId,
    ) -> Result<(), GraphicsAdapterError> {
        self.require_active_bound_context(webgl, context)?;
        let view = webgl.texture(texture).ok_or(GraphicsAdapterError::WebGl(
            WebGlError::UnknownTexture(texture),
        ))?;
        if view.context() != context {
            return Err(GraphicsAdapterError::TextureOwnerDrift(texture));
        }
        let binding = self
            .textures
            .get(&texture)
            .ok_or(GraphicsAdapterError::TextureNotBound(texture))?;
        if binding.context != context || binding.state != ResourceBindingState::Active {
            return Err(GraphicsAdapterError::TextureOwnerDrift(texture));
        }
        Ok(())
    }

    fn check_context_capacity(&self) -> Result<(), GraphicsAdapterError> {
        let next = self.contexts.len().checked_add(1).ok_or(
            GraphicsAdapterError::ContextBindingLimitExceeded {
                bindings: usize::MAX,
                limit: self.limits.max_context_bindings,
            },
        )?;
        if next > self.limits.max_context_bindings {
            return Err(GraphicsAdapterError::ContextBindingLimitExceeded {
                bindings: next,
                limit: self.limits.max_context_bindings,
            });
        }
        Ok(())
    }

    fn check_resource_capacity(&self) -> Result<(), GraphicsAdapterError> {
        let current = self.buffers.len().checked_add(self.textures.len()).ok_or(
            GraphicsAdapterError::ResourceBindingLimitExceeded {
                bindings: usize::MAX,
                limit: self.limits.max_resource_bindings,
            },
        )?;
        let next =
            current
                .checked_add(1)
                .ok_or(GraphicsAdapterError::ResourceBindingLimitExceeded {
                    bindings: usize::MAX,
                    limit: self.limits.max_resource_bindings,
                })?;
        if next > self.limits.max_resource_bindings {
            return Err(GraphicsAdapterError::ResourceBindingLimitExceeded {
                bindings: next,
                limit: self.limits.max_resource_bindings,
            });
        }
        Ok(())
    }

    fn mark_context_resources_cleanup_pending(&mut self, context: WebGlContextId) {
        for binding in self.buffers.values_mut() {
            if binding.context == context {
                binding.state = ResourceBindingState::CleanupPending;
            }
        }
        for binding in self.textures.values_mut() {
            if binding.context == context {
                binding.state = ResourceBindingState::CleanupPending;
            }
        }
    }

    fn cleanup_buffer_binding(
        &mut self,
        context: WebGlContextId,
        buffer: WebGlBufferId,
    ) -> Result<(), GraphicsBackendErrorKind> {
        let result = {
            let backend = &mut self.backend;
            let contexts = &mut self.contexts;
            let buffers = &mut self.buffers;
            let context_binding = contexts
                .get_mut(&context)
                .expect("validated graphics context binding");
            let buffer_binding = buffers
                .get_mut(&buffer)
                .expect("validated graphics buffer binding");
            backend.destroy_buffer(&mut context_binding.handle, &mut buffer_binding.handle)
        };
        match result {
            Ok(()) => {
                self.buffers.remove(&buffer);
                Ok(())
            }
            Err(error) => {
                if let Some(binding) = self.buffers.get_mut(&buffer) {
                    binding.state = ResourceBindingState::CleanupPending;
                }
                Err(error.kind())
            }
        }
    }

    fn cleanup_texture_binding(
        &mut self,
        context: WebGlContextId,
        texture: WebGlTextureId,
    ) -> Result<(), GraphicsBackendErrorKind> {
        let result = {
            let backend = &mut self.backend;
            let contexts = &mut self.contexts;
            let textures = &mut self.textures;
            let context_binding = contexts
                .get_mut(&context)
                .expect("validated graphics context binding");
            let texture_binding = textures
                .get_mut(&texture)
                .expect("validated graphics texture binding");
            backend.destroy_texture(&mut context_binding.handle, &mut texture_binding.handle)
        };
        match result {
            Ok(()) => {
                self.textures.remove(&texture);
                Ok(())
            }
            Err(error) => {
                if let Some(binding) = self.textures.get_mut(&texture) {
                    binding.state = ResourceBindingState::CleanupPending;
                }
                Err(error.kind())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_canvas::{CanvasLimits, CanvasRegistry};
    use rarog_webgl::{WebGlLimits, WebGlRegistry};
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    #[derive(Default)]
    struct MockState {
        next: u64,
        contexts: BTreeMap<u64, WebGlContextId>,
        buffers: BTreeMap<u64, WebGlBufferId>,
        textures: BTreeMap<u64, WebGlTextureId>,
        fail_buffer_create: bool,
        fail_buffer_destroy: bool,
        fail_context_destroy: bool,
    }

    #[derive(Clone)]
    struct MockBackend {
        state: Rc<RefCell<MockState>>,
    }

    impl MockBackend {
        fn new() -> (Self, Rc<RefCell<MockState>>) {
            let state = Rc::new(RefCell::new(MockState::default()));
            (
                Self {
                    state: Rc::clone(&state),
                },
                state,
            )
        }

        fn allocate(&self) -> u64 {
            let mut state = self.state.borrow_mut();
            state.next += 1;
            state.next
        }
    }

    impl GraphicsBackend for MockBackend {
        type ContextHandle = u64;
        type BufferHandle = u64;
        type TextureHandle = u64;

        fn create_context(
            &mut self,
            context: WebGlContextView,
        ) -> Result<Self::ContextHandle, GraphicsBackendError> {
            let handle = self.allocate();
            self.state
                .borrow_mut()
                .contexts
                .insert(handle, context.id());
            Ok(handle)
        }

        fn create_buffer(
            &mut self,
            _context: &mut Self::ContextHandle,
            buffer: WebGlBufferView,
        ) -> Result<Self::BufferHandle, GraphicsBackendError> {
            if std::mem::take(&mut self.state.borrow_mut().fail_buffer_create) {
                return Err(GraphicsBackendError::new(
                    GraphicsBackendErrorKind::ResourcePressure,
                ));
            }
            let handle = self.allocate();
            self.state.borrow_mut().buffers.insert(handle, buffer.id());
            Ok(handle)
        }

        fn create_texture(
            &mut self,
            _context: &mut Self::ContextHandle,
            texture: WebGlTextureView,
        ) -> Result<Self::TextureHandle, GraphicsBackendError> {
            let handle = self.allocate();
            self.state
                .borrow_mut()
                .textures
                .insert(handle, texture.id());
            Ok(handle)
        }

        fn destroy_buffer(
            &mut self,
            _context: &mut Self::ContextHandle,
            buffer: &mut Self::BufferHandle,
        ) -> Result<(), GraphicsBackendError> {
            if std::mem::take(&mut self.state.borrow_mut().fail_buffer_destroy) {
                return Err(GraphicsBackendError::new(
                    GraphicsBackendErrorKind::Unavailable,
                ));
            }
            self.state.borrow_mut().buffers.remove(buffer);
            Ok(())
        }

        fn destroy_texture(
            &mut self,
            _context: &mut Self::ContextHandle,
            texture: &mut Self::TextureHandle,
        ) -> Result<(), GraphicsBackendError> {
            self.state.borrow_mut().textures.remove(texture);
            Ok(())
        }

        fn destroy_context(
            &mut self,
            context: &mut Self::ContextHandle,
        ) -> Result<(), GraphicsBackendError> {
            if std::mem::take(&mut self.state.borrow_mut().fail_context_destroy) {
                return Err(GraphicsBackendError::new(
                    GraphicsBackendErrorKind::Unavailable,
                ));
            }
            self.state.borrow_mut().contexts.remove(context);
            Ok(())
        }
    }

    #[derive(Default)]
    struct NoopBackend;

    impl GraphicsBackend for NoopBackend {
        type ContextHandle = ();
        type BufferHandle = ();
        type TextureHandle = ();

        fn create_context(
            &mut self,
            _context: WebGlContextView,
        ) -> Result<Self::ContextHandle, GraphicsBackendError> {
            Ok(())
        }

        fn create_buffer(
            &mut self,
            _context: &mut Self::ContextHandle,
            _buffer: WebGlBufferView,
        ) -> Result<Self::BufferHandle, GraphicsBackendError> {
            Ok(())
        }

        fn create_texture(
            &mut self,
            _context: &mut Self::ContextHandle,
            _texture: WebGlTextureView,
        ) -> Result<Self::TextureHandle, GraphicsBackendError> {
            Ok(())
        }

        fn destroy_buffer(
            &mut self,
            _context: &mut Self::ContextHandle,
            _buffer: &mut Self::BufferHandle,
        ) -> Result<(), GraphicsBackendError> {
            Ok(())
        }

        fn destroy_texture(
            &mut self,
            _context: &mut Self::ContextHandle,
            _texture: &mut Self::TextureHandle,
        ) -> Result<(), GraphicsBackendError> {
            Ok(())
        }

        fn destroy_context(
            &mut self,
            _context: &mut Self::ContextHandle,
        ) -> Result<(), GraphicsBackendError> {
            Ok(())
        }
    }

    fn canvas() -> CanvasRegistry {
        CanvasRegistry::try_new(CanvasLimits {
            max_surfaces: 8,
            max_contexts: 8,
            max_pixels_per_surface: 64,
            max_total_pixels: 256,
            max_state_stack_depth: 4,
        })
        .unwrap()
    }

    fn webgl() -> WebGlRegistry {
        WebGlRegistry::try_new(WebGlLimits {
            max_contexts: 4,
            max_resources_per_context: 4,
            max_resources: 8,
            max_buffer_bytes: 16,
            max_total_buffer_bytes: 32,
            max_texture_dimension: 8,
            max_texture_pixels: 16,
            max_total_texture_pixels: 32,
        })
        .unwrap()
    }

    fn adapter_limits() -> GraphicsAdapterLimits {
        GraphicsAdapterLimits {
            max_context_bindings: 4,
            max_resource_bindings: 8,
        }
    }

    #[test]
    fn exact_semantic_context_and_resource_authority_is_required() {
        let mut canvas = canvas();
        let surface = canvas.create_surface(1, 1).unwrap();
        let other_surface = canvas.create_surface(1, 1).unwrap();
        let mut webgl = webgl();
        let context = webgl.create_context(&mut canvas, surface).unwrap();
        let other = webgl.create_context(&mut canvas, other_surface).unwrap();
        let buffer = webgl.create_buffer(context, 4).unwrap();
        let (backend, state) = MockBackend::new();
        let mut adapter = GraphicsAdapter::try_new(adapter_limits(), backend).unwrap();

        adapter.bind_context(&webgl, context).unwrap();
        adapter.bind_context(&webgl, other).unwrap();
        assert_eq!(
            adapter.bind_buffer(&webgl, other, buffer),
            Err(GraphicsAdapterError::BufferOwnerDrift(buffer))
        );
        assert_eq!(state.borrow().buffers.len(), 0);
        adapter.bind_buffer(&webgl, context, buffer).unwrap();
        assert_eq!(state.borrow().buffers.len(), 1);
    }

    #[test]
    fn backend_create_failure_is_atomic_for_adapter_state() {
        let mut canvas = canvas();
        let surface = canvas.create_surface(1, 1).unwrap();
        let mut webgl = webgl();
        let context = webgl.create_context(&mut canvas, surface).unwrap();
        let buffer = webgl.create_buffer(context, 4).unwrap();
        let (backend, state) = MockBackend::new();
        state.borrow_mut().fail_buffer_create = true;
        let mut adapter = GraphicsAdapter::try_new(adapter_limits(), backend).unwrap();
        adapter.bind_context(&webgl, context).unwrap();

        assert_eq!(
            adapter.bind_buffer(&webgl, context, buffer),
            Err(GraphicsAdapterError::Backend(
                GraphicsBackendErrorKind::ResourcePressure
            ))
        );
        assert_eq!(adapter.resource_binding_count(), 0);
        assert!(webgl.buffer(buffer).is_some());
        assert_eq!(state.borrow().buffers.len(), 0);
    }

    #[test]
    fn failed_backend_destroy_stays_charged_until_retry() {
        let mut canvas = canvas();
        let surface = canvas.create_surface(1, 1).unwrap();
        let mut webgl = webgl();
        let context = webgl.create_context(&mut canvas, surface).unwrap();
        let buffer = webgl.create_buffer(context, 4).unwrap();
        let (backend, state) = MockBackend::new();
        let mut adapter = GraphicsAdapter::try_new(
            GraphicsAdapterLimits {
                max_context_bindings: 2,
                max_resource_bindings: 1,
            },
            backend,
        )
        .unwrap();
        adapter.bind_context(&webgl, context).unwrap();
        adapter.bind_buffer(&webgl, context, buffer).unwrap();
        state.borrow_mut().fail_buffer_destroy = true;

        assert_eq!(
            adapter.destroy_buffer(&mut webgl, context, buffer),
            Err(GraphicsAdapterError::BackendCleanupPending(
                GraphicsBackendErrorKind::Unavailable
            ))
        );
        assert!(webgl.buffer(buffer).is_none());
        assert_eq!(adapter.resource_binding_count(), 1);

        let replacement = webgl.create_buffer(context, 4).unwrap();
        assert!(matches!(
            adapter.bind_buffer(&webgl, context, replacement),
            Err(GraphicsAdapterError::ResourceBindingLimitExceeded { .. })
        ));
        adapter.retry_cleanup(context).unwrap();
        assert_eq!(adapter.resource_binding_count(), 0);
        adapter.bind_buffer(&webgl, context, replacement).unwrap();
    }

    #[test]
    fn backend_loss_maps_to_engine_owned_loss_and_retires_resources() {
        let mut canvas = canvas();
        let surface = canvas.create_surface(2, 2).unwrap();
        let mut webgl = webgl();
        let context = webgl.create_context(&mut canvas, surface).unwrap();
        let buffer = webgl.create_buffer(context, 8).unwrap();
        let texture = webgl.create_texture(context, 2, 2).unwrap();
        let (backend, _) = MockBackend::new();
        let mut adapter = GraphicsAdapter::try_new(adapter_limits(), backend).unwrap();
        adapter.bind_context(&webgl, context).unwrap();
        adapter.bind_buffer(&webgl, context, buffer).unwrap();
        adapter.bind_texture(&webgl, context, texture).unwrap();

        assert!(
            adapter
                .propagate_backend_loss(&mut webgl, context, GraphicsBackendLoss::DeviceReset)
                .unwrap()
        );
        assert_eq!(
            webgl.context(context).unwrap().state(),
            WebGlContextState::Lost(WebGlContextLossReason::DeviceReset)
        );
        assert!(webgl.buffer(buffer).is_none());
        assert!(webgl.texture(texture).is_none());
        assert_eq!(adapter.resource_binding_count(), 0);
        assert_eq!(
            adapter.context_binding(context).unwrap().state(),
            GraphicsContextBindingState::Lost
        );
    }

    #[test]
    fn failed_context_cleanup_does_not_manufacture_backend_capacity() {
        let mut canvas = canvas();
        let surface = canvas.create_surface(1, 1).unwrap();
        let mut webgl = webgl();
        let context = webgl.create_context(&mut canvas, surface).unwrap();
        let (backend, state) = MockBackend::new();
        let mut adapter = GraphicsAdapter::try_new(
            GraphicsAdapterLimits {
                max_context_bindings: 1,
                max_resource_bindings: 4,
            },
            backend,
        )
        .unwrap();
        adapter.bind_context(&webgl, context).unwrap();
        state.borrow_mut().fail_context_destroy = true;

        assert_eq!(
            adapter.destroy_context(&mut canvas, &mut webgl, context),
            Err(GraphicsAdapterError::BackendCleanupPending(
                GraphicsBackendErrorKind::Unavailable
            ))
        );
        assert!(webgl.context(context).is_none());
        assert_eq!(adapter.context_binding_count(), 1);

        let replacement = webgl.create_context(&mut canvas, surface).unwrap();
        assert!(matches!(
            adapter.bind_context(&webgl, replacement),
            Err(GraphicsAdapterError::ContextBindingLimitExceeded { .. })
        ));
        adapter.retry_cleanup(context).unwrap();
        assert_eq!(adapter.context_binding_count(), 0);
        adapter.bind_context(&webgl, replacement).unwrap();
    }

    #[test]
    fn cross_registry_identity_is_rejected_before_backend_mutation() {
        let mut first_canvas = canvas();
        let first_surface = first_canvas.create_surface(1, 1).unwrap();
        let mut first_webgl = webgl();
        let first_context = first_webgl
            .create_context(&mut first_canvas, first_surface)
            .unwrap();
        let mut second_canvas = canvas();
        let second_surface = second_canvas.create_surface(1, 1).unwrap();
        let mut second_webgl = webgl();
        let second_context = second_webgl
            .create_context(&mut second_canvas, second_surface)
            .unwrap();
        let (backend, state) = MockBackend::new();
        let mut adapter = GraphicsAdapter::try_new(adapter_limits(), backend).unwrap();
        adapter.bind_context(&first_webgl, first_context).unwrap();

        assert!(matches!(
            adapter.bind_context(&first_webgl, second_context),
            Err(GraphicsAdapterError::WebGl(WebGlError::UnknownContext(id))) if id == second_context
        ));
        assert_eq!(state.borrow().contexts.len(), 1);
    }

    fn exercise_replaceable_backend<B: GraphicsBackend>(backend: B) {
        let mut canvas = canvas();
        let surface = canvas.create_surface(1, 1).unwrap();
        let mut webgl = webgl();
        let context = webgl.create_context(&mut canvas, surface).unwrap();
        let buffer = webgl.create_buffer(context, 1).unwrap();
        let mut adapter = GraphicsAdapter::try_new(adapter_limits(), backend).unwrap();
        adapter.bind_context(&webgl, context).unwrap();
        adapter.bind_buffer(&webgl, context, buffer).unwrap();
        adapter.destroy_buffer(&mut webgl, context, buffer).unwrap();
        adapter
            .destroy_context(&mut canvas, &mut webgl, context)
            .unwrap();
        assert_eq!(adapter.context_binding_count(), 0);
        assert_eq!(adapter.resource_binding_count(), 0);
    }

    #[test]
    fn backend_implementation_is_replaceable_behind_one_adapter_contract() {
        let (backend, _) = MockBackend::new();
        exercise_replaceable_backend(backend);
        exercise_replaceable_backend(NoopBackend);
    }
}
