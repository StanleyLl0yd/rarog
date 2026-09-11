use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_MAX_MEDIA_RESOURCES: usize = 256;
pub const DEFAULT_MAX_MEDIA_STREAMS: usize = 2_048;
pub const DEFAULT_MAX_MEDIA_STREAMS_PER_RESOURCE: usize = 32;
pub const DEFAULT_MAX_MEDIA_PLAYBACKS: usize = 256;
pub const DEFAULT_MAX_AUDIO_CHANNELS: u16 = 64;
pub const DEFAULT_MAX_AUDIO_SAMPLE_RATE_HZ: u32 = 768_000;
pub const DEFAULT_MAX_VIDEO_WIDTH: u32 = 16_384;
pub const DEFAULT_MAX_VIDEO_HEIGHT: u32 = 16_384;

static NEXT_MEDIA_REGISTRY_SCOPE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaLimits {
    pub max_resources: usize,
    pub max_streams: usize,
    pub max_streams_per_resource: usize,
    pub max_playbacks: usize,
    pub max_audio_channels: u16,
    pub max_audio_sample_rate_hz: u32,
    pub max_video_width: u32,
    pub max_video_height: u32,
}

impl MediaLimits {
    pub const fn is_valid(self) -> bool {
        self.max_resources > 0
            && self.max_streams > 0
            && self.max_streams_per_resource > 0
            && self.max_streams_per_resource <= self.max_streams
            && self.max_playbacks > 0
            && self.max_audio_channels > 0
            && self.max_audio_sample_rate_hz > 0
            && self.max_video_width > 0
            && self.max_video_height > 0
    }
}

