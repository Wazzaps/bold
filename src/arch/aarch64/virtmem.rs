use crate::arch::aarch64::phymem::PhySlice;
use crate::println;
use linked_list_allocator::{Heap, LockedHeap};

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub unsafe fn init(range: PhySlice) {
    println!("[DBUG] Kernel virtual memory area: {:?}", range);
    *ALLOCATOR.lock() = unsafe { Heap::new(range.base.0, range.len) };
}