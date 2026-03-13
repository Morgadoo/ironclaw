//! Three-tier progressive disclosure for skill activation.
//!
//! Tier 1: Always injected (skill catalog summary).
//! Tier 2: Activated by keyword/pattern matching.
//! Tier 3: Explicit user activation.

use crate::catalog::SkillDefinition;

/// Activation tier for a skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationTier {
    /// Always injected as a brief catalog entry.
    Tier1,
    /// Activated by keyword/pattern matching.
    Tier2,
    /// Explicitly activated by the user or system.
    Tier3,
}

/// Determines the activation tier for a skill based on context.
pub fn determine_tier(skill: &SkillDefinition, context: &str) -> ActivationTier {
    let context_lower = context.to_ascii_lowercase();

    // Check patterns first (higher specificity)
    for pattern in &skill.patterns {
        if context_lower.contains(&pattern.to_ascii_lowercase()) {
            return ActivationTier::Tier2;
        }
    }

    // Check keywords
    let keyword_hits = skill
        .keywords
        .iter()
        .filter(|k| context_lower.contains(&k.to_ascii_lowercase()))
        .count();

    if keyword_hits >= 2 {
        ActivationTier::Tier2
    } else {
        ActivationTier::Tier1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SkillDefinition;

    #[test]
    fn pattern_match_activates_tier2() {
        let skill = SkillDefinition {
            name: "deploy".to_string(),
            patterns: vec!["deploy to production".to_string()],
            keywords: vec!["deploy".to_string()],
            ..SkillDefinition::default()
        };

        assert_eq!(
            determine_tier(&skill, "please deploy to production"),
            ActivationTier::Tier2
        );
    }

    #[test]
    fn keyword_match_activates_tier2() {
        let skill = SkillDefinition {
            name: "energy".to_string(),
            keywords: vec!["energy".to_string(), "tariff".to_string()],
            ..SkillDefinition::default()
        };

        assert_eq!(
            determine_tier(&skill, "analyze energy tariff data"),
            ActivationTier::Tier2
        );
    }

    #[test]
    fn no_match_stays_tier1() {
        let skill = SkillDefinition {
            name: "weather".to_string(),
            keywords: vec!["weather".to_string(), "forecast".to_string()],
            ..SkillDefinition::default()
        };

        assert_eq!(
            determine_tier(&skill, "analyze energy data"),
            ActivationTier::Tier1
        );
    }
}
