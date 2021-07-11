use crate::arch::aarch64::mmio;
use crate::arch::aarch64::phymem::{PhyAddr, PhySlice};
use crate::{get_msr, println, set_msr};
use alloc::boxed::Box;
use core::mem::size_of;

const PAGE_SIZE: u64 = 4096;

// Granularity
const PT_PAGE: u64 = 0b11;
const PT_BLOCK: u64 = 0b01;

// Accessibility
const PT_KERNEL: u64 = 0 << 6;
const PT_USER: u64 = 1 << 6;
const PT_RW: u64 = 0 << 7;
const PT_RO: u64 = 1 << 7;
const PT_AF: u64 = 1 << 10;
const PT_NX: u64 = 1 << 54;

// Share-ability
const PT_OSH: u64 = 0b10 << 8;
const PT_ISH: u64 = 0b11 << 8;

// defined in MAIR register
const PT_MEM: u64 = 0 << 2;
const PT_DEV: u64 = 1 << 2;
#[allow(dead_code)]
const PT_NC: u64 = 2 << 2;

const TTBR_CNP: u64 = 1;

static mut PAGING: PageTables = unsafe { PageTables::new() };

#[repr(C, align(4096))]
struct PageTable(pub [u64; 512]);

impl PageTable {
    pub const unsafe fn new() -> Self {
        Self([0; 512])
    }

    pub unsafe fn use_child<F: FnOnce(Option<(usize, &PageTable)>)>(&self, idx: usize, f: F) {
        let paddr = self.0[idx] & 0x7FFFFFF000;
        let raw = self.0[idx];
        if paddr != 0 {
            // TODO: Assuming ident map
            let vaddr = paddr as usize;
            (f)((vaddr as *const PageTable)
                .as_ref()
                .map(|v| (raw as usize, v)));
        } else {
            (f)(None);
        }
    }

    pub unsafe fn use_child_mut<F: FnOnce(Option<(usize, &mut PageTable)>)>(
        &mut self,
        idx: usize,
        f: F,
    ) {
        let paddr = self.0[idx] & 0x7FFFFFF000;
        let raw = self.0[idx];
        if paddr != 0 {
            // TODO: Assuming ident map
            let vaddr = paddr as usize;
            (f)((vaddr as *mut PageTable)
                .as_mut()
                .map(|v| (raw as usize, v)));
        } else {
            (f)(None);
        }
    }
}

