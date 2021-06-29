use crate::arch::aarch64::mmio::{mmio_read, mmio_write, UART0_DR, UART0_FR};
use crate::driver_manager::{DeviceType, DriverInfo};
use crate::file_interface::IoResult;
use crate::{driver_manager, fi};
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
                read: None,
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

impl fi::Write for Device {
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

static DEVICE: Device = Device;
