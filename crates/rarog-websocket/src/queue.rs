use crate::WebSocketMessage;
use std::collections::VecDeque;
use std::fmt;

pub const DEFAULT_MAX_WEBSOCKET_QUEUED_MESSAGES: usize = 64;
pub const DEFAULT_MAX_WEBSOCKET_QUEUED_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebSocketQueueLimits {
    pub max_outbound_messages: usize,
    pub max_outbound_bytes: usize,
    pub max_inbound_messages: usize,
    pub max_inbound_bytes: usize,
}

impl WebSocketQueueLimits {
    pub fn is_valid(self) -> bool {
        self.max_outbound_messages > 0
            && self.max_outbound_bytes > 0
            && self.max_inbound_messages > 0
            && self.max_inbound_bytes > 0
    }
}

impl Default for WebSocketQueueLimits {
    fn default() -> Self {
        Self {
            max_outbound_messages: DEFAULT_MAX_WEBSOCKET_QUEUED_MESSAGES,
            max_outbound_bytes: DEFAULT_MAX_WEBSOCKET_QUEUED_BYTES,
            max_inbound_messages: DEFAULT_MAX_WEBSOCKET_QUEUED_MESSAGES,
            max_inbound_bytes: DEFAULT_MAX_WEBSOCKET_QUEUED_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebSocketQueueErrorKind {
    InvalidLimits,
    OutboundMessageLimitExceeded,
    OutboundByteLimitExceeded,
    InboundMessageLimitExceeded,
    InboundByteLimitExceeded,
    AccountingInvariantViolated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebSocketQueueError {
    pub kind: WebSocketQueueErrorKind,
    pub message: String,
}

impl WebSocketQueueError {
    fn new(kind: WebSocketQueueErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for WebSocketQueueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for WebSocketQueueError {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WebSocketQueueSnapshot {
    outbound_messages: usize,
    outbound_bytes: usize,
    inbound_messages: usize,
    inbound_bytes: usize,
}

impl WebSocketQueueSnapshot {
    pub fn outbound_messages(self) -> usize {
        self.outbound_messages
    }

    pub fn outbound_bytes(self) -> usize {
        self.outbound_bytes
    }

    pub fn inbound_messages(self) -> usize {
        self.inbound_messages
    }

    pub fn inbound_bytes(self) -> usize {
        self.inbound_bytes
    }
}

#[derive(Clone, Debug)]
pub struct WebSocketMessageQueues {
    limits: WebSocketQueueLimits,
    outbound: VecDeque<WebSocketMessage>,
    outbound_bytes: usize,
    inbound: VecDeque<WebSocketMessage>,
    inbound_bytes: usize,
}

impl WebSocketMessageQueues {
    pub fn try_new(limits: WebSocketQueueLimits) -> Result<Self, WebSocketQueueError> {
        if !limits.is_valid() {
            return Err(WebSocketQueueError::new(
                WebSocketQueueErrorKind::InvalidLimits,
                "WebSocket queue limits must all be non-zero",
            ));
        }
        Ok(Self {
            limits,
            outbound: VecDeque::new(),
            outbound_bytes: 0,
            inbound: VecDeque::new(),
            inbound_bytes: 0,
        })
    }

    pub fn limits(&self) -> WebSocketQueueLimits {
        self.limits
    }

    pub fn snapshot(&self) -> WebSocketQueueSnapshot {
        WebSocketQueueSnapshot {
            outbound_messages: self.outbound.len(),
            outbound_bytes: self.outbound_bytes,
            inbound_messages: self.inbound.len(),
            inbound_bytes: self.inbound_bytes,
        }
    }

    pub fn enqueue_outbound(
        &mut self,
        message: WebSocketMessage,
    ) -> Result<(), WebSocketQueueError> {
        if self.outbound.len() >= self.limits.max_outbound_messages {
            return Err(WebSocketQueueError::new(
                WebSocketQueueErrorKind::OutboundMessageLimitExceeded,
                format!(
                    "WebSocket outbound message limit {} reached",
                    self.limits.max_outbound_messages
                ),
            ));
        }
        let next_bytes = checked_enqueue_bytes(
            self.outbound_bytes,
            message.len(),
            self.limits.max_outbound_bytes,
            WebSocketQueueErrorKind::OutboundByteLimitExceeded,
            "outbound",
        )?;
        self.outbound.push_back(message);
        self.outbound_bytes = next_bytes;
        Ok(())
    }

    pub fn outbound_front(&self) -> Option<&WebSocketMessage> {
        self.outbound.front()
    }

    pub fn complete_outbound(&mut self) -> Result<Option<WebSocketMessage>, WebSocketQueueError> {
        let Some(front) = self.outbound.front() else {
            return Ok(None);
        };
        let next_bytes = self
            .outbound_bytes
            .checked_sub(front.len())
            .ok_or_else(|| {
                WebSocketQueueError::new(
                    WebSocketQueueErrorKind::AccountingInvariantViolated,
                    "WebSocket outbound byte accounting underflow",
                )
            })?;
        let message = self.outbound.pop_front().ok_or_else(|| {
            WebSocketQueueError::new(
                WebSocketQueueErrorKind::AccountingInvariantViolated,
                "WebSocket outbound queue changed during completion",
            )
        })?;
        self.outbound_bytes = next_bytes;
        Ok(Some(message))
    }

    pub fn inbound_receive_limit(&self) -> Result<Option<usize>, WebSocketQueueError> {
        if self.inbound.len() >= self.limits.max_inbound_messages {
            return Ok(None);
        }
        let remaining = self
            .limits
            .max_inbound_bytes
            .checked_sub(self.inbound_bytes)
            .ok_or_else(|| {
                WebSocketQueueError::new(
                    WebSocketQueueErrorKind::AccountingInvariantViolated,
                    "WebSocket inbound byte accounting exceeds its configured limit",
                )
            })?;
        Ok(Some(remaining))
    }

    pub fn enqueue_inbound(
        &mut self,
        message: WebSocketMessage,
    ) -> Result<(), WebSocketQueueError> {
        if self.inbound.len() >= self.limits.max_inbound_messages {
            return Err(WebSocketQueueError::new(
                WebSocketQueueErrorKind::InboundMessageLimitExceeded,
                format!(
                    "WebSocket inbound message limit {} reached",
                    self.limits.max_inbound_messages
                ),
            ));
        }
        let next_bytes = checked_enqueue_bytes(
            self.inbound_bytes,
            message.len(),
            self.limits.max_inbound_bytes,
            WebSocketQueueErrorKind::InboundByteLimitExceeded,
            "inbound",
        )?;
        self.inbound.push_back(message);
        self.inbound_bytes = next_bytes;
        Ok(())
    }

    pub fn take_inbound(&mut self) -> Result<Option<WebSocketMessage>, WebSocketQueueError> {
        let Some(front) = self.inbound.front() else {
            return Ok(None);
        };
        let next_bytes = self.inbound_bytes.checked_sub(front.len()).ok_or_else(|| {
            WebSocketQueueError::new(
                WebSocketQueueErrorKind::AccountingInvariantViolated,
                "WebSocket inbound byte accounting underflow",
            )
        })?;
        let message = self.inbound.pop_front().ok_or_else(|| {
            WebSocketQueueError::new(
                WebSocketQueueErrorKind::AccountingInvariantViolated,
                "WebSocket inbound queue changed during dequeue",
            )
        })?;
        self.inbound_bytes = next_bytes;
        Ok(Some(message))
    }

    pub fn clear(&mut self) {
        self.outbound.clear();
        self.outbound_bytes = 0;
        self.inbound.clear();
        self.inbound_bytes = 0;
    }
}

impl Default for WebSocketMessageQueues {
    fn default() -> Self {
        let limits = WebSocketQueueLimits::default();
        Self {
            limits,
            outbound: VecDeque::new(),
            outbound_bytes: 0,
            inbound: VecDeque::new(),
            inbound_bytes: 0,
        }
    }
}

fn checked_enqueue_bytes(
    current: usize,
    added: usize,
    limit: usize,
    limit_kind: WebSocketQueueErrorKind,
    direction: &str,
) -> Result<usize, WebSocketQueueError> {
    let next = current.checked_add(added).ok_or_else(|| {
        WebSocketQueueError::new(
            WebSocketQueueErrorKind::AccountingInvariantViolated,
            format!("WebSocket {direction} byte accounting overflow"),
        )
    })?;
    if next > limit {
        return Err(WebSocketQueueError::new(
            limit_kind,
            format!("WebSocket {direction} queue would retain {next} bytes; limit is {limit}"),
        ));
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WebSocketLimits;

    fn message(text: &str) -> WebSocketMessage {
        WebSocketMessage::text(text, WebSocketLimits::default()).unwrap()
    }

    fn limits() -> WebSocketQueueLimits {
        WebSocketQueueLimits {
            max_outbound_messages: 2,
            max_outbound_bytes: 5,
            max_inbound_messages: 2,
            max_inbound_bytes: 5,
        }
    }

    #[test]
    fn invalid_limits_are_rejected() {
        let mut invalid = limits();
        invalid.max_inbound_bytes = 0;
        assert_eq!(
            WebSocketMessageQueues::try_new(invalid).unwrap_err().kind,
            WebSocketQueueErrorKind::InvalidLimits
        );
    }

    #[test]
    fn outbound_queue_is_fifo_and_recovers_exact_capacity() {
        let mut queues = WebSocketMessageQueues::try_new(limits()).unwrap();
        queues.enqueue_outbound(message("ab")).unwrap();
        queues.enqueue_outbound(message("cde")).unwrap();
        assert_eq!(queues.snapshot().outbound_messages(), 2);
        assert_eq!(queues.snapshot().outbound_bytes(), 5);
        assert_eq!(queues.outbound_front().unwrap().as_text(), Some("ab"));
        assert_eq!(
            queues.enqueue_outbound(message("x")).unwrap_err().kind,
            WebSocketQueueErrorKind::OutboundMessageLimitExceeded
        );
        assert_eq!(
            queues.complete_outbound().unwrap().unwrap().as_text(),
            Some("ab")
        );
        assert_eq!(queues.snapshot().outbound_bytes(), 3);
        queues.enqueue_outbound(message("xy")).unwrap();
        assert_eq!(queues.snapshot().outbound_bytes(), 5);
    }

    #[test]
    fn outbound_byte_rejection_does_not_mutate_accounting() {
        let mut queues = WebSocketMessageQueues::try_new(limits()).unwrap();
        queues.enqueue_outbound(message("abcd")).unwrap();
        assert_eq!(
            queues.enqueue_outbound(message("xy")).unwrap_err().kind,
            WebSocketQueueErrorKind::OutboundByteLimitExceeded
        );
        assert_eq!(queues.snapshot().outbound_messages(), 1);
        assert_eq!(queues.snapshot().outbound_bytes(), 4);
        assert_eq!(queues.outbound_front().unwrap().as_text(), Some("abcd"));
    }

    #[test]
    fn inbound_queue_is_fifo_and_exposes_remaining_receive_bound() {
        let mut queues = WebSocketMessageQueues::try_new(limits()).unwrap();
        assert_eq!(queues.inbound_receive_limit().unwrap(), Some(5));
        queues.enqueue_inbound(message("abc")).unwrap();
        assert_eq!(queues.inbound_receive_limit().unwrap(), Some(2));
        queues.enqueue_inbound(message("de")).unwrap();
        assert_eq!(queues.inbound_receive_limit().unwrap(), None);
        assert_eq!(queues.snapshot().inbound_messages(), 2);
        assert_eq!(queues.snapshot().inbound_bytes(), 5);
        assert_eq!(
            queues.take_inbound().unwrap().unwrap().as_text(),
            Some("abc")
        );
        assert_eq!(queues.inbound_receive_limit().unwrap(), Some(3));
    }

    #[test]
    fn byte_full_inbound_queue_can_still_accept_empty_message_when_count_allows() {
        let mut custom = limits();
        custom.max_inbound_messages = 2;
        custom.max_inbound_bytes = 1;
        let mut queues = WebSocketMessageQueues::try_new(custom).unwrap();
        queues.enqueue_inbound(message("x")).unwrap();
        assert_eq!(queues.inbound_receive_limit().unwrap(), Some(0));
        queues.enqueue_inbound(message("")).unwrap();
        assert_eq!(queues.snapshot().inbound_messages(), 2);
        assert_eq!(queues.snapshot().inbound_bytes(), 1);
        assert_eq!(queues.inbound_receive_limit().unwrap(), None);
    }

    #[test]
    fn inbound_byte_rejection_does_not_mutate_accounting() {
        let mut queues = WebSocketMessageQueues::try_new(limits()).unwrap();
        queues.enqueue_inbound(message("abcd")).unwrap();
        assert_eq!(
            queues.enqueue_inbound(message("xy")).unwrap_err().kind,
            WebSocketQueueErrorKind::InboundByteLimitExceeded
        );
        assert_eq!(queues.snapshot().inbound_messages(), 1);
        assert_eq!(queues.snapshot().inbound_bytes(), 4);
    }

    #[test]
    fn clear_releases_both_direction_budgets() {
        let mut queues = WebSocketMessageQueues::try_new(limits()).unwrap();
        queues.enqueue_outbound(message("abc")).unwrap();
        queues.enqueue_inbound(message("de")).unwrap();
        queues.clear();
        assert_eq!(queues.snapshot(), WebSocketQueueSnapshot::default());
        assert_eq!(queues.inbound_receive_limit().unwrap(), Some(5));
        queues.enqueue_outbound(message("12345")).unwrap();
        queues.enqueue_inbound(message("12345")).unwrap();
    }
}
