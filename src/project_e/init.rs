//! Project E startup — wires all bridge components at app startup.
//!
//! Call `init_project_e()` from `main.rs` **after** `AppBuilder::build_all()`
//! returns `AppComponents`. It reads `config.project_e`, creates the event
//! bus, wires the knowledge bridge, LLM bridge, forge runner, and cognitive
//! cycle runner, then spawns the background tasks.
//!
//! Returns a `ProjectEHandle` that the caller should store. When the process
//! wants to shut down, set `shutdown_tx` to `true` and await `runner_handle`.
//!
//! # Example (in `main.rs`)
//!
//! ```ignore
//! #[cfg(feature = "project_e")]
//! let project_e_handle = {
//!     let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
//!     crate::project_e::init::init_project_e(&components, shutdown_rx)
//! };
//! // … on shutdown:
//! #[cfg(feature = "project_e")]
//! {
//!     let _ = project_e_handle.shutdown_tx.send(true);
//!     project_e_handle.runner_handle.await.ok();
//! }
//! ```

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tracing::{info, warn};

use project_e_event_bus::{EventBus, InMemoryEventBus};

use crate::app::AppComponents;
use crate::project_e::cycle_runner::CognitiveCycleRunner;
use crate::project_e::forge_runner::ForgeSandboxRunner;
use crate::project_e::hook_bridge::subscribe_cognitive_events;
use crate::project_e::knowledge_bridge::WorkspaceKnowledgeStore;
use crate::project_e::llm_bridge::IronclawLlmBridge;

/// Live handles for the Project E background tasks.
///
/// Store this in your application's main struct. On shutdown:
/// 1. Send `true` to `shutdown_tx`.
/// 2. Await both `JoinHandle`s.
pub struct ProjectEHandle {
    /// Send `true` to trigger graceful shutdown of all background tasks.
    pub shutdown_tx: watch::Sender<bool>,
    /// Background task running the cognitive cycle.
    pub runner_handle: tokio::task::JoinHandle<()>,
    /// Background task forwarding cognitive events to tracing.
    pub event_subscriber_handle: tokio::task::JoinHandle<()>,
    /// Shared event bus (usable for ad-hoc event emission).
    pub event_bus: Arc<EventBus>,
}

/// Initialise the Project E cognitive architecture layer.
///
/// Returns `None` when:
/// - The `project_e` feature flag is compiled in but `config.project_e.enabled` is `false`.
/// - The workspace is not available (required for the knowledge bridge).
pub fn init_project_e(
    components: &AppComponents,
    shutdown_rx: watch::Receiver<bool>,
) -> Option<ProjectEHandle> {
    let pe_config = &components.config.project_e;

    if !pe_config.enabled {
        info!("project_e: disabled (PROJECT_E_ENABLED=false)");
        return None;
    }

    let workspace = match &components.workspace {
        Some(ws) => Arc::clone(ws),
        None => {
            warn!("project_e: workspace not available — cognitive layer disabled");
            return None;
        }
    };

    // ── Event Bus ────────────────────────────────────────────────────────
    let transport = Arc::new(InMemoryEventBus::new(pe_config.event_bus_capacity));
    let event_bus = Arc::new(EventBus::new(transport));

    // ── Knowledge Bridge ─────────────────────────────────────────────────
    let knowledge = WorkspaceKnowledgeStore::new(workspace);

    // ── LLM Bridge ───────────────────────────────────────────────────────
    let inner_config = &pe_config.inner;
    let mut cb_config = inner_config.llm.circuit_breaker.clone();
    // Project E's breaker uses a higher threshold than IronClaw's inner
    // CircuitBreakerProvider (default 5) so the two don't interfere.
    // We bump to at least 8 unless the user configured a higher value.
    if cb_config.failure_threshold < 8 {
        cb_config.failure_threshold = 8;
    }

    let llm_bridge = Arc::new(IronclawLlmBridge::new(
        Arc::clone(&components.llm),
        Arc::clone(&components.cost_guard),
        cb_config,
        pe_config.max_llm_calls_per_cycle,
    ));

    // ── Forge Runner ─────────────────────────────────────────────────────
    let forge = ForgeSandboxRunner::new(
        inner_config.forge.clone(),
        Arc::clone(&event_bus),
        knowledge.clone(),
    );

    // ── Cognitive Cycle Runner ────────────────────────────────────────────
    let heartbeat_interval = Duration::from_secs(
        std::env::var("HEARTBEAT_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(1800), // default 30 min
    );

    let runner = CognitiveCycleRunner::new(
        Arc::clone(&event_bus),
        knowledge,
        llm_bridge,
        forge,
        heartbeat_interval,
        pe_config.instance.clone(),
        inner_config.perception.clone(),
        inner_config.analysis.clone(),
        inner_config.metacognition.clone(),
        inner_config.limits.clone(),
    );

    // ── Spawn background tasks ─────────────────────────────────────────────
    let (shutdown_tx, shutdown_rx2) = watch::channel(false);

    // Forward the external shutdown signal to our internal channel.
    let shutdown_tx_clone = shutdown_tx.clone();
    let mut external_rx = shutdown_rx;
    tokio::spawn(async move {
        let _ = external_rx.changed().await;
        if *external_rx.borrow() {
            let _ = shutdown_tx_clone.send(true);
        }
    });

    let event_subscriber_handle =
        subscribe_cognitive_events(Arc::clone(&event_bus), shutdown_rx2.clone());

    let runner_handle = runner.spawn(shutdown_rx2);

    info!(
        instance = %pe_config.instance,
        heartbeat_secs = heartbeat_interval.as_secs(),
        max_llm_calls = pe_config.max_llm_calls_per_cycle,
        "project_e: cognitive architecture layer started"
    );

    Some(ProjectEHandle {
        shutdown_tx,
        runner_handle,
        event_subscriber_handle,
        event_bus,
    })
}
