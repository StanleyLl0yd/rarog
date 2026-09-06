use rarog_paint::{DamageRegion, DisplayList};
use rarog_resources::ImageResourceStore;
use rarog_types::{Color, Point, Rect};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SurfaceId(u64);

impl SurfaceId {
    pub const fn new(value: u64) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurfaceSize {
    pub width: u32,
    pub height: u32,
}

impl SurfaceSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn is_suspended(self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn rect(self) -> Rect {
        Rect::new(0.0, 0.0, self.width as f32, self.height as f32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameId(u64);

impl FrameId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DisplayListRevision(u64);

impl DisplayListRevision {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameCause {
    Initial,
    Resize,
    SceneChange,
    Scroll,
    ResourceReady,
    Explicit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameUpdateKind {
    Full,
    Partial,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameRequestReasons(u8);

impl FrameRequestReasons {
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, cause: FrameCause) -> bool {
        self.0 & frame_cause_bit(cause) != 0
    }

    pub fn primary_cause(self) -> Option<FrameCause> {
        [
            FrameCause::Initial,
            FrameCause::Resize,
            FrameCause::ResourceReady,
            FrameCause::Scroll,
            FrameCause::SceneChange,
            FrameCause::Explicit,
        ]
        .into_iter()
        .find(|&cause| self.contains(cause))
    }

    fn insert(&mut self, cause: FrameCause) {
        self.0 |= frame_cause_bit(cause);
    }
}

const fn frame_cause_bit(cause: FrameCause) -> u8 {
    match cause {
        FrameCause::Initial => 1 << 0,
        FrameCause::Resize => 1 << 1,
        FrameCause::SceneChange => 1 << 2,
        FrameCause::Scroll => 1 << 3,
        FrameCause::ResourceReady => 1 << 4,
        FrameCause::Explicit => 1 << 5,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameRequestId(u64);

impl FrameRequestId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduledFrameRequest {
    id: FrameRequestId,
    reasons: FrameRequestReasons,
}

impl ScheduledFrameRequest {
    pub const fn id(self) -> FrameRequestId {
        self.id
    }

    pub const fn reasons(self) -> FrameRequestReasons {
        self.reasons
    }

    pub fn primary_cause(self) -> FrameCause {
        match self.reasons.primary_cause() {
            Some(cause) => cause,
            None => FrameCause::Explicit,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameSchedulerError {
    RequestIdExhausted,
    FrameInProgress(FrameRequestId),
    NoActiveFrame,
    WrongCompletion {
        expected: FrameRequestId,
        actual: FrameRequestId,
    },
}

impl fmt::Display for FrameSchedulerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequestIdExhausted => {
                formatter.write_str("frame scheduler request identity space exhausted")
            }
            Self::FrameInProgress(request) => {
                write!(formatter, "frame request {request:?} is still in progress")
            }
            Self::NoActiveFrame => formatter.write_str("frame scheduler has no active request"),
            Self::WrongCompletion { expected, actual } => write!(
                formatter,
                "frame scheduler expected completion for {expected:?}, got {actual:?}"
            ),
        }
    }
}

impl std::error::Error for FrameSchedulerError {}

#[derive(Debug)]
pub struct FrameScheduler {
    next_request: u64,
    pending: FrameRequestReasons,
    active: Option<ScheduledFrameRequest>,
}

impl Default for FrameScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameScheduler {
    pub const fn new() -> Self {
        Self {
            next_request: 1,
            pending: FrameRequestReasons(0),
            active: None,
        }
    }

    pub fn request(&mut self, cause: FrameCause) {
        self.pending.insert(cause);
    }

    pub const fn pending_reasons(&self) -> FrameRequestReasons {
        self.pending
    }

    pub const fn active_request(&self) -> Option<FrameRequestId> {
        match self.active {
            Some(active) => Some(active.id),
            None => None,
        }
    }

    pub fn begin(&mut self) -> Result<Option<ScheduledFrameRequest>, FrameSchedulerError> {
        if let Some(active) = self.active {
            return Err(FrameSchedulerError::FrameInProgress(active.id));
        }
        if self.pending.is_empty() {
            return Ok(None);
        }
        if self.next_request == 0 {
            return Err(FrameSchedulerError::RequestIdExhausted);
        }

        let id = FrameRequestId(self.next_request);
        self.next_request = self.next_request.checked_add(1).unwrap_or(0);
        let reasons = std::mem::take(&mut self.pending);
        let scheduled = ScheduledFrameRequest { id, reasons };
        self.active = Some(scheduled);
        Ok(Some(scheduled))
    }

    pub fn complete(&mut self, request: FrameRequestId) -> Result<(), FrameSchedulerError> {
        self.take_active(request)?;
        Ok(())
    }

    pub fn discard(&mut self, request: FrameRequestId) -> Result<(), FrameSchedulerError> {
        let active = self.take_active(request)?;
        self.pending.0 |= active.reasons.0;
        Ok(())
    }

    fn take_active(
        &mut self,
        request: FrameRequestId,
    ) -> Result<ScheduledFrameRequest, FrameSchedulerError> {
        let Some(active) = self.active else {
            return Err(FrameSchedulerError::NoActiveFrame);
        };
        if active.id != request {
            return Err(FrameSchedulerError::WrongCompletion {
                expected: active.id,
                actual: request,
            });
        }
        self.active = None;
        Ok(active)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FramePlan {
    id: FrameId,
    surface: SurfaceId,
    size: SurfaceSize,
    revision: DisplayListRevision,
    cause: FrameCause,
    update_kind: FrameUpdateKind,
    damage: Vec<Rect>,
}

impl FramePlan {
    pub const fn id(&self) -> FrameId {
        self.id
    }

    pub const fn surface(&self) -> SurfaceId {
        self.surface
    }

    pub const fn size(&self) -> SurfaceSize {
        self.size
    }

    pub const fn revision(&self) -> DisplayListRevision {
        self.revision
    }

    pub const fn cause(&self) -> FrameCause {
        self.cause
    }

    pub const fn update_kind(&self) -> FrameUpdateKind {
        self.update_kind
    }

    pub fn damage(&self) -> &[Rect] {
        &self.damage
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FrameDecision {
    Noop,
    Suspended { surface: SurfaceId },
    Submit(FramePlan),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramePlannerError {
    InvalidDamage,
    FrameIdExhausted,
    FramePending(FrameId),
    NoPendingFrame,
    WrongFrameCompletion { expected: FrameId, actual: FrameId },
}

impl fmt::Display for FramePlannerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDamage => formatter.write_str("frame damage must contain finite geometry"),
            Self::FrameIdExhausted => {
                formatter.write_str("compositor frame identity space exhausted")
            }
            Self::FramePending(frame) => {
                write!(formatter, "compositor frame {frame:?} is still pending")
            }
            Self::NoPendingFrame => formatter.write_str("compositor has no pending frame"),
            Self::WrongFrameCompletion { expected, actual } => write!(
                formatter,
                "compositor expected completion for {expected:?}, got {actual:?}"
            ),
        }
    }
}

impl std::error::Error for FramePlannerError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PresentedState {
    size: SurfaceSize,
    revision: DisplayListRevision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PendingState {
    id: FrameId,
    size: SurfaceSize,
    revision: DisplayListRevision,
}

#[derive(Debug)]
pub struct FramePlanner {
    surface: SurfaceId,
    next_frame: u64,
    presented: Option<PresentedState>,
    pending: Option<PendingState>,
}

impl FramePlanner {
    pub const fn new(surface: SurfaceId) -> Self {
        Self {
            surface,
            next_frame: 1,
            presented: None,
            pending: None,
        }
    }

    pub const fn surface(&self) -> SurfaceId {
        self.surface
    }

    pub const fn pending_frame(&self) -> Option<FrameId> {
        match self.pending {
            Some(pending) => Some(pending.id),
            None => None,
        }
    }

    pub const fn presented_revision(&self) -> Option<DisplayListRevision> {
        match self.presented {
            Some(presented) => Some(presented.revision),
            None => None,
        }
    }

    pub fn plan(
        &mut self,
        size: SurfaceSize,
        revision: DisplayListRevision,
        damage: &DamageRegion,
        cause: FrameCause,
    ) -> Result<FrameDecision, FramePlannerError> {
        if let Some(pending) = self.pending {
            return Err(FramePlannerError::FramePending(pending.id));
        }

        let normalized_damage = normalize_damage(size, damage)?;
        if size.is_suspended() {
            self.presented = None;
            return Ok(FrameDecision::Suspended {
                surface: self.surface,
            });
        }

        let surface_rect = size.rect();
        let (update_kind, frame_cause, frame_damage) = match self.presented {
            None => (
                FrameUpdateKind::Full,
                FrameCause::Initial,
                vec![surface_rect],
            ),
            Some(presented) if presented.size != size => (
                FrameUpdateKind::Full,
                FrameCause::Resize,
                vec![surface_rect],
            ),
            Some(_) if normalized_damage.is_empty() => return Ok(FrameDecision::Noop),
            Some(_) if normalized_damage.len() == 1 && normalized_damage[0] == surface_rect => {
                (FrameUpdateKind::Full, cause, normalized_damage)
            }
            Some(_) => (FrameUpdateKind::Partial, cause, normalized_damage),
        };

        let id = self.allocate_frame_id()?;
        self.pending = Some(PendingState { id, size, revision });
        Ok(FrameDecision::Submit(FramePlan {
            id,
            surface: self.surface,
            size,
            revision,
            cause: frame_cause,
            update_kind,
            damage: frame_damage,
        }))
    }

    pub fn complete(&mut self, frame: FrameId) -> Result<(), FramePlannerError> {
        let pending = self.pending.ok_or(FramePlannerError::NoPendingFrame)?;
        if pending.id != frame {
            return Err(FramePlannerError::WrongFrameCompletion {
                expected: pending.id,
                actual: frame,
            });
        }
        self.pending = None;
        self.presented = Some(PresentedState {
            size: pending.size,
            revision: pending.revision,
        });
        Ok(())
    }

    pub fn discard(&mut self, frame: FrameId) -> Result<(), FramePlannerError> {
        let pending = self.pending.ok_or(FramePlannerError::NoPendingFrame)?;
        if pending.id != frame {
            return Err(FramePlannerError::WrongFrameCompletion {
                expected: pending.id,
                actual: frame,
            });
        }
        self.pending = None;
        Ok(())
    }

    fn allocate_frame_id(&mut self) -> Result<FrameId, FramePlannerError> {
        if self.next_frame == 0 {
            return Err(FramePlannerError::FrameIdExhausted);
        }
        let id = FrameId(self.next_frame);
        self.next_frame = self.next_frame.checked_add(1).unwrap_or(0);
        Ok(id)
    }
}

pub struct FrameSubmission<'a> {
    pub plan: &'a FramePlan,
    pub display_list: &'a DisplayList,
    pub image_resources: Option<&'a ImageResourceStore>,
    pub viewport_translation: Point,
    pub clear_color: Color,
}

pub trait CompositorBackend {
    type Error;

    fn submit(&mut self, frame: FrameSubmission<'_>) -> Result<(), Self::Error>;
}

fn normalize_damage(
    size: SurfaceSize,
    damage: &DamageRegion,
) -> Result<Vec<Rect>, FramePlannerError> {
    let surface = size.rect();
    let mut output = Vec::new();
    let mut seen = BTreeSet::new();

    for rect in &damage.rects {
        if !rect_is_finite(*rect) {
            return Err(FramePlannerError::InvalidDamage);
        }
        if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
            continue;
        }
        let Some(clipped) = intersection(*rect, surface) else {
            continue;
        };
        if clipped.size.width <= 0.0 || clipped.size.height <= 0.0 {
            continue;
        }
        let key = (
            canonical_float_bits(clipped.origin.x),
            canonical_float_bits(clipped.origin.y),
            canonical_float_bits(clipped.size.width),
            canonical_float_bits(clipped.size.height),
        );
        if seen.insert(key) {
            output.push(clipped);
        }
    }

    Ok(output)
}

fn rect_is_finite(rect: Rect) -> bool {
    [
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
    ]
    .into_iter()
    .all(f32::is_finite)
}

fn intersection(left: Rect, right: Rect) -> Option<Rect> {
    let x0 = left.origin.x.max(right.origin.x);
    let y0 = left.origin.y.max(right.origin.y);
    let x1 = (left.origin.x + left.size.width).min(right.origin.x + right.size.width);
    let y1 = (left.origin.y + left.size.height).min(right.origin.y + right.size.height);
    (x1 > x0 && y1 > y0).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
}

fn canonical_float_bits(value: f32) -> u32 {
    if value == 0.0 { 0 } else { value.to_bits() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surface() -> SurfaceId {
        SurfaceId::new(7).unwrap()
    }

    fn size() -> SurfaceSize {
        SurfaceSize::new(100, 80)
    }

    fn empty_damage() -> DamageRegion {
        DamageRegion::default()
    }

    fn complete_initial(planner: &mut FramePlanner) -> FrameId {
        let FrameDecision::Submit(plan) = planner
            .plan(
                size(),
                DisplayListRevision::new(1),
                &empty_damage(),
                FrameCause::SceneChange,
            )
            .unwrap()
        else {
            panic!("expected initial frame");
        };
        assert_eq!(plan.update_kind(), FrameUpdateKind::Full);
        assert_eq!(plan.cause(), FrameCause::Initial);
        assert_eq!(plan.damage(), &[size().rect()]);
        let id = plan.id();
        planner.complete(id).unwrap();
        id
    }

    #[test]
    fn frame_scheduler_coalesces_duplicate_and_mixed_causes() {
        let mut scheduler = FrameScheduler::new();
        scheduler.request(FrameCause::SceneChange);
        scheduler.request(FrameCause::SceneChange);
        scheduler.request(FrameCause::ResourceReady);

        let request = scheduler.begin().unwrap().unwrap();
        assert_eq!(request.id().get(), 1);
        assert!(request.reasons().contains(FrameCause::SceneChange));
        assert!(request.reasons().contains(FrameCause::ResourceReady));
        assert_eq!(request.primary_cause(), FrameCause::ResourceReady);
        assert!(scheduler.pending_reasons().is_empty());
    }

    #[test]
    fn frame_scheduler_queues_requests_arriving_during_active_frame() {
        let mut scheduler = FrameScheduler::new();
        scheduler.request(FrameCause::Initial);
        let first = scheduler.begin().unwrap().unwrap();
        scheduler.request(FrameCause::Scroll);
        scheduler.request(FrameCause::Resize);

        assert_eq!(
            scheduler.begin().unwrap_err(),
            FrameSchedulerError::FrameInProgress(first.id())
        );
        scheduler.complete(first.id()).unwrap();

        let second = scheduler.begin().unwrap().unwrap();
        assert_eq!(second.id().get(), 2);
        assert_eq!(second.primary_cause(), FrameCause::Resize);
        assert!(second.reasons().contains(FrameCause::Scroll));
        scheduler.complete(second.id()).unwrap();
        assert!(scheduler.begin().unwrap().is_none());
    }

    #[test]
    fn frame_scheduler_discard_requeues_all_active_reasons() {
        let mut scheduler = FrameScheduler::new();
        scheduler.request(FrameCause::Scroll);
        scheduler.request(FrameCause::ResourceReady);
        let request = scheduler.begin().unwrap().unwrap();
        scheduler.request(FrameCause::SceneChange);

        scheduler.discard(request.id()).unwrap();
        assert_eq!(scheduler.active_request(), None);
        assert!(scheduler.pending_reasons().contains(FrameCause::Scroll));
        assert!(
            scheduler
                .pending_reasons()
                .contains(FrameCause::ResourceReady)
        );
        assert!(
            scheduler
                .pending_reasons()
                .contains(FrameCause::SceneChange)
        );

        let retry = scheduler.begin().unwrap().unwrap();
        assert!(retry.reasons().contains(FrameCause::Scroll));
        assert!(retry.reasons().contains(FrameCause::ResourceReady));
        assert!(retry.reasons().contains(FrameCause::SceneChange));
    }

    #[test]
    fn frame_scheduler_wrong_completion_preserves_active_request() {
        let mut scheduler = FrameScheduler::new();
        scheduler.request(FrameCause::Explicit);
        let request = scheduler.begin().unwrap().unwrap();
        let wrong = FrameRequestId(request.id().get() + 1);

        assert_eq!(
            scheduler.complete(wrong).unwrap_err(),
            FrameSchedulerError::WrongCompletion {
                expected: request.id(),
                actual: wrong,
            }
        );
        assert_eq!(scheduler.active_request(), Some(request.id()));
        scheduler.discard(request.id()).unwrap();
        assert_eq!(scheduler.active_request(), None);
    }

    #[test]
    fn frame_scheduler_identity_exhaustion_never_reuses_requests() {
        let mut scheduler = FrameScheduler::new();
        scheduler.next_request = u64::MAX;
        scheduler.request(FrameCause::Explicit);
        let last = scheduler.begin().unwrap().unwrap();
        assert_eq!(last.id().get(), u64::MAX);
        scheduler.complete(last.id()).unwrap();

        scheduler.request(FrameCause::Explicit);
        assert_eq!(
            scheduler.begin().unwrap_err(),
            FrameSchedulerError::RequestIdExhausted
        );
        assert!(scheduler.pending_reasons().contains(FrameCause::Explicit));
    }

    #[test]
    fn surface_ids_reject_zero() {
        assert!(SurfaceId::new(0).is_none());
        assert_eq!(surface().get(), 7);
    }

    #[test]
    fn first_active_frame_is_a_full_surface_update() {
        let mut planner = FramePlanner::new(surface());
        let id = complete_initial(&mut planner);
        assert_eq!(id.get(), 1);
        assert_eq!(
            planner.presented_revision(),
            Some(DisplayListRevision::new(1))
        );
    }

    #[test]
    fn subsequent_damage_is_clipped_and_deduplicated() {
        let mut planner = FramePlanner::new(surface());
        complete_initial(&mut planner);
        let damage = DamageRegion {
            rects: vec![
                Rect::new(90.0, 70.0, 20.0, 20.0),
                Rect::new(90.0, 70.0, 20.0, 20.0),
                Rect::new(-20.0, -20.0, 5.0, 5.0),
            ],
        };

        let FrameDecision::Submit(plan) = planner
            .plan(
                size(),
                DisplayListRevision::new(2),
                &damage,
                FrameCause::SceneChange,
            )
            .unwrap()
        else {
            panic!("expected partial frame");
        };

        assert_eq!(plan.update_kind(), FrameUpdateKind::Partial);
        assert_eq!(plan.cause(), FrameCause::SceneChange);
        assert_eq!(plan.damage(), &[Rect::new(90.0, 70.0, 10.0, 10.0)]);
    }

    #[test]
    fn unchanged_surface_with_empty_damage_is_noop() {
        let mut planner = FramePlanner::new(surface());
        complete_initial(&mut planner);
        assert_eq!(
            planner
                .plan(
                    size(),
                    DisplayListRevision::new(2),
                    &empty_damage(),
                    FrameCause::SceneChange,
                )
                .unwrap(),
            FrameDecision::Noop
        );
    }

    #[test]
    fn resize_forces_full_redraw() {
        let mut planner = FramePlanner::new(surface());
        complete_initial(&mut planner);
        let resized = SurfaceSize::new(120, 90);
        let FrameDecision::Submit(plan) = planner
            .plan(
                resized,
                DisplayListRevision::new(2),
                &empty_damage(),
                FrameCause::SceneChange,
            )
            .unwrap()
        else {
            panic!("expected resize frame");
        };

        assert_eq!(plan.cause(), FrameCause::Resize);
        assert_eq!(plan.update_kind(), FrameUpdateKind::Full);
        assert_eq!(plan.damage(), &[resized.rect()]);
    }

    #[test]
    fn suspended_surface_drops_presented_state_and_resume_is_full() {
        let mut planner = FramePlanner::new(surface());
        complete_initial(&mut planner);
        assert_eq!(
            planner
                .plan(
                    SurfaceSize::new(0, 80),
                    DisplayListRevision::new(2),
                    &empty_damage(),
                    FrameCause::Explicit,
                )
                .unwrap(),
            FrameDecision::Suspended { surface: surface() }
        );
        assert_eq!(planner.presented_revision(), None);

        let FrameDecision::Submit(plan) = planner
            .plan(
                size(),
                DisplayListRevision::new(3),
                &empty_damage(),
                FrameCause::Explicit,
            )
            .unwrap()
        else {
            panic!("expected full resume frame");
        };
        assert_eq!(plan.cause(), FrameCause::Initial);
        assert_eq!(plan.update_kind(), FrameUpdateKind::Full);
    }

    #[test]
    fn pending_frame_blocks_replanning_until_completed_or_discarded() {
        let mut planner = FramePlanner::new(surface());
        let FrameDecision::Submit(plan) = planner
            .plan(
                size(),
                DisplayListRevision::new(1),
                &empty_damage(),
                FrameCause::Explicit,
            )
            .unwrap()
        else {
            panic!("expected frame");
        };
        let id = plan.id();
        assert_eq!(
            planner
                .plan(
                    size(),
                    DisplayListRevision::new(1),
                    &empty_damage(),
                    FrameCause::Explicit,
                )
                .unwrap_err(),
            FramePlannerError::FramePending(id)
        );

        let wrong = FrameId(id.get() + 1);
        assert_eq!(
            planner.complete(wrong).unwrap_err(),
            FramePlannerError::WrongFrameCompletion {
                expected: id,
                actual: wrong,
            }
        );
        assert_eq!(planner.pending_frame(), Some(id));
        planner.discard(id).unwrap();
        assert_eq!(planner.pending_frame(), None);

        let FrameDecision::Submit(next) = planner
            .plan(
                size(),
                DisplayListRevision::new(1),
                &empty_damage(),
                FrameCause::Explicit,
            )
            .unwrap()
        else {
            panic!("expected replacement frame");
        };
        assert_eq!(next.id().get(), id.get() + 1);
    }

    #[test]
    fn invalid_damage_fails_before_frame_allocation() {
        let mut planner = FramePlanner::new(surface());
        let damage = DamageRegion {
            rects: vec![Rect::new(f32::NAN, 0.0, 10.0, 10.0)],
        };
        assert_eq!(
            planner
                .plan(
                    size(),
                    DisplayListRevision::new(1),
                    &damage,
                    FrameCause::SceneChange,
                )
                .unwrap_err(),
            FramePlannerError::InvalidDamage
        );

        let FrameDecision::Submit(plan) = planner
            .plan(
                size(),
                DisplayListRevision::new(1),
                &empty_damage(),
                FrameCause::SceneChange,
            )
            .unwrap()
        else {
            panic!("expected initial frame after invalid input");
        };
        assert_eq!(plan.id().get(), 1);
    }

    #[test]
    fn frame_identity_exhaustion_never_reuses_ids() {
        let mut planner = FramePlanner::new(surface());
        planner.next_frame = u64::MAX;
        let FrameDecision::Submit(last) = planner
            .plan(
                size(),
                DisplayListRevision::new(1),
                &empty_damage(),
                FrameCause::Explicit,
            )
            .unwrap()
        else {
            panic!("expected final frame id");
        };
        assert_eq!(last.id().get(), u64::MAX);
        planner.discard(last.id()).unwrap();
        assert_eq!(
            planner
                .plan(
                    size(),
                    DisplayListRevision::new(1),
                    &empty_damage(),
                    FrameCause::Explicit,
                )
                .unwrap_err(),
            FramePlannerError::FrameIdExhausted
        );
    }

    #[test]
    fn backend_submission_contract_contains_only_rarog_types() {
        struct Backend {
            submitted: usize,
        }

        impl CompositorBackend for Backend {
            type Error = ();

            fn submit(&mut self, frame: FrameSubmission<'_>) -> Result<(), Self::Error> {
                assert_eq!(frame.plan.surface(), surface());
                assert_eq!(frame.display_list.len(), 0);
                assert_eq!(frame.clear_color, Color::WHITE);
                self.submitted += 1;
                Ok(())
            }
        }

        let mut planner = FramePlanner::new(surface());
        let FrameDecision::Submit(plan) = planner
            .plan(
                size(),
                DisplayListRevision::new(1),
                &empty_damage(),
                FrameCause::Explicit,
            )
            .unwrap()
        else {
            panic!("expected frame");
        };
        let list = DisplayList::default();
        let mut backend = Backend { submitted: 0 };
        backend
            .submit(FrameSubmission {
                plan: &plan,
                display_list: &list,
                image_resources: None,
                viewport_translation: Point::default(),
                clear_color: Color::WHITE,
            })
            .unwrap();
        assert_eq!(backend.submitted, 1);
    }
}
