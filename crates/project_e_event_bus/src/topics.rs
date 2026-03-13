//! Topic taxonomy for the Project E event bus.
//!
//! Topics follow the NATS hierarchical naming convention:
//! `project_e.<domain>.<event_name>`

/// Topic namespace constants for each cognitive domain.
pub mod namespace {
    pub const SYSTEM: &str = "project_e.system";
    pub const PERCEPTION: &str = "project_e.perception";
    pub const ANALYSIS: &str = "project_e.analysis";
    pub const META: &str = "project_e.meta";
    pub const FORGE: &str = "project_e.forge";
    pub const REGISTRY: &str = "project_e.registry";
}

/// Wildcard subscriptions for consuming entire domains.
pub mod wildcards {
    pub const ALL: &str = "project_e.>";
    pub const ALL_SYSTEM: &str = "project_e.system.*";
    pub const ALL_PERCEPTION: &str = "project_e.perception.*";
    pub const ALL_ANALYSIS: &str = "project_e.analysis.*";
    pub const ALL_META: &str = "project_e.meta.*";
    pub const ALL_FORGE: &str = "project_e.forge.*";
    pub const ALL_REGISTRY: &str = "project_e.registry.*";
    pub const ALL_SCRAPERS: &str = "project_e.perception.scraper.*";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_namespaces_are_consistent() {
        assert!(wildcards::ALL_SYSTEM.starts_with(namespace::SYSTEM));
        assert!(wildcards::ALL_FORGE.starts_with(namespace::FORGE));
    }
}
