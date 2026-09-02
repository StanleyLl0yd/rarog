use rarog_types::Color;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

pub const DEFAULT_MAX_IMAGE_RESOURCES: usize = 1_024;
pub const DEFAULT_MAX_IMAGE_PIXELS_PER_RESOURCE: u64 = 16_777_216;
pub const DEFAULT_MAX_TOTAL_IMAGE_PIXELS: u64 = 67_108_864;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageResourceLimits {
    pub max_resources: usize,
    pub max_pixels_per_resource: u64,
    pub max_total_pixels: u64,
}

impl ImageResourceLimits {
    pub fn is_valid(self) -> bool {
        self.max_resources > 0
            && self.max_pixels_per_resource > 0
            && self.max_total_pixels > 0
            && self.max_pixels_per_resource <= self.max_total_pixels
    }
}

impl Default for ImageResourceLimits {
    fn default() -> Self {
        Self {
            max_resources: DEFAULT_MAX_IMAGE_RESOURCES,
            max_pixels_per_resource: DEFAULT_MAX_IMAGE_PIXELS_PER_RESOURCE,
            max_total_pixels: DEFAULT_MAX_TOTAL_IMAGE_PIXELS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageResourceId(u64);

impl ImageResourceId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageResourceRef {
    id: ImageResourceId,
    revision: u64,
}

impl ImageResourceRef {
    pub const fn id(self) -> ImageResourceId {
        self.id
    }

    pub const fn revision(self) -> u64 {
        self.revision
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageResourceStatus {
    Pending,
    Ready,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedImage {
    width: u32,
    height: u32,
    pixels: Arc<[Color]>,
}

impl DecodedImage {
    pub fn try_new(
        width: u32,
        height: u32,
        pixels: Vec<Color>,
    ) -> Result<Self, ImageResourceError> {
        if width == 0 || height == 0 {
            return Err(ImageResourceError::InvalidDimensions);
        }
        let expected = u64::from(width) * u64::from(height);
        let expected_length =
            usize::try_from(expected).map_err(|_| ImageResourceError::PixelCountOverflow)?;
        if pixels.len() != expected_length {
            return Err(ImageResourceError::PixelCountMismatch {
                expected,
                actual: pixels.len(),
            });
        }
        Ok(Self {
            width,
            height,
            pixels: pixels.into(),
        })
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn pixel_count(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    pub fn pixels(&self) -> &[Color] {
        &self.pixels
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<Color> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = u64::from(y) * u64::from(self.width) + u64::from(x);
        usize::try_from(index)
            .ok()
            .and_then(|index| self.pixels.get(index).copied())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageResourceError {
    InvalidLimits,
    InvalidDimensions,
    PixelCountOverflow,
    PixelCountMismatch { expected: u64, actual: usize },
    ResourceLimitExceeded { resources: usize, limit: usize },
    ImagePixelLimitExceeded { pixels: u64, limit: u64 },
    TotalPixelLimitExceeded { pixels: u64, limit: u64 },
    UnknownResource(ImageResourceId),
    InvalidState(ImageResourceId),
    ResourceIdExhausted,
    RevisionExhausted(ImageResourceId),
}

impl fmt::Display for ImageResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => formatter
                .write_str("image resource limits must be non-zero and internally consistent"),
            Self::InvalidDimensions => {
                formatter.write_str("decoded image dimensions must be non-zero")
            }
            Self::PixelCountOverflow => formatter
                .write_str("decoded image pixel count does not fit the current architecture"),
            Self::PixelCountMismatch { expected, actual } => write!(
                formatter,
                "decoded image requires {expected} pixels but received {actual}"
            ),
            Self::ResourceLimitExceeded { resources, limit } => write!(
                formatter,
                "image store would contain {resources} resources; limit is {limit}"
            ),
            Self::ImagePixelLimitExceeded { pixels, limit } => write!(
                formatter,
                "decoded image requires {pixels} pixels; per-resource limit is {limit}"
            ),
            Self::TotalPixelLimitExceeded { pixels, limit } => write!(
                formatter,
                "decoded image store would retain {pixels} pixels; limit is {limit}"
            ),
            Self::UnknownResource(id) => write!(formatter, "unknown image resource {}", id.get()),
            Self::InvalidState(id) => write!(
                formatter,
                "image resource {} is not in the required lifecycle state",
                id.get()
            ),
            Self::ResourceIdExhausted => {
                formatter.write_str("image resource identifier space is exhausted")
            }
            Self::RevisionExhausted(id) => write!(
                formatter,
                "image resource {} revision space is exhausted",
                id.get()
            ),
        }
    }
}

impl std::error::Error for ImageResourceError {}

#[derive(Clone, Debug)]
enum StoredImageState {
    Pending,
    Ready(DecodedImage),
    Failed,
}

#[derive(Clone, Debug)]
struct ImageResourceEntry {
    revision: u64,
    state: StoredImageState,
}

#[derive(Clone, Debug)]
pub struct ImageResourceStore {
    limits: ImageResourceLimits,
    next_id: u64,
    total_pixels: u64,
    entries: BTreeMap<ImageResourceId, ImageResourceEntry>,
}

impl Default for ImageResourceStore {
    fn default() -> Self {
        Self {
            limits: ImageResourceLimits::default(),
            next_id: 1,
            total_pixels: 0,
            entries: BTreeMap::new(),
        }
    }
}

impl ImageResourceStore {
    pub fn try_new(limits: ImageResourceLimits) -> Result<Self, ImageResourceError> {
        if !limits.is_valid() {
            return Err(ImageResourceError::InvalidLimits);
        }
        Ok(Self {
            limits,
            next_id: 1,
            total_pixels: 0,
            entries: BTreeMap::new(),
        })
    }

    pub const fn limits(&self) -> ImageResourceLimits {
        self.limits
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub const fn total_pixels(&self) -> u64 {
        self.total_pixels
    }

    pub fn reserve(&mut self) -> Result<ImageResourceId, ImageResourceError> {
        let resources = self.entries.len().saturating_add(1);
        if resources > self.limits.max_resources {
            return Err(ImageResourceError::ResourceLimitExceeded {
                resources,
                limit: self.limits.max_resources,
            });
        }
        let id = ImageResourceId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(ImageResourceError::ResourceIdExhausted)?;
        self.entries.insert(
            id,
            ImageResourceEntry {
                revision: 0,
                state: StoredImageState::Pending,
            },
        );
        Ok(id)
    }

    pub fn status(&self, id: ImageResourceId) -> Option<ImageResourceStatus> {
        self.entries.get(&id).map(|entry| match entry.state {
            StoredImageState::Pending => ImageResourceStatus::Pending,
            StoredImageState::Ready(_) => ImageResourceStatus::Ready,
            StoredImageState::Failed => ImageResourceStatus::Failed,
        })
    }

    pub fn resolve(
        &mut self,
        id: ImageResourceId,
        image: DecodedImage,
    ) -> Result<ImageResourceRef, ImageResourceError> {
        let entry = self
            .entries
            .get(&id)
            .ok_or(ImageResourceError::UnknownResource(id))?;
        if !matches!(entry.state, StoredImageState::Pending) {
            return Err(ImageResourceError::InvalidState(id));
        }
        let revision = entry
            .revision
            .checked_add(1)
            .ok_or(ImageResourceError::RevisionExhausted(id))?;
        let pixels = image.pixel_count();
        let total = self.validate_pixels(pixels, 0)?;
        let entry = self
            .entries
            .get_mut(&id)
            .expect("validated image resource exists");
        entry.revision = revision;
        entry.state = StoredImageState::Ready(image);
        self.total_pixels = total;
        Ok(ImageResourceRef { id, revision })
    }

    pub fn replace_ready(
        &mut self,
        id: ImageResourceId,
        image: DecodedImage,
    ) -> Result<ImageResourceRef, ImageResourceError> {
        let entry = self
            .entries
            .get(&id)
            .ok_or(ImageResourceError::UnknownResource(id))?;
        let old_pixels = match &entry.state {
            StoredImageState::Ready(image) => image.pixel_count(),
            StoredImageState::Pending | StoredImageState::Failed => {
                return Err(ImageResourceError::InvalidState(id));
            }
        };
        let revision = entry
            .revision
            .checked_add(1)
            .ok_or(ImageResourceError::RevisionExhausted(id))?;
        let total = self.validate_pixels(image.pixel_count(), old_pixels)?;
        let entry = self
            .entries
            .get_mut(&id)
            .expect("validated image resource exists");
        entry.revision = revision;
        entry.state = StoredImageState::Ready(image);
        self.total_pixels = total;
        Ok(ImageResourceRef { id, revision })
    }

    pub fn fail(&mut self, id: ImageResourceId) -> Result<(), ImageResourceError> {
        let entry = self
            .entries
            .get_mut(&id)
            .ok_or(ImageResourceError::UnknownResource(id))?;
        if !matches!(entry.state, StoredImageState::Pending) {
            return Err(ImageResourceError::InvalidState(id));
        }
        entry.revision = entry
            .revision
            .checked_add(1)
            .ok_or(ImageResourceError::RevisionExhausted(id))?;
        entry.state = StoredImageState::Failed;
        Ok(())
    }

    pub fn current_ref(&self, id: ImageResourceId) -> Option<ImageResourceRef> {
        let entry = self.entries.get(&id)?;
        matches!(entry.state, StoredImageState::Ready(_)).then_some(ImageResourceRef {
            id,
            revision: entry.revision,
        })
    }

    pub fn image(&self, reference: ImageResourceRef) -> Option<&DecodedImage> {
        let entry = self.entries.get(&reference.id)?;
        if entry.revision != reference.revision {
            return None;
        }
        match &entry.state {
            StoredImageState::Ready(image) => Some(image),
            StoredImageState::Pending | StoredImageState::Failed => None,
        }
    }

    pub fn remove(&mut self, id: ImageResourceId) -> bool {
        let Some(entry) = self.entries.remove(&id) else {
            return false;
        };
        if let StoredImageState::Ready(image) = entry.state {
            self.total_pixels = self.total_pixels.saturating_sub(image.pixel_count());
        }
        true
    }

    fn validate_pixels(
        &self,
        new_pixels: u64,
        replacing_pixels: u64,
    ) -> Result<u64, ImageResourceError> {
        if new_pixels > self.limits.max_pixels_per_resource {
            return Err(ImageResourceError::ImagePixelLimitExceeded {
                pixels: new_pixels,
                limit: self.limits.max_pixels_per_resource,
            });
        }
        let retained = self.total_pixels.saturating_sub(replacing_pixels);
        let total = retained
            .checked_add(new_pixels)
            .ok_or(ImageResourceError::PixelCountOverflow)?;
        if total > self.limits.max_total_pixels {
            return Err(ImageResourceError::TotalPixelLimitExceeded {
                pixels: total,
                limit: self.limits.max_total_pixels,
            });
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: u32, height: u32, color: Color) -> DecodedImage {
        DecodedImage::try_new(
            width,
            height,
            vec![color; usize::try_from(u64::from(width) * u64::from(height)).unwrap()],
        )
        .unwrap()
    }

    #[test]
    fn decoded_image_validates_shape_and_pixel_access() {
        let image = DecodedImage::try_new(2, 1, vec![Color::BLACK, Color::WHITE]).unwrap();
        assert_eq!(image.pixel(0, 0), Some(Color::BLACK));
        assert_eq!(image.pixel(1, 0), Some(Color::WHITE));
        assert_eq!(image.pixel(2, 0), None);
        assert_eq!(
            DecodedImage::try_new(2, 2, vec![Color::BLACK]),
            Err(ImageResourceError::PixelCountMismatch {
                expected: 4,
                actual: 1,
            })
        );
    }

    #[test]
    fn store_lifecycle_uses_stable_ids_and_revisioned_ready_refs() {
        let mut store = ImageResourceStore::default();
        let id = store.reserve().unwrap();
        assert_eq!(store.status(id), Some(ImageResourceStatus::Pending));
        let first = store.resolve(id, image(2, 2, Color::BLACK)).unwrap();
        assert_eq!(first.id(), id);
        assert_eq!(first.revision(), 1);
        assert_eq!(store.status(id), Some(ImageResourceStatus::Ready));
        assert!(store.image(first).is_some());

        let second = store.replace_ready(id, image(1, 1, Color::WHITE)).unwrap();
        assert_eq!(second.id(), id);
        assert_eq!(second.revision(), 2);
        assert!(store.image(first).is_none());
        assert_eq!(store.image(second).unwrap().pixel_count(), 1);
    }

    #[test]
    fn store_enforces_per_image_total_and_entry_limits() {
        let limits = ImageResourceLimits {
            max_resources: 2,
            max_pixels_per_resource: 4,
            max_total_pixels: 5,
        };
        let mut store = ImageResourceStore::try_new(limits).unwrap();
        let first = store.reserve().unwrap();
        let second = store.reserve().unwrap();
        assert_eq!(
            store.reserve(),
            Err(ImageResourceError::ResourceLimitExceeded {
                resources: 3,
                limit: 2,
            })
        );
        store.resolve(first, image(2, 2, Color::BLACK)).unwrap();
        assert_eq!(
            store.resolve(second, image(2, 1, Color::WHITE)),
            Err(ImageResourceError::TotalPixelLimitExceeded {
                pixels: 6,
                limit: 5,
            })
        );
        assert_eq!(store.status(second), Some(ImageResourceStatus::Pending));
        assert_eq!(store.total_pixels(), 4);
    }

    #[test]
    fn removal_releases_retained_pixel_budget_and_stale_refs() {
        let limits = ImageResourceLimits {
            max_resources: 2,
            max_pixels_per_resource: 4,
            max_total_pixels: 4,
        };
        let mut store = ImageResourceStore::try_new(limits).unwrap();
        let first = store.reserve().unwrap();
        let first_ref = store.resolve(first, image(2, 2, Color::BLACK)).unwrap();
        assert!(store.remove(first));
        assert_eq!(store.total_pixels(), 0);
        assert!(store.image(first_ref).is_none());

        let second = store.reserve().unwrap();
        store.resolve(second, image(2, 2, Color::WHITE)).unwrap();
        assert_eq!(store.total_pixels(), 4);
    }
}
