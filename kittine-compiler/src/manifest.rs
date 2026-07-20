//! `kittine.toml`: the manifest for a Kittine package -- its own name and
//! version, plus the other packages it depends on. Deliberately small:
//! one `[package]` table and a flat `name = "version"` dependency map, no
//! semver ranges (an exact version, or omitted to mean "whatever `add`
//! resolved to latest at the time"), matching the same "no more than the
//! feature actually needs yet" scope the rest of the compiler holds to.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub package: Package,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

impl Manifest {
    pub const FILE_NAME: &'static str = "kittine.toml";

    pub fn load(dir: &Path) -> Result<Manifest, String> {
        let path = dir.join(Self::FILE_NAME);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("failed to read '{}': {e}", path.display()))?;
        toml::from_str(&text)
            .map_err(|e| format!("failed to parse '{}': {e}", path.display()))
    }

    pub fn save(&self, dir: &Path) -> Result<(), String> {
        let path = dir.join(Self::FILE_NAME);
        let text = toml::to_string_pretty(self)
            .map_err(|e| format!("failed to serialize '{}': {e}", path.display()))?;
        std::fs::write(&path, text)
            .map_err(|e| format!("failed to write '{}': {e}", path.display()))
    }

    /// A minimal manifest for `dir` when it has no `kittine.toml` yet,
    /// named after `dir` itself -- mirrors `cargo add` silently
    /// initializing a bare `Cargo.toml` the first time it's needed rather
    /// than requiring a separate `init` step first.
    pub fn init_for_dir(dir: &Path) -> Manifest {
        let name = dir
            .canonicalize()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "kittine-package".to_string());
        Manifest {
            package: Package {
                name,
                version: "0.1.0".to_string(),
                description: String::new(),
            },
            dependencies: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_toml() {
        let mut deps = BTreeMap::new();
        deps.insert("kittine-http".to_string(), "0.1.0".to_string());
        let m = Manifest {
            package: Package {
                name: "my-app".to_string(),
                version: "0.2.0".to_string(),
                description: "an app".to_string(),
            },
            dependencies: deps,
        };
        let text = toml::to_string_pretty(&m).unwrap();
        let parsed: Manifest = toml::from_str(&text).unwrap();
        assert_eq!(parsed.package.name, "my-app");
        assert_eq!(parsed.dependencies.get("kittine-http").unwrap(), "0.1.0");
    }

    #[test]
    fn dependencies_default_to_empty_when_omitted() {
        let text = "[package]\nname = \"x\"\nversion = \"0.1.0\"\n";
        let parsed: Manifest = toml::from_str(text).unwrap();
        assert!(parsed.dependencies.is_empty());
    }
}
