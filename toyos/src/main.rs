//不使用标准库
#![no_std]
//去除main函数定位
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(llvm_asm)]

//引入alloc库依赖
extern crate alloc;

#[macro_use]
extern crate bitflags;

use core::arch::global_asm;
use sbi::*;

use crate::timer::get_time_ms;

#[macro_use]
mod console;
mod lang_items;
mod sbi;
mod syscall;
mod trap;
mod sync;
mod timer;
mod loader;
mod task;
mod config;
mod memory;
mod monitor;
mod fs;
mod drivers;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

fn bss_clear(){
    /*
     * extern "C"可以引用外部接口，此处引用位置标志以获取地址
     */
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe{
        core::slice::from_raw_parts_mut(
            sbss as usize as *mut u8,
            ebss as usize - sbss as usize,
        ).fill(0);
    }
}
pub fn id() -> usize {
    let cpu_id;
    unsafe {
        llvm_asm!("mv $0, tp" : "=r"(cpu_id));
    }
    cpu_id
}

#[no_mangle]
pub fn os_main() -> !{
    let core = id();
    let init_time = get_time_ms();
    if core != 0 {
        loop{
            if(get_time_ms() - init_time > 1000000){
                break;
            }
        };//两个内核有不同的cpu，有各自的寄存器，故需要各自初始化
        memory::othercore_init();
        trap::init();
        trap::enable_timer_interrupt();
        timer::set_next_trigger();
        //println!("core 1 begin running..");
        task::run_tasks();
    }
    println!("init_time: {}", init_time);
    println!("core {} is running", core);
    print!("
                                      ^ ^
                                   ^ ^ ^ ^ ^
                               ^ ^ ^ ^ ^ ^ ^ ^ ^
                           ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^
                       ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^
                   ^ ^ ^ ^ ^ ^===================^ ^ ^ ^ ^ ^
               ^ ^ ^ ^ ^ ^ ^ ^|| W e l c o m e ||^ ^ ^ ^ ^ ^ ^ ^
           ^ ^ ^ ^ ^ ^ ^ ^ ^ ^===================^ ^ ^ ^ ^ ^ ^ ^ ^ ^
               ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^
                   ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^ ^
                                 ^ ^ ^ ^ ^ ^ ^
                                 ^ ^ ^ ^ ^ ^ ^
                                 ^ ^ ^ ^ ^ ^ ^
                                 ^ ^ ^ ^ ^ ^ ^
                                 ^ ^ ^ ^ ^ ^ ^
                                 ^ ^ ^ ^ ^ ^ ^
            ");
    bss_clear();
    trap::init();//trap初始化，设置在kernel时的入口地址
    info!("[kernel]trap init succeed!");
    memory::init();//内存初始化
    info!("[kernel]load app succeed!");
    trap::enable_timer_interrupt();//时钟中断使能
    timer::set_next_trigger();//设置下一个时钟中断时间
    //drivers::block::block_device_test();
    fs::init_rootfs();
    info!("[kernel]fs init succeed!");
    task::add_initproc();//加载初始进程
    /*clear_bss();*/
    //info!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
    //debug!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
    //warn!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
    info!("[kernel]toyos begin running");

    loader::list_apps();

    info!("time cost: {}", get_time_ms() - init_time);
    //let mask:usize = 1 << 1;
    //sbi_send_ipi(&mask as *const usize as usize);
    task::run_tasks();//运行用户程序
    panic!("Shutdown machine!");

}
