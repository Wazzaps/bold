use crate::arch::aarch64::mmio::{get_uptime_us, sleep_us};
use crate::driver_manager;
use crate::driver_manager::DeviceType;
use crate::fonts;
use crate::framebuffer;
use crate::ipc;
use crate::prelude::*;
use core::sync::atomic::{AtomicBool, Ordering};
use ipc::signal::Signal;
use spin::{Mutex, MutexGuard};

pub struct PerfInfo {
    sample_us_sum: u64,
    sample_count: u64,
    last_sample_us: u64,
}

#[derive(Debug)]
pub struct PerfReport {
    sample_avg_us: u64,
    avg_fps: u64,
    last_sample_us: u64,
}

impl PerfInfo {
    const fn new() -> Self {
        Self {
            sample_us_sum: 0,
            sample_count: 0,
            last_sample_us: 0,
        }
    }

    fn update(&mut self, time_us: u64) {
        self.last_sample_us = time_us;
        self.sample_us_sum += time_us;
        self.sample_count += 1;
    }

    fn report(&self) -> PerfReport {
        let sample_avg_us = self
            .sample_us_sum
            .checked_div(self.sample_count)
            .unwrap_or(0);
        PerfReport {
            sample_avg_us,
            avg_fps: 1000000u64.checked_div(sample_avg_us).unwrap_or(0),
            last_sample_us: self.last_sample_us,
        }
    }
}

static PERF_INFO: Mutex<PerfInfo> = Mutex::new(PerfInfo::new());

pub fn perf_report() -> PerfReport {
    PERF_INFO.lock().report()
}

async fn vsync<F, Fut>(mut f: F)
where
    F: FnMut() -> Fut,
    Fut: core::future::Future<Output = bool>,
{
    // FIXME: this is racy, fix when condvars are a thing
    if !FRAMEBUFFER_DIRTY.swap(false, Ordering::SeqCst) {
        RENDER_WAKER.wait().await;
    }
    let start = get_uptime_us();
    let did_render = (f)().await;
    let end = get_uptime_us();
    if did_render {
        PERF_INFO.lock().update(end - start);
    }
    if end < start + 16666 {
        sleep_us(16666 - (end - start)).await;
    } else {
        yield_now().await;
    }
}

struct ConsoleState {
    pub text_buf: [u8; 80 * 30],
    pub last_text_buf: [u8; 80 * 30],
    pub cursor: u32,
    pub font: &'static [u8],
}

lazy_static! {
    static ref STATE: Mutex<ConsoleState> = Mutex::new(ConsoleState {
        text_buf: [b' '; 80 * 30],
        last_text_buf: [b' '; 80 * 30],
        cursor: 0,
        font: fonts::TERMINUS.get(),
    });
    static ref RENDER_WAKER: Signal = Signal::new();
}

pub static FRAMEBUFFER_DIRTY: AtomicBool = AtomicBool::new(true);
pub static FRAMEBUFFER_ALL_DIRTY: AtomicBool = AtomicBool::new(true);

async fn yield_lock<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    loop {
        match lock.try_lock() {
            None => {
                yield_now().await;
            }
            Some(it) => {
                return it;
            }
        }
    }
}

pub fn redraw() {
    FRAMEBUFFER_ALL_DIRTY.store(true, Ordering::SeqCst);
    FRAMEBUFFER_DIRTY.store(true, Ordering::SeqCst);
    RENDER_WAKER.notify_all();
}

pub async fn set_font(font: &'static [u8]) {
    yield_lock(&STATE).await.font = font;
    redraw();
}

fn scroll_buffer(state: &mut ConsoleState) {
    for line in 0..29 {
        let (top, bottom) = state.text_buf.split_at_mut((line + 1) * 80);
        (&mut top[line * 80..(line + 1) * 80]).copy_from_slice(&bottom[0..80])
    }
    state.cursor = (state.text_buf.len() - 80) as u32;

    let last_line_end = state.text_buf.len();
    let last_line_start = last_line_end - 80;
    (&mut state.text_buf[last_line_start..last_line_end]).fill(b' ');
}

