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
/// Skips existing symlinks (additive copy). Use [`copy_dir_recursive_overwrite`]
/// if you need to replace existing files.
///
/// Returns the total size in bytes of all files copied.
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<u64> {
    copy_dir_recursive_impl(src, dst, false)
}

/// Copy a directory recursively, overwriting existing files and symlinks.
///
/// Unlike [`copy_dir_recursive`], this will remove and replace existing symlinks.
/// Useful for overlay creation where you want to replace the destination contents.
///
/// Returns the total size in bytes of all files copied.
pub fn copy_dir_recursive_overwrite(src: &Path, dst: &Path) -> Result<u64> {
    copy_dir_recursive_impl(src, dst, true)
}

/// Internal implementation for recursive directory copy.
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

/// Copy a library from source to destination, handling symlinks.
///
/// The `dest_lib64_path` and `dest_lib_path` parameters specify where
/// libraries should be copied (e.g., "lib64" for initramfs, "usr/lib64" for rootfs).
///
/// The `private_lib_dirs` parameter specifies subdirectories that should preserve
/// their structure (e.g., `&["systemd"]` for LevitateOS, `&["openrc"]` for AcornOS,
/// or `&[]` if no private library directories are needed).
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

    // Check if this is a private library (e.g., systemd, openrc)
    let src_str = src.to_string_lossy();
    let private_dir = private_lib_dirs.iter().find(|dir| {
        src_str.contains(&format!("lib64/{}", dir)) || src_str.contains(&format!("lib/{}", dir))
    });

    let dest_path = if let Some(dir) = private_dir {
        // Private libraries stay in their own subdirectory
        let dest_dir = dest_root.join(dest_lib64_path).join(dir);
        fs::create_dir_all(&dest_dir)?;
        dest_dir.join(lib_name)
    } else if src_str.contains("lib64") {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_copy_dir_recursive() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();

        // Create source directory structure
        let src = base.join("src_dir");
        fs::create_dir_all(src.join("subdir")).unwrap();
        fs::write(src.join("file1.txt"), "content1").unwrap();
        fs::write(src.join("subdir/file2.txt"), "content2").unwrap();

        // Copy to destination
        let dst = base.join("dst_dir");
        copy_dir_recursive(&src, &dst).expect("copy_dir_recursive should succeed");

        // Verify structure
        assert!(dst.join("file1.txt").exists());
        assert!(dst.join("subdir/file2.txt").exists());
        assert_eq!(fs::read_to_string(dst.join("file1.txt")).unwrap(), "content1");
        assert_eq!(fs::read_to_string(dst.join("subdir/file2.txt")).unwrap(), "content2");
    }

    #[test]
    fn test_make_executable() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test_exec");
        fs::write(&file_path, "test").unwrap();

        make_executable(&file_path).expect("make_executable should succeed");

        let metadata = fs::metadata(&file_path).unwrap();
        let mode = metadata.permissions().mode();

        // Check executable bits (755 = rwxr-xr-x)
        assert_eq!(mode & 0o111, 0o111, "File should be executable");
    }

    #[test]
    fn test_create_symlink_if_missing() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();

        let target = base.join("target");
        let link = base.join("link");

        // First call should create symlink
        let created = create_symlink_if_missing(&target, &link).unwrap();
        assert!(created, "First call should create symlink");
        assert!(link.is_symlink());

        // Second call should not recreate
        let created = create_symlink_if_missing(&target, &link).unwrap();
        assert!(!created, "Second call should not recreate symlink");
    }

    #[test]
    fn test_copy_dir_recursive_skips_existing_symlinks() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();

        // Create source with a symlink
        let src = base.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("file.txt"), "content").unwrap();
        std::os::unix::fs::symlink("file.txt", src.join("link")).unwrap();

        // Create destination with an existing symlink pointing elsewhere
        let dst = base.join("dst");
        fs::create_dir_all(&dst).unwrap();
        std::os::unix::fs::symlink("other.txt", dst.join("link")).unwrap();

        // copy_dir_recursive should skip the existing symlink
        copy_dir_recursive(&src, &dst).unwrap();

        // The symlink should still point to "other.txt" (not overwritten)
        assert_eq!(fs::read_link(dst.join("link")).unwrap().to_str().unwrap(), "other.txt");
    }

    #[test]
    fn test_copy_dir_recursive_overwrite_replaces_symlinks() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();

        // Create source with a symlink
        let src = base.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("file.txt"), "content").unwrap();
        std::os::unix::fs::symlink("file.txt", src.join("link")).unwrap();

        // Create destination with an existing symlink pointing elsewhere
        let dst = base.join("dst");
        fs::create_dir_all(&dst).unwrap();
        std::os::unix::fs::symlink("other.txt", dst.join("link")).unwrap();

        // copy_dir_recursive_overwrite should replace the symlink
        copy_dir_recursive_overwrite(&src, &dst).unwrap();

        // The symlink should now point to "file.txt" (was overwritten)
        assert_eq!(fs::read_link(dst.join("link")).unwrap().to_str().unwrap(), "file.txt");
    }
}
