//! Bundle installation

use std::path::{Component, Path, PathBuf};

use crate::bundler::blob::BlobStore;
use crate::bundler::package::{BundlePackage};
use crate::error::{MsError, Result};

#[derive(Debug, Clone, serde::Serialize)]
pub struct InstallReport {
    pub bundle_id: String,
    pub installed: Vec<String>,
    pub skipped: Vec<String>,
    pub blobs_written: usize,
}

/// Install a bundle into the git archive root.
pub fn install(
    package: &BundlePackage,
    archive_root: &Path,
    only_skills: &[String],
) -> Result<InstallReport> {
    package.verify()?;

    let store = BlobStore::open(archive_root.join("bundles"))?;
    let blobs_written = package.write_missing_blobs(&store)?;

    let mut installed = Vec::new();
    let mut skipped = Vec::new();

    for skill in &package.manifest.skills {
        if !only_skills.is_empty() && !only_skills.contains(&skill.name) {
            skipped.push(skill.name.clone());
            continue;
        }
        let hash = skill.hash.as_ref().ok_or_else(|| {
            MsError::ValidationFailed(format!("missing blob hash for {}", skill.name))
        })?;
        let blob = package
            .blobs
            .iter()
            .find(|b| &b.hash == hash)
            .ok_or_else(|| MsError::ValidationFailed(format!(
                "bundle missing blob {} for {}",
                hash, skill.name
            )))?;

        let target = resolve_target_path(archive_root, &skill.path, &skill.name)?;
        if target.exists() {
            return Err(MsError::ValidationFailed(format!(
                "skill already exists at {}",
                target.display()
            )));
        }

        std::fs::create_dir_all(&target).map_err(|err| {
            MsError::Config(format!("create {}: {err}", target.display()))
        })?;
        unpack_blob(&target, &blob.bytes)?;
        installed.push(skill.name.clone());
    }

    Ok(InstallReport {
        bundle_id: package.manifest.bundle.id.clone(),
        installed,
        skipped,
        blobs_written,
    })
}

fn resolve_target_path(root: &Path, path: &Path, fallback_id: &str) -> Result<PathBuf> {
    if !path.as_os_str().is_empty() {
        ensure_relative(path)?;
        return Ok(root.join(path));
    }
    Ok(root.join("skills").join("by-id").join(fallback_id))
}

fn ensure_relative(path: &Path) -> Result<()> {
    if path.is_absolute() {
        return Err(MsError::ValidationFailed(format!(
            "bundle path must be relative: {}",
            path.display()
        )));
    }
    for comp in path.components() {
        match comp {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(MsError::ValidationFailed(format!(
                    "bundle path contains invalid component: {}",
                    path.display()
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

fn unpack_blob(target: &Path, bytes: &[u8]) -> Result<()> {
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let name_len = read_u64(bytes, &mut cursor)? as usize;
        if name_len == 0 {
            return Err(MsError::ValidationFailed(
                "bundle entry has empty path".to_string(),
            ));
        }
        let name_bytes = read_slice(bytes, &mut cursor, name_len)?;
        let name = std::str::from_utf8(name_bytes).map_err(|_| {
            MsError::ValidationFailed("bundle entry path is invalid UTF-8".to_string())
        })?;
        let file_len = read_u64(bytes, &mut cursor)? as usize;
        let file_bytes = read_slice(bytes, &mut cursor, file_len)?;

        let rel = Path::new(name);
        ensure_relative(rel)?;
        let path = target.join(rel);
        if path.exists() {
            return Err(MsError::ValidationFailed(format!(
                "bundle file already exists: {}",
                path.display()
            )));
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                MsError::Config(format!("create {}: {err}", parent.display()))
            })?;
        }
        std::fs::write(&path, file_bytes).map_err(|err| {
            MsError::Config(format!("write {}: {err}", path.display()))
        })?;
    }
    Ok(())
}

fn read_u64(input: &[u8], cursor: &mut usize) -> Result<u64> {
    let end = cursor.checked_add(8).ok_or_else(|| {
        MsError::ValidationFailed("bundle parse overflow".to_string())
    })?;
    if end > input.len() {
        return Err(MsError::ValidationFailed(
            "bundle parse truncated".to_string(),
        ));
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&input[*cursor..end]);
    *cursor = end;
    Ok(u64::from_be_bytes(buf))
}

fn read_slice<'a>(input: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8]> {
    let end = cursor.checked_add(len).ok_or_else(|| {
        MsError::ValidationFailed("bundle parse overflow".to_string())
    })?;
    if end > input.len() {
        return Err(MsError::ValidationFailed(
            "bundle parse truncated".to_string(),
        ));
    }
    let slice = &input[*cursor..end];
    *cursor = end;
    Ok(slice)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundler::manifest::{BundleInfo, BundleManifest, BundledSkill};
    use crate::bundler::package::Bundle;
    use tempfile::tempdir;

    #[test]
    fn install_unpacks_skill_files() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("skills/by-id/demo");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "content").unwrap();

        let manifest = BundleManifest {
            bundle: BundleInfo {
                id: "bundle".to_string(),
                name: "Bundle".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                authors: vec![],
                license: None,
                repository: None,
                keywords: vec![],
                ms_version: None,
            },
            skills: vec![BundledSkill {
                name: "demo".to_string(),
                path: PathBuf::from("skills/by-id/demo"),
                version: Some("1.0.0".to_string()),
                hash: None,
                optional: false,
            }],
            dependencies: vec![],
            checksum: None,
            signatures: vec![],
        };

        let bundle = Bundle::new(manifest, dir.path());
        let package = bundle.package().unwrap();

        let install_root = tempdir().unwrap();
        let report = install(&package, install_root.path(), &[]).unwrap();
        assert_eq!(report.installed, vec!["demo".to_string()]);
        let installed_path = install_root
            .path()
            .join("skills/by-id/demo/SKILL.md");
        assert!(installed_path.exists());
    }
}
