from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


# Bind the adapter to one exact WebGL registry scope.
webgl_path = Path("crates/rarog-webgl/src/lib.rs")
webgl = webgl_path.read_text()
webgl = replace_once(
    webgl,
    """    pub const fn limits(&self) -> WebGlLimits {\n        self.limits\n    }\n    pub fn context_count(&self) -> usize {\n""",
    """    pub const fn limits(&self) -> WebGlLimits {\n        self.limits\n    }\n\n    pub fn scope(&self) -> u64 {\n        self.ids.scope.get()\n    }\n\n    pub fn context_count(&self) -> usize {\n""",
    "WebGlRegistry::scope",
)
webgl_path.write_text(webgl)

adapter_path = Path("crates/rarog-graphics-adapter/src/lib.rs")
adapter = adapter_path.read_text()

adapter = replace_once(
    adapter,
    """pub enum GraphicsAdapterError {\n    InvalidLimits,\n    ContextBindingLimitExceeded { bindings: usize, limit: usize },\n""",
    """pub enum GraphicsAdapterError {\n    InvalidLimits,\n    RegistryScopeMismatch { expected: u64, actual: u64 },\n    ContextBindingLimitExceeded { bindings: usize, limit: usize },\n""",
    "adapter registry mismatch error",
)

adapter = replace_once(
    adapter,
    """pub struct GraphicsAdapter<B: GraphicsBackend> {\n    limits: GraphicsAdapterLimits,\n    backend: B,\n    contexts: BTreeMap<WebGlContextId, ContextBinding<B::ContextHandle>>,\n""",
    """pub struct GraphicsAdapter<B: GraphicsBackend> {\n    limits: GraphicsAdapterLimits,\n    backend: B,\n    webgl_scope: Option<u64>,\n    contexts: BTreeMap<WebGlContextId, ContextBinding<B::ContextHandle>>,\n""",
    "adapter registry scope field",
)

adapter = replace_once(
    adapter,
    """        Ok(Self {\n            limits,\n            backend,\n            contexts: BTreeMap::new(),\n""",
    """        Ok(Self {\n            limits,\n            backend,\n            webgl_scope: None,\n            contexts: BTreeMap::new(),\n""",
    "adapter registry scope init",
)

adapter = replace_once(
    adapter,
    """    pub fn bind_context(\n        &mut self,\n        webgl: &WebGlRegistry,\n        context: WebGlContextId,\n    ) -> Result<(), GraphicsAdapterError> {\n        if self.contexts.contains_key(&context) {\n            return Err(GraphicsAdapterError::ContextAlreadyBound(context));\n        }\n        let view = webgl.context(context).ok_or(GraphicsAdapterError::WebGl(\n            WebGlError::UnknownContext(context),\n        ))?;\n        if view.state() != WebGlContextState::Active {\n            return Err(GraphicsAdapterError::ContextNotActive(context));\n        }\n        self.check_context_capacity()?;\n""",
    """    pub fn bind_context(\n        &mut self,\n        webgl: &WebGlRegistry,\n        context: WebGlContextId,\n    ) -> Result<(), GraphicsAdapterError> {\n        self.ensure_registry_scope(webgl)?;\n        if self.contexts.contains_key(&context) {\n            return Err(GraphicsAdapterError::ContextAlreadyBound(context));\n        }\n        let view = webgl.context(context).ok_or(GraphicsAdapterError::WebGl(\n            WebGlError::UnknownContext(context),\n        ))?;\n        if view.state() != WebGlContextState::Active {\n            return Err(GraphicsAdapterError::ContextNotActive(context));\n        }\n        self.bind_registry_scope(webgl)?;\n        self.check_context_capacity()?;\n""",
    "bind context scope",
)

