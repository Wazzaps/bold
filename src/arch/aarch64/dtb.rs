use crate::{println, AsciiStr};

use arrayvec::ArrayVec;
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
    // dtb_tree(&dtb);
}
