use core::convert::TryInto;

use crate::println;

pub fn parse_tar(start_addr: u32, end_addr: u32) {
    let data = unsafe {
        core::slice::from_raw_parts(
            start_addr as *const u8,
            (end_addr - start_addr).try_into().unwrap(),
        )
    };

    match tar::parse_tar(data) {
        Ok((_, entries)) => {
            for e in entries.iter() {
                println!("{:?}", e);
            }
        }
        Err(e) => {
            println!("error or incomplete: {:?}", e);
            panic!("cannot parse tar archive");
        }
    }
}
