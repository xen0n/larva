use std::{
    collections::BTreeMap,
    ops::{Add, Sub},
};

use memmap;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct HostAddr(pub u64);

impl HostAddr {
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    pub fn as_mut_ptr<T>(&self) -> *mut T {
        self.0 as *mut T
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct GuestAddr(pub u64);

impl GuestAddr {
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn checked_add(&self, offset: usize) -> Option<Self> {
        self.0.checked_add(offset as u64).map(Self)
    }
}

impl From<GuestAddr> for u64 {
    fn from(x: GuestAddr) -> Self {
        x.0
    }
}

impl From<u64> for GuestAddr {
    fn from(x: u64) -> Self {
        Self(x)
    }
}

impl Add<usize> for GuestAddr {
    type Output = Self;

    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs as u64)
    }
}

impl Sub<GuestAddr> for GuestAddr {
    type Output = usize;

    fn sub(self, rhs: GuestAddr) -> Self::Output {
        (self.0 - rhs.0) as usize
    }
}

/// Memory permissions for a mapping.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct MemPerms {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
}

impl MemPerms {
    pub const fn none() -> Self {
        Self {
            read: false,
            write: false,
            exec: false,
        }
    }

    pub const fn r() -> Self {
        Self {
            read: true,
            write: false,
            exec: false,
        }
    }

    pub const fn rw() -> Self {
        Self {
            read: true,
            write: true,
            exec: false,
        }
    }

    pub const fn rx() -> Self {
        Self {
            read: true,
            write: false,
            exec: true,
        }
    }

    pub const fn rwx() -> Self {
        Self {
            read: true,
            write: true,
            exec: true,
        }
    }
}

fn host_page_size() -> usize {
    unsafe { ::libc::sysconf(::libc::_SC_PAGESIZE) as usize }
}

fn get_page_shift(page_size: usize) -> usize {
    // only common sizes
    match page_size {
        1024 => 10,
        2048 => 11,
        4096 => 12,
        8192 => 13,
        16384 => 14,
        32768 => 15,
        65536 => 16,
        131072 => 17,
        262144 => 18,
        524288 => 19,
        1048576 => 20,
        _ => unimplemented!(),
    }
}

fn align_to_page(len: usize, page_size: usize, page_shift: usize) -> usize {
    if len.is_multiple_of(page_size) {
        len
    } else {
        // Round up to next page: ((len / page_size) + 1) * page_size
        ((len >> page_shift) + 1) << page_shift
    }
}

#[allow(dead_code)]
fn align_down(addr: u64, align: usize) -> u64 {
    addr & !(align as u64 - 1)
}

#[allow(dead_code)]
fn align_up(addr: u64, align: usize) -> u64 {
    addr.div_ceil(align as u64)
}

/// A single memory mapping region.
struct MemRegion {
    /// Start address in guest address space
    guest_start: GuestAddr,
    /// Length of the mapping
    len: usize,
    /// Host memory backing this region
    host_ptr: *mut u8,
    /// Permissions
    perms: MemPerms,
    /// Whether this is a file-backed mapping (currently unused)
    _file_backed: bool,
    /// The underlying mmap (if we own it)
    _mmap: Option<memmap::MmapMut>,
}

impl MemRegion {
    fn contains(&self, gaddr: GuestAddr) -> bool {
        let start = self.guest_start.as_u64();
        let end = start + self.len as u64;
        gaddr.as_u64() >= start && gaddr.as_u64() < end
    }

    fn offset_of(&self, gaddr: GuestAddr) -> usize {
        gaddr - self.guest_start
    }

    fn host_addr(&self, gaddr: GuestAddr) -> HostAddr {
        let offset = self.offset_of(gaddr);
        HostAddr(self.host_ptr as u64 + offset as u64)
    }

    #[allow(dead_code)]
    fn end(&self) -> GuestAddr {
        GuestAddr(self.guest_start.as_u64() + self.len as u64)
    }

    /// Check if this region overlaps with the given range
    fn overlaps(&self, start: GuestAddr, len: usize) -> bool {
        let self_start = self.guest_start.as_u64();
        let self_end = self_start + self.len as u64;
        let other_start = start.as_u64();
        let other_end = other_start + len as u64;

        self_start < other_end && other_start < self_end
    }
}

