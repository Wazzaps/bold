#![feature(lang_items)]
#![feature(asm)]
#![feature(panic_info_message)]
#![no_builtins]
#![no_std]
#![allow(warnings)]

use crate::arch::aarch64::mmio::{delay_us, get_uptime_us};
use crate::arch::aarch64::{entropy, mailbox_methods, uart};
use crate::driver_manager::FramebufferDriver;
use crate::framebuffer::Framebuffer;
use qemu_exit::QEMUExit;
use spin::RwLock;

pub(crate) mod arch;
pub(crate) mod console;
pub(crate) mod driver_manager;
pub(crate) mod framebuffer;
mod lang_items;

fn vsync<F: FnMut()>(mut f: F) {
    let start = get_uptime_us();
    (f)();
    let end = get_uptime_us();
    if end < start + 16666 {
        delay_us(16666 - (end - start))
    }
}

#[no_mangle]
pub unsafe extern "C" fn kmain() {
    driver_manager::init_driver_by_name(b"QEMU-Only Raspberry Pi 3 UART0");
    console::set_main_console_by_name(b"QEMU-Only Raspberry Pi 3 UART0");
    println!("--- Bold Kernel v{} ---", env!("CARGO_PKG_VERSION"));
    println!("[INFO] Early console working");

    println!("[INFO] Loaded drivers: {:?}", driver_manager::drivers());

    println!("[INFO] Initializing main console");
    driver_manager::init_driver_by_name(b"Raspberry Pi 3 UART0");
    console::set_main_console_by_name(b"Raspberry Pi 3 UART0");
    println!("[INFO] Main console working");

    driver_manager::init_all_drivers();

    // Get root clock
    let rate = mailbox_methods::get_clock_rate(0).unwrap();
    println!("[INFO] Root clock = {}Hz", rate);

    // Generate a random number
    // entropy::init();
    // let lucky_number = entropy::get();
    // println!("[INFO] Today's lucky number: {}", lucky_number);

    // Draw something
    println!("[INFO] Drawing something");
    let mut framebuffer = driver_manager::driver_by_type::<FramebufferDriver>()
        .unwrap()
        .write();
    for i in 0..100 {
        vsync(|| {
            framebuffer.draw_example(i);
        });
    }
    println!("[INFO] Draw ok");

    qemu_exit::AArch64::new().exit(0);
}
