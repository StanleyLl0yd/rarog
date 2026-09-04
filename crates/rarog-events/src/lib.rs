use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroU64;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_LISTENER_SCOPE: AtomicU64 = AtomicU64::new(1);

pub const DEFAULT_MAX_EVENT_LISTENER_REGISTRATIONS: usize = 65_536;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventType(Arc<str>);

impl EventType {
    pub fn new(value: impl Into<Arc<str>>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for EventType {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for EventType {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventError {
    ListenerAllocatorIdentityExhausted,
    ListenerIdentityExhausted,
    RegistrationIdentityExhausted,
    InvalidRegistrationLimit,
    RegistrationLimitExceeded { registrations: usize, limit: usize },
}

impl fmt::Display for EventError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ListenerAllocatorIdentityExhausted => {
                "event listener allocator identity space is exhausted"
            }
            Self::ListenerIdentityExhausted => "event listener identity space is exhausted",
            Self::RegistrationIdentityExhausted => {
                "event listener registration identity space is exhausted"
            }
            Self::InvalidRegistrationLimit => "event listener registration limit must be non-zero",
            Self::RegistrationLimitExceeded { .. } => "event listener registration limit exceeded",
        })
    }
}

impl std::error::Error for EventError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventListenerId {
    scope: NonZeroU64,
    serial: NonZeroU64,
}

#[derive(Debug)]
pub struct EventListenerIdAllocator {
    scope: NonZeroU64,
    next: u64,
}