impl Default for MediaLimits {
    fn default() -> Self {
        Self {
            max_resources: DEFAULT_MAX_MEDIA_RESOURCES,
            max_streams: DEFAULT_MAX_MEDIA_STREAMS,
            max_streams_per_resource: DEFAULT_MAX_MEDIA_STREAMS_PER_RESOURCE,
            max_playbacks: DEFAULT_MAX_MEDIA_PLAYBACKS,
            max_audio_channels: DEFAULT_MAX_AUDIO_CHANNELS,
            max_audio_sample_rate_hz: DEFAULT_MAX_AUDIO_SAMPLE_RATE_HZ,
            max_video_width: DEFAULT_MAX_VIDEO_WIDTH,
            max_video_height: DEFAULT_MAX_VIDEO_HEIGHT,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaResourceId {
    scope: NonZeroU64,
    serial: NonZeroU64,
}

impl MediaResourceId {
    pub const fn scope(self) -> u64 {
        self.scope.get()
    }

    pub const fn serial(self) -> u64 {
        self.serial.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaStreamId {
    scope: NonZeroU64,
    serial: NonZeroU64,
}

impl MediaStreamId {
    pub const fn scope(self) -> u64 {
        self.scope.get()
    }

    pub const fn serial(self) -> u64 {
        self.serial.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaPlaybackId {
    scope: NonZeroU64,
    serial: NonZeroU64,
}

impl MediaPlaybackId {
    pub const fn scope(self) -> u64 {
        self.scope.get()
    }

    pub const fn serial(self) -> u64 {
        self.serial.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MediaTime(u64);

impl MediaTime {
    pub const ZERO: Self = Self(0);

    pub const fn from_micros(micros: u64) -> Self {
        Self(micros)
    }

    pub const fn as_micros(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaStreamKind {
    Audio,
    Video,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaStreamDescriptor {
    Audio { channels: u16, sample_rate_hz: u32 },
    Video { width: u32, height: u32 },
}

impl MediaStreamDescriptor {
    pub const fn kind(self) -> MediaStreamKind {
        match self {
            Self::Audio { .. } => MediaStreamKind::Audio,
            Self::Video { .. } => MediaStreamKind::Video,
        }
    }

    fn validate(self, limits: MediaLimits) -> Result<(), MediaError> {
        match self {
            Self::Audio {
                channels,
                sample_rate_hz,
            } => {
                if channels == 0 || channels > limits.max_audio_channels {
                    return Err(MediaError::InvalidAudioChannels {
                        channels,
                        limit: limits.max_audio_channels,
                    });
                }
                if sample_rate_hz == 0 || sample_rate_hz > limits.max_audio_sample_rate_hz {
                    return Err(MediaError::InvalidAudioSampleRate {
                        sample_rate_hz,
                        limit: limits.max_audio_sample_rate_hz,
                    });
                }
            }
            Self::Video { width, height } => {
                if width == 0
                    || height == 0
                    || width > limits.max_video_width
                    || height > limits.max_video_height
                {
                    return Err(MediaError::InvalidVideoDimensions {
                        width,
                        height,
                        max_width: limits.max_video_width,
                        max_height: limits.max_video_height,
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaResourceDescriptor {
    duration: MediaTime,
    streams: Vec<MediaStreamDescriptor>,
}

impl MediaResourceDescriptor {
    pub fn try_new(
        duration: MediaTime,
        streams: &[MediaStreamDescriptor],
        limits: MediaLimits,
    ) -> Result<Self, MediaError> {
        validate_limits(limits)?;
        validate_stream_descriptors(streams, limits)?;
        Ok(Self {
            duration,
            streams: streams.to_vec(),
        })
    }

    pub const fn duration(&self) -> MediaTime {
        self.duration
    }

    pub fn streams(&self) -> &[MediaStreamDescriptor] {
        &self.streams
    }

    fn validate_against(&self, limits: MediaLimits) -> Result<(), MediaError> {
        validate_stream_descriptors(&self.streams, limits)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaResource {
    id: MediaResourceId,
    duration: MediaTime,
    streams: Vec<MediaStreamId>,
}

impl MediaResource {
    pub const fn id(&self) -> MediaResourceId {
        self.id
    }

    pub const fn duration(&self) -> MediaTime {
        self.duration
    }

    pub fn streams(&self) -> &[MediaStreamId] {
        &self.streams
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaStream {
    id: MediaStreamId,
    resource: MediaResourceId,
    descriptor: MediaStreamDescriptor,
}

impl MediaStream {
    pub const fn id(&self) -> MediaStreamId {
        self.id
    }

    pub const fn resource(&self) -> MediaResourceId {
        self.resource
    }

    pub const fn descriptor(&self) -> MediaStreamDescriptor {
        self.descriptor
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaPlaybackState {
    Ready,
    Playing,
    Paused,
    Ended,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaPlayback {
    id: MediaPlaybackId,
    resource: MediaResourceId,
    streams: Vec<MediaStreamId>,
    state: MediaPlaybackState,
    position: MediaTime,
}

impl MediaPlayback {
    pub const fn id(&self) -> MediaPlaybackId {
        self.id
    }

    pub const fn resource(&self) -> MediaResourceId {
        self.resource
    }

    pub fn streams(&self) -> &[MediaStreamId] {
        &self.streams
    }

    pub const fn state(&self) -> MediaPlaybackState {
        self.state
    }

    pub const fn position(&self) -> MediaTime {
        self.position
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaRegistrySnapshot {
    resources: usize,
    streams: usize,
    playbacks: usize,
}

impl MediaRegistrySnapshot {
    pub const fn resources(self) -> usize {
        self.resources
    }

    pub const fn streams(self) -> usize {
        self.streams
    }

    pub const fn playbacks(self) -> usize {
        self.playbacks
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaIdentityKind {
    Registry,
    Resource,
    Stream,
    Playback,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MediaError {
    InvalidLimits,
    NoStreams,
    ResourceLimitExceeded {
        resources: usize,
        limit: usize,
    },
    StreamLimitExceeded {
        streams: usize,
        limit: usize,
    },
    StreamsPerResourceLimitExceeded {
        streams: usize,
        limit: usize,
    },
    PlaybackLimitExceeded {
        playbacks: usize,
        limit: usize,
    },
    InvalidAudioChannels {
        channels: u16,
        limit: u16,
    },
    InvalidAudioSampleRate {
        sample_rate_hz: u32,
        limit: u32,
    },
    InvalidVideoDimensions {
        width: u32,
        height: u32,
        max_width: u32,
        max_height: u32,
    },
    UnknownResource(MediaResourceId),
    UnknownStream(MediaStreamId),
    UnknownPlayback(MediaPlaybackId),
    NoSelectedStreams,
    DuplicateStream(MediaStreamId),
    StreamNotOwnedByResource {
        stream: MediaStreamId,
        resource: MediaResourceId,
    },
    ResourceInUse(MediaResourceId),
    InvalidPlaybackTransition {
        playback: MediaPlaybackId,
        state: MediaPlaybackState,
    },
    PositionOutOfRange {
        position: MediaTime,
        duration: MediaTime,
    },
    IdentitySpaceExhausted(MediaIdentityKind),
    ArithmeticOverflow,
    InconsistentState,
}

impl fmt::Display for MediaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => {
                formatter.write_str("media limits must be non-zero and internally consistent")
            }
            Self::NoStreams => {
                formatter.write_str("media resource must contain at least one stream")
            }
            Self::ResourceLimitExceeded { resources, limit } => write!(
                formatter,
                "media registry would retain {resources} resources; limit is {limit}"
            ),
            Self::StreamLimitExceeded { streams, limit } => write!(
                formatter,
                "media registry would retain {streams} streams; limit is {limit}"
            ),
            Self::StreamsPerResourceLimitExceeded { streams, limit } => write!(
                formatter,
                "media resource would retain {streams} streams; limit is {limit}"
            ),
            Self::PlaybackLimitExceeded { playbacks, limit } => write!(
                formatter,
                "media registry would retain {playbacks} playback sessions; limit is {limit}"
            ),
            Self::InvalidAudioChannels { channels, limit } => write!(
                formatter,
                "audio stream declares {channels} channels; limit is {limit}"
            ),
            Self::InvalidAudioSampleRate {
                sample_rate_hz,
                limit,
            } => write!(
                formatter,
                "audio stream declares {sample_rate_hz} Hz; limit is {limit} Hz"
            ),
            Self::InvalidVideoDimensions {
                width,
                height,
                max_width,
                max_height,
            } => write!(
                formatter,
                "video stream dimensions {width}x{height} exceed valid non-zero bounds {max_width}x{max_height}"
            ),
            Self::UnknownResource(id) => write!(
                formatter,
                "unknown or foreign media resource {}:{}",
                id.scope(),
                id.serial()
            ),
            Self::UnknownStream(id) => write!(
                formatter,
                "unknown or foreign media stream {}:{}",
                id.scope(),
                id.serial()
            ),
            Self::UnknownPlayback(id) => write!(
                formatter,
                "unknown or foreign media playback {}:{}",
                id.scope(),
                id.serial()
            ),
            Self::NoSelectedStreams => {
                formatter.write_str("media playback must select at least one stream")
            }
            Self::DuplicateStream(id) => write!(
                formatter,
                "media playback selected stream {}:{} more than once",
                id.scope(),
                id.serial()
            ),
            Self::StreamNotOwnedByResource { stream, resource } => write!(
                formatter,
                "media stream {}:{} is not owned by resource {}:{}",
                stream.scope(),
                stream.serial(),
                resource.scope(),
                resource.serial()
            ),
            Self::ResourceInUse(id) => write!(
                formatter,
                "media resource {}:{} still owns a live playback session",
                id.scope(),
                id.serial()
            ),
            Self::InvalidPlaybackTransition { playback, state } => write!(
                formatter,
                "media playback {}:{} cannot transition from {state:?}",
                playback.scope(),
                playback.serial()
            ),
            Self::PositionOutOfRange { position, duration } => write!(
                formatter,
                "media position {}us exceeds resource duration {}us",
                position.as_micros(),
                duration.as_micros()
            ),
            Self::IdentitySpaceExhausted(kind) => {
                write!(formatter, "media {kind:?} identity space is exhausted")
            }
            Self::ArithmeticOverflow => formatter.write_str("media capacity arithmetic overflowed"),
            Self::InconsistentState => {
                formatter.write_str("media registry detected inconsistent retained ownership")
            }
        }
    }
}

impl std::error::Error for MediaError {}

#[derive(Debug)]
pub struct MediaRegistry {
    limits: MediaLimits,
    scope: NonZeroU64,
    next_resource: Option<NonZeroU64>,
    next_stream: Option<NonZeroU64>,
    next_playback: Option<NonZeroU64>,
    resources: BTreeMap<MediaResourceId, MediaResource>,
    streams: BTreeMap<MediaStreamId, MediaStream>,
    playbacks: BTreeMap<MediaPlaybackId, MediaPlayback>,
}

impl MediaRegistry {
    pub fn try_new(limits: MediaLimits) -> Result<Self, MediaError> {
        validate_limits(limits)?;
        Ok(Self {
            limits,
            scope: allocate_scope()?,
            next_resource: NonZeroU64::new(1),
            next_stream: NonZeroU64::new(1),
            next_playback: NonZeroU64::new(1),
            resources: BTreeMap::new(),
            streams: BTreeMap::new(),
            playbacks: BTreeMap::new(),
        })
    }

    pub const fn limits(&self) -> MediaLimits {
        self.limits
    }

    pub fn snapshot(&self) -> MediaRegistrySnapshot {
        MediaRegistrySnapshot {
            resources: self.resources.len(),
            streams: self.streams.len(),
            playbacks: self.playbacks.len(),
        }
    }

    pub fn resource(&self, id: MediaResourceId) -> Result<&MediaResource, MediaError> {
        self.resources
            .get(&id)
            .ok_or(MediaError::UnknownResource(id))
    }

    pub fn stream(&self, id: MediaStreamId) -> Result<&MediaStream, MediaError> {
        self.streams.get(&id).ok_or(MediaError::UnknownStream(id))
    }

    pub fn playback(&self, id: MediaPlaybackId) -> Result<&MediaPlayback, MediaError> {
        self.playbacks
            .get(&id)
            .ok_or(MediaError::UnknownPlayback(id))
    }

    pub fn create_resource(
        &mut self,
        descriptor: MediaResourceDescriptor,
    ) -> Result<MediaResourceId, MediaError> {
        descriptor.validate_against(self.limits)?;
        let resources = self
            .resources
            .len()
            .checked_add(1)
            .ok_or(MediaError::ArithmeticOverflow)?;
        if resources > self.limits.max_resources {
            return Err(MediaError::ResourceLimitExceeded {
                resources,
                limit: self.limits.max_resources,
            });
        }
        let streams = self
            .streams
            .len()
            .checked_add(descriptor.streams.len())
            .ok_or(MediaError::ArithmeticOverflow)?;
        if streams > self.limits.max_streams {
            return Err(MediaError::StreamLimitExceeded {
                streams,
                limit: self.limits.max_streams,
            });
        }

        let id = MediaResourceId {
            scope: self.scope,
            serial: allocate_serial(&mut self.next_resource, MediaIdentityKind::Resource)?,
        };
        if self.resources.contains_key(&id) {
            return Err(MediaError::InconsistentState);
        }

        let mut stream_entries = Vec::with_capacity(descriptor.streams.len());
        for stream_descriptor in descriptor.streams.iter().copied() {
            let stream_id = MediaStreamId {
                scope: self.scope,
                serial: allocate_serial(&mut self.next_stream, MediaIdentityKind::Stream)?,
            };
            if self.streams.contains_key(&stream_id)
                || stream_entries
                    .iter()
                    .any(|(existing, _)| *existing == stream_id)
            {
                return Err(MediaError::InconsistentState);
            }
            stream_entries.push((stream_id, stream_descriptor));
        }

        let stream_ids = stream_entries.iter().map(|(stream, _)| *stream).collect();
        for (stream_id, stream_descriptor) in stream_entries {
            if self
                .streams
                .insert(
                    stream_id,
                    MediaStream {
                        id: stream_id,
                        resource: id,
                        descriptor: stream_descriptor,
                    },
                )
                .is_some()
            {
                return Err(MediaError::InconsistentState);
            }
        }
        if self
            .resources
            .insert(
                id,
                MediaResource {
                    id,
                    duration: descriptor.duration,
                    streams: stream_ids,
                },
            )
            .is_some()
        {
            return Err(MediaError::InconsistentState);
        }
        Ok(id)
    }

    pub fn retire_resource(&mut self, id: MediaResourceId) -> Result<(), MediaError> {
        let resource = self.resource(id)?;
        if self
            .playbacks
            .values()
            .any(|playback| playback.resource == id)
        {
            return Err(MediaError::ResourceInUse(id));
        }
        for stream in &resource.streams {
            let retained = self.stream(*stream)?;
            if retained.resource != id {
                return Err(MediaError::InconsistentState);
            }
        }
        let streams = resource.streams.clone();
        self.resources
            .remove(&id)
            .ok_or(MediaError::InconsistentState)?;
        for stream in streams {
            self.streams
                .remove(&stream)
                .ok_or(MediaError::InconsistentState)?;
        }
        Ok(())
    }

    pub fn create_playback(
        &mut self,
        resource: MediaResourceId,
        selected_streams: &[MediaStreamId],
    ) -> Result<MediaPlaybackId, MediaError> {
        let owned_streams = self.resource(resource)?.streams.clone();
        if selected_streams.is_empty() {
            return Err(MediaError::NoSelectedStreams);
        }
        if selected_streams.len() > self.limits.max_streams_per_resource {
            return Err(MediaError::StreamsPerResourceLimitExceeded {
                streams: selected_streams.len(),
                limit: self.limits.max_streams_per_resource,
            });
        }
        for (index, stream) in selected_streams.iter().copied().enumerate() {
            if selected_streams[..index].contains(&stream) {
                return Err(MediaError::DuplicateStream(stream));
            }
            let retained = self.stream(stream)?;
            if retained.resource != resource || !owned_streams.contains(&stream) {
                return Err(MediaError::StreamNotOwnedByResource { stream, resource });
            }
        }
        let playbacks = self
            .playbacks
            .len()
            .checked_add(1)
            .ok_or(MediaError::ArithmeticOverflow)?;
        if playbacks > self.limits.max_playbacks {
            return Err(MediaError::PlaybackLimitExceeded {
                playbacks,
                limit: self.limits.max_playbacks,
            });
        }

        let id = MediaPlaybackId {
            scope: self.scope,
            serial: allocate_serial(&mut self.next_playback, MediaIdentityKind::Playback)?,
        };
        if self.playbacks.contains_key(&id) {
            return Err(MediaError::InconsistentState);
        }
        if self
            .playbacks
            .insert(
                id,
                MediaPlayback {
                    id,
                    resource,
                    streams: selected_streams.to_vec(),
                    state: MediaPlaybackState::Ready,
                    position: MediaTime::ZERO,
                },
            )
            .is_some()
        {
            return Err(MediaError::InconsistentState);
        }
        Ok(id)
    }

    pub fn play(&mut self, id: MediaPlaybackId) -> Result<(), MediaError> {
        let playback = self.playback_mut(id)?;
        match playback.state {
            MediaPlaybackState::Ready | MediaPlaybackState::Paused => {
                playback.state = MediaPlaybackState::Playing;
                Ok(())
            }
            MediaPlaybackState::Playing => Ok(()),
            MediaPlaybackState::Ended => Err(MediaError::InvalidPlaybackTransition {
                playback: id,
                state: MediaPlaybackState::Ended,
            }),
        }
    }

    pub fn pause(&mut self, id: MediaPlaybackId) -> Result<(), MediaError> {
        let playback = self.playback_mut(id)?;
        match playback.state {
            MediaPlaybackState::Playing => {
                playback.state = MediaPlaybackState::Paused;
                Ok(())
            }
            MediaPlaybackState::Paused => Ok(()),
            state => Err(MediaError::InvalidPlaybackTransition {
                playback: id,
                state,
            }),
        }
    }

    pub fn set_playback_position(
        &mut self,
        id: MediaPlaybackId,
        position: MediaTime,
    ) -> Result<(), MediaError> {
        let resource = self.playback(id)?.resource;
        let duration = self.resource(resource)?.duration;
        if position > duration {
            return Err(MediaError::PositionOutOfRange { position, duration });
        }
        let playback = self.playback_mut(id)?;
        if playback.state == MediaPlaybackState::Ended {
            return Err(MediaError::InvalidPlaybackTransition {
                playback: id,
                state: MediaPlaybackState::Ended,
            });
        }
        playback.position = position;
        Ok(())
    }

    pub fn mark_ended(&mut self, id: MediaPlaybackId) -> Result<(), MediaError> {
        let resource = self.playback(id)?.resource;
        let duration = self.resource(resource)?.duration;
        let playback = self.playback_mut(id)?;
        playback.state = MediaPlaybackState::Ended;
        playback.position = duration;
        Ok(())
    }

    pub fn retire_playback(&mut self, id: MediaPlaybackId) -> Result<(), MediaError> {
        self.playbacks
            .remove(&id)
            .ok_or(MediaError::UnknownPlayback(id))?;
        Ok(())
    }

    fn playback_mut(&mut self, id: MediaPlaybackId) -> Result<&mut MediaPlayback, MediaError> {
        self.playbacks
            .get_mut(&id)
            .ok_or(MediaError::UnknownPlayback(id))
    }
}

fn validate_limits(limits: MediaLimits) -> Result<(), MediaError> {
    if limits.is_valid() {
        Ok(())
    } else {
        Err(MediaError::InvalidLimits)
    }
}

fn validate_stream_descriptors(
    streams: &[MediaStreamDescriptor],
    limits: MediaLimits,
) -> Result<(), MediaError> {
    if streams.is_empty() {
        return Err(MediaError::NoStreams);
    }
    if streams.len() > limits.max_streams_per_resource {
        return Err(MediaError::StreamsPerResourceLimitExceeded {
            streams: streams.len(),
            limit: limits.max_streams_per_resource,
        });
    }
    for stream in streams {
        stream.validate(limits)?;
    }
    Ok(())
}

fn allocate_scope() -> Result<NonZeroU64, MediaError> {
    let raw = NEXT_MEDIA_REGISTRY_SCOPE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| MediaError::IdentitySpaceExhausted(MediaIdentityKind::Registry))?;
    NonZeroU64::new(raw).ok_or(MediaError::IdentitySpaceExhausted(
        MediaIdentityKind::Registry,
    ))
}

fn allocate_serial(
    next: &mut Option<NonZeroU64>,
    kind: MediaIdentityKind,
) -> Result<NonZeroU64, MediaError> {
    let current = (*next).ok_or(MediaError::IdentitySpaceExhausted(kind))?;
    *next = current.get().checked_add(1).and_then(NonZeroU64::new);
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> MediaLimits {
        MediaLimits {
            max_resources: 3,
            max_streams: 5,
            max_streams_per_resource: 3,
            max_playbacks: 2,
            max_audio_channels: 8,
            max_audio_sample_rate_hz: 192_000,
            max_video_width: 4_096,
            max_video_height: 2_160,
        }
    }

    fn audio() -> MediaStreamDescriptor {
        MediaStreamDescriptor::Audio {
            channels: 2,
            sample_rate_hz: 48_000,
        }
    }

    fn video() -> MediaStreamDescriptor {
        MediaStreamDescriptor::Video {
            width: 1_920,
            height: 1_080,
        }
    }

    fn descriptor(streams: &[MediaStreamDescriptor]) -> MediaResourceDescriptor {
        MediaResourceDescriptor::try_new(MediaTime::from_micros(5_000_000), streams, limits())
            .unwrap()
    }

    #[test]
    fn limits_and_stream_metadata_fail_closed() {
        let mut invalid = limits();
        invalid.max_resources = 0;
        assert_eq!(
            MediaRegistry::try_new(invalid).unwrap_err(),
            MediaError::InvalidLimits
        );
        assert_eq!(
            MediaResourceDescriptor::try_new(MediaTime::ZERO, &[], limits()).unwrap_err(),
            MediaError::NoStreams
        );
        assert_eq!(
            MediaResourceDescriptor::try_new(
                MediaTime::ZERO,
                &[MediaStreamDescriptor::Audio {
                    channels: 9,
                    sample_rate_hz: 48_000,
                }],
                limits(),
            )
            .unwrap_err(),
            MediaError::InvalidAudioChannels {
                channels: 9,
                limit: 8,
            }
        );
        assert!(matches!(
            MediaResourceDescriptor::try_new(
                MediaTime::ZERO,
                &[MediaStreamDescriptor::Video {
                    width: 8_192,
                    height: 1_080,
                }],
                limits(),
            ),
            Err(MediaError::InvalidVideoDimensions { .. })
        ));
    }

    #[test]
    fn identities_are_scoped_monotonic_and_foreign_ids_fail_closed() {
        let mut first = MediaRegistry::try_new(limits()).unwrap();
        let mut second = MediaRegistry::try_new(limits()).unwrap();
        let first_id = first.create_resource(descriptor(&[audio()])).unwrap();
        let second_first = first.create_resource(descriptor(&[video()])).unwrap();
        let foreign = second.create_resource(descriptor(&[audio()])).unwrap();

        assert_eq!(second_first.scope(), first_id.scope());
        assert!(second_first.serial() > first_id.serial());
        assert_ne!(foreign.scope(), first_id.scope());
        assert_eq!(
            second.resource(first_id).unwrap_err(),
            MediaError::UnknownResource(first_id)
        );
    }

    #[test]
    fn resource_and_stream_capacity_recovers_exactly_after_retirement() {
        let mut constrained = limits();
        constrained.max_resources = 2;
        constrained.max_streams = 2;
        constrained.max_streams_per_resource = 2;
        let mut registry = MediaRegistry::try_new(constrained).unwrap();
        let first = registry
            .create_resource(
                MediaResourceDescriptor::try_new(MediaTime::ZERO, &[audio(), video()], constrained)
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(registry.snapshot().streams(), 2);
        assert!(matches!(
            registry.create_resource(
                MediaResourceDescriptor::try_new(MediaTime::ZERO, &[audio()], constrained).unwrap()
            ),
            Err(MediaError::StreamLimitExceeded { .. })
        ));

        let stale_stream = registry.resource(first).unwrap().streams()[0];
        registry.retire_resource(first).unwrap();
        assert_eq!(registry.snapshot().streams(), 0);
        assert_eq!(
            registry.stream(stale_stream).unwrap_err(),
            MediaError::UnknownStream(stale_stream)
        );
        registry
            .create_resource(
                MediaResourceDescriptor::try_new(MediaTime::ZERO, &[audio()], constrained).unwrap(),
            )
            .unwrap();
    }

    #[test]
    fn playback_selection_requires_exact_owned_unique_streams() {
        let mut registry = MediaRegistry::try_new(limits()).unwrap();
        let first = registry
            .create_resource(descriptor(&[audio(), video()]))
            .unwrap();
        let second = registry.create_resource(descriptor(&[audio()])).unwrap();
        let first_streams = registry.resource(first).unwrap().streams().to_vec();
        let second_stream = registry.resource(second).unwrap().streams()[0];

        assert_eq!(
            registry.create_playback(first, &[]).unwrap_err(),
            MediaError::NoSelectedStreams
        );
        assert_eq!(
            registry
                .create_playback(first, &[first_streams[0], first_streams[0]])
                .unwrap_err(),
            MediaError::DuplicateStream(first_streams[0])
        );
        assert_eq!(
            registry
                .create_playback(first, &[second_stream])
                .unwrap_err(),
            MediaError::StreamNotOwnedByResource {
                stream: second_stream,
                resource: first,
            }
        );
        registry.create_playback(first, &first_streams).unwrap();
    }

    #[test]
    fn playback_lifecycle_is_explicit_and_terminal() {
        let mut registry = MediaRegistry::try_new(limits()).unwrap();
        let resource = registry.create_resource(descriptor(&[audio()])).unwrap();
        let stream = registry.resource(resource).unwrap().streams()[0];
        let playback = registry.create_playback(resource, &[stream]).unwrap();

        assert_eq!(
            registry.playback(playback).unwrap().state(),
            MediaPlaybackState::Ready
        );
        assert!(matches!(
            registry.pause(playback),
            Err(MediaError::InvalidPlaybackTransition { .. })
        ));
        registry.play(playback).unwrap();
        registry
            .set_playback_position(playback, MediaTime::from_micros(1_000_000))
            .unwrap();
        registry.pause(playback).unwrap();
        registry.play(playback).unwrap();
        registry.mark_ended(playback).unwrap();
        let ended = registry.playback(playback).unwrap();
        assert_eq!(ended.state(), MediaPlaybackState::Ended);
        assert_eq!(ended.position(), MediaTime::from_micros(5_000_000));
        assert!(matches!(
            registry.play(playback),
            Err(MediaError::InvalidPlaybackTransition { .. })
        ));
        assert!(matches!(
            registry.set_playback_position(playback, MediaTime::ZERO),
            Err(MediaError::InvalidPlaybackTransition { .. })
        ));
    }

    #[test]
    fn playback_position_is_bounded_by_exact_resource_duration() {
        let mut registry = MediaRegistry::try_new(limits()).unwrap();
        let resource = registry.create_resource(descriptor(&[video()])).unwrap();
        let stream = registry.resource(resource).unwrap().streams()[0];
        let playback = registry.create_playback(resource, &[stream]).unwrap();
        assert_eq!(
            registry
                .set_playback_position(playback, MediaTime::from_micros(5_000_001))
                .unwrap_err(),
            MediaError::PositionOutOfRange {
                position: MediaTime::from_micros(5_000_001),
                duration: MediaTime::from_micros(5_000_000),
            }
        );
        assert_eq!(
            registry.playback(playback).unwrap().position(),
            MediaTime::ZERO
        );
    }

    #[test]
    fn live_playback_blocks_resource_retirement_and_capacity_recovers() {
        let mut constrained = limits();
        constrained.max_playbacks = 1;
        let mut registry = MediaRegistry::try_new(constrained).unwrap();
        let resource = registry
            .create_resource(
                MediaResourceDescriptor::try_new(MediaTime::ZERO, &[audio()], constrained).unwrap(),
            )
            .unwrap();
        let stream = registry.resource(resource).unwrap().streams()[0];
        let playback = registry.create_playback(resource, &[stream]).unwrap();
        assert_eq!(
            registry.retire_resource(resource).unwrap_err(),
            MediaError::ResourceInUse(resource)
        );
        assert!(matches!(
            registry.create_playback(resource, &[stream]),
            Err(MediaError::PlaybackLimitExceeded { .. })
        ));

        registry.retire_playback(playback).unwrap();
        assert_eq!(registry.snapshot().playbacks(), 0);
        registry.create_playback(resource, &[stream]).unwrap();
    }

    #[test]
    fn retired_playback_identity_never_revives() {
        let mut registry = MediaRegistry::try_new(limits()).unwrap();
        let resource = registry.create_resource(descriptor(&[audio()])).unwrap();
        let stream = registry.resource(resource).unwrap().streams()[0];
        let first = registry.create_playback(resource, &[stream]).unwrap();
        registry.retire_playback(first).unwrap();
        let second = registry.create_playback(resource, &[stream]).unwrap();
        assert!(second.serial() > first.serial());
        assert_eq!(
            registry.playback(first).unwrap_err(),
            MediaError::UnknownPlayback(first)
        );
    }
}
