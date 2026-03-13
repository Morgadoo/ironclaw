//! Module capabilities manifest generation.
//!
//! Each forged module gets a `capabilities.json` sidecar that constrains
//! what the module can do at runtime.

use serde::{Deserialize, Serialize};

/// Network access policy for a module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPolicy {
    /// No network access.
    DenyAll,
    /// Only whitelisted domains.
    AllowListed,
}

/// Capabilities manifest generated per module by the forge pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub allow_http: bool,
    pub allow_domains: Vec<String>,
    pub allow_secrets: Vec<String>,
    pub max_http_requests: u32,
    pub max_memory_bytes: u64,
    pub timeout_secs: u32,
    pub network_policy: NetworkPolicy,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            allow_http: false,
            allow_domains: vec![],
            allow_secrets: vec![],
            max_http_requests: 50,
            max_memory_bytes: 128 * 1024 * 1024, // 128MB
            timeout_secs: 30,
            network_policy: NetworkPolicy::DenyAll,
        }
    }
}

impl Capabilities {
    /// Generate capabilities from gap report properties.
    pub fn from_web_access(requires_web: bool, domains: Vec<String>) -> Self {
        Self {
            allow_http: requires_web,
            allow_domains: domains,
            network_policy: if requires_web {
                NetworkPolicy::AllowListed
            } else {
                NetworkPolicy::DenyAll
            },
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_denies_network() {
        let caps = Capabilities::default();
        assert!(!caps.allow_http);
        assert_eq!(caps.network_policy, NetworkPolicy::DenyAll);
    }

    #[test]
    fn web_access_caps() {
        let caps = Capabilities::from_web_access(true, vec!["api.example.com".to_string()]);
        assert!(caps.allow_http);
        assert_eq!(caps.network_policy, NetworkPolicy::AllowListed);
        assert_eq!(caps.allow_domains.len(), 1);
    }

    #[test]
    fn serialization_round_trip() {
        let caps = Capabilities::default();
        let json = serde_json::to_string(&caps).unwrap();
        let parsed: Capabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.max_memory_bytes, caps.max_memory_bytes);
    }
}
