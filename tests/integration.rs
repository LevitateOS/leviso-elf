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
