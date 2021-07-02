use crate::println;
use core::fmt;
use core::fmt::{Debug, Formatter};
use spin::Mutex;

extern "C" {
    static mut __ram_start: u8;
    static mut __ram_end: u8;
}

const PAGE_SIZE: usize = 4096;
const RAM_SIZE: usize = 256 * 1024 * 1024;
pub static PHYMEM_FREE_LIST: Mutex<FreeList> = Mutex::new(unsafe { FreeList::new() });

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PhyAddr(pub usize);

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PhySlice {
    pub base: PhyAddr,
    pub len: usize,
}

impl Debug for PhyAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "0p{:x}", self.0)
    }
}

impl Debug for PhySlice {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "0p{:x}..0p{:x}", self.base.0, self.base.0 + self.len)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FreeRange {
    pub base: u32,
    pub len: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FreeList {
    head: u32,
    free_count: u32,
    data: [FreeRange; RAM_SIZE / PAGE_SIZE],
}

impl FreeList {
    pub const unsafe fn new() -> FreeList {
        FreeList {
            head: 0,
            free_count: 0,
            data: [FreeRange { base: 0, len: 0 }; RAM_SIZE / PAGE_SIZE],
        }
    }

    pub unsafe fn init(&mut self) {
        let ram_start = (&__ram_start as *const u8 as usize);
        let ram_end = (&__ram_end as *const u8 as usize);
        let ram_start_pages = (&__ram_start as *const u8 as usize) / PAGE_SIZE;
        let ram_end_pages = (&__ram_end as *const u8 as usize) / PAGE_SIZE;
        println!("[DBUG] Phymem range: 0p{:x}..0p{:x}", ram_start, ram_end);
        self.free_count = (ram_end_pages - ram_start_pages) as u32;
        self.data[0] = FreeRange {
            base: ram_start_pages as u32,
            len: self.free_count,
        };
    }

    pub unsafe fn alloc_page(&mut self) -> Option<PhyAddr> {
        let head = &mut self.data[self.head as usize];
        if head.len > 0 {
            let result = PhyAddr(((head.base + head.len - 1) as usize * PAGE_SIZE) as usize);
            head.len -= 1;
            if head.len == 0 && self.head != 0 {
                head.base = 0;
            }
            self.free_count -= 1;
            Some(result)
        } else {
            None
        }
    }

    pub unsafe fn alloc_pages(&mut self, len: u32) -> Option<PhySlice> {
        let head = &mut self.data[self.head as usize];
        if head.len >= len {
            let result = PhySlice {
                base: PhyAddr(((head.base + head.len - len) as usize * PAGE_SIZE) as usize),
                len: (len as usize) * PAGE_SIZE,
            };
            head.len -= len;
            if head.len == 0 && self.head != 0 {
                head.base = 0;
            }
            self.free_count -= len;
            Some(result)
        } else {
            None
        }
    }

    pub unsafe fn free_page(&mut self, addr: PhyAddr) {
        let page = (addr.0 / PAGE_SIZE) as u32;

        let head = &mut self.data[self.head as usize];
        if page == head.base - 1 {
            head.base -= 1;
        } else if page == head.base + head.len {
            head.len += 1;
        } else {
            self.head += 1;
            if self.head == self.data.len() as u32 {
                panic!("Phymem free list overflow");
            }
            let new_head = &mut self.data[self.head as usize];
            new_head.base = page;
            new_head.len = 1;
        }
        self.free_count += 1;
    }
}
