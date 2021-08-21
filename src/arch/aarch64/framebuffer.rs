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

            let fb_info = _send_fb_property_tags(FB_WIDTH, FB_HEIGHT)?;
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

static SWAP_BUF: Mutex<[u8; (4 * FB_WIDTH * FB_HEIGHT) as usize]> =
    Mutex::new([0; (4 * FB_WIDTH * FB_HEIGHT) as usize]);

#[inline(never)]
#[optimize(speed)]
unsafe fn memzero(dst: *mut u64, len: usize) {
    let val = 0;

    if len % 32 == 0 {
        for i in 0..(len / 32) {
            *dst.add(i * 32) = val;
            *dst.add((i * 32) + 1) = val;
            *dst.add((i * 32) + 2) = val;
            *dst.add((i * 32) + 3) = val;
            *dst.add((i * 32) + 4) = val;
            *dst.add((i * 32) + 5) = val;
            *dst.add((i * 32) + 6) = val;
            *dst.add((i * 32) + 7) = val;
            *dst.add((i * 32) + 8) = val;
            *dst.add((i * 32) + 9) = val;
            *dst.add((i * 32) + 10) = val;
            *dst.add((i * 32) + 11) = val;
            *dst.add((i * 32) + 12) = val;
            *dst.add((i * 32) + 13) = val;
            *dst.add((i * 32) + 14) = val;
            *dst.add((i * 32) + 15) = val;
            *dst.add((i * 32) + 16) = val;
            *dst.add((i * 32) + 17) = val;
            *dst.add((i * 32) + 18) = val;
            *dst.add((i * 32) + 19) = val;
            *dst.add((i * 32) + 20) = val;
            *dst.add((i * 32) + 21) = val;
            *dst.add((i * 32) + 22) = val;
            *dst.add((i * 32) + 23) = val;
            *dst.add((i * 32) + 24) = val;
            *dst.add((i * 32) + 25) = val;
            *dst.add((i * 32) + 26) = val;
            *dst.add((i * 32) + 27) = val;
            *dst.add((i * 32) + 28) = val;
            *dst.add((i * 32) + 29) = val;
            *dst.add((i * 32) + 30) = val;
            *dst.add((i * 32) + 31) = val;
        }
    } else {
        for i in 0..len {
            *dst.add(i) = val;
        }
    }
}

#[inline(never)]
#[optimize(speed)]
unsafe fn memset(dst: *mut u64, val: u8, len: usize) {
    let val = val as u64;
    let val = val
        | (val << 8)
        | (val << 16)
        | (val << 24)
        | (val << 32)
        | (val << 40)
        | (val << 48)
        | (val << 56);

    if len % 32 == 0 {
        for i in 0..(len / 32) {
            *dst.add(i * 32) = val;
            *dst.add((i * 32) + 1) = val;
            *dst.add((i * 32) + 2) = val;
            *dst.add((i * 32) + 3) = val;
            *dst.add((i * 32) + 4) = val;
            *dst.add((i * 32) + 5) = val;
            *dst.add((i * 32) + 6) = val;
            *dst.add((i * 32) + 7) = val;
            *dst.add((i * 32) + 8) = val;
            *dst.add((i * 32) + 9) = val;
            *dst.add((i * 32) + 10) = val;
            *dst.add((i * 32) + 11) = val;
            *dst.add((i * 32) + 12) = val;
            *dst.add((i * 32) + 13) = val;
            *dst.add((i * 32) + 14) = val;
            *dst.add((i * 32) + 15) = val;
            *dst.add((i * 32) + 16) = val;
            *dst.add((i * 32) + 17) = val;
            *dst.add((i * 32) + 18) = val;
            *dst.add((i * 32) + 19) = val;
            *dst.add((i * 32) + 20) = val;
            *dst.add((i * 32) + 21) = val;
            *dst.add((i * 32) + 22) = val;
            *dst.add((i * 32) + 23) = val;
            *dst.add((i * 32) + 24) = val;
            *dst.add((i * 32) + 25) = val;
            *dst.add((i * 32) + 26) = val;
            *dst.add((i * 32) + 27) = val;
            *dst.add((i * 32) + 28) = val;
            *dst.add((i * 32) + 29) = val;
            *dst.add((i * 32) + 30) = val;
            *dst.add((i * 32) + 31) = val;
        }
    } else {
        for i in 0..len {
            *dst.add(i) = val;
        }
    }
}

