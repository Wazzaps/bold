#![allow(dead_code)]
use crate::arch::aarch64::mmio::{
    mmio_read, mmio_write, RNG_CTRL, RNG_DATA, RNG_INT_MASK, RNG_STATUS,
};

pub unsafe fn init() {
    mmio_write(RNG_STATUS, 0x40000);

    // mask interrupt
    mmio_write(RNG_INT_MASK, mmio_read(RNG_INT_MASK) | 1);

    // enable
    mmio_write(RNG_CTRL, mmio_read(RNG_CTRL) | 1);
}

pub unsafe fn get() -> u32 {
    while ((mmio_read(RNG_STATUS)) >> 24) == 0 {}
    mmio_read(RNG_DATA)
}