adapter = replace_once(
    adapter,
    """    pub fn destroy_buffer(\n        &mut self,\n        webgl: &mut WebGlRegistry,\n        context: WebGlContextId,\n        buffer: WebGlBufferId,\n    ) -> Result<(), GraphicsAdapterError> {\n        self.require_exact_buffer_binding(webgl, context, buffer)?;\n        webgl.destroy_buffer(context, buffer)?;\n        match self.cleanup_buffer_binding(context, buffer) {\n            Ok(()) => Ok(()),\n            Err(kind) => Err(GraphicsAdapterError::BackendCleanupPending(kind)),\n        }\n    }\n""",
    """    pub fn destroy_buffer(\n        &mut self,\n        webgl: &mut WebGlRegistry,\n        context: WebGlContextId,\n        buffer: WebGlBufferId,\n    ) -> Result<(), GraphicsAdapterError> {\n        self.require_exact_buffer_binding(webgl, context, buffer)?;\n        webgl.destroy_buffer(context, buffer)?;\n        self.cleanup_buffer_binding(context, buffer)\n    }\n""",
    "destroy buffer cleanup result",
)

adapter = replace_once(
    adapter,
    """    pub fn destroy_texture(\n        &mut self,\n        webgl: &mut WebGlRegistry,\n        context: WebGlContextId,\n        texture: WebGlTextureId,\n    ) -> Result<(), GraphicsAdapterError> {\n        self.require_exact_texture_binding(webgl, context, texture)?;\n        webgl.destroy_texture(context, texture)?;\n        match self.cleanup_texture_binding(context, texture) {\n            Ok(()) => Ok(()),\n            Err(kind) => Err(GraphicsAdapterError::BackendCleanupPending(kind)),\n        }\n    }\n""",
    """    pub fn destroy_texture(\n        &mut self,\n        webgl: &mut WebGlRegistry,\n        context: WebGlContextId,\n        texture: WebGlTextureId,\n    ) -> Result<(), GraphicsAdapterError> {\n        self.require_exact_texture_binding(webgl, context, texture)?;\n        webgl.destroy_texture(context, texture)?;\n        self.cleanup_texture_binding(context, texture)\n    }\n""",
    "destroy texture cleanup result",
)

adapter = replace_once(
    adapter,
    """    pub fn propagate_backend_loss(\n        &mut self,\n        webgl: &mut WebGlRegistry,\n        context: WebGlContextId,\n        loss: GraphicsBackendLoss,\n    ) -> Result<bool, GraphicsAdapterError> {\n        let view = webgl.context(context).ok_or(GraphicsAdapterError::WebGl(\n""",
    """    pub fn propagate_backend_loss(\n        &mut self,\n        webgl: &mut WebGlRegistry,\n        context: WebGlContextId,\n        loss: GraphicsBackendLoss,\n    ) -> Result<bool, GraphicsAdapterError> {\n        self.ensure_registry_scope(webgl)?;\n        let view = webgl.context(context).ok_or(GraphicsAdapterError::WebGl(\n""",
    "backend loss scope",
)

adapter = replace_once(
    adapter,
    """    pub fn destroy_context(\n        &mut self,\n        canvas: &mut CanvasRegistry,\n        webgl: &mut WebGlRegistry,\n        context: WebGlContextId,\n    ) -> Result<(), GraphicsAdapterError> {\n        let view = webgl.context(context).ok_or(GraphicsAdapterError::WebGl(\n""",
    """    pub fn destroy_context(\n        &mut self,\n        canvas: &mut CanvasRegistry,\n        webgl: &mut WebGlRegistry,\n        context: WebGlContextId,\n    ) -> Result<(), GraphicsAdapterError> {\n        self.ensure_registry_scope(webgl)?;\n        let view = webgl.context(context).ok_or(GraphicsAdapterError::WebGl(\n""",
    "destroy context scope",
)

