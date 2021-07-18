use crate::arch::aarch64::mmio::{delay_us, get_uptime_us};
use crate::driver_manager;
use crate::driver_manager::DeviceType;
use crate::fonts;
use crate::framebuffer;
use crate::ktask::yield_now;
use crate::println;
use crate::spawn_task;
use crate::utils::ErrWarn;
use alloc::boxed::Box;

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

pub fn init() {
    spawn_task!({
        println!("[INFO] Drawing something");
        let framebuffer = driver_manager::device_by_type(DeviceType::Framebuffer)
            .unwrap()
            .ctrl
            .unwrap();
        let mut i = 0;
        loop {
            let s = [
                b"Hello, World!                                                                   ",
                b"                                                                                ",
                b"  Lorem ipsum dolor sit amet, consectetur adipiscing elit. Donec lacus ligula,  ",
                b"  vulputate at ullamcorper non, rutrum nec ipsum. Maecenas rutrum in tellus.    ",
                b"  kernel@bold /#                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
                b"                                                                                ",
            ];
            vsync(async || {
                for (i, font) in [fonts::ISO88591, fonts::ISO, fonts::TERMINUS, fonts::VGA]
                    .iter()
                    .enumerate()
                {
                    for col in 0..80 {
                        for row in 0..30 {
                            framebuffer
                                .call(framebuffer::FramebufferCM::DrawChar {
                                    font,
                                    char: s[row][col],
                                    row: row + i * 7,
                                    col,
                                })
                                .await
                                .warn();
                        }
                    }
                }
            })
            .await;
            i += 1;
            break;
        }
    });
}
