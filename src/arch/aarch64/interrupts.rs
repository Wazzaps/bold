use crate::arch::aarch64::exceptions::ExceptionContext;
use crate::arch::aarch64::mmio::{
    get_uptime_us, mmio_read, mmio_write, ENABLE_IRQS_1, ENABLE_IRQS_2, IRQ_PENDING_1,
    SYSTEM_TIMER_IRQ_1, TIMER_C1, TIMER_CLO, TIMER_CS, TIMER_CS_M1, UART_IRQ,
};
use crate::prelude::*;
use crate::sleep_queue;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use core::task::Waker;
use cortex_a::registers::DAIF;
use spin::Mutex;
use tock_registers::interfaces::{Readable, Writeable};

static NEXT_WAKEUP: AtomicU32 = AtomicU32::new(0);
static NEXT_WAKER: Mutex<Option<Waker>> = Mutex::new(None);

static CALIBRATION_START_TICKS: AtomicU32 = AtomicU32::new(0);
static CALIBRATION_START_UPTIME_US: AtomicU64 = AtomicU64::new(0);
static TIMER_FACTOR: AtomicU32 = AtomicU32::new(0);

const CALIBRATION_DURATION: u32 = 100 * 1000; // 100 ms

pub struct IrqLock {
    prev_state: u64,
}

pub fn irq_lock() -> IrqLock {
    let prev_state = unsafe { get_msr!(DAIF) };
    unsafe { disable() };
    assert_eq!(unsafe { get_msr!(DAIF) } & (1 << 7), 1 << 7);
    assert_eq!(unsafe { get_msr!(DAIF) }, DAIF.get());
    IrqLock { prev_state }
}

impl Drop for IrqLock {
    fn drop(&mut self) {
        DAIF.set(self.prev_state);
        assert_eq!(DAIF.get(), self.prev_state);
    }
}

pub unsafe fn enable() {
    set_msr_const!(daifclr, 2);
}

pub unsafe fn disable() {
    set_msr_const!(daifset, 2);
}

pub unsafe fn init() {
    // Time Calibration Setup
    let uptime_now = get_uptime_us();
    let timer_now = mmio_read(TIMER_CLO);
    println!("[DBUG] Start ticks={} uptime={}", timer_now, uptime_now);
    CALIBRATION_START_TICKS.store(timer_now, Ordering::SeqCst);
    CALIBRATION_START_UPTIME_US.store(uptime_now, Ordering::SeqCst);
    mmio_write(TIMER_C1, timer_now + CALIBRATION_DURATION);

    // Interrupt enable
    mmio_write(ENABLE_IRQS_1, SYSTEM_TIMER_IRQ_1);
    mmio_write(ENABLE_IRQS_2, UART_IRQ);
    enable();
}

pub fn wake_up_in(time_us: u64) {
    // FIXME: This whole place is probably Race-City, though it should only cause spurious wake-ups
    let timer_factor = TIMER_FACTOR.load(Ordering::SeqCst);
    if timer_factor != 0 {
        let ticks_to_sleep = ((time_us * timer_factor as u64) >> 16).min((1 << 32) - 1) as u32;
        let current_time = unsafe { mmio_read(TIMER_CLO) };
        let new_wakeup_time = current_time.wrapping_add(ticks_to_sleep);
        let current_next_wakeup = NEXT_WAKEUP.load(Ordering::SeqCst);
        let current_ticks_to_sleep = current_next_wakeup.wrapping_sub(current_time);

        if ticks_to_sleep < current_ticks_to_sleep {
            unsafe { mmio_write(TIMER_C1, new_wakeup_time) };

            let post_update_time = unsafe { mmio_read(TIMER_CLO) };
            // FIXME: overflow situation?
            if post_update_time > new_wakeup_time {
                // We missed it, wakeup now
                if let Some(waker) = NEXT_WAKER.lock().take() {
                    waker.wake_by_ref();
                }
            }
        }
    }
}

/// The timer factor is ticks_per_microsecond, but multiplied by 2**16 so it can be used without
/// floating point or integer division operations.
fn calc_timer_factor(
    calibration_duration_ticks: u32,
    calibration_duration_uptime: u64,
) -> Result<u32, ()> {
    // FIXME: This logic won't work on very slow or very fast processors
    if calibration_duration_ticks == 0 || calibration_duration_uptime == 0 {
        return Err(());
    }

    let timer_factor =
        (calibration_duration_ticks as u64 * (1 << 16)) / calibration_duration_uptime;
    if timer_factor > 1 << 32 {
        return Err(());
    }

    Ok(timer_factor as u32)
}

#[allow(dead_code, unused_variables)]
unsafe fn print_stacktrace(e: &mut ExceptionContext) {
    println!("@@@@");
    let sp = get_msr!(sp_el0);
    println!("- PC: 0x{:x} ", e.elr_el1);
    println!("- LR: 0x{:x} ", e.lr);
    for i in 0..128 {
        let val = *(sp as *const u64).offset(i);
        if (0x80000..=0x80000 + 0x3f400).contains(&val) {
            println!("- {}: 0x{:x} ", i, val);
        }
    }
    println!("@@@@");
}

unsafe fn handle_timer(_e: &mut ExceptionContext) {
    let timer_factor = TIMER_FACTOR.load(Ordering::SeqCst);
    if timer_factor == 0 {
        let calibration_end_ticks = mmio_read(TIMER_CLO);
        let calibration_end_uptime = get_uptime_us();

        let calibration_duration_ticks =
            calibration_end_ticks - CALIBRATION_START_TICKS.load(Ordering::SeqCst);
        let calibration_duration_uptime =
            calibration_end_uptime - CALIBRATION_START_UPTIME_US.load(Ordering::SeqCst);

        println!(
            "[DBUG] End ticks={} uptime={}",
            calibration_end_ticks, calibration_end_uptime
        );
        println!(
            "[INFO] Timer Calibration: {} ticks = {} us",
            calibration_duration_ticks, calibration_duration_uptime,
        );

        let timer_factor =
            calc_timer_factor(calibration_duration_ticks, calibration_duration_uptime)
                .unwrap_or_else(|_| {
                    println!("[WARN] Overflow in timer factor calculation, using default value");
                    1 << 16
                });
        println!("[INFO] Timer Factor: {} / {}", timer_factor, 1 << 16);
        TIMER_FACTOR.store(timer_factor, Ordering::SeqCst);
    }

    // Wake last event
    if let Some(waker) = &*NEXT_WAKER.lock() {
        waker.wake_by_ref();
    }

    // Queue next event
    let (next_wakeup, waker) = sleep_queue::pop();
    *NEXT_WAKER.lock() = waker;

    wake_up_in(next_wakeup);

    // Ack interrupt
    mmio_write(TIMER_CS, TIMER_CS_M1);
}

pub unsafe fn handle_irq(e: &mut ExceptionContext) {
    let pending = mmio_read(IRQ_PENDING_1);
    match pending {
        SYSTEM_TIMER_IRQ_1 => handle_timer(e),
        _ => {
            panic!("Unknown IRQ: 0x{:x}", pending);
        }
    };
}
