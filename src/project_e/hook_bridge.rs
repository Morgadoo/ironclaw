//! Event Bus → Hook Bridge — forwards cognitive events to IronClaw logging.
//!
//! `subscribe_cognitive_events()` spawns a background tokio task that:
//! 1. Subscribes to the wildcard `>` topic on the in-process event bus
//! 2. Deserialises each message as an `EventEnvelope`
//! 3. Emits structured `tracing` events at appropriate severity levels
//!
//! This approach is intentionally narrow: we don't funnel into IronClaw's
//! `HookRegistry` because hooks are typed to user-message events, not
//! cognitive-cycle events. The observability layer (logs, Langfuse, etc.)
//! picks up everything via the tracing subscriber.
//!
//! The task runs until the shutdown channel fires, then exits cleanly.

use std::sync::Arc;

use tokio::sync::watch;
use tracing::{debug, error, info, warn};

use project_e_event_bus::{EventBus, protocol::EventEnvelope};

/// Spawn a background task that subscribes to all cognitive events and logs them.
///
/// Returns the `JoinHandle` — the caller should store it and await on shutdown.
///
/// The task exits when:
/// - `shutdown` receiver fires (value changes to `true`), OR
/// - the event bus transport is closed (no more senders)
pub fn subscribe_cognitive_events(
    bus: Arc<EventBus>,
    mut shutdown: watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Subscribe to all topics (wildcard).
        let mut rx = match bus.transport().subscribe(">").await {
            Ok(rx) => rx,
            Err(e) => {
                error!(error = %e, "project_e: failed to subscribe to event bus");
                return;
            }
        };

        info!("project_e: cognitive event subscriber running");

        loop {
            tokio::select! {
                biased;

                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        info!("project_e: event subscriber shutting down");
                        break;
                    }
                }

                msg = rx.recv() => {
                    match msg {
                        Ok(payload) => log_payload(&payload),
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            warn!(
                                missed = n,
                                "project_e: event subscriber lagged — some cognitive events missed"
                            );
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            info!("project_e: event bus closed — subscriber exiting");
                            break;
                        }
                    }
                }
            }
        }
    })
}

/// Deserialise a raw bus payload and emit a tracing event at the right level.
fn log_payload(payload: &[u8]) {
    let envelope: EventEnvelope = match serde_json::from_slice(payload) {
        Ok(e) => e,
        Err(e) => {
            warn!(error = %e, "project_e: could not deserialise event envelope");
            return;
        }
    };

    use project_e_event_bus::protocol::CognitiveEvent as CE;
    match &envelope.payload {
        CE::CycleStarted { cycle_id, instance } => {
            info!(
                cycle_id = %cycle_id,
                instance,
                "project_e: cognitive cycle started"
            );
        }
        CE::CycleClosed { cycle_id, duration_secs } => {
            info!(
                cycle_id = %cycle_id,
                duration_secs,
                "project_e: cognitive cycle closed"
            );
        }
        CE::ActionPlanReady { cycle_id, plan_id, actions_count } => {
            info!(
                cycle_id = %cycle_id,
                plan_id = %plan_id,
                actions = actions_count,
                "project_e: action plan ready"
            );
        }
        CE::ForgeStarted { cycle_id, gap_id, module_name } => {
            info!(
                cycle_id = %cycle_id,
                gap_id = %gap_id,
                module = module_name,
                "project_e: forge pipeline started"
            );
        }
        CE::ModulePublished { cycle_id, module_id, module_name } => {
            info!(
                cycle_id = %cycle_id,
                module_id = %module_id,
                module = module_name,
                "project_e: module published"
            );
        }
        CE::ModuleRejected { cycle_id, gap_id, reason, attempts } => {
            warn!(
                cycle_id = %cycle_id,
                gap_id = %gap_id,
                reason,
                attempts,
                "project_e: module rejected after all retries"
            );
        }
        CE::GateFail { cycle_id, module_id, gate, error } => {
            warn!(
                cycle_id = %cycle_id,
                module_id = %module_id,
                gate = ?gate,
                error,
                "project_e: forge gate failed"
            );
        }
        CE::SupervisorIntervention { pattern, intervention } => {
            warn!(
                pattern = ?pattern,
                intervention = ?intervention,
                "project_e: supervisor intervention applied"
            );
        }
        CE::HealthAlert { indicator, value, threshold } => {
            warn!(
                indicator = ?indicator,
                value,
                threshold,
                "project_e: health alert triggered"
            );
        }
        CE::CircuitBreakerOpen { provider, failures } => {
            warn!(
                provider,
                failures,
                "project_e: LLM circuit breaker opened"
            );
        }
        CE::CircuitBreakerClosed { provider } => {
            info!(provider, "project_e: LLM circuit breaker closed (recovered)");
        }
        other => {
            debug!(topic = envelope.topic, event = ?other, "project_e: cognitive event");
        }
    }
}
