use crate::arch::aarch64::exceptions::ExceptionContext;
use crate::prelude::*;
use core::ops::Deref;
use core::ptr::slice_from_raw_parts;
use num_enum::TryFromPrimitive;

#[repr(u64)]
#[derive(TryFromPrimitive)]
pub enum Syscall {
    Exit = 0,
    KLogWrite = 1,
}

unsafe fn cstr_ptr_to_asciistr(ptr: *const u8) -> AsciiStr<'static> {
    let mut length = 0;
    for i in 0.. {
        if *ptr.offset(i) == 0 {
            length = i;
            break;
        }
    }

    let slice = &*slice_from_raw_parts(ptr, length as usize);
    AsciiStr(slice)
}

pub unsafe fn handle_syscall(e: &mut ExceptionContext, syscall_no: Syscall) {Usermode
    match syscall_no {
        Syscall::Exit => {
            unimplemented!();
        }
        Syscall::KLogWrite => {
            let format = cstr_ptr_to_asciistr(e.gpr[1] as *const u8);
            println!("[UM] {}", format);
        }
    }
}

pub async fn usermode() {
    // Prepare code
    const CODE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/example_app.bin"));
    const CODE_LENGTH: usize = CODE_BYTES.len();

    #[repr(align(4096))]
    struct PageAligned<const LEN: usize>([u8; LEN]);

    impl<const LEN: usize> Deref for PageAligned<LEN> {
        type Target = [u8; LEN];

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[no_mangle]
    #[link_section = ".text"]
    static CODE: PageAligned<CODE_LENGTH> = PageAligned(*include_bytes!(concat!(
        env!("OUT_DIR"),
        "/example_app.bin"
    )));

    // TODO: Prepare page tables

    // TODO: Prepare stack
    // let stack = Box::new(PageAligned([0u8; 4096 * 8]));

    let code: extern "C" fn() = unsafe { core::mem::transmute(&*CODE as *const _) };
    (code)();
}
