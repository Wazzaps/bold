#![feature(lang_items)]
#![feature(asm)]
#![feature(panic_info_message)]
#![feature(default_alloc_error_handler)]
#![feature(async_closure)]
#![no_builtins]
#![no_std]
#![no_main]
#![feature(naked_functions)]
#![allow(dead_code)]

extern crate alloc;

use crate::arch::aarch64::mmio::{delay_us, delay_us_sync};
use crate::arch::aarch64::{mailbox_methods, mmu, phymem, virtmem};
use alloc::boxed::Box;

pub(crate) mod arch;
pub(crate) mod console;
pub(crate) mod driver_manager;
mod file_interface;
pub(crate) mod fonts;
pub(crate) mod framebuffer;
pub(crate) mod framebuffer_console;
pub(crate) mod ipc;
mod kshell;
pub(crate) mod ktask;
mod lang_items;
pub(crate) mod utils;

use crate::arch::aarch64::phymem::PhyAddr;
use crate::arch::aarch64::uart1::init_uart1;
use crate::console::{dump_hex, dump_hex_slice};
use crate::ErrWarn;
use core::ops::Deref;
use core::ptr::slice_from_raw_parts;
pub(crate) use file_interface as fi;
pub(crate) use utils::*;

#[inline(always)]
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
/// It also assumes dtb_addr is either a valid dtb or null
#[no_mangle]
pub unsafe extern "C" fn kmain(dtb_addr: *const u8) -> ! {
    init_uart1();
    // Init mmu so devices work
    mmu::init().unwrap();

    // Earlycon
    console::set_main_console_by_name(b"Raspberry Pi 3 UART1");
    println!("[INFO] Early console working");

    // let mac = mailbox_methods::get_nic_mac().unwrap();
    // println!(
    //     "[INFO] MAC Address = {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
    //     mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    // );

    // Physical Memory allocator
    {
        let mut phymem = phymem::PHYMEM_FREE_LIST.lock();
        phymem.init();
    }

    driver_manager::early_init_all_drivers();

    // Create new stack for interrupts
    let core0_stack = {
        let mut phymem = phymem::PHYMEM_FREE_LIST.lock();
        phymem
            .alloc_pages(64) // 256KiB
            .expect("Failed to allocate core 0 stack")
    };

    println!("[DBUG] core 0 stack at: {:?}", core0_stack);
    asm!(
        "mov sp, {:x}",
        "mov x0, {:x}",
        "b kmain_on_stack",
        in(reg) core0_stack.base.0 + core0_stack.len,
        in(reg) dtb_addr,
        options(noreturn)
    )
}