impl EventListenerIdAllocator {
    pub fn new() -> Result<Self, EventError> {
        let scope = NEXT_LISTENER_SCOPE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| EventError::ListenerAllocatorIdentityExhausted)?;
        let scope = NonZeroU64::new(scope).ok_or(EventError::ListenerAllocatorIdentityExhausted)?;
        Ok(Self { scope, next: 1 })
    }

    pub fn allocate(&mut self) -> Result<EventListenerId, EventError> {
        let serial = NonZeroU64::new(self.next).ok_or(EventError::ListenerIdentityExhausted)?;
        self.next = self.next.checked_add(1).unwrap_or(0);
        Ok(EventListenerId {
            scope: self.scope,
            serial,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventListenerOptions {
    pub capture: bool,
    pub once: bool,
    pub passive: bool,
}

impl EventListenerOptions {
    pub const fn new(capture: bool, once: bool, passive: bool) -> Self {
        Self {
            capture,
            once,
            passive,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventPhase {
    Capturing,
    AtTarget,
    Bubbling,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    event_type: EventType,
    bubbles: bool,
    cancelable: bool,
    default_prevented: bool,
    propagation_stopped: bool,
    immediate_propagation_stopped: bool,
}

impl Event {
    pub fn new(event_type: impl Into<EventType>, bubbles: bool, cancelable: bool) -> Self {
        Self {
            event_type: event_type.into(),
            bubbles,
            cancelable,
            default_prevented: false,
            propagation_stopped: false,
            immediate_propagation_stopped: false,
        }
    }

    pub fn event_type(&self) -> &EventType {
        &self.event_type
    }

    pub fn bubbles(&self) -> bool {
        self.bubbles
    }

    pub fn cancelable(&self) -> bool {
        self.cancelable
    }

    pub fn default_prevented(&self) -> bool {
        self.default_prevented
    }

    pub fn propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }

    pub fn immediate_propagation_stopped(&self) -> bool {
        self.immediate_propagation_stopped
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct RegistrationId(NonZeroU64);

#[derive(Debug)]
struct Registration {
    id: RegistrationId,
    event_type: EventType,
    listener: EventListenerId,
    options: EventListenerOptions,
}

#[derive(Debug)]
pub struct EventTargetRegistry<T> {
    listeners: BTreeMap<T, Vec<Registration>>,
    next_registration: u64,
    max_registrations: usize,
    registration_count: usize,
}

impl<T> Default for EventTargetRegistry<T> {
    fn default() -> Self {
        Self {
            listeners: BTreeMap::new(),
            next_registration: 1,
            max_registrations: DEFAULT_MAX_EVENT_LISTENER_REGISTRATIONS,
            registration_count: 0,
        }
    }
}

impl<T: Ord + Clone> EventTargetRegistry<T> {
    pub fn try_with_max_registrations(max_registrations: usize) -> Result<Self, EventError> {
        if max_registrations == 0 {
            return Err(EventError::InvalidRegistrationLimit);
        }
        Ok(Self {
            listeners: BTreeMap::new(),
            next_registration: 1,
            max_registrations,
            registration_count: 0,
        })
    }

    pub fn max_registrations(&self) -> usize {
        self.max_registrations
    }

    pub fn registration_count(&self) -> usize {
        self.registration_count
    }

    pub fn add_listener(
        &mut self,
        target: T,
        event_type: impl Into<EventType>,
        listener: EventListenerId,
        options: EventListenerOptions,
    ) -> Result<bool, EventError> {
        let event_type = event_type.into();
        if self.listeners.get(&target).is_some_and(|listeners| {
            listeners.iter().any(|registration| {
                registration.event_type == event_type
                    && registration.listener == listener
                    && registration.options.capture == options.capture
            })
        }) {
            return Ok(false);
        }
        if self.registration_count >= self.max_registrations {
            return Err(EventError::RegistrationLimitExceeded {
                registrations: self.registration_count.saturating_add(1),
                limit: self.max_registrations,
            });
        }

        let id = NonZeroU64::new(self.next_registration)
            .map(RegistrationId)
            .ok_or(EventError::RegistrationIdentityExhausted)?;
        self.next_registration = self.next_registration.checked_add(1).unwrap_or(0);
        self.listeners
            .entry(target)
            .or_default()
            .push(Registration {
                id,
                event_type,
                listener,
                options,
            });
        self.registration_count += 1;
        Ok(true)
    }

    pub fn remove_listener(
        &mut self,
        target: &T,
        event_type: &EventType,
        listener: EventListenerId,
        capture: bool,
    ) -> bool {
        let Some(listeners) = self.listeners.get_mut(target) else {
            return false;
        };
        let Some(position) = listeners.iter().position(|registration| {
            &registration.event_type == event_type
                && registration.listener == listener
                && registration.options.capture == capture
        }) else {
            return false;
        };
        listeners.remove(position);
        self.registration_count -= 1;
        let remove_target = listeners.is_empty();
        if remove_target {
            self.listeners.remove(target);
        }
        true
    }

    pub fn listener_count(&self, target: &T) -> usize {
        self.listeners.get(target).map_or(0, Vec::len)
    }

    pub fn next_listener(
        &mut self,
        dispatch: &mut EventDispatch<T>,
    ) -> Option<EventListenerInvocation<T>> {
        dispatch.current_passive = None;

        loop {
            if dispatch.finished || dispatch.event.immediate_propagation_stopped {
                dispatch.finished = true;
                return None;
            }

            if !dispatch.snapshot_loaded {
                let group = dispatch.groups.get(dispatch.group_index)?;
                dispatch.snapshot = self.snapshot(group, dispatch.event.event_type());
                dispatch.snapshot_index = 0;
                dispatch.snapshot_loaded = true;
            }

            while dispatch.snapshot_index < dispatch.snapshot.len() {
                let registration_id = dispatch.snapshot[dispatch.snapshot_index];
                dispatch.snapshot_index += 1;
                let group = &dispatch.groups[dispatch.group_index];
                let Some((listener, options)) =
                    self.resolve_registration(&group.target, registration_id)
                else {
                    continue;
                };
                dispatch.current_passive = Some(options.passive);
                return Some(EventListenerInvocation {
                    current_target: group.target.clone(),
                    listener,
                    phase: group.phase,
                    passive: options.passive,
                });
            }

            let current_index = dispatch.group_index;
            let next_index = current_index + 1;
            if next_index >= dispatch.groups.len() {
                dispatch.finished = true;
                return None;
            }

            if dispatch.event.propagation_stopped
                && !same_at_target(
                    &dispatch.groups[current_index],
                    &dispatch.groups[next_index],
                )
            {
                dispatch.finished = true;
                return None;
            }

            dispatch.group_index = next_index;
            dispatch.snapshot.clear();
            dispatch.snapshot_loaded = false;
        }
    }

    fn snapshot(&self, group: &DispatchGroup<T>, event_type: &EventType) -> Vec<RegistrationId> {
        self.listeners
            .get(&group.target)
            .into_iter()
            .flatten()
            .filter(|registration| {
                &registration.event_type == event_type
                    && registration.options.capture == group.capture
            })
            .map(|registration| registration.id)
            .collect()
    }

    fn resolve_registration(
        &mut self,
        target: &T,
        registration_id: RegistrationId,
    ) -> Option<(EventListenerId, EventListenerOptions)> {
        let (listener, options, remove_target) = {
            let listeners = self.listeners.get_mut(target)?;
            let position = listeners
                .iter()
                .position(|registration| registration.id == registration_id)?;
            let listener = listeners[position].listener;
            let options = listeners[position].options;
            if options.once {
                listeners.remove(position);
                self.registration_count -= 1;
            }
            (listener, options, listeners.is_empty())
        };
        if remove_target {
            self.listeners.remove(target);
        }
        Some((listener, options))
    }
}

#[derive(Clone, Debug)]
struct DispatchGroup<T> {
    target: T,
    phase: EventPhase,
    capture: bool,
}

fn same_at_target<T: PartialEq>(first: &DispatchGroup<T>, second: &DispatchGroup<T>) -> bool {
    first.phase == EventPhase::AtTarget
        && second.phase == EventPhase::AtTarget
        && first.target == second.target
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventListenerInvocation<T> {
    pub current_target: T,
    pub listener: EventListenerId,
    pub phase: EventPhase,
    pub passive: bool,
}

#[derive(Debug)]
pub struct EventDispatch<T> {
    event: Event,
    target: T,
    groups: Vec<DispatchGroup<T>>,
    group_index: usize,
    snapshot: Vec<RegistrationId>,
    snapshot_index: usize,
    snapshot_loaded: bool,
    current_passive: Option<bool>,
    finished: bool,
}

impl<T: Clone> EventDispatch<T> {
    pub fn new(target: T, ancestors: &[T], event: Event) -> Self {
        let mut groups = Vec::with_capacity(ancestors.len().saturating_mul(2).saturating_add(2));
        for ancestor in ancestors.iter().rev() {
            groups.push(DispatchGroup {
                target: ancestor.clone(),
                phase: EventPhase::Capturing,
                capture: true,
            });
        }
        groups.push(DispatchGroup {
            target: target.clone(),
            phase: EventPhase::AtTarget,
            capture: true,
        });
        groups.push(DispatchGroup {
            target: target.clone(),
            phase: EventPhase::AtTarget,
            capture: false,
        });
        if event.bubbles() {
            for ancestor in ancestors {
                groups.push(DispatchGroup {
                    target: ancestor.clone(),
                    phase: EventPhase::Bubbling,
                    capture: false,
                });
            }
        }
        Self {
            event,
            target,
            groups,
            group_index: 0,
            snapshot: Vec::new(),
            snapshot_index: 0,
            snapshot_loaded: false,
            current_passive: None,
            finished: false,
        }
    }

    pub fn event(&self) -> &Event {
        &self.event
    }

    pub fn target(&self) -> &T {
        &self.target
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn stop_propagation(&mut self) {
        self.event.propagation_stopped = true;
    }

    pub fn stop_immediate_propagation(&mut self) {
        self.event.propagation_stopped = true;
        self.event.immediate_propagation_stopped = true;
    }

    pub fn prevent_default(&mut self) -> bool {
        if !self.event.cancelable || self.current_passive == Some(true) {
            return false;
        }
        self.event.default_prevented = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listener_ids(count: usize) -> Vec<EventListenerId> {
        let mut allocator = EventListenerIdAllocator::new().unwrap();
        (0..count).map(|_| allocator.allocate().unwrap()).collect()
    }

    #[test]
    fn listener_ids_from_different_allocators_do_not_alias() {
        let first = EventListenerIdAllocator::new().unwrap().allocate().unwrap();
        let second = EventListenerIdAllocator::new().unwrap().allocate().unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn listener_id_allocator_reports_serial_exhaustion() {
        let mut allocator = EventListenerIdAllocator::new().unwrap();
        allocator.next = u64::MAX;
        assert_eq!(allocator.allocate().unwrap().serial.get(), u64::MAX);
        assert_eq!(
            allocator.allocate().unwrap_err(),
            EventError::ListenerIdentityExhausted
        );
    }

    #[test]
    fn registration_limits_are_global_and_removal_releases_capacity() {
        let ids = listener_ids(3);
        let mut registry = EventTargetRegistry::try_with_max_registrations(2).unwrap();
        assert_eq!(registry.max_registrations(), 2);

        assert!(
            registry
                .add_listener(1_u8, "click", ids[0], EventListenerOptions::default())
                .unwrap()
        );
        assert!(
            registry
                .add_listener(2_u8, "click", ids[1], EventListenerOptions::default())
                .unwrap()
        );
        assert_eq!(registry.registration_count(), 2);

        assert_eq!(
            registry
                .add_listener(3_u8, "click", ids[2], EventListenerOptions::default())
                .unwrap_err(),
            EventError::RegistrationLimitExceeded {
                registrations: 3,
                limit: 2,
            }
        );

        let event_type = EventType::from("click");
        assert!(registry.remove_listener(&1, &event_type, ids[0], false));
        assert_eq!(registry.registration_count(), 1);
        assert!(
            registry
                .add_listener(3_u8, "click", ids[2], EventListenerOptions::default())
                .unwrap()
        );
        assert_eq!(registry.registration_count(), 2);
    }

    #[test]
    fn duplicate_registration_does_not_consume_listener_budget() {
        let listener = listener_ids(1)[0];
        let mut registry = EventTargetRegistry::try_with_max_registrations(1).unwrap();
        assert!(
            registry
                .add_listener(1_u8, "click", listener, EventListenerOptions::default())
                .unwrap()
        );
        assert!(
            !registry
                .add_listener(1_u8, "click", listener, EventListenerOptions::default())
                .unwrap()
        );
        assert_eq!(registry.registration_count(), 1);
    }

    #[test]
    fn zero_registration_limit_is_rejected() {
        assert!(matches!(
            EventTargetRegistry::<u8>::try_with_max_registrations(0),
            Err(EventError::InvalidRegistrationLimit)
        ));
    }

    #[test]
    fn duplicate_registration_uses_callback_and_capture_identity() {
        let listener = listener_ids(1)[0];
        let mut registry = EventTargetRegistry::default();
        assert!(
            registry
                .add_listener(1_u8, "click", listener, EventListenerOptions::default())
                .unwrap()
        );
        assert!(
            !registry
                .add_listener(
                    1_u8,
                    "click",
                    listener,
                    EventListenerOptions::new(false, true, true),
                )
                .unwrap()
        );
        assert!(
            registry
                .add_listener(
                    1_u8,
                    "click",
                    listener,
                    EventListenerOptions::new(true, false, false),
                )
                .unwrap()
        );
        assert_eq!(registry.listener_count(&1), 2);
    }

    #[test]
    fn removal_matches_capture_but_not_once_or_passive() {
        let listener = listener_ids(1)[0];
        let event_type = EventType::from("click");
        let mut registry = EventTargetRegistry::default();
        registry
            .add_listener(
                1_u8,
                event_type.clone(),
                listener,
                EventListenerOptions::new(false, true, true),
            )
            .unwrap();
        assert!(registry.remove_listener(&1, &event_type, listener, false));
        assert_eq!(registry.listener_count(&1), 0);
    }

    #[test]
    fn dispatch_orders_capture_target_and_bubble_phases() {
        let ids = listener_ids(6);
        let mut registry = EventTargetRegistry::default();
        registry
            .add_listener(
                0_u8,
                "click",
                ids[0],
                EventListenerOptions::new(true, false, false),
            )
            .unwrap();
        registry
            .add_listener(
                1_u8,
                "click",
                ids[1],
                EventListenerOptions::new(true, false, false),
            )
            .unwrap();
        registry
            .add_listener(
                2_u8,
                "click",
                ids[2],
                EventListenerOptions::new(true, false, false),
            )
            .unwrap();
        registry
            .add_listener(2_u8, "click", ids[3], EventListenerOptions::default())
            .unwrap();
        registry
            .add_listener(1_u8, "click", ids[4], EventListenerOptions::default())
            .unwrap();
        registry
            .add_listener(0_u8, "click", ids[5], EventListenerOptions::default())
            .unwrap();

        let mut dispatch = EventDispatch::new(2_u8, &[1, 0], Event::new("click", true, true));
        let mut seen = Vec::new();
        while let Some(invocation) = registry.next_listener(&mut dispatch) {
            seen.push((invocation.listener, invocation.phase));
        }

        assert_eq!(
            seen,
            vec![
                (ids[0], EventPhase::Capturing),
                (ids[1], EventPhase::Capturing),
                (ids[2], EventPhase::AtTarget),
                (ids[3], EventPhase::AtTarget),
                (ids[4], EventPhase::Bubbling),
                (ids[5], EventPhase::Bubbling),
            ]
        );
    }

    #[test]
    fn non_bubbling_events_still_invoke_at_target_non_capture_listeners() {
        let ids = listener_ids(2);
        let mut registry = EventTargetRegistry::default();
        registry
            .add_listener(2_u8, "load", ids[0], EventListenerOptions::default())
            .unwrap();
        registry
            .add_listener(1_u8, "load", ids[1], EventListenerOptions::default())
            .unwrap();
        let mut dispatch = EventDispatch::new(2_u8, &[1], Event::new("load", false, false));
        let first = registry.next_listener(&mut dispatch).unwrap();
        assert_eq!(first.listener, ids[0]);
        assert_eq!(first.phase, EventPhase::AtTarget);
        assert!(registry.next_listener(&mut dispatch).is_none());
    }

    #[test]
    fn stop_propagation_keeps_remaining_listeners_on_current_target() {
        let ids = listener_ids(3);
        let mut registry = EventTargetRegistry::default();
        for listener in &ids[..2] {
            registry
                .add_listener(
                    0_u8,
                    "click",
                    *listener,
                    EventListenerOptions::new(true, false, false),
                )
                .unwrap();
        }
        registry
            .add_listener(
                1_u8,
                "click",
                ids[2],
                EventListenerOptions::new(true, false, false),
            )
            .unwrap();
        let mut dispatch = EventDispatch::new(1_u8, &[0], Event::new("click", true, false));
        assert_eq!(
            registry.next_listener(&mut dispatch).unwrap().listener,
            ids[0]
        );
        dispatch.stop_propagation();
        assert_eq!(
            registry.next_listener(&mut dispatch).unwrap().listener,
            ids[1]
        );
        assert!(registry.next_listener(&mut dispatch).is_none());
    }

    #[test]
    fn stop_propagation_at_target_allows_the_other_target_phase() {
        let ids = listener_ids(2);
        let mut registry = EventTargetRegistry::default();
        registry
            .add_listener(
                1_u8,
                "click",
                ids[0],
                EventListenerOptions::new(true, false, false),
            )
            .unwrap();
        registry
            .add_listener(1_u8, "click", ids[1], EventListenerOptions::default())
            .unwrap();
        let mut dispatch = EventDispatch::new(1_u8, &[], Event::new("click", true, false));
        assert_eq!(
            registry.next_listener(&mut dispatch).unwrap().listener,
            ids[0]
        );
        dispatch.stop_propagation();
        assert_eq!(
            registry.next_listener(&mut dispatch).unwrap().listener,
            ids[1]
        );
        assert!(registry.next_listener(&mut dispatch).is_none());
    }

    #[test]
    fn stop_immediate_propagation_stops_current_and_later_targets() {
        let ids = listener_ids(2);
        let mut registry = EventTargetRegistry::default();
        for listener in &ids {
            registry
                .add_listener(1_u8, "click", *listener, EventListenerOptions::default())
                .unwrap();
        }
        let mut dispatch = EventDispatch::new(1_u8, &[], Event::new("click", true, false));
        assert_eq!(
            registry.next_listener(&mut dispatch).unwrap().listener,
            ids[0]
        );
        dispatch.stop_immediate_propagation();
        assert!(registry.next_listener(&mut dispatch).is_none());
    }

    #[test]
    fn once_listener_is_removed_before_it_is_returned_for_invocation() {
        let listener = listener_ids(1)[0];
        let mut registry = EventTargetRegistry::default();
        registry
            .add_listener(
                1_u8,
                "click",
                listener,
                EventListenerOptions::new(false, true, false),
            )
            .unwrap();
        let mut dispatch = EventDispatch::new(1_u8, &[], Event::new("click", true, false));
        assert_eq!(
            registry.next_listener(&mut dispatch).unwrap().listener,
            listener
        );
        assert_eq!(registry.listener_count(&1), 0);
        assert_eq!(registry.registration_count(), 0);
        assert!(registry.next_listener(&mut dispatch).is_none());
    }

    #[test]
    fn removed_listener_is_skipped_from_an_existing_snapshot() {
        let ids = listener_ids(2);
        let event_type = EventType::from("click");
        let mut registry = EventTargetRegistry::default();
        for listener in &ids {
            registry
                .add_listener(
                    1_u8,
                    event_type.clone(),
                    *listener,
                    EventListenerOptions::default(),
                )
                .unwrap();
        }
        let mut dispatch = EventDispatch::new(1_u8, &[], Event::new("click", true, false));
        assert_eq!(
            registry.next_listener(&mut dispatch).unwrap().listener,
            ids[0]
        );
        assert!(registry.remove_listener(&1, &event_type, ids[1], false));
        assert!(registry.next_listener(&mut dispatch).is_none());
    }

    #[test]
    fn listener_added_to_current_group_waits_for_the_next_dispatch() {
        let ids = listener_ids(2);
        let mut registry = EventTargetRegistry::default();
        registry
            .add_listener(1_u8, "click", ids[0], EventListenerOptions::default())
            .unwrap();
        let mut first = EventDispatch::new(1_u8, &[], Event::new("click", true, false));
        assert_eq!(registry.next_listener(&mut first).unwrap().listener, ids[0]);
        registry
            .add_listener(1_u8, "click", ids[1], EventListenerOptions::default())
            .unwrap();
        assert!(registry.next_listener(&mut first).is_none());

        let mut second = EventDispatch::new(1_u8, &[], Event::new("click", true, false));
        assert_eq!(
            registry.next_listener(&mut second).unwrap().listener,
            ids[0]
        );
        assert_eq!(
            registry.next_listener(&mut second).unwrap().listener,
            ids[1]
        );
    }

    #[test]
    fn passive_listener_cannot_prevent_default() {
        let ids = listener_ids(2);
        let mut registry = EventTargetRegistry::default();
        registry
            .add_listener(
                1_u8,
                "submit",
                ids[0],
                EventListenerOptions::new(false, false, true),
            )
            .unwrap();
        registry
            .add_listener(1_u8, "submit", ids[1], EventListenerOptions::default())
            .unwrap();
        let mut dispatch = EventDispatch::new(1_u8, &[], Event::new("submit", true, true));
        assert_eq!(
            registry.next_listener(&mut dispatch).unwrap().listener,
            ids[0]
        );
        assert!(!dispatch.prevent_default());
        assert!(!dispatch.event().default_prevented());
        assert_eq!(
            registry.next_listener(&mut dispatch).unwrap().listener,
            ids[1]
        );
        assert!(dispatch.prevent_default());
        assert!(dispatch.event().default_prevented());
    }
}
