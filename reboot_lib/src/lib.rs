#![no_std]
#![feature(allocator_api)]
extern crate alloc;
mod memory;
mod video;
mod allocator;
pub use allocator::ALLOCATOR;
pub use video::{*};

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
