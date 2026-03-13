//! LLM Health Monitor — circuit breaker for LLM provider resilience.
//!
//! Implements the Closed → Open → HalfOpen → Closed state machine.
//! Called by the CycleOrchestrator before each LLM-dependent phase.

use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::config::CircuitBreakerConfig;
use crate::error::{ProjectEError, Result};

/// Circuit breaker state machine.
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    /// Healthy — requests pass through.
    Closed,
    /// Failing — requests are rejected immediately.
    Open { opened_at: Instant },
    /// Testing recovery with a single probe request.
    HalfOpen,
}

/// Monitors LLM provider health and implements circuit breaker pattern.
pub struct LlmHealthMonitor {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    config: CircuitBreakerConfig,
    provider_name: String,
}

impl LlmHealthMonitor {
    pub fn new(provider_name: String, config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            config,
            provider_name,
        }
    }

    /// Check whether an LLM call is allowed. Returns `Err(CircuitOpen)` if
    /// the circuit is open and cooldown has not elapsed.
    pub async fn check_before_call(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let state = self.state.read().await;
        match &*state {
            CircuitState::Closed => Ok(()),
            CircuitState::HalfOpen => {
                // Allow the probe request through
                Ok(())
            }
            CircuitState::Open { opened_at } => {
                if opened_at.elapsed() >= self.config.cooldown {
                    // Cooldown elapsed — transition to HalfOpen
                    drop(state);
                    let mut state_w = self.state.write().await;
                    *state_w = CircuitState::HalfOpen;
                    info!(
                        provider = %self.provider_name,
                        "circuit breaker transitioning to half-open"
                    );
                    Ok(())
                } else {
                    Err(ProjectEError::CircuitOpen {
                        provider: self.provider_name.clone(),
                    })
                }
            }
        }
    }

    /// Record the outcome of an LLM call. Updates failure count and
    /// transitions the circuit breaker state as needed.
    pub async fn record_outcome(&self, success: bool) {
        if !self.config.enabled {
            return;
        }

        if success {
            let mut count = self.failure_count.write().await;
            *count = 0;

            let mut state = self.state.write().await;
            if *state != CircuitState::Closed {
                info!(
                    provider = %self.provider_name,
                    "circuit breaker closing — provider recovered"
                );
                *state = CircuitState::Closed;
            }
        } else {
            let mut count = self.failure_count.write().await;
            *count += 1;

            if *count >= self.config.failure_threshold {
                let mut state = self.state.write().await;
                if *state == CircuitState::Closed || *state == CircuitState::HalfOpen {
                    warn!(
                        provider = %self.provider_name,
                        failures = *count,
                        "circuit breaker opening — too many failures"
                    );
                    *state = CircuitState::Open {
                        opened_at: Instant::now(),
                    };
                }
            }
        }
    }

    /// Returns the current circuit state.
    pub async fn current_state(&self) -> CircuitState {
        self.state.read().await.clone()
    }

    /// Returns the current failure count.
    pub async fn failure_count(&self) -> u32 {
        *self.failure_count.read().await
    }

    /// Returns the provider name this monitor tracks.
    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }

    /// Run a background probe loop that periodically checks provider health.
    /// Intended to be spawned as a tokio task.
    pub async fn run_probe_loop(self: Arc<Self>, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        let interval = self.config.probe_interval;
        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {
                    let state = self.current_state().await;
                    if matches!(state, CircuitState::Open { .. }) {
                        // Check if cooldown has elapsed; if so, the next
                        // check_before_call will transition to HalfOpen
                        info!(
                            provider = %self.provider_name,
                            "probe loop: circuit still open, awaiting cooldown"
                        );
                    }
                }
                _ = shutdown.changed() => {
                    info!(
                        provider = %self.provider_name,
                        "probe loop shutting down"
                    );
                    break;
                }
            }
        }
    }
}

/// Per-phase model router using ModelCapabilities for cost estimation.
pub struct PhaseModelRouter {
    routing: crate::config::PhaseRoutingConfig,
}

impl PhaseModelRouter {
    pub fn new(routing: crate::config::PhaseRoutingConfig) -> Self {
        Self { routing }
    }

    /// Returns the model name assigned to a given cognitive phase.
    pub fn model_for_phase(&self, phase: crate::types::CognitivePhase) -> &str {
        use crate::types::CognitivePhase;
        match phase {
            CognitivePhase::Perception => &self.routing.perception,
            CognitivePhase::Interpretation => &self.routing.interpretation,
            CognitivePhase::Deliberation => &self.routing.deliberation,
            CognitivePhase::Action => &self.routing.action,
            CognitivePhase::Evaluation => &self.routing.evaluation,
            CognitivePhase::MetaAdaptation => &self.routing.meta_adaptation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CircuitBreakerConfig;
    use std::time::Duration;

    fn test_config() -> CircuitBreakerConfig {
        CircuitBreakerConfig {
            enabled: true,
            probe_interval: Duration::from_millis(100),
            failure_threshold: 3,
            cooldown: Duration::from_millis(200),
            probe_timeout: Duration::from_millis(50),
        }
    }

    #[tokio::test]
    async fn circuit_starts_closed() {
        let monitor = LlmHealthMonitor::new("test".to_string(), test_config());
        assert_eq!(monitor.current_state().await, CircuitState::Closed);
        assert!(monitor.check_before_call().await.is_ok());
    }

    #[tokio::test]
    async fn circuit_opens_after_threshold_failures() {
        let monitor = LlmHealthMonitor::new("test".to_string(), test_config());

        for _ in 0..3 {
            monitor.record_outcome(false).await;
        }

        assert!(matches!(
            monitor.current_state().await,
            CircuitState::Open { .. }
        ));
        assert!(monitor.check_before_call().await.is_err());
    }

    #[tokio::test]
    async fn circuit_recovers_after_cooldown() {
        let monitor = LlmHealthMonitor::new("test".to_string(), test_config());

        for _ in 0..3 {
            monitor.record_outcome(false).await;
        }
        assert!(monitor.check_before_call().await.is_err());

        // Wait for cooldown
        tokio::time::sleep(Duration::from_millis(250)).await;

        // Should transition to HalfOpen
        assert!(monitor.check_before_call().await.is_ok());
        assert_eq!(monitor.current_state().await, CircuitState::HalfOpen);

        // Record success to close
        monitor.record_outcome(true).await;
        assert_eq!(monitor.current_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn success_resets_failure_count() {
        let monitor = LlmHealthMonitor::new("test".to_string(), test_config());

        monitor.record_outcome(false).await;
        monitor.record_outcome(false).await;
        assert_eq!(monitor.failure_count().await, 2);

        monitor.record_outcome(true).await;
        assert_eq!(monitor.failure_count().await, 0);
    }

    #[tokio::test]
    async fn disabled_circuit_always_allows() {
        let config = CircuitBreakerConfig {
            enabled: false,
            ..test_config()
        };
        let monitor = LlmHealthMonitor::new("test".to_string(), config);

        for _ in 0..10 {
            monitor.record_outcome(false).await;
        }
        assert!(monitor.check_before_call().await.is_ok());
    }

    #[test]
    fn phase_model_routing() {
        use crate::config::PhaseRoutingConfig;
        use crate::types::CognitivePhase;

        let router = PhaseModelRouter::new(PhaseRoutingConfig::default());
        assert_eq!(
            router.model_for_phase(CognitivePhase::Perception),
            "model_for_analysis"
        );
        assert_eq!(
            router.model_for_phase(CognitivePhase::Action),
            "model_for_generation"
        );
    }
}
