//! `kittine.lock`: the exact, checksummed set of package versions
//! `install` actually resolved and downloaded -- so a second `install` on
//! another machine gets byte-identical dependencies instead of whatever
//! happens to be latest on the registry that day, the same job Cargo's
//! own `Cargo.lock` does for Rust crates.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Lockfile {
    #[serde(rename = "package", default)]
    pub packages: Vec<LockedPackage>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub checksum: String,
    pub source: String,
}

impl Lockfile {
    pub const FILE_NAME: &'static str = "kittine.lock";

    pub fn load(dir: &Path) -> Result<Lockfile, String> {
        let path = dir.join(Self::FILE_NAME);
        if !path.exists() {
            return Ok(Lockfile::default());
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("failed to read '{}': {e}", path.display()))?;
        toml::from_str(&text)
            .map_err(|e| format!("failed to parse '{}': {e}", path.display()))
    }

    pub fn save(&self, dir: &Path) -> Result<(), String> {
        let mut sorted = self.clone();
        sorted.packages.sort();
        let path = dir.join(Self::FILE_NAME);
        let text = toml::to_string_pretty(&sorted)
            .map_err(|e| format!("failed to serialize '{}': {e}", path.display()))?;
        std::fs::write(&path, text)
            .map_err(|e| format!("failed to write '{}': {e}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_sorts_on_save() {
        let lock = Lockfile {
            packages: vec![
                LockedPackage {
                    name: "z-pkg".to_string(),
                    version: "1.0.0".to_string(),
                    checksum: "sha256:deadbeef".to_string(),
                    source: "registry+https://example".to_string(),
                },
                LockedPackage {
                    name: "a-pkg".to_string(),
                    version: "1.0.0".to_string(),
                    checksum: "sha256:cafef00d".to_string(),
                    source: "registry+https://example".to_string(),
                },
            ],
        };
        let dir = std::env::temp_dir().join(format!(
            "kittine-lockfile-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        lock.save(&dir).unwrap();
        let loaded = Lockfile::load(&dir).unwrap();
        assert_eq!(loaded.packages[0].name, "a-pkg");
        assert_eq!(loaded.packages[1].name, "z-pkg");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_lockfile_loads_as_empty() {
        let dir = std::env::temp_dir().join(format!(
            "kittine-lockfile-missing-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let loaded = Lockfile::load(&dir).unwrap();
        assert!(loaded.packages.is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
