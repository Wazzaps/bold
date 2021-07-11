#![feature(lang_items)]
#![feature(asm)]
#![feature(panic_info_message)]
#![feature(default_alloc_error_handler)]
#![feature(async_closure)]
#![no_builtins]
#![no_std]
#![no_main]
#![feature(naked_functions)]

extern crate alloc;

use crate::arch::aarch64::mmio::{delay_us, get_uptime_us};
use crate::arch::aarch64::{mailbox_methods, mmu, phymem, virtmem};
use crate::driver_manager::DeviceType;
use alloc::boxed::Box;

pub(crate) mod arch;
pub(crate) mod console;
pub(crate) mod driver_manager;
mod file_interface;
pub(crate) mod framebuffer;
pub(crate) mod ktask;
mod lang_items;
pub(crate) mod utils;

use crate::console::{dump_hex, dump_hex_slice};
use crate::ktask::yield_now;
use crate::ErrWarn;
use core::ops::Deref;
use core::ptr::slice_from_raw_parts;
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
    } else {
        yield_now().await;
    }
}

#[inline(always)]
#[allow(dead_code)]
unsafe fn syscall1(num: usize, mut arg1: usize) -> usize {
    asm!(
        "svc #0",
        in("x8") num,
        inout("x0") arg1,
    );
    arg1
}

/// # Safety
///
/// This function assumes it runs only once, on a clean machine
#[no_mangle]
pub unsafe extern "C" fn kmain(dtb_addr: *const u8) {
    // Init mmu so devices work
    mmu::init().unwrap();

    // Early console
    driver_manager::init_driver_by_name(b"Raspberry Pi 3 UART1").warn();
    console::set_main_console_by_name(b"Raspberry Pi 3 UART1");
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

    println!("[INFO] Loaded drivers: {:?}", driver_manager::drivers());

    // Initialize main console, currently same as early-con
    println!("[INFO] Initializing main console");
    driver_manager::init_driver_by_name(b"Raspberry Pi 3 UART1").warn();
    console::set_main_console_by_name(b"Raspberry Pi 3 UART1");
    println!("[INFO] Main console working");

    // Get kernel command line
    let args = mailbox_methods::get_kernel_args().unwrap();
    println!("[INFO] Kernel command line: {:?}", args.deref());

    if !dtb_addr.is_null() {
        println!("[DBUG] DTB snippet:");
        let something = &*slice_from_raw_parts(dtb_addr, 128);
        dump_hex_slice(something);
    } else {
        println!("[DBUG] No DTB given");
    }

    driver_manager::init_all_drivers();

    // Start kernel tasks
    ktask::init();

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
        let framebuffer = driver_manager::device_by_type(DeviceType::Framebuffer)
            .unwrap()
            .ctrl
            .unwrap();
        let mut i = 0;
        loop {
            vsync(async || {
                framebuffer
                    .call(framebuffer::FramebufferCM::DrawExample { variant: i })
                    .await
                    .warn();
            })
            .await;
            i += 1;
        }
    });

    // Spawn some more tasks
    async fn example_task(id: usize) {
        let mut i = 0;
        loop {
            println!("[DBUG] Hello from task #{}: {}", id, i + 1);
            delay_us(1000000).await; // 1 second
            i += 1;
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

    // Call syscall (not yet implemented)
    // println!("result = 0x{:x}", syscall1(123, 321));

    let mut sdhc = arch::aarch64::sdhc::Sdhc::init().unwrap();
    let mut buf = [0; 512 / 4];
    sdhc.read_block(0, &mut buf).unwrap();

    println!("[INFO] EMMC: First block: ");
    dump_hex(&buf);

    // Modify first dword to demonstrate writing
    // buf[0] = 0xdeadbeef;
    // sdhc.write_block(0, &buf).unwrap();
    // sdhc.read_block(0, &mut buf).unwrap();
    // assert_eq!(buf[0], 0xdeadbeef);

    ktask::run();

    loop {
        asm!("wfe");
    }
    // qemu_exit::AArch64::new().exit(0);
}
