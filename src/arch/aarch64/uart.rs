use crate::arch::aarch64::mailbox_methods;
use crate::arch::aarch64::mmio::{
    delay, mmio_read, mmio_write, GPPUD, GPPUDCLK0, RASPI, UART0_CR, UART0_DR, UART0_FBRD,
    UART0_FR, UART0_IBRD, UART0_ICR, UART0_IMSC, UART0_LCRH,
};
use crate::driver_manager::{DeviceType, DriverInfo};
use crate::prelude::*;
use crate::{driver_manager, fi, ktask};
use core::cell::UnsafeCell;
use spin::RwLock;

// ----- Driver -----

#[derive(Debug)]
struct Driver {
    info: UnsafeCell<DriverInfo>,
}

impl driver_manager::Driver for Driver {
    fn init(&self) -> Result<(), ()> {
        unsafe {
            // Disable UART0.
            mmio_write(UART0_CR, 0x00000000);
            // Setup the GPIO pin 14 && 15.

            // Disable pull up/down for all GPIO pins & delay for 150 cycles.
            mmio_write(GPPUD, 0x00000000);
            delay(150);

            // Disable pull up/down for pin 14,15 & delay for 150 cycles.
            mmio_write(GPPUDCLK0, (1 << 14) | (1 << 15));
            delay(150);

            // Write 0 to GPPUDCLK0 to make it take effect.
            mmio_write(GPPUDCLK0, 0x00000000);

            // Clear pending interrupts.
            mmio_write(UART0_ICR, 0x7FF);

            // Set integer & fractional part of baud rate.
            // Divider = UART_CLOCK/(16 * Baud)
            // Fraction part register = (Fractional part * 64) + 0.5
            // Baud = 115200.

            // For Raspi3 and 4 the UART_CLOCK is system-clock dependent by default.
            // Set it to 3Mhz so that we can consistently set the baud rate
            if RASPI >= 3 {
                // A Mailbox message with set clock rate of PL011 to 3MHz tag
                mailbox_methods::set_clock_rate(2, 3000000, false).unwrap();
            }

            // Divider = 3000000 / (16 * 115200) = 1.627 = ~1.
            mmio_write(UART0_IBRD, 1);
            // Fractional part register = (.627 * 64) + 0.5 = 40.6 = ~40.
            mmio_write(UART0_FBRD, 40);

            // Enable FIFO & 8 bit data transmission (1 stop bit, no parity).
            mmio_write(UART0_LCRH, (1 << 4) | (1 << 5) | (1 << 6));

            // Mask all interrupts.
            mmio_write(
                UART0_IMSC,
                (1 << 1)
                    | (1 << 4)
                    | (1 << 5)
                    | (1 << 6)
                    | (1 << 7)
                    | (1 << 8)
                    | (1 << 9)
                    | (1 << 10),
            );

            // Enable UART0, receive & transfer part of UART.
            mmio_write(UART0_CR, (1 << 0) | (1 << 8) | (1 << 9));
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
        name: b"Raspberry Pi 3 UART0",
        initialized: false,
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
    }),
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
