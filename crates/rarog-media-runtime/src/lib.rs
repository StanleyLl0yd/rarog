use rarog_media::{
    MediaError, MediaPlaybackId, MediaPlaybackState, MediaRegistry, MediaStreamId, MediaStreamKind,
};
use std::collections::VecDeque;
use std::fmt;
use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_MEDIA_RUNTIME_SCOPE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaRuntimeLimits {
    max_outstanding_work: NonZeroUsize,
}

impl MediaRuntimeLimits {
    pub fn try_new(max_outstanding_work: usize) -> Result<Self, MediaRuntimeError> {
        Ok(Self {
            max_outstanding_work: NonZeroUsize::new(max_outstanding_work)
                .ok_or(MediaRuntimeError::InvalidLimits)?,
        })
    }

    pub const fn max_outstanding_work(self) -> usize {
        self.max_outstanding_work.get()
    }
}

impl Default for MediaRuntimeLimits {
    fn default() -> Self {
        Self {
            max_outstanding_work: NonZeroUsize::new(256).expect("non-zero media runtime limit"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaEnvironment {
    Foreground,
    Background,
    Suspended,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaBackgroundPolicy {
    pub allow_audio: bool,
    pub allow_video: bool,
}

impl MediaBackgroundPolicy {
    pub const fn new(allow_audio: bool, allow_video: bool) -> Self {
        Self {
            allow_audio,
            allow_video,
        }
    }

    const fn permits(self, kind: MediaStreamKind) -> bool {
        match kind {
            MediaStreamKind::Audio => self.allow_audio,
            MediaStreamKind::Video => self.allow_video,
        }
    }
}

impl Default for MediaBackgroundPolicy {
    fn default() -> Self {
        Self::new(false, false)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaWorkId {
    scope: NonZeroU64,
    serial: NonZeroU64,
}

impl MediaWorkId {
    pub const fn scope(self) -> u64 {
        self.scope.get()
    }

    pub const fn serial(self) -> u64 {
        self.serial.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaWorkKind {
    Demux(MediaStreamId),
    Decode(MediaStreamId),
    Output(MediaStreamKind),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaWork {
    id: MediaWorkId,
    playback: MediaPlaybackId,
    kind: MediaWorkKind,
}

impl MediaWork {
    pub const fn id(self) -> MediaWorkId {
        self.id
    }

    pub const fn playback(self) -> MediaPlaybackId {
        self.playback
    }

    pub const fn kind(self) -> MediaWorkKind {
        self.kind
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MediaRuntimeError {
    InvalidLimits,
    IdentitySpaceExhausted,
    QueueFull {
        outstanding: usize,
        limit: usize,
    },
    DuplicateWork {
        playback: MediaPlaybackId,
        kind: MediaWorkKind,
    },
    PlaybackNotPlaying {
        playback: MediaPlaybackId,
        state: MediaPlaybackState,
    },
    StreamNotSelected {
        playback: MediaPlaybackId,
        stream: MediaStreamId,
    },
    StreamNotOwnedByPlaybackResource {
        playback: MediaPlaybackId,
        stream: MediaStreamId,
    },
    OutputKindNotSelected {
        playback: MediaPlaybackId,
        kind: MediaStreamKind,
    },
    WorkBlockedByEnvironment {
        environment: MediaEnvironment,
        kind: MediaStreamKind,
    },
    WorkInProgress(MediaWorkId),
    NoActiveWork,
    WrongCompletion {
        expected: MediaWorkId,
        actual: MediaWorkId,
    },
    Media(MediaError),
}

impl fmt::Display for MediaRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => formatter.write_str("media runtime limits must be non-zero"),
            Self::IdentitySpaceExhausted => {
                formatter.write_str("media runtime work identity space is exhausted")
            }
            Self::QueueFull { outstanding, limit } => write!(
                formatter,
                "media runtime would retain {outstanding} outstanding work items; limit is {limit}"
            ),
            Self::DuplicateWork { playback, kind } => write!(
                formatter,
                "media runtime already retains {kind:?} work for playback {}:{}",
                playback.scope(),
                playback.serial()
            ),
            Self::PlaybackNotPlaying { playback, state } => write!(
                formatter,
                "media playback {}:{} is {state:?}, not Playing",
                playback.scope(),
                playback.serial()
            ),
            Self::StreamNotSelected { playback, stream } => write!(
                formatter,
                "media stream {}:{} is not selected by playback {}:{}",
                stream.scope(),
                stream.serial(),
                playback.scope(),
                playback.serial()
            ),
            Self::StreamNotOwnedByPlaybackResource { playback, stream } => write!(
                formatter,
                "media stream {}:{} is not owned by playback {}:{} resource",
                stream.scope(),
                stream.serial(),
                playback.scope(),
                playback.serial()
            ),
            Self::OutputKindNotSelected { playback, kind } => write!(
                formatter,
                "media playback {}:{} has no selected {kind:?} stream",
                playback.scope(),
                playback.serial()
            ),
            Self::WorkBlockedByEnvironment { environment, kind } => write!(
                formatter,
                "{kind:?} media work is blocked in {environment:?} environment"
            ),
            Self::WorkInProgress(work) => write!(
                formatter,
                "media work {}:{} is still in progress",
                work.scope(),
                work.serial()
            ),
            Self::NoActiveWork => formatter.write_str("media runtime has no active work"),
            Self::WrongCompletion { expected, actual } => write!(
                formatter,
                "media runtime expected completion for {}:{}, got {}:{}",
                expected.scope(),
                expected.serial(),
                actual.scope(),
                actual.serial()
            ),
            Self::Media(error) => write!(formatter, "media ownership validation failed: {error}"),
        }
    }
}

impl std::error::Error for MediaRuntimeError {}

impl From<MediaError> for MediaRuntimeError {
    fn from(value: MediaError) -> Self {
        Self::Media(value)
    }
}

#[derive(Debug)]
pub struct MediaRuntime {
    limits: MediaRuntimeLimits,
    scope: NonZeroU64,
    next_serial: Option<NonZeroU64>,
    environment: MediaEnvironment,
    background_policy: MediaBackgroundPolicy,
    pending: VecDeque<MediaWork>,
    active: Option<MediaWork>,
}

impl MediaRuntime {
    pub fn try_new(
        limits: MediaRuntimeLimits,
        background_policy: MediaBackgroundPolicy,
    ) -> Result<Self, MediaRuntimeError> {
        let scope = NEXT_MEDIA_RUNTIME_SCOPE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| MediaRuntimeError::IdentitySpaceExhausted)?;
        let scope = NonZeroU64::new(scope).ok_or(MediaRuntimeError::IdentitySpaceExhausted)?;
        Ok(Self {
            limits,
            scope,
            next_serial: NonZeroU64::new(1),
            environment: MediaEnvironment::Foreground,
            background_policy,
            pending: VecDeque::new(),
            active: None,
        })
    }

    pub const fn environment(&self) -> MediaEnvironment {
        self.environment
    }

    pub const fn background_policy(&self) -> MediaBackgroundPolicy {
        self.background_policy
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn active_work(&self) -> Option<MediaWork> {
        self.active
    }

    pub fn outstanding_count(&self) -> usize {
        self.pending.len() + usize::from(self.active.is_some())
    }

    pub fn schedule(
        &mut self,
        registry: &MediaRegistry,
        playback: MediaPlaybackId,
        kind: MediaWorkKind,
    ) -> Result<MediaWorkId, MediaRuntimeError> {
        let stream_kind = validate_work(registry, playback, kind)?;
        self.ensure_environment_permits(stream_kind)?;
        if self
            .pending
            .iter()
            .chain(self.active.iter())
            .any(|work| work.playback == playback && work.kind == kind)
        {
            return Err(MediaRuntimeError::DuplicateWork { playback, kind });
        }
        let outstanding =
            self.outstanding_count()
                .checked_add(1)
                .ok_or(MediaRuntimeError::QueueFull {
                    outstanding: usize::MAX,
                    limit: self.limits.max_outstanding_work(),
                })?;
        if outstanding > self.limits.max_outstanding_work() {
            return Err(MediaRuntimeError::QueueFull {
                outstanding,
                limit: self.limits.max_outstanding_work(),
            });
        }
        let id = self.allocate_work_id()?;
        self.pending.push_back(MediaWork { id, playback, kind });
        Ok(id)
    }

    pub fn next_work(
        &mut self,
        registry: &MediaRegistry,
    ) -> Result<Option<MediaWork>, MediaRuntimeError> {
        if let Some(active) = self.active {
            return Err(MediaRuntimeError::WorkInProgress(active.id));
        }
        while let Some(work) = self.pending.pop_front() {
            let Ok(kind) = validate_work(registry, work.playback, work.kind) else {
                continue;
            };
            if !environment_permits(self.environment, self.background_policy, kind) {
                continue;
            }
            self.active = Some(work);
            return Ok(Some(work));
        }
        Ok(None)
    }

    pub fn complete(&mut self, work: MediaWorkId) -> Result<(), MediaRuntimeError> {
        let active = self.active.ok_or(MediaRuntimeError::NoActiveWork)?;
        if active.id != work {
            return Err(MediaRuntimeError::WrongCompletion {
                expected: active.id,
                actual: work,
            });
        }
        self.active = None;
        Ok(())
    }

    pub fn cancel_pending(&mut self, work: MediaWorkId) -> bool {
        let Some(position) = self.pending.iter().position(|queued| queued.id == work) else {
            return false;
        };
        self.pending.remove(position);
        true
    }

    pub fn reconcile(&mut self, registry: &MediaRegistry) -> usize {
        let environment = self.environment;
        let policy = self.background_policy;
        let before = self.pending.len();
        self.pending.retain(|work| {
            validate_work(registry, work.playback, work.kind)
                .map(|kind| environment_permits(environment, policy, kind))
                .unwrap_or(false)
        });
        before.saturating_sub(self.pending.len())
    }

    pub fn set_environment(
        &mut self,
        environment: MediaEnvironment,
        registry: &MediaRegistry,
    ) -> Result<usize, MediaRuntimeError> {
        if let Some(active) = self.active {
            return Err(MediaRuntimeError::WorkInProgress(active.id));
        }
        self.environment = environment;
        Ok(self.reconcile(registry))
    }

    pub fn set_background_policy(
        &mut self,
        policy: MediaBackgroundPolicy,
        registry: &MediaRegistry,
    ) -> Result<usize, MediaRuntimeError> {
        if let Some(active) = self.active {
            return Err(MediaRuntimeError::WorkInProgress(active.id));
        }
        self.background_policy = policy;
        Ok(self.reconcile(registry))
    }

    fn ensure_environment_permits(&self, kind: MediaStreamKind) -> Result<(), MediaRuntimeError> {
        if environment_permits(self.environment, self.background_policy, kind) {
            Ok(())
        } else {
            Err(MediaRuntimeError::WorkBlockedByEnvironment {
                environment: self.environment,
                kind,
            })
        }
    }

    fn allocate_work_id(&mut self) -> Result<MediaWorkId, MediaRuntimeError> {
        let serial = self
            .next_serial
            .ok_or(MediaRuntimeError::IdentitySpaceExhausted)?;
        self.next_serial = serial.get().checked_add(1).and_then(NonZeroU64::new);
        Ok(MediaWorkId {
            scope: self.scope,
            serial,
        })
    }
}

fn environment_permits(
    environment: MediaEnvironment,
    policy: MediaBackgroundPolicy,
    kind: MediaStreamKind,
) -> bool {
    match environment {
        MediaEnvironment::Foreground => true,
        MediaEnvironment::Background => policy.permits(kind),
        MediaEnvironment::Suspended => false,
    }
}

fn validate_work(
    registry: &MediaRegistry,
    playback_id: MediaPlaybackId,
    kind: MediaWorkKind,
) -> Result<MediaStreamKind, MediaRuntimeError> {
    let playback = registry.playback(playback_id)?;
    if playback.state() != MediaPlaybackState::Playing {
        return Err(MediaRuntimeError::PlaybackNotPlaying {
            playback: playback_id,
            state: playback.state(),
        });
    }
    match kind {
        MediaWorkKind::Demux(stream) | MediaWorkKind::Decode(stream) => {
            if !playback.streams().contains(&stream) {
                return Err(MediaRuntimeError::StreamNotSelected {
                    playback: playback_id,
                    stream,
                });
            }
            let retained = registry.stream(stream)?;
            if retained.resource() != playback.resource() {
                return Err(MediaRuntimeError::StreamNotOwnedByPlaybackResource {
                    playback: playback_id,
                    stream,
                });
            }
            Ok(retained.descriptor().kind())
        }
        MediaWorkKind::Output(kind) => {
            for stream in playback.streams() {
                let retained = registry.stream(*stream)?;
                if retained.resource() != playback.resource() {
                    return Err(MediaRuntimeError::StreamNotOwnedByPlaybackResource {
                        playback: playback_id,
                        stream: *stream,
                    });
                }
                if retained.descriptor().kind() == kind {
                    return Ok(kind);
                }
            }
            Err(MediaRuntimeError::OutputKindNotSelected {
                playback: playback_id,
                kind,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_media::{MediaLimits, MediaResourceDescriptor, MediaStreamDescriptor, MediaTime};

    fn fixture(
        selected_both: bool,
    ) -> (MediaRegistry, MediaPlaybackId, MediaStreamId, MediaStreamId) {
        let limits = MediaLimits::default();
        let mut registry = MediaRegistry::try_new(limits).unwrap();
        let resource = registry
            .create_resource(
                MediaResourceDescriptor::try_new(
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
                .unwrap(),
            )
            .unwrap();
        let streams = registry.resource(resource).unwrap().streams().to_vec();
        let selected = if selected_both {
            streams.clone()
        } else {
            vec![streams[0]]
        };
        let playback = registry.create_playback(resource, &selected).unwrap();
        registry.play(playback).unwrap();
        (registry, playback, streams[0], streams[1])
    }

    fn new_runtime(limit: usize, policy: MediaBackgroundPolicy) -> MediaRuntime {
        MediaRuntime::try_new(MediaRuntimeLimits::try_new(limit).unwrap(), policy).unwrap()
    }

    #[test]
    fn zero_work_limit_is_rejected() {
        assert_eq!(
            MediaRuntimeLimits::try_new(0).unwrap_err(),
            MediaRuntimeError::InvalidLimits
        );
    }

    #[test]
    fn independent_runtimes_never_alias_work_ids() {
        let (registry, playback, audio, _) = fixture(true);
        let mut first = new_runtime(2, MediaBackgroundPolicy::default());
        let mut second = new_runtime(2, MediaBackgroundPolicy::default());
        let first_id = first
            .schedule(&registry, playback, MediaWorkKind::Decode(audio))
            .unwrap();
        let second_id = second
            .schedule(&registry, playback, MediaWorkKind::Decode(audio))
            .unwrap();
        assert_ne!(first_id, second_id);
        assert!(!first.cancel_pending(second_id));
    }

    #[test]
    fn only_playing_playbacks_admit_new_work_and_reconcile_drops_stale_work() {
        let (mut registry, playback, audio, _) = fixture(true);
        let mut runtime = new_runtime(4, MediaBackgroundPolicy::default());
        runtime
            .schedule(&registry, playback, MediaWorkKind::Decode(audio))
            .unwrap();
        registry.pause(playback).unwrap();
        assert_eq!(runtime.reconcile(&registry), 1);
        assert_eq!(runtime.pending_count(), 0);
        assert!(matches!(
            runtime
                .schedule(&registry, playback, MediaWorkKind::Decode(audio))
                .unwrap_err(),
            MediaRuntimeError::PlaybackNotPlaying {
                state: MediaPlaybackState::Paused,
                ..
            }
        ));
    }

    #[test]
    fn background_policy_can_allow_audio_while_suppressing_video() {
        let (registry, playback, audio, video) = fixture(true);
        let mut runtime = new_runtime(4, MediaBackgroundPolicy::new(true, false));
        let audio_id = runtime
            .schedule(&registry, playback, MediaWorkKind::Decode(audio))
            .unwrap();
        runtime
            .schedule(&registry, playback, MediaWorkKind::Decode(video))
            .unwrap();
        assert_eq!(
            runtime
                .set_environment(MediaEnvironment::Background, &registry)
                .unwrap(),
            1
        );
        assert_eq!(runtime.pending_count(), 1);
        assert_eq!(
            runtime.next_work(&registry).unwrap().unwrap().id(),
            audio_id
        );
    }

    #[test]
    fn suspended_runtime_drops_pending_work_and_resume_requires_fresh_identity() {
        let (registry, playback, audio, _) = fixture(true);
        let mut runtime = new_runtime(4, MediaBackgroundPolicy::new(true, true));
        let old = runtime
            .schedule(&registry, playback, MediaWorkKind::Decode(audio))
            .unwrap();
        assert_eq!(
            runtime
                .set_environment(MediaEnvironment::Suspended, &registry)
                .unwrap(),
            1
        );
        assert_eq!(runtime.pending_count(), 0);
        assert!(matches!(
            runtime
                .schedule(&registry, playback, MediaWorkKind::Decode(audio))
                .unwrap_err(),
            MediaRuntimeError::WorkBlockedByEnvironment {
                environment: MediaEnvironment::Suspended,
                ..
            }
        ));
        runtime
            .set_environment(MediaEnvironment::Foreground, &registry)
            .unwrap();
        let fresh = runtime
            .schedule(&registry, playback, MediaWorkKind::Decode(audio))
            .unwrap();
        assert_ne!(old, fresh);
    }

    #[test]
    fn exact_selected_stream_and_output_kind_are_revalidated() {
        let (registry, playback, _audio, video) = fixture(false);
        let mut runtime = new_runtime(4, MediaBackgroundPolicy::default());
        assert!(matches!(
            runtime
                .schedule(&registry, playback, MediaWorkKind::Decode(video))
                .unwrap_err(),
            MediaRuntimeError::StreamNotSelected { .. }
        ));
        assert!(matches!(
            runtime
                .schedule(
                    &registry,
                    playback,
                    MediaWorkKind::Output(MediaStreamKind::Video)
                )
                .unwrap_err(),
            MediaRuntimeError::OutputKindNotSelected { .. }
        ));
    }

    #[test]
    fn outstanding_limit_includes_active_work_and_recovers_exactly() {
        let (registry, playback, audio, _) = fixture(true);
        let mut runtime = new_runtime(1, MediaBackgroundPolicy::default());
        let first = runtime
            .schedule(&registry, playback, MediaWorkKind::Decode(audio))
            .unwrap();
        let work = runtime.next_work(&registry).unwrap().unwrap();
        assert_eq!(work.id(), first);
        assert!(matches!(
            runtime
                .schedule(
                    &registry,
                    playback,
                    MediaWorkKind::Output(MediaStreamKind::Audio)
                )
                .unwrap_err(),
            MediaRuntimeError::QueueFull { .. }
        ));
        runtime.complete(first).unwrap();
        runtime
            .schedule(
                &registry,
                playback,
                MediaWorkKind::Output(MediaStreamKind::Audio),
            )
            .unwrap();
    }

    #[test]
    fn active_work_blocks_environment_changes_and_wrong_completion_is_fail_closed() {
        let (registry, playback, audio, _) = fixture(true);
        let mut runtime = new_runtime(2, MediaBackgroundPolicy::new(true, false));
        let first = runtime
            .schedule(&registry, playback, MediaWorkKind::Decode(audio))
            .unwrap();
        runtime.next_work(&registry).unwrap().unwrap();
        assert_eq!(
            runtime
                .set_environment(MediaEnvironment::Background, &registry)
                .unwrap_err(),
            MediaRuntimeError::WorkInProgress(first)
        );
        let mut foreign = new_runtime(2, MediaBackgroundPolicy::default());
        let other = foreign
            .schedule(&registry, playback, MediaWorkKind::Decode(audio))
            .unwrap();
        assert_eq!(
            runtime.complete(other).unwrap_err(),
            MediaRuntimeError::WrongCompletion {
                expected: first,
                actual: other,
            }
        );
        assert_eq!(runtime.active_work().unwrap().id(), first);
        runtime.complete(first).unwrap();
    }

    #[test]
    fn retired_or_ended_playback_work_is_never_exported() {
        let (mut registry, playback, audio, _) = fixture(true);
        let mut runtime = new_runtime(4, MediaBackgroundPolicy::default());
        runtime
            .schedule(&registry, playback, MediaWorkKind::Demux(audio))
            .unwrap();
        registry.mark_ended(playback).unwrap();
        assert!(runtime.next_work(&registry).unwrap().is_none());

        let (mut registry, playback, audio, _) = fixture(true);
        let mut runtime = new_runtime(4, MediaBackgroundPolicy::default());
        runtime
            .schedule(&registry, playback, MediaWorkKind::Demux(audio))
            .unwrap();
        registry.retire_playback(playback).unwrap();
        assert!(runtime.next_work(&registry).unwrap().is_none());
    }
}
