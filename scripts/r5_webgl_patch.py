from pathlib import Path
import subprocess

BASE = "e9be9ba819896e9920322c57c48d2c6fc451b801"
canvas_path = Path("crates/rarog-canvas/src/lib.rs")
canvas_path.write_bytes(
    subprocess.check_output(["git", "show", f"{BASE}:crates/rarog-canvas/src/lib.rs"])
)
Path("crates/rarog-canvas/src/implementation.rs").unlink(missing_ok=True)
s = canvas_path.read_text()


def replace_once(old: str, new: str) -> None:
    global s
    if old not in s:
        raise RuntimeError(f"missing Canvas patch marker: {old[:100]!r}")
    s = s.replace(old, new, 1)


marker = "#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub struct CanvasContentRevision"
insert = '''#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

'''
replace_once(marker, insert + marker)
replace_once(
    "    output: Arc<[Color]>,\n    context: Option<CanvasContextId>,\n}",
    "    output: Arc<[Color]>,\n    context: Option<CanvasContextId>,\n    external_context: Option<CanvasExternalContextLease>,\n}",
)
replace_once(
    '''    pub const fn context(&self) -> Option<CanvasContextId> {
        self.context
    }
}''',
    '''    pub const fn context(&self) -> Option<CanvasContextId> {
        self.context
    }

    pub const fn external_context(&self) -> Option<CanvasExternalContextLease> {
        self.external_context
    }
}''',
)
replace_once(
    "    ContextIdentitySpaceExhausted,\n    UnknownSurface(CanvasSurfaceId),",
    "    ContextIdentitySpaceExhausted,\n    ExternalContextIdentitySpaceExhausted,\n    UnknownSurface(CanvasSurfaceId),",
)
replace_once(
    "    UnknownContext(CanvasContextId),\n    SurfaceAlreadyHasContext(CanvasSurfaceId),",
    "    UnknownContext(CanvasContextId),\n    UnknownExternalContextLease(CanvasExternalContextLease),\n    SurfaceAlreadyHasContext(CanvasSurfaceId),",
)
replace_once(
    '''            Self::ContextIdentitySpaceExhausted => {
                formatter.write_str("Canvas context identity space is exhausted")
            }
            Self::UnknownSurface(id) => write!(''',
    '''            Self::ContextIdentitySpaceExhausted => {
                formatter.write_str("Canvas context identity space is exhausted")
            }
            Self::ExternalContextIdentitySpaceExhausted => {
                formatter.write_str("Canvas external-context lease identity space is exhausted")
            }
            Self::UnknownSurface(id) => write!(''',
)
replace_once(
    '''            Self::UnknownContext(id) => write!(
                formatter,
                "unknown Canvas context {}:{}",
                id.scope(),
                id.serial()
            ),
            Self::SurfaceAlreadyHasContext(id) => write!(''',
    '''            Self::UnknownContext(id) => write!(
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
            Self::SurfaceAlreadyHasContext(id) => write!(''',
)
replace_once("already has a 2D context", "already has a rendering context")
replace_once(
    "cannot retire while its 2D context is live",
    "cannot retire while its rendering context is live",
)
replace_once(
    '''    fn allocate_context(&mut self) -> Result<CanvasContextId, CanvasError> {
        let serial =
            NonZeroU64::new(self.next_serial).ok_or(CanvasError::ContextIdentitySpaceExhausted)?;
        self.next_serial = self.next_serial.checked_add(1).unwrap_or(0);
        Ok(CanvasContextId {
            scope: self.scope,
            serial,
        })
    }
}''',
    '''    fn allocate_context(&mut self) -> Result<CanvasContextId, CanvasError> {
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
}''',
)
replace_once(
    "    context_ids: IdentityAllocator,\n    surfaces: BTreeMap<CanvasSurfaceId, CanvasSurface>,",
    "    context_ids: IdentityAllocator,\n    external_context_ids: IdentityAllocator,\n    surfaces: BTreeMap<CanvasSurfaceId, CanvasSurface>,",
)
replace_once(
    "            context_ids: IdentityAllocator::new(scope),\n            surfaces: BTreeMap::new(),",
    "            context_ids: IdentityAllocator::new(scope),\n            external_context_ids: IdentityAllocator::new(scope),\n            surfaces: BTreeMap::new(),",
)
replace_once(
    "                output,\n                context: None,\n            },",
    "                output,\n                context: None,\n                external_context: None,\n            },",
)
replace_once(
    "if surface.context.is_some() {\n            return Err(CanvasError::SurfaceHasLiveContext(id));",
    "if surface.context.is_some() || surface.external_context.is_some() {\n            return Err(CanvasError::SurfaceHasLiveContext(id));",
)
replace_once(
    "if owner.context.is_some() {\n            return Err(CanvasError::SurfaceAlreadyHasContext(surface));",
    "if owner.context.is_some() || owner.external_context.is_some() {\n            return Err(CanvasError::SurfaceAlreadyHasContext(surface));",
)
retire = '''    pub fn retire_context(&mut self, id: CanvasContextId) -> Result<(), CanvasError> {
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
'''
lease_api = retire + '''
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
        let lease = self.external_context_ids.allocate_external_context(surface)?;
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
'''
replace_once(retire, lease_api)
last = s.rfind("\n}")
if last == -1:
    raise RuntimeError("missing Canvas test module close")
