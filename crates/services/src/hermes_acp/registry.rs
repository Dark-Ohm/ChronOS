use std::collections::HashMap;
use std::fs;

use serde::{Deserialize, Serialize};
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
#[derive(Debug, Clone, Deserialize, Serialize)]
struct AgentToml {
    id: String,
    display_name: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
}

/// TOML root schema: `[[agents]]` array.
#[derive(Debug, Clone, Deserialize, Serialize)]
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

/// Path to `~/.config/chronos/agents.toml`.
pub fn agents_config_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~/.config"))
        .join("chronos/agents.toml")
}

/// Add an agent entry to `agents.toml`. If the file doesn't exist, it's created.
/// If an agent with the same `id` already exists, it's overwritten (upsert).
pub fn add_agent(id: &str, display_name: &str, command: &str, args: &[String]) -> Result<(), String> {
    let path = agents_config_path();
    let mut config: AgentsConfig = match fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content).map_err(|e| format!("Parse error: {e}"))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => AgentsConfig { agents: Vec::new() },
        Err(e) => return Err(format!("Read error: {e}")),
    };

    let entry = AgentToml {
        id: id.to_string(),
        display_name: display_name.to_string(),
        command: command.to_string(),
        args: args.to_vec(),
    };

    // Upsert: replace existing entry with same id, or push.
    if let Some(pos) = config.agents.iter().position(|a| a.id == entry.id) {
        config.agents[pos] = entry;
    } else {
        config.agents.push(entry);
    }

    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Create dir error: {e}"))?;
    }

    let serialized = toml::to_string_pretty(&config).map_err(|e| format!("Serialize error: {e}"))?;
    fs::write(&path, serialized).map_err(|e| format!("Write error: {e}"))?;
    tracing::info!(%id, "agents.toml: agent added/updated");
    Ok(())
}

/// Remove an agent entry from `agents.toml` by id.
/// Returns `Ok(true)` if the entry was found and removed, `Ok(false)` if
/// it wasn't in the file, or `Err` on I/O/parse errors.
pub fn remove_agent(id: &str) -> Result<bool, String> {
    let path = agents_config_path();
    let mut config: AgentsConfig = match fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content).map_err(|e| format!("Parse error: {e}"))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(format!("Read error: {e}")),
    };

    let len_before = config.agents.len();
    config.agents.retain(|a| a.id != id);
    if config.agents.len() == len_before {
        return Ok(false); // Not found.
    }

    let serialized = toml::to_string_pretty(&config).map_err(|e| format!("Serialize error: {e}"))?;
    fs::write(&path, serialized).map_err(|e| format!("Write error: {e}"))?;
    tracing::info!(%id, "agents.toml: agent removed");
    Ok(true)
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