retry_old = """        let mut first_failure = None;\n        for buffer in buffer_ids {\n            if let Err(kind) = self.cleanup_buffer_binding(context, buffer) {\n                first_failure.get_or_insert(kind);\n            }\n        }\n        for texture in texture_ids {\n            if let Err(kind) = self.cleanup_texture_binding(context, texture) {\n                first_failure.get_or_insert(kind);\n            }\n        }\n"""
retry_new = """        let mut first_failure = None;\n        for buffer in buffer_ids {\n            match self.cleanup_buffer_binding(context, buffer) {\n                Ok(()) => {}\n                Err(GraphicsAdapterError::BackendCleanupPending(kind)) => {\n                    first_failure.get_or_insert(kind);\n                }\n                Err(error) => return Err(error),\n            }\n        }\n        for texture in texture_ids {\n            match self.cleanup_texture_binding(context, texture) {\n                Ok(()) => {}\n                Err(GraphicsAdapterError::BackendCleanupPending(kind)) => {\n                    first_failure.get_or_insert(kind);\n                }\n                Err(error) => return Err(error),\n            }\n        }\n"""
adapter = replace_once(adapter, retry_old, retry_new, "retry cleanup helper errors")

cleanup_old = """    fn cleanup_buffer_binding(\n        &mut self,\n        context: WebGlContextId,\n        buffer: WebGlBufferId,\n    ) -> Result<(), GraphicsBackendErrorKind> {\n        let result = {\n            let backend = &mut self.backend;\n            let contexts = &mut self.contexts;\n            let buffers = &mut self.buffers;\n            let context_binding = contexts\n                .get_mut(&context)\n                .expect(\"validated graphics context binding\");\n            let buffer_binding = buffers\n                .get_mut(&buffer)\n                .expect(\"validated graphics buffer binding\");\n            backend.destroy_buffer(&mut context_binding.handle, &mut buffer_binding.handle)\n        };\n        match result {\n            Ok(()) => {\n                self.buffers.remove(&buffer);\n                Ok(())\n            }\n            Err(error) => {\n                if let Some(binding) = self.buffers.get_mut(&buffer) {\n                    binding.state = ResourceBindingState::CleanupPending;\n                }\n                Err(error.kind())\n            }\n        }\n    }\n\n    fn cleanup_texture_binding(\n        &mut self,\n        context: WebGlContextId,\n        texture: WebGlTextureId,\n    ) -> Result<(), GraphicsBackendErrorKind> {\n        let result = {\n            let backend = &mut self.backend;\n            let contexts = &mut self.contexts;\n            let textures = &mut self.textures;\n            let context_binding = contexts\n                .get_mut(&context)\n                .expect(\"validated graphics context binding\");\n            let texture_binding = textures\n                .get_mut(&texture)\n                .expect(\"validated graphics texture binding\");\n            backend.destroy_texture(&mut context_binding.handle, &mut texture_binding.handle)\n        };\n        match result {\n            Ok(()) => {\n                self.textures.remove(&texture);\n                Ok(())\n            }\n            Err(error) => {\n                if let Some(binding) = self.textures.get_mut(&texture) {\n                    binding.state = ResourceBindingState::CleanupPending;\n                }\n                Err(error.kind())\n            }\n        }\n    }\n"""
cleanup_new = """    fn cleanup_buffer_binding(\n        &mut self,\n        context: WebGlContextId,\n        buffer: WebGlBufferId,\n    ) -> Result<(), GraphicsAdapterError> {\n        let result = {\n            let backend = &mut self.backend;\n            let contexts = &mut self.contexts;\n            let buffers = &mut self.buffers;\n            let context_binding = contexts\n                .get_mut(&context)\n                .ok_or(GraphicsAdapterError::InconsistentState)?;\n            let buffer_binding = buffers\n                .get_mut(&buffer)\n                .ok_or(GraphicsAdapterError::InconsistentState)?;\n            backend.destroy_buffer(&mut context_binding.handle, &mut buffer_binding.handle)\n        };\n        match result {\n            Ok(()) => {\n                self.buffers.remove(&buffer);\n                Ok(())\n            }\n            Err(error) => {\n                if let Some(binding) = self.buffers.get_mut(&buffer) {\n                    binding.state = ResourceBindingState::CleanupPending;\n                }\n                Err(GraphicsAdapterError::BackendCleanupPending(error.kind()))\n            }\n        }\n    }\n\n    fn cleanup_texture_binding(\n        &mut self,\n        context: WebGlContextId,\n        texture: WebGlTextureId,\n    ) -> Result<(), GraphicsAdapterError> {\n        let result = {\n            let backend = &mut self.backend;\n            let contexts = &mut self.contexts;\n            let textures = &mut self.textures;\n            let context_binding = contexts\n                .get_mut(&context)\n                .ok_or(GraphicsAdapterError::InconsistentState)?;\n            let texture_binding = textures\n                .get_mut(&texture)\n                .ok_or(GraphicsAdapterError::InconsistentState)?;\n            backend.destroy_texture(&mut context_binding.handle, &mut texture_binding.handle)\n        };\n        match result {\n            Ok(()) => {\n                self.textures.remove(&texture);\n                Ok(())\n            }\n            Err(error) => {\n                if let Some(binding) = self.textures.get_mut(&texture) {\n                    binding.state = ResourceBindingState::CleanupPending;\n                }\n                Err(GraphicsAdapterError::BackendCleanupPending(error.kind()))\n            }\n        }\n    }\n"""
adapter = replace_once(adapter, cleanup_old, cleanup_new, "fail-closed cleanup helpers")

