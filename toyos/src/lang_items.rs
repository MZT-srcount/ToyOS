use core::panic::PanicInfo;
use crate::sbi::shutdown;

/*
 * location.file打印出错文件
 * location.line打印出错行
 * info.message打印出错信息
 */

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
            );
    } else {
        println!("Panicked: {}", info.message().unwrap());
    }
    shutdown()
}

pub trait Bytes<T>{
    fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<T>();
        unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const T as usize as *const u8,
                size,
            )
        }
    }
    
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        let size = core::mem::size_of::<T>();
        unsafe {
            core::slice::from_raw_parts_mut(
                self as *mut _ as *mut T as usize as *mut u8,
                size,
            )
        }
    }
}