use rarog_media::{
    MediaPlayback, MediaPlaybackId, MediaPlaybackState, MediaResource, MediaResourceId,
    MediaStream, MediaStreamId, MediaStreamKind, MediaTime,
};
use rarog_media_adapter::{
    MediaAdapterError, MediaAdapterErrorKind, MediaDecoder, MediaDecoderTicket, MediaDemuxTicket,
    MediaDemuxer, MediaOutput, MediaOutputTicket,
};
use std::collections::BTreeMap;
use std::fmt;
use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_MAX_WINDOWS_MEDIA_DEMUX_SESSIONS: usize = 64;
pub const DEFAULT_MAX_WINDOWS_MEDIA_DECODER_SESSIONS: usize = 64;
pub const DEFAULT_MAX_WINDOWS_MEDIA_OUTPUT_SESSIONS: usize = 64;

static NEXT_WINDOWS_MEDIA_TICKET: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowsMediaLimits {
    max_demux_sessions: NonZeroUsize,
    max_decoder_sessions: NonZeroUsize,
    max_output_sessions: NonZeroUsize,
}

impl WindowsMediaLimits {
    pub fn try_new(
        max_demux_sessions: usize,
        max_decoder_sessions: usize,
        max_output_sessions: usize,
    ) -> Result<Self, WindowsMediaBackendError> {
        Ok(Self {
            max_demux_sessions: NonZeroUsize::new(max_demux_sessions)
                .ok_or(WindowsMediaBackendError::InvalidLimits)?,
            max_decoder_sessions: NonZeroUsize::new(max_decoder_sessions)
                .ok_or(WindowsMediaBackendError::InvalidLimits)?,
            max_output_sessions: NonZeroUsize::new(max_output_sessions)
                .ok_or(WindowsMediaBackendError::InvalidLimits)?,
        })
    }

    pub const fn max_demux_sessions(self) -> usize {
        self.max_demux_sessions.get()
    }

    pub const fn max_decoder_sessions(self) -> usize {
        self.max_decoder_sessions.get()
    }

    pub const fn max_output_sessions(self) -> usize {
        self.max_output_sessions.get()
    }
}

impl Default for WindowsMediaLimits {
    fn default() -> Self {
        Self {
            max_demux_sessions: NonZeroUsize::new(DEFAULT_MAX_WINDOWS_MEDIA_DEMUX_SESSIONS)
                .expect("non-zero Windows media demux limit"),
            max_decoder_sessions: NonZeroUsize::new(DEFAULT_MAX_WINDOWS_MEDIA_DECODER_SESSIONS)
                .expect("non-zero Windows media decoder limit"),
            max_output_sessions: NonZeroUsize::new(DEFAULT_MAX_WINDOWS_MEDIA_OUTPUT_SESSIONS)
                .expect("non-zero Windows media output limit"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsMediaBackendError {
    UnsupportedTarget,
    InvalidLimits,
}

impl fmt::Display for WindowsMediaBackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTarget => {
                formatter.write_str("Windows media backend is unavailable on this target")
            }
            Self::InvalidLimits => formatter.write_str("Windows media limits must be non-zero"),
        }
    }
}

impl std::error::Error for WindowsMediaBackendError {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowsMediaBackendSnapshot {
    demux_sessions: usize,
    decoder_sessions: usize,
    output_sessions: usize,
}

impl WindowsMediaBackendSnapshot {
    pub const fn demux_sessions(self) -> usize {
        self.demux_sessions
    }

    pub const fn decoder_sessions(self) -> usize {
        self.decoder_sessions
    }

