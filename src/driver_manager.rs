use crate::console::Console;
use crate::println;
use core::fmt;
use core::fmt::{Debug, Formatter, Write};
use core::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut};
use spin::RwLock;

extern "C" {
    static mut __drivers_start: u8;
    static mut __drivers_end: u8;
}

#[non_exhaustive]
#[derive(Debug)]
pub enum DriverType {
    Console(&'static RwLock<dyn Console + Send + Sync>),
}

pub struct Driver {
    pub name: &'static [u8],
    pub initialized: bool,
    pub vtable: DriverType,
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

pub fn init_driver_by_name(name: &[u8]) {
    for driver in unsafe { drivers_mut() } {
        if driver.name == name && !driver.initialized {
            println!("[INFO] Initializing driver \"{:?}\"", driver);
            match driver.vtable {
                DriverType::Console(console) => {
                    console.write().init().unwrap();
                }
            }
            driver.initialized = true;
            return;
        }
    }
}

pub fn init_all_drivers() {
    for driver in unsafe { drivers_mut() } {
        if !driver.initialized {
            println!("[INFO] Initializing driver \"{:?}\"", driver);
            match driver.vtable {
                DriverType::Console(console) => {
                    console.write().init().unwrap();
                }
            }
            driver.initialized = true;
        }
    }
}
