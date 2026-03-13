//! Module → Tool Adapter — bridges Project E `Module` trait to IronClaw `Tool`.
//!
//! `ModuleToolAdapter<M>` wraps any type implementing `project_e_core::module_trait::Module`
//! and makes it discoverable as an IronClaw `Tool` in the `ToolRegistry`. This
//! means the LLM can invoke any active Project E module as a first-class tool
//! without any additional wiring.
//!
//! # Mapping
//!
//! | Module field | Tool field |
//! |---|---|
//! | `metadata().name` | `Tool::name()` |
//! | `metadata().description` | `Tool::description()` |
//! | fixed JSON schema (see below) | `Tool::parameters_schema()` |
//! | `execute(&ctx)` | `Tool::execute(params, job_ctx)` |
//!
//! Parameters schema exposed to the LLM:
//! ```json
//! {
//!   "type": "object",
//!   "properties": {
//!     "parameters": { "description": "JSON parameters passed to the module" }
//!   },
//!   "required": []
//! }
//! ```
//! The `parameters` field is passed verbatim to `ExecutionContext::parameters`.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use uuid::Uuid;

use project_e_core::module_trait::Module;
use project_e_core::types::{CognitivePhase, ExecutionContext};

use crate::context::JobContext;
use crate::tools::tool::{ApprovalRequirement, Tool, ToolError, ToolOutput};

/// Adapts a Project E `Module` for use as an IronClaw `Tool`.
///
/// Stores module metadata at construction time so `name()` / `description()`
/// can return `&str` without calling `metadata()` on every invocation (which
/// would require holding `&self` across an await point).
pub struct ModuleToolAdapter<M: Module> {
    module: M,
    /// Pre-computed tool name (`project_e.<module_name>`).
    tool_name: String,
    /// Module description (verbatim from `ModuleMetadata::description`).
    description: String,
    /// Module ID used as the `module_id` in every `ExecutionContext`.
    module_id: Uuid,
}

impl<M: Module> ModuleToolAdapter<M> {
    /// Wrap a module. Reads metadata once and caches name / description.
    pub fn new(module: M) -> Self {
        let meta = module.metadata();
        let tool_name = format!("project_e.{}", meta.name);
        let description = meta.description.clone();
        let module_id = meta.id;
        Self {
            module,
            tool_name,
            description,
            module_id,
        }
    }

    /// Expose the inner module (e.g. for health checks or teardown).
    pub fn inner(&self) -> &M {
        &self.module
    }

    /// Expose the inner module mutably (e.g. for `init()` / `teardown()`).
    pub fn inner_mut(&mut self) -> &mut M {
        &mut self.module
    }
}

#[async_trait]
impl<M: Module + 'static> Tool for ModuleToolAdapter<M> {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "parameters": {
                    "description": "JSON parameters forwarded to the Project E module"
                },
                "phase": {
                    "type": "string",
                    "description": "Optional cognitive phase hint (perception/interpretation/deliberation/action/evaluation/meta_adaptation)"
                }
            },
            "required": []
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let start = Instant::now();

        let parameters = params
            .get("parameters")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        // Optional phase override from caller; defaults to Action.
        let phase = params
            .get("phase")
            .and_then(|v| v.as_str())
            .and_then(parse_phase)
            .unwrap_or(CognitivePhase::Action);

        let exec_ctx = ExecutionContext {
            cycle_id: ctx.job_id, // reuse job_id as cycle surrogate when called ad-hoc
            module_id: self.module_id,
            phase,
            parameters,
        };

        let output = self.module.execute(&exec_ctx).await.map_err(|e| {
            ToolError::ExecutionFailed(format!("module '{}' error: {e}", self.tool_name))
        })?;

        let duration = start.elapsed();
        Ok(ToolOutput::success(output.data, duration))
    }

    /// Module tools are orchestrator-safe (no direct FS / shell access).
    fn requires_sanitization(&self) -> bool {
        true // module output may include external data
    }

    fn requires_approval(&self, _params: &serde_json::Value) -> ApprovalRequirement {
        ApprovalRequirement::Never
    }

    fn execution_timeout(&self) -> Duration {
        Duration::from_secs(120) // modules may do analysis or generate code
    }
}

/// Parse a phase string into `CognitivePhase`.
fn parse_phase(s: &str) -> Option<CognitivePhase> {
    match s {
        "perception" => Some(CognitivePhase::Perception),
        "interpretation" => Some(CognitivePhase::Interpretation),
        "deliberation" => Some(CognitivePhase::Deliberation),
        "action" => Some(CognitivePhase::Action),
        "evaluation" => Some(CognitivePhase::Evaluation),
        "meta_adaptation" => Some(CognitivePhase::MetaAdaptation),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use project_e_core::error::Result as PeResult;
    use project_e_core::module_trait::Module;
    use project_e_core::types::{
        HealthStatus, ModuleMetadata, ModuleOutput, ModuleState,
    };

    struct EchoModule {
        meta: ModuleMetadata,
    }

    impl EchoModule {
        fn new() -> Self {
            Self {
                meta: ModuleMetadata {
                    id: Uuid::new_v4(),
                    name: "echo".to_string(),
                    version: "0.1.0".to_string(),
                    description: "Echoes parameters".to_string(),
                    category: "test".to_string(),
                    tags: vec![],
                    state: ModuleState::Active,
                    subscriptions: vec![],
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
            }
        }
    }

    #[async_trait]
    impl Module for EchoModule {
        fn metadata(&self) -> ModuleMetadata {
            self.meta.clone()
        }
        async fn init(&mut self, _: &serde_json::Value) -> PeResult<()> {
            Ok(())
        }
        async fn execute(&self, ctx: &ExecutionContext) -> PeResult<ModuleOutput> {
            Ok(ModuleOutput {
                data: ctx.parameters.clone(),
                output_type: "echo".to_string(),
                events: vec![],
            })
        }
        fn subscriptions(&self) -> Vec<String> {
            vec![]
        }
        async fn handle_event(&self, e: &serde_json::Value) -> PeResult<Vec<serde_json::Value>> {
            Ok(vec![e.clone()])
        }
        fn health_check(&self) -> HealthStatus {
            HealthStatus::Healthy
        }
    }

    #[tokio::test]
    async fn adapter_echoes_parameters() {
        let adapter = ModuleToolAdapter::new(EchoModule::new());
        assert_eq!(adapter.name(), "project_e.echo");
        assert!(!adapter.description().is_empty());

        let ctx = JobContext::default();
        let params = serde_json::json!({ "parameters": { "msg": "hello" } });
        let out = adapter.execute(params, &ctx).await.unwrap();
        assert_eq!(out.result, serde_json::json!({ "msg": "hello" }));
    }

    #[test]
    fn parse_phase_all_variants() {
        for (s, expected) in [
            ("perception", CognitivePhase::Perception),
            ("interpretation", CognitivePhase::Interpretation),
            ("deliberation", CognitivePhase::Deliberation),
            ("action", CognitivePhase::Action),
            ("evaluation", CognitivePhase::Evaluation),
            ("meta_adaptation", CognitivePhase::MetaAdaptation),
        ] {
            assert_eq!(parse_phase(s), Some(expected), "phase '{s}' should parse");
        }
        assert_eq!(parse_phase("unknown"), None);
    }
}
