use crate::arch::aarch64::mmio;
use crate::prelude::*;
use core::mem::size_of;

pub const PAGE_SIZE: u64 = 4096;

// Granularity
pub const PT_PAGE: u64 = 0b11;
pub const PT_BLOCK: u64 = 0b01;

// Accessibility
pub const PT_KERNEL: u64 = 0 << 6;
pub const PT_USER: u64 = 1 << 6;
pub const PT_RW: u64 = 0 << 7;
pub const PT_RO: u64 = 1 << 7;
pub const PT_AF: u64 = 1 << 10;
pub const PT_NX: u64 = 1 << 54;

// Share-ability
pub const PT_OSH: u64 = 0b10 << 8;
pub const PT_ISH: u64 = 0b11 << 8;

// defined in MAIR register
pub const PT_MEM: u64 = 0 << 2;
pub const PT_DEV: u64 = 1 << 2;
pub const PT_NC: u64 = 2 << 2;

const TTBR_CNP: u64 = 1;

static mut PAGING: PageTables = unsafe { PageTables::new() };

#[repr(C, align(4096))]
struct PageTable(pub [u64; 512]);

impl PageTable {
    pub const unsafe fn new() -> Self {
        Self([0; 512])
    }

    pub unsafe fn use_child<F: FnOnce(Option<(&u64, &PageTable)>)>(&self, idx: usize, f: F) {
        let paddr = self.0[idx] & 0x7FFFFFF000;
        let raw = &self.0[idx];
        if paddr != 0 {
            // FIXME: Assuming ident map
            let vaddr = PhyAddr(paddr as usize).virt() as *const PageTable;
            (f)(vaddr.as_ref().map(|v| (raw, v)));
        } else {
            (f)(None);
        }
    }

    pub unsafe fn use_child_mut<F: FnOnce(Option<(&mut u64, &mut PageTable)>)>(
        &mut self,
        idx: usize,
        f: F,
    ) {
        let paddr = self.0[idx] & 0x7FFFFFF000;
        let raw = &mut self.0[idx];
        if paddr != 0 {
            // FIXME: Assuming ident map
            let vaddr = PhyAddr(paddr as usize).virt_mut() as *mut PageTable;
            (f)(vaddr.as_mut().map(|v| (raw, v)));
        } else {
            (f)(None);
        }
    }
}

unsafe fn drop_pagetable(pt: &mut PageTable) {
    let pt_addr = pt as *const PageTable as usize;
    let paging_addr = &PAGING as *const PageTables as usize;
    let paging_end = paging_addr + size_of::<PageTable>();
    if pt_addr < paging_addr || pt_addr >= paging_end {
        drop(Box::from_raw(pt));
    }
    println!("[DBUG] VMAP: Dropped a page table at 0x{:x}", pt_addr);
}

// TODO: AtomicU64?
#[repr(C, align(4096))]
struct PageTables {
    user_l1: PageTable,
    user_l2: PageTable,
    user_l3: PageTable,
}

impl PageTables {
    const unsafe fn new() -> Self {
        Self {
            user_l1: PageTable::new(),
            user_l2: PageTable::new(),
            user_l3: PageTable::new(),
        }
    }
}

extern "C" {
    static mut __data_start: u8;
    static mut __dma_start: u8;
    static mut __dma_end: u8;
}

