//! Skill catalog — discovers and indexes SKILL.md files.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::SkillTrust;

/// A discovered skill definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub name: String,
    pub version: String,
    pub description: String,
    pub path: PathBuf,
    pub keywords: Vec<String>,
    pub patterns: Vec<String>,
    pub tags: Vec<String>,
    pub max_context_tokens: usize,
    #[serde(skip)]
    pub trust: SkillTrust,
}

impl Default for SkillDefinition {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "0.1.0".to_string(),
            description: String::new(),
            path: PathBuf::new(),
            keywords: vec![],
            patterns: vec![],
            tags: vec![],
            max_context_tokens: 2000,
            trust: SkillTrust::Installed,
        }
    }
}

/// Catalog of available skills.
pub struct SkillCatalog {
    skills: HashMap<String, SkillDefinition>,
}

impl SkillCatalog {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Register a skill in the catalog.
    pub fn register(&mut self, skill: SkillDefinition) {
        self.skills.insert(skill.name.clone(), skill);
    }

    /// Look up a skill by name.
    pub fn get(&self, name: &str) -> Option<&SkillDefinition> {
        self.skills.get(name)
    }

    /// List all skills.
    pub fn list(&self) -> Vec<&SkillDefinition> {
        self.skills.values().collect()
    }

    /// Find skills relevant to a query by keyword scoring.
    pub fn search(&self, query: &str) -> Vec<(&SkillDefinition, f32)> {
        let query_lower = query.to_ascii_lowercase();
        let mut results: Vec<(&SkillDefinition, f32)> = self
            .skills
            .values()
            .filter_map(|skill| {
                let mut score = 0.0f32;

                for keyword in &skill.keywords {
                    if query_lower.contains(&keyword.to_ascii_lowercase()) {
                        score += 10.0;
                    }
                }

                for tag in &skill.tags {
                    if query_lower.contains(&tag.to_ascii_lowercase()) {
                        score += 3.0;
                    }
                }

                if score > 0.0 {
                    Some((skill, score))
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}

impl Default for SkillCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_find() {
        let mut catalog = SkillCatalog::new();
        catalog.register(SkillDefinition {
            name: "rust-module-generation".to_string(),
            keywords: vec![
                "rust".to_string(),
                "module".to_string(),
                "generate".to_string(),
            ],
            tags: vec!["forge".to_string()],
            ..SkillDefinition::default()
        });

        assert!(catalog.get("rust-module-generation").is_some());
        assert!(catalog.get("nonexistent").is_none());
    }

    #[test]
    fn search_by_keywords() {
        let mut catalog = SkillCatalog::new();
        catalog.register(SkillDefinition {
            name: "energy-tariff".to_string(),
            keywords: vec!["energy".to_string(), "tariff".to_string()],
            ..SkillDefinition::default()
        });
        catalog.register(SkillDefinition {
            name: "weather-data".to_string(),
            keywords: vec!["weather".to_string()],
            ..SkillDefinition::default()
        });

        let results = catalog.search("energy tariff analysis");
        assert!(!results.is_empty());
        assert_eq!(results[0].0.name, "energy-tariff");
    }
}
