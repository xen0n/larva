//! ELF loader for RISC-V binaries.
//!
//! This is a minimal ELF64 loader focused on static RISC-V binaries.

use super::mem::{GuestAddr, GuestMmu, MemPerms};
use std::path::Path;

/// ELF64 file header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Elf64Ehdr {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

/// ELF64 program header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Elf64Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

// ELF constants
const ELFMAG: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1; // Little-endian
const EM_RISCV: u16 = 243;

const PT_LOAD: u32 = 1;
const PT_INTERP: u32 = 3;
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;

/// Errors that can occur during ELF loading.
#[derive(Debug)]
pub enum ElfLoadError {
    Io(std::io::Error),
    Parse(String),
    Unsupported(String),
    Memory(std::io::Error),
}

impl std::fmt::Display for ElfLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElfLoadError::Io(e) => write!(f, "IO error: {e}"),
            ElfLoadError::Parse(s) => write!(f, "Parse error: {s}"),
            ElfLoadError::Unsupported(s) => write!(f, "Unsupported: {s}"),
            ElfLoadError::Memory(e) => write!(f, "Memory error: {e}"),
        }
    }
}

impl std::error::Error for ElfLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ElfLoadError::Io(e) => Some(e),
            ElfLoadError::Memory(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ElfLoadError {
    fn from(e: std::io::Error) -> Self {
        ElfLoadError::Io(e)
    }
}

/// Information about a loaded ELF binary.
#[derive(Debug, Clone)]
pub struct LoadedElf {
    /// Entry point address
    pub entry: GuestAddr,
    /// Base address of the loaded image (lowest PT_LOAD address)
    pub base_addr: GuestAddr,
    /// End address of the loaded image (highest PT_LOAD end)
    pub end_addr: GuestAddr,
    /// ELF interpreter (dynamic linker) if any
    pub interp: Option<String>,
}

fn read_u16_le(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_u64_le(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

fn parse_ehdr(data: &[u8]) -> Result<Elf64Ehdr, ElfLoadError> {
    if data.len() < 64 {
        return Err(ElfLoadError::Parse("file too small for ELF header".into()));
    }

    // Check magic
    if data[0..4] != ELFMAG {
        return Err(ElfLoadError::Parse("invalid ELF magic".into()));
    }

    // Check class (64-bit)
    if data[4] != ELFCLASS64 {
        return Err(ElfLoadError::Parse("not a 64-bit ELF".into()));
    }

    // Check endianness (little)
    if data[5] != ELFDATA2LSB {
        return Err(ElfLoadError::Unsupported(
            "big-endian ELF not supported".into(),
        ));
    }

    Ok(Elf64Ehdr {
        e_ident: data[0..16].try_into().unwrap(),
        e_type: read_u16_le(data, 16),
        e_machine: read_u16_le(data, 18),
        e_version: read_u32_le(data, 20),
        e_entry: read_u64_le(data, 24),
        e_phoff: read_u64_le(data, 32),
        e_shoff: read_u64_le(data, 40),
        e_flags: read_u32_le(data, 48),
        e_ehsize: read_u16_le(data, 52),
        e_phentsize: read_u16_le(data, 54),
        e_phnum: read_u16_le(data, 56),
        e_shentsize: read_u16_le(data, 58),
        e_shnum: read_u16_le(data, 60),
        e_shstrndx: read_u16_le(data, 62),
    })
}

fn parse_phdr(data: &[u8], offset: usize) -> Result<Elf64Phdr, ElfLoadError> {
    if data.len() < offset + 56 {
        return Err(ElfLoadError::Parse("file too small for program header".into()));
    }

    Ok(Elf64Phdr {
        p_type: read_u32_le(data, offset),
        p_flags: read_u32_le(data, offset + 4),
        p_offset: read_u64_le(data, offset + 8),
        p_vaddr: read_u64_le(data, offset + 16),
        p_paddr: read_u64_le(data, offset + 24),
        p_filesz: read_u64_le(data, offset + 32),
        p_memsz: read_u64_le(data, offset + 40),
        p_align: read_u64_le(data, offset + 48),
    })
}

/// Load a static ELF binary into guest memory.
pub fn load_elf_static<P: AsRef<Path>>(
    mmu: &mut GuestMmu,
    path: P,
) -> Result<LoadedElf, ElfLoadError> {
    let data = std::fs::read(path)?;
    load_elf_static_from_bytes(mmu, &data)
}

/// Load a static ELF binary from bytes into guest memory.
pub fn load_elf_static_from_bytes(
    mmu: &mut GuestMmu,
    data: &[u8],
) -> Result<LoadedElf, ElfLoadError> {
    let ehdr = parse_ehdr(data)?;

    // Validate it's a RISC-V binary
    if ehdr.e_machine != EM_RISCV {
        return Err(ElfLoadError::Unsupported(format!(
            "unsupported architecture: expected RISC-V ({EM_RISCV}), got {}",
            ehdr.e_machine
        )));
    }

    // Parse and load program headers
    let mut load_segments = Vec::new();
    let mut interp = None;

    for i in 0..ehdr.e_phnum as usize {
        let phdr_offset = ehdr.e_phoff as usize + i * ehdr.e_phentsize as usize;
        let ph = parse_phdr(data, phdr_offset)?;

        match ph.p_type {
            PT_LOAD => load_segments.push(ph),
            PT_INTERP => {
                // Read interpreter path
                let offset = ph.p_offset as usize;
                let mut end = offset;
                while end < data.len() && data[end] != 0 {
                    end += 1;
                }
                interp = Some(
                    std::str::from_utf8(&data[offset..end])
                        .map_err(|_| ElfLoadError::Parse("invalid interpreter path".into()))?
                        .to_string(),
                );
            }
            _ => {}
        }
    }

    // Check for dynamic linker
    if let Some(interp_path) = interp {
        return Err(ElfLoadError::Unsupported(format!(
            "dynamic binaries not supported (interpreter: {interp_path}), link with -static"
        )));
    }

    if load_segments.is_empty() {
        return Err(ElfLoadError::Parse("no PT_LOAD segments found".into()));
    }

    // Sort segments by virtual address
    load_segments.sort_by_key(|ph| ph.p_vaddr);

    // Find the overall range needed (merging adjacent/overlapping segments)
    let page_size = mmu.page_size() as u64;
    let mut min_vaddr = u64::MAX;
    let mut max_end = 0u64;

    for ph in &load_segments {
        let seg_start = ph.p_vaddr;
        let seg_end = ph.p_vaddr + ph.p_memsz;
        min_vaddr = min_vaddr.min(seg_start);
        max_end = max_end.max(seg_end);
    }

    // Align to page boundaries
    let page_offset = min_vaddr % page_size;
    let aligned_start = min_vaddr - page_offset;
    let aligned_end = max_end.div_ceil(page_size) * page_size;
    let total_size = (aligned_end - aligned_start) as usize;

    // Map the entire region with RWX permissions initially
    // (we'll handle per-segment permissions properly later if needed)
    mmu.mmap_fixed(
        GuestAddr(aligned_start),
        total_size,
        MemPerms::rwx(),
        false,
    )
    .map_err(ElfLoadError::Memory)?;

    // Load each segment's data
    for ph in &load_segments {
        load_segment_data(mmu, data, ph)?;
    }

    Ok(LoadedElf {
        entry: GuestAddr(ehdr.e_entry),
        base_addr: GuestAddr(min_vaddr),
        end_addr: GuestAddr(max_end),
        interp: None,
    })
}

/// Load data for a single PT_LOAD segment into an already-mapped region.
fn load_segment_data(
    mmu: &mut GuestMmu,
    data: &[u8],
    ph: &Elf64Phdr,
) -> Result<(), ElfLoadError> {
    let vaddr = ph.p_vaddr;
    let filesz = ph.p_filesz as usize;
    let memsz = ph.p_memsz as usize;
    let offset = ph.p_offset as usize;

    // Copy file data into the mapping
    if filesz > 0 {
        let gaddr = GuestAddr(vaddr);
        let haddr = mmu.g2h(gaddr).ok_or_else(|| {
            ElfLoadError::Memory(std::io::Error::other(
                "failed to translate guest address",
            ))
        })?;

        // Copy the file content
        let src = &data[offset..offset + filesz];
        let dst = unsafe { std::slice::from_raw_parts_mut(haddr.as_mut_ptr::<u8>(), filesz) };
        dst.copy_from_slice(src);
    }

    // Zero-initialize the BSS (memory beyond file size)
    if memsz > filesz {
        let bss_start = vaddr + filesz as u64;
        let bss_size = memsz - filesz;

        let gaddr = GuestAddr(bss_start);
        if let Some(haddr) = mmu.g2h(gaddr) {
            let bss =
                unsafe { std::slice::from_raw_parts_mut(haddr.as_mut_ptr::<u8>(), bss_size) };
            bss.fill(0);
        }
    }

    Ok(())
}

/// Load an ELF binary and set up the execution environment.
///
/// This is a convenience function that loads the ELF, creates a stack,
/// and returns the entry point and initial stack pointer.
pub fn load_and_setup<P: AsRef<Path>>(
    mmu: &mut GuestMmu,
    path: P,
    _argv: &[String],
    _envp: &[String],
    stack_size: usize,
) -> Result<(GuestAddr, GuestAddr), ElfLoadError> {
    // Load the ELF
    let elf = load_elf_static(mmu, path)?;

    // Allocate stack - always use high addresses but avoid the problematic 0x7fff... range
    let stack_size = mmu.page_size().max(stack_size);
    // Use a fixed high address that's known to work
    let stack_addr = GuestAddr(0x0000_7f00_0000_0000u64);
    mmu.mmap_fixed(stack_addr, stack_size, MemPerms::rw(), true)
        .map_err(ElfLoadError::Memory)?;

    // Workaround for potential LoongArch toolchain issue with large address arithmetic
    // Use checked_add to avoid overflow issues
    let stack_top = GuestAddr(stack_addr.0.checked_add(stack_size as u64).expect("stack overflow"));

    // TODO: Proper stack setup with argc/argv/envp
    // For now, just return the top of stack
    // The runner needs to write the initial stack frame

    Ok((elf.entry, stack_top))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ehdr_validates_magic() {
        let mut data = vec![0u8; 64];
        // Wrong magic
        data[0] = 0x7f;
        data[1] = b'X';
        data[2] = b'L';
        data[3] = b'F';
        data[4] = ELFCLASS64;
        data[5] = ELFDATA2LSB;

        let result = parse_ehdr(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("magic"));
    }

    #[test]
    fn test_parse_ehdr_validates_class() {
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(&ELFMAG);
        data[4] = 1; // 32-bit
        data[5] = ELFDATA2LSB;

        let result = parse_ehdr(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("64-bit"));
    }
}
