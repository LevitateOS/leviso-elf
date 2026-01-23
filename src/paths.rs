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
