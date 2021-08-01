pub(crate) mod entropy;
pub(crate) mod exceptions;
pub(crate) mod framebuffer;
mod init;
pub(crate) mod mailbox;
pub(crate) mod mailbox_methods;
pub(crate) mod mmio;
pub(crate) mod mmu;
pub(crate) mod phymem;
// pub(crate) mod qemu_uart;
pub(crate) mod sdhc;
// pub(crate) mod uart;
pub(crate) mod dtb;
pub(crate) mod uart1;
pub(crate) mod virtmem;

#[macro_export]
macro_rules! set_msr {
    ($name: ident, $value: expr) => {
        asm!(
            concat!("msr ", stringify!($name), ", {:x}"),
            in(reg) $value,
            options(nomem, nostack)
        );
    };
}

#[macro_export]
macro_rules! get_msr {
    ($name: ident) => {{
        let val: u64;
        asm!(
            concat!("mrs {:x}, ", stringify!($name)),
            out(reg) val,
            options(nomem, nostack)
        );
        val
    }};
}
