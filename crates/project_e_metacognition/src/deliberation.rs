//! DeliberationEngine — filters gap reports by operating mode and
//! assigns Cynefin-based forge strategies.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use project_e_analysis::gap_report::GapReport;
use project_e_core::config::LimitsConfig;
use project_e_core::types::{CynefinDomain, OperatingMode};

use crate::self_model::SystemSelfModel;

/// A planned action derived from a gap report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedAction {
    pub gap_id: Uuid,
    pub strategy: ForgeStrategy,
    pub max_retries: u8,
    pub experimental: bool,
    pub action_label: String,
}

/// Forge strategy derived from Cynefin domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgeStrategy {
    CreateFromTemplate,
    CreateWithVariants { variant_count: u8 },
    CreateExperimental,
    CreateRapidPrototype,
}

/// A gap that was deferred (not acted on this cycle).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferredGap {
    pub gap_id: Uuid,
    pub reason: String,
}

/// The output of deliberation: actions to take and gaps deferred.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    pub id: Uuid,
    pub cycle_id: Uuid,
    /// Ordered by priority.
    pub actions: Vec<PlannedAction>,
    /// Gaps not acted on this cycle, with reasons.
    pub deferred: Vec<DeferredGap>,
    /// Estimated LLM cost in USD.
    pub estimated_llm_cost: f32,
    /// From deliberation.max_concurrent_actions config.
    pub max_concurrent: usize,
}

pub struct DeliberationEngine {
    limits: LimitsConfig,
}

impl DeliberationEngine {
    pub fn new(limits: LimitsConfig) -> Self {
        Self { limits }
    }

    /// Run deliberation: filter gaps by operating mode and assign strategies.
    pub fn deliberate(
        &self,
        cycle_id: Uuid,
        gap_reports: Vec<GapReport>,
        self_model: &SystemSelfModel,
    ) -> ActionPlan {
        let mut actions = Vec::new();
        let mut deferred = Vec::new();

        match self_model.operating_mode {
            OperatingMode::Recovery => {
                // Recovery: defer everything
                for gap in &gap_reports {
                    deferred.push(DeferredGap {
                        gap_id: gap.id,
                        reason: "System in Recovery mode — deferring all actions".to_string(),
                    });
                }
            }
            OperatingMode::Cautious => {
                // Cautious: only Simple/Confirmed gaps, max 2 actions
                let max_actions = 2.min(self.limits.max_actions_per_cycle);
                for gap in &gap_reports {
                    if actions.len() >= max_actions {
                        deferred.push(DeferredGap {
                            gap_id: gap.id,
                            reason: "Cautious mode action limit reached".to_string(),
                        });
                        continue;
                    }
                    if gap.cynefin_domain == CynefinDomain::Simple
                        || gap.classification
                            == project_e_core::types::GapClassification::Confirmation
                    {
                        actions.push(self.plan_action(gap));
                    } else {
                        deferred.push(DeferredGap {
                            gap_id: gap.id,
                            reason: format!(
                                "Cautious mode — deferring {:?} domain gap",
                                gap.cynefin_domain
                            ),
                        });
                    }
                }
            }
            OperatingMode::Nominal => {
                // Nominal: full action plan up to limit
                for gap in &gap_reports {
                    if actions.len() >= self.limits.max_actions_per_cycle {
                        deferred.push(DeferredGap {
                            gap_id: gap.id,
                            reason: "Max actions per cycle reached".to_string(),
                        });
                        continue;
                    }
                    actions.push(self.plan_action(gap));
                }
            }
        }

        ActionPlan {
            id: Uuid::new_v4(),
            cycle_id,
            actions,
            deferred,
            estimated_llm_cost: 0.0, // TODO: wire to ModelCapabilities
            max_concurrent: self.limits.max_concurrent_actions,
        }
    }

    fn plan_action(&self, gap: &GapReport) -> PlannedAction {
        let strategy = match gap.cynefin_domain {
            CynefinDomain::Simple => ForgeStrategy::CreateFromTemplate,
            CynefinDomain::Complicated => ForgeStrategy::CreateWithVariants { variant_count: 2 },
            CynefinDomain::Complex => ForgeStrategy::CreateExperimental,
            CynefinDomain::Chaotic => ForgeStrategy::CreateRapidPrototype,
        };

        PlannedAction {
            gap_id: gap.id,
            strategy,
            max_retries: gap.max_retries(),
            experimental: matches!(
                gap.cynefin_domain,
                CynefinDomain::Complex | CynefinDomain::Chaotic
            ),
            action_label: format!("module.create.{}", gap.cynefin_domain.label()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use project_e_core::types::*;

    fn test_gap(cynefin: CynefinDomain) -> GapReport {
        GapReport::new(
            Uuid::new_v4(),
            GapClassification::Gap,
            0.9,
            cynefin,
            WardleyStage::Custom,
            Priority::High,
            LifecycleAction::Create,
            "Test gap".to_string(),
        )
    }

    fn test_self_model(mode: OperatingMode) -> SystemSelfModel {
        SystemSelfModel {
            coverage_score: 0.5,
            stability_score: 0.5,
            efficiency_score: 0.5,
            utilization_score: 0.5,
            freshness_signal: 0.5,
            operating_mode: mode,
            reflection_window: vec![],
        }
    }

    #[test]
    fn recovery_mode_defers_everything() {
        let engine = DeliberationEngine::new(LimitsConfig::default());
        let gaps = vec![test_gap(CynefinDomain::Simple)];
        let model = test_self_model(OperatingMode::Recovery);

        let plan = engine.deliberate(Uuid::new_v4(), gaps, &model);
        assert!(plan.actions.is_empty());
        assert_eq!(plan.deferred.len(), 1);
    }

    #[test]
    fn cautious_mode_limits_to_simple() {
        let engine = DeliberationEngine::new(LimitsConfig::default());
        let gaps = vec![
            test_gap(CynefinDomain::Simple),
            test_gap(CynefinDomain::Complex),
            test_gap(CynefinDomain::Simple),
        ];
        let model = test_self_model(OperatingMode::Cautious);

        let plan = engine.deliberate(Uuid::new_v4(), gaps, &model);
        assert_eq!(plan.actions.len(), 2); // max 2 in cautious
        assert!(!plan.deferred.is_empty());
    }

    #[test]
    fn nominal_mode_plans_all() {
        let engine = DeliberationEngine::new(LimitsConfig {
            max_actions_per_cycle: 10,
            ..LimitsConfig::default()
        });
        let gaps = vec![
            test_gap(CynefinDomain::Simple),
            test_gap(CynefinDomain::Complicated),
            test_gap(CynefinDomain::Complex),
        ];
        let model = test_self_model(OperatingMode::Nominal);

        let plan = engine.deliberate(Uuid::new_v4(), gaps, &model);
        assert_eq!(plan.actions.len(), 3);
        assert!(plan.deferred.is_empty());
    }

    #[test]
    fn cynefin_maps_to_strategy() {
        let engine = DeliberationEngine::new(LimitsConfig::default());
        let model = test_self_model(OperatingMode::Nominal);

        let plan = engine.deliberate(
            Uuid::new_v4(),
            vec![test_gap(CynefinDomain::Complicated)],
            &model,
        );
        assert!(matches!(
            plan.actions[0].strategy,
            ForgeStrategy::CreateWithVariants { variant_count: 2 }
        ));
    }
}
