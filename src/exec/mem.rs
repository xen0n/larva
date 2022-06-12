use std::{
    collections::HashMap,
    ops::{Add, Sub},
};

use memmap;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct HostAddr(u64);

impl HostAddr {
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct GuestAddr(u64);

impl GuestAddr {
    pub fn as_u64(&self) -> u64 {
        self.0
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
    if len % page_size == 0 {
        len
    } else {
        (len >> page_shift + 1) << page_shift
    }
}

enum MemBlock {
    Map(memmap::MmapMut),
    Injected { _p: *const u8, len: usize },
    InjectedMut { _p: *mut u8, len: usize },
}

impl MemBlock {
    fn len(&self) -> usize {
        match self {
            MemBlock::Map(x) => x.len(),
            MemBlock::Injected { _p: _, len } => *len,
            MemBlock::InjectedMut { _p: _, len } => *len,
        }
    }
}

/// Naïve implementation of an MMU.
pub struct GuestMmu {
    guest_page_size: usize,
    guest_page_shift: usize,
    host_page_size: usize,
    host_page_shift: usize,
    maps: std::sync::RwLock<HashMap<GuestAddr, MemBlock>>,
}
impl GuestMmu {
    pub fn new(guest_page_size: usize) -> Self {
        let host_page_size = host_page_size();
        Self {
            guest_page_size,
            guest_page_shift: get_page_shift(guest_page_size),
            host_page_size,
            host_page_shift: get_page_shift(host_page_size),
            maps: std::sync::RwLock::new(HashMap::new()),
        }
    }

    pub fn consume_host(&mut self, mem: *const u8, len: usize) -> ::std::io::Result<GuestAddr> {
        let m = MemBlock::Injected { _p: mem, len: len };
        let addr = mem as u64;

        let mut maps = self.maps.write().unwrap();
        maps.insert(addr.into(), m);

        Ok(addr.into())
    }

    pub fn consume_host_mut(&mut self, mem: *mut u8, len: usize) -> ::std::io::Result<GuestAddr> {
        let m = MemBlock::InjectedMut { _p: mem, len: len };
        let addr = mem as u64;

        let mut maps = self.maps.write().unwrap();
        maps.insert(addr.into(), m);

        Ok(addr.into())
    }

    // Naïve implementation; doesn't support fixed maps nor file-backed maps.
    pub fn mmap(&mut self, len: usize, stack: bool) -> ::std::io::Result<GuestAddr> {
        if len == 0 {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }

        // align to host page only if host page size is bigger than guest's,
        // else align to guest page
        let len = if self.host_page_size > self.guest_page_size {
            align_to_page(len, self.host_page_size, self.host_page_shift)
        } else {
            align_to_page(len, self.guest_page_size, self.guest_page_shift)
        };

        let mut maps = self.maps.write().unwrap();

        let mut m = memmap::MmapOptions::new();
        m.len(len);
        if stack {
            m.stack();
        }

        let m = m.map_anon()?;
        let addr = m.as_ptr() as u64;
        maps.insert(addr.into(), MemBlock::Map(m));

        Ok(addr.into())
    }

    pub fn munmap(&mut self, g: GuestAddr, len: usize) {
        let mut maps = self.maps.write().unwrap();
        maps.retain(|g_start, m| {
            // check for intersection
            // [g, g + len) vs [g_start, g_start + m.len())
            // only keep those ranges NOT overlapping the requested range
            (g + len) <= *g_start || (*g_start + m.len()) <= g
        });
    }

    pub fn g2h(&self, g: GuestAddr) -> Option<HostAddr> {
        let maps = self.maps.read().unwrap();
        for (g_start, m) in maps.iter() {
            if g < *g_start {
                continue;
            }

            let offset = g - *g_start;
            if offset < m.len() {
                return Some(HostAddr(g.0));
            }
        }

        None
    }
}
