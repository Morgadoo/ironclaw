//! Project E Core — shared types, traits, configuration, and infrastructure.
//!
//! This crate provides the foundational contracts that all cognitive-layer
//! crates depend on: the `Module` trait, typed errors, configuration loading,
//! and the LLM health monitor (circuit breaker).

pub mod config;
pub mod error;
pub mod llm_health;
pub mod module_trait;
pub mod types;
