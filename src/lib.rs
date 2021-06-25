#![feature(lang_items)]
#![feature(asm)]
#![feature(panic_info_message)]
#![no_builtins]
#![no_std]

use crate::arch::aarch64::uart;

mod lang_items;
pub(crate) mod arch;

#[no_mangle]
pub unsafe extern "C" fn kmain() {
    println!("Hello, {}!", 5);
}
