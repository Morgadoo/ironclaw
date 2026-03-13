//! Forge Pipeline Bridge — wraps Project E's six-gate validation pipeline.
//!
//! `ForgeSandboxRunner` runs the forge gate sequence in a temporary directory:
//!
//! 1. Creates a minimal Rust project scaffold under a system temp directory.
//! 2. Writes the generated source code to `src/lib.rs`.
//! 3. Calls `project_e_forge::pipeline::run_gates()` which runs six gates:
//!    `UnsafeCheck → ForbiddenPatterns → Compile → Clippy → Audit → Tests`.
//! 4. On success, persists the source to the knowledge workspace and emits
//!    a `ModulePublished` cognitive event.
//! 5. On failure, emits `GateFail` per gate and ultimately `ModuleRejected`.
//!
//! The runner does NOT launch Docker containers. Forge output (successful
//! source code) is saved as a workspace document for human review.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use thiserror::Error;
use tokio::fs;
use tracing::{info, warn};
use uuid::Uuid;

use project_e_core::config::ForgeConfig;
use project_e_core::types::GateName;
use project_e_event_bus::EventBus;
use project_e_event_bus::protocol::CognitiveEvent;
use project_e_forge::gates::GateResult;
use project_e_forge::pipeline::run_gates;

use crate::project_e::knowledge_bridge::WorkspaceKnowledgeStore;

/// Error from the forge runner.
#[derive(Debug, Error)]
pub enum ForgeError {
    #[error("failed to set up forge workspace: {0}")]
    WorkspaceSetup(String),

    #[error("gate '{gate:?}' failed: {message}")]
    GateFailed { gate: GateName, message: String },

    #[error("all retries exhausted after {attempts} attempt(s)")]
    RetriesExhausted { attempts: u8 },

    #[error("event bus error: {0}")]
    EventBus(String),
}

/// Runs the forge pipeline for a single planned module.
pub struct ForgeSandboxRunner {
    config: ForgeConfig,
    event_bus: Arc<EventBus>,
    knowledge: WorkspaceKnowledgeStore,
}

impl ForgeSandboxRunner {
    pub fn new(
        config: ForgeConfig,
        event_bus: Arc<EventBus>,
        knowledge: WorkspaceKnowledgeStore,
    ) -> Self {
        Self {
            config,
            event_bus,
            knowledge,
        }
    }

    /// Run the forge pipeline for the given module source code.
    ///
    /// - `cycle_id` ties this run to the parent cognitive cycle.
    /// - `gap_id` is the gap this module is intended to fill.
    /// - `module_name` is the human-readable identifier.
    /// - `source_code` is the generated Rust source to validate.
    ///
    /// Returns the validated source on success.
    pub async fn run(
        &self,
        cycle_id: Uuid,
        gap_id: Uuid,
        module_name: &str,
        source_code: &str,
    ) -> Result<String, ForgeError> {
        let module_id = Uuid::new_v4();

        // Scaffold first — emit ForgeStarted only once the workspace is ready
        // so observers don't see an orphaned "started" with no completion.
        let workdir = self
            .scaffold_project(module_name, source_code)
            .await
            .map_err(|e| ForgeError::WorkspaceSetup(e.to_string()))?;

        // Emit ForgeStarted.
        self.event_bus
            .emit(CognitiveEvent::ForgeStarted {
                cycle_id,
                gap_id,
                module_name: module_name.to_string(),
            })
            .await
            .map_err(|e| ForgeError::EventBus(e.to_string()))?;

        let max_retries = self.config.max_retries;
        let mut last_error: Option<ForgeError> = None;

        // Retries guard against transient environmental failures (network for `cargo audit`,
        // file-system races, etc.). Semantic failures (compilation, clippy) will not self-heal
        // on retry — LLM regeneration happens at the caller (`CognitiveCycleRunner`) level.
        for attempt in 0..=max_retries {
            let gate_result = run_gates(source_code, &workdir, &self.config).await;

            match gate_result {
                GateResult::Pass => {
                    info!(
                        cycle_id = %cycle_id,
                        module = module_name,
                        attempt,
                        "project_e: forge pipeline passed all gates"
                    );

                    // Persist the validated source to workspace.
                    let date = Utc::now().format("%Y-%m-%d").to_string();
                    let path = format!("modules/{date}/{module_name}.rs");
                    let _ = self.knowledge.write(&path, source_code).await; // best-effort

                    // Emit ModulePublished.
                    let _ = self
                        .event_bus
                        .emit(CognitiveEvent::ModulePublished {
                            cycle_id,
                            module_id,
                            module_name: module_name.to_string(),
                        })
                        .await;

                    let _ = fs::remove_dir_all(&workdir).await; // best-effort cleanup
                    return Ok(source_code.to_string());
                }
                GateResult::Fail(failure) => {
                    let gate = failure.gate;
                    let message = failure.message.clone();

                    warn!(
                        cycle_id = %cycle_id,
                        module = module_name,
                        gate = ?gate,
                        attempt,
                        error = %message,
                        "project_e: forge gate failed"
                    );

                    // Emit GateFail.
                    let _ = self
                        .event_bus
                        .emit(CognitiveEvent::GateFail {
                            cycle_id,
                            module_id,
                            gate,
                            error: message.clone(),
                        })
                        .await;

                    last_error = Some(ForgeError::GateFailed { gate, message });
                }
            }
        }

        // All retries exhausted.
        let _ = fs::remove_dir_all(&workdir).await;

        let rejection_reason = last_error
            .as_ref()
            .map(|e| e.to_string())
            .unwrap_or_else(|| "unknown gate failure".to_string());

        let _ = self
            .event_bus
            .emit(CognitiveEvent::ModuleRejected {
                cycle_id,
                gap_id,
                reason: rejection_reason,
                attempts: max_retries + 1,
            })
            .await;

        Err(last_error.unwrap_or(ForgeError::RetriesExhausted {
            attempts: max_retries + 1,
        }))
    }

    /// Create a minimal Rust project scaffold in a temp directory.
    async fn scaffold_project(
        &self,
        module_name: &str,
        source_code: &str,
    ) -> std::io::Result<PathBuf> {
        let run_id = Uuid::new_v4();
        let workdir = std::env::temp_dir()
            .join("ironclaw_forge")
            .join(format!("{module_name}_{run_id}"));

        fs::create_dir_all(workdir.join("src")).await?;

        let cargo_toml = format!(
            "[package]\nname = \"{module_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\nname = \"{module_name}\"\n"
        );
        fs::write(workdir.join("Cargo.toml"), &cargo_toml).await?;
        fs::write(workdir.join("src").join("lib.rs"), source_code).await?;

        Ok(workdir)
    }
}
