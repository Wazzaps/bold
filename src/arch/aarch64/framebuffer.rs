use crate::arch::aarch64::mailbox;
use crate::println;

// const FB_WIDTH: u32 = 1280;
// const FB_HEIGHT: u32 = 720;

const FB_WIDTH: u32 = 640;
const FB_HEIGHT: u32 = 480;

#[derive(Debug)]
#[repr(align(16), C)]
struct FramebufferInfo {
    width: u32,
    height: u32,
    virt_width: u32,
    virt_height: u32,
    pitch: u32,
    depth: u32,
    x_offset: u32,
    y_offset: u32,
    pointer: u32,
    size: u32,
}

static mut FB_INFO: FramebufferInfo = FramebufferInfo {
    width: FB_WIDTH,
    height: FB_HEIGHT,
    virt_width: FB_WIDTH,
    virt_height: FB_HEIGHT,
    pitch: 0,
    depth: 24,
    x_offset: 0,
    y_offset: 0,
    pointer: 0,
    size: 0,
};

pub unsafe fn init() {
    mailbox::write_raw(((&mut FB_INFO as *mut FramebufferInfo as usize as u32) & !0xF) | 1);
    println!("{:?}", FB_INFO);
}

pub unsafe fn draw_example(variant: u32) {
    let fb_info = (&FB_INFO as *const FramebufferInfo).read_volatile();
    let fb: *mut u8 = fb_info.pointer as usize as *mut u8;
    let width = fb_info.width;
    let height = fb_info.height;
    let pitch = fb_info.pitch;

    for y in 0..height {
        for x in 0..width {
            let pixel = fb.offset((y * pitch + x * 3) as isize);
            pixel.offset(0).write_volatile(((x + variant) % 256) as u8);
            pixel.offset(1).write_volatile((y % 256) as u8);
            pixel.offset(2).write_volatile(0);
        }
    }
}