// Safety: MemRegion is Send/Sync if the pointer is valid and we don't
// mutate through it concurrently. The MMU ensures exclusive access.
unsafe impl Send for MemRegion {}
unsafe impl Sync for MemRegion {}

/// Guest Memory Management Unit.
///
/// Manages the mapping from guest virtual addresses to host physical addresses.
/// Uses a BTreeMap for efficient range-based lookups.
pub struct GuestMmu {
    guest_page_size: usize,
    guest_page_shift: usize,
    host_page_size: usize,
    host_page_shift: usize,
    /// Maps guest start address to memory region
    regions: std::sync::RwLock<BTreeMap<GuestAddr, MemRegion>>,
    /// Next available address for anonymous mappings ( grows downward from high addresses)
    next_mmap_addr: std::sync::Mutex<u64>,
}

impl GuestMmu {
    pub fn new(guest_page_size: usize) -> Self {
        let host_page_size = host_page_size();
        Self {
            guest_page_size,
            guest_page_shift: get_page_shift(guest_page_size),
            host_page_size,
            host_page_shift: get_page_shift(host_page_size),
            regions: std::sync::RwLock::new(BTreeMap::new()),
            // Start allocating from high addresses (below 2^48 for RISC-V SV39)
            next_mmap_addr: std::sync::Mutex::new(0x7fff_ffff_0000),
        }
    }

    /// Find a free region of at least `len` bytes.
    fn find_free_region(&self, len: usize, hint: Option<GuestAddr>) -> Option<GuestAddr> {
        let len = self.align_alloc_size(len);
        let regions = self.regions.read().unwrap();

        if let Some(hint) = hint {
            // Try the hint address first
            if self.is_range_free(hint, len, &regions) {
                return Some(hint);
            }
        }

        // Allocate from the top down
        let mut addr = *self.next_mmap_addr.lock().unwrap();
        loop {
            let gaddr = GuestAddr(addr);
            if self.is_range_free(gaddr, len, &regions) {
                return Some(gaddr);
            }
            // Move down by page size
            addr = addr.saturating_sub(len as u64);
            if addr < 0x10000 {
                // Don't go below 64KB
                return None;
            }
        }
    }

    /// Check if a range is free (no overlapping regions).
    fn is_range_free(
        &self,
        start: GuestAddr,
        len: usize,
        regions: &BTreeMap<GuestAddr, MemRegion>,
    ) -> bool {
        let start_addr = start.as_u64();
        let end_addr = start_addr + len as u64;

        // Check for overlap with any existing region
        for (_, region) in regions.iter() {
            let region_start = region.guest_start.as_u64();
            let region_end = region_start + region.len as u64;

            if start_addr < region_end && region_start < end_addr {
                return false; // Overlap found
            }
        }
        true
    }

    fn align_alloc_size(&self, len: usize) -> usize {
        // Align to the larger of host and guest page size
        if self.host_page_size > self.guest_page_size {
            align_to_page(len, self.host_page_size, self.host_page_shift)
        } else {
            align_to_page(len, self.guest_page_size, self.guest_page_shift)
        }
    }

