extern crate circle_sys;

use self::circle_sys::CircleShim;
use crate::arch::aarch64::interrupts;
use crate::arch::aarch64::mmio::sleep_us;
use crate::arch::aarch64::uart1::write_uart1;
use crate::ipc;
use crate::prelude::*;
use crate::syscalls::PageAligned;
use core::ptr::slice_from_raw_parts;
use spin::Mutex;

#[link_section = ".dma.bss"]
#[used]
static mut CIRCLE_SHIM_DMA_AREA: PageAligned<{ 4096 * 512 }> = PageAligned([0; 4096 * 512]);

static KEYS_QUEUE: Mutex<Option<ipc::IpcRef>> = Mutex::new(None);

unsafe extern "C" fn log_write(buf: *const u8, len: usize) -> u32 {
    print!("[INFO] CircleShim: ");
    write_uart1(&*slice_from_raw_parts(buf, len)).warn();
    len as u32
}

pub async unsafe fn init(keys_queue: ipc::IpcRef) {
    *KEYS_QUEUE.lock() = Some(keys_queue);
    let mut shim = CircleShim::new(log_write);
    loop {
        shim.test_keyboard();
        sleep_us(1000 * 100).await;
    }
}

#[no_mangle]
unsafe extern "C" fn circle_shim_key_press_handler(chr: u8) {
    if let Some(queue) = KEYS_QUEUE.lock().as_mut() {
        queue.queue_write(&[chr]).warn();
    }
}

#[no_mangle]
unsafe extern "C" fn MemoryAllocate(len: usize) -> *mut u8 {
    // println!("Alloc of {} bytes", len);
    let res = alloc::alloc::alloc(
        alloc::alloc::Layout::from_size_align(len, 32).expect("USB: Invalid allocation request"),
    );
    // println!(" --> {:p}", res);
    res
}

#[no_mangle]
unsafe extern "C" fn MemoryDeallocate(addr: *mut u8) {
    // println!("Dealloc of {:p}", addr);
    alloc::alloc::dealloc(addr, alloc::alloc::Layout::from_size_align_unchecked(1, 1))
}

#[no_mangle]
unsafe extern "C" fn GetMemCoherentRegion() -> *mut u8 {
    CIRCLE_SHIM_DMA_AREA.as_mut_ptr()
}

#[no_mangle]
unsafe extern "C" fn IrqConnect(irq: u32, handler: interrupts::IrqHandlerFunc, param: usize) {
    println!(
        "[DBUG] CircleShim: IrqConnect({}, {:p}, 0x{:x})",
        irq, handler, param
    );
    interrupts::attach_irq_handler(irq as usize, handler, param);
}

#[no_mangle]
unsafe extern "C" fn IrqDisconnect(irq: u32) {
    println!("[DBUG] CircleShim: IrqDisconnect({})", irq);
    interrupts::detach_irq_handler(irq as usize);
}
