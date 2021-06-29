use core::fmt;
use core::fmt::Write;

pub fn display_bstr(fmt: &mut fmt::Formatter<'_>, bstr: &[u8]) -> fmt::Result {
    bstr.iter().try_for_each(|c| fmt.write_char(char::from(*c)))
}
