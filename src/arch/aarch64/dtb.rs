use crate::{println, AsciiStr};
use alloc::vec::Vec;
use arrayvec::ArrayVec;
use core::convert::TryInto;
use dtb::StructItem;

fn dtb_tree(reader: &dtb::Reader) {
    let mut depth = 0;
    for si in reader.struct_items() {
        match si {
            StructItem::BeginNode { name } => {
                println!("{}{}:", "| ".repeat(depth), name);
                depth += 1;
            }
            StructItem::Property { name, value } => {
                println!("{}{}: \"{}\"", "| ".repeat(depth), name, AsciiStr(value));
            }
            StructItem::EndNode => {
                depth -= 1;
            }
        }
    }
}

pub unsafe fn parse(dtb: *const u8) {
    // let dtb = dtb::Reader::read(dtb).unwrap();
    let dtb = dtb::Reader::read_from_address(dtb as usize).unwrap();
    println!(
        "{:?}",
        dtb.reserved_mem_entries().collect::<ArrayVec<_, 64>>()
    );
    // for si in dtb.struct_items() {
    //     println!("- {:?}", si);
    // }

    let mut initrd_start = 0 as u32;
    let mut initrd_end = 0 as u32;

    for si in dtb.struct_items() {
        match si {
            StructItem::BeginNode { name } => {}
            StructItem::Property { name, value } => {
                if name.contains("initrd") {
                    match name {
                        "linux,initrd-start" => {
                            initrd_start = u32::from_be_bytes(value.try_into().unwrap())
                        }
                        "linux,initrd-end" => {
                            initrd_end = u32::from_be_bytes(value.try_into().unwrap())
                        }
                        _ => {
                            panic!("unhandeled initrd property: {} = {}", name, AsciiStr(value))
                        }
                    }
                }
            }
            StructItem::EndNode => {}
        }
    }

    // println!("initrd-start: {}", initrd_start);
    // println!("initrd-end: {}", initrd_end);
    crate::boot::parse_initrd::parse_tar(initrd_start, initrd_end);
}
