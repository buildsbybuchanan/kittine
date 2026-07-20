//! High-level package-manager operations (`add`/`install`/`publish`),
//! layered on `manifest`, `lockfile`, and `registry`.

use crate::lockfile::{LockedPackage, Lockfile};
use crate::manifest::Manifest;
use crate::registry;
use std::path::Path;

pub const MODULES_DIR: &str = "kitten_modules";
pub const ENTRY_FILE: &str = "lib.kitty";

/// `kittine-compiler add <name> [version]`: records `name` (pinned to
/// `version`, or whatever the registry says is latest if omitted) as a
/// dependency in `dir`'s `kittine.toml`, creating a minimal manifest if
/// `dir` doesn't have one yet. Does not download anything -- that's
/// `install`'s job, the same division `cargo add`/`cargo build` draw.
pub fn add(dir: &Path, name: &str, version: Option<&str>) -> Result<String, String> {
    let mut manifest = Manifest::load(dir).unwrap_or_else(|_| Manifest::init_for_dir(dir));
    let resolved_version = match version {
        Some(v) => v.to_string(),
        None => {
            let index = registry::fetch_index(name)?;
            index
                .latest()
                .ok_or_else(|| format!("'{name}' has no published versions yet"))?
                .version
                .clone()
        }
    };
    manifest
        .dependencies
        .insert(name.to_string(), resolved_version.clone());
    manifest.save(dir)?;
    Ok(resolved_version)
}

/// `kittine-compiler install`: resolves every dependency in `dir`'s
/// `kittine.toml` against the registry, downloads and sha256-verifies
/// each tarball, extracts it to `kitten_modules/<name>/`, and writes the
/// exact resolved set to `kittine.lock`.
pub fn install(dir: &Path) -> Result<Vec<LockedPackage>, String> {
    let manifest = Manifest::load(dir)?;
    let modules_dir = dir.join(MODULES_DIR);
    let mut locked = Vec::new();

    for (name, version_req) in &manifest.dependencies {
        let index = registry::fetch_index(name)?;
        let resolved = if version_req.is_empty() {
            index.latest()
        } else {
            index.find(version_req)
        }
        .ok_or_else(|| format!("no published version of '{name}' matches '{version_req}'"))?;

        let bytes = registry::download_tarball(&resolved.tarball)?;
        let actual_sha = registry::sha256_hex(&bytes);
        if actual_sha != resolved.sha256 {
            return Err(format!(
                "checksum mismatch for '{name}' {}: expected {}, got {actual_sha} -- \
                 refusing to extract a tarball that doesn't match the registry index",
                resolved.version, resolved.sha256
            ));
        }

        let pkg_dir = modules_dir.join(name);
        extract_tarball(&bytes, &pkg_dir)?;

        locked.push(LockedPackage {
            name: name.clone(),
            version: resolved.version.clone(),
            checksum: format!("sha256:{actual_sha}"),
            source: format!(
                "registry+https://github.com/{}/{}",
                registry::REGISTRY_OWNER,
                registry::REGISTRY_REPO
            ),
        });
    }

    let lockfile = Lockfile {
        packages: locked.clone(),
    };
    lockfile.save(dir)?;
    Ok(locked)
}

fn extract_tarball(bytes: &[u8], dest: &Path) -> Result<(), String> {
    if dest.exists() {
        std::fs::remove_dir_all(dest)
            .map_err(|e| format!("failed to clear '{}': {e}", dest.display()))?;
    }
    std::fs::create_dir_all(dest)
        .map_err(|e| format!("failed to create '{}': {e}", dest.display()))?;
    let decompressed = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(decompressed);
    archive
        .unpack(dest)
        .map_err(|e| format!("failed to extract into '{}': {e}", dest.display()))
}

/// Excluded from a package tarball: build/VCS/dependency directories that
/// have no business being re-published or re-downloaded.
const EXCLUDED_DIRS: [&str; 4] = ["target", ".git", "node_modules", MODULES_DIR];

/// `kittine-compiler publish`: packs every file under `dir` (except the
/// excluded dirs above) into a `.tar.gz`, then hands it to
/// `registry::publish`. Requires a `kittine.toml` with `[package] name`/
/// `version` set, and a `lib.kitty` entry file at the package root -- the
/// fixed convention a dependent's bare-name import resolves against.
pub fn publish(dir: &Path) -> Result<String, String> {
    let manifest = Manifest::load(dir)?;
    let entry = dir.join(ENTRY_FILE);
    if !entry.exists() {
        return Err(format!(
            "'{}' has no {ENTRY_FILE} -- every published package needs one as its \
             import entry point",
            dir.display()
        ));
    }

    let tarball_path = std::env::temp_dir().join(format!(
        "{}-{}.tar.gz",
        manifest.package.name, manifest.package.version
    ));
    build_tarball(dir, &tarball_path)?;

    let result = registry::publish(
        &manifest.package.name,
        &manifest.package.version,
        &tarball_path,
    );
    let _ = std::fs::remove_file(&tarball_path);
    result
}

fn build_tarball(dir: &Path, out_path: &Path) -> Result<(), String> {
    let file = std::fs::File::create(out_path)
        .map_err(|e| format!("failed to create '{}': {e}", out_path.display()))?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut builder = tar::Builder::new(encoder);
    add_dir_to_tar(&mut builder, dir, dir)?;
    builder
        .into_inner()
        .and_then(|enc| enc.finish())
        .map_err(|e| format!("failed to finish '{}': {e}", out_path.display()))?;
    Ok(())
}

fn add_dir_to_tar<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    base: &Path,
    dir: &Path,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("failed to read '{}': {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read '{}': {e}", dir.display()))?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if path.is_dir() {
            if EXCLUDED_DIRS.contains(&name_str.as_ref()) || name_str.starts_with('.') {
                continue;
            }
            add_dir_to_tar(builder, base, &path)?;
        } else {
            let rel = path
                .strip_prefix(base)
                .map_err(|e| format!("failed to relativize '{}': {e}", path.display()))?;
            builder
                .append_path_with_name(&path, rel)
                .map_err(|e| format!("failed to add '{}' to tarball: {e}", path.display()))?;
        }
    }
    Ok(())
}
