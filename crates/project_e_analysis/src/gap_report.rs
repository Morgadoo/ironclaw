//! Enriched GapReport with Cynefin/Wardley classification.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use project_e_core::types::{
    CynefinDomain, GapClassification, LifecycleAction, Priority, WardleyStage,
};

/// A gap report produced by the analysis engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapReport {
    pub id: Uuid,
    pub cycle_id: Uuid,
    pub classification: GapClassification,
    /// 0.0–1.0; only gaps above the confidence threshold proceed.
    pub confidence: f32,
    /// Cynefin domain determines forge strategy in Phase 4.
    pub cynefin_domain: CynefinDomain,
    /// Wardley evolution stage determines fault tolerance.
    pub wardley_stage: WardleyStage,
    pub priority: Priority,
    pub suggested_action: LifecycleAction,
    pub justification: String,
    pub affected_modules: Vec<Uuid>,
    /// Source URLs / data item references.
    pub evidence: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl GapReport {
    /// Create a new gap report with auto-generated ID and timestamp.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cycle_id: Uuid,
        classification: GapClassification,
        confidence: f32,
        cynefin_domain: CynefinDomain,
        wardley_stage: WardleyStage,
        priority: Priority,
        suggested_action: LifecycleAction,
        justification: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            cycle_id,
            classification,
            confidence,
            cynefin_domain,
            wardley_stage,
            priority,
            suggested_action,
            justification,
            affected_modules: vec![],
            evidence: vec![],
            created_at: Utc::now(),
        }
    }

    /// Whether this gap requires web access for the resulting module.
    pub fn requires_web_access(&self) -> bool {
        // Heuristic: if evidence contains URLs, the module likely needs web access
        self.evidence
            .iter()
            .any(|e| e.starts_with("http://") || e.starts_with("https://"))
    }

    /// Maximum retry attempts based on Wardley stage.
    pub fn max_retries(&self) -> u8 {
        self.wardley_stage.max_retries()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gap_report_serialization_round_trip() {
        let report = GapReport::new(
            Uuid::new_v4(),
            GapClassification::Gap,
            0.85,
            CynefinDomain::Complicated,
            WardleyStage::Custom,
            Priority::High,
            LifecycleAction::Create,
            "Missing tariff change detection module".to_string(),
        );

        let json = serde_json::to_string(&report).unwrap();
        let parsed: GapReport = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, report.id);
        assert_eq!(parsed.cynefin_domain, CynefinDomain::Complicated);
        assert_eq!(parsed.wardley_stage, WardleyStage::Custom);
        assert_eq!(parsed.max_retries(), 4);
    }

    #[test]
    fn requires_web_access_detection() {
        let mut report = GapReport::new(
            Uuid::new_v4(),
            GapClassification::Gap,
            0.9,
            CynefinDomain::Simple,
            WardleyStage::Product,
            Priority::Medium,
            LifecycleAction::Create,
            "test".to_string(),
        );

        assert!(!report.requires_web_access());

        report.evidence.push("https://example.com/data".to_string());
        assert!(report.requires_web_access());
    }
}