pub fn init() {
    // TODO: Unimplemented until USB
    // spawn_task!(b"FBCON.input", {
    //     // Create the input queue
    //     let root = ipc::ROOT.read().as_ref().unwrap().clone();
    //     let _input_queue = root
    //         .dir_get(ipc::well_known::ROOT_DEVICES)
    //         .await
    //         .unwrap()
    //         .dir_get(ipc::well_known::DEVICES_RPI_FB_CON)
    //         .await
    //         .unwrap()
    //         .dir_get(ipc::well_known::RPI_FB_CON0)
    //         .await
    //         .unwrap()
    //         .dir_link(ipc::well_known::RPI_FB_CON_IN, ipc::IpcSpscQueue::new())
    //         .await
    //         .unwrap();
    //
    //     // Write to it forever
    //     let mut buf = [0u8; 1];
    //     loop {
    //         if let Ok(1) = fi::Read::read(&DEVICE, &mut buf).await {
    //             input_queue.queue_write(&buf).warn();
    //         }
    //         yield_now().await;
    //     }
    // });

    spawn_task!(b"FBCON.ttyemu", {
        // Create the output queue
        let root = ipc::ROOT.read().as_ref().unwrap().clone();
        let output_queue = root
            .dir_get(ipc::well_known::ROOT_DEVICES)
            .await
            .unwrap()
            .dir_get(ipc::well_known::DEVICES_RPI_FB_CON)
            .await
            .unwrap()
            .dir_get(ipc::well_known::RPI_FB_CON0)
            .await
            .unwrap()
            .dir_link(ipc::well_known::RPI_FB_CON_OUT, ipc::IpcSpscQueue::new())
            .await
            .unwrap();

        fn print_unk(data: u8, state: &mut ConsoleState) {
            let cursor = state.cursor;
            fn to_hex_upper(data: u8) -> u8 {
                if (data >> 4) > 9 {
                    b'a' + (data >> 4) - 10
                } else {
                    b'0' + (data >> 4)
                }
            }
            fn to_hex_lower(data: u8) -> u8 {
                if (data & 0xf) > 9 {
                    b'a' + (data & 0xf) - 10
                } else {
                    b'0' + (data & 0xf)
                }
            }
            state.text_buf[(cursor) as usize] = b'<';
            state.text_buf[(cursor + 1) as usize] = to_hex_upper(data);
            state.text_buf[(cursor + 2) as usize] = to_hex_lower(data);
            state.text_buf[(cursor + 3) as usize] = b'>';
            state.cursor += 4;
        }

        // Write to it forever
        let mut buf = [0u8; 1];
        loop {
            if let Some(1) = output_queue.queue_read(&mut buf).await {
                match buf[0] {
                    // Newline
                    b'\n' => {
                        let mut state = yield_lock(&STATE).await;
                        if state.cursor as usize > (state.text_buf.len() - 80) {
                            scroll_buffer(&mut *state);
                        } else {
                            state.cursor = (state.cursor / 80 * 80) + 80;
                        }
                    }
                    // Backspace
                    0x7f | 0x08 => {
                        let mut state = yield_lock(&STATE).await;
                        state.cursor = (state.cursor / 80 * 80).max(state.cursor - 1);
                    }
                    // Bell (ignored)
                    0x07 => {}
                    // Control chars
                    0x1b => {
                        // print_unk(buf[0], &mut state);
                        if let Some(1) = output_queue.queue_read(&mut buf).await {
                            // print_unk(buf[0], &mut state);
                            match buf[0] {
                                b'[' => {
                                    if let Some(1) = output_queue.queue_read(&mut buf).await {
                                        let mut state = yield_lock(&STATE).await;
                                        // print_unk(buf[0], &mut state);
                                        match buf[0] {
                                            // Clear rest of line
                                            b'K' => {
                                                for clear_cursor in
                                                    state.cursor..(state.cursor / 80 * 80) + 80
                                                {
                                                    state.text_buf[clear_cursor as usize] = b' ';
                                                }
                                            }
                                            // Left
                                            b'D' => {
                                                state.cursor =
                                                    (state.cursor / 80 * 80).max(state.cursor - 1);
                                            }
                                            // Right
                                            b'C' => {
                                                state.cursor = ((state.cursor / 80 * 80) + 80)
                                                    .min(state.cursor + 1);
                                            }
                                            _ => {
                                                print_unk(buf[0], &mut state);
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    let mut state = yield_lock(&STATE).await;
                                    print_unk(buf[0], &mut state);
                                }
                            }
                        }
                    }
                    // Normal letters
                    b' '..=b'~' => {
                        let mut state = yield_lock(&STATE).await;
                        let cursor = state.cursor as usize;
                        state.text_buf[cursor] = buf[0];
                        state.cursor += 1;
                    }
                    // Unknown
                    _ => {
                        let mut state = yield_lock(&STATE).await;
                        print_unk(buf[0], &mut state);
                    }
                }

                {
                    let mut state = yield_lock(&STATE).await;
                    if (state.cursor as usize) >= state.text_buf.len() {
                        // state.cursor = state.text_buf.len() as u32 - 1;
                        scroll_buffer(&mut *state);
                    }
                }

                FRAMEBUFFER_DIRTY.store(true, Ordering::SeqCst);
                RENDER_WAKER.notify_all();
            }
        }
    });

    // Drawing thread
    spawn_task!(b"FBCON.render", {
        println!("[INFO] Framebuffer console initialising");

        // Draw loop
        let framebuffer = driver_manager::device_by_type(DeviceType::Framebuffer)
            .unwrap()
            .ctrl
            .unwrap();
        loop {
            vsync(async || {
                let all_dirty = FRAMEBUFFER_ALL_DIRTY.swap(false, Ordering::SeqCst);
                for row in 0..30 {
                    let mut modified = false;
                    for col in 0..80 {
                        {
                            let mut state = yield_lock(&STATE).await;
                            let font = state.font;

                            if all_dirty
                                || state.text_buf[row * 80 + col]
                                    != state.last_text_buf[row * 80 + col]
                            {
                                framebuffer
                                    .call(framebuffer::FramebufferCM::DrawChar {
                                        font,
                                        char: state.text_buf[row * 80 + col],
                                        row,
                                        col,
                                    })
                                    .await
                                    .warn();
                                state.last_text_buf[row * 80 + col] =
                                    state.text_buf[row * 80 + col];
                                modified = true;
                            }
                        }
                    }
                    if modified {
                        yield_now().await;
                    }
                }

                true
            })
            .await;
        }
    });
}
