//! ELF binary analysis using readelf.

use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

use crate::paths::find_library;

/// Extract library dependencies from an ELF binary using readelf.
///
/// This is architecture-independent - readelf reads the ELF headers directly
/// without executing the binary, unlike ldd which uses the host dynamic linker.
pub fn get_library_dependencies(binary_path: &Path) -> Result<Vec<String>> {
    let output = Command::new("readelf")
        .args(["-d"])
        .arg(binary_path)
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return Ok(Vec::new()), // readelf not found or failed
    };

    if !output.status.success() {
        // Not an ELF binary or readelf failed - return empty list
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_readelf_output(&stdout)
}

/// Parse readelf -d output to extract NEEDED library names.
///
/// Example readelf output:
/// ```text
/// Dynamic section at offset 0x2d0e0 contains 28 entries:
///   Tag        Type                         Name/Value
///  0x0000000000000001 (NEEDED)             Shared library: [libtinfo.so.6]
///  0x0000000000000001 (NEEDED)             Shared library: [libc.so.6]
/// ```
pub fn parse_readelf_output(output: &str) -> Result<Vec<String>> {
    let mut libs = Vec::new();

    for line in output.lines() {
        // Look for lines containing "(NEEDED)" and "Shared library:"
        if line.contains("(NEEDED)") && line.contains("Shared library:") {
            // Extract library name from [libname.so.X]
            if let Some(start) = line.find('[') {
                if let Some(end) = line.find(']') {
                    let lib_name = &line[start + 1..end];
                    libs.push(lib_name.to_string());
                }
            }
        }
    }

    Ok(libs)
}

/// Recursively get all library dependencies (including transitive).
///
/// Some libraries depend on other libraries. We need to copy all of them.
/// The `extra_lib_paths` parameter is passed to `find_library` for each lookup.
pub fn get_all_dependencies(
    source_root: &Path,
    binary_path: &Path,
    extra_lib_paths: &[&str],
) -> Result<HashSet<String>> {
    let mut all_libs = HashSet::new();
    let mut to_process = vec![binary_path.to_path_buf()];
    let mut processed = HashSet::new();

    while let Some(path) = to_process.pop() {
        if processed.contains(&path) {
            continue;
        }
        processed.insert(path.clone());

        let deps = get_library_dependencies(&path)?;
        for lib_name in deps {
            if all_libs.insert(lib_name.clone()) {
                // New library - find it and check its dependencies too
                if let Some(lib_path) = find_library(source_root, &lib_name, extra_lib_paths) {
                    to_process.push(lib_path);
                }
            }
        }
    }

    Ok(all_libs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_readelf_output() {
        let output = r#"
Dynamic section at offset 0x2d0e0 contains 28 entries:
  Tag        Type                         Name/Value
 0x0000000000000001 (NEEDED)             Shared library: [libtinfo.so.6]
 0x0000000000000001 (NEEDED)             Shared library: [libc.so.6]
 0x000000000000000c (INIT)               0x5000
"#;
        let libs = parse_readelf_output(output).unwrap();
        assert_eq!(libs, vec!["libtinfo.so.6", "libc.so.6"]);
    }

    #[test]
    fn test_parse_readelf_empty() {
        let output = "not an ELF file";
        let libs = parse_readelf_output(output).unwrap();
        assert!(libs.is_empty());
    }
}
