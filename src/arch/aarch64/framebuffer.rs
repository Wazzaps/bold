use crate::arch::aarch64::mailbox::_send_fb_property_tags;
use crate::prelude::*;

use crate::arch::aarch64::mmu;
use crate::arch::aarch64::phymem;
use crate::driver_manager::{DeviceType, DriverInfo};
use crate::framebuffer::FramebufferCM;
use crate::{driver_manager, fi, println};

use core::cell::UnsafeCell;
use spin::{Mutex, RwLock};

// const FB_WIDTH: u32 = 1280;
// const FB_HEIGHT: u32 = 720;

const FB_WIDTH: u32 = 640;
const FB_HEIGHT: u32 = 480;

const FB_PIXEL_ORDER_RGB: u32 = 1;
const FB_PIXEL_ORDER_BGR: u32 = 0;

#[derive(Debug)]
pub(crate) struct FramebufferInfo {
    pub width: u32,
    pub height: u32,
    pub virt_width: u32,
    pub virt_height: u32,
    pub pitch: u32,
    pub depth: u32,
    pub x_offset: u32,
    pub y_offset: u32,
    pub pointer: u32,
    pub size: u32,
}

// ----- Driver -----

#[derive(Debug)]
struct Driver {
    info: UnsafeCell<DriverInfo>,
    pub fb_info: Mutex<FramebufferInfo>,
}

impl driver_manager::Driver for Driver {
    fn early_init(&self) -> Result<(), ()> {
        unsafe {
            println!("Getting framebuffer");

            let fb_info = _send_fb_property_tags()?;
            println!("{:?}", fb_info);
            let slice = PhySlice {
                base: PhyAddr(fb_info.pointer as usize),
                len: fb_info.size as usize,
            };

            *self.fb_info.lock() = fb_info;

            let pointer = slice.base.virt_mut() as usize;
            let page_count = ((slice.len as u64 + PAGE_SIZE - 1) / PAGE_SIZE) as usize;
            phymem::reserve(slice).unwrap();
            for page in 0..page_count {
                mmu::virt2pte_mut(pointer + page * PAGE_SIZE as usize, |pte| {
                    let (pte, _offset) = pte.unwrap();
                    const PAGE_FLAGS: u64 = mmu::PT_BLOCK
                        | mmu::PT_AF
                        | mmu::PT_OSH
                        | mmu::PT_DEV
                        | mmu::PT_RW
                        | mmu::PT_NX;
                    let prev_ptr = *pte & 0xfffff000;
                    *pte = prev_ptr | PAGE_FLAGS;
                });
            }

            println!("Framebuffer OK");
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
    fb_info: Mutex::new(FramebufferInfo {
        width: 0,
        height: 0,
        virt_width: 0,
        virt_height: 0,
        pitch: 0,
        depth: 0,
        x_offset: 0,
        y_offset: 0,
        pointer: 0,
        size: 0,
    }),
};

#[link_section = ".drivers"]
#[used]
static mut DRIVER_REF: &dyn driver_manager::Driver = unsafe { &DRIVER };

// ----- Device -----

#[derive(Debug)]
struct Device;

fn draw_char(fb: *mut u8, pitch: u32, font: &'static [u8], char: u8, row: usize, col: usize) {
    let char = char as usize;
    let char = font[char * 8 * 16 * 4..(char + 1) * 8 * 16 * 4].as_ptr();

    for y in 0..16 {
        unsafe {
            let dst = fb.offset((row as isize * 16 + y) * (pitch as isize) + col as isize * 8 * 4);
            let src = char.offset(y * 8 * 4);
            for x in 0..(8 * 4) {
                dst.offset(x).write_volatile(*src.offset(x));
            }
        }
    }
}

#[async_trait]
impl fi::Control for Device {
    async fn call(&self, msg: FramebufferCM) -> IoResult<()> {
        match msg {
            FramebufferCM::DrawExample { variant } => {
                let fb_info = unsafe { DRIVER.fb_info.lock() };
                if fb_info.pointer != 0 {
                    let fb = unsafe { PhyAddr(fb_info.pointer as usize).virt_mut() as *mut u8 };
                    let width = fb_info.width;
                    let height = fb_info.height;
                    let pitch = fb_info.pitch;

                    for y in 0..height {
                        for x in 0..width {
                            unsafe {
                                let pixel = fb.offset((y * pitch + x * 4) as isize);
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
                let fb_info = unsafe { DRIVER.fb_info.lock() };
                if fb_info.pointer != 0 {
                    let fb = unsafe { PhyAddr(fb_info.pointer as usize).virt_mut() as *mut u8 };
                    let pitch = fb_info.pitch;

                    draw_char(fb, pitch, font, char, row, col);
                }
            }
        }
        Ok(())
    }
}

pub unsafe fn panic(message: &[u8]) {
    DRIVER.fb_info.force_unlock();
    let fb_info = DRIVER.fb_info.lock();
    if fb_info.pointer != 0 {
        let fb = unsafe { PhyAddr(fb_info.pointer as usize).virt_mut() as *mut u8 };
        let width = fb_info.width;
        let height = fb_info.height;
        let pitch = fb_info.pitch;

        // Clear screen
        for y in 0..height {
            for x in 0..width {
                let pixel = fb.offset((y * pitch + x * 4) as isize);
                pixel.offset(0).write_volatile(69);
                pixel.offset(1).write_volatile(27);
                pixel.offset(2).write_volatile(49);
            }
        }

        // Draw message
        for (i, c) in message.iter().enumerate() {
            draw_char(
                fb,
                pitch,
                crate::fonts::TERMINUS.get(),
                if *c == b'\n' { b' ' } else { *c },
                3 + i / 70,
                5 + i % 70,
            );
        }
    }
}

static DEVICE: Device = Device;
