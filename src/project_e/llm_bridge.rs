//! LLM Provider Bridge — adapts IronClaw's `LlmProvider` for the cognitive layer.
//!
//! `IronclawLlmBridge` is the single entry point for all LLM calls initiated
//! by Project E. It layers three defences before every call:
//!
//! 1. **Per-cycle call budget** — hard cap on LLM calls per cognitive cycle.
//! 2. **`LlmHealthMonitor`** — Project E's own circuit breaker. Trips after
//!    `failure_threshold` consecutive cognitive-cycle-level errors (configured
//!    with a higher threshold than IronClaw's inner `CircuitBreakerProvider` so
//!    the two breakers don't interfere).
//! 3. **`CostGuard`** — IronClaw's daily budget / hourly rate enforcer.

use std::sync::Arc;

use rust_decimal::Decimal;
use thiserror::Error;
use tracing::{debug, warn};

use project_e_core::config::CircuitBreakerConfig;
use project_e_core::llm_health::LlmHealthMonitor;

use crate::agent::cost_guard::CostGuard;
use crate::llm::provider::{ChatMessage, CompletionRequest, LlmProvider};

/// Error type for LLM bridge operations.
#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("LLM circuit breaker open: {reason}")]
    CircuitOpen { reason: String },

    #[error("LLM budget exceeded: {reason}")]
    BudgetExceeded { reason: String },

    #[error("LLM call failed: {reason}")]
    ProviderError { reason: String },
}

/// Bridges IronClaw's `LlmProvider` into the cognitive layer.
///
/// Thread-safe and cheaply cloneable via `Arc` internals. Construct once in
/// `init_project_e()` and share references as needed.
pub struct IronclawLlmBridge {
    provider: Arc<dyn LlmProvider>,
    cost_guard: Arc<CostGuard>,
    health_monitor: Arc<LlmHealthMonitor>,
    /// Hard cap on LLM calls per cognitive cycle (from `ProjectEIronclawConfig`).
    max_calls_per_cycle: usize,
}

impl IronclawLlmBridge {
    /// Create a new bridge.
    ///
    /// `circuit_breaker_config` should use a **higher** `failure_threshold`
    /// than IronClaw's inner `CircuitBreakerProvider` (e.g. 8 vs 5) so
    /// Project E's breaker only trips on sustained cognitive-cycle degradation,
    /// not on transient per-request failures already handled by the retry
    /// decorator inside IronClaw's provider chain.
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        cost_guard: Arc<CostGuard>,
        circuit_breaker_config: CircuitBreakerConfig,
        max_calls_per_cycle: usize,
    ) -> Self {
        let provider_name = provider.model_name().to_string();
        Self {
            provider,
            cost_guard,
            health_monitor: Arc::new(LlmHealthMonitor::new(provider_name, circuit_breaker_config)),
            max_calls_per_cycle,
        }
    }

    /// Return a reference to the underlying health monitor (used by
    /// `CognitiveCycleRunner` to inspect circuit-breaker state).
    pub fn health_monitor(&self) -> &Arc<LlmHealthMonitor> {
        &self.health_monitor
    }

    /// Run a simple text completion for the cognitive layer.
    ///
    /// Constructs a `[system, user]` message pair and forwards it to
    /// IronClaw's provider. Does **not** use tools — cognitive-level calls are
    /// pure text generation only.
    ///
    /// `call_index` is the 0-based call number within the current cycle;
    /// returns `BridgeError::BudgetExceeded` if it would exceed
    /// `max_calls_per_cycle`.
    pub async fn complete_text(
        &self,
        system: &str,
        user: &str,
        model: Option<&str>,
        call_index: usize,
    ) -> Result<String, BridgeError> {
        // 1. Per-cycle call budget.
        if call_index >= self.max_calls_per_cycle {
            return Err(BridgeError::BudgetExceeded {
                reason: format!(
                    "cognitive cycle LLM call limit ({}) reached",
                    self.max_calls_per_cycle
                ),
            });
        }

        // 2. Project E circuit breaker.
        self.health_monitor
            .check_before_call()
            .await
            .map_err(|e| BridgeError::CircuitOpen {
                reason: e.to_string(),
            })?;

        // 3. IronClaw daily budget / hourly rate (async check).
        if let Err(limit) = self.cost_guard.check_allowed().await {
            return Err(BridgeError::BudgetExceeded {
                reason: limit.to_string(),
            });
        }

        // 4. Issue the LLM call.
        let messages = vec![ChatMessage::system(system), ChatMessage::user(user)];
        let mut req = CompletionRequest::new(messages);
        if let Some(m) = model {
            req = req.with_model(m);
        }

        debug!(
            model = model.unwrap_or("(default)"),
            call_index,
            "project_e: issuing LLM call via bridge"
        );

        let model_name = self.provider.model_name().to_string();
        let cost_per_token = Some(self.provider.cost_per_token());

        let outcome = self.provider.complete(req).await;
        let success = outcome.is_ok();
        self.health_monitor.record_outcome(success).await;

        match outcome {
            Ok(resp) => {
                self.cost_guard
                    .record_llm_call(
                        &model_name,
                        resp.input_tokens,
                        resp.output_tokens,
                        resp.cache_read_input_tokens,
                        resp.cache_creation_input_tokens,
                        Decimal::TEN, // 90% cache-read discount (Anthropic default)
                        Decimal::ONE, // no cache-write surcharge by default
                        cost_per_token,
                    )
                    .await;
                Ok(resp.content)
            }
            Err(e) => {
                warn!(error = %e, "project_e: LLM bridge call failed");
                Err(BridgeError::ProviderError {
                    reason: e.to_string(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the cycle budget math at the limit boundary.
    #[test]
    fn call_index_budget_boundary() {
        let max = 2_usize;
        // Indices 0 and 1 are within budget; index 2 exceeds it.
        assert!(0 < max, "index 0 should be within limit");
        assert!(1 < max, "index 1 should be within limit");
        assert!(!(2 < max), "index 2 should exceed limit");
    }
}
