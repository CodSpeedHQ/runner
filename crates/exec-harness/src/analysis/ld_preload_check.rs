use crate::prelude::*;
use std::fs;
use std::path::Path;

/// Checks if the given executable will honor LD_PRELOAD.
///
/// Returns `Ok(())` if LD_PRELOAD will work, or an error with a descriptive message if not.
///
/// LD_PRELOAD works for:
/// - Dynamically linked ELF binaries
/// - Scripts (the interpreter is typically dynamically linked)
///
/// LD_PRELOAD does NOT work for:
/// - Statically linked ELF binaries (no dynamic linker involved)
pub fn check_ld_preload_compatible(executable: &str) -> Result<()> {
    let path = resolve_executable(executable)?;
    let data = fs::read(&path)
        .with_context(|| format!("Failed to read executable: {}", path.display()))?;

    // Check ELF magic bytes
    if data.len() >= 4 && &data[0..4] == b"\x7FELF" {
        check_elf_is_dynamic(&data, &path)
    } else {
        // Not an ELF file - likely a script with a shebang.
        // Scripts use an interpreter which is typically dynamically linked.
        Ok(())
    }
}

/// Resolve executable name to its full path using PATH lookup.
fn resolve_executable(executable: &str) -> Result<std::path::PathBuf> {
    let path = Path::new(executable);

    // If it's already an absolute or relative path, use it directly
    if path.is_absolute() || executable.contains('/') {
        return Ok(path.to_path_buf());
    }

    // Search in PATH
    if let Ok(path_env) = std::env::var("PATH") {
        for dir in path_env.split(':') {
            let candidate = Path::new(dir).join(executable);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    bail!("Executable not found in PATH: {executable}")
}

/// Check if an ELF binary is dynamically linked.
fn check_elf_is_dynamic(data: &[u8], path: &Path) -> Result<()> {
    use object::Endianness;
    use object::read::elf::ElfFile;

    // Try parsing as 64-bit ELF first, then 32-bit
    if let Ok(elf) = ElfFile::<object::elf::FileHeader64<Endianness>>::parse(data) {
        return check_elf_has_interp(elf, path);
    }

    if let Ok(elf) = ElfFile::<object::elf::FileHeader32<Endianness>>::parse(data) {
        return check_elf_has_interp(elf, path);
    }

    bail!("Failed to parse ELF file: {}", path.display())
}

/// Check if an ELF file has a PT_INTERP or PT_DYNAMIC segment, indicating dynamic linking.
fn check_elf_has_interp<'data, Elf>(
    elf: object::read::elf::ElfFile<'data, Elf>,
    path: &Path,
) -> Result<()>
where
    Elf: object::read::elf::FileHeader,
{
    use object::read::elf::ProgramHeader;

    let endian = elf.endian();

    for segment in elf.elf_program_headers() {
        let p_type = segment.p_type(endian);
        // Either PT_INTERP or PT_DYNAMIC indicates a dynamically linked binary
        if p_type == object::elf::PT_INTERP || p_type == object::elf::PT_DYNAMIC {
            return Ok(());
        }
    }

    // No PT_INTERP found - this is a statically linked binary
    bail!(
        "The codspeed CLI in CPU Simulation mode does not support statically linked binaries.\n\n\
         Executable '{}' is statically linked.\n\n\
         Please either:\n\
         - Use a dynamically linked executable, or\n\
         - Use a different measurement mode, or\n\
         - Use one of the CodSpeed framework benchmark integrations",
        path.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_binary() {
        // /bin/sh or similar should be dynamically linked on most systems
        let result = check_ld_preload_compatible("sh");
        assert!(
            result.is_ok(),
            "sh should be dynamically linked: {result:?}"
        );
    }

    #[test]
    fn test_nonexistent_binary() {
        let result = check_ld_preload_compatible("nonexistent_binary_12345");
        assert!(result.is_err());
    }
}