# Make all context-sensitive lookups reject a foreign registry before inspecting IDs.
adapter = replace_once(
    adapter,
    """    fn require_active_bound_context(\n        &self,\n        webgl: &WebGlRegistry,\n        context: WebGlContextId,\n    ) -> Result<WebGlContextView, GraphicsAdapterError> {\n        let view = webgl.context(context).ok_or(GraphicsAdapterError::WebGl(\n""",
    """    fn require_active_bound_context(\n        &self,\n        webgl: &WebGlRegistry,\n        context: WebGlContextId,\n    ) -> Result<WebGlContextView, GraphicsAdapterError> {\n        self.ensure_registry_scope(webgl)?;\n        let view = webgl.context(context).ok_or(GraphicsAdapterError::WebGl(\n""",
    "active context registry scope",
)

# Insert reconciliation and scope helpers immediately before require_active_bound_context.
marker = """    fn require_active_bound_context(\n        &self,\n        webgl: &WebGlRegistry,\n        context: WebGlContextId,\n    ) -> Result<WebGlContextView, GraphicsAdapterError> {\n"""
reconcile = """    pub fn reconcile(&mut self, webgl: &WebGlRegistry) -> Result<(), GraphicsAdapterError> {\n        self.ensure_registry_scope(webgl)?;\n        let contexts: Vec<_> = self.contexts.keys().copied().collect();\n        let mut first_failure = None;\n\n        for context in contexts {\n            match webgl.context(context) {\n                None => {\n                    let binding = self\n                        .contexts\n                        .get_mut(&context)\n                        .ok_or(GraphicsAdapterError::InconsistentState)?;\n                    binding.state = GraphicsContextBindingState::CleanupPending;\n                    self.mark_context_resources_cleanup_pending(context);\n                }\n                Some(view) => {\n                    let binding = self\n                        .contexts\n                        .get(&context)\n                        .ok_or(GraphicsAdapterError::InconsistentState)?;\n                    if binding.surface != view.surface() {\n                        return Err(GraphicsAdapterError::ContextSurfaceDrift(context));\n                    }\n                    match view.state() {\n                        WebGlContextState::Active => {\n                            if binding.state != GraphicsContextBindingState::Active {\n                                return Err(GraphicsAdapterError::ContextNotActive(context));\n                            }\n                            for (id, binding) in self.buffers.iter_mut() {\n                                if binding.context == context\n                                    && webgl.buffer(*id).is_none_or(|item| item.context() != context)\n                                {\n                                    binding.state = ResourceBindingState::CleanupPending;\n                                }\n                            }\n                            for (id, binding) in self.textures.iter_mut() {\n                                if binding.context == context\n                                    && webgl.texture(*id).is_none_or(|item| item.context() != context)\n                                {\n                                    binding.state = ResourceBindingState::CleanupPending;\n                                }\n                            }\n                        }\n                        WebGlContextState::Lost(_) => {\n                            let binding = self\n                                .contexts\n                                .get_mut(&context)\n                                .ok_or(GraphicsAdapterError::InconsistentState)?;\n                            binding.state = GraphicsContextBindingState::Lost;\n                            self.mark_context_resources_cleanup_pending(context);\n                        }\n                    }\n                }\n            }\n\n            match self.retry_cleanup(context) {\n                Ok(()) => {}\n                Err(GraphicsAdapterError::BackendCleanupPending(kind)) => {\n                    first_failure.get_or_insert(kind);\n                }\n                Err(error) => return Err(error),\n            }\n        }\n\n        match first_failure {\n            Some(kind) => Err(GraphicsAdapterError::BackendCleanupPending(kind)),\n            None => Ok(()),\n        }\n    }\n\n    fn ensure_registry_scope(&self, webgl: &WebGlRegistry) -> Result<(), GraphicsAdapterError> {\n        let actual = webgl.scope();\n        match self.webgl_scope {\n            Some(expected) if expected != actual => {\n                Err(GraphicsAdapterError::RegistryScopeMismatch { expected, actual })\n            }\n            _ => Ok(()),\n        }\n    }\n\n    fn bind_registry_scope(&mut self, webgl: &WebGlRegistry) -> Result<(), GraphicsAdapterError> {\n        self.ensure_registry_scope(webgl)?;\n        if self.webgl_scope.is_none() {\n            self.webgl_scope = Some(webgl.scope());\n        }\n        Ok(())\n    }\n\n""" + marker
adapter = replace_once(adapter, marker, reconcile, "reconcile and registry helpers")

