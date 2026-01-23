//! ELF binary analysis and copying utilities.
//!
//! Uses `readelf -d` instead of `ldd` to extract library dependencies.
//! This works for cross-compilation since readelf reads ELF headers directly
//! without executing the binary (which ldd does via the host dynamic linker).

mod analyze;
mod copy;
mod paths;

pub use analyze::{get_all_dependencies, get_library_dependencies, parse_readelf_output};
pub use copy::{copy_dir_recursive, copy_library_to, create_symlink_if_missing, make_executable};
pub use paths::{find_binary, find_library, find_sbin_binary};