#[inline(never)]
unsafe fn memcpy(dst: *mut u64, src: *const u64, len: usize) {
    for i in 0..len {
        *dst.add(i) = *src.add(i);
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
            FramebufferCM::Clear { color } => {
                let fb_info = unsafe { DRIVER.fb_info.lock() };
                if fb_info.pointer != 0 {
                    let fb = unsafe { PhyAddr(fb_info.pointer as usize).virt_mut() as *mut u8 };

                    unsafe {
                        // let mut swap_fb = SWAP_BUF.lock();
                        // memset(
                        //     swap_fb.as_ptr() as *mut u64,
                        //     color as u8,
                        //     (fb_info.size / 8) as usize,
                        // );
                        // memcpy(
                        //     fb,
                        //     swap_fb.as_ptr() as *const u64,
                        //     (fb_info.size / 8) as usize,
                        // );

                        // Clear screen
                        if (color % 1024) == 0 {
                            memzero(fb as *mut u64, (fb_info.size / 8) as usize);
                        } else {
                            // Draw rectangle
                            // let pos_x: isize = (color as isize * 36123 ^ color as isize * 26123) % 540;
                            // let pos_y: isize = (color as isize * 52123 ^ color as isize * 7123) % 380;
                            // let color_r: u8 = (color as isize * 20123 ^ color as isize * 12123) as u8;
                            // let color_g: u8 = (color as isize * 19123 ^ color as isize * 11123) as u8;
                            // let color_b: u8 = (color as isize * 18123 ^ color as isize * 10123) as u8;
                            // for y in 0..100 {
                            //     for x in 0..100 {
                            //         *fb.offset(
                            //             (x + pos_x) * 4 + (pos_y + y) * fb_info.pitch as isize,
                            //         ) = color_r;
                            //         *fb.offset(
                            //             1 + (x + pos_x) * 4 + (pos_y + y) * fb_info.pitch as isize,
                            //         ) = color_g;
                            //         *fb.offset(
                            //             2 + (x + pos_x) * 4 + (pos_y + y) * fb_info.pitch as isize,
                            //         ) = color_b;
                            //     }
                            //     // memset(
                            //     //     fb.offset(pos_x * 4 + (pos_y + y) * fb_info.pitch as isize)
                            //     //         as *mut u64,
                            //     //     color,
                            //     //     100 * 4 / 8,
                            //     // );
                            // }

                            // Draw circle
                            let size: isize =
                                ((color as isize * 325) ^ (color as isize * 503)) % 30;
                            let pos_x: isize = ((color as isize * 361) ^ (color as isize * 261))
                                % (fb_info.virt_width as isize - size);
                            let pos_y: isize = ((color as isize * 521) ^ (color as isize * 712))
                                % (fb_info.virt_height as isize - size);
                            let color_r: u8 =
                                ((color as isize * 201) ^ (color as isize * 121)) as u8;
                            let color_g: u8 =
                                ((color as isize * 191) ^ (color as isize * 111)) as u8;
                            let color_b: u8 =
                                ((color as isize * 181) ^ (color as isize * 10123)) as u8;
                            let gw = fb_info.virt_width.min(fb_info.virt_height) as isize - 50;
                            let gw2 = fb_info.virt_width.min(fb_info.virt_height) as isize - 300;
                            for y in 0..size {
                                for x in 0..size {
                                    let rx = x - size / 2;
                                    let ry = y - size / 2;
                                    let gx = (pos_x + x) - (fb_info.virt_width as isize) / 2;
                                    let gy = (pos_y + y) - (fb_info.virt_height as isize) / 2;
                                    if (rx * rx + ry * ry) <= (size / 2) * (size / 2)
                                        && ((gx * gx + gy * gy) <= (gw / 2) * (gw / 2)
                                            && (gx * gx + gy * gy) >= (gw2 / 2) * (gw2 / 2))
                                            != ((gx > 0) != (gy > 0))
                                    {
                                        *fb.offset(
                                            (x + pos_x) * 4 + (pos_y + y) * fb_info.pitch as isize,
                                        ) = color_r;
                                        *fb.offset(
                                            1 + (x + pos_x) * 4
                                                + (pos_y + y) * fb_info.pitch as isize,
                                        ) = color_g + 30;
                                        *fb.offset(
                                            2 + (x + pos_x) * 4
                                                + (pos_y + y) * fb_info.pitch as isize,
                                        ) = color_b + 80;
                                    }
                                }
                            }
                        }
                    }
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
        let fb = PhyAddr(fb_info.pointer as usize).virt_mut() as *mut u8;
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
