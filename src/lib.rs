#![feature(lang_items)]
#![feature(asm)]
#![feature(panic_info_message)]
#![feature(default_alloc_error_handler)]
#![no_builtins]
#![no_std]
#![allow(warnings)]
#![warn(unused_imports)]
#![warn(unused_import_braces)]

use crate::arch::aarch64::mmio::{delay_us, get_uptime_us};
use crate::arch::aarch64::{mailbox_methods, phymem, virtmem};
use qemu_exit::QEMUExit;
use alloc::boxed::Box;
use crate::driver_manager::DeviceType;

pub(crate) mod arch;
pub(crate) mod console;
pub(crate) mod driver_manager;
mod file_interface;
pub(crate) mod framebuffer;
mod lang_items;
mod utils;

pub(crate) use file_interface as fi;
pub(crate) use utils::*;

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
    // Early console
    driver_manager::init_driver_by_name(b"QEMU-Only Raspberry Pi 3 UART0");
    console::set_main_console_by_name(b"QEMU-Only Raspberry Pi 3 UART0");
    println!("--- Bold Kernel v{} ---", env!("CARGO_PKG_VERSION"));
    println!("[INFO] Early console working");

    // Memory allocator
    phymem::PHYMEM_FREE_LIST.lock().init();
    let kernel_virtmem = phymem::PHYMEM_FREE_LIST
        .lock()
        .alloc_pages(16384)
        .expect("Failed to allocate dynamic kernel memory"); // 64MiB
    virtmem::init(kernel_virtmem);

    // Test it
    {
        let heap_val = Box::new(123);
        let heap_val2 = Box::new(321);
        println!("Boxed val: {} (at &{:p})", heap_val, &heap_val);
        println!("Boxed val2: {} (at &{:p})", heap_val2, &heap_val2);
    }

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
    let mut framebuffer = driver_manager::device_by_type(DeviceType::Framebuffer)
        .unwrap()
        .ctrl
        .unwrap();
    for i in 0..1000000 {
        vsync(|| {
            framebuffer.call(framebuffer::FramebufferCM::DrawExample { variant: i });
        });
    }
    println!("[INFO] Draw ok");

    qemu_exit::AArch64::new().exit(0);
}