#[no_mangle]
unsafe extern "C" fn kmain_on_stack(dtb_addr: *const u8) -> ! {
    // Virtual Memory allocator
    {
        let mut phymem = phymem::PHYMEM_FREE_LIST.lock();
        let kernel_virtmem = phymem
            .alloc_pages(16384) // 64MiB
            .expect("Failed to allocate dynamic kernel memory");
        virtmem::init(kernel_virtmem);
    }

    if !dtb_addr.is_null() {
        println!("[DBUG] DTB @ {:p} snippet:", dtb_addr);
        let something = &*slice_from_raw_parts(dtb_addr, 128);
        dump_hex_slice(something);
        println!("[DBUG] Parsed:");
        arch::aarch64::dtb::parse(dtb_addr);
    } else {
        println!("[DBUG] No DTB given");
    }

    // IPC
    ipc::init();

    // Start kernel tasks
    ktask::init();

    // Early console
    driver_manager::init_driver_by_name(b"Raspberry Pi 3 UART1").warn();
    println!("--- Bold Kernel v{} ---", env!("CARGO_PKG_VERSION"));

    // Should be skipped by exception handler
    *(0x99999999 as *mut u8) = 0xaa;

    // Test timer (interrupts not working yet)
    set_msr!(CNTP_TVAL_EL0, 10000);
    set_msr!(CNTP_CTL_EL0, 3);
    for _ in 0..10 {
        println!("{} {:x}", get_msr!(CNTP_CTL_EL0), get_msr!(CNTP_TVAL_EL0));
        delay_us_sync(1);
    }

    // // IPC test
    // // ktask::SimpleExecutor::run_blocking(ktask::Task::new_raw(b"IPC Test", Box::pin(ipc::test())));
    //
    // println!("[DBUG] virt2phy tests:");
    // for addr in [
    //     0,
    //     0x1000,
    //     0x1234,
    //     0x80000,
    //     0x1f4 << 21,
    //     (0x1f4 << 21) + 0x12345,
    //     0x1f5 << 21,
    //     0x1f6 << 21,
    //     0x1f7 << 21,
    //     0x1f7 << 21,
    //     0x200 << 21,
    // ] {
    //     println!("V2P(0x{:x}) -> {:?}", addr, mmu::virt2phy(addr));
    // }
    //
    // // Map some page
    // const PAGE_FLAGS: u64 = mmu::PT_USER | // non-privileged
    //     mmu::PT_ISH | // inner shareable
    //     mmu::PT_MEM; // normal memory;
    // mmu::vmap(0x40000000, PhyAddr(0x80000), PAGE_FLAGS).unwrap();
    // println!(
    //     "[DBUG] Accessing kernel code at {:?} via mapping at 0x{:x}",
    //     PhyAddr(0x80000),
    //     0x40000000
    // );
    // dump_hex_slice(&*slice_from_raw_parts(0x40000000 as *const u8, 64));
    // mmu::vunmap(0x40000000).unwrap();
    // // The following line will crash as expected:
    // // dump_hex_slice(&*slice_from_raw_parts(0x40000000 as *const u8, 64));
    //
    // // Test it
    // {
    //     let heap_val = Box::new(123);
    //     let heap_val2 = Box::new(321);
    //     println!("[DBUG] Boxed val: {} (at &{:p})", heap_val, &heap_val);
    //     println!("[DBUG] Boxed val2: {} (at &{:p})", heap_val2, &heap_val2);
    // }

    println!("[INFO] Loaded drivers: {:?}", driver_manager::drivers());

    // Initialize main console, currently same as early-con
    // println!("[INFO] Initializing main console");
    // driver_manager::init_driver_by_name(b"Raspberry Pi 3 UART1").warn();
    // console::set_main_console_by_name(b"Raspberry Pi 3 UART1");
    // println!("[INFO] Main console working");

    // Get kernel command line
    // let args = mailbox_methods::get_kernel_args().unwrap();
    // println!("[INFO] Kernel command line: {:?}", args.deref());

    driver_manager::init_all_drivers();

    // Get root clock
    // let rate = mailbox_methods::get_clock_rate(0).unwrap();
    // println!("[INFO] Root clock = {}", rate);

    // Generate a random number
    // entropy::init();
    // let lucky_number = entropy::get();
    // println!("[INFO] Today's lucky number: {}", lucky_number);

    // Draw something
    framebuffer_console::init();

    // Spawn some more tasks
    // async fn example_task(id: usize) {
    //     let mut i = 0;
    //     loop {
    //         println!("[DBUG] Hello from task #{}: {}", id, i + 1);
    //         delay_us(1000000).await; // 1 second
    //         i += 1;
    //     }
    // }
    // spawn_task!({ example_task(1).await });
    // spawn_task!({ example_task(2).await });
    // spawn_task!({ example_task(3).await });

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

    // let mut sdhc = arch::aarch64::sdhc::Sdhc::init().unwrap();
    // let mut buf = [0; 512 / 4];
    // sdhc.read_block(0, &mut buf).unwrap();
    //
    // println!("[INFO] EMMC: First block: ");
    // dump_hex(&buf);

    // Modify first dword to demonstrate writing
    // buf[0] = 0xdeadbeef;
    // sdhc.write_block(0, &buf).unwrap();
    // sdhc.read_block(0, &mut buf).unwrap();
    // assert_eq!(buf[0], 0xdeadbeef);

    spawn_task!(b"KShell.launcher", {
        let root = ipc::ROOT.read().as_ref().unwrap().clone();

        async fn navigate(mut root: ipc::IpcRef, path: &[u64]) -> ipc::IpcRef {
            for p in path {
                root = root.dir_get(*p).await.unwrap();
            }
            root
        }

        // TODO: Non-functional until usb
        // let fb_shell_in = navigate(
        //     root.clone(),
        //     &[
        //         ipc::well_known::ROOT_DEVICES,
        //         ipc::well_known::DEVICES_RPI_FB_CON,
        //         ipc::well_known::RPI_FB_CON0,
        //         ipc::well_known::RPI_FB_CON_IN,
        //     ],
        // )
        // .await;

        let fb_shell_out = navigate(
            root.clone(),
            &[
                ipc::well_known::ROOT_DEVICES,
                ipc::well_known::DEVICES_RPI_FB_CON,
                ipc::well_known::RPI_FB_CON0,
                ipc::well_known::RPI_FB_CON_OUT,
            ],
        )
        .await;

        let uart_shell_in = navigate(
            root.clone(),
            &[
                ipc::well_known::ROOT_DEVICES,
                ipc::well_known::DEVICES_RPI_UART,
                ipc::well_known::RPI_UART1,
                ipc::well_known::RPI_UART_IN,
            ],
        )
        .await;

        let uart_shell_out = navigate(
            root.clone(),
            &[
                ipc::well_known::ROOT_DEVICES,
                ipc::well_known::DEVICES_RPI_UART,
                ipc::well_known::RPI_UART1,
                ipc::well_known::RPI_UART_OUT,
            ],
        )
        .await;

        let (input1, input2) = ipc::spsc_mux::mux(uart_shell_in);

        kshell::launch(input1, fb_shell_out, false);
        kshell::launch(input2, uart_shell_out, true);
    });

    ktask::run();

    println!("[Done]");
    loop {
        asm!("wfe");
    }
    // qemu_exit::AArch64::new().exit(0);
}
