use crate::prelude::*;
use core::convert::TryFrom;

#[no_mangle]
pub unsafe fn exception_handler(etype: u64, esr: u64, elr: u64, spsr: u64, far: u64) -> ! {
    println!("Exception SPSel: {}", (get_msr!(SPSel) >> 2) & 0b11);
    panic!(
        "Exception:\netype=0x{:x} esr=0x{:x} elr=0x{:x} spsr=0x{:x} far=0x{:x}",
        etype, esr, elr, spsr, far
    );
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ExceptionContext {
    /// General Purpose Registers.
    pub gpr: [u64; 30],

    /// The link register, aka x30.
    pub lr: u64,

    /// Exception link register. The program counter at the time the exception happened.
    pub pc: u64,

    /// Saved program status.
    pub spsr: u64,

    /// The stack pointer.
    pub sp: u64,
}

#[allow(dead_code, unused_variables)]
unsafe fn print_stacktrace(e: &mut ExceptionContext) {
    println!("@@@@");
    let sp = get_msr!(sp_el0);
    println!("- PC: 0x{:x} ", e.pc);
    println!("- LR: 0x{:x} ", e.lr);
    for i in 0..128 {
        let val = *(sp as *const u64).offset(i);
        if (0xffffff8000080000..=0xffffff8000080000 + 0x03f400).contains(&val) {
            println!("- {}: 0x{:x} ", i, val);
        }
    }
    println!("@@@@");
}

#[no_mangle]
pub unsafe extern "C" fn exception_handler2(e: &mut ExceptionContext) {
    if get_msr!(esr_el1) == 0x56000000 {
        match crate::syscalls::Syscall::try_from(e.gpr[8]) {
            Ok(syscall_no) => {
                crate::syscalls::handle_syscall(e, syscall_no);
            }
            Err(_) => {
                // Unknown syscall, return error
                println!("[WARN] Called unknown syscall 0x{:x}", e.gpr[0]);
                e.gpr[0] = u64::MAX;
            }
        }
        return;
    }

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
    println!("PC: 0x{:x}", e.pc);
    println!("LR: 0x{:x}", e.lr);
    println!("SP: 0x{:016x}", e.sp);
    println!("SPSR: 0x{:x}", e.spsr);
    println!("-------------------------------------------");
    print_stacktrace(e);

    loop {
        asm!("wfi");
    }
}

#[no_mangle]
pub unsafe extern "C" fn irq_handler(e: &mut ExceptionContext) {
    crate::arch::aarch64::interrupts::handle_irq(e);
}
