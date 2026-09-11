use rarog_media::{
    MediaError, MediaLimits, MediaRegistry, MediaResourceDescriptor, MediaStreamDescriptor,
    MediaTime,
};

fn limits() -> MediaLimits {
    MediaLimits {
        max_resources: 2,
        max_streams: 4,
        max_streams_per_resource: 2,
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

fn descriptor(streams: &[MediaStreamDescriptor], limits: MediaLimits) -> MediaResourceDescriptor {
    MediaResourceDescriptor::try_new(MediaTime::from_micros(1_000_000), streams, limits).unwrap()
}

#[test]
fn resource_limit_rejection_does_not_mutate_registry() {
    let mut constrained = limits();
    constrained.max_resources = 1;
    let mut registry = MediaRegistry::try_new(constrained).unwrap();
    registry
        .create_resource(descriptor(&[audio()], constrained))
        .unwrap();
    let before = registry.snapshot();

    assert_eq!(
        registry
            .create_resource(descriptor(&[video()], constrained))
            .unwrap_err(),
        MediaError::ResourceLimitExceeded {
            resources: 2,
            limit: 1,
        }
    );
    assert_eq!(registry.snapshot(), before);
}

#[test]
fn streams_per_resource_limit_is_revalidated_before_registry_ownership() {
    let broad = limits();
    let mut constrained = broad;
    constrained.max_streams_per_resource = 1;
    let descriptor = descriptor(&[audio(), video()], broad);
    let mut registry = MediaRegistry::try_new(constrained).unwrap();
    let before = registry.snapshot();

    assert_eq!(
        registry.create_resource(descriptor).unwrap_err(),
        MediaError::StreamsPerResourceLimitExceeded {
            streams: 2,
            limit: 1,
        }
    );
    assert_eq!(registry.snapshot(), before);
}

#[test]
fn foreign_stream_identity_cannot_be_selected_for_local_playback() {
    let mut first = MediaRegistry::try_new(limits()).unwrap();
    let mut second = MediaRegistry::try_new(limits()).unwrap();
    let first_resource = first
        .create_resource(descriptor(&[audio()], limits()))
        .unwrap();
    let second_resource = second
        .create_resource(descriptor(&[audio()], limits()))
        .unwrap();
    let foreign_stream = second.resource(second_resource).unwrap().streams()[0];
    let before = first.snapshot();

    assert_eq!(
        first
            .create_playback(first_resource, &[foreign_stream])
            .unwrap_err(),
        MediaError::UnknownStream(foreign_stream)
    );
    assert_eq!(first.snapshot(), before);
}