    pub const fn output_sessions(self) -> usize {
        self.output_sessions
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DemuxSession {
    resource: MediaResourceId,
    duration: MediaTime,
    selected_stream: Option<MediaStreamId>,
    position: MediaTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DecoderSession {
    stream: MediaStreamId,
    position: MediaTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OutputSession {
    playback: MediaPlaybackId,
    kind: MediaStreamKind,
    state: MediaPlaybackState,
    position: MediaTime,
}

#[derive(Debug)]
pub struct WindowsMediaBackend {
    limits: WindowsMediaLimits,
    demux_sessions: BTreeMap<MediaDemuxTicket, DemuxSession>,
    decoder_sessions: BTreeMap<MediaDecoderTicket, DecoderSession>,
    output_sessions: BTreeMap<MediaOutputTicket, OutputSession>,
}

impl WindowsMediaBackend {
    pub fn try_new(limits: WindowsMediaLimits) -> Result<Self, WindowsMediaBackendError> {
        if !Self::target_available() {
            return Err(WindowsMediaBackendError::UnsupportedTarget);
        }
        Ok(Self::new_unchecked(limits))
    }

    pub fn with_default_limits() -> Result<Self, WindowsMediaBackendError> {
        Self::try_new(WindowsMediaLimits::default())
    }

    pub const fn target_available() -> bool {
        cfg!(target_os = "windows")
    }

    pub const fn limits(&self) -> WindowsMediaLimits {
        self.limits
    }

    pub fn snapshot(&self) -> WindowsMediaBackendSnapshot {
        WindowsMediaBackendSnapshot {
            demux_sessions: self.demux_sessions.len(),
            decoder_sessions: self.decoder_sessions.len(),
            output_sessions: self.output_sessions.len(),
        }
    }

    fn new_unchecked(limits: WindowsMediaLimits) -> Self {
        Self {
            limits,
            demux_sessions: BTreeMap::new(),
            decoder_sessions: BTreeMap::new(),
            output_sessions: BTreeMap::new(),
        }
    }

    fn ensure_demux_capacity(&self) -> Result<(), MediaAdapterError> {
        if self.demux_sessions.len() >= self.limits.max_demux_sessions() {
            Err(adapter_error(MediaAdapterErrorKind::Backend))
        } else {
            Ok(())
        }
    }

    fn ensure_decoder_capacity(&self) -> Result<(), MediaAdapterError> {
        if self.decoder_sessions.len() >= self.limits.max_decoder_sessions() {
            Err(adapter_error(MediaAdapterErrorKind::Backend))
        } else {
            Ok(())
        }
    }

    fn ensure_output_capacity(&self) -> Result<(), MediaAdapterError> {
        if self.output_sessions.len() >= self.limits.max_output_sessions() {
            Err(adapter_error(MediaAdapterErrorKind::Backend))
        } else {
            Ok(())
        }
    }
}

impl MediaDemuxer for WindowsMediaBackend {
    fn open(&mut self, resource: &MediaResource) -> Result<MediaDemuxTicket, MediaAdapterError> {
        self.ensure_demux_capacity()?;
        let ticket = MediaDemuxTicket::new(allocate_ticket()?);
        if self.demux_sessions.contains_key(&ticket) {
            return Err(adapter_error(MediaAdapterErrorKind::Backend));
        }
        self.demux_sessions.insert(
            ticket,
            DemuxSession {
                resource: resource.id(),
                duration: resource.duration(),
                selected_stream: None,
                position: MediaTime::ZERO,
            },
        );
        Ok(ticket)
    }

    fn select_stream(
        &mut self,
        ticket: MediaDemuxTicket,
        stream: &MediaStream,
    ) -> Result<(), MediaAdapterError> {
        let session = self
            .demux_sessions
            .get_mut(&ticket)
            .ok_or_else(invalid_ticket)?;
        if stream.resource() != session.resource {
            return Err(adapter_error(MediaAdapterErrorKind::InvalidState));
        }
        session.selected_stream = Some(stream.id());
        Ok(())
    }

    fn seek(
        &mut self,
        ticket: MediaDemuxTicket,
        position: MediaTime,
    ) -> Result<(), MediaAdapterError> {
        let session = self
            .demux_sessions
            .get_mut(&ticket)
            .ok_or_else(invalid_ticket)?;
        if position > session.duration {
            return Err(adapter_error(MediaAdapterErrorKind::InvalidState));
        }
        session.position = position;
        Ok(())
    }

    fn close(&mut self, ticket: MediaDemuxTicket) -> Result<(), MediaAdapterError> {
        self.demux_sessions
            .remove(&ticket)
            .ok_or_else(invalid_ticket)?;
        Ok(())
    }
}

impl MediaDecoder for WindowsMediaBackend {
    fn open(&mut self, stream: &MediaStream) -> Result<MediaDecoderTicket, MediaAdapterError> {
        self.ensure_decoder_capacity()?;
        let ticket = MediaDecoderTicket::new(allocate_ticket()?);
        if self.decoder_sessions.contains_key(&ticket) {
            return Err(adapter_error(MediaAdapterErrorKind::Backend));
        }
        self.decoder_sessions.insert(
            ticket,
            DecoderSession {
                stream: stream.id(),
                position: MediaTime::ZERO,
            },
        );
        Ok(ticket)
    }

    fn flush(&mut self, ticket: MediaDecoderTicket) -> Result<(), MediaAdapterError> {
        self.decoder_sessions
            .get(&ticket)
            .ok_or_else(invalid_ticket)?;
        Ok(())
    }

    fn reset(
        &mut self,
        ticket: MediaDecoderTicket,
        position: MediaTime,
    ) -> Result<(), MediaAdapterError> {
        let session = self
            .decoder_sessions
            .get_mut(&ticket)
            .ok_or_else(invalid_ticket)?;
        session.position = position;
        Ok(())
    }

    fn close(&mut self, ticket: MediaDecoderTicket) -> Result<(), MediaAdapterError> {
        self.decoder_sessions
            .remove(&ticket)
            .ok_or_else(invalid_ticket)?;
        Ok(())
    }
}

impl MediaOutput for WindowsMediaBackend {
    fn open(
        &mut self,
        playback: &MediaPlayback,
        kind: MediaStreamKind,
    ) -> Result<MediaOutputTicket, MediaAdapterError> {
        self.ensure_output_capacity()?;
        let ticket = MediaOutputTicket::new(allocate_ticket()?);
        if self.output_sessions.contains_key(&ticket) {
            return Err(adapter_error(MediaAdapterErrorKind::Backend));
        }
        self.output_sessions.insert(
            ticket,
            OutputSession {
                playback: playback.id(),
                kind,
                state: playback.state(),
                position: playback.position(),
            },
        );
        Ok(ticket)
    }

    fn apply_state(
        &mut self,
        ticket: MediaOutputTicket,
        state: MediaPlaybackState,
        position: MediaTime,
    ) -> Result<(), MediaAdapterError> {
        let session = self
            .output_sessions
            .get_mut(&ticket)
            .ok_or_else(invalid_ticket)?;
        if session.state == MediaPlaybackState::Ended && state != MediaPlaybackState::Ended {
            return Err(adapter_error(MediaAdapterErrorKind::InvalidState));
        }
        session.state = state;
        session.position = position;
        Ok(())
    }

    fn close(&mut self, ticket: MediaOutputTicket) -> Result<(), MediaAdapterError> {
        self.output_sessions
            .remove(&ticket)
            .ok_or_else(invalid_ticket)?;
        Ok(())
    }
}

fn allocate_ticket() -> Result<NonZeroU64, MediaAdapterError> {
    let raw = NEXT_WINDOWS_MEDIA_TICKET
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| adapter_error(MediaAdapterErrorKind::Backend))?;
    NonZeroU64::new(raw).ok_or_else(|| adapter_error(MediaAdapterErrorKind::Backend))
}

fn adapter_error(kind: MediaAdapterErrorKind) -> MediaAdapterError {
    MediaAdapterError::new(kind)
}

fn invalid_ticket() -> MediaAdapterError {
    adapter_error(MediaAdapterErrorKind::InvalidTicket)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_media::{MediaLimits, MediaRegistry, MediaResourceDescriptor, MediaStreamDescriptor};

    fn limits(demux: usize, decoder: usize, output: usize) -> WindowsMediaLimits {
        WindowsMediaLimits::try_new(demux, decoder, output).unwrap()
    }

    fn backend(demux: usize, decoder: usize, output: usize) -> WindowsMediaBackend {
        WindowsMediaBackend::new_unchecked(limits(demux, decoder, output))
    }

    fn registry_fixture() -> (
        MediaRegistry,
        MediaResourceId,
        MediaResourceId,
        MediaPlaybackId,
    ) {
        let limits = MediaLimits::default();
        let mut registry = MediaRegistry::try_new(limits).unwrap();
        let first = registry
            .create_resource(
                MediaResourceDescriptor::try_new(
                    MediaTime::from_micros(5_000_000),
                    &[
                        MediaStreamDescriptor::Audio {
                            channels: 2,
                            sample_rate_hz: 48_000,
                        },
                        MediaStreamDescriptor::Video {
                            width: 1_920,
                            height: 1_080,
                        },
                    ],
                    limits,
                )
                .unwrap(),
            )
            .unwrap();
        let second = registry
            .create_resource(
                MediaResourceDescriptor::try_new(
                    MediaTime::from_micros(2_000_000),
                    &[MediaStreamDescriptor::Audio {
                        channels: 1,
                        sample_rate_hz: 44_100,
                    }],
                    limits,
                )
                .unwrap(),
            )
            .unwrap();
        let selected = registry.resource(first).unwrap().streams().to_vec();
        let playback = registry.create_playback(first, &selected).unwrap();
        registry.play(playback).unwrap();
        (registry, first, second, playback)
    }

    #[test]
    fn limits_must_be_non_zero() {
        assert_eq!(
            WindowsMediaLimits::try_new(0, 1, 1).unwrap_err(),
            WindowsMediaBackendError::InvalidLimits
        );
        assert_eq!(
            WindowsMediaLimits::try_new(1, 0, 1).unwrap_err(),
            WindowsMediaBackendError::InvalidLimits
        );
        assert_eq!(
            WindowsMediaLimits::try_new(1, 1, 0).unwrap_err(),
            WindowsMediaBackendError::InvalidLimits
        );
    }

    #[test]
    fn public_construction_matches_target() {
        let result = WindowsMediaBackend::try_new(WindowsMediaLimits::default());
        if cfg!(target_os = "windows") {
            assert!(result.is_ok());
        } else {
            assert!(matches!(
                result,
                Err(WindowsMediaBackendError::UnsupportedTarget)
            ));
        }
    }

    #[test]
    fn demux_sessions_are_bounded_and_recover_capacity_without_ticket_reuse() {
        let (registry, first_resource, _, _) = registry_fixture();
        let resource = registry.resource(first_resource).unwrap();
        let mut backend = backend(1, 1, 1);
        let first = MediaDemuxer::open(&mut backend, resource).unwrap();
        assert_eq!(backend.snapshot().demux_sessions(), 1);
        assert_eq!(
            MediaDemuxer::open(&mut backend, resource)
                .unwrap_err()
                .kind(),
            MediaAdapterErrorKind::Backend
        );
        MediaDemuxer::close(&mut backend, first).unwrap();
        let second = MediaDemuxer::open(&mut backend, resource).unwrap();
        assert_ne!(first, second);
        assert_eq!(backend.snapshot().demux_sessions(), 1);
    }

    #[test]
    fn demux_session_rejects_foreign_resource_stream_and_out_of_range_seek() {
        let (registry, first_resource, second_resource, _) = registry_fixture();
        let mut backend = backend(2, 1, 1);
        let ticket =
            MediaDemuxer::open(&mut backend, registry.resource(first_resource).unwrap()).unwrap();
        let foreign_stream = registry
            .stream(registry.resource(second_resource).unwrap().streams()[0])
            .unwrap();
        assert_eq!(
            MediaDemuxer::select_stream(&mut backend, ticket, foreign_stream)
                .unwrap_err()
                .kind(),
            MediaAdapterErrorKind::InvalidState
        );
        assert_eq!(
            MediaDemuxer::seek(&mut backend, ticket, MediaTime::from_micros(5_000_001))
                .unwrap_err()
                .kind(),
            MediaAdapterErrorKind::InvalidState
        );
    }

    #[test]
    fn closed_and_cross_backend_demux_tickets_fail_closed() {
        let (registry, resource_id, _, _) = registry_fixture();
        let resource = registry.resource(resource_id).unwrap();
        let mut first = backend(2, 1, 1);
        let mut second = backend(2, 1, 1);
        let ticket = MediaDemuxer::open(&mut first, resource).unwrap();
        assert_eq!(
            MediaDemuxer::seek(&mut second, ticket, MediaTime::ZERO)
                .unwrap_err()
                .kind(),
            MediaAdapterErrorKind::InvalidTicket
        );
        MediaDemuxer::close(&mut first, ticket).unwrap();
        assert_eq!(
            MediaDemuxer::close(&mut first, ticket).unwrap_err().kind(),
            MediaAdapterErrorKind::InvalidTicket
        );
    }

    #[test]
    fn decoder_sessions_bind_exact_stream_and_recover_capacity() {
        let (registry, resource_id, _, _) = registry_fixture();
        let stream = registry
            .stream(registry.resource(resource_id).unwrap().streams()[0])
            .unwrap();
        let mut backend = backend(1, 1, 1);
        let first = MediaDecoder::open(&mut backend, stream).unwrap();
        assert_eq!(backend.snapshot().decoder_sessions(), 1);
        assert_eq!(
            MediaDecoder::open(&mut backend, stream).unwrap_err().kind(),
            MediaAdapterErrorKind::Backend
        );
        MediaDecoder::flush(&mut backend, first).unwrap();
        MediaDecoder::reset(&mut backend, first, MediaTime::from_micros(250_000)).unwrap();
        MediaDecoder::close(&mut backend, first).unwrap();
        let second = MediaDecoder::open(&mut backend, stream).unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn decoder_ticket_from_another_backend_is_rejected() {
        let (registry, resource_id, _, _) = registry_fixture();
        let stream = registry
            .stream(registry.resource(resource_id).unwrap().streams()[0])
            .unwrap();
        let mut first = backend(1, 1, 1);
        let mut second = backend(1, 1, 1);
        let ticket = MediaDecoder::open(&mut first, stream).unwrap();
        assert_eq!(
            MediaDecoder::flush(&mut second, ticket).unwrap_err().kind(),
            MediaAdapterErrorKind::InvalidTicket
        );
    }

    #[test]
    fn output_sessions_bind_playback_and_kind_and_ended_is_terminal() {
        let (registry, _, _, playback_id) = registry_fixture();
        let playback = registry.playback(playback_id).unwrap();
        let mut backend = backend(1, 1, 1);
        let ticket = MediaOutput::open(&mut backend, playback, MediaStreamKind::Audio).unwrap();
        assert_eq!(backend.snapshot().output_sessions(), 1);
        MediaOutput::apply_state(
            &mut backend,
            ticket,
            MediaPlaybackState::Ended,
            MediaTime::from_micros(5_000_000),
        )
        .unwrap();
        assert_eq!(
            MediaOutput::apply_state(
                &mut backend,
                ticket,
                MediaPlaybackState::Playing,
                MediaTime::ZERO,
            )
            .unwrap_err()
            .kind(),
            MediaAdapterErrorKind::InvalidState
        );
        MediaOutput::close(&mut backend, ticket).unwrap();
        assert_eq!(backend.snapshot().output_sessions(), 0);
        assert_eq!(
            MediaOutput::close(&mut backend, ticket).unwrap_err().kind(),
            MediaAdapterErrorKind::InvalidTicket
        );
    }

    #[test]
    fn output_capacity_and_cross_backend_ticket_checks_are_exact() {
        let (registry, _, _, playback_id) = registry_fixture();
        let playback = registry.playback(playback_id).unwrap();
        let mut first = backend(1, 1, 1);
        let mut second = backend(1, 1, 1);
        let ticket = MediaOutput::open(&mut first, playback, MediaStreamKind::Audio).unwrap();
        assert_eq!(
            MediaOutput::open(&mut first, playback, MediaStreamKind::Video)
                .unwrap_err()
                .kind(),
            MediaAdapterErrorKind::Backend
        );
        assert_eq!(
            MediaOutput::apply_state(
                &mut second,
                ticket,
                MediaPlaybackState::Playing,
                MediaTime::ZERO,
            )
            .unwrap_err()
            .kind(),
            MediaAdapterErrorKind::InvalidTicket
        );
    }

    #[test]
    fn ticket_values_are_process_global_across_adapter_kinds() {
        let (registry, resource_id, _, playback_id) = registry_fixture();
        let resource = registry.resource(resource_id).unwrap();
        let stream = registry.stream(resource.streams()[0]).unwrap();
        let playback = registry.playback(playback_id).unwrap();
        let mut backend = backend(2, 2, 2);
        let demux = MediaDemuxer::open(&mut backend, resource).unwrap();
        let decoder = MediaDecoder::open(&mut backend, stream).unwrap();
        let output = MediaOutput::open(&mut backend, playback, MediaStreamKind::Audio).unwrap();
        assert_ne!(demux.get(), decoder.get());
        assert_ne!(demux.get(), output.get());
        assert_ne!(decoder.get(), output.get());
    }
}
