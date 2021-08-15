use crate::arch::aarch64::uart1::write_uart1;
use crate::driver_manager::{drivers, DeviceType};
use crate::fi;
use crate::println;
use crate::{ipc, ErrWarn};

use alloc::string::ToString;
use core::fmt;
use core::fmt::Formatter;
use core::mem::size_of;
use spin::RwLock;

pub static MAIN_CONSOLE: RwLock<Option<&'static fi::FileInterface>> = RwLock::new(None);

pub fn set_main_console_by_name(name: &[u8]) {
    for driver in drivers() {
        if driver.info().name == name {
            if let Some(interface) = driver.info().device_by_type(DeviceType::Console) {
                MAIN_CONSOLE.write().replace(interface);
                return;
            }
        }
    }
    panic!("Couldn't find the requested console driver: {:?}", name);
}

/// Like the `print!` macro in the standard library, but prints to the UART.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

/// Like the `println!` macro in the standard library, but prints to the UART.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Like the `print!` macro in the standard library, but prints to the UART.
#[macro_export]
#[cfg(wtf_prints)]
macro_rules! wtf {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

/// Like the `println!` macro in the standard library, but prints to the UART.
#[macro_export]
#[cfg(wtf_prints)]
macro_rules! wtfln {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Like the `print!` macro in the standard library, but prints to the UART.
#[macro_export]
#[cfg(not(wtf_prints))]
macro_rules! wtf {
    ($($arg:tt)*) => {};
}

/// Like the `println!` macro in the standard library, but prints to the UART.
#[macro_export]
#[cfg(not(wtf_prints))]
macro_rules! wtfln {
    () => {};
    ($($arg:tt)*) => {};
}

/// Like the `write!` macro in the standard library, but prints to the queue.
#[macro_export]
macro_rules! queue_write {
    ($fmt: expr, $($arg:tt)*) => ($crate::console::_print_queue($fmt, format_args!($($arg)*)));
}

/// Like the `writeln!` macro in the standard library, but prints to the queue.
#[macro_export]
macro_rules! queue_writeln {
    ($fmt: expr) => ($crate::queue_write!($fmt, "\r\n"));
    ($fmt: expr, $($arg:tt)*) => ($crate::queue_write!($fmt, "{}\n", format_args!($($arg)*)));
}

struct FmtWriteAdapter<'a>(&'a (dyn fi::SyncWrite));
struct FmtWriteAdapter2;

impl fmt::Write for FmtWriteAdapter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

impl fmt::Write for FmtWriteAdapter2 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_uart1(s.as_bytes())
            .map(|_| ())
            .map_err(|_| fmt::Error)
    }
}

/// Prints the given formatted string to the UART.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;

    let _ = FmtWriteAdapter2.write_fmt(args);
    // if let Some(console) = MAIN_CONSOLE.read().as_deref() {
    //     // Ignore return code
    //     let _ = FmtWriteAdapter(console.sync_write.unwrap()).write_fmt(args);
    // }
}

struct FmtQueueWriteAdapter(ipc::IpcRef);

impl fmt::Write for FmtQueueWriteAdapter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let queue = self.0.clone();
        let s = s.to_string();
        queue
            .queue_write(s.as_bytes())
            .map_err(|_| fmt::Error)
            .warn();
        Ok(())
    }
}

/// Prints the given formatted string to the queue.
#[doc(hidden)]
pub fn _print_queue(queue: ipc::IpcRef, args: fmt::Arguments) {
    use core::fmt::Write;

    // Ignore return code
    let _ = FmtQueueWriteAdapter(queue).write_fmt(args);
}

pub fn dump_hex<T>(val: &T) {
    let size = size_of::<T>() as isize;
    let val = val as *const T as *const u8;
    for i in 0..size {
        unsafe {
            print!("{:02x}", *val.offset(i));
        }
        if i % 4 == 3 {
            print!(" ");
        }
        if i % 32 == 31 {
            println!();
        }
    }
    println!();
}

pub fn dump_hex_slice(val: &[u8]) {
    for (i, byte) in val.iter().enumerate() {
        print!("{:02x}", byte);
        if i % 4 == 3 {
            print!(" ");
        }
        if i % 32 == 31 {
            println!();
        }
    }
    println!();
}

pub struct Freq(pub u64);

impl fmt::Display for Freq {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.0 < 1000 {
            write!(f, "{}Hz", self.0)
        } else if self.0 < 1000000 {
            write!(f, "{}KHz", self.0 / 1000)
        } else if self.0 < 1000000000 {
            write!(f, "{}MHz", self.0 / 1000000)
        } else if self.0 < 1000000000000 {
            write!(f, "{}GHz", self.0 / 1000000)
        } else {
            write!(f, "{}THz", self.0 / 1000000000)
        }
    }
}
