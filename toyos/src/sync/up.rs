use core::cell::{RefCell, RefMut};

pub struct UPSafeCell<T/*: ?Sized*/> {
    inner: RefCell<T>,
}

//unsafe impl<T: ?Sized> Sync for UPSafeCell<T> {}
//unsafe impl<T: ?Sized> Send for UPSafeCell<T> {}
unsafe impl<T> Sync for UPSafeCell<T> {}

impl<T> UPSafeCell<T> {
    pub unsafe fn new(value: T) -> Self {
        Self { inner: RefCell::new(value) }
    }
    
    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
    
    
}

pub struct RefFMutex<T: ?Sized>{
     inner: RefCell<T>,
}

unsafe impl<T: ?Sized> Sync for RefFMutex<T> {}
unsafe impl<T: ?Sized> Send for RefFMutex<T> {}

impl<T> RefFMutex<T> {
    pub unsafe fn new(value: T) -> Self {
        Self { inner: RefCell::new(value) }
    }
    
    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
    
 
}
