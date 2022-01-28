use crate::prelude::*;
use buddy_system_allocator::{Heap, LockedHeap};
use core::alloc::{GlobalAlloc, Layout};

struct IrqLockedHeap {
    inner: LockedHeap<32>,
}

impl IrqLockedHeap {
    pub const fn empty() -> IrqLockedHeap {
        Self {
            inner: LockedHeap::empty(),
        }
    }

    pub fn lock(&self) -> spin::mutex::MutexGuard<'_, Heap<32>> {
        self.inner.lock()
    }
}

unsafe impl GlobalAlloc for IrqLockedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _locked = irq_lock();
        self.inner.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _locked = irq_lock();
        self.inner.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: IrqLockedHeap = IrqLockedHeap::empty();

pub unsafe fn init(range: PhySlice) {
    println!("[DBUG] Kernel virtual memory area: {:?}", range);
    let _locked = irq_lock();
    *ALLOCATOR.lock() = {
        let mut heap = Heap::<32>::new();
        heap.init(range.base.virt_mut() as usize, range.len);
        heap
    };
}

pub fn get_free() -> usize {
    let _locked = irq_lock();
    // ALLOCATOR.lock().free()
    0
}

pub fn get_used() -> usize {
    let _locked = irq_lock();
    // ALLOCATOR.lock().used()
    0
}
