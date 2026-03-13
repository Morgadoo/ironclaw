//! Typed cognitive event protocol.
//!
//! Every event published on the event bus carries an `EventEnvelope`
//! with metadata and a typed `CognitiveEvent` payload.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use project_e_core::types::{
    AnomalyPattern, GateName, HealthIndicator, InterventionType, OperatingMode, Priority,
};

/// Every event published on the event bus carries this envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_id: Uuid,
    /// Stable ID linking all events of one cognitive cycle.
    pub cycle_id: Option<Uuid>,
    /// Links request/response pairs.
    pub correlation_id: Uuid,
    pub topic: String,
    pub schema_version: u8,
    pub producer: String,
    pub timestamp: DateTime<Utc>,
    pub payload: CognitiveEvent,
}

impl EventEnvelope {
    /// Wrap a cognitive event into an envelope with auto-generated metadata.
    pub fn wrap(event: CognitiveEvent) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            cycle_id: event.cycle_id(),
            correlation_id: Uuid::new_v4(),
            topic: event.topic().to_string(),
            schema_version: 1,
            producer: "project-e".to_string(),
            timestamp: Utc::now(),
            payload: event,
        }
    }
}

/// All cognitive-phase transition events as a typed enum.
/// Serialized as JSON to the event bus; deserialized by each phase consumer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CognitiveEvent {
    // Phase 1 — Perceive
    CycleStarted {
        cycle_id: Uuid,
        instance: String,
    },
    PerceptionComplete {
        cycle_id: Uuid,
        data_count: usize,
        avg_relevance: f32,
    },
    CrawlComplete {
        cycle_id: Uuid,
        url: String,
        items: usize,
    },

    // Phase 2 — Interpret
    GapReportReady {
        cycle_id: Uuid,
        report_id: Uuid,
        gap_count: usize,
        conflict_count: usize,
        confirmation_count: usize,
    },
    GapDetected {
        cycle_id: Uuid,
        gap_id: Uuid,
        priority: Priority,
    },

    // Phase 3 — Deliberate
    SelfModelUpdated {
        cycle_id: Uuid,
        mode: OperatingMode,
        efficiency: f32,
    },
    ActionPlanReady {
        cycle_id: Uuid,
        plan_id: Uuid,
        actions_count: usize,
    },
    ActionDeferred {
        cycle_id: Uuid,
        gap_id: Uuid,
        reason: String,
    },

    // Phase 4 — Act
    ForgeStarted {
        cycle_id: Uuid,
        gap_id: Uuid,
        module_name: String,
    },
    GatePass {
        cycle_id: Uuid,
        module_id: Uuid,
        gate: GateName,
    },
    GateFail {
        cycle_id: Uuid,
        module_id: Uuid,
        gate: GateName,
        error: String,
    },
    ModulePublished {
        cycle_id: Uuid,
        module_id: Uuid,
        module_name: String,
    },
    ModuleRejected {
        cycle_id: Uuid,
        gap_id: Uuid,
        reason: String,
        attempts: u8,
    },

    // Phase 5 — Evaluate
    EvaluationComplete {
        cycle_id: Uuid,
        published: usize,
        rejected: usize,
        efficiency: f32,
        skill_scores: Vec<(String, f32)>,
    },
    ModuleDeprecated {
        cycle_id: Uuid,
        module_id: Uuid,
        reason: String,
    },

    // Phase 6 — Meta-Adapt
    ParameterAdjusted {
        cycle_id: Uuid,
        param: String,
        old_value: f32,
        new_value: f32,
        justification: String,
    },
    ConstitutionalLesson {
        cycle_id: Uuid,
        lesson: String,
    },
    CycleClosed {
        cycle_id: Uuid,
        duration_secs: u64,
    },

    // Cross-cutting
    HealthAlert {
        indicator: HealthIndicator,
        value: f32,
        threshold: f32,
    },
    SupervisorIntervention {
        pattern: AnomalyPattern,
        intervention: InterventionType,
    },
    CircuitBreakerOpen {
        provider: String,
        failures: u32,
    },
    CircuitBreakerClosed {
        provider: String,
    },
}

