#![feature(lang_items)]
#![feature(asm)]
#![feature(panic_info_message)]
#![no_builtins]
#![no_std]
#![allow(warnings)]

use crate::arch::aarch64::mmio::{delay_us, get_uptime_us};
use crate::arch::aarch64::{framebuffer, mailbox_methods, uart};
use qemu_exit::QEMUExit;

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
    println!("[INFO] UART working");

    let rate = mailbox_methods::get_clock_rate(0).unwrap();
    println!("[INFO] Root clock = {}Hz", rate);

    println!("[INFO] Initializing framebuffer");
    framebuffer::init();
    println!("[INFO] Drawing something");
    for i in 0..200 {
        vsync(|| {
            framebuffer::draw_example(i);
        });
    }
    println!("[INFO] Draw ok");

    qemu_exit::AArch64::new().exit(0);
}
