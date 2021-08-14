use crate::arch::aarch64::interrupts;
use crate::arch::aarch64::interrupts::wake_up_in;
use crate::arch::aarch64::mmio::get_uptime_us;
use crate::println;
use alloc::collections::VecDeque;
use core::task::Waker;
use spin::{Mutex, Once};

static SLEEP_QUEUE: Once<Mutex<VecDeque<(u64, Waker)>>> = Once::new();
const EARLY_WAKE_MARGIN_US: u64 = 1000;

pub fn pop() -> (u64, Option<Waker>) {
    let current_time = get_uptime_us();
    let mut sleep_queue = SLEEP_QUEUE.call_once(|| Mutex::new(VecDeque::new())).lock();
    loop {
        let wake_time = sleep_queue.pop_front();
        if let Some((wake_time, waker)) = wake_time {
            // println!("Wanted wakeup in {} ms", (wake_time - current_time) / 1000);
            if wake_time <= current_time + EARLY_WAKE_MARGIN_US {
                // println!("Not Sleeping");
                // Wake immediately and continue
                waker.wake_by_ref();
                continue;
            } else {
                // println!("Sleeping");
                return (wake_time - current_time, Some(waker));
            }
        } else {
            // println!("Empty");
            // Default to 1 second wake-ups
            return (1000 * 1000, None);
        }
    }
}

pub fn push(wake_time: u64, waker: Waker) {
    unsafe { interrupts::disable() };
    let mut sleep_queue = SLEEP_QUEUE.call_once(|| Mutex::new(VecDeque::new())).lock();

    // Set timer if earlier than first wakeup
    let current_first_wakeup = sleep_queue.get(0).map(|i| i.0).unwrap_or(u64::MAX);
    if wake_time < current_first_wakeup {
        // println!(
        //     "Setup wakeup in {} ms",
        //     (wake_time - get_uptime_us()) / 1000
        // );
        wake_up_in(wake_time - get_uptime_us());
    }

    // println!("Pushing");
    sleep_queue.push_back((wake_time, waker));
    sleep_queue.make_contiguous().sort_by_key(|t| t.0);
    unsafe { interrupts::enable() };
}
