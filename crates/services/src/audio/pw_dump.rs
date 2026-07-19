//! Parse `pw-dump` JSON for Audio/Sink + Audio/Source nodes and session defaults.
//!
//! Pure functions — unit-tested against a real-machine fixture (no live PipeWire
//! required in tests). Live path: `run_pw_dump()` shells out.

use std::process::Command;

use serde_json::Value;

use super::types::AudioDevice;

/// Shell out to `pw-dump` and return stdout.
pub fn run_pw_dump() -> anyhow::Result<String> {
    let output = Command::new("pw-dump")
        .output()
        .map_err(|e| anyhow::anyhow!("failed to spawn pw-dump: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("pw-dump failed ({}): {}", output.status, stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Parse full `pw-dump` JSON into sink/source device lists with `is_default` set.
pub fn parse_pw_dump_devices(json: &str) -> anyhow::Result<(Vec<AudioDevice>, Vec<AudioDevice>)> {
    let root: Value = serde_json::from_str(json)
        .map_err(|e| anyhow::anyhow!("pw-dump JSON parse: {e}"))?;
    let arr = root
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("pw-dump root is not an array"))?;

    let (default_sink, default_source) = extract_defaults(arr);
    let mut sinks = Vec::new();
    let mut sources = Vec::new();

    for obj in arr {
        let Some(id) = obj.get("id").and_then(|v| v.as_u64()).map(|n| n as u32) else {
            continue;
        };
        let props = obj
            .get("info")
            .and_then(|i| i.get("props"))
            .and_then(|p| p.as_object());
        let Some(props) = props else {
            continue;
        };
        let media_class = props
            .get("media.class")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let node_name = props
            .get("node.name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let description = props
            .get("node.description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        // Fallback label if description missing.
        let name = if description.is_empty() {
            node_name.clone()
        } else {
            description
        };

        if is_audio_sink(media_class) {
            let is_default = default_sink
                .as_ref()
                .is_some_and(|d| d == &node_name);
            sinks.push(AudioDevice {
                id,
                name,
                node_name,
                is_default,
            });
        } else if is_audio_source(media_class) {
            let is_default = default_source
                .as_ref()
                .is_some_and(|d| d == &node_name);
            sources.push(AudioDevice {
                id,
                name,
                node_name,
                is_default,
            });
        }
    }

    Ok((sinks, sources))
}

fn is_audio_sink(media_class: &str) -> bool {
    media_class == "Audio/Sink" || media_class.starts_with("Audio/Sink/")
}

fn is_audio_source(media_class: &str) -> bool {
    // Includes `Audio/Source/Virtual` (Easy Effects Source, etc.).
    media_class == "Audio/Source" || media_class.starts_with("Audio/Source/")
}

/// Extract default sink/source `node.name` from Metadata objects.
fn extract_defaults(arr: &[Value]) -> (Option<String>, Option<String>) {
    let mut sink = None;
    let mut source = None;
    for obj in arr {
        let ty = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if ty != "PipeWire:Interface:Metadata" {
            continue;
        }
        let Some(meta) = obj.get("metadata").and_then(|m| m.as_array()) else {
            continue;
        };
        for entry in meta {
            let key = entry.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let name = metadata_value_name(entry.get("value"));
            match key {
                "default.audio.sink" => {
                    if sink.is_none() {
                        sink = name;
                    }
                }
                "default.audio.source" => {
                    if source.is_none() {
                        source = name;
                    }
                }
                _ => {}
            }
        }
    }
    (sink, source)
}

/// Metadata values are usually `{ "name": "..." }` (Spa:String:JSON). Also
/// accept a bare string or a JSON-encoded string of that object.
fn metadata_value_name(value: Option<&Value>) -> Option<String> {
    let value = value?;
    if let Some(obj) = value.as_object() {
        return obj
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }
    if let Some(s) = value.as_str() {
        // Sometimes still a JSON string: `{"name":"..."}`.
        if let Ok(inner) = serde_json::from_str::<Value>(s) {
            if let Some(name) = inner.get("name").and_then(|v| v.as_str()) {
                return Some(name.to_string());
            }
        }
        if !s.is_empty() {
            return Some(s.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("fixtures/pw_dump_sample.json");

    #[test]
    fn parse_fixture_lists_sinks_and_sources() {
        let (sinks, sources) = parse_pw_dump_devices(FIXTURE).expect("fixture parses");
        assert!(
            sinks.len() >= 2,
            "expected ≥2 sinks in fixture, got {}",
            sinks.len()
        );
        assert!(
            sources.len() >= 2,
            "expected ≥2 sources (incl virtual), got {}",
            sources.len()
        );

        let built_in = sinks
            .iter()
            .find(|d| d.name.contains("Built-in Audio Analog Stereo"))
            .expect("built-in sink present");
        assert!(
            built_in.is_default,
            "Built-in should be default sink in fixture"
        );
        assert_eq!(
            built_in.node_name,
            "alsa_output.pci-0000_00_1f.3.analog-stereo"
        );

        let ee_sink = sinks
            .iter()
            .find(|d| d.node_name == "easyeffects_sink")
            .expect("EE sink");
        assert!(!ee_sink.is_default);

        let ee_source = sources
            .iter()
            .find(|d| d.node_name == "easyeffects_source")
            .expect("EE source");
        assert!(ee_source.is_default, "EE source is default in fixture");
        assert_eq!(ee_source.name, "Easy Effects Source");
    }

    #[test]
    fn parse_empty_array_ok() {
        let (s, c) = parse_pw_dump_devices("[]").unwrap();
        assert!(s.is_empty());
        assert!(c.is_empty());
    }

    #[test]
    fn parse_garbage_errors() {
        assert!(parse_pw_dump_devices("not json").is_err());
    }
}
