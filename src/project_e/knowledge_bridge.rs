//! Knowledge Base ↔ Workspace Bridge — routes Project E knowledge through IronClaw Workspace.
//!
//! `WorkspaceKnowledgeStore` wraps `Arc<Workspace>` and exposes two operations:
//!
//! - `write(path, content)` — persists a knowledge document at a Project E
//!   path (`project_e/knowledge/<path>`). Delegates to `Workspace::write()`.
//! - `search(query, limit)` — hybrid RRF search (BM25 + vector) via
//!   `Workspace::search()`. Returns ranked results as `KnowledgeHit` values.
//!
//! This bridge is the single point coupling Project E's knowledge layer to
//! IronClaw's workspace storage. Replacing the workspace backend (PostgreSQL →
//! libSQL → remote) automatically benefits Project E with no changes here.

use std::sync::Arc;

use thiserror::Error;
use tracing::warn;

use crate::workspace::{SearchResult, Workspace};

/// Prefix applied to all Project E knowledge paths inside the workspace.
const PATH_PREFIX: &str = "project_e/knowledge";

/// Error type for knowledge bridge operations.
#[derive(Debug, Error)]
pub enum KnowledgeError {
    #[error("workspace write failed: {0}")]
    Write(String),

    #[error("workspace search failed: {0}")]
    Search(String),
}

/// A single search result returned to Project E's cognitive layer.
#[derive(Debug, Clone)]
pub struct KnowledgeHit {
    /// Workspace document path (without the `project_e/knowledge/` prefix).
    pub path: String,
    /// Matched chunk content.
    pub content: String,
    /// Combined RRF score (higher is better).
    pub score: f32,
}

/// Routes knowledge operations through IronClaw's `Workspace`.
///
/// Thread-safe and cheaply cloneable.
#[derive(Clone)]
pub struct WorkspaceKnowledgeStore {
    workspace: Arc<Workspace>,
}

impl WorkspaceKnowledgeStore {
    /// Construct a store backed by the given workspace.
    pub fn new(workspace: Arc<Workspace>) -> Self {
        Self { workspace }
    }

    /// Write a knowledge document.
    ///
    /// `path` is relative to the `project_e/knowledge/` prefix; e.g.
    /// `"cycle/2026-03-13.md"` is stored as
    /// `"project_e/knowledge/cycle/2026-03-13.md"`.
    pub async fn write(&self, path: &str, content: &str) -> Result<(), KnowledgeError> {
        let full_path = format!("{PATH_PREFIX}/{path}");
        self.workspace
            .write(&full_path, content)
            .await
            .map(|_| ())
            .map_err(|e| KnowledgeError::Write(e.to_string()))
    }

    /// Hybrid-search the knowledge store (BM25 + vector, fused via RRF).
    ///
    /// `limit` is the maximum number of results to return.
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<KnowledgeHit>, KnowledgeError> {
        let raw: Vec<SearchResult> = self
            .workspace
            .search(query, limit)
            .await
            .map_err(|e| KnowledgeError::Search(e.to_string()))?;

        let hits = raw
            .into_iter()
            .filter_map(|r| {
                // Only return documents that live under the project_e prefix.
                if !r.document_path.starts_with(PATH_PREFIX) {
                    return None;
                }
                let relative_path = r
                    .document_path
                    .strip_prefix(&format!("{PATH_PREFIX}/"))
                    .unwrap_or(&r.document_path)
                    .to_string();
                Some(KnowledgeHit {
                    path: relative_path,
                    content: r.content,
                    score: r.score,
                })
            })
            .collect();

        if hits.is_empty() && !query.is_empty() {
            warn!(query, limit, "project_e: knowledge search returned no results");
        }

        Ok(hits)
    }

    /// Append content to a knowledge document (used by the cognitive cycle log).
    pub async fn append(&self, path: &str, content: &str) -> Result<(), KnowledgeError> {
        let full_path = format!("{PATH_PREFIX}/{path}");
        self.workspace
            .append(&full_path, content)
            .await
            .map_err(|e| KnowledgeError::Write(e.to_string()))
    }
}
