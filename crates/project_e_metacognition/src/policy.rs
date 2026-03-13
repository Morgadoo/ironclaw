//! Glob-pattern tool policy engine.
//!
//! Evaluates planned actions against configurable policy rules.
//! First-match-wins semantics; falls back to Allow if no rule matches.

use serde::{Deserialize, Serialize};

use project_e_core::config::PolicyRuleConfig;

use crate::deliberation::{ActionPlan, DeferredGap, PlannedAction};

/// Policy decision for an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    /// Proceed autonomously.
    Allow,
    /// Require human confirmation.
    Ask,
    /// Never allowed without explicit config change.
    Deny,
}

/// A compiled policy rule with glob matching.
#[derive(Debug, Clone)]
pub struct PolicyRule {
    pub pattern: glob::Pattern,
    pub action: PolicyAction,
    pub min_confidence: Option<f32>,
}

/// Evaluates planned actions against policy rules.
pub struct PolicyEngine {
    rules: Vec<PolicyRule>,
}

impl PolicyEngine {
    /// Build from configuration.
    pub fn from_config(rules: &[PolicyRuleConfig]) -> Self {
        let compiled_rules = rules
            .iter()
            .filter_map(|r| {
                let pattern = glob::Pattern::new(&r.pattern).ok()?;
                let action = match r.action.as_str() {
                    "allow" => PolicyAction::Allow,
                    "ask" => PolicyAction::Ask,
                    "deny" => PolicyAction::Deny,
                    _ => return None,
                };
                let min_confidence = r.condition.as_ref().and_then(|c| c.min_confidence);
                Some(PolicyRule {
                    pattern,
                    action,
                    min_confidence,
                })
            })
            .collect();

        Self {
            rules: compiled_rules,
        }
    }

    /// Evaluate a single action against policy rules.
    /// First-match-wins. Falls back to Allow if no rule matches.
    pub fn evaluate(&self, action: &PlannedAction, confidence: f32) -> PolicyAction {
        for rule in &self.rules {
            if rule.pattern.matches(&action.action_label) {
                // Check condition if present
                if let Some(min_conf) = rule.min_confidence
                    && confidence < min_conf
                {
                    return PolicyAction::Deny;
                }
                return rule.action;
            }
        }
        PolicyAction::Allow
    }

    /// Filter an action plan through the policy engine.
    /// Denied/asked actions are moved to deferred.
    pub fn filter_plan(
        &self,
        plan: &mut ActionPlan,
        confidences: &std::collections::HashMap<uuid::Uuid, f32>,
    ) {
        let mut kept = Vec::new();
        for action in plan.actions.drain(..) {
            let confidence = confidences.get(&action.gap_id).copied().unwrap_or(1.0);
            match self.evaluate(&action, confidence) {
                PolicyAction::Allow => kept.push(action),
                PolicyAction::Deny => {
                    plan.deferred.push(DeferredGap {
                        gap_id: action.gap_id,
                        reason: format!("Policy denied: {}", action.action_label),
                    });
                }
                PolicyAction::Ask => {
                    plan.deferred.push(DeferredGap {
                        gap_id: action.gap_id,
                        reason: format!("Policy requires confirmation: {}", action.action_label),
                    });
                }
            }
        }
        plan.actions = kept;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deliberation::ForgeStrategy;

    fn default_engine() -> PolicyEngine {
        PolicyEngine::from_config(&project_e_core::config::PoliciesConfig::default().rules)
    }

    fn make_action(label: &str) -> PlannedAction {
        PlannedAction {
            gap_id: uuid::Uuid::new_v4(),
            strategy: ForgeStrategy::CreateFromTemplate,
            max_retries: 3,
            experimental: false,
            action_label: label.to_string(),
        }
    }

    #[test]
    fn allow_simple_create() {
        let engine = default_engine();
        let action = make_action("module.create.simple");
        assert_eq!(engine.evaluate(&action, 0.9), PolicyAction::Allow);
    }

    #[test]
    fn deny_module_remove() {
        let engine = default_engine();
        let action = make_action("module.remove.old_module");
        assert_eq!(engine.evaluate(&action, 1.0), PolicyAction::Deny);
    }

    #[test]
    fn ask_module_deprecate() {
        let engine = default_engine();
        let action = make_action("module.deprecate.old_module");
        assert_eq!(engine.evaluate(&action, 1.0), PolicyAction::Ask);
    }

    #[test]
    fn complex_create_requires_confidence() {
        let engine = default_engine();
        let action = make_action("module.create.complex");

        // High confidence → allow
        assert_eq!(engine.evaluate(&action, 0.9), PolicyAction::Allow);
        // Low confidence → deny (condition check fails)
        assert_eq!(engine.evaluate(&action, 0.5), PolicyAction::Deny);
    }

    #[test]
    fn unknown_action_defaults_to_allow() {
        let engine = default_engine();
        let action = make_action("custom.action.something");
        assert_eq!(engine.evaluate(&action, 0.5), PolicyAction::Allow);
    }

    #[test]
    fn filter_plan_removes_denied() {
        let engine = default_engine();
        let mut plan = ActionPlan {
            id: uuid::Uuid::new_v4(),
            cycle_id: uuid::Uuid::new_v4(),
            actions: vec![
                make_action("module.create.simple"),
                make_action("module.remove.bad"),
                make_action("module.create.complicated"),
            ],
            deferred: vec![],
            estimated_llm_cost: 0.0,
            max_concurrent: 3,
        };

        let confidences = std::collections::HashMap::new();
        engine.filter_plan(&mut plan, &confidences);

        assert_eq!(plan.actions.len(), 2); // simple + complicated
        assert_eq!(plan.deferred.len(), 1); // remove denied
    }
}