impl Drop for PageTable {
    fn drop(&mut self) {
        unsafe {
            let self_addr = self as *const Self as usize;
            let paging_addr = &PAGING as *const PageTables as usize;
            let paging_end = paging_addr + size_of::<PageTable>();
            if self_addr < paging_addr || self_addr >= paging_end {
                drop(Box::from_raw(self));
            }
        }
    }
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

pub unsafe fn init() -> Result<(), ()> {
    println!("[DBUG] CurrentEL: {}", (get_msr!(CurrentEL) >> 2) & 0b11);

    // Identity map user area, L1 Table
    PAGING.user_l1.0[0] = {
        (PAGING.user_l2.0.as_ptr() as u64) | // Physical address
            PT_PAGE |     // it has the "Present" flag, which must be set, and we have area in it mapped by pages
            PT_AF |       // accessed flag. Without this we're going to have a Data Abort exception
            PT_USER |     // non-privileged
            PT_ISH |      // inner shareable
            PT_MEM // normal memory
    };

    // Identity map user area, L2 Table, first block
    PAGING.user_l2.0[0] = {
        (PAGING.user_l3.0.as_ptr() as u64) | // Physical address
            PT_PAGE |     // it has the "Present" flag, which must be set, and we have area in it mapped by pages
            PT_AF |       // accessed flag. Without this we're going to have a Data Abort exception
            PT_USER |     // non-privileged
            PT_ISH |      // inner shareable
            PT_MEM // normal memory
    };

    // Identity map user area, L2 Table
    let iomem_cutoff = (mmio::MMIO_BASE >> 21) as usize;
    let data_cutoff = ((&__data_start) as *const u8 as u64 / PAGE_SIZE) as usize;
    let dma_start = ((&__dma_start) as *const u8 as u64 / PAGE_SIZE) as usize;
    let dma_end = ((&__dma_end) as *const u8 as u64 / PAGE_SIZE) as usize;
    println!("dma_start = 0x{:x}, dma_end = 0x{:x}", dma_start, dma_end);
    for (i, tbl) in PAGING.user_l2.0.iter_mut().enumerate().skip(1) {
        *tbl = {
            (i << 21) as u64 | // Physical address
                PT_BLOCK |    // map 2M block
                PT_AF |       // accessed flag
                PT_NX |       // no execute
                PT_USER |     // non-privileged
                // different attributes for device memory
                if i >= iomem_cutoff {
                    println!("Defining 0x{:x} as DMA memory", i << 21);
                    PT_OSH | PT_DEV
                } else {
                    PT_ISH | PT_MEM
                }
        };
    }
    println!("Defining 0x{:x} as DMA memory [fb]", 0x1f4 << 21);
    PAGING.user_l2.0[500] = {
        (0x1f4u64 << 21) as u64 | // Physical address
            PT_BLOCK |    // map 2M block
            PT_AF |       // accessed flag
            PT_NX |       // no execute
            PT_USER |     // non-privileged
            PT_OSH | PT_DEV
    };
    // println!("->{:x}", PAGING.user_l2_high[500]);
    println!("Defining 0x{:x} as DMA memory [fb]", 0x1f5 << 21);
    PAGING.user_l2.0[501] = {
        (0x1f5u64 << 21) as u64 | // Physical address
            PT_BLOCK |    // map 2M block
            PT_AF |       // accessed flag
            PT_NX |       // no execute
            PT_USER |     // non-privileged
            PT_OSH | PT_DEV
    };

    // User L3 table
    for (i, tbl) in PAGING.user_l3.0.iter_mut().enumerate() {
        *tbl = {
            (i as u64 * PAGE_SIZE) | // Physical address
                PT_PAGE |     // map 4k
                PT_AF |       // accessed flag
                PT_USER |     // non-privileged
                if i >= dma_start && i < dma_end {
                    println!("Defining 0x{:x} as DMA memory", i as u64 * PAGE_SIZE);
                    PT_OSH | PT_DEV | PT_RW | PT_NX
                } else if i < 0x80 || i >= data_cutoff {
                    PT_MEM | PT_ISH | PT_RW | PT_NX
                } else {
                    PT_MEM | PT_ISH | PT_RO
                }
        };
    }

    // Map kernel area, L1 Table
    // PAGING.kernel_l1[511] = {
    //     (PAGING.kernel_l2.as_ptr() as u64) | // Physical address
    //         PT_PAGE |     // it has the "Present" flag, which must be set, and we have area in it mapped by pages
    //         PT_AF |       // accessed flag. Without this we're going to have a Data Abort exception
    //         PT_KERNEL |     // privileged
    //         PT_ISH |      // inner shareable
    //         PT_MEM // normal memory
    // };
    //
    // // Map kernel area, L2 Table
    // PAGING.kernel_l2[511] = {
    //     (PAGING.kernel_l3.as_ptr() as u64) | // Physical address
    //         PT_PAGE |     // it has the "Present" flag, which must be set, and we have area in it mapped by pages
    //         PT_AF |       // accessed flag. Without this we're going to have a Data Abort exception
    //         PT_KERNEL |     // privileged
    //         PT_ISH |      // inner shareable
    //         PT_MEM // normal memory
    // };
    //
    // // Map kernel area, L3 Table
    // for (i, tbl) in PAGING.kernel_l3.iter_mut().enumerate() {
    //     *tbl = {
    //         (i as u64 * PAGE_SIZE) | // Physical address
    //             PT_PAGE |     // it has the "Present" flag, which must be set, and we have area in it mapped by pages
    //             PT_AF |       // accessed flag. Without this we're going to have a Data Abort exception
    //             PT_NX |
    //             PT_KERNEL |     // privileged
    //             PT_OSH |
    //             PT_DEV
    //     }
    // }

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
    // - lower half, user space
    set_msr!(ttbr0_el1, PAGING.user_l1.0.as_ptr() as u64 + TTBR_CNP);
    // - upper half, kernel space
    // set_msr!(ttbr1_el1, PAGING.kernel_l1.as_ptr() as u64 + TTBR_CNP);

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

    println!("[DBUG] MMU Initialized");
    Ok(())
}

#[allow(dead_code)]
pub unsafe fn virt2phy(vaddr: usize) -> Option<PhyAddr> {
    // TODO: Only support ttbr0 for now
    if vaddr % (PAGE_SIZE as usize) != 0 {
        return None;
    }
    // println!("V2P: looking for 0x{:x}", vaddr);
    let mut res = None;
    let lvl1 = &mut PAGING.user_l1;
    lvl1.use_child(vaddr >> 30, |lvl2| {
        if let Some((raw1, lvl2)) = lvl2 {
            if (raw1 as u64) & PT_PAGE != PT_PAGE {
                // Huge page
                res = Some(raw1 & 0x7FFFFFF000);
            } else {
                lvl2.use_child((vaddr >> 21) % 512, |lvl3| {
                    if let Some((raw2, lvl3)) = lvl3 {
                        if (raw2 as u64) & PT_PAGE != PT_PAGE {
                            // Huge page
                            res = Some(raw2 & 0x7FFFFFF000);
                        } else {
                            res = Some((lvl3.0[(vaddr >> 12) % 512] & 0x7FFFFFF000) as usize);
                        }
                    } else {
                        // println!("V2P: Not found in L2");
                    }
                });
            }
        } else {
            // println!("V2P: Not found in L1");
        }
    });

    res.map(|r| PhyAddr(r))
}
