use crate::arch::aarch64::mmio::{
    delay, mmio_read, mmio_write, GPFSEL1, GPPUD, GPPUDCLK0, RASPI, UART0_CR, UART0_DR, UART0_FBRD,
    UART0_FR, UART0_IBRD, UART0_ICR, UART0_IMSC, UART0_LCRH,
};
use crate::driver_manager::{DeviceType, DriverInfo};
use crate::ipc;
use crate::prelude::*;

use crate::arch::aarch64::mailbox_methods;
use crate::{driver_manager, fi, ktask};
use core::cell::UnsafeCell;
use spin::RwLock;

// ----- Driver -----

#[derive(Debug)]
struct Driver {
    info: UnsafeCell<DriverInfo>,
}

pub fn init_uart0() {
    unsafe {
        // Disable UART0.
        mmio_write(UART0_CR, 0x00000000);

        // For Raspi3 and 4 the UART_CLOCK is system-clock dependent by default.
        // Set it to 4Mhz so that we can consistently set the baud rate
        if RASPI >= 3 {
            // A Mailbox message with set clock rate of PL011 to 4MHz tag
            mailbox_methods::set_clock_rate(2, 4000000, false).unwrap();
        }

        // Map UART0 to GPIO pins
        mmio_write(
            GPFSEL1,
            mmio_read(GPFSEL1) & !((7 << 12) | (7 << 15)) | ((4 << 12) | (4 << 15)),
        );

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

        mmio_write(UART0_IBRD, 2); // 115200 baud

        mmio_write(UART0_FBRD, 0xB);

        // Enable FIFO & 8 bit data transmission (1 stop bit, no parity).
        mmio_write(UART0_LCRH, (1 << 4) | (1 << 5) | (1 << 6));

        // Mask all interrupts.
        mmio_write(
            UART0_IMSC,
            (1 << 1) | (1 << 4) | (1 << 5) | (1 << 6) | (1 << 7) | (1 << 8) | (1 << 9) | (1 << 10),
        );

        // Enable UART0, receive & transfer part of UART.
        mmio_write(UART0_CR, 0x301);
    }
}

pub fn write_uart0(buf: &[u8]) -> IoResult<usize> {
    for c in buf {
        unsafe {
            // Wait for UART to become ready to transmit.
            while mmio_read(UART0_FR) & (1 << 5) != 0 {}
            mmio_write(UART0_DR, *c as u32);
        }
    }
    Ok(buf.len())
}

impl driver_manager::Driver for Driver {
    fn init(&self) -> Result<(), ()> {
        // FIXME: Vulnerability
        unsafe {
            (*self.info.get()).initialized = true;
        }

        spawn_task!(b"UART0.input", {
            // Create the input queue
            let root = ipc::ROOT.read().as_ref().unwrap().clone();
            let input_queue = root
                .dir_get(ipc::well_known::ROOT_DEVICES)
                .await
                .unwrap()
                .dir_get(ipc::well_known::DEVICES_RPI_UART)
                .await
                .unwrap()
                .dir_get(ipc::well_known::RPI_UART0)
                .await
                .unwrap()
                .dir_link(ipc::well_known::RPI_UART_IN, ipc::IpcSpscQueue::new())
                .await
                .unwrap();

            // Write to it forever
            let mut buf = [0u8; 1];
            loop {
                if let Ok(1) = fi::Read::read(&DEVICE, &mut buf).await {
                    input_queue.queue_write(&buf).warn();
                }
                ktask::yield_now().await;
            }
        });

        spawn_task!(b"UART0.output", {
            // Create the output queue
            let root = ipc::ROOT.read().as_ref().unwrap().clone();
            let output_queue = root
                .dir_get(ipc::well_known::ROOT_DEVICES)
                .await
                .unwrap()
                .dir_get(ipc::well_known::DEVICES_RPI_UART)
                .await
                .unwrap()
                .dir_get(ipc::well_known::RPI_UART0)
                .await
                .unwrap()
                .dir_link(ipc::well_known::RPI_UART_OUT, ipc::IpcSpscQueue::new())
                .await
                .unwrap();

            // Write to it forever
            let mut buf = [0u8; 512];
            loop {
                if let Some(count) = output_queue.queue_read(&mut buf).await {
                    if count != 0 {
                        // fi::SyncWrite::write_all(&DEVICE, &buf).await.unwrap();
                        fi::SyncWrite::write_all(&DEVICE, &buf[0..count]).unwrap();
                    }
                }
                ktask::yield_now().await;
            }
        });

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
                // Poll UART0 at 120hz (1000×1000÷120 = 8333us)
                wtfln!("(UART)");
                crate::arch::aarch64::mmio::sleep_us(8333).await;
                // crate::arch::aarch64::mmio::sleep_us(1010).await;
                // yield_now().await;
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
