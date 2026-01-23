# leviso-elf

ELF binary analysis and library dependency copying utilities. Uses `readelf` for cross-compilation safe dependency detection (unlike `ldd` which executes binaries).

## Features

- **Dependency Analysis**: Parse `readelf -d` output to find shared library dependencies
- **Recursive Resolution**: Trace full dependency tree including transitive dependencies
- **Library Copying**: Copy binaries with all required libraries to a target directory
- **Path Search**: Find binaries and libraries in standard Linux paths

## Usage

```rust
use leviso_elf::{get_all_dependencies, copy_library_to, find_binary};

// Find all library dependencies for a binary
let deps = get_all_dependencies("/usr/bin/bash", &["/lib64", "/usr/lib64"])?;

// Copy a library to target directory
copy_library_to("/lib64/libc.so.6", &target_dir)?;

// Find a binary in standard paths
let path = find_binary("bash", &source_rootfs)?;
```

## Why readelf instead of ldd?

`ldd` executes the binary to resolve dependencies, which fails for cross-compiled binaries and can be a security risk. `readelf -d` parses the ELF headers directly without execution.

## License

MIT