/// # Safety
///
/// This function assumes it runs only once, from low memory
pub unsafe fn init() -> Result<(), ()> {
    // println!("[DBUG] CurrentEL: {}", (get_msr!(CurrentEL) >> 2) & 0b11);
    let mut paging =
        &mut *((&mut PAGING as *mut PageTables as usize & 0xffffffff) as *mut PageTables);

    // Identity map user area, L1 Table
    paging.user_l1.0[0] = {
        (paging.user_l2.0.as_ptr() as u64) | // Physical address
            PT_PAGE |     // it has the "Present" flag, which must be set, and we have area in it mapped by pages
            PT_AF |       // accessed flag. Without this we're going to have a Data Abort exception
            PT_USER |     // non-privileged
            PT_ISH |      // inner shareable
            PT_MEM // normal memory
    };

    // Identity map user area, L2 Table, first block
    paging.user_l2.0[0] = {
        (paging.user_l3.0.as_ptr() as u64) | // Physical address
            PT_PAGE |     // it has the "Present" flag, which must be set, and we have area in it mapped by pages
            PT_AF |       // accessed flag. Without this we're going to have a Data Abort exception
            PT_USER |     // non-privileged
            PT_ISH |      // inner shareable
            PT_MEM // normal memory
    };

    // Identity map user area, L2 Table
    let iomem_cutoff = (mmio::MMIO_BASE >> 21) as usize;
    let data_cutoff = (((&__data_start) as *const u8 as u64 & 0xffffffff) / PAGE_SIZE) as usize;
    let dma_start = (((&__dma_start) as *const u8 as u64 & 0xffffffff) / PAGE_SIZE) as usize;
    let dma_end = (((&__dma_end) as *const u8 as u64 & 0xffffffff) / PAGE_SIZE) as usize;
    // println!("dma_start = 0x{:x}, dma_end = 0x{:x}", dma_start, dma_end);
    for (i, tbl) in paging.user_l2.0.iter_mut().enumerate().skip(1) {
        *tbl = {
            (i << 21) as u64 | // Physical address
                PT_BLOCK |    // map 2M block
                PT_AF |       // accessed flag
                PT_NX |       // no execute
                PT_USER |     // non-privileged
                // different attributes for device memory
                if i >= iomem_cutoff {
                    // println!("Defining 0x{:x} as DMA memory", i << 21);
                    PT_OSH | PT_DEV
                } else {
                    PT_ISH | PT_MEM
                }
        };
    }

    // User L3 table
    for (i, tbl) in paging.user_l3.0.iter_mut().enumerate() {
        *tbl = {
            (i as u64 * PAGE_SIZE) | // Physical address
                PT_PAGE |     // map 4k
                PT_AF |       // accessed flag
                PT_USER |     // non-privileged
                if i >= dma_start && i < dma_end {
                    // println!("Defining 0x{:x} as DMA memory", i as u64 * PAGE_SIZE);
                    PT_OSH | PT_NC | PT_RW | PT_NX
                } else if i < 0x80 || i >= data_cutoff {
                    PT_MEM | PT_ISH | PT_RW | PT_NX
                } else {
                    PT_MEM | PT_ISH | PT_RO
                }
        };
    }

    // Verify MMU is capable
    let id_aa64mmfr0_el1 = get_msr!(id_aa64mmfr0_el1);
    let tgran4_supp = id_aa64mmfr0_el1 & (0xF << 28) == 0;
    let pa_range = id_aa64mmfr0_el1 & 0xF;
    let pa_range_supp = pa_range >= 1;

    if !tgran4_supp || !pa_range_supp {
        return Err(());
    }

    // Set Memory Attributes array, indexed by PT_MEM, PT_DEV, PT_NC in our example
    set_msr!(mair_el1, {
        0xFF | // AttrIdx=0: normal, IWBWA, OWBWA, NTR
        (0x04 << 8) | // AttrIdx=1: device, nGnRE (must be OSH too)
        (0x44 << 16) // AttrIdx=2: non cacheable
    });

    // Specify mapping characteristics in translate control register
    #[allow(clippy::identity_op)]
    {
        set_msr!(tcr_el1, {
            (0b00 << 37) | // TBI=0, no tagging
                ((pa_range as u64) << 32) |      // IPS=autodetected
                (0b10 << 30) | // TG1=4k
                (0b11 << 28) | // SH1=3 inner
                (0b01 << 26) | // ORGN1=1 write back
                (0b01 << 24) | // IRGN1=1 write back
                (0b0  << 23) | // EPD1 enable higher half
                (25   << 16) | // T1SZ=25, 3 levels (512G)
                (0b00 << 14) | // TG0=4k
                (0b11 << 12) | // SH0=3 inner
                (0b01 << 10) | // ORGN0=1 write back
                (0b01 << 8) |  // IRGN0=1 write back
                (0b0  << 7) |  // EPD0 enable lower half
                (25   << 0) as u64 // T0SZ=25, 3 levels (512G)
        });
    }

    asm!("isb");

    // Tell the MMU where our translation tables are. TTBR_CNP bit not documented, but required
    let paging_addr = (paging.user_l1.0.as_ptr() as u64) & 0xffffffff;
    // - lower half, user space
    set_msr!(ttbr0_el1, paging_addr + TTBR_CNP);
    // - upper half, kernel space
    set_msr!(ttbr1_el1, paging_addr + TTBR_CNP);

    // Finally, toggle some bits in system control register to enable page translation
    asm!("dsb ish", "isb", options(nomem, nostack));
    let mut sctlr_el1 = get_msr!(sctlr_el1);
    sctlr_el1 |= 0xC00800; // set mandatory reserved bits
    sctlr_el1 &= !((1<<25) |   // clear EE, little endian translation tables
            // (1<<57) |   // clear PAN3
            // (1<<12) |   // clear SPAN
            (1<<24) |   // clear E0E
            (1<<19) |   // clear WXN
            (1<<12) |   // clear I, no instruction cache
            (1<<4) |    // clear SA0
            (1<<3) |    // clear SA
            // (1<<2) |    // clear C, no cache at all
            (1<<1)); // clear A, no aligment check
    sctlr_el1 |= (1 << 0) // Set M, enable MMU
        | (1<<2); // Set C, no cache at all
    set_msr!(sctlr_el1, sctlr_el1);
    asm!("isb");

    // println!("[DBUG] MMU Initialized");
    Ok(())
}

