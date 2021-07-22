use crate::arch::aarch64::mailbox;
use crate::driver_manager::{DeviceType, DriverInfo};
use crate::file_interface::IoResult;
use crate::framebuffer::FramebufferCM;
use crate::{driver_manager, fi, println};
use alloc::prelude::v1::Box;
use async_trait::async_trait;
use core::cell::UnsafeCell;
use spin::RwLock;

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

#[link_section = ".dma"]
#[used]
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

// ----- Driver -----

#[derive(Debug)]
struct Driver {
    info: UnsafeCell<DriverInfo>,
}

impl driver_manager::Driver for Driver {
    fn init(&self) -> Result<(), ()> {
        unsafe {
            mailbox::write_raw(((&mut FB_INFO as *mut FramebufferInfo as usize as u32) & !0xF) | 1);
            println!("{:?}", FB_INFO);
        }
        // FIXME: Vulnerability
        unsafe {
            (*self.info.get()).initialized = true;
        }
        Ok(())
    }

    fn info(&'static self) -> &'static DriverInfo {
        // FIXME: Vulnerability
        unsafe { self.info.get().as_ref().unwrap() }
    }
}

static mut DRIVER: Driver = Driver {
    info: UnsafeCell::new(DriverInfo {
        name: b"Raspberry Pi 3 Framebuffer",
        initialized: false,
        devices: RwLock::new([driver_manager::Device {
            device_type: DeviceType::Framebuffer,
            interface: fi::FileInterface {
                sync_read: None,
                read: None,
                sync_write: None,
                write: None,
                ctrl: Some(&DEVICE),
            },
        }]),
    }),
};

#[link_section = ".drivers"]
#[used]
static mut DRIVER_REF: &dyn driver_manager::Driver = unsafe { &DRIVER };

// ----- Device -----

#[derive(Debug)]
struct Device;

#[async_trait]
impl fi::Control for Device {
    async fn call(&self, msg: FramebufferCM) -> IoResult<()> {
        match msg {
            FramebufferCM::DrawExample { variant } => {
                let fb_info = unsafe { (&FB_INFO as *const FramebufferInfo).read_volatile() };
                let fb: *mut u8 = (fb_info.pointer & 0x3FFFFFFF) as usize as *mut u8;
                let width = fb_info.width;
                let height = fb_info.height;
                let pitch = fb_info.pitch;

                if !fb.is_null() {
                    for y in 0..height {
                        for x in 0..width {
                            unsafe {
                                let pixel = fb.offset((y * pitch + x * 3) as isize);
                                pixel.offset(0).write_volatile(((x + variant) % 256) as u8);
                                pixel.offset(1).write_volatile((y % 256) as u8);
                                pixel.offset(2).write_volatile(0);
                            }
                        }
                    }
                }
            }
            FramebufferCM::DrawChar {
                font,
                char,
                row,
                col,
            } => {
                let fb_info = unsafe { (&FB_INFO as *const FramebufferInfo).read_volatile() };
                let fb: *mut u8 = (fb_info.pointer & 0x3FFFFFFF) as usize as *mut u8;
                let pitch = fb_info.pitch;

                if !fb.is_null() {
                    let char = char as usize;
                    let char = font[char * 8 * 16 * 3..(char + 1) * 8 * 16 * 3].as_ptr();

                    for y in 0..16 {
                        unsafe {
                            let dst = fb.offset(
                                (row as isize * 16 + y) * (pitch as isize) + col as isize * 8 * 3,
                            );
                            let src = char.offset(y * 8 * 3);
                            for x in 0..(8 * 3) {
                                dst.offset(x).write_volatile(*src.offset(x));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

static DEVICE: Device = Device;
