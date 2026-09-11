use rarog_media::{
    MediaPlayback, MediaPlaybackState, MediaResource, MediaStream, MediaStreamKind, MediaTime,
};
use std::fmt;
use std::num::NonZeroU64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaDemuxTicket(NonZeroU64);

impl MediaDemuxTicket {
    pub const fn new(value: NonZeroU64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaDecoderTicket(NonZeroU64);

impl MediaDecoderTicket {
    pub const fn new(value: NonZeroU64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaOutputTicket(NonZeroU64);

impl MediaOutputTicket {
    pub const fn new(value: NonZeroU64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaAdapterErrorKind {
    InvalidTicket,
    Unsupported,
    InvalidState,
    Backend,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaAdapterError {
    kind: MediaAdapterErrorKind,
}

impl MediaAdapterError {
    pub const fn new(kind: MediaAdapterErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> MediaAdapterErrorKind {
        self.kind
    }
}

impl fmt::Display for MediaAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.kind {
            MediaAdapterErrorKind::InvalidTicket => "media adapter ticket is invalid or stale",
            MediaAdapterErrorKind::Unsupported => "media adapter operation is unsupported",
            MediaAdapterErrorKind::InvalidState => {
                "media adapter operation is invalid in the current state"
            }
            MediaAdapterErrorKind::Backend => "media adapter backend operation failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for MediaAdapterError {}

pub trait MediaDemuxer {
    fn open(&mut self, resource: &MediaResource) -> Result<MediaDemuxTicket, MediaAdapterError>;

    fn select_stream(
        &mut self,
        ticket: MediaDemuxTicket,
        stream: &MediaStream,
    ) -> Result<(), MediaAdapterError>;

    fn seek(
        &mut self,
        ticket: MediaDemuxTicket,
        position: MediaTime,
    ) -> Result<(), MediaAdapterError>;

    fn close(&mut self, ticket: MediaDemuxTicket) -> Result<(), MediaAdapterError>;
}

pub trait MediaDecoder {
    fn open(&mut self, stream: &MediaStream) -> Result<MediaDecoderTicket, MediaAdapterError>;

    fn flush(&mut self, ticket: MediaDecoderTicket) -> Result<(), MediaAdapterError>;

    fn reset(
        &mut self,
        ticket: MediaDecoderTicket,
        position: MediaTime,
    ) -> Result<(), MediaAdapterError>;

    fn close(&mut self, ticket: MediaDecoderTicket) -> Result<(), MediaAdapterError>;
}

pub trait MediaOutput {
    fn open(
        &mut self,
        playback: &MediaPlayback,
        kind: MediaStreamKind,
    ) -> Result<MediaOutputTicket, MediaAdapterError>;

    fn apply_state(
        &mut self,
        ticket: MediaOutputTicket,
        state: MediaPlaybackState,
        position: MediaTime,
    ) -> Result<(), MediaAdapterError>;

    fn close(&mut self, ticket: MediaOutputTicket) -> Result<(), MediaAdapterError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_media::{
        MediaLimits, MediaPlaybackId, MediaRegistry, MediaResourceDescriptor, MediaResourceId,
        MediaStreamDescriptor, MediaStreamId,
    };

    #[derive(Debug, Default)]
    struct RecordingDemuxer {
        opened: Vec<MediaResourceId>,
        selected: Vec<MediaStreamId>,
        seeks: Vec<MediaTime>,
        closed: Vec<MediaDemuxTicket>,
        ticket: u64,
    }

    impl RecordingDemuxer {
        fn with_ticket(ticket: u64) -> Self {
            Self {
                ticket,
                ..Self::default()
            }
        }
    }

    impl MediaDemuxer for RecordingDemuxer {
        fn open(
            &mut self,
            resource: &MediaResource,
        ) -> Result<MediaDemuxTicket, MediaAdapterError> {
            self.opened.push(resource.id());
            Ok(MediaDemuxTicket::new(NonZeroU64::new(self.ticket).unwrap()))
        }

        fn select_stream(
            &mut self,
            _ticket: MediaDemuxTicket,
            stream: &MediaStream,
        ) -> Result<(), MediaAdapterError> {
            self.selected.push(stream.id());
            Ok(())
        }

        fn seek(
            &mut self,
            _ticket: MediaDemuxTicket,
            position: MediaTime,
        ) -> Result<(), MediaAdapterError> {
            self.seeks.push(position);
            Ok(())
        }

        fn close(&mut self, ticket: MediaDemuxTicket) -> Result<(), MediaAdapterError> {
            self.closed.push(ticket);
            Ok(())
        }
    }

    #[derive(Debug, Default)]
    struct RecordingDecoder {
        opened: Vec<MediaStreamId>,
        flushes: Vec<MediaDecoderTicket>,
        resets: Vec<MediaTime>,
        closes: Vec<MediaDecoderTicket>,
    }

    impl MediaDecoder for RecordingDecoder {
        fn open(&mut self, stream: &MediaStream) -> Result<MediaDecoderTicket, MediaAdapterError> {
            self.opened.push(stream.id());
            Ok(MediaDecoderTicket::new(NonZeroU64::new(17).unwrap()))
        }

        fn flush(&mut self, ticket: MediaDecoderTicket) -> Result<(), MediaAdapterError> {
            self.flushes.push(ticket);
            Ok(())
        }

        fn reset(
            &mut self,
            _ticket: MediaDecoderTicket,
            position: MediaTime,
        ) -> Result<(), MediaAdapterError> {
            self.resets.push(position);
            Ok(())
        }

        fn close(&mut self, ticket: MediaDecoderTicket) -> Result<(), MediaAdapterError> {
            self.closes.push(ticket);
            Ok(())
        }
    }

    #[derive(Debug, Default)]
    struct RecordingOutput {
        opened: Vec<(MediaPlaybackId, MediaStreamKind)>,
        states: Vec<(MediaPlaybackState, MediaTime)>,
        closes: Vec<MediaOutputTicket>,
    }

    impl MediaOutput for RecordingOutput {
        fn open(
            &mut self,
            playback: &MediaPlayback,
            kind: MediaStreamKind,
        ) -> Result<MediaOutputTicket, MediaAdapterError> {
            self.opened.push((playback.id(), kind));
            Ok(MediaOutputTicket::new(NonZeroU64::new(23).unwrap()))
        }

        fn apply_state(
            &mut self,
            _ticket: MediaOutputTicket,
            state: MediaPlaybackState,
            position: MediaTime,
        ) -> Result<(), MediaAdapterError> {
            self.states.push((state, position));
            Ok(())
        }

        fn close(&mut self, ticket: MediaOutputTicket) -> Result<(), MediaAdapterError> {
            self.closes.push(ticket);
            Ok(())
        }
    }

    fn registry_fixture() -> (MediaRegistry, MediaResourceId, MediaPlaybackId) {
        let limits = MediaLimits::default();
        let descriptor = MediaResourceDescriptor::try_new(
            MediaTime::from_micros(10_000_000),
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
        .unwrap();
        let mut registry = MediaRegistry::try_new(limits).unwrap();
        let resource = registry.create_resource(descriptor).unwrap();
        let selected = registry.resource(resource).unwrap().streams().to_vec();
        let playback = registry.create_playback(resource, &selected).unwrap();
        (registry, resource, playback)
    }

    fn drive_demuxer<T: MediaDemuxer>(
        demuxer: &mut T,
        resource: &MediaResource,
        stream: &MediaStream,
    ) -> MediaDemuxTicket {
        let ticket = demuxer.open(resource).unwrap();
        demuxer.select_stream(ticket, stream).unwrap();
        demuxer
            .seek(ticket, MediaTime::from_micros(2_000_000))
            .unwrap();
        demuxer.close(ticket).unwrap();
        ticket
    }

    #[test]
    fn ticket_types_are_distinct_opaque_correlation_values() {
        let raw = NonZeroU64::new(7).unwrap();
        assert_eq!(MediaDemuxTicket::new(raw).get(), 7);
        assert_eq!(MediaDecoderTicket::new(raw).get(), 7);
        assert_eq!(MediaOutputTicket::new(raw).get(), 7);
    }

    #[test]
    fn adapter_errors_retain_only_fixed_classification() {
        let error = MediaAdapterError::new(MediaAdapterErrorKind::Backend);
        assert_eq!(error.kind(), MediaAdapterErrorKind::Backend);
        assert_eq!(error.to_string(), "media adapter backend operation failed");
    }

    #[test]
    fn demuxer_is_replaceable_and_preserves_exact_semantic_references() {
        let (registry, resource_id, _) = registry_fixture();
        let resource = registry.resource(resource_id).unwrap();
        let stream_id = resource.streams()[0];
        let stream = registry.stream(stream_id).unwrap();

        let mut first = RecordingDemuxer::with_ticket(11);
        let first_ticket = drive_demuxer(&mut first, resource, stream);
        let mut second = RecordingDemuxer::with_ticket(12);
        let second_ticket = drive_demuxer(&mut second, resource, stream);

        assert_eq!(first.opened, vec![resource_id]);
        assert_eq!(first.selected, vec![stream_id]);
        assert_eq!(first.seeks, vec![MediaTime::from_micros(2_000_000)]);
        assert_eq!(first.closed, vec![first_ticket]);
        assert_eq!(second.opened, vec![resource_id]);
        assert_ne!(first_ticket, second_ticket);
    }

    #[test]
    fn decoder_contract_uses_only_stream_identity_and_portable_time() {
        let (registry, resource_id, _) = registry_fixture();
        let stream = registry
            .stream(registry.resource(resource_id).unwrap().streams()[0])
            .unwrap();
        let mut decoder = RecordingDecoder::default();
        let ticket = decoder.open(stream).unwrap();
        decoder.flush(ticket).unwrap();
        decoder
            .reset(ticket, MediaTime::from_micros(4_000_000))
            .unwrap();
        decoder.close(ticket).unwrap();

        assert_eq!(decoder.opened, vec![stream.id()]);
        assert_eq!(decoder.flushes, vec![ticket]);
        assert_eq!(decoder.resets, vec![MediaTime::from_micros(4_000_000)]);
        assert_eq!(decoder.closes, vec![ticket]);
    }

    #[test]
    fn output_contract_preserves_exact_playback_state_kind_and_position() {
        let (mut registry, _, playback_id) = registry_fixture();
        registry.play(playback_id).unwrap();
        registry
            .set_playback_position(playback_id, MediaTime::from_micros(750_000))
            .unwrap();
        let playback = registry.playback(playback_id).unwrap().clone();
        let mut output = RecordingOutput::default();
        let ticket = output.open(&playback, MediaStreamKind::Audio).unwrap();
        output
            .apply_state(ticket, playback.state(), playback.position())
            .unwrap();
        output.close(ticket).unwrap();

        assert_eq!(output.opened, vec![(playback_id, MediaStreamKind::Audio)]);
        assert_eq!(
            output.states,
            vec![(MediaPlaybackState::Playing, MediaTime::from_micros(750_000))]
        );
        assert_eq!(output.closes, vec![ticket]);
    }
}
