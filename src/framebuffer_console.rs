use crate::arch::aarch64::mmio::{delay_us, get_uptime_us};
use crate::driver_manager;
use crate::driver_manager::DeviceType;
use crate::fonts;
use crate::framebuffer;
use crate::ipc;
use crate::ktask::yield_now;
use crate::println;
use crate::spawn_task;
use crate::utils::ErrWarn;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;

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

struct ConsoleState {
    pub text_buf: [u8; 80 * 30],
    pub cursor: u32,
    pub font: &'static [u8],
}

static STATE: Mutex<ConsoleState> = Mutex::new(ConsoleState {
    text_buf: [b' '; 80 * 30],
    cursor: 0,
    font: fonts::TERMINUS,
});
static IS_CHANGED: AtomicBool = AtomicBool::new(true);

pub fn set_font(font: &'static [u8]) {
    STATE.lock().font = font;
    IS_CHANGED.store(true, Ordering::SeqCst);
}

pub fn init() {
    spawn_task!({
        // Create the input queue
        let root = ipc::ROOT.read().as_ref().unwrap().clone();
        let _input_queue = root
            .dir_get(ipc::well_known::ROOT_DEVICES)
            .await
            .unwrap()
            .dir_get(ipc::well_known::DEVICES_RPI_FB_CON)
            .await
            .unwrap()
            .dir_get(ipc::well_known::RPI_FB_CON0)
            .await
            .unwrap()
            .dir_link(ipc::well_known::RPI_FB_CON_IN, ipc::IpcSpscQueue::new())
            .await
            .unwrap();

        // Write to it forever
        // TODO: Unimplemented until USB
        // let mut buf = [0u8; 1];
        // loop {
        //     if let Ok(1) = fi::Read::read(&DEVICE, &mut buf).await {
        //         input_queue.queue_write(&buf).await.warn();
        //     }
        //     yield_now().await;
        // }
    });

    spawn_task!({
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

        // Write to it forever
        let mut buf = [0u8; 1];
        loop {
            if let Some(1) = output_queue.queue_read(&mut buf).await {
                let mut state = STATE.lock();
                let cursor = state.cursor;
                match buf[0] {
                    // Newline
                    b'\n' => {
                        state.cursor = (cursor / 80 * 80) + 80;
                    }
                    // Normal letters
                    b' '..=b'~' => {
                        state.text_buf[cursor as usize] = buf[0];
                        state.cursor += 1;
                    }
                    // Unknown
                    _ => {
                        fn to_hex_upper(data: u8) -> u8 {
                            if (data >> 4) > 9 {
                                b'a' + (data >> 4) - 9
                            } else {
                                b'0' + (data >> 4)
                            }
                        }
                        fn to_hex_lower(data: u8) -> u8 {
                            if (data & 0xf) > 9 {
                                b'a' + (data & 0xf) - 9
                            } else {
                                b'0' + (data & 0xf)
                            }
                        }
                        state.text_buf[(cursor) as usize] = b'<';
                        state.text_buf[(cursor + 1) as usize] = to_hex_upper(buf[0]);
                        state.text_buf[(cursor + 2) as usize] = to_hex_lower(buf[0]);
                        state.text_buf[(cursor + 3) as usize] = b'>';
                        state.cursor += 4;
                    }
                }

                if (state.cursor as usize) >= state.text_buf.len() {
                    state.cursor = state.text_buf.len() as u32 - 1;
                }

                IS_CHANGED.store(true, Ordering::SeqCst);
            }
            yield_now().await;
        }
    });

    // Drawing thread
    spawn_task!({
        println!("[INFO] Framebuffer console initialising");

        // Draw loop
        let framebuffer = driver_manager::device_by_type(DeviceType::Framebuffer)
            .unwrap()
            .ctrl
            .unwrap();
        loop {
            vsync(async || {
                if IS_CHANGED.swap(false, Ordering::SeqCst) {
                    let state = STATE.lock();
                    let buf = state.text_buf;
                    let font = state.font;

                    for col in 0..80 {
                        for row in 0..30 {
                            framebuffer
                                .call(framebuffer::FramebufferCM::DrawChar {
                                    font,
                                    char: buf[row * 80 + col],
                                    row,
                                    col,
                                })
                                .await
                                .warn();
                        }
                    }
                }
            })
            .await;
        }
    });
}
