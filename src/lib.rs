#![feature(lang_items)]
#![feature(asm)]
#![feature(panic_info_message)]
#![no_builtins]
#![no_std]

use crate::arch::aarch64::mmio::{delay, delay_us, get_uptime_us};
use crate::arch::aarch64::uart::RaspberryPiUART;
use crate::arch::aarch64::{framebuffer, mailbox, uart};
use qemu_exit::QEMUExit;
use spin::Mutex;
use crate::arch::aarch64::mailbox::get_stc;

pub(crate) mod arch;
mod lang_items;

fn vsync<F: Fn()>(f: F) {
    let start = get_uptime_us();
    (f)();
    let end = get_uptime_us();
    if end < start + 16666 {
        delay_us(16666 - (end - start))
    }
}

#[no_mangle]
pub unsafe extern "C" fn kmain() {
    uart::init_global_uart();
    println!("UART working");

    println!("Initializing framebuffer");
    framebuffer::init();
    println!("Drawing something");
    for i in 0..200 {
        vsync(|| {
            framebuffer::draw_example(i);
        });
    }
    println!("Draw ok");

    qemu_exit::AArch64::new().exit(0);
}
