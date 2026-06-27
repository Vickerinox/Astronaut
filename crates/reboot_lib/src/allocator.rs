use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    hint,
    ops::{Deref, DerefMut, Drop},
    ptr::{self, NonNull},
};
use linked_list_allocator::Heap;

#[global_allocator]
pub static ALLOCATOR: DSiAllocator = DSiAllocator;

/// An wrapper that simply redirects to the real allocator which is in main memory.
pub struct DSiAllocator;
impl Deref for DSiAllocator {
    type Target = DualSuperAllocator;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(ALLOCATOR_LOCATION as *const DualSuperAllocator) }
    }
}
impl DerefMut for DSiAllocator {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(ALLOCATOR_LOCATION as *mut DualSuperAllocator) }
    }
}
unsafe impl GlobalAlloc for DSiAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.inner_alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner_dealloc(ptr, layout);
    }
}

#[repr(C)]
pub struct DualSuperAllocator {
    cell: UnsafeCell<Heap>,
    locked: UnsafeCell<bool>,
    valid: UnsafeCell<bool>,
}
pub struct LockGuard<'a>(&'a DualSuperAllocator);

//master interrupt enable register.
const ALLOCATOR_LOCATION: usize = 0x200_1000;
const HEAP_START: usize = ALLOCATOR_LOCATION + size_of::<DualSuperAllocator>();
const HEAP_LEN: usize = 0x2F0_0000 - HEAP_START;
impl DualSuperAllocator {
    /// Locks the Allocator, returning a lockguard which allows access to the heap
    ///
    /// This process uses a basic spinlock
    unsafe fn lock<'a>(&'a self) -> LockGuard<'a> {
        if self.valid.get().read() == false {
            panic!("Allocation after invalidation");
        }
        crate::critical_function(|| {
            while ptr::replace(self.locked.get(), true) {
                hint::spin_loop();
            }
        });
        return LockGuard(self);
    }

    /// Unlocks the allocator
    ///
    /// NOTE: this should only be done when it is GUARANTEED to follow rusts safety rules, i.e whenever a lockguard is dropped.
    unsafe fn unlock(&self) {
        crate::critical_function(|| ptr::write_volatile(self.locked.get(), false));
    }
    pub unsafe fn invalidate(&self) {
        (*self.valid.get()) = false;
    }
    /// initialize ourself from uninitialized memory
    unsafe fn self_init(&self) {
        // IDGAF if this is UB like the people on the discord server say, ill do it anyway!
        //let ourself_mut = self as *const Self as usize as *mut Self;
        *(self.cell.get()) = Heap::empty();
        *(self.locked.get()) = false;
        *(self.valid.get()) = true;
        //(*ourself_mut).cell = UnsafeCell::new(Heap::empty());
        //(*ourself_mut).locked = UnsafeCell::new(false);
    }
    /// Initiialize allocator
    pub unsafe fn init(&self) {
        self.self_init();
        self.lock().init(HEAP_START as *mut u8, HEAP_LEN);
    }
    unsafe fn inner_alloc(&self, layout: Layout) -> *mut u8 {
        match self.lock().allocate_first_fit(layout) {
            Ok(success) => success.as_ptr(),
            Err(()) => ptr::null_mut(),
        }
    }
    unsafe fn inner_dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.lock().deallocate(NonNull::new_unchecked(ptr), layout)
    }
}
impl<'a> Deref for LockGuard<'a> {
    type Target = Heap;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.cell.get() }
    }
}
impl<'a> DerefMut for LockGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.cell.get() }
    }
}
impl<'a> Drop for LockGuard<'a> {
    fn drop(&mut self) {
        unsafe { self.0.unlock() };
    }
}