    /// Create an anonymous mapping at a specific address.
    /// Returns error if the address is already occupied.
    pub fn mmap_fixed(
        &mut self,
        addr: GuestAddr,
        len: usize,
        perms: MemPerms,
        stack: bool,
    ) -> ::std::io::Result<GuestAddr> {
        if len == 0 {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }

        let len = self.align_alloc_size(len);

        // Check if the range is free
        {
            let regions = self.regions.read().unwrap();
            if !self.is_range_free(addr, len, &regions) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "address range already occupied",
                ));
            }
        }

        // Create the mapping
        let mut m = memmap::MmapOptions::new();
        m.len(len);
        if stack {
            m.stack();
        }

        let mmap = m.map_anon()?;
        let host_ptr = mmap.as_ptr() as *mut u8;

        let region = MemRegion {
            guest_start: addr,
            len,
            host_ptr,
            perms,
            _file_backed: false,
            _mmap: Some(mmap),
        };

        let mut regions = self.regions.write().unwrap();
        regions.insert(addr, region);

        Ok(addr)
    }

    /// Create an anonymous mapping at any available address.
    pub fn mmap(
        &mut self,
        len: usize,
        perms: MemPerms,
        stack: bool,
    ) -> ::std::io::Result<GuestAddr> {
        if len == 0 {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }

        let len_aligned = self.align_alloc_size(len);
        let addr = self
            .find_free_region(len_aligned, None)
            .ok_or_else(|| std::io::Error::other("out of memory"))?;

        self.mmap_fixed(addr, len, perms, stack)?;

        // Update next_mmap_addr if we allocated from there
        let mut next = self.next_mmap_addr.lock().unwrap();
        if addr.as_u64() <= *next {
            *next = addr.as_u64().saturating_sub(len_aligned as u64);
        }

        Ok(addr)
    }

    /// Map host memory into the guest address space at a specific address.
    pub fn map_host_fixed(
        &mut self,
        gaddr: GuestAddr,
        host_ptr: *mut u8,
        len: usize,
        perms: MemPerms,
    ) -> ::std::io::Result<GuestAddr> {
        let len = self.align_alloc_size(len);

        // Check if the range is free
        {
            let regions = self.regions.read().unwrap();
            if !self.is_range_free(gaddr, len, &regions) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "address range already occupied",
                ));
            }
        }

        let region = MemRegion {
            guest_start: gaddr,
            len,
            host_ptr,
            perms,
            _file_backed: false,
            _mmap: None,
        };

        let mut regions = self.regions.write().unwrap();
        regions.insert(gaddr, region);

        Ok(gaddr)
    }

    /// Map host memory into the guest address space at any available address.
    /// This is useful for injecting static data or code.
    pub fn map_host(
        &mut self,
        host_ptr: *const u8,
        len: usize,
        perms: MemPerms,
    ) -> ::std::io::Result<GuestAddr> {
        let len_aligned = self.align_alloc_size(len);

        // Find a free region
        let gaddr = self
            .find_free_region(len_aligned, None)
            .ok_or_else(|| std::io::Error::other("out of memory"))?;

        // Check if the range is free
        {
            let regions = self.regions.read().unwrap();
            if !self.is_range_free(gaddr, len_aligned, &regions) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "address range already occupied",
                ));
            }
        }

        let region = MemRegion {
            guest_start: gaddr,
            len: len_aligned,
            host_ptr: host_ptr as *mut u8,
            perms,
            _file_backed: false,
            _mmap: None,
        };

        let mut regions = self.regions.write().unwrap();
        regions.insert(gaddr, region);

        Ok(gaddr)
    }

    /// Unmap a region of memory.
    pub fn munmap(&mut self, g: GuestAddr, len: usize) {
        let mut regions = self.regions.write().unwrap();

        // Find all regions that overlap with the range to unmap
        let to_remove: Vec<GuestAddr> = regions
            .values()
            .filter(|r| r.overlaps(g, len))
            .map(|r| r.guest_start)
            .collect();

        for addr in to_remove {
            regions.remove(&addr);
        }
    }

    /// Find the region containing the given guest address.
    fn find_region(&self, g: GuestAddr) -> Option<MemRegion> {
        let regions = self.regions.read().unwrap();

        // Find the region with the largest start address that is <= g
        let candidate = regions.range(..=g).next_back().map(|(_, r)| r);

        if let Some(region) = candidate
            && region.contains(g)
        {
            // Clone the region data (pointers are Copy, other fields are small)
            return Some(MemRegion {
                guest_start: region.guest_start,
                len: region.len,
                host_ptr: region.host_ptr,
                perms: region.perms,
                _file_backed: region._file_backed,
                _mmap: None, // Don't clone the mmap
            });
        }

        None
    }

    /// Translate guest address to host address.
    pub fn g2h(&self, g: GuestAddr) -> Option<HostAddr> {
        self.find_region(g).map(|r| r.host_addr(g))
    }

    /// Translate guest address to host address with permission check.
    pub fn g2h_with_perms(
        &self,
        g: GuestAddr,
        read: bool,
        write: bool,
        exec: bool,
    ) -> Option<HostAddr> {
        let region = self.find_region(g)?;

        if read && !region.perms.read {
            return None;
        }
        if write && !region.perms.write {
            return None;
        }
        if exec && !region.perms.exec {
            return None;
        }

        Some(region.host_addr(g))
    }

    /// Get the permissions for a guest address.
    pub fn get_perms(&self, g: GuestAddr) -> Option<MemPerms> {
        self.find_region(g).map(|r| r.perms)
    }

    /// Set permissions for a region.
    pub fn mprotect(&mut self, g: GuestAddr, len: usize, perms: MemPerms) -> ::std::io::Result<()> {
        // For now, we don't support changing permissions on partial regions
        // Find the containing region
        let regions = self.regions.read().unwrap();
        let candidate = regions.range(..=g).next_back().map(|(_, r)| r);

        if let Some(region) = candidate
            && region.guest_start == g
            && region.len == self.align_alloc_size(len)
        {
            drop(regions);
            let mut regions = self.regions.write().unwrap();
            if let Some(r) = regions.get_mut(&g) {
                r.perms = perms;
                return Ok(());
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "mprotect on partial or non-existent region not supported",
        ))
    }

    /// Get the page size used by this MMU.
    pub fn page_size(&self) -> usize {
        self.guest_page_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_to_page() {
        // 4K page size
        let page_size = 4096;
        let page_shift = 12;

        // Already aligned
        assert_eq!(align_to_page(4096, page_size, page_shift), 4096);
        assert_eq!(align_to_page(8192, page_size, page_shift), 8192);

        // Small values that need rounding up
        assert_eq!(align_to_page(1, page_size, page_shift), 4096);
        assert_eq!(align_to_page(4095, page_size, page_shift), 4096);
        assert_eq!(align_to_page(4097, page_size, page_shift), 8192);
        assert_eq!(align_to_page(5000, page_size, page_shift), 8192);
    }

    #[test]
    fn test_g2h_with_mmap() {
        let mut mmu = GuestMmu::new(4096);

        // Allocate a page
        let gaddr = mmu.mmap(4096, MemPerms::rwx(), false).unwrap();

        // g2h should return a valid host address
        let haddr = mmu.g2h(gaddr);
        assert!(
            haddr.is_some(),
            "g2h should return Some for allocated memory"
        );

        // The host address should be valid (non-zero and properly aligned)
        let haddr = haddr.unwrap();
        assert!(haddr.as_u64() != 0, "Host address should be non-zero");
        // And it should be page-aligned
        assert_eq!(
            haddr.as_u64() % 4096,
            0,
            "Host address should be page-aligned"
        );
    }

    #[test]
    fn test_g2h_with_fixed_mmap() {
        let mut mmu = GuestMmu::new(4096);

        // Map at a specific address
        let target_addr = GuestAddr(0x10000);
        let gaddr = mmu
            .mmap_fixed(target_addr, 4096, MemPerms::rwx(), false)
            .unwrap();
        assert_eq!(gaddr, target_addr);

        // g2h should work
        let haddr = mmu.g2h(gaddr);
        assert!(haddr.is_some());

        // Trying to map at the same address should fail
        let result = mmu.mmap_fixed(target_addr, 4096, MemPerms::rwx(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_munmap() {
        let mut mmu = GuestMmu::new(4096);

        // Allocate two adjacent pages
        let gaddr1 = mmu.mmap(4096, MemPerms::rwx(), false).unwrap();
        let gaddr2 = mmu.mmap(4096, MemPerms::rwx(), false).unwrap();

        // Both should be accessible
        assert!(mmu.g2h(gaddr1).is_some());
        assert!(mmu.g2h(gaddr2).is_some());

        // Unmap the first
        mmu.munmap(gaddr1, 4096);

        // First should no longer be accessible
        assert!(mmu.g2h(gaddr1).is_none());
        // Second should still be accessible
        assert!(mmu.g2h(gaddr2).is_some());
    }

    #[test]
    fn test_permission_checks() {
        let mut mmu = GuestMmu::new(4096);

        // Map with read-only permissions
        let gaddr = mmu.mmap(4096, MemPerms::r(), false).unwrap();

        // Should succeed with read permission
        assert!(mmu.g2h_with_perms(gaddr, true, false, false).is_some());

        // Should fail with write permission
        assert!(mmu.g2h_with_perms(gaddr, false, true, false).is_none());

        // Should fail with exec permission
        assert!(mmu.g2h_with_perms(gaddr, false, false, true).is_none());
    }

    #[test]
    fn test_unmapped() {
        let mmu = GuestMmu::new(4096);

        // g2h on unmapped address should return None
        let unmapped: GuestAddr = 0xDEADBEEFu64.into();
        assert!(mmu.g2h(unmapped).is_none());
    }
}
