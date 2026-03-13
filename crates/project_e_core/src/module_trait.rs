//! The seven-method `Module` trait — the foundational contract every
//! Project E module must implement. Extends IronClaw's tool infrastructure
//! so modules are automatically discoverable by the existing tool registry.

use async_trait::async_trait;

use crate::error::Result;
use crate::types::{ExecutionContext, HealthStatus, ModuleMetadata, ModuleOutput};

/// The seven-method contract every Project E module must implement.
///
/// This trait is a superset of IronClaw's `Tool` trait. All modules are
/// automatically tools, discoverable by the existing tool registry.
///
/// Forge pipeline rejects modules failing these constraints:
/// - No `unsafe` blocks (AST-checked, not regex)
/// - No `unwrap()`, `panic!()`, `todo!()`, `unimplemented!()`
/// - Only whitelisted crate dependencies
/// - All inter-module communication via NATS events (no direct calls)
#[async_trait]
pub trait Module: Send + Sync {
    /// Identity: unique ID, name, version, category, tags, UI hints,
    /// runtime metadata (topics, resource budget, health SLA).
    fn metadata(&self) -> ModuleMetadata;

    /// One-time initialization. Must succeed before entering ACTIVE state.
    async fn init(&mut self, config: &serde_json::Value) -> Result<()>;

    /// Main business logic. Returns structured output with type hint.
    async fn execute(&self, ctx: &ExecutionContext) -> Result<ModuleOutput>;

    /// Declares which NATS topics this module subscribes to.
    fn subscriptions(&self) -> Vec<String>;

    /// Processes an incoming event; may emit zero or more new events.
    async fn handle_event(&self, event: &serde_json::Value) -> Result<Vec<serde_json::Value>>;

    /// Reports current health: Healthy / Degraded(reason) / Unhealthy(reason).
    fn health_check(&self) -> HealthStatus {
        HealthStatus::Healthy
    }

    /// Graceful cleanup before shutdown.
    async fn teardown(&mut self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CognitivePhase, ModuleState};
    use chrono::Utc;
    use uuid::Uuid;

    /// A minimal test module that implements the trait.
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
                    description: "Echoes input data".to_string(),
                    category: "test".to_string(),
                    tags: vec!["echo".to_string()],
                    state: ModuleState::Draft,
                    subscriptions: vec!["project_e.test.echo".to_string()],
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

        async fn init(&mut self, _config: &serde_json::Value) -> Result<()> {
            self.meta.state = ModuleState::Active;
            Ok(())
        }

        async fn execute(&self, ctx: &ExecutionContext) -> Result<ModuleOutput> {
            Ok(ModuleOutput {
                data: ctx.parameters.clone(),
                output_type: "echo".to_string(),
                events: vec![],
            })
        }

        fn subscriptions(&self) -> Vec<String> {
            self.meta.subscriptions.clone()
        }

        async fn handle_event(&self, event: &serde_json::Value) -> Result<Vec<serde_json::Value>> {
            Ok(vec![event.clone()])
        }
    }

    #[tokio::test]
    async fn echo_module_lifecycle() {
        let mut module = EchoModule::new();
        assert_eq!(module.metadata().state, ModuleState::Draft);

        module.init(&serde_json::json!({})).await.unwrap();
        assert_eq!(module.metadata().state, ModuleState::Active);

        let ctx = ExecutionContext {
            cycle_id: Uuid::new_v4(),
            module_id: module.metadata().id,
            phase: CognitivePhase::Action,
            parameters: serde_json::json!({"msg": "hello"}),
        };

        let output = module.execute(&ctx).await.unwrap();
        assert_eq!(output.data, serde_json::json!({"msg": "hello"}));
        assert_eq!(output.output_type, "echo");

        assert_eq!(module.health_check(), HealthStatus::Healthy);

        module.teardown().await.unwrap();
    }
}
