use crate::arch::aarch64::mmio::{
    delay_us_sync, mmio_read, mmio_write, ENABLE_IRQS_1, ENABLE_IRQS_2, IRQ_PENDING_1,
    SYSTEM_TIMER_IRQ_1, TIMER_C1, TIMER_CLO, TIMER_CS, TIMER_CS_M1, UART_IRQ,
};
use crate::print;
use crate::set_msr_const;
use core::sync::atomic::{AtomicU32, Ordering};

static NEXT_WAKEUP: AtomicU32 = AtomicU32::new(0);
const INTERVAL: u32 = 2000000;

pub unsafe fn enable() {
    set_msr_const!(daifclr, 2);
}

pub unsafe fn disable() {
    set_msr_const!(daifset, 2);
}

pub unsafe fn init() {
    // Interrupt enable
    mmio_write(ENABLE_IRQS_1, SYSTEM_TIMER_IRQ_1);
    mmio_write(ENABLE_IRQS_2, UART_IRQ);
    enable();

    // Timer init
    let timer_now = mmio_read(TIMER_CLO);
    let next_wakeup = timer_now + INTERVAL;
    NEXT_WAKEUP.store(timer_now + INTERVAL, Ordering::SeqCst);
    mmio_write(TIMER_C1, next_wakeup);

    // TODO: Time Calibration
}

unsafe fn handle_timer() {
    let next_wakeup = NEXT_WAKEUP.fetch_add(INTERVAL, Ordering::SeqCst);
    mmio_write(TIMER_C1, next_wakeup + INTERVAL);
    mmio_write(TIMER_CS, TIMER_CS_M1);
    print!("%");
}

pub unsafe fn handle_irq() {
    let pending = mmio_read(IRQ_PENDING_1);
    match pending {
        SYSTEM_TIMER_IRQ_1 => handle_timer(),
        _ => {
            panic!("Unknown IRQ: 0x{:x}", pending);
        }
    };
}
