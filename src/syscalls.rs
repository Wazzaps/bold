use crate::arch::aarch64::exceptions::ExceptionContext;
use crate::arch::aarch64::mmu;
use crate::arch::aarch64::mmu::PageTable;
use crate::prelude::*;
use core::ops::Deref;
use core::pin::Pin;
use core::ptr::slice_from_raw_parts;
use num_enum::TryFromPrimitive;
use spin::Mutex;

#[repr(u64)]
#[derive(TryFromPrimitive)]
pub enum Syscall {
    Exit = 0,
    KLogWrite = 1,
    KLogWriteInt = 2,
    EnterUsermode = 0xffff,
}

unsafe fn cstr_ptr_to_asciistr(ptr: *const u8) -> AsciiStr<'static> {
    let mut length = 0;
    for i in 0.. {
        if *ptr.offset(i) == 0 {
            length = i;
            break;
        }
    }

    let slice = &*slice_from_raw_parts(ptr, length as usize);
    AsciiStr(slice)
}

pub unsafe fn handle_syscall(e: &mut ExceptionContext, syscall_no: Syscall) {
    match syscall_no {
        Syscall::Exit => {
            panic!("Init process tried to exit!")
        }
        Syscall::KLogWrite => {
            let format = cstr_ptr_to_asciistr(e.gpr[1] as *const u8);
            println!("[UM] {}", format);
        }
        Syscall::KLogWriteInt => {
            println!("[UM] 0x{:x}", e.gpr[1]);
        }
        Syscall::EnterUsermode => {
            e.elr_el1 = e.gpr[1];
            e.gpr[0] = 0;
            e.gpr[1] = 0;
            e.spsr_el1 = 0x340;
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ThreadState {
    /// General Purpose Registers.
    pub gpr: [u64; 30],

    /// The link register, aka x30.
    pub lr: u64,

    /// The program counter.
    pub pc: u64,

    /// The stack pointer.
    pub sp: u64,

    /// Saved program status.
    pub spsr: u64,
}

#[repr(align(4096))]
pub struct PageAligned<const LEN: usize>([u8; LEN]);

impl<const LEN: usize> Deref for PageAligned<LEN> {
    type Target = [u8; LEN];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Thread {
    pub state: ThreadState,
    pub page_tables: Pin<Box<PageTable>>,
    pub stack: Pin<Box<PageAligned<{ 4096 * 8 }>>>,
}

pub static MAIN_THREAD: Mutex<Option<Thread>> = Mutex::new(None);

#[inline(always)]
unsafe fn enter_usermode() -> ! {
    asm!(
        "mov x0, 0xffff",
        "mov x1, 0x20000",
        "mov sp, 0x20000",
        "svc #0",
        options(noreturn)
    )
}

pub async fn usermode() {
    // Prepare code
    const CODE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/example_app.bin"));
    const CODE_LENGTH: usize = CODE_BYTES.len();

    #[no_mangle]
    #[link_section = ".text"]
    static CODE: PageAligned<CODE_LENGTH> = PageAligned(*include_bytes!(concat!(
        env!("OUT_DIR"),
        "/example_app.bin"
    )));

    let code: extern "C" fn() = unsafe { core::mem::transmute(&*CODE as *const _) };

    // Prepare stack
    let stack = Box::pin(PageAligned([0u8; 4096 * 8]));

    unsafe {
        // Prepare page tables
        let mut page_tables = Box::pin(PageTable::new());
        set_msr!(ttbr0_el1, (page_tables.0.as_ptr() as u64) & 0x7FFFFFFFFF);
        const PAGE_FLAGS: u64 = mmu::PT_USER | // non-privileged
            mmu::PT_ISH | // inner shareable
            mmu::PT_MEM; // normal memory
        mmu::vmap_to(
            &mut page_tables,
            0x10000,
            PhyAddr(stack.0.as_ptr() as usize & 0x7FFFFFFFFF),
            PAGE_FLAGS | mmu::PT_RW | mmu::PT_NX | mmu::PT_USER,
        )
        .unwrap();
        mmu::vmap_to(
            &mut page_tables,
            0x20000,
            PhyAddr(code as usize & 0x7FFFFFFFFF),
            PAGE_FLAGS | mmu::PT_RO | mmu::PT_USER,
        )
        .unwrap();

        // Prepare thread state
        {
            *MAIN_THREAD.lock() = Some(Thread {
                state: ThreadState {
                    gpr: [
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0,
                    ],
                    lr: 0,
                    pc: 0x20000,
                    sp: 0x20000,
                    spsr: 0x340,
                },
                page_tables,
                stack,
            });
        };
        enter_usermode();
    }
}
