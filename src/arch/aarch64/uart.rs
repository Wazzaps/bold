use crate::arch::aarch64::mmio::{
    delay, mmio_read, mmio_write, GPPUD, GPPUDCLK0, RASPI, UART0_CR, UART0_DR, UART0_FBRD,
    UART0_FR, UART0_IBRD, UART0_ICR, UART0_IMSC, UART0_LCRH,
};
use crate::arch::aarch64::{mailbox, mailbox_methods};
use crate::console::Console;
use crate::driver_manager::{Driver, DriverType};
use crate::println;
use core::fmt;
use core::marker::PhantomData;
use spin::{Mutex, RwLock};

#[derive(Debug)]
pub struct RaspberryPiUART0;

impl Console for RaspberryPiUART0 {
    fn init(&mut self) -> Result<(), ()> {
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
                #[repr(align(16), C)]
                struct MBoxMsg([u32; 9]);

                // UART_CLOCK = 30000000
                // A Mailbox message with set clock rate of PL011 to 3MHz tag
                // println!("Previous clock: {}", mailbox::get_clock_rate(2).unwrap());
                mailbox_methods::set_clock_rate(2, 3000000, false).unwrap();
                // println!("New clock: {}", mailbox::get_clock_rate(2).unwrap());
                // let mut mbox = MBoxMsg([9 * 4, 0, 0x38002, 12, 8, 2, 3000000, 0, 0]);
                // mailbox::write_raw(((&mut mbox as *mut MBoxMsg as *mut u8 as usize as u32) & !0xF) | 8);
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
        Ok(())
    }

    fn as_byte_writer(&mut self) -> &mut dyn genio::Write<WriteError = (), FlushError = ()> {
        self
    }
}

impl genio::Write for RaspberryPiUART0 {
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

static INSTANCE_RASPI_UART0: RwLock<RaspberryPiUART0> = RwLock::new(RaspberryPiUART0);

#[link_section = ".drivers"]
#[used]
static mut DRIVER_RASPI_UART0: Driver = Driver {
    name: b"Raspberry Pi 3 UART0",
    initialized: false,
    vtable: DriverType::Console(&INSTANCE_RASPI_UART0),
};
