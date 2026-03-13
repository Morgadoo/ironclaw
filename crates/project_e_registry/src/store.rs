//! In-memory module store with lifecycle enforcement.
//!
//! In production, this wraps PostgreSQL as the single source of truth
//! with a DashMap read-through cache. This implementation provides
//! the in-memory store for testing and development.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use uuid::Uuid;

use project_e_core::error::{ProjectEError, Result};
use project_e_core::types::{ModuleMetadata, ModuleState};

/// In-memory module registry (development/testing).
/// Production implementation wraps PostgreSQL.
pub struct ModuleStore {
    modules: Arc<RwLock<HashMap<Uuid, ModuleMetadata>>>,
}

impl ModuleStore {
    pub fn new() -> Self {
        Self {
            modules: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Insert a new module.
    pub async fn insert(&self, module: ModuleMetadata) -> Result<()> {
        let mut modules = self.modules.write().await;
        modules.insert(module.id, module);
        Ok(())
    }

    /// Get a module by ID.
    pub async fn get(&self, id: Uuid) -> Result<Option<ModuleMetadata>> {
        let modules = self.modules.read().await;
        Ok(modules.get(&id).cloned())
    }

    /// List all modules, optionally filtered by state.
    pub async fn list(&self, state_filter: Option<ModuleState>) -> Vec<ModuleMetadata> {
        let modules = self.modules.read().await;
        modules
            .values()
            .filter(|m| state_filter.is_none_or(|s| m.state == s))
            .cloned()
            .collect()
    }

    /// Transition module state; enforces valid transitions.
    pub async fn transition(&self, id: Uuid, new_state: ModuleState, _reason: &str) -> Result<()> {
        let mut modules = self.modules.write().await;
        let module = modules
            .get_mut(&id)
            .ok_or_else(|| ProjectEError::Registry {
                reason: format!("module {id} not found"),
            })?;

        if !module.state.can_transition_to(new_state) {
            return Err(ProjectEError::Registry {
                reason: format!(
                    "invalid transition {:?} → {:?} for module {}",
                    module.state, new_state, module.name
                ),
            });
        }

        module.state = new_state;
        module.updated_at = chrono::Utc::now();
        Ok(())
    }

    /// Count of modules in a given state.
    pub async fn count_by_state(&self, state: ModuleState) -> usize {
        let modules = self.modules.read().await;
        modules.values().filter(|m| m.state == state).count()
    }
}

impl Default for ModuleStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_module(name: &str) -> ModuleMetadata {
        ModuleMetadata {
            id: Uuid::new_v4(),
            name: name.to_string(),
            version: "0.1.0".to_string(),
            description: "test module".to_string(),
            category: "test".to_string(),
            tags: vec![],
            state: ModuleState::Draft,
            subscriptions: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn insert_and_get() {
        let store = ModuleStore::new();
        let module = make_module("test");
        let id = module.id;

        store.insert(module).await.unwrap();
        let found = store.get(id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test");
    }

    #[tokio::test]
    async fn valid_transition() {
        let store = ModuleStore::new();
        let module = make_module("test");
        let id = module.id;

        store.insert(module).await.unwrap();
        store
            .transition(id, ModuleState::Compiling, "starting build")
            .await
            .unwrap();

        let module = store.get(id).await.unwrap().unwrap();
        assert_eq!(module.state, ModuleState::Compiling);
    }

    #[tokio::test]
    async fn invalid_transition_fails() {
        let store = ModuleStore::new();
        let module = make_module("test");
        let id = module.id;

        store.insert(module).await.unwrap();
        let result = store
            .transition(id, ModuleState::Active, "skip ahead")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_by_state() {
        let store = ModuleStore::new();

        let mut m1 = make_module("draft-module");
        store.insert(m1.clone()).await.unwrap();

        let mut m2 = make_module("active-module");
        m2.state = ModuleState::Draft;
        store.insert(m2.clone()).await.unwrap();

        let drafts = store.list(Some(ModuleState::Draft)).await;
        assert_eq!(drafts.len(), 2);

        let actives = store.list(Some(ModuleState::Active)).await;
        assert_eq!(actives.len(), 0);
    }
}
