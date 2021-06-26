use crate::console::Console;
use crate::framebuffer::Framebuffer;
use crate::println;
use core::convert::{TryFrom, TryInto};
use core::fmt;
use core::fmt::{Debug, Formatter, Write};
use core::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut};
use spin::RwLock;

extern "C" {
    static mut __drivers_start: u8;
    static mut __drivers_end: u8;
}

pub type ConsoleDriver = &'static RwLock<dyn Console + Send + Sync>;
pub type FramebufferDriver = &'static RwLock<dyn Framebuffer + Send + Sync>;

#[derive(Debug)]
pub enum DriverType {
    Console(ConsoleDriver),
    Framebuffer(FramebufferDriver),
}

pub struct Driver {
    pub name: &'static [u8],
    pub initialized: bool,
    pub vtable: DriverType,
}

impl TryFrom<&Driver> for ConsoleDriver {
    type Error = ();

    fn try_from(driver: &Driver) -> Result<Self, Self::Error> {
        if let DriverType::Console(console) = driver.vtable {
            Ok(console)
        } else {
            Err(())
        }
    }
}

impl TryFrom<&Driver> for FramebufferDriver {
    type Error = ();

    fn try_from(driver: &Driver) -> Result<Self, Self::Error> {
        if let DriverType::Framebuffer(framebuffer) = driver.vtable {
            Ok(framebuffer)
        } else {
            Err(())
        }
    }
}

impl Debug for Driver {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.name.iter().for_each(|c| {
            f.write_char(char::from(*c));
        });
        Ok(())
    }
}

pub fn drivers() -> &'static [Driver] {
    unsafe {
        let start = &__drivers_start as *const u8 as *const Driver;
        let end = &__drivers_end as *const u8 as *const Driver;
        &*slice_from_raw_parts(start, end.offset_from(start) as usize)
    }
}

pub unsafe fn drivers_mut() -> &'static mut [Driver] {
    unsafe {
        let start = &mut __drivers_start as *mut u8 as *mut Driver;
        let end = &mut __drivers_end as *mut u8 as *mut Driver;
        &mut *slice_from_raw_parts_mut(start, end.offset_from(start) as usize)
    }
}

fn init_driver(driver: &mut Driver) {
    println!("[INFO] Initializing driver \"{:?}\"", driver);
    match driver.vtable {
        DriverType::Console(console) => {
            console.write().init().unwrap();
        }
        DriverType::Framebuffer(framebuffer) => {
            framebuffer.write().init().unwrap();
        }
    }
    driver.initialized = true;
}

pub fn init_driver_by_name(name: &[u8]) {
    for driver in unsafe { drivers_mut() } {
        if driver.name == name && !driver.initialized {
            init_driver(driver);
            return;
        }
    }
}

pub fn init_all_drivers() {
    for driver in unsafe { drivers_mut() } {
        if !driver.initialized {
            init_driver(driver);
        }
    }
}

pub fn driver_by_type<'a, T: TryFrom<&'a Driver>>() -> Option<T> {
    for driver in drivers() {
        if let Ok(result) = driver.try_into() {
            return Some(result);
        }
    }
    return None;
}