tests = r'''

    #[test]
    fn external_context_is_exclusive_with_2d_and_surface_retirement() {
        let mut registry = CanvasRegistry::try_new(tiny_limits()).unwrap();
        let surface = registry.create_surface(2, 2).unwrap();
        let lease = registry.acquire_external_context(surface).unwrap();
        assert_eq!(registry.surface(surface).unwrap().external_context(), Some(lease));
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
        assert_eq!(registry.surface(surface).unwrap().external_context(), Some(second));
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
'''
s = s[:last] + tests + s[last:]
canvas_path.write_text(s)

webgl_path = Path("crates/rarog-webgl/src/lib.rs")
w = webgl_path.read_text()


def wreplace(old: str, new: str) -> None:
    global w
    if old not in w:
        raise RuntimeError(f"missing WebGL patch marker: {old[:100]!r}")
    w = w.replace(old, new, 1)


wreplace(
    '''        let id = self.ids.buffer()?;
        let old = self.buffers.insert(id, BufferRecord { context, bytes });
        debug_assert!(old.is_none());
        let owner = self.contexts.get_mut(&context).ok_or(WebGlError::InconsistentState)?;
        owner.resource_count += 1;
        owner.buffer_bytes += bytes;
        self.total_buffer_bytes = next_total;
        Ok(id)''',
    '''        let owner = self.contexts.get(&context).ok_or(WebGlError::InconsistentState)?;
        let next_owner_resources = owner.resource_count.checked_add(1).ok_or(WebGlError::InconsistentState)?;
        let next_owner_bytes = owner.buffer_bytes.checked_add(bytes).ok_or(WebGlError::InconsistentState)?;
        let id = self.ids.buffer()?;
        let old = self.buffers.insert(id, BufferRecord { context, bytes });
        debug_assert!(old.is_none());
        let owner = self.contexts.get_mut(&context).ok_or(WebGlError::InconsistentState)?;
        owner.resource_count = next_owner_resources;
        owner.buffer_bytes = next_owner_bytes;
        self.total_buffer_bytes = next_total;
        Ok(id)''',
)
wreplace(
    '''        let id = self.ids.texture()?;
        let old = self.textures.insert(id, TextureRecord { context, width, height, pixels });
        debug_assert!(old.is_none());
        let owner = self.contexts.get_mut(&context).ok_or(WebGlError::InconsistentState)?;
        owner.resource_count += 1;
        owner.texture_pixels += pixels;
        self.total_texture_pixels = next_total;
        Ok(id)''',
    '''        let owner = self.contexts.get(&context).ok_or(WebGlError::InconsistentState)?;
        let next_owner_resources = owner.resource_count.checked_add(1).ok_or(WebGlError::InconsistentState)?;
        let next_owner_pixels = owner.texture_pixels.checked_add(pixels).ok_or(WebGlError::InconsistentState)?;
        let id = self.ids.texture()?;
        let old = self.textures.insert(id, TextureRecord { context, width, height, pixels });
        debug_assert!(old.is_none());
        let owner = self.contexts.get_mut(&context).ok_or(WebGlError::InconsistentState)?;
        owner.resource_count = next_owner_resources;
        owner.texture_pixels = next_owner_pixels;
        self.total_texture_pixels = next_total;
        Ok(id)''',
)
wreplace(
    '''        let total_next = self.resource_count().checked_add(1).ok_or(WebGlError::ResourceLimitExceeded {
            resources: usize::MAX, limit: self.limits.max_resources,
        })?;''',
    '''        let current_total = self.buffers.len().checked_add(self.textures.len()).ok_or(
            WebGlError::ResourceLimitExceeded {
                resources: usize::MAX,
                limit: self.limits.max_resources,
            },
        )?;
        let total_next = current_total.checked_add(1).ok_or(WebGlError::ResourceLimitExceeded {
            resources: usize::MAX, limit: self.limits.max_resources,
        })?;''',
)
wreplace(
    '''        let lease = self.contexts.get(&context).ok_or(WebGlError::UnknownContext(context))?.lease;
        self.retire_resources(context)?;
        canvas.release_external_context(lease)?;
        self.contexts.remove(&context);''',
    '''        let record = self.contexts.get(&context).ok_or(WebGlError::UnknownContext(context))?;
        let lease = record.lease;
        if canvas.surface(record.surface).and_then(|surface| surface.external_context()) != Some(lease) {
            return Err(WebGlError::InconsistentState);
        }
        self.retire_resources(context)?;
        canvas.release_external_context(lease)?;
        self.contexts.remove(&context);''',
)
webgl_path.write_text(w)
