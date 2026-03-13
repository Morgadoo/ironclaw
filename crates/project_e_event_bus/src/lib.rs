//! Project E Event Bus — typed NATS JetStream event protocol.
//!
//! Defines the `CognitiveEvent` enum covering every cognitive-phase
//! transition, and the `EventBus` abstraction for publishing/subscribing.

pub mod protocol;
pub mod topics;

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::broadcast;
use tracing::debug;

use crate::protocol::{CognitiveEvent, EventEnvelope};
use project_e_core::error::{ProjectEError, Result};

/// Abstraction over the event bus transport (NATS JetStream in production,
/// in-memory broadcast for testing).
#[async_trait]
pub trait EventBusTransport: Send + Sync {
    async fn publish(&self, topic: &str, payload: &[u8]) -> Result<()>;
    async fn subscribe(&self, topic: &str) -> Result<broadcast::Receiver<Vec<u8>>>;
}

/// In-memory event bus for development and testing.
/// Production implementations will use NATS JetStream.
pub struct InMemoryEventBus {
    sender: broadcast::Sender<(String, Vec<u8>)>,
}

impl InMemoryEventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }
}

#[async_trait]
impl EventBusTransport for InMemoryEventBus {
    async fn publish(&self, topic: &str, payload: &[u8]) -> Result<()> {
        // Ignore "no receivers" errors — this is expected when no one
        // has subscribed yet (common in tests and during startup).
        let _ = self.sender.send((topic.to_string(), payload.to_vec()));
        Ok(())
    }

    async fn subscribe(&self, _topic: &str) -> Result<broadcast::Receiver<Vec<u8>>> {
        // Simplified: returns a receiver that receives all messages.
        // A production NATS implementation would filter by topic.
        let rx = self.sender.subscribe();
        // Convert (topic, payload) to just payload for the subscriber
        let (tx, new_rx) = broadcast::channel(256);
        let topic = _topic.to_string();
        tokio::spawn(async move {
            let mut rx = rx;
            loop {
                match rx.recv().await {
                    Ok((msg_topic, payload)) => {
                        if topic_matches(&topic, &msg_topic) {
                            let _ = tx.send(payload);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("event bus subscriber lagged by {n} messages");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        Ok(new_rx)
    }
}

/// Simple wildcard topic matching (supports trailing `*` and `>`).
fn topic_matches(pattern: &str, topic: &str) -> bool {
    if pattern == topic {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix(".*") {
        return topic.starts_with(prefix) && topic[prefix.len()..].starts_with('.');
    }
    if let Some(prefix) = pattern.strip_suffix(".>") {
        return topic.starts_with(prefix) && topic[prefix.len()..].starts_with('.');
    }
    if pattern == ">" {
        return true;
    }
    false
}

/// High-level event bus that serializes/deserializes typed cognitive events.
pub struct EventBus {
    transport: Arc<dyn EventBusTransport>,
}

impl EventBus {
    pub fn new(transport: Arc<dyn EventBusTransport>) -> Self {
        Self { transport }
    }

    /// Create an in-memory event bus (for testing).
    pub fn in_memory(capacity: usize) -> Self {
        Self {
            transport: Arc::new(InMemoryEventBus::new(capacity)),
        }
    }

    /// Publish a typed cognitive event.
    pub async fn emit(&self, event: CognitiveEvent) -> Result<()> {
        let topic = event.topic();
        let envelope = EventEnvelope::wrap(event);
        let payload = serde_json::to_vec(&envelope).map_err(|e| ProjectEError::EventBus {
            reason: format!("serialization failed: {e}"),
        })?;
        debug!(topic, "emitting cognitive event");
        self.transport.publish(topic, &payload).await
    }

    /// Get a reference to the underlying transport for custom subscriptions.
    pub fn transport(&self) -> &Arc<dyn EventBusTransport> {
        &self.transport
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_matching() {
        assert!(topic_matches(
            "project_e.system.cycle_started",
            "project_e.system.cycle_started"
        ));
        assert!(topic_matches(
            "project_e.perception.*",
            "project_e.perception.complete"
        ));
        assert!(!topic_matches(
            "project_e.perception.*",
            "project_e.analysis.ready"
        ));
        assert!(topic_matches(">", "anything.goes.here"));
    }

    #[tokio::test]
    async fn emit_and_receive() {
        let bus = EventBus::in_memory(64);
        let event = CognitiveEvent::CycleStarted {
            cycle_id: uuid::Uuid::new_v4(),
            instance: "test".to_string(),
        };
        // Just verify emit doesn't error
        bus.emit(event).await.unwrap();
    }
}
