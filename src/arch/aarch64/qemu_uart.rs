use crate::arch::aarch64::mmio::{mmio_read, mmio_write, UART0_DR, UART0_FR};
use crate::driver_manager::{DeviceType, DriverInfo};
use crate::prelude::*;
use crate::{driver_manager, fi, ktask};
use spin::RwLock;

// ----- Driver -----

#[derive(Debug)]
struct Driver {
    info: DriverInfo,
}

impl driver_manager::Driver for Driver {
    fn info(&'static self) -> &'static DriverInfo {
        &self.info
    }
}

static mut DRIVER: Driver = Driver {
    info: DriverInfo {
        name: b"QEMU-Only Raspberry Pi 3 UART0",
        initialized: true,
        devices: RwLock::new([driver_manager::Device {
            device_type: DeviceType::Console,
            interface: fi::FileInterface {
                sync_read: Some(&DEVICE),
                read: Some(&DEVICE),
                sync_write: Some(&DEVICE),
                write: Some(&DEVICE),
                ctrl: None,
            },
        }]),
    },
};

#[link_section = ".drivers"]
#[used]
static mut DRIVER_REF: &dyn driver_manager::Driver = unsafe { &DRIVER };

// ----- Device -----

#[derive(Debug)]
struct Device;

#[async_trait]
impl fi::Write for Device {
    async fn write(&self, buf: &[u8]) -> IoResult<usize> {
        for c in buf {
            unsafe {
                // Wait for UART to become ready to transmit.
                while mmio_read(UART0_FR) & (1 << 5) != 0 {
                    ktask::yield_now().await;
                }
                mmio_write(UART0_DR, *c as u32);
            }
        }
        Ok(buf.len())
    }
}

#[async_trait]
impl fi::Read for Device {
    async fn read(&self, buf: &mut [u8]) -> IoResult<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        unsafe {
            // Wait for UART to become ready to receive.
            while mmio_read(UART0_FR) & (1 << 4) != 0 {
                ktask::yield_now().await;
            }
            buf[0] = mmio_read(UART0_DR) as u8;
        }
        Ok(1)
    }
}

impl fi::SyncWrite for Device {
    fn write(&self, buf: &[u8]) -> IoResult<usize> {
        for c in buf {
            unsafe {
                // Wait for UART to become ready to transmit.
                while mmio_read(UART0_FR) & (1 << 5) != 0 {}
                mmio_write(UART0_DR, *c as u32);
            }
        }
        Ok(buf.len())
    }
}

impl fi::SyncRead for Device {
    fn read(&self, buf: &mut [u8]) -> IoResult<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        unsafe {
            // Wait for UART to become ready to receive.
            while mmio_read(UART0_FR) & (1 << 4) != 0 {}
            buf[0] = mmio_read(UART0_DR) as u8;
        }
        Ok(1)
    }
}

static DEVICE: Device = Device;
