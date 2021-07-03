#![feature(lang_items)]
#![feature(asm)]
#![feature(panic_info_message)]
#![feature(default_alloc_error_handler)]
#![feature(async_closure)]
#![no_builtins]
#![no_std]
#![allow(warnings)]
#![warn(unused_imports)]
#![warn(unused_import_braces)]

extern crate alloc;

use crate::arch::aarch64::mmio::{delay_us, get_uptime_us};
use crate::arch::aarch64::{mailbox_methods, phymem, virtmem};
use crate::driver_manager::DeviceType;
use alloc::boxed::Box;
use qemu_exit::QEMUExit;

pub(crate) mod arch;
pub(crate) mod console;
pub(crate) mod driver_manager;
mod file_interface;
pub(crate) mod framebuffer;
pub(crate) mod ktask;
mod lang_items;
pub(crate) mod utils;

pub(crate) use file_interface as fi;
pub(crate) use utils::*;

async fn vsync<F, Fut>(mut f: F)
where
    F: FnMut() -> Fut,
    Fut: core::future::Future<Output = ()>,
{
    let start = get_uptime_us();
    (f)().await;
    let end = get_uptime_us();
    if end < start + 16666 {
        delay_us(16666 - (end - start)).await;
    }
}

#[no_mangle]
pub unsafe extern "C" fn kmain() {
    // Early console
    driver_manager::init_driver_by_name(b"QEMU-Only Raspberry Pi 3 UART0");
    console::set_main_console_by_name(b"QEMU-Only Raspberry Pi 3 UART0");
    println!("--- Bold Kernel v{} ---", env!("CARGO_PKG_VERSION"));
    println!("[INFO] Early console working");

    // Try input
    // let con = driver_manager::device_by_type(DeviceType::Console).unwrap();
    // let mut buf = [0; 4];
    // con.read.unwrap()
    //     .read_exact(&mut buf).unwrap();
    // println!("{:?}", buf);

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
        println!("[DBUG] Boxed val: {} (at &{:p})", heap_val, &heap_val);
        println!("[DBUG] Boxed val2: {} (at &{:p})", heap_val2, &heap_val2);
    }

    // Start kernel tasks
    ktask::init();

    println!("[INFO] Loaded drivers: {:?}", driver_manager::drivers());

    println!("[INFO] Initializing main console");
    driver_manager::init_driver_by_name(b"Raspberry Pi 3 UART0");
    console::set_main_console_by_name(b"Raspberry Pi 3 UART0");
    println!("[INFO] Main console working");

    driver_manager::init_all_drivers();

    // Get root clock
    let rate = mailbox_methods::get_clock_rate(0).unwrap();
    println!("[INFO] Root clock = {}", rate);

    // Generate a random number
    // entropy::init();
    // let lucky_number = entropy::get();
    // println!("[INFO] Today's lucky number: {}", lucky_number);

    // Draw something
    spawn_task!({
        println!("[INFO] Drawing something");
        let mut framebuffer = driver_manager::device_by_type(DeviceType::Framebuffer)
            .unwrap()
            .ctrl
            .unwrap();
        for i in 0..180 {
            vsync(async || {
                framebuffer
                    .call(framebuffer::FramebufferCM::DrawExample { variant: i })
                    .await;
            })
            .await;
        }
    });

    // Spawn some more tasks
    async fn example_task(id: usize) {
        for i in 0..3 {
            println!("[DBUG] Hello from task #{}: {}/3", id, i + 1);
            delay_us(1000000).await; // 1 second
        }
    }
    spawn_task!({ example_task(1).await });
    spawn_task!({ example_task(2).await });
    spawn_task!({ example_task(3).await });

    // Spawn echo task (WIP: Not working yet)
    // spawn_task!({
    //     loop {
    //         if let Some(console) = console::MAIN_CONSOLE.read().as_deref() {
    //             let buf = [0; 1];
    //             console.read.unwrap().read_exact(&mut buf).await;
    //             console.write.unwrap().write_all(&mut buf).await;
    //         }
    //     }
    // });

    ktask::run();

    qemu_exit::AArch64::new().exit(0);
}
