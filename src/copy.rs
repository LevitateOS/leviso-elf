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

/// Copy a directory tree into `dst` and return the bytes of regular files
/// copied. Symlinks to directories are followed; other symlinks are recreated
/// unless something already exists at the destination (see
/// [`copy_dir_recursive_overwrite`]). Regular files are always overwritten. A
/// `src` that is not a directory copies nothing.
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<u64> {
    copy_dir_recursive_impl(src, dst, false)
}

/// Like [`copy_dir_recursive`], but a symlink replaces whatever file or link
/// already exists at its destination.
pub fn copy_dir_recursive_overwrite(src: &Path, dst: &Path) -> Result<u64> {
    copy_dir_recursive_impl(src, dst, true)
}

fn copy_dir_recursive_impl(src: &Path, dst: &Path, overwrite: bool) -> Result<u64> {
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
            total_size += copy_dir_recursive_impl(&path, &dest_path, overwrite)?;
        } else if path.is_symlink() {
            let target = fs::read_link(&path)?;
            if overwrite && (dest_path.exists() || dest_path.is_symlink()) {
                fs::remove_file(&dest_path)?;
            }
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

/// Copy library `lib_name`, found under `source_root` by [`find_library`]
/// (which also searches `extra_lib_paths`), to `dest_lib64_path` or
/// `dest_lib_path` under `dest_root`, by whether it was found in a lib64
/// directory. A library under one of `private_lib_dirs` (e.g. `systemd`) keeps
/// that subdirectory, under `dest_lib64_path`. A symlinked library is copied
/// as its target file plus the link. Does nothing when the destination
/// already exists.
pub fn copy_library_to(
    source_root: &Path,
    lib_name: &str,
    dest_root: &Path,
    dest_lib64_path: &str,
    dest_lib_path: &str,
    extra_lib_paths: &[&str],
    private_lib_dirs: &[&str],
) -> Result<()> {
    let src = find_library(source_root, lib_name, extra_lib_paths).with_context(|| {
        format!(
            "Could not find library '{}' in source (searched lib64, lib, extra paths)",
            lib_name
        )
    })?;

    let src_str = src.to_string_lossy();
    let private_dir = private_lib_dirs.iter().find(|dir| {
        src_str.contains(&format!("lib64/{}", dir)) || src_str.contains(&format!("lib/{}", dir))
    });

    let dest_path = if let Some(dir) = private_dir {
        let dest_dir = dest_root.join(dest_lib64_path).join(dir);
        fs::create_dir_all(&dest_dir)?;
        dest_dir.join(lib_name)
    } else if src_str.contains("lib64") {
        dest_root.join(dest_lib64_path).join(lib_name)
    } else {
        dest_root.join(dest_lib_path).join(lib_name)
    };

    if dest_path.exists() {
        return Ok(());
    }

    if src.is_symlink() {
        let link_target = fs::read_link(&src)?;

        // An absolute link target is inside source_root, not on the host.
        let actual_src = if link_target.is_relative() {
            src.parent()
                .context("Library path has no parent")?
                .join(&link_target)
        } else {
            source_root.join(link_target.to_str().unwrap().trim_start_matches('/'))
        };

        if actual_src.exists() {
            let target_name = link_target.file_name().unwrap_or(link_target.as_os_str());
            let target_dest = dest_path.parent().unwrap().join(target_name);
            if !target_dest.exists() {
                fs::copy(&actual_src, &target_dest)?;
            }
            if !dest_path.exists() {
                std::os::unix::fs::symlink(&link_target, &dest_path)?;
            }
        } else {
            // Target not under source_root: fs::copy follows the link as the
            // host resolves it, and fails if it dangles.
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
#[must_use = "return value indicates whether symlink was created"]
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
