#![feature(lang_items)]
#![feature(asm)]
#![feature(panic_info_message)]
#![no_builtins]
#![no_std]

use crate::arch::aarch64::mmio::delay;
use crate::arch::aarch64::uart::RaspberryPiUART;
use crate::arch::aarch64::{framebuffer, mailbox, uart};
use qemu_exit::QEMUExit;
use spin::Mutex;

pub(crate) mod arch;
mod lang_items;

#[no_mangle]
pub unsafe extern "C" fn kmain() {
    uart::init_global_uart();
    println!("UART working");

    println!("Initializing framebuffer");
    framebuffer::init();
    println!("Drawing something");
    framebuffer::draw_example();
    println!("Draw ok");
    qemu_exit::AArch64::new().exit(0);
}
