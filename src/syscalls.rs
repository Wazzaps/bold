use crate::arch::aarch64::exceptions::ExceptionContext;
use crate::arch::aarch64::mmio::get_uptime_us;
use crate::arch::aarch64::mmu;
use crate::arch::aarch64::mmu::PageTable;
use crate::ktask::thread_waker;
use crate::prelude::*;
use crate::threads::{current_core, Thread};
use crate::{sleep_queue, threads};
use core::ops::{Deref, DerefMut};
use core::ptr::slice_from_raw_parts;
use num_enum::TryFromPrimitive;

#[repr(u64)]
#[derive(TryFromPrimitive)]
pub enum Syscall {
    Exit = 0,
    KLogWrite = 1,
    KLogWriteInt = 2,
    USleep = 3,
    GetTid = 4,
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
            let current_core = current_core();
            let executor = &threads::EXECUTORS.get().unwrap()[current_core];
            let current_thread = executor.current_thread().unwrap();
            current_thread.read().kill();
            executor.switch(e);
        }
        Syscall::KLogWrite => {
            let format = cstr_ptr_to_asciistr(e.gpr[0] as *const u8);
            println!("[UM] {}", format);
        }
        Syscall::KLogWriteInt => {
            println!("[UM] 0x{:x}", e.gpr[0]);
        }
        Syscall::USleep => {
            let sleep_time = e.gpr[0];
            let wake_time = get_uptime_us() + sleep_time.max(threads::THREAD_TIMEOUT_US as u64);

            let current_core = current_core();
            let executor = &threads::EXECUTORS.get().unwrap()[current_core];
            let last_tid = executor.switch(e);
            if last_tid != 0 {
                sleep_queue::push(wake_time, thread_waker(last_tid));
            }
        }
        Syscall::GetTid => {
            let current_core = current_core();
            let executor = &threads::EXECUTORS.get().unwrap()[current_core];
            e.gpr[0] = executor.current_thread().unwrap().read().id() as u64;
        }
    }
}

#[repr(align(4096))]
pub struct PageAligned<const LEN: usize>(pub [u8; LEN]);

impl<const LEN: usize> Deref for PageAligned<LEN> {
    type Target = [u8; LEN];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const LEN: usize> DerefMut for PageAligned<LEN> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
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

        let thread = Thread::new(
            b"Usermode Runner",
            ExceptionContext {
                gpr: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0,
                ],
                lr: 0,
                pc: 0x20000,
                sp: 0x20000,
                spsr: 0x300,
            },
            Some(page_tables),
        );
        threads::EXECUTORS.get().unwrap()[0].spawn(thread);
    }
}
