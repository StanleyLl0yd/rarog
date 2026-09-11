use rarog_canvas::{CanvasContentRevision, CanvasError, CanvasRegistry, CanvasSurfaceId};
use rarog_resources::{
    DecodedImage, ImageResourceError, ImageResourceId, ImageResourceRef, ImageResourceStore,
};
use std::collections::BTreeMap;
use std::fmt;

pub const DEFAULT_MAX_CANVAS_OUTPUT_BINDINGS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanvasOutputLimits {
    pub max_bindings: usize,
}

impl CanvasOutputLimits {
    pub const fn is_valid(self) -> bool {
        self.max_bindings > 0
    }
}

impl Default for CanvasOutputLimits {
    fn default() -> Self {
        Self {
            max_bindings: DEFAULT_MAX_CANVAS_OUTPUT_BINDINGS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasPublication {
    Updated(ImageResourceRef),
    Unchanged(ImageResourceRef),
}

impl CanvasPublication {
    pub const fn reference(self) -> ImageResourceRef {
        match self {
            Self::Updated(reference) | Self::Unchanged(reference) => reference,
        }
    }

    pub const fn changed(self) -> bool {
        matches!(self, Self::Updated(_))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CanvasOutputError {
    InvalidLimits,
    BindingLimitExceeded { bindings: usize, limit: usize },
    PixelCopyAllocationFailed { pixels: usize },
    Canvas(CanvasError),
    Image(ImageResourceError),
    InconsistentImageResource(CanvasSurfaceId),
}

impl fmt::Display for CanvasOutputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => formatter.write_str("Canvas output limits are invalid"),
            Self::BindingLimitExceeded { bindings, limit } => write!(
                formatter,
                "Canvas output bridge would contain {bindings} bindings; limit is {limit}"
            ),
            Self::PixelCopyAllocationFailed { pixels } => write!(
                formatter,
                "Canvas output could not allocate a bounded copy of {pixels} pixels"
            ),
            Self::Canvas(error) => write!(formatter, "Canvas output source failed: {error}"),
            Self::Image(error) => write!(formatter, "Canvas image publication failed: {error}"),
            Self::InconsistentImageResource(surface) => write!(
                formatter,
                "Canvas surface {}:{} no longer owns the image resource revision recorded by the output bridge",
                surface.scope(),
                surface.serial()
            ),
        }
    }
}

impl std::error::Error for CanvasOutputError {}

impl From<CanvasError> for CanvasOutputError {
    fn from(error: CanvasError) -> Self {
        Self::Canvas(error)
    }
}

impl From<ImageResourceError> for CanvasOutputError {
    fn from(error: ImageResourceError) -> Self {
        Self::Image(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CanvasOutputBinding {
    resource: ImageResourceId,
    published_revision: CanvasContentRevision,
    current_ref: ImageResourceRef,
}

#[derive(Debug)]
pub struct CanvasOutputBridge {
    limits: CanvasOutputLimits,
    bindings: BTreeMap<CanvasSurfaceId, CanvasOutputBinding>,
}

impl CanvasOutputBridge {
    pub fn try_new(limits: CanvasOutputLimits) -> Result<Self, CanvasOutputError> {
        if !limits.is_valid() {
            return Err(CanvasOutputError::InvalidLimits);
        }
        Ok(Self {
            limits,
            bindings: BTreeMap::new(),
        })
    }

    pub const fn limits(&self) -> CanvasOutputLimits {
        self.limits
    }

    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }

    pub fn image_reference(&self, surface: CanvasSurfaceId) -> Option<ImageResourceRef> {
        self.bindings
            .get(&surface)
            .map(|binding| binding.current_ref)
    }

    pub fn published_revision(&self, surface: CanvasSurfaceId) -> Option<CanvasContentRevision> {
        self.bindings
            .get(&surface)
            .map(|binding| binding.published_revision)
    }

    pub fn publish(
        &mut self,
        canvas: &CanvasRegistry,
        surface: CanvasSurfaceId,
        images: &mut ImageResourceStore,
    ) -> Result<CanvasPublication, CanvasOutputError> {
        let snapshot = canvas.surface_snapshot(surface)?;
        if let Some(binding) = self.bindings.get(&surface).copied() {
            return self.publish_existing(surface, binding, snapshot, images);
        }

        let next_count =
            self.bindings
                .len()
                .checked_add(1)
                .ok_or(CanvasOutputError::BindingLimitExceeded {
                    bindings: usize::MAX,
                    limit: self.limits.max_bindings,
                })?;
        if next_count > self.limits.max_bindings {
            return Err(CanvasOutputError::BindingLimitExceeded {
                bindings: next_count,
                limit: self.limits.max_bindings,
            });
        }

        let revision = snapshot.content_revision();
        let image = decoded_image(&snapshot)?;
        let resource = images.reserve()?;
        let reference = match images.resolve(resource, image) {
            Ok(reference) => reference,
            Err(error) => {
                let removed = images.remove(resource);
                debug_assert!(removed);
                return Err(CanvasOutputError::Image(error));
            }
        };
        let previous = self.bindings.insert(
            surface,
            CanvasOutputBinding {
                resource,
                published_revision: revision,
                current_ref: reference,
            },
        );
        debug_assert!(previous.is_none());
        Ok(CanvasPublication::Updated(reference))
    }

    pub fn detach(
        &mut self,
        surface: CanvasSurfaceId,
        images: &mut ImageResourceStore,
    ) -> Result<bool, CanvasOutputError> {
        let Some(binding) = self.bindings.get(&surface).copied() else {
            return Ok(false);
        };
        if images.current_ref(binding.resource) != Some(binding.current_ref) {
            return Err(CanvasOutputError::InconsistentImageResource(surface));
        }
        if !images.remove(binding.resource) {
            return Err(CanvasOutputError::InconsistentImageResource(surface));
        }
        self.bindings.remove(&surface);
        Ok(true)
    }

    fn publish_existing(
        &mut self,
        surface: CanvasSurfaceId,
        binding: CanvasOutputBinding,
        snapshot: rarog_canvas::CanvasSurfaceSnapshot,
        images: &mut ImageResourceStore,
    ) -> Result<CanvasPublication, CanvasOutputError> {
        if images.current_ref(binding.resource) != Some(binding.current_ref) {
            return Err(CanvasOutputError::InconsistentImageResource(surface));
        }
        if snapshot.content_revision() == binding.published_revision {
            return Ok(CanvasPublication::Unchanged(binding.current_ref));
        }
        if snapshot.content_revision() < binding.published_revision {
            return Err(CanvasOutputError::InconsistentImageResource(surface));
        }

        let image = decoded_image(&snapshot)?;
        let reference = images.replace_ready(binding.resource, image)?;
        let current = self
            .bindings
            .get_mut(&surface)
            .ok_or(CanvasOutputError::InconsistentImageResource(surface))?;
        if *current != binding {
            return Err(CanvasOutputError::InconsistentImageResource(surface));
        }
        current.published_revision = snapshot.content_revision();
        current.current_ref = reference;
        Ok(CanvasPublication::Updated(reference))
    }
}

fn decoded_image(
    snapshot: &rarog_canvas::CanvasSurfaceSnapshot,
) -> Result<DecodedImage, CanvasOutputError> {
    let pixels = snapshot.pixels();
    let mut copied = Vec::new();
    copied.try_reserve_exact(pixels.len()).map_err(|_| {
        CanvasOutputError::PixelCopyAllocationFailed {
            pixels: pixels.len(),
        }
    })?;
    copied.extend_from_slice(pixels);
    DecodedImage::try_new(snapshot.width(), snapshot.height(), copied).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_canvas::CanvasLimits;
    use rarog_compositor::{
        DisplayListRevision, FrameCause, FrameDecision, FramePlanner, FrameScheduler, SurfaceId,
        SurfaceSize,
    };
    use rarog_paint::{DamageRegion, DisplayCommand, DisplayItemId, DisplayList};
    use rarog_resources::ImageResourceLimits;
    use rarog_types::{Color, Rect};

    fn canvas_limits() -> CanvasLimits {
        CanvasLimits {
            max_surfaces: 4,
            max_contexts: 4,
            max_pixels_per_surface: 16,
            max_total_pixels: 32,
            max_state_stack_depth: 4,
        }
    }

    fn image_limits() -> ImageResourceLimits {
        ImageResourceLimits {
            max_resources: 4,
            max_pixels_per_resource: 16,
            max_total_pixels: 32,
        }
    }

    fn bridge() -> CanvasOutputBridge {
        CanvasOutputBridge::try_new(CanvasOutputLimits { max_bindings: 2 }).unwrap()
    }

    fn image_list(reference: ImageResourceRef) -> DisplayList {
        DisplayList::try_from_parts(
            vec![DisplayItemId {
                source: 1,
                fragment: 1,
                slot: 0,
            }],
            vec![DisplayCommand::DrawImage {
                rect: Rect::new(0.0, 0.0, 2.0, 2.0),
                image: reference,
            }],
        )
        .unwrap()
    }

    #[test]
    fn limits_must_be_non_zero() {
        assert!(matches!(
            CanvasOutputBridge::try_new(CanvasOutputLimits { max_bindings: 0 }),
            Err(CanvasOutputError::InvalidLimits)
        ));
    }

    #[test]
    fn first_publish_exports_transparent_canvas_output() {
        let mut canvas = CanvasRegistry::try_new(canvas_limits()).unwrap();
        let surface = canvas.create_surface(2, 2).unwrap();
        let mut images = ImageResourceStore::try_new(image_limits()).unwrap();
        let mut output = bridge();

        let publication = output.publish(&canvas, surface, &mut images).unwrap();
        assert!(publication.changed());
        let reference = publication.reference();
        assert_eq!(reference.revision(), 1);
        assert_eq!(output.binding_count(), 1);
        assert_eq!(output.image_reference(surface), Some(reference));
        assert_eq!(
            output.published_revision(surface),
            Some(canvas.surface(surface).unwrap().content_revision())
        );
        let image = images.image(reference).unwrap();
        assert_eq!(image.width(), 2);
        assert_eq!(image.height(), 2);
        assert_eq!(image.pixels(), &[Color::TRANSPARENT; 4]);
    }

    #[test]
    fn unchanged_publish_is_a_noop_for_image_revision() {
        let mut canvas = CanvasRegistry::try_new(canvas_limits()).unwrap();
        let surface = canvas.create_surface(2, 2).unwrap();
        let mut images = ImageResourceStore::try_new(image_limits()).unwrap();
        let mut output = bridge();
        let first = output.publish(&canvas, surface, &mut images).unwrap();
        let second = output.publish(&canvas, surface, &mut images).unwrap();

        assert!(first.changed());
        assert!(!second.changed());
        assert_eq!(first.reference(), second.reference());
        assert_eq!(images.len(), 1);
        assert_eq!(
            images.current_ref(first.reference().id()),
            Some(first.reference())
        );
    }

    #[test]
    fn changed_canvas_reuses_resource_and_advances_image_revision() {
        let mut canvas = CanvasRegistry::try_new(canvas_limits()).unwrap();
        let surface = canvas.create_surface(2, 2).unwrap();
        let mut images = ImageResourceStore::try_new(image_limits()).unwrap();
        let mut output = bridge();
        let first = output.publish(&canvas, surface, &mut images).unwrap();
        let old = first.reference();

        canvas.fill_surface(surface, Color::WHITE).unwrap();
        let second = output.publish(&canvas, surface, &mut images).unwrap();
        let new = second.reference();

        assert!(second.changed());
        assert_eq!(old.id(), new.id());
        assert_eq!(new.revision(), old.revision() + 1);
        assert!(images.image(old).is_none());
        assert_eq!(images.image(new).unwrap().pixels(), &[Color::WHITE; 4]);
    }

    #[test]
    fn bridge_rejects_foreign_or_retired_surface_authority() {
        let mut first = CanvasRegistry::try_new(canvas_limits()).unwrap();
        let mut second = CanvasRegistry::try_new(canvas_limits()).unwrap();
        let foreign = first.create_surface(1, 1).unwrap();
        let retired = second.create_surface(1, 1).unwrap();
        second.retire_surface(retired).unwrap();
        let mut images = ImageResourceStore::try_new(image_limits()).unwrap();
        let mut output = bridge();

        assert!(matches!(
            output.publish(&second, foreign, &mut images),
            Err(CanvasOutputError::Canvas(CanvasError::UnknownSurface(id))) if id == foreign
        ));
        assert!(matches!(
            output.publish(&second, retired, &mut images),
            Err(CanvasOutputError::Canvas(CanvasError::UnknownSurface(id))) if id == retired
        ));
        assert_eq!(output.binding_count(), 0);
        assert_eq!(images.len(), 0);
    }

    #[test]
    fn binding_limit_is_checked_before_image_store_mutation() {
        let mut canvas = CanvasRegistry::try_new(canvas_limits()).unwrap();
        let first = canvas.create_surface(1, 1).unwrap();
        let second = canvas.create_surface(1, 1).unwrap();
        let mut images = ImageResourceStore::try_new(image_limits()).unwrap();
        let mut output =
            CanvasOutputBridge::try_new(CanvasOutputLimits { max_bindings: 1 }).unwrap();
        output.publish(&canvas, first, &mut images).unwrap();
        let before = images.len();

        assert_eq!(
            output.publish(&canvas, second, &mut images),
            Err(CanvasOutputError::BindingLimitExceeded {
                bindings: 2,
                limit: 1,
            })
        );
        assert_eq!(output.binding_count(), 1);
        assert_eq!(images.len(), before);
        assert!(output.image_reference(second).is_none());
    }

    #[test]
    fn failed_first_publication_releases_reserved_resource_and_keeps_bridge_clean() {
        let mut canvas = CanvasRegistry::try_new(canvas_limits()).unwrap();
        let surface = canvas.create_surface(2, 2).unwrap();
        let mut images = ImageResourceStore::try_new(ImageResourceLimits {
            max_resources: 1,
            max_pixels_per_resource: 1,
            max_total_pixels: 1,
        })
        .unwrap();
        let mut output = bridge();

        assert_eq!(
            output.publish(&canvas, surface, &mut images),
            Err(CanvasOutputError::Image(
                ImageResourceError::ImagePixelLimitExceeded {
                    pixels: 4,
                    limit: 1,
                }
            ))
        );
        assert_eq!(output.binding_count(), 0);
        assert_eq!(images.len(), 0);
        assert_eq!(images.total_pixels(), 0);
    }

    #[test]
    fn external_resource_drift_fails_closed_without_advancing_published_revision() {
        let mut canvas = CanvasRegistry::try_new(canvas_limits()).unwrap();
        let surface = canvas.create_surface(1, 1).unwrap();
        let mut images = ImageResourceStore::try_new(image_limits()).unwrap();
        let mut output = bridge();
        let first = output.publish(&canvas, surface, &mut images).unwrap();
        let published = output.published_revision(surface).unwrap();
        assert!(images.remove(first.reference().id()));
        canvas.fill_surface(surface, Color::WHITE).unwrap();

        assert_eq!(
            output.publish(&canvas, surface, &mut images),
            Err(CanvasOutputError::InconsistentImageResource(surface))
        );
        assert_eq!(output.published_revision(surface), Some(published));
        assert_eq!(output.image_reference(surface), Some(first.reference()));
    }

    #[test]
    fn detach_releases_image_and_binding_capacity() {
        let mut canvas = CanvasRegistry::try_new(canvas_limits()).unwrap();
        let first = canvas.create_surface(1, 1).unwrap();
        let second = canvas.create_surface(1, 1).unwrap();
        let mut images = ImageResourceStore::try_new(ImageResourceLimits {
            max_resources: 1,
            ..image_limits()
        })
        .unwrap();
        let mut output =
            CanvasOutputBridge::try_new(CanvasOutputLimits { max_bindings: 1 }).unwrap();
        let first_ref = output
            .publish(&canvas, first, &mut images)
            .unwrap()
            .reference();

        assert!(output.detach(first, &mut images).unwrap());
        assert_eq!(output.binding_count(), 0);
        assert_eq!(images.len(), 0);
        assert!(images.image(first_ref).is_none());
        let second_ref = output
            .publish(&canvas, second, &mut images)
            .unwrap()
            .reference();
        assert_ne!(first_ref.id(), second_ref.id());
        assert_eq!(output.binding_count(), 1);
        assert_eq!(images.len(), 1);
    }

    #[test]
    fn resource_revision_refresh_drives_paint_damage_and_compositor_scheduling() {
        let mut canvas = CanvasRegistry::try_new(canvas_limits()).unwrap();
        let surface = canvas.create_surface(2, 2).unwrap();
        let mut images = ImageResourceStore::try_new(image_limits()).unwrap();
        let mut output = bridge();
        let old_ref = output
            .publish(&canvas, surface, &mut images)
            .unwrap()
            .reference();
        let mut list = image_list(old_ref);
        let old_snapshot = list.snapshot();

        let compositor_surface = SurfaceId::new(77).unwrap();
        let size = SurfaceSize::new(2, 2);
        let display_revision = DisplayListRevision::new(1);
        let mut planner = FramePlanner::new(compositor_surface);
        let FrameDecision::Submit(initial) = planner
            .plan(
                size,
                display_revision,
                &DamageRegion::default(),
                FrameCause::SceneChange,
            )
            .unwrap()
        else {
            panic!("initial Canvas image frame must submit");
        };
        planner.complete(initial.id()).unwrap();

        canvas.fill_surface(surface, Color::WHITE).unwrap();
        let publication = output.publish(&canvas, surface, &mut images).unwrap();
        let new_ref = publication.reference();
        assert!(publication.changed());
        assert_eq!(old_ref.id(), new_ref.id());
        assert!(images.image(old_ref).is_none());
        assert!(images.image(new_ref).is_some());

        let refresh = list.refresh_image_resource(new_ref);
        assert_eq!(refresh.updated_commands, 1);
        assert_eq!(refresh.damage.rects, vec![Rect::new(0.0, 0.0, 2.0, 2.0)]);
        assert_ne!(old_snapshot, list.snapshot());

        let mut scheduler = FrameScheduler::new();
        scheduler.request(FrameCause::ResourceReady);
        let request = scheduler.begin().unwrap().unwrap();
        assert!(request.reasons().contains(FrameCause::ResourceReady));
        let FrameDecision::Submit(update) = planner
            .plan(
                size,
                display_revision,
                &refresh.damage,
                request.primary_cause(),
            )
            .unwrap()
        else {
            panic!("Canvas image resource refresh must schedule a frame");
        };
        assert_eq!(update.cause(), FrameCause::ResourceReady);
        assert_eq!(update.revision(), display_revision);
    }
}
