use crate::fi;
use crate::prelude::*;
use core::fmt;
use core::fmt::{Debug, Formatter};
use core::mem;
use core::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut};
use spin::RwLock;

extern "C" {
    static mut __drivers_start: u8;
    static mut __drivers_end: u8;
}

pub trait Driver {
    fn early_init(&self) -> Result<(), ()> {
        Ok(())
    }

    fn init(&self) -> Result<(), ()> {
        Ok(())
    }

    // FIXME: Once allocator works, change devices to be refcounted and remove 'static lifetime
    fn info(&'static self) -> &'static DriverInfo;
}

impl Debug for &'static dyn Driver {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // FIXME: Vulnerability
        let self_static: &'static Self = unsafe { mem::transmute(self) };
        self_static.info().fmt(f)
    }
}

impl Debug for &'static mut dyn Driver {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // FIXME: Vulnerability
        let self_static: &'static Self = unsafe { mem::transmute(self) };
        self_static.info().fmt(f)
    }
}

pub struct DriverInfo {
    pub name: &'static [u8],
    pub initialized: bool,
    // pub devices: RwLock<ArrayVec<Device, 4>>,
    pub devices: RwLock<[Device; 1]>,
}

impl DriverInfo {
    pub fn device_by_type(
        &'static self,
        device_type: DeviceType,
    ) -> Option<&'static fi::FileInterface> {
        for dev in self.devices.read().iter() {
            if dev.device_type == device_type {
                let interface: &'static fi::FileInterface =
                    unsafe { mem::transmute(&dev.interface) };
                return Some(interface);
            }
        }
        None
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DeviceType {
    Console,
    Framebuffer,
}

pub struct Device {
    pub device_type: DeviceType,
    pub interface: fi::FileInterface,
}

impl Debug for DriverInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        display_bstr(f, self.name)
    }
}

pub fn drivers() -> &'static [&'static dyn Driver] {
    unsafe {
        let start = &__drivers_start as *const u8 as *const &dyn Driver;
        let end = &__drivers_end as *const u8 as *const &dyn Driver;
        &*slice_from_raw_parts(start, end.offset_from(start) as usize)
    }
}

pub unsafe fn drivers_mut() -> &'static mut [&'static mut dyn Driver] {
    let start = &mut __drivers_start as *mut u8 as *mut &mut dyn Driver;
    let end = &mut __drivers_end as *mut u8 as *mut &mut dyn Driver;
    &mut *slice_from_raw_parts_mut(start, end.offset_from(start) as usize)
}

fn early_init_driver(driver: &'static dyn Driver) -> Result<(), ()> {
    println!("[INFO] Early-Initializing driver \"{:?}\"", driver);

    driver.early_init()
}

fn init_driver(driver: &'static dyn Driver) -> Result<(), ()> {
    println!("[INFO] Initializing driver \"{:?}\"", driver);

    driver.init()
}

pub fn init_driver_by_name(name: &[u8]) -> Result<(), ()> {
    for driver in drivers() {
        let info = driver.info();
        if !info.initialized && info.name == name {
            return init_driver(*driver);
        }
    }
    Err(())
}

pub unsafe fn early_init_all_drivers() {
    for driver in drivers_mut() {
        if early_init_driver(*driver).is_err() {
            println!("[EROR] Failed to initialize driver: \"{:?}\"", driver);
        }
    }
}

pub fn init_all_drivers() {
    for driver in unsafe { drivers_mut() } {
        if !driver.info().initialized && init_driver(*driver).is_err() {
            println!("[EROR] Failed to initialize driver: \"{:?}\"", driver);
        }
    }
}

pub fn device_by_type(device_type: DeviceType) -> Option<&'static fi::FileInterface> {
    for driver in drivers() {
        if driver.info().initialized {
            if let Some(interface) = driver.info().device_by_type(device_type) {
                return Some(interface);
            }
        }
    }
    None
}
