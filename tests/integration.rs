//! Integration tests for leviso-elf using real system binaries.

use leviso_cheat_test::cheat_aware;
use leviso_elf::get_library_dependencies;
use std::path::Path;

#[cheat_aware(
    protects = "ELF library detection finds actual runtime dependencies",
    severity = "HIGH",
    ease = "MEDIUM",
    cheats = ["Return empty list for all binaries", "Hardcode common libs instead of parsing"],
    consequence = "Missing libraries in initramfs/rootfs, binaries crash with 'not found' at runtime",
    legitimate_change = "Library detection must use readelf or equivalent to find real dependencies. \
        If detection method changes, verify it still finds libc for /bin/sh."
)]
#[test]
fn test_get_deps_of_real_binary() {
    // /bin/sh exists on all Linux systems and is dynamically linked
    let deps = get_library_dependencies(Path::new("/bin/sh")).unwrap();
    // Should have at least libc dependency
    assert!(
        deps.iter().any(|d| d.contains("libc")),
        "Expected libc dependency in /bin/sh, got: {:?}",
        deps
    );
}

#[cheat_aware(
    protects = "Nonexistent binary produces clear error, not empty deps",
    severity = "HIGH",
    ease = "EASY",
    cheats = ["Return empty Vec for missing files", "Silently ignore file errors"],
    consequence = "Build silently skips missing binaries, user gets incomplete rootfs",
    legitimate_change = "Missing files must fail loudly. Empty deps is reserved for non-ELF files."
)]
#[test]
fn test_nonexistent_binary() {
    let result = get_library_dependencies(Path::new("/nonexistent/path/to/binary"));
    assert!(result.is_err(), "Expected error for nonexistent file");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("does not exist"),
        "Expected 'does not exist' in error message, got: {}",
        err_msg
    );
}

#[test]
fn test_non_elf_file() {
    // /etc/passwd is a text file, not an ELF binary
    let deps = get_library_dependencies(Path::new("/etc/passwd")).unwrap();
    assert!(
        deps.is_empty(),
        "Expected empty deps for non-ELF file, got: {:?}",
        deps
    );
}

#[test]
fn test_directory_not_file() {
    // Directories are not ELF files
    let result = get_library_dependencies(Path::new("/tmp"));
    // readelf will error on directories, but we should handle it gracefully
    // Either an error or empty result is acceptable
    match result {
        Ok(deps) => assert!(deps.is_empty()),
        Err(_) => {} // Also acceptable - readelf may error on directories
    }
}