# Use an MSRV-compatible Option check rather than newer convenience methods.
adapter = adapter.replace(
    "webgl.buffer(*id).is_none_or(|item| item.context() != context)",
    "webgl.buffer(*id).map_or(true, |item| item.context() != context)",
).replace(
    "webgl.texture(*id).is_none_or(|item| item.context() != context)",
    "webgl.texture(*id).map_or(true, |item| item.context() != context)",
)

# Add regressions before the generic replaceability helper.
test_marker = """    fn exercise_replaceable_backend<B: GraphicsBackend>(backend: B) {\n"""
tests = r'''    #[test]
    fn foreign_registry_scope_is_rejected_before_backend_mutation() {
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

        assert_eq!(
            adapter.bind_context(&second_webgl, second_context),
            Err(GraphicsAdapterError::RegistryScopeMismatch {
                expected: first_webgl.scope(),
                actual: second_webgl.scope(),
            })
        );
        assert_eq!(state.borrow().contexts.len(), 1);
    }

    #[test]
    fn reconcile_cleans_resource_destroyed_outside_adapter() {
        let mut canvas = canvas();
        let surface = canvas.create_surface(1, 1).unwrap();
        let mut webgl = webgl();
        let context = webgl.create_context(&mut canvas, surface).unwrap();
        let buffer = webgl.create_buffer(context, 4).unwrap();
        let (backend, state) = MockBackend::new();
        let mut adapter = GraphicsAdapter::try_new(adapter_limits(), backend).unwrap();
        adapter.bind_context(&webgl, context).unwrap();
        adapter.bind_buffer(&webgl, context, buffer).unwrap();

        webgl.destroy_buffer(context, buffer).unwrap();
        assert_eq!(state.borrow().buffers.len(), 1);
        adapter.reconcile(&webgl).unwrap();
        assert_eq!(adapter.resource_binding_count(), 0);
        assert_eq!(state.borrow().buffers.len(), 0);
    }

    #[test]
    fn reconcile_cleans_context_destroyed_outside_adapter() {
        let mut canvas = canvas();
        let surface = canvas.create_surface(1, 1).unwrap();
        let mut webgl = webgl();
        let context = webgl.create_context(&mut canvas, surface).unwrap();
        let buffer = webgl.create_buffer(context, 4).unwrap();
        let (backend, state) = MockBackend::new();
        let mut adapter = GraphicsAdapter::try_new(adapter_limits(), backend).unwrap();
        adapter.bind_context(&webgl, context).unwrap();
        adapter.bind_buffer(&webgl, context, buffer).unwrap();

        webgl.destroy_context(&mut canvas, context).unwrap();
        adapter.reconcile(&webgl).unwrap();
        assert_eq!(adapter.context_binding_count(), 0);
        assert_eq!(adapter.resource_binding_count(), 0);
        assert_eq!(state.borrow().contexts.len(), 0);
        assert_eq!(state.borrow().buffers.len(), 0);
    }

    #[test]
    fn reconcile_tracks_direct_semantic_loss_without_destroying_context_binding() {
        let mut canvas = canvas();
        let surface = canvas.create_surface(1, 1).unwrap();
        let mut webgl = webgl();
        let context = webgl.create_context(&mut canvas, surface).unwrap();
        let texture = webgl.create_texture(context, 1, 1).unwrap();
        let (backend, state) = MockBackend::new();
        let mut adapter = GraphicsAdapter::try_new(adapter_limits(), backend).unwrap();
        adapter.bind_context(&webgl, context).unwrap();
        adapter.bind_texture(&webgl, context, texture).unwrap();

        webgl
            .lose_context(context, WebGlContextLossReason::BackendUnavailable)
            .unwrap();
        adapter.reconcile(&webgl).unwrap();
        assert_eq!(adapter.resource_binding_count(), 0);
        assert_eq!(state.borrow().textures.len(), 0);
        assert_eq!(
            adapter.context_binding(context).unwrap().state(),
            GraphicsContextBindingState::Lost
        );
        assert_eq!(state.borrow().contexts.len(), 1);
    }

'''
adapter = replace_once(adapter, test_marker, tests + test_marker, "hardening regressions")
adapter_path.write_text(adapter)

