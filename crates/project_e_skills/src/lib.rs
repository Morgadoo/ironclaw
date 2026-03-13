//! Project E Skills — SKILL.md discovery, activation, and authoring.

pub mod activation;
pub mod catalog;

/// Trust levels for skills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkillTrust {
    /// User-placed skills with full tool access.
    Trusted,
    /// Registry-installed skills with restricted tool access.
    #[default]
    Installed,
}
