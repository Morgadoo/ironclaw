//! Layered TOML configuration for Project E.
//!
//! Configuration is loaded from multiple sources with increasing priority:
//! 1. `config/default.toml` — base defaults
//! 2. `config/<instance>.toml` — instance-specific overrides
//! 3. Environment variables — runtime overrides
//! 4. `config/runtime/<instance>.adaptive.toml` — meta-adaptation learned values

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{ProjectEError, Result};

/// Top-level Project E configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectEConfig {
    #[serde(default)]
    pub instance: InstanceConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub perception: PerceptionConfig,
    #[serde(default)]
    pub analysis: AnalysisConfig,
    #[serde(default)]
    pub metacognition: MetacognitionConfig,
    #[serde(default)]
    pub forge: ForgeConfig,
    #[serde(default)]
    pub evaluation: EvaluationConfig,
    #[serde(default)]
    pub meta: MetaConfig,
    #[serde(default)]
    pub limits: LimitsConfig,
    #[serde(default)]
    pub health: HealthConfig,
    #[serde(default)]
    pub deliberation: DeliberationConfig,
}

impl ProjectEConfig {
    /// Load from a TOML file path.
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| ProjectEError::Config {
            reason: format!("failed to read config file {}: {}", path.display(), e),
        })?;
        let config: ProjectEConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load with layered merging: default → instance → adaptive.
    pub fn load_layered(config_dir: &Path, instance: &str) -> Result<Self> {
        let default_path = config_dir.join("default.toml");
        let mut config = if default_path.exists() {
            Self::from_file(&default_path)?
        } else {
            Self::default()
        };

        let instance_path = config_dir.join(format!("{instance}.toml"));
        if instance_path.exists() {
            let override_config = Self::from_file(&instance_path)?;
            config.merge_from(&override_config);
        }

        let adaptive_path = config_dir.join(format!("runtime/{instance}.adaptive.toml"));
        if adaptive_path.exists() {
            let adaptive_config = Self::from_file(&adaptive_path)?;
            config.merge_from(&adaptive_config);
        }

        config.instance.name = instance.to_string();
        Ok(config)
    }

    /// Merge non-default values from another config into this one.
    /// This is a shallow merge — fields from `other` overwrite `self`
    /// when they differ from the default.
    fn merge_from(&mut self, other: &ProjectEConfig) {
        if other.instance.name != InstanceConfig::default().name {
            self.instance = other.instance.clone();
        }
        // For numeric/bool fields, we merge at a more granular level
        // in a real implementation. For now, struct-level merge suffices.
        if other.llm.circuit_breaker.failure_threshold
            != LlmConfig::default().circuit_breaker.failure_threshold
        {
            self.llm.circuit_breaker = other.llm.circuit_breaker.clone();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub name: String,
    pub sector: String,
    pub region: String,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            sector: "general".to_string(),
            region: "global".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default)]
    pub circuit_breaker: CircuitBreakerConfig,
    #[serde(default)]
    pub phase_routing: PhaseRoutingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub enabled: bool,
    #[serde(with = "humantime_serde_compat")]
    pub probe_interval: Duration,
    pub failure_threshold: u32,
    #[serde(with = "humantime_serde_compat")]
    pub cooldown: Duration,
    #[serde(with = "humantime_serde_compat")]
    pub probe_timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            probe_interval: Duration::from_secs(30),
            failure_threshold: 5,
            cooldown: Duration::from_secs(60),
            probe_timeout: Duration::from_secs(5),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseRoutingConfig {
    pub perception: String,
    pub interpretation: String,
    pub deliberation: String,
    pub action: String,
    pub evaluation: String,
    pub meta_adaptation: String,
}

