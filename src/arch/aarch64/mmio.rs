use crate::ktask;

/// board type, raspi3
pub const RASPI: u32 = 3;

/*
MMIO_BASE = 0x3F000000, // for raspi2 & 3
MMIO_BASE = 0xFE000000, // for raspi4
MMIO_BASE = 0x20000000, // for raspi1, raspi zero etc.
*/
/// The MMIO area base address.
pub const MMIO_BASE: u32 = 0x3F000000;

/// The offsets for reach register.
pub const GPIO_BASE: u32 = MMIO_BASE + 0x200000;

pub const GPFSEL0: u32 = GPIO_BASE + 0x00;
pub const GPFSEL1: u32 = GPIO_BASE + 0x04;
pub const GPFSEL2: u32 = GPIO_BASE + 0x08;
pub const GPFSEL3: u32 = GPIO_BASE + 0x0c;
pub const GPFSEL4: u32 = GPIO_BASE + 0x10;
pub const GPFSEL5: u32 = GPIO_BASE + 0x14;

pub const GPSET0: u32 = GPIO_BASE + 0x1c;
pub const GPSET1: u32 = GPIO_BASE + 0x20;

pub const GPCLR0: u32 = GPIO_BASE + 0x28;

pub const GPLEV0: u32 = GPIO_BASE + 0x34;
pub const GPLEV1: u32 = GPIO_BASE + 0x38;

pub const GPEDS0: u32 = GPIO_BASE + 0x40;
pub const GPEDS1: u32 = GPIO_BASE + 0x44;

pub const GPHEN0: u32 = GPIO_BASE + 0x64;
pub const GPHEN1: u32 = GPIO_BASE + 0x68;

/// Controls actuation of pull up/down to ALL GPIO pins.
pub const GPPUD: u32 = GPIO_BASE + 0x94;

/// Controls actuation of pull up/down for specific GPIO pin.
pub const GPPUDCLK0: u32 = GPIO_BASE + 0x98;
pub const GPPUDCLK1: u32 = GPIO_BASE + 0x9C;

/*
for raspi4 0xFE201000, ras/pi2 & 3 0x3F201000, and 0x20201000 for raspi1
*/
/// The base address for UART.
pub const UART0_BASE: u32 = GPIO_BASE + 0x1000;

/// The offsets for reach register for the UART.
pub const UART0_DR: u32 = UART0_BASE + 0x00;
pub const UART0_RSRECR: u32 = UART0_BASE + 0x04;
pub const UART0_FR: u32 = UART0_BASE + 0x18;
pub const UART0_ILPR: u32 = UART0_BASE + 0x20;
pub const UART0_IBRD: u32 = UART0_BASE + 0x24;
pub const UART0_FBRD: u32 = UART0_BASE + 0x28;
pub const UART0_LCRH: u32 = UART0_BASE + 0x2C;
pub const UART0_CR: u32 = UART0_BASE + 0x30;
pub const UART0_IFLS: u32 = UART0_BASE + 0x34;
pub const UART0_IMSC: u32 = UART0_BASE + 0x38;
pub const UART0_RIS: u32 = UART0_BASE + 0x3C;
pub const UART0_MIS: u32 = UART0_BASE + 0x40;
pub const UART0_ICR: u32 = UART0_BASE + 0x44;
pub const UART0_DMACR: u32 = UART0_BASE + 0x48;
pub const UART0_ITCR: u32 = UART0_BASE + 0x80;
pub const UART0_ITIP: u32 = UART0_BASE + 0x84;
pub const UART0_ITOP: u32 = UART0_BASE + 0x88;
pub const UART0_TDR: u32 = UART0_BASE + 0x8C;

/// The offsets for Mailbox registers
pub const MBOX_BASE: u32 = MMIO_BASE + 0xB880;
pub const MBOX_READ: u32 = MBOX_BASE + 0x00;
pub const MBOX_STATUS: u32 = MBOX_BASE + 0x18;
pub const MBOX_WRITE: u32 = MBOX_BASE + 0x20;

pub const SYSTMR_LO: u32 = MBOX_BASE + 0x3004;
pub const SYSTMR_HI: u32 = MBOX_BASE + 0x3008;

pub const RNG_CTRL: u32 = MBOX_BASE + 0x00104000;
pub const RNG_STATUS: u32 = MBOX_BASE + 0x00104004;
pub const RNG_DATA: u32 = MBOX_BASE + 0x00104008;
pub const RNG_INT_MASK: u32 = MBOX_BASE + 0x00104010;

pub unsafe fn mmio_read(addr: u32) -> u32 {
    (addr as usize as *const u32).read_volatile()
}

pub unsafe fn mmio_write(addr: u32, value: u32) {
    (addr as usize as *mut u32).write_volatile(value);
}

#[inline(always)]
pub fn delay(count: i32) {
    // SAFETY: No memory accesses are made, this is simply a count-down
    unsafe {
        asm!(
            "1:",
            "subs {count:x}, {count:x}, #1",
            "bne 1b",
            count = inout(reg) count => _,
            options(nomem, nostack)
        );
    }
}

// Unimplemented in QEMU
pub fn get_system_timer() -> u64 {
    unsafe {
        let hi = mmio_read(SYSTMR_HI);
        let lo = mmio_read(SYSTMR_LO);

        ((hi as u64) << 32) | lo as u64
    }
}

pub fn delay_us_sync(time: u64) {
    unsafe {
        let mut freq: u64 = 0;
        let mut counter: u64 = 0;
        asm!(
            "mrs {freq}, cntfrq_el0",
            "mrs {counter}, cntpct_el0",
            freq = out(reg) freq,
            counter = out(reg) counter,
            options(nomem, nostack)
        );

        let expires_at = counter + ((freq / 1000) * time) / 1000;
        loop {
            asm!(
                "mrs {counter}, cntpct_el0",
                counter = out(reg) counter,
                options(nomem, nostack)
            );
            if counter >= expires_at {
                break;
            }
        }
    }
}

pub async fn delay_us(time: u64) {
    unsafe {
        let mut freq: u64 = 0;
        let mut counter: u64 = 0;
        asm!(
        "mrs {freq}, cntfrq_el0",
        "mrs {counter}, cntpct_el0",
        freq = out(reg) freq,
        counter = out(reg) counter,
        options(nomem, nostack)
        );

        let expires_at = counter + ((freq / 1000) * time) / 1000;
        loop {
            asm!(
            "mrs {counter}, cntpct_el0",
            counter = out(reg) counter,
            options(nomem, nostack)
            );
            if counter >= expires_at {
                break;
            }
            ktask::yield_now().await;
        }
    }
}

pub fn get_uptime_us() -> u64 {
    unsafe {
        let mut freq: u64 = 0;
        let mut counter: u64 = 0;
        asm!(
            "mrs {freq}, cntfrq_el0",
            "mrs {counter}, cntpct_el0",
            freq = out(reg) freq,
            counter = out(reg) counter,
            options(nomem, nostack)
        );

        if freq == 0 {
            0
        } else {
            counter * 1000 * 1000 / freq
        }
    }
}
