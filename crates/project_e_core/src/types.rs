//! Shared types used across all Project E cognitive-layer crates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Six-phase cognitive cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CognitivePhase {
    Perception,
    Interpretation,
    Deliberation,
    Action,
    Evaluation,
    MetaAdaptation,
}

impl std::fmt::Display for CognitivePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Perception => write!(f, "perception"),
            Self::Interpretation => write!(f, "interpretation"),
            Self::Deliberation => write!(f, "deliberation"),
            Self::Action => write!(f, "action"),
            Self::Evaluation => write!(f, "evaluation"),
            Self::MetaAdaptation => write!(f, "meta_adaptation"),
        }
    }
}

/// Module lifecycle state machine:
/// Draft → Compiling → Testing → Active → Deprecated → Removed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleState {
    Draft,
    Compiling,
    Testing,
    Active,
    Deprecated,
    Removed,
}

impl ModuleState {
    /// Returns true if transitioning from `self` to `target` is valid.
    pub fn can_transition_to(&self, target: ModuleState) -> bool {
        matches!(
            (self, target),
            (Self::Draft, ModuleState::Compiling)
                | (Self::Compiling, ModuleState::Testing)
                | (Self::Compiling, ModuleState::Draft) // failed compilation → back to draft
                | (Self::Testing, ModuleState::Active)
                | (Self::Testing, ModuleState::Draft) // failed tests → back to draft
                | (Self::Active, ModuleState::Deprecated)
                | (Self::Deprecated, ModuleState::Removed)
                | (Self::Deprecated, ModuleState::Active) // re-activation
        )
    }
}

/// Priority levels for gap reports and actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

impl Priority {
    pub fn ordinal(&self) -> u8 {
        match self {
            Self::Critical => 3,
            Self::High => 2,
            Self::Medium => 1,
            Self::Low => 0,
        }
    }
}

/// Cynefin domain determines forge strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CynefinDomain {
    /// Apply best-practice template, high confidence, low risk.
    Simple,
    /// Generate 2–3 variants, test, select best.
    Complicated,
    /// Generate experimental module with "experimental" flag, monitor closely.
    Complex,
    /// Rapid prototype, temporary module, iterate fast.
    Chaotic,
}

impl CynefinDomain {
    /// Human-readable label for use in policy pattern matching.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Complicated => "complicated",
            Self::Complex => "complex",
            Self::Chaotic => "chaotic",
        }
    }
}

/// Wardley evolution stage determines fault tolerance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WardleyStage {
    /// High fault tolerance; many retry attempts allowed.
    Genesis,
    /// Medium tolerance; evaluate generalizability.
    Custom,
    /// Low tolerance; incremental changes only.
    Product,
    /// Candidate for simplification or merger.
    Commodity,
}

impl WardleyStage {
    /// Maximum retry attempts for this evolution stage.
    pub fn max_retries(&self) -> u8 {
        match self {
            Self::Genesis => 5,
            Self::Custom => 4,
            Self::Product => 2,
            Self::Commodity => 1,
        }
    }
}

/// Gap classification in analysis reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapClassification {
    Gap,
    Conflict,
    Confirmation,
}

/// Lifecycle actions suggested by gap analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleAction {
    Create,
    Update,
    Merge,
    Split,
    Deprecate,
}

/// Health status reported by modules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
}

/// Operating mode for the deliberation engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatingMode {
    /// Efficiency very low, recent failures high — minimal action.
    Recovery,
    /// Mixed signals — selective action, defer uncertain gaps.
    Cautious,
    /// Good efficiency, stable system — normal operation.
    Nominal,
}

/// Named forge validation gates in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateName {
    UnsafeCheck,
    ForbiddenPatterns,
    Compile,
    Clippy,
    Audit,
    Test,
}

impl GateName {
    /// Ordinal position in the gate sequence (0-indexed).
    pub fn ordinal(&self) -> u8 {
        match self {
            Self::UnsafeCheck => 0,
            Self::ForbiddenPatterns => 1,
            Self::Compile => 2,
            Self::Clippy => 3,
            Self::Audit => 4,
            Self::Test => 5,
        }
    }
}

/// Supervisor anomaly patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyPattern {
    RepeatedCompilationErrors,
    EfficiencyCollapse,
    RecoveryLoop,
    ErrorStorm,
    StaleData,
    RedundantModules,
    ContextOverflow,
}

/// Supervisor intervention types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum InterventionType {
    PromptInjection { guidance: String },
    ModelEscalation { to_model: String },
    TightenConfidenceThreshold { delta: f32 },
    PauseCycle { duration_secs: u64 },
    SkipPhase { phase: CognitivePhase },
    ForceRecoveryMode,
    ResetCounters,
}

/// Five homeostatic health indicators.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthIndicator {
    Coverage,
    Stability,
    Efficiency,
    Utilization,
    Freshness,
}

/// Metadata describing a module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub state: ModuleState,
    pub subscriptions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Execution context passed to modules during invocation.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub cycle_id: Uuid,
    pub module_id: Uuid,
    pub phase: CognitivePhase,
    pub parameters: serde_json::Value,
}

/// Output produced by module execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleOutput {
    pub data: serde_json::Value,
    pub output_type: String,
    pub events: Vec<serde_json::Value>,
}

/// Cycle-level metrics recorded after each cognitive cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleMetrics {
    pub cycle_id: Uuid,
    pub duration_secs: u64,
    pub gaps_detected: usize,
    pub modules_published: usize,
    pub modules_rejected: usize,
    pub forge_efficiency: f32,
    pub operating_mode: OperatingMode,
    pub completed_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_state_valid_transitions() {
        assert!(ModuleState::Draft.can_transition_to(ModuleState::Compiling));
        assert!(ModuleState::Active.can_transition_to(ModuleState::Deprecated));
        assert!(!ModuleState::Draft.can_transition_to(ModuleState::Active));
        assert!(!ModuleState::Removed.can_transition_to(ModuleState::Active));
    }

    #[test]
    fn gate_name_ordering() {
        assert!(GateName::UnsafeCheck.ordinal() < GateName::Compile.ordinal());
        assert!(GateName::Compile.ordinal() < GateName::Test.ordinal());
    }

    #[test]
    fn priority_ordering() {
        assert!(Priority::Critical.ordinal() > Priority::Low.ordinal());
    }

    #[test]
    fn cognitive_event_round_trip() {
        let phase = CognitivePhase::Action;
        let json = serde_json::to_string(&phase).unwrap();
        let parsed: CognitivePhase = serde_json::from_str(&json).unwrap();
        assert_eq!(phase, parsed);
    }

    #[test]
    fn health_status_serialization() {
        let degraded = HealthStatus::Degraded {
            reason: "high latency".to_string(),
        };
        let json = serde_json::to_string(&degraded).unwrap();
        let parsed: HealthStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(degraded, parsed);
    }
}
