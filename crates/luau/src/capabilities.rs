use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct Capabilities {
    pub fs: bool,
    pub spawn: bool,
    pub network: bool,
    pub ipc: bool,
}

#[derive(Debug, Clone)]
pub struct Manifest {
    pub meta: PluginMeta,
    pub capabilities: Capabilities,
    pub unsafe_mode: bool,
}

#[derive(serde::Deserialize)]
struct ManifestFile {
    plugin: ManifestPlugin,
}

#[derive(serde::Deserialize)]
struct ManifestPlugin {
    name: String,
    version: Option<String>,
    author: Option<String>,
    description: Option<String>,
    capabilities: Option<ManifestCapabilities>,
    #[serde(rename = "unsafe")]
    unsafe_mode: Option<bool>,
}

#[derive(serde::Deserialize, Default)]
struct ManifestCapabilities {
    fs: Option<bool>,
    spawn: Option<bool>,
    network: Option<bool>,
    ipc: Option<bool>,
}

impl Manifest {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let file: ManifestFile = toml::from_str(&content)?;
        let caps = file.plugin.capabilities.unwrap_or_default();
        Ok(Manifest {
            meta: PluginMeta {
                name: file.plugin.name,
                version: file.plugin.version.unwrap_or_default(),
                author: file.plugin.author.unwrap_or_default(),
                description: file.plugin.description.unwrap_or_default(),
            },
            capabilities: Capabilities {
                fs: caps.fs.unwrap_or(false),
                spawn: caps.spawn.unwrap_or(false),
                network: caps.network.unwrap_or(false),
                ipc: caps.ipc.unwrap_or(false),
            },
            unsafe_mode: file.plugin.unsafe_mode.unwrap_or(false),
        })
    }

    /// Check if a capability is granted (either via manifest or unsafe mode).
    pub fn has_capability(&self, cap: &str) -> bool {
        if self.unsafe_mode {
            return true;
        }
        match cap {
            "fs" => self.capabilities.fs,
            "spawn" => self.capabilities.spawn,
            "network" => self.capabilities.network,
            "ipc" => self.capabilities.ipc,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_valid_manifest() {
        let dir = std::env::temp_dir().join("chronos_test_manifest");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("manifest.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"[plugin]
name = "clock"
version = "0.1.0"
author = "Test"
description = "A test plugin"

[plugin.capabilities]
fs = true
spawn = false

unsafe = false"#).unwrap();

        let m = Manifest::from_path(&path).unwrap();
        assert_eq!(m.meta.name, "clock");
        assert!(m.capabilities.fs);
        assert!(!m.capabilities.spawn);
        assert!(!m.unsafe_mode);
        assert!(m.has_capability("fs"));
        assert!(!m.has_capability("spawn"));
    }

    #[test]
    fn parse_minimal_manifest() {
        let dir = std::env::temp_dir().join("chronos_test_manifest_min");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("manifest.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "[plugin]\nname = \"minimal\"").unwrap();

        let m = Manifest::from_path(&path).unwrap();
        assert_eq!(m.meta.name, "minimal");
        assert!(!m.capabilities.fs);
        assert!(!m.unsafe_mode);
    }

    #[test]
    fn unsafe_enables_all() {
        let dir = std::env::temp_dir().join("chronos_test_manifest_unsafe");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("manifest.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "[plugin]\nname = \"unsafe_p\"\nunsafe = true").unwrap();

        let m = Manifest::from_path(&path).unwrap();
        assert!(m.has_capability("fs"));
        assert!(m.has_capability("spawn"));
        assert!(m.has_capability("network"));
        assert!(m.has_capability("ipc"));
    }

    #[test]
    fn invalid_manifest_returns_error() {
        let dir = std::env::temp_dir().join("chronos_test_manifest_bad");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("manifest.toml");
        std::fs::write(&path, "not valid toml {{{{").unwrap();

        assert!(Manifest::from_path(&path).is_err());
    }
}
