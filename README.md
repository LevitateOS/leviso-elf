# leviso-elf

ELF binary analysis and library dependency copying utilities. Uses `readelf` for cross-compilation safe dependency detection (unlike `ldd` which executes binaries).

## Status

| Metric | Value |
|--------|-------|
| Stage | Beta |
| Target | x86_64 Linux |
| Last verified | 2026-01-23 |

### Works

- Dependency analysis via readelf -d
- Recursive transitive dependency resolution
- Library copying with path preservation
- Binary search in standard Linux paths

### Known Issues

- See parent repo issues

---

## Author

<!-- HUMAN WRITTEN - DO NOT MODIFY -->

[Waiting for human input]

<!-- END HUMAN WRITTEN -->

---

## Features

- **Dependency Analysis**: Parse `readelf -d` output to find shared library dependencies
- **Recursive Resolution**: Trace full dependency tree including transitive dependencies
- **Library Copying**: Copy binaries with all required libraries to a target directory
- **Path Search**: Find binaries and libraries in standard Linux paths

## Usage

```rust
use leviso_elf::{get_all_dependencies, copy_library_to, find_binary, find_library};
use std::path::Path;

let source_root = Path::new("/path/to/source/rootfs");
let dest_root = Path::new("/path/to/dest/rootfs");

// Find all library dependencies for a binary (recursive)
let binary_path = source_root.join("usr/bin/bash");
let deps = get_all_dependencies(source_root, &binary_path, &["usr/libexec/sudo"])?;

// Copy a library to target directory with configurable paths
copy_library_to(
    source_root,
    "libc.so.6",
    dest_root,
    "usr/lib64",           // dest lib64 path
    "usr/lib",             // dest lib path
    &["usr/libexec/sudo"], // extra search paths
    &["systemd"],          // private lib dirs (use &[] for musl/OpenRC)
)?;

// Find a binary in standard paths
if let Some(path) = find_binary(source_root, "bash") {
    println!("Found bash at: {}", path.display());
}

// Find a library with extra search paths
if let Some(path) = find_library(source_root, "libpam.so.0", &[]) {
    println!("Found libpam at: {}", path.display());
}
```

## Why readelf instead of ldd?

`ldd` executes the binary to resolve dependencies, which fails for cross-compiled binaries and can be a security risk. `readelf -d` parses the ELF headers directly without execution.

## License

MIT
