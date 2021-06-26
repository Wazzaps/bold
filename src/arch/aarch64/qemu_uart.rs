use crate::arch::aarch64::mmio::{mmio_read, mmio_write, UART0_DR, UART0_FR};
use crate::console::Console;
use crate::driver_manager::{Driver, DriverType};
use core::fmt;
use genio::Write;
use spin::RwLock;

#[derive(Debug)]
struct QEMURaspberryPiUART0;

impl Console for QEMURaspberryPiUART0 {
    fn init(&mut self) -> Result<(), ()> {
        Ok(())
    }

    fn as_byte_writer(&mut self) -> &mut dyn genio::Write<WriteError = (), FlushError = ()> {
        self
    }
}

impl genio::Write for QEMURaspberryPiUART0 {
    type WriteError = ();
    type FlushError = ();

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::WriteError> {
        for c in buf {
            unsafe {
                // Wait for UART to become ready to transmit.
                while mmio_read(UART0_FR) & (1 << 5) != 0 {}
                mmio_write(UART0_DR, *c as u32);
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::FlushError> {
        Ok(())
    }

    fn size_hint(&mut self, _bytes: usize) {}
}

static INSTANCE_QEMU_CONSOLE: RwLock<QEMURaspberryPiUART0> = RwLock::new(QEMURaspberryPiUART0);

#[link_section = ".drivers"]
#[used]
static mut DRIVER_QEMU_CONSOLE: Driver = Driver {
    name: b"QEMU-Only Raspberry Pi 3 UART0",
    initialized: true,
    vtable: DriverType::Console(&INSTANCE_QEMU_CONSOLE),
};
