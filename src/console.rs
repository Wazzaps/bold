use crate::driver_manager::{drivers, DeviceType};
use crate::fi;
use crate::println;
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
    ($($arg:tt)*) => ($crate::print!("{}\r\n", format_args!($($arg)*)));
}

struct FmtWriteAdapter<'a>(&'a (dyn fi::SyncWrite));

impl fmt::Write for FmtWriteAdapter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

/// Prints the given formatted string to the UART.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;

    if let Some(console) = MAIN_CONSOLE.read().as_deref() {
        // Ignore return code
        let _ = FmtWriteAdapter(console.sync_write.unwrap()).write_fmt(args);
    }
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