impl CognitiveEvent {
    /// Returns the NATS topic for this event type.
    pub fn topic(&self) -> &'static str {
        match self {
            Self::CycleStarted { .. } => "project_e.system.cycle_started",
            Self::PerceptionComplete { .. } => "project_e.perception.complete",
            Self::CrawlComplete { .. } => "project_e.perception.crawl_complete",
            Self::GapReportReady { .. } => "project_e.analysis.gap_report_ready",
            Self::GapDetected { .. } => "project_e.analysis.gap_detected",
            Self::SelfModelUpdated { .. } => "project_e.meta.self_model_updated",
            Self::ActionPlanReady { .. } => "project_e.meta.action_plan_created",
            Self::ActionDeferred { .. } => "project_e.meta.action_deferred",
            Self::ForgeStarted { .. } => "project_e.forge.started",
            Self::GatePass { .. } => "project_e.forge.gate_pass",
            Self::GateFail { .. } => "project_e.forge.gate_fail",
            Self::ModulePublished { .. } => "project_e.forge.module_published",
            Self::ModuleRejected { .. } => "project_e.forge.module_rejected",
            Self::EvaluationComplete { .. } => "project_e.meta.cycle_evaluated",
            Self::ModuleDeprecated { .. } => "project_e.registry.module_deprecated",
            Self::ParameterAdjusted { .. } => "project_e.meta.parameter_adjusted",
            Self::ConstitutionalLesson { .. } => "project_e.meta.constitutional_lesson",
            Self::CycleClosed { .. } => "project_e.system.cycle_closed",
            Self::HealthAlert { .. } => "project_e.system.health_alert",
            Self::SupervisorIntervention { .. } => "project_e.system.supervisor_intervention",
            Self::CircuitBreakerOpen { .. } => "project_e.system.circuit_open",
            Self::CircuitBreakerClosed { .. } => "project_e.system.circuit_closed",
        }
    }

    /// Extract the cycle_id if present in this event.
    pub fn cycle_id(&self) -> Option<Uuid> {
        match self {
            Self::CycleStarted { cycle_id, .. }
            | Self::PerceptionComplete { cycle_id, .. }
            | Self::CrawlComplete { cycle_id, .. }
            | Self::GapReportReady { cycle_id, .. }
            | Self::GapDetected { cycle_id, .. }
            | Self::SelfModelUpdated { cycle_id, .. }
            | Self::ActionPlanReady { cycle_id, .. }
            | Self::ActionDeferred { cycle_id, .. }
            | Self::ForgeStarted { cycle_id, .. }
            | Self::GatePass { cycle_id, .. }
            | Self::GateFail { cycle_id, .. }
            | Self::ModulePublished { cycle_id, .. }
            | Self::ModuleRejected { cycle_id, .. }
            | Self::EvaluationComplete { cycle_id, .. }
            | Self::ModuleDeprecated { cycle_id, .. }
            | Self::ParameterAdjusted { cycle_id, .. }
            | Self::ConstitutionalLesson { cycle_id, .. }
            | Self::CycleClosed { cycle_id, .. } => Some(*cycle_id),
            Self::HealthAlert { .. }
            | Self::SupervisorIntervention { .. }
            | Self::CircuitBreakerOpen { .. }
            | Self::CircuitBreakerClosed { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cognitive_event_json_round_trip() {
        let event = CognitiveEvent::CycleStarted {
            cycle_id: Uuid::new_v4(),
            instance: "energy-pt".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: CognitiveEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event.topic(), parsed.topic());
    }

    #[test]
    fn envelope_wrapping() {
        let event = CognitiveEvent::GapReportReady {
            cycle_id: Uuid::new_v4(),
            report_id: Uuid::new_v4(),
            gap_count: 3,
            conflict_count: 1,
            confirmation_count: 2,
        };
        let envelope = EventEnvelope::wrap(event);
        assert_eq!(envelope.schema_version, 1);
        assert_eq!(envelope.topic, "project_e.analysis.gap_report_ready");

        let json = serde_json::to_string(&envelope).unwrap();
        let parsed: EventEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.topic, envelope.topic);
    }

    #[test]
    fn all_events_have_topics() {
        let cycle_id = Uuid::new_v4();
        let events = vec![
            CognitiveEvent::CycleStarted {
                cycle_id,
                instance: "test".into(),
            },
            CognitiveEvent::PerceptionComplete {
                cycle_id,
                data_count: 5,
                avg_relevance: 0.7,
            },
            CognitiveEvent::CycleClosed {
                cycle_id,
                duration_secs: 120,
            },
            CognitiveEvent::CircuitBreakerOpen {
                provider: "openai".into(),
                failures: 5,
            },
        ];
        for event in events {
            assert!(!event.topic().is_empty());
        }
    }

    #[test]
    fn cycle_id_extraction() {
        let cycle_id = Uuid::new_v4();
        let with_cycle = CognitiveEvent::ForgeStarted {
            cycle_id,
            gap_id: Uuid::new_v4(),
            module_name: "test".into(),
        };
        assert_eq!(with_cycle.cycle_id(), Some(cycle_id));

        let without_cycle = CognitiveEvent::CircuitBreakerOpen {
            provider: "test".into(),
            failures: 3,
        };
        assert_eq!(without_cycle.cycle_id(), None);
    }
}