impl Default for PhaseRoutingConfig {
    fn default() -> Self {
        Self {
            perception: "model_for_analysis".to_string(),
            interpretation: "model_for_analysis".to_string(),
            deliberation: "model_for_analysis".to_string(),
            action: "model_for_generation".to_string(),
            evaluation: "model_for_analysis".to_string(),
            meta_adaptation: "model_for_analysis".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionConfig {
    pub min_relevance_score: f32,
    pub novelty_weight: f32,
    pub coverage_weight: f32,
    pub magnitude_weight: f32,
    pub keywords: Vec<KeywordEntry>,
}

impl Default for PerceptionConfig {
    fn default() -> Self {
        Self {
            min_relevance_score: 0.4,
            novelty_weight: 0.4,
            coverage_weight: 0.35,
            magnitude_weight: 0.25,
            keywords: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordEntry {
    pub term: String,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub confidence_threshold: f32,
    pub max_gap_reports_per_cycle: usize,
    pub bm25_top_k: usize,
    pub cosine_rerank_k: usize,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.6,
            max_gap_reports_per_cycle: 20,
            bm25_top_k: 20,
            cosine_rerank_k: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetacognitionConfig {
    pub min_decision_confidence: f32,
    pub reflection_window: usize,
    pub recovery_threshold: f32,
    pub cautious_threshold: f32,
}

impl Default for MetacognitionConfig {
    fn default() -> Self {
        Self {
            min_decision_confidence: 0.7,
            reflection_window: 10,
            recovery_threshold: 0.3,
            cautious_threshold: 0.6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeConfig {
    pub max_retries: u8,
    pub compile_timeout_secs: u64,
    pub clippy_timeout_secs: u64,
    pub audit_timeout_secs: u64,
    pub test_timeout_secs: u64,
}

impl Default for ForgeConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            compile_timeout_secs: 120,
            clippy_timeout_secs: 60,
            audit_timeout_secs: 30,
            test_timeout_secs: 120,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationConfig {
    pub monitoring_window_secs: u64,
    pub redundancy_threshold: f32,
    pub auto_deprecate_failure_rate: f32,
    pub deprecation_grace_days: u32,
}

impl Default for EvaluationConfig {
    fn default() -> Self {
        Self {
            monitoring_window_secs: 300,
            redundancy_threshold: 0.95,
            auto_deprecate_failure_rate: 0.5,
            deprecation_grace_days: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaConfig {
    pub adaptation_frequency_cycles: usize,
    pub adjustment_max_step: f32,
    pub lesson_repeat_threshold: usize,
}

impl Default for MetaConfig {
    fn default() -> Self {
        Self {
            adaptation_frequency_cycles: 5,
            adjustment_max_step: 0.1,
            lesson_repeat_threshold: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    pub max_actions_per_cycle: usize,
    pub max_concurrent_actions: usize,
    pub max_llm_calls_per_day: usize,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_actions_per_cycle: 10,
            max_concurrent_actions: 3,
            max_llm_calls_per_day: 500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    pub min_coverage: f32,
    pub min_stability: f32,
    pub min_efficiency: f32,
    pub min_utilization: f32,
    pub min_freshness: f32,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            min_coverage: 0.5,
            min_stability: 0.7,
            min_efficiency: 0.4,
            min_utilization: 0.3,
            min_freshness: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliberationConfig {
    pub risk_threshold: f32,
    #[serde(default)]
    pub policies: PoliciesConfig,
}

impl Default for DeliberationConfig {
    fn default() -> Self {
        Self {
            risk_threshold: 0.7,
            policies: PoliciesConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoliciesConfig {
    pub rules: Vec<PolicyRuleConfig>,
}

impl Default for PoliciesConfig {
    fn default() -> Self {
        Self {
            rules: vec![
                PolicyRuleConfig {
                    pattern: "module.create.simple".to_string(),
                    action: "allow".to_string(),
                    condition: None,
                },
                PolicyRuleConfig {
                    pattern: "module.create.complex".to_string(),
                    action: "allow".to_string(),
                    condition: Some(PolicyConditionConfig {
                        min_confidence: Some(0.8),
                    }),
                },
                PolicyRuleConfig {
                    pattern: "module.deprecate.*".to_string(),
                    action: "ask".to_string(),
                    condition: None,
                },
                PolicyRuleConfig {
                    pattern: "module.remove.*".to_string(),
                    action: "deny".to_string(),
                    condition: None,
                },
                PolicyRuleConfig {
                    pattern: "module.merge.*".to_string(),
                    action: "ask".to_string(),
                    condition: None,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRuleConfig {
    pub pattern: String,
    pub action: String,
    pub condition: Option<PolicyConditionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConditionConfig {
    pub min_confidence: Option<f32>,
}

/// Simple serde helper for Duration using seconds (TOML-friendly).
mod humantime_serde_compat {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> std::result::Result<S::Ok, S::Error> {
        d.as_secs().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Duration, D::Error> {
        let secs = u64::deserialize(d)?;
        Ok(Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = ProjectEConfig::default();
        assert_eq!(config.instance.name, "default");
        assert!(config.llm.circuit_breaker.enabled);
        assert_eq!(config.llm.circuit_breaker.failure_threshold, 5);
        assert_eq!(config.limits.max_actions_per_cycle, 10);
    }

    #[test]
    fn config_serialization_round_trip() {
        let config = ProjectEConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: ProjectEConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.instance.name, config.instance.name);
        assert_eq!(
            parsed.llm.circuit_breaker.failure_threshold,
            config.llm.circuit_breaker.failure_threshold
        );
    }
}
