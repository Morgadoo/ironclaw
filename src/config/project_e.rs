//! Project E cognitive architecture — IronClaw config bridge.
//!
//! Loads Project E settings from env vars and delegates inner configuration
//! to `project_e_core::config::ProjectEConfig::load_layered()`.
//!
//! # Env vars
//! | Variable | Default | Description |
//! |----------|---------|-------------|
//! | `PROJECT_E_ENABLED`            | `false` | Enable the cognitive layer |
//! | `PROJECT_E_CONFIG_DIR`         | `~/.ironclaw/project_e/config` | TOML config directory |
//! | `PROJECT_E_INSTANCE`           | `default` | Config-layering instance name |
//! | `PROJECT_E_BUS_CAPACITY`       | `1024` | In-process event bus channel size |
//! | `PROJECT_E_CYCLE_ON_HEARTBEAT` | `true` | Run cognitive cycle on heartbeat tick |
//! | `PROJECT_E_FORGE_SANDBOX`      | `true` | Isolate forge gates in sandbox |
//! | `PROJECT_E_MAX_LLM_CALLS`      | `20` | LLM call budget per cognitive cycle |

use std::path::PathBuf;

use project_e_core::config::ProjectEConfig;

use crate::config::helpers::{parse_bool_env, parse_optional_env, parse_string_env};
use crate::error::ConfigError;

/// IronClaw-side wrapper for Project E configuration.
#[derive(Debug, Clone)]
pub struct ProjectEIronclawConfig {
    pub enabled: bool,
    pub config_dir: PathBuf,
    pub instance: String,
    pub event_bus_capacity: usize,
    pub cycle_on_heartbeat: bool,
    pub forge_use_sandbox: bool,
    pub max_llm_calls_per_cycle: usize,
    /// Inner cognitive-architecture configuration loaded from TOML files.
    pub inner: ProjectEConfig,
}

impl ProjectEIronclawConfig {
    /// Resolve from environment variables.
    ///
    /// Returns a cheaply-constructed disabled instance when
    /// `PROJECT_E_ENABLED=false` (the default). No TOML files are read
    /// and Project E has zero runtime cost.
    pub fn resolve() -> Result<Self, ConfigError> {
        if !parse_bool_env("PROJECT_E_ENABLED", false)? {
            return Ok(Self::disabled());
        }

        let config_dir: PathBuf =
            parse_string_env("PROJECT_E_CONFIG_DIR", default_config_dir())?.into();
        let instance = parse_string_env("PROJECT_E_INSTANCE", "default")?;
        let event_bus_capacity =
            parse_optional_env::<usize>("PROJECT_E_BUS_CAPACITY")?.unwrap_or(1024);
        let cycle_on_heartbeat = parse_bool_env("PROJECT_E_CYCLE_ON_HEARTBEAT", true)?;
        let forge_use_sandbox = parse_bool_env("PROJECT_E_FORGE_SANDBOX", true)?;
        let max_llm_calls_per_cycle =
            parse_optional_env::<usize>("PROJECT_E_MAX_LLM_CALLS")?.unwrap_or(20);

        let inner = ProjectEConfig::load_layered(&config_dir, &instance)
            .map_err(|e| ConfigError::ParseError(format!("Project E config: {e}")))?;

        Ok(Self {
            enabled: true,
            config_dir,
            instance,
            event_bus_capacity,
            cycle_on_heartbeat,
            forge_use_sandbox,
            max_llm_calls_per_cycle,
            inner,
        })
    }

    /// A disabled (no-op) instance that skips all file I/O.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            config_dir: default_config_dir().into(),
            instance: "default".to_string(),
            event_bus_capacity: 1024,
            cycle_on_heartbeat: false,
            forge_use_sandbox: false,
            max_llm_calls_per_cycle: 20,
            inner: ProjectEConfig::default(),
        }
    }
}

fn default_config_dir() -> String {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ironclaw")
        .join("project_e")
        .join("config")
        .to_string_lossy()
        .into_owned()
}
