use std::collections::HashMap;
use std::fs;

use serde::Deserialize;
use tracing::warn;

use super::transport::HermesConfig;

/// Descriptor for an ACP-compatible agent backend.
#[derive(Debug, Clone)]
pub struct AgentDescriptor {
    /// Stable identifier (e.g. "hermes", "cline").
    pub id: String,
    /// Display name shown in the UI (e.g. "Hermes", "Cline").
    pub display_name: String,
    /// Command + args to spawn this backend via ACP stdio.
    pub config: HermesConfig,
}

/// TOML schema for a single agent entry.
#[derive(Debug, Clone, Deserialize)]
struct AgentToml {
    id: String,
    display_name: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
}

/// TOML root schema: `[[agents]]` array.
#[derive(Debug, Clone, Deserialize)]
struct AgentsConfig {
    #[serde(default)]
    agents: Vec<AgentToml>,
}

/// Parse a `.env` file into KEY=VALUE pairs.
///
/// Skips blank lines and lines starting with `#`.
/// Values may be quoted (single or double) — quotes are stripped.
fn parse_env_file(content: &str) -> HashMap<String, String> {
    let mut env = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            let mut value = value.trim().to_string();
            // Strip surrounding quotes.
            if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value = value[1..value.len() - 1].to_string();
            }
            if !key.is_empty() {
                env.insert(key, value);
            }
        }
    }
    env
}

/// Load the shared `.env` file from `~/.config/chronos/.env`.
///
/// Returns an empty map if the file is missing or unparseable.
pub fn load_shared_env() -> HashMap<String, String> {
    let path = match dirs::config_dir() {
        Some(d) => d.join("chronos/.env"),
        None => return HashMap::new(),
    };
    match fs::read_to_string(&path) {
        Ok(content) => {
            let env = parse_env_file(&content);
            if !env.is_empty() {
                tracing::debug!(
                    "Loaded {} env vars from {}",
                    env.len(),
                    path.display()
                );
            }
            env
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // No .env file — not an error, agents use system env.
            HashMap::new()
        }
        Err(e) => {
            warn!("Failed to read {}: {e}", path.display());
            HashMap::new()
        }
    }
}

/// Load agent entries from `~/.config/chronos/agents.toml`.
///
/// Returns an empty vec if the file is missing or unparseable.
fn load_config_agents() -> Vec<AgentToml> {
    let path = match dirs::config_dir() {
        Some(d) => d.join("chronos/agents.toml"),
        None => return Vec::new(),
    };
    match fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<AgentsConfig>(&content) {
            Ok(config) => config.agents,
            Err(e) => {
                warn!("Failed to parse {}: {e}", path.display());
                Vec::new()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => {
            warn!("Failed to read {}: {e}", path.display());
            Vec::new()
        }
    }
}

/// Returns the final list of known ACP-compatible agent backends.
///
/// Builtin agents (Hermes) are always present. Additional agents from
/// `~/.config/chronos/agents.toml` are merged additively. If a config
/// entry has the same `id` as a builtin, it **overrides** the builtin
/// (lets users change command/args without editing source).
pub fn known_agents() -> Vec<AgentDescriptor> {
    let mut agents: Vec<AgentDescriptor> = builtin_agents()
        .into_iter()
        .map(AgentDescriptor::from)
        .collect();

    for entry in load_config_agents() {
        if let Some(pos) = agents.iter().position(|a| a.id == entry.id) {
            agents[pos] = AgentDescriptor::from(entry);
        } else {
            agents.push(AgentDescriptor::from(entry));
        }
    }

    agents
}

/// Built-in agents (always present, no config needed).
fn builtin_agents() -> Vec<AgentToml> {
    vec![AgentToml {
        id: "hermes".to_string(),
        display_name: "Hermes".to_string(),
        command: "hermes".to_string(),
        args: vec!["acp".to_string(), "--accept-hooks".to_string()],
    }]
}

impl From<AgentToml> for AgentDescriptor {
    fn from(toml: AgentToml) -> Self {
        Self {
            id: toml.id,
            display_name: toml.display_name,
            config: HermesConfig {
                command: toml.command,
                args: toml.args,
            },
        }
    }
}
