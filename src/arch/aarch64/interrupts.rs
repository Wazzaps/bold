use crate::arch::aarch64::exceptions::ExceptionContext;
use crate::arch::aarch64::mmio::{
    get_uptime_us, mmio_read, mmio_write, ENABLE_IRQS_1, ENABLE_IRQS_2, IRQ_PENDING_1,
    SYSTEM_TIMER_IRQ_1, TIMER_C1, TIMER_CLO, TIMER_CS, TIMER_CS_M1, UART_IRQ,
};
use crate::ktask::null_waker;
use crate::prelude::*;
use crate::threads::current_core;
use crate::{sleep_queue, threads};
use core::mem::transmute;
use core::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use core::task::Waker;
use cortex_a::registers::DAIF;
use spin::Mutex;
use tock_registers::interfaces::{Readable, Writeable};

static NEXT_WAKEUP: AtomicU32 = AtomicU32::new(0);
static NEXT_WAKER: Mutex<Option<Waker>> = Mutex::new(None);

static CALIBRATION_START_TICKS: AtomicU32 = AtomicU32::new(0);
static CALIBRATION_START_UPTIME_US: AtomicU64 = AtomicU64::new(0);
static TIMER_FACTOR: AtomicU32 = AtomicU32::new(0);

// FIXME: Do this better
static IRQ_HANDLERS: [IrqHandler; 32] = [
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
    IrqHandler::default(),
];

const CALIBRATION_DURATION: u32 = 100 * 1000; // 100 ms

pub type IrqHandlerFunc = unsafe extern "C" fn(usize);

pub struct IrqHandler {
    func: AtomicUsize,
    arg: AtomicUsize,
}

impl IrqHandler {
    const fn default() -> Self {
        IrqHandler {
            func: AtomicUsize::new(0),
            arg: AtomicUsize::new(0),
        }
    }
}

pub struct IrqLock {
    prev_state: u64,
}

pub fn irq_lock() -> IrqLock {
    let prev_state = unsafe { get_msr!(DAIF) };
    unsafe { disable() };
    // assert_eq!(unsafe { get_msr!(DAIF) } & (1 << 7), 1 << 7);
    // assert_eq!(unsafe { get_msr!(DAIF) }, DAIF.get());
    IrqLock { prev_state }
}

impl Drop for IrqLock {
    fn drop(&mut self) {
        DAIF.set(self.prev_state);
        assert_eq!(DAIF.get(), self.prev_state);
    }
}

pub unsafe fn enable() {
    set_msr_const!(daifclr, 1 | 2);
}
pub unsafe fn enable_fiq() {
    set_msr_const!(daifclr, 1);
}

pub unsafe fn disable() {
    set_msr_const!(daifset, 1 | 2);
}

pub unsafe fn init() {
    // Time Calibration Setup
    let uptime_now = get_uptime_us();
    let timer_now = mmio_read(TIMER_CLO);
    println!("[INFO] Calibrating timer...");
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

unsafe fn handle_timer(e: &mut ExceptionContext) {
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

    let current_core = current_core();
    if current_core >= threads::CORE_COUNT {
        panic!("Got interrupt on unknown core #{}", current_core);
    }

    let executor = &threads::EXECUTORS.get().unwrap()[current_core];

    // Context switch if needed
    if executor.did_timeout() {
        let last_tid = executor.switch(e);
        if last_tid != 0 {
            executor.wake(last_tid);
        }
        sleep_queue::push(
            get_uptime_us() + threads::THREAD_TIMEOUT_US as u64,
            null_waker(),
        );
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
    let mut pending = mmio_read(IRQ_PENDING_1);

    if (pending & SYSTEM_TIMER_IRQ_1) != 0 {
        handle_timer(e);
        pending &= !SYSTEM_TIMER_IRQ_1;
    }

    if pending != 0 {
        // Required because Circle's irqlock verifies FIQs are not masked
        enable_fiq();

        for irq in 4..32 {
            if ((1 << irq) & pending) != 0 {
                let handler = &IRQ_HANDLERS[irq];
                let func = handler.func.load(Ordering::SeqCst);
                if func != 0 {
                    let arg = handler.arg.load(Ordering::SeqCst);
                    let func = transmute::<_, IrqHandlerFunc>(func);
                    (func)(arg);
                    pending &= !(1 << irq);
                }
            }
        }

        if pending != 0 {
            panic!("Unknown IRQ(s): 0x{:x}", pending);
        }
    }
}

pub unsafe fn attach_irq_handler(irq: usize, func: IrqHandlerFunc, arg: usize) {
    let handler = &IRQ_HANDLERS[irq];
    handler.arg.store(arg, Ordering::SeqCst);
    handler.func.store(func as usize, Ordering::SeqCst);
}

pub unsafe fn detach_irq_handler(irq: usize) {
    let handler = &IRQ_HANDLERS[irq];
    handler.func.store(0, Ordering::SeqCst);
    handler.arg.store(0, Ordering::SeqCst);
}
