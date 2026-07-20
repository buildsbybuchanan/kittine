//! A thin HTTP client for the Kittine package registry.
//!
//! The registry has no server of its own -- it's the public
//! `buildsbybuchanan/kittine-registry` GitHub repo: one `index/<name>.json`
//! file per package (the list of published versions, each with a tarball
//! URL and a sha256), and the tarballs themselves as GitHub Release
//! assets on that same repo. `install`/`add` only ever do plain,
//! unauthenticated `GET`s against public GitHub URLs -- real HTTP, no
//! token needed, works for anyone. `publish` is the one maintainer-only
//! operation and shells out to the `gh` CLI (already-authenticated,
//! already-trusted local tooling) instead of reimplementing GitHub OAuth.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;
use std::process::Command;

pub const REGISTRY_OWNER: &str = "buildsbybuchanan";
pub const REGISTRY_REPO: &str = "kittine-registry";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageIndex {
    pub name: String,
    #[serde(default)]
    pub versions: Vec<IndexedVersion>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IndexedVersion {
    pub version: String,
    pub tarball: String,
    pub sha256: String,
}

impl PackageIndex {
    /// The version `install`/`add` picks when `kittine.toml` didn't pin
    /// one: whichever was published most recently. `publish` always
    /// appends, so "last in the list" and "newest" already coincide --
    /// no semver-max parsing needed for that.
    pub fn latest(&self) -> Option<&IndexedVersion> {
        self.versions.last()
    }

    pub fn find(&self, version: &str) -> Option<&IndexedVersion> {
        self.versions.iter().find(|v| v.version == version)
    }
}

fn index_url(name: &str) -> String {
    format!(
        "https://raw.githubusercontent.com/{REGISTRY_OWNER}/{REGISTRY_REPO}/main/index/{name}.json"
    )
}

pub fn fetch_index(name: &str) -> Result<PackageIndex, String> {
    let url = index_url(name);
    let resp = ureq::get(&url).call().map_err(|e| match e {
        ureq::Error::Status(404, _) => format!(
            "no package named '{name}' on the registry ({url} returned 404)"
        ),
        other => format!("failed to fetch package index for '{name}' ({url}): {other}"),
    })?;
    resp.into_json::<PackageIndex>()
        .map_err(|e| format!("failed to parse package index for '{name}': {e}"))
}

pub fn download_tarball(url: &str) -> Result<Vec<u8>, String> {
    let resp = ureq::get(url)
        .call()
        .map_err(|e| format!("failed to download '{url}': {e}"))?;
    let mut bytes = Vec::new();
    resp.into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("failed to read tarball body from '{url}': {e}"))?;
    Ok(bytes)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Publishes `tarball_path` (already built by the caller) as `name`@
/// `version`: creates a GitHub Release tagged `<name>-v<version>` on the
/// registry repo with the tarball as its asset (`gh release create`),
/// then appends the new version to that package's `index/<name>.json` via
/// the GitHub Contents API (`gh api`). Returns the tarball's public
/// download URL. Fails loudly if `version` is already published --
/// `publish` never overwrites an existing version.
pub fn publish(name: &str, version: &str, tarball_path: &Path) -> Result<String, String> {
    let repo = format!("{REGISTRY_OWNER}/{REGISTRY_REPO}");
    let tag = format!("{name}-v{version}");
    let title = format!("{name} {version}");
    let tarball_str = tarball_path
        .to_str()
        .ok_or("tarball path is not valid UTF-8")?;

    let status = Command::new("gh")
        .args([
            "release",
            "create",
            &tag,
            tarball_str,
            "--repo",
            &repo,
            "--title",
            &title,
            "--notes",
            "Published by `kittine-compiler publish`.",
        ])
        .status()
        .map_err(|e| format!("failed to run 'gh release create {tag}': {e}"))?;
    if !status.success() {
        return Err(format!("'gh release create {tag}' exited with {status}"));
    }

    let asset_name = tarball_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("tarball path has no file name")?;
    let tarball_url =
        format!("https://github.com/{repo}/releases/download/{tag}/{asset_name}");

    let bytes = std::fs::read(tarball_path)
        .map_err(|e| format!("failed to re-read '{}': {e}", tarball_path.display()))?;
    let sha256 = sha256_hex(&bytes);

    update_index(name, version, &tarball_url, &sha256)?;
    Ok(tarball_url)
}

/// Appends one version to `index/<name>.json` on the registry repo via
/// the GitHub Contents API, creating the file on the first publish of a
/// brand-new package. Uses `gh api` (not a local registry checkout) so
/// `publish` never needs to clone/commit/push the registry repo itself.
fn update_index(
    name: &str,
    version: &str,
    tarball_url: &str,
    sha256: &str,
) -> Result<(), String> {
    let repo = format!("{REGISTRY_OWNER}/{REGISTRY_REPO}");
    let api_path = format!("repos/{repo}/contents/index/{name}.json");

    let existing = Command::new("gh")
        .args(["api", &api_path])
        .output()
        .map_err(|e| format!("failed to run 'gh api {api_path}': {e}"))?;

    let mut index = PackageIndex {
        name: name.to_string(),
        versions: Vec::new(),
    };
    let mut existing_sha = None;
    if existing.status.success() {
        let body: serde_json::Value = serde_json::from_slice(&existing.stdout)
            .map_err(|e| format!("failed to parse 'gh api {api_path}' output: {e}"))?;
        existing_sha = body["sha"].as_str().map(str::to_string);
        let content_b64 = body["content"]
            .as_str()
            .ok_or("'gh api' response had no 'content' field")?;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(content_b64.replace('\n', ""))
            .map_err(|e| format!("failed to decode existing index for '{name}': {e}"))?;
        index = serde_json::from_slice(&decoded)
            .map_err(|e| format!("failed to parse existing index for '{name}': {e}"))?;
    }

    if index.versions.iter().any(|v| v.version == version) {
        return Err(format!(
            "'{name}' already has a published version '{version}' -- publish never \
             overwrites an existing version"
        ));
    }
    index.versions.push(IndexedVersion {
        version: version.to_string(),
        tarball: tarball_url.to_string(),
        sha256: sha256.to_string(),
    });

    let new_content = serde_json::to_vec_pretty(&index)
        .map_err(|e| format!("failed to serialize index for '{name}': {e}"))?;
    let new_content_b64 = base64::engine::general_purpose::STANDARD.encode(new_content);

    let mut args = vec![
        "api".to_string(),
        "-X".to_string(),
        "PUT".to_string(),
        api_path.clone(),
        "-f".to_string(),
        format!("message=kittine-compiler publish: {name} {version}"),
        "-f".to_string(),
        format!("content={new_content_b64}"),
    ];
    if let Some(sha) = existing_sha {
        args.push("-f".to_string());
        args.push(format!("sha={sha}"));
    }

    let result = Command::new("gh")
        .args(&args)
        .output()
        .map_err(|e| format!("failed to run 'gh api PUT {api_path}': {e}"))?;
    if !result.status.success() {
        return Err(format!(
            "'gh api PUT {api_path}' exited with {}: {}",
            result.status,
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_is_the_last_published_version() {
        let idx = PackageIndex {
            name: "pkg".to_string(),
            versions: vec![
                IndexedVersion {
                    version: "0.1.0".to_string(),
                    tarball: "https://example/a".to_string(),
                    sha256: "aaaa".to_string(),
                },
                IndexedVersion {
                    version: "0.2.0".to_string(),
                    tarball: "https://example/b".to_string(),
                    sha256: "bbbb".to_string(),
                },
            ],
        };
        assert_eq!(idx.latest().unwrap().version, "0.2.0");
        assert_eq!(idx.find("0.1.0").unwrap().tarball, "https://example/a");
        assert!(idx.find("9.9.9").is_none());
    }

    #[test]
    fn sha256_hex_matches_a_known_vector() {
        // sha256("") -- a fixed, well-known test vector.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
