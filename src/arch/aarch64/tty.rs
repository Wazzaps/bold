use core::fmt;
use crate::arch::aarch64::uart::RASPI_UART;
use core::mem::size_of;

/// Like the `print!` macro in the standard library, but prints to the UART.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::arch::aarch64::tty::_print(format_args!($($arg)*)));
}

/// Like the `println!` macro in the standard library, but prints to the UART.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Prints the given formatted string to the UART.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;

    let _ = RASPI_UART.lock().write_fmt(args);

    // interrupts::without_interrupts(|| {
    //     MUXWRITER1.lock().write_fmt(args).unwrap();
    // });
}

pub fn dump_hex<T>(val: &T) {
    let size = size_of::<T>();
    let val = val as *const T as *const u8;
    for i in 0..size {
        unsafe {
            print!("{:02x}", *val.offset(i as isize));
        }
        if i % 4 == 3 {
            print!(" ");
        }
    }
    println!();
}