#![no_std]
#![feature(allocator_api)]
pub mod memory;

use core::alloc::{Allocator, GlobalAlloc};
extern crate alloc;

pub struct DSIAllocator();
unsafe impl GlobalAlloc for DSIAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        todo!()
    }
}
