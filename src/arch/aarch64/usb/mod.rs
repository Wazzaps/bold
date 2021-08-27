mod hcd;
mod regs;

use crate::prelude::*;

pub unsafe fn init() {
    hcd::init().expect("Failed to init hcd");
    hcd::start().expect("Failed to start hcd");
    println!("[INFO] USB up!");
}
