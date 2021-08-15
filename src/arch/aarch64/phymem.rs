use crate::println;

use arrayvec::ArrayVec;
use core::fmt::{Debug, Formatter};
use core::{fmt, ops};
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

impl ops::Sub for FreeRange {
    type Output = (Option<FreeRange>, Option<FreeRange>);

    fn sub(self, rhs: Self) -> Self::Output {
        if rhs.base < self.base {
            // Reserved region starts before us
            if rhs.base + rhs.len > self.base {
                // Reserved region intersects us on the left
                if rhs.base + rhs.len > self.base + self.len {
                    // Reserved region contains us
                    (None, None)
                } else {
                    // Reserved region intersects a chunk of us on the left
                    let mut new = self;
                    new.base = rhs.base + rhs.len;
                    new.len -= new.base - self.base;
                    (Some(new), None)
                }
            } else {
                // Reserved region is entirely before us, ignore it
                (Some(self), None)
            }
        } else {
            // Reserved region starts after our base
            if rhs.base >= self.base + self.len {
                // Reserved region starts after our end, ignore it
                (Some(self), None)
            } else {
                // Reserved region starts before our end

                let mut left = self;
                left.len = rhs.base - self.base;

                if rhs.base + rhs.len >= self.base + self.len {
                    // Reserved region ends after us
                    (Some(left), None)
                } else {
                    // Reserved region ends inside of us, need to split
                    let mut right = self;
                    right.base = rhs.base + rhs.len;
                    right.len = self.len - rhs.len - left.len;
                    (Some(left), Some(right))
                }
            }
        }
    }
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
        // FIXME: For framebuffer, etc
        const END_RESERVE: usize = 0xf000000;
        const END_RESERVE_PAGES: usize = END_RESERVE / PAGE_SIZE;

        let ram_start = &__ram_start as *const u8 as usize;
        let ram_end = &__ram_end as *const u8 as usize;
        let ram_start_pages = (&__ram_start as *const u8 as usize) / PAGE_SIZE;
        let ram_end_pages = (&__ram_end as *const u8 as usize) / PAGE_SIZE;
        println!(
            "[DBUG] Phymem range: 0p{:x}..0p{:x}",
            ram_start,
            ram_end - END_RESERVE
        );
        self.free_count = ((ram_end_pages - END_RESERVE_PAGES) - ram_start_pages) as u32;
        self.data[0] = FreeRange {
            base: ram_start_pages as u32,
            len: self.free_count,
        };
    }

    pub unsafe fn alloc_page(&mut self) -> Option<PhyAddr> {
        self.alloc_pages(1).map(|slice| slice.base)
    }

    pub unsafe fn alloc_pages(&mut self, len: u32) -> Option<PhySlice> {
        for range in &mut self.data[0..=self.head as usize].iter_mut().rev() {
            if range.len >= len {
                let result = PhySlice {
                    base: PhyAddr(((range.base + range.len - len) as usize * PAGE_SIZE) as usize),
                    len: (len as usize) * PAGE_SIZE,
                };
                range.len -= len;
                if range.len == 0 && self.head != 0 {
                    range.base = 0;
                }
                self.free_count -= len;
                return Some(result);
            }
        }
        None
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

    pub unsafe fn reserve_range(&mut self, range: PhySlice) -> Result<(), ()> {
        println!("[INFO] Reserving range {:?}", range);
        let base_pages = (range.base.0 / PAGE_SIZE) as u32;
        let len_pages = (range.len / PAGE_SIZE) as u32;
        let mut added_ranges: ArrayVec<_, 8> = ArrayVec::new();
        for range in &mut self.data[0..=self.head as usize] {
            let (left, right) = *range
                - FreeRange {
                    base: base_pages,
                    len: len_pages,
                };
            if let Some(left_range) = left {
                *range = left_range;
            } else {
                // FIXME: leaving hole
                *range = FreeRange { base: 0, len: 0 }
            }
            if let Some(right_range) = right {
                added_ranges.push(right_range);
            }
        }

        if self.head as usize + added_ranges.len() >= self.data.len() {
            panic!("Phymem free list overflow");
        }
        (&mut self.data[self.head as usize + 1..self.head as usize + 1 + added_ranges.len()])
            .copy_from_slice(&added_ranges);
        self.head += added_ranges.len() as u32;

        Ok(())
    }

    // TODO: free_pages
}

pub unsafe fn reserve(range: PhySlice) -> Result<(), ()> {
    PHYMEM_FREE_LIST.lock().reserve_range(range)
}
