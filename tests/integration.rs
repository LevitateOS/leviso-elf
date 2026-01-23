//! Integration tests for leviso-elf using real system binaries.

use leviso_elf::get_library_dependencies;
use std::path::Path;

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