pub unsafe fn eject_lowmem() {
    extern "C" {
        static mut _vectors: u8;
    }
    set_msr!(vbar_el1, &_vectors as *const u8 as usize);
    set_msr!(ttbr0_el1, 0);
}

pub unsafe fn virt2pte_mut<F: FnMut(Option<(&mut u64, usize)>)>(vaddr: usize, mut f: F) {
    // TODO: Only supports ttbr0 for now
    let mut vaddr = vaddr & 0x7fffffffff;
    let lvl3_offset = vaddr % (PAGE_SIZE as usize);
    vaddr -= lvl3_offset;

    // println!("V2P: looking for 0x{:x}", vaddr);
    let mut called = false;
    let lvl1 = &mut PAGING.user_l1;
    lvl1.use_child_mut(vaddr >> 30, |lvl2| {
        if let Some((raw1, lvl2)) = lvl2 {
            if *raw1 & PT_PAGE != PT_PAGE {
                // Huge page
                let lvl1_offset = vaddr & 0x3fffffff;
                (f)(Some((raw1, lvl1_offset + lvl3_offset)));
                called = true;
            } else {
                lvl2.use_child_mut((vaddr >> 21) % 512, |lvl3| {
                    if let Some((raw2, lvl3)) = lvl3 {
                        if *raw2 & PT_PAGE != PT_PAGE {
                            // Huge page
                            let lvl2_offset = vaddr & 0x1fffff;
                            (f)(Some((raw2, lvl2_offset + lvl3_offset)));
                        } else {
                            (f)(Some((&mut lvl3.0[(vaddr >> 12) % 512], lvl3_offset)));
                        }
                        called = true;
                    } else {
                        // println!("V2P: Not found in L2");
                    }
                });
            }
        } else {
            // println!("V2P: Not found in L1");
        }
    });
    if !called {
        (f)(None);
    }
}

pub unsafe fn virt2pte(vaddr: usize) -> Option<(u64, usize)> {
    let mut res = None;
    {
        let res = &mut res;
        virt2pte_mut(vaddr, move |r| *res = r.map(|(pte, offset)| (*pte, offset)));
    }
    res
}

pub unsafe fn virt2phy(vaddr: usize) -> Option<PhyAddr> {
    virt2pte(vaddr).map(|(pte, offset)| PhyAddr(((pte as usize) & 0x7FFFFFF000) + offset))
}

