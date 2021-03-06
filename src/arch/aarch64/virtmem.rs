use crate::prelude::*;
use linked_list_allocator::{Heap, LockedHeap};

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub unsafe fn init(range: PhySlice) {
    println!("[DBUG] Kernel virtual memory area: {:?}", range);
    *ALLOCATOR.lock() = Heap::new(range.base.virt_mut() as usize, range.len);
}

pub fn get_free() -> usize {
    ALLOCATOR.lock().free()
}

pub fn get_used() -> usize {
    ALLOCATOR.lock().used()
}
