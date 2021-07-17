use crate::println;
use core::fmt;
use core::fmt::Write;

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
    bstr.iter().try_for_each(|c| fmt.write_char(char::from(*c)))
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
