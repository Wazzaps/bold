use crate::prelude::*;

#[no_mangle]
pub unsafe fn exception_handler(etype: u64, esr: u64, elr: u64, spsr: u64, far: u64) -> ! {
    println!("Exception SPSel: {}", (get_msr!(SPSel) >> 2) & 0b11);
    panic!(
        "Exception:\netype=0x{:x} esr=0x{:x} elr=0x{:x} spsr=0x{:x} far=0x{:x}",
        etype, esr, elr, spsr, far
    );
}

#[repr(C)]
pub struct ExceptionContext {
    /// General Purpose Registers.
    pub gpr: [u64; 30],

    /// The link register, aka x30.
    pub lr: u64,

    /// Exception link register. The program counter at the time the exception happened.
    pub elr_el1: u64,

    /// Saved program status.
    pub spsr_el1: u64,
}

#[no_mangle]
pub unsafe fn exception_handler2(e: &mut ExceptionContext) {
    println!("-------------------------------------------");
    // let sp = (e as *const ExceptionContext as *const u8)
    //     .offset(size_of::<ExceptionContext>() as isize) as *const u64;
    println!("Registers:");
    for reg in e.gpr {
        print!("{:016x} ", reg);
    }
    println!();
    println!("Exception reason: 0x{:x}", get_msr!(esr_el1));
    println!("FAR (Address accessed): 0x{:x}", get_msr!(far_el1));
    println!("PC: 0x{:x}", e.elr_el1);
    println!("LR: 0x{:x}", e.lr);
    println!("SP: {:016x}", get_msr!(sp_el0));
    println!("SPSR: 0x{:x}", e.spsr_el1);
    println!("-------------------------------------------");

    if get_msr!(esr_el1) == 0x96000045 {
        e.elr_el1 += 4;
    } else {
        loop {
            asm!("wfi");
        }
    }
}

#[no_mangle]
pub unsafe fn irq_handler(e: &mut ExceptionContext) {
    crate::arch::aarch64::interrupts::handle_irq(e);
}