architecture_path = Path("docs/ARCHITECTURE.md")
architecture = architecture_path.read_text()
arch_marker = """- See [ADR-0133](adr/0133-webgl-ownership-loss.md).\n\n### Element names, namespaces and atoms\n"""
arch_new = """- See [ADR-0133](adr/0133-webgl-ownership-loss.md).\n\n### R5 replaceable graphics backend boundary\n\n- `rarog-graphics-adapter` is a portable integration layer depending only on `rarog-canvas` and `rarog-webgl`; it does not select `wgpu`, D3D/DXGI, platform APIs or native handles.\n- One adapter instance binds to one exact `WebGlRegistry` scope. Exact live WebGL context/resource identities remain the only semantic authority; a foreign registry is rejected before backend mutation or cleanup.\n- Concrete backend context/buffer/texture handle types are private generic adapter state. They are never returned as Web/DOM/WebGL authority and never substitute for `WebGlContextId`, `WebGlBufferId` or `WebGlTextureId`.\n- Adapter context/resource bindings have independent explicit bounds. Backend cleanup failures retain private handles as cleanup-pending bindings, so failed destruction remains charged and cannot manufacture backend capacity.\n- Semantic resource/context destruction retires WebGL authority first. Backend destruction then either succeeds or remains bounded for explicit retry; stale backend state cannot resurrect retired semantic IDs.\n- `GraphicsAdapter::reconcile` handles semantic resource destruction, context loss or context destruction performed outside the adapter, but only after exact registry-scope validation. Lost contexts reuse the fixed engine-owned `WebGlContextLossReason` model rather than exposing native/backend errors upward.\n- Mock and no-op implementations prove the backend contract is replaceable on portable CI. Real GPU objects remain private to a future concrete backend and this boundary does not claim WebGL command or hardware-backend completeness.\n- See [ADR-0134](adr/0134-replaceable-graphics-backend-boundary.md).\n\n### Element names, namespaces and atoms\n"""
architecture = replace_once(architecture, arch_marker, arch_new, "architecture graphics adapter section")
architecture_path.write_text(architecture)
