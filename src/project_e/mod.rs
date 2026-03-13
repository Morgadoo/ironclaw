//! Project E cognitive architecture — IronClaw integration layer.
//!
//! This module bridges IronClaw's infrastructure (LLM, tools, hooks, workspace,
//! sandbox, heartbeat) with Project E's ten-crate cognitive architecture.
//!
//! # Module Map
//!
//! | File | Role |
//! |------|------|
//! | `llm_bridge.rs`       | `IronclawLlmBridge` — adapts `LlmProvider` to the cognitive layer with `CostGuard` + circuit breaker |
//! | `module_adapter.rs`   | `ModuleToolAdapter<M>` — adapts any `Module` to IronClaw's `Tool` trait |
//! | `hook_bridge.rs`      | `subscribe_cognitive_events()` — forwards bus events to `tracing` |
//! | `knowledge_bridge.rs` | `WorkspaceKnowledgeStore` — routes knowledge through `Workspace` |
//! | `cycle_runner.rs`     | `CognitiveCycleRunner` — independent tokio task; fires on heartbeat |
//! | `forge_runner.rs`     | `ForgeSandboxRunner` — runs the six-gate forge pipeline |
//! | `init.rs`             | `init_project_e()` — wires everything together at app startup |
//!
//! # Feature gate
//!
//! The entire module is compiled only when `--features project_e` is set.
//! IronClaw has zero compile-time or runtime cost when the feature is off.
//!
//! # Usage (in `main.rs`)
//!
//! ```ignore
//! #[cfg(feature = "project_e")]
//! let _project_e = {
//!     let (tx, rx) = tokio::sync::watch::channel(false);
//!     crate::project_e::init::init_project_e(&components, rx)
//! };
//! ```

pub mod cycle_runner;
pub mod forge_runner;
pub mod hook_bridge;
pub mod init;
pub mod knowledge_bridge;
pub mod llm_bridge;
pub mod module_adapter;
