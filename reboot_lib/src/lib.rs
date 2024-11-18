#![no_std]
#![feature(allocator_api)]
pub mod memory;
pub mod video;

use core::alloc::{Allocator, GlobalAlloc};
pub use video::{*};

extern crate alloc;



pub struct RegisterWrapper<T>(*mut T);
impl<T> core::ops::Deref for RegisterWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}
impl<T> core::ops::DerefMut for RegisterWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}



pub struct DSIAllocator();
unsafe impl GlobalAlloc for DSIAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        todo!()
    }
}
