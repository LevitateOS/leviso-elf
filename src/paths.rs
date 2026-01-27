//! Library and binary path searching.

use std::path::{Path, PathBuf};

/// Find a library in standard paths within a rootfs.
///
/// Searches lib64, lib, and systemd private library paths.
/// The `extra_paths` parameter allows callers to add additional search paths
/// (e.g., `/usr/libexec/sudo` for rootfs builds).
///
/// Returns `None` if the library is not found in any search path.
#[must_use = "found library path should be used"]
pub fn find_library(source_root: &Path, lib_name: &str, extra_paths: &[&str]) -> Option<PathBuf> {
    // Standard library paths
    let mut candidates = vec![
        source_root.join("usr/lib64").join(lib_name),
        source_root.join("lib64").join(lib_name),
        source_root.join("usr/lib").join(lib_name),
        source_root.join("lib").join(lib_name),
        // Systemd private libraries
        source_root.join("usr/lib64/systemd").join(lib_name),
        source_root.join("usr/lib/systemd").join(lib_name),
    ];

    // Add extra paths from caller
    for extra in extra_paths {
        candidates.push(source_root.join(extra).join(lib_name));
    }

    candidates
        .into_iter()
        .find(|p| p.exists() || p.is_symlink())
}

/// Find a binary in standard bin/sbin directories.
///
/// Returns `None` if the binary is not found in any search path.
#[must_use = "found binary path should be used"]
pub fn find_binary(source_root: &Path, binary: &str) -> Option<PathBuf> {
    let bin_candidates = [
        source_root.join("usr/bin").join(binary),
        source_root.join("bin").join(binary),
        source_root.join("usr/sbin").join(binary),
        source_root.join("sbin").join(binary),
    ];

    bin_candidates.into_iter().find(|p| p.exists())
}

/// Find a binary, prioritizing sbin directories.
///
/// Returns `None` if the binary is not found in any search path.
#[must_use = "found binary path should be used"]
pub fn find_sbin_binary(source_root: &Path, binary: &str) -> Option<PathBuf> {
    let sbin_candidates = [
        source_root.join("usr/sbin").join(binary),
        source_root.join("sbin").join(binary),
        source_root.join("usr/bin").join(binary),
        source_root.join("bin").join(binary),
    ];

    sbin_candidates.into_iter().find(|p| p.exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    fn create_mock_rootfs(rootfs: &Path) {
        let dirs = ["usr/bin", "usr/sbin", "bin", "sbin", "usr/lib64", "lib64"];
        for dir in dirs {
            fs::create_dir_all(rootfs.join(dir)).unwrap();
        }
    }

    fn create_mock_binary(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, "#!/bin/bash\necho mock\n").unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    #[test]
    fn test_find_binary_usr_bin() {
        let temp = TempDir::new().unwrap();
        let rootfs = temp.path();
        create_mock_rootfs(rootfs);

        let binary_path = rootfs.join("usr/bin/testbin");
        create_mock_binary(&binary_path);

        let found = find_binary(rootfs, "testbin");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), binary_path);
    }

    #[test]
    fn test_find_binary_bin() {
        let temp = TempDir::new().unwrap();
        let rootfs = temp.path();
        create_mock_rootfs(rootfs);

        let binary_path = rootfs.join("bin/testbin2");
        create_mock_binary(&binary_path);

        let found = find_binary(rootfs, "testbin2");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), binary_path);
    }

    #[test]
    fn test_find_binary_not_found() {
        let temp = TempDir::new().unwrap();
        let rootfs = temp.path();
        create_mock_rootfs(rootfs);

        let found = find_binary(rootfs, "nonexistent");
        assert!(found.is_none());
    }

    #[test]
    fn test_find_binary_search_order() {
        let temp = TempDir::new().unwrap();
        let rootfs = temp.path();
        create_mock_rootfs(rootfs);

        // Create binary in both /usr/bin and /bin
        let usr_bin_path = rootfs.join("usr/bin/dupbin");
        let bin_path = rootfs.join("bin/dupbin");
        create_mock_binary(&usr_bin_path);
        create_mock_binary(&bin_path);

        // Should prefer /usr/bin
        let found = find_binary(rootfs, "dupbin");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), usr_bin_path);
    }
}