/// Gnarly code ahead
pub unsafe fn vmap(vaddr: usize, paddr: PhyAddr, attrs: u64) -> Result<(), ()> {
    // TODO: Only supports ttbr0 for now
    // TODO: Doesn't ever free lvl2,3 tables if empty
    if vaddr % (PAGE_SIZE as usize) != 0 {
        return Err(());
    }

    let mut res = Err(());
    println!("[DBUG] VMAP: Mapping 0x{:x} to {:?}", vaddr, paddr);

    const COMMON_FLAGS: u64 = PT_PAGE | // it has the "Present" flag, which must be set, and we have area in it mapped by pages
        PT_AF; // accessed flag. Without this we're going to have a Data Abort exception

    const TABLE_FLAGS: u64 = PT_PAGE | // it has the "Present" flag, which must be set, and we have area in it mapped by pages
            PT_AF | // accessed flag. Without this we're going to have a Data Abort exception
            PT_USER | // non-privileged
            PT_ISH | // inner shareable
            PT_MEM; // normal memory

    // Look for lvl1 entry
    let lvl2 = PAGING.user_l1.0[vaddr >> 30];
    if lvl2 == 0 {
        // Create new lvl2 table
        let new_table = Box::new(PageTable::new());
        PAGING.user_l1.0[vaddr >> 30] =
            Box::leak(new_table) as *mut PageTable as usize as u64 | TABLE_FLAGS;
        println!(
            "[DBUG] VMAP: Allocated new lvl2 page table: 0x{:x}",
            PAGING.user_l1.0[vaddr >> 30]
        );
    }

    // lvl2 table exists now
    PAGING.user_l1.use_child_mut(vaddr >> 30, |lvl2| {
        if let Some((_, lvl2)) = lvl2 {
            let lvl3 = lvl2.0[(vaddr >> 21) % 512];
            if lvl3 == 0 {
                // Create new lvl3 table
                let new_table = Box::new(PageTable::new());
                lvl2.0[(vaddr >> 21) % 512] =
                    Box::leak(new_table) as *mut PageTable as usize as u64 | TABLE_FLAGS;
                println!(
                    "[DBUG] VMAP: Allocated new lvl3 page table: 0x{:x}",
                    lvl2.0[(vaddr >> 21) % 512]
                );
            }

            // lvl3 table exists now
            lvl2.use_child_mut((vaddr >> 21) % 512, |lvl3| {
                if let Some((_, lvl3)) = lvl3 {
                    if lvl3.0[(vaddr >> 12) % 512] == 0 {
                        lvl3.0[(vaddr >> 12) % 512] = paddr.0 as u64 | COMMON_FLAGS | attrs;
                        res = Ok(());
                    } else {
                        // Already mapped!
                        println!(
                            "[WARN] VMAP: Tried to map 0x{:x} to {:?}, already mapped to {:?}",
                            vaddr,
                            paddr,
                            PhyAddr((lvl3.0[(vaddr >> 12) % 512] & 0x7FFFFFF000) as usize)
                        );
                        res = Err(());
                    }
                } else {
                    unreachable!();
                };
            });
        } else {
            unreachable!();
        }
    });
    if res.is_ok() {
        asm!("dsb ishst", "tlbi vmalle1is", "dsb ish", "isb"); // taken from linux
    }
    res
}

/// Gnarly code ahead
pub unsafe fn vunmap(vaddr: usize) -> Result<(), ()> {
    // TODO: Only supports ttbr0 for now
    // TODO: Doesn't ever free lvl2,3 tables if empty
    // TODO: Frees whole huge pages
    if vaddr % (PAGE_SIZE as usize) != 0 {
        return Err(());
    }

    let mut res = Err(());
    println!("[DBUG] VMAP: Unmapping 0x{:x}", vaddr);

    virt2pte_mut(vaddr, |pte| {
        if let Some((pte, _)) = pte {
            *pte = 0;
            res = Ok(());
        } else {
            println!("[WARN] VMAP: Double vunmap of 0x{:x}", vaddr);
        }
    });
    if res.is_ok() {
        asm!("dsb ishst", "tlbi vmalle1is", "dsb ish", "isb"); // taken from linux
    }
    res
}
