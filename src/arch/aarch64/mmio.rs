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

/// Controls actuation of pull up/down to ALL GPIO pins.
pub const GPPUD: u32 = GPIO_BASE + 0x94;

/// Controls actuation of pull up/down for specific GPIO pin.
pub const GPPUDCLK0: u32 = GPIO_BASE + 0x98;

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

/// This bit is set in the status register if there is no space to write into the mailbox
pub const MAIL_FULL: u32 = 0x80000000;

/// This bit is set in the status register if there is nothing to read from the mailbox
pub const MAIL_EMPTY: u32 = 0x40000000;

pub unsafe fn mmio_read(addr: u32) -> u32 {
    (addr as usize as *const u32).read_volatile()
}

pub unsafe fn mmio_write(addr: u32, value: u32) {
    (addr as usize as *mut u32).write_volatile(value);
}

pub unsafe fn mailbox_write(data: u32) {
    while mmio_read(MBOX_STATUS) & MAIL_FULL != 0 {}
    mmio_write(MBOX_WRITE, data);
}

pub unsafe fn mailbox_read(channel: u32) -> u32 {
    loop {
        while (mmio_read(MBOX_STATUS) & MAIL_EMPTY) == 0 {}
        let val = mmio_read(MBOX_READ);
        if val & 0xF == channel {
            return val & !0xF;
        }
    }
}

#[inline(always)]
pub fn delay(count: i32) {
    // SAFETY: No memory accesses are made, this is simply a count-down
    unsafe {
        asm!(
            "1:",
            "sub {count:x}, {count:x}, #1",
            "bne 1b",
            count = in(reg) count
        );
    }
}