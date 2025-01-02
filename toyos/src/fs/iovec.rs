use alloc::vec::Vec;
use alloc::vec;
use crate::memory::{
    get_data_buffer,
    translated_array_copy,
};


#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct IoVec {
    base: *mut u8,
    len: usize,
}

pub struct IoVecs(pub Vec<&'static mut [u8]>);

impl IoVecs {
    pub unsafe fn new(
        iov_ptr: *mut IoVec,
        iov_num: usize,
        token: usize,
    )-> Self {
        let mut iovecs: Vec<&'static mut [u8]> = vec![];
        let iovref_vec = translated_array_copy(token, iov_ptr, iov_num);
        iovecs.reserve(iovref_vec.len());
        for iovref in iovref_vec {
            if iovref.len == 0 {
                continue;
            }
            let mut buf:Vec<&'static mut [u8]> = get_data_buffer(token, iovref.base, iovref.len);
            iovecs.append(&mut buf);
        }
        Self(iovecs)
    }
}