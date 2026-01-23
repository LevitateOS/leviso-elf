//! File and library copying utilities.

use anyhow::{Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::paths::find_library;

/// Make a file executable (chmod 755).
pub fn make_executable(path: &Path) -> Result<()> {
    let mut perms = fs::metadata(path)
        .with_context(|| format!("Failed to read metadata: {}", path.display()))?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)
        .with_context(|| format!("Failed to set permissions: {}", path.display()))?;
    Ok(())
}

/// Copy a directory recursively, handling symlinks.
///
/// Returns the total size in bytes of all files copied.
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<u64> {
    let mut total_size: u64 = 0;

    if !src.is_dir() {
        return Ok(0);
    }

    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            total_size += copy_dir_recursive(&path, &dest_path)?;
        } else if path.is_symlink() {
            let target = fs::read_link(&path)?;
            if !dest_path.exists() && !dest_path.is_symlink() {
                std::os::unix::fs::symlink(&target, &dest_path)?;
            }
        } else {
            fs::copy(&path, &dest_path)?;
            if let Ok(meta) = fs::metadata(&dest_path) {
                total_size += meta.len();
            }
        }
    }

    Ok(total_size)
}

/// Copy a library from source to destination, handling symlinks.
///
/// The `dest_lib64_path` and `dest_lib_path` parameters specify where
/// libraries should be copied (e.g., "lib64" for initramfs, "usr/lib64" for rootfs).
pub fn copy_library_to(
    source_root: &Path,
    lib_name: &str,
    dest_root: &Path,
    dest_lib64_path: &str,
    dest_lib_path: &str,
    extra_lib_paths: &[&str],
) -> Result<()> {
    let src = find_library(source_root, lib_name, extra_lib_paths).with_context(|| {
        format!(
            "Could not find library '{}' in source (searched lib64, lib, systemd paths)",
            lib_name
        )
    })?;

    // Check if this is a systemd private library
    let dest_path = if src.to_string_lossy().contains("lib64/systemd")
        || src.to_string_lossy().contains("lib/systemd")
    {
        // Systemd private libraries stay in their own directory
        let dest_dir = dest_root.join(dest_lib64_path).join("systemd");
        fs::create_dir_all(&dest_dir)?;
        dest_dir.join(lib_name)
    } else if src.to_string_lossy().contains("lib64") {
        dest_root.join(dest_lib64_path).join(lib_name)
    } else {
        dest_root.join(dest_lib_path).join(lib_name)
    };

    if dest_path.exists() {
        return Ok(()); // Already copied
    }

    // Handle symlinks - copy both the symlink target and create the symlink
    if src.is_symlink() {
        let link_target = fs::read_link(&src)?;

        // Resolve the actual file
        let actual_src = if link_target.is_relative() {
            src.parent()
                .context("Library path has no parent")?
                .join(&link_target)
        } else {
            source_root.join(link_target.to_str().unwrap().trim_start_matches('/'))
        };

        if actual_src.exists() {
            // Copy the actual file first
            let target_name = link_target.file_name().unwrap_or(link_target.as_os_str());
            let target_dest = dest_path.parent().unwrap().join(target_name);
            if !target_dest.exists() {
                fs::copy(&actual_src, &target_dest)?;
            }
            // Create symlink
            if !dest_path.exists() {
                std::os::unix::fs::symlink(&link_target, &dest_path)?;
            }
        } else {
            // Symlink target not found, copy the symlink itself
            fs::copy(&src, &dest_path)?;
        }
    } else {
        fs::copy(&src, &dest_path)?;
    }

    Ok(())
}

/// Create a symlink if it doesn't already exist.
///
/// Returns `Ok(true)` if the symlink was created, `Ok(false)` if it already existed.
/// This is useful for idempotent symlink creation (e.g., enabling systemd services).
pub fn create_symlink_if_missing(target: &Path, link: &Path) -> Result<bool> {
    if link.exists() || link.is_symlink() {
        return Ok(false);
    }
    std::os::unix::fs::symlink(target, link).with_context(|| {
        format!(
            "Failed to create symlink {} -> {}",
            link.display(),
            target.display()
        )
    })?;
    Ok(true)
}
