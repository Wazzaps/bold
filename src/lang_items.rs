use crate::prelude::*;
use core::fmt;

use core::fmt::Write;
use core::panic::PanicInfo;

#[lang = "eh_personality"]
pub extern "C" fn eh_personality() {}

struct FmtWriteAdapter<'a, const CAP: usize>(&'a mut ArrayVec<u8, CAP>, usize);

impl<const CAP: usize> fmt::Write for FmtWriteAdapter<'_, CAP> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let formatted = s.as_bytes();
        let copied_len = formatted.len().min(CAP - self.1);
        let _ = self.0.try_extend_from_slice(&formatted[..copied_len]);
        self.1 += copied_len;

        Ok(())
    }
}

#[panic_handler]
unsafe fn panic_handler(info: &PanicInfo) -> ! {
    println!("+++ Bold Kernel v{} Panic! +++", env!("CARGO_PKG_VERSION"));
    if let Some(message) = info.message() {
        println!("{}", message);
    }
    if let Some(location) = info.location() {
        println!("at {}", location);
    }
    println!("--- Stack:");
    let sp: u64;
    let lr: u64;
    asm!(
        "mov {:x}, sp",
        out(reg) sp,
        options(nomem, nostack)
    );
    asm!(
        "mov {:x}, lr",
        out(reg) lr,
        options(nomem, nostack)
    );
    println!("- LR: 0x{:x} ", lr);
    for i in 0..128 {
        let val = *(sp as *const u64).offset(i);
        if (0xffffff8000080000..=0xffffff8000080000 + 0x03f400).contains(&val) {
            println!("- {}: 0x{:x} ", i, val);
        }
    }
    println!("--- Bold Kernel v{} Panic! ---", env!("CARGO_PKG_VERSION"));

    let mut message_buf = ArrayVec::<u8, 512>::new();
    let _ = write!(FmtWriteAdapter(&mut message_buf, 0), "KERNEL PANIC: ");
    if let Some(message) = info.message() {
        let _ = writeln!(FmtWriteAdapter(&mut message_buf, 0), "{}", message);
    }
    if let Some(location) = info.location() {
        let _ = write!(FmtWriteAdapter(&mut message_buf, 0), "at {}", location);
    }
    crate::arch::aarch64::framebuffer::panic(message_buf.as_slice());

    loop {
        asm!("wfi");
    }
    // qemu_exit::AArch64::new().exit(1)
    // poweroff(false);
}

#[alloc_error_handler]
pub fn oom(info: core::alloc::Layout) -> ! {
    panic!(
        "memory allocation of {} bytes failed [align={}]",
        info.size(),
        info.align()
    )
}
