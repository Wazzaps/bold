use crate::println;
use arrayvec::ArrayVec;
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
