use crate::driver_manager::{drivers, Driver, DriverType};
use core::fmt;
use core::fmt::Debug;
use core::mem::size_of;
use core::ops::DerefMut;
use spin::{Mutex, RwLock};

static MAIN_CONSOLE: Mutex<
    Option<&'static RwLock<dyn Console<WriteError = (), FlushError = ()> + Send + Sync>>,
> = Mutex::new(None);

#[derive(Debug)]
struct MainConsole;

impl Console for MainConsole {
    fn init(&mut self) -> Result<(), ()> {
        Ok(())
    }

    fn as_byte_writer(&mut self) -> &mut dyn genio::Write<WriteError = (), FlushError = ()> {
        self
    }
}

impl genio::Write for MainConsole {
    type WriteError = ();
    type FlushError = ();

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::WriteError> {
        match MAIN_CONSOLE.lock().as_mut() {
            None => Err(()),
            Some(console) => console.write().write(buf),
        }
    }

    fn flush(&mut self) -> Result<(), Self::FlushError> {
        match MAIN_CONSOLE.lock().as_mut() {
            None => Err(()),
            Some(console) => console.write().flush(),
        }
    }

    fn size_hint(&mut self, _bytes: usize) {}
}

pub fn set_main_console_by_name(name: &[u8]) {
    for driver in drivers() {
        if driver.name == name {
            if let DriverType::Console(console) = driver.vtable {
                MAIN_CONSOLE.lock().replace(console);
                return;
            }
        }
    }
    panic!("Couldn't find the requested console driver: {:?}", name);
}

pub trait Console: genio::Write<WriteError = (), FlushError = ()> + Debug {
    fn init(&mut self) -> Result<(), ()>;
    fn as_byte_writer(&mut self) -> &mut dyn genio::Write<WriteError = (), FlushError = ()>;
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

struct FmtWriteAdapter<'a>(&'a mut dyn genio::Write<WriteError = (), FlushError = ()>);

impl fmt::Write for FmtWriteAdapter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_all(s.as_bytes());
        Ok(())
    }
}

/// Prints the given formatted string to the UART.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;

    if let Some(console) = MAIN_CONSOLE.lock().as_mut() {
        FmtWriteAdapter(console.write().as_byte_writer().deref_mut()).write_fmt(args);
    }
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
        if i % 64 == 63 {
            print!("\n");
        }
    }
    println!();
}
