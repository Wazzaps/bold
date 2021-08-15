use crate::prelude::*;
use core::fmt;
use core::fmt::{Display, Formatter, Write};
use core::ops::Deref;

#[macro_export]
macro_rules! unwrap_variant {
    ($source: expr, $variant: path) => {{
        if let $variant(val) = $source {
            val
        } else {
            panic!(concat!(
                "Invalid variant in '",
                stringify!($source),
                "', expected ''",
                stringify!($variant)
            ));
        }
    }};
}

pub fn display_bstr(fmt: &mut fmt::Formatter<'_>, bstr: &[u8]) -> fmt::Result {
    bstr.iter().try_for_each(|c| match c {
        // Normal letters
        b' '..=b'~' => fmt.write_char(char::from(*c)),
        c => write!(fmt, "\\x{:02x}", *c),
    })
}

pub struct AsciiStr<'a>(pub &'a [u8]);

impl<'a> Display for AsciiStr<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        display_bstr(f, self.0)
    }
}

impl<'a> Deref for AsciiStr<'a> {
    type Target = [u8];

    fn deref(&self) -> &'a Self::Target {
        self.0
    }
}

pub struct DurationFmt(pub u64);

impl Display for DurationFmt {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "{:02}:{:02}.{:02}",
            self.0 / 60000000,
            self.0 % 60000000 / 1000000,
            self.0 % 1000000 / 10000,
        )
    }
}

impl Deref for DurationFmt {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait ErrWarn {
    fn warn(self);
}

impl<T, E: fmt::Debug> ErrWarn for Result<T, E> {
    fn warn(self) {
        match self {
            Ok(_) => {}
            Err(e) => println!(
                "[WARN] called `Result::unwrap()` on an `Err` value: {:?}",
                &e
            ),
        }
    }
}
