#![feature(lang_items)]
#![feature(asm)]
#![feature(panic_info_message)]
#![no_builtins]
#![no_std]

use crate::arch::aarch64::{uart, framebuffer};

mod lang_items;
pub(crate) mod arch;

#[no_mangle]
pub unsafe extern "C" fn kmain() {
    println!("Initializing FB");
    framebuffer::init();
    println!("Initialized");
    framebuffer::draw_example();
    println!("Draw ok");
}
