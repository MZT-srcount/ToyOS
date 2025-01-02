// #![no_std]
// #![feature(linkage)]
// #![feature(panic_info_message)]
// #![feature(alloc_error_handler)]

// #[macro_use]
// pub mod console;
// mod syscall;
// mod lang_items;

// //支持动态内存
// use buddy_system_allocator::LockedHeap;
// use syscall::*;
// const USER_HEAP_SIZE: usize = 16384;

// static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

// #[global_allocator]
// static HEAP: LockedHeap = LockedHeap::empty();

// #[alloc_error_handler]
// pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
//     panic!("Heap allocation error, layout = {:?}", layout);
// }

// #[no_mangle]
// #[link_section = ".text.entry"]
// pub extern "C" fn _start() -> ! {
//     exit(main());
//     panic!("unreachable after sys_exit!");
// }

// #[linkage = "weak"]
// #[no_mangle]
// fn main() -> i32 {
//     panic!("Cannot find main!");
// }
#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
pub mod console;
mod lang_items;
mod syscall;

use syscall::*;
use buddy_system_allocator::LockedHeap;

const USER_HEAP_SIZE: usize = 16384;

static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start()  {
    unsafe {
        HEAP.lock()
            .init(HEAP_SPACE.as_ptr() as usize, USER_HEAP_SIZE);
    }
    exit(main());
}

#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
    panic!("Cannot find main!");
}

pub struct TimeSpec{
    pub tv_sec: usize,
    pub tv_nsec: usize,
}

pub struct TMS
{ 
    pub tms_utime: usize,          /* User CPU time.  用户程序 CPU 时间*/ 
    pub tms_stime: usize,         /* System CPU time. 系统调用所耗费的 CPU 时间 */ 
    pub tms_cutime: usize,         /* User CPU time of dead children. 已死掉子进程的 CPU 时间*/ 
    pub tms_cstime: usize,    /* System CPU time of dead children.  已死掉子进程所耗费的系统调用 CPU 时间*/ 
}

pub struct UTSNAME{
    pub sysname: [u8; 65],
    pub nodename:[u8; 65],
    pub release: [u8; 65],
    pub machine: [u8; 65],
    pub domainame: [u8; 65],
}

pub fn get_time(timespec: *mut TimeSpec) -> isize { sys_gettimeofday(timespec as usize) }
pub fn get_times(tms: *mut TMS) -> isize { sys_times(tms as usize)}
pub fn mmap(start: usize, len: usize, prot: u8, flags: usize, fd: usize, off: usize) -> isize{sys_mmap(start, len, prot, flags, fd, off)}
pub fn munmap(start: usize, len: usize) -> isize{sys_munmap(start, len)}
pub fn read(fd:usize,buffer:& mut[u8])->isize{sys_read(fd,buffer)}
pub fn write(fd: usize, buf: &[u8]) -> isize { sys_write(fd, buf) }
pub fn uname(utsname: *mut UTSNAME) -> isize{sys_uname(utsname as usize)}

pub fn yield_() -> isize { sys_yield() }
pub fn exit(exit_code: i32) -> isize { sys_exit(exit_code) }
pub fn fork()->isize{sys_fork()}
pub fn exec(path:&str)->isize{sys_exec(path)}
pub fn waitpid(pid:usize,exit_code:* mut i32)->isize{
    loop{

        match sys_waitpid(pid as isize,exit_code as * mut _)
        {
            -2 =>{
                //println!("get there?0");
                 yield_();
                //println!("get there?1");
            }, 
            exit_pid=>return exit_pid,
        }
        //println!("out?");
    }
}

pub fn wait(exit_code:*mut i32)->isize{
    loop{
        match sys_waitpid(-1,exit_code as * mut _){
            -2=>{yield_();},
            exit_pid=>{
                println!("can we get there?, {}", exit_pid);
                return exit_pid
            }
        }
    }
}

pub fn nanosleep(req: *mut TimeSpec, rem: *mut TimeSpec) -> isize{
    sys_nanosleep(req as usize, rem as usize)
}

pub fn getpid() -> isize {
    sys_getpid()
}

// #define SYS_getppid 173
// 功能：获取父进程ID；
// 输入：系统调用ID；
// 返回值：成功返回父进程ID；
pub fn getppid()->isize{
    sys_getppid()
}