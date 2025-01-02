const FD_STDIN:usize=0;
use core::ops::DerefMut;

use crate::config::{PAGE_SIZE_BITS, SYSNAME, NODENAME, RELEASE, MACHINE, DOMAINNAME, UNAME_LEN};
use crate::task::{current_user_token, current_task};
use crate::memory::{get_data_buffer, SectionBit};
use crate::sbi::console_getchar;
use core::mem::size_of;
use alloc::sync::Arc;
use xmas_elf::header::Machine;
use crate::timer::{get_time_s, get_time_ns, get_time_ms, get_time, TimeSpec, TMS};
/*
 * 系统接口设计
 */
 
 use crate::task::{
    suspend_and_rnext,
    exit_and_rnext,
};

/*
 * *****系统调用******
 */
pub fn sys_thread_create(entry: usize, arg: usize) -> isize{
    -1
}
pub fn sys_yield() -> isize {
    current_task().unwrap().inner_exclusive_access().refresh_hartid(2);//防止双处理器均出现待机状态而死锁
    suspend_and_rnext();
    0
}

pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}
/*
const FD_StDOUT: usize = 1;
pub fn sys_write(fd: usize, buf:*const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let slice = get_data_buffer(current_user_token(), buf, len);
            for byte in slice{
            	print!("{}", core::str::from_utf8(byte).unwrap());
            }
            len as isize
        },
        _=> {
            panic!("Unsupported fd in sys_write!");
        }
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "Only support len = 1 in sys_read!");
            let mut c: usize;
            loop {
                c = console_getchar();
                if c == 0 {
                    suspend_and_rnext();
                    continue;
                } else {
                    break;
                }
            }
            let ch = c as u8;
            //将读入的内容写入到对应物理地址
            let mut buffers = get_data_buffer(current_user_token(), buf, len);
            unsafe {
                buffers[0].as_mut_ptr().write_volatile(ch);
            }
            1
        }
        _ => {
            panic!("Unsupported fd in sys_read!");
        }
    }
}
*/
pub fn sys_gettimeofday(vir_addr: usize) -> isize{
    let mut task = current_task().unwrap();
    let option_pte = task.inner_exclusive_access().memory_manager().virt_find_pte(vir_addr >> PAGE_SIZE_BITS);
    if option_pte.is_some() {
        let pte = option_pte.unwrap();
        if(pte.is_writable()){//如果是写时复制，则还需要进一步判断
            let timespec = TimeSpec{
                tv_sec: get_time_s(),//修改了timer部分
                tv_nsec: get_time_ns(),
            };
            task.write_data(vir_addr, &timespec);
            return 0;
        }
        else{
            return -1;
        }
    }
    else{
        info!("[kernel]：illegal address!!!");
    }
    -1
}

pub fn sys_times(vir_addr: usize) -> isize{
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if let Some(pte) = inner.memory_manager().virt_find_pte(vir_addr >> PAGE_SIZE_BITS){
        let time = get_time();//需要提前声明以统一时长
        let mut tms_cutime = 0;
        let mut tms_cstime = 0;
        //应该使用深度/广度优先搜索策略
        for child in inner.get_children().iter(){//遍历task的children，子进程的子进程需要记录？
            tms_cutime += child.inner_exclusive_access().utime;
            tms_cstime += time - child.inner_exclusive_access().stime;
        }
        let mut tms = TMS{
            tms_utime: inner.utime,//task需要记录使用时长，每次切换时需修改一次
            tms_stime: inner.stime,//task需要记录创建时间
            tms_cutime: tms_cutime,
            tms_cstime: tms_cstime,
        };
        info!("[info]tms: tms_cstime: {}, tms_cutime: {}, tms_stime: {}, tms_utime: {}, inner_stime: {}", 
        unsafe{(tms).tms_cstime}, unsafe{(tms).tms_cutime}, unsafe{(tms).tms_stime}, unsafe{(tms).tms_utime}, unsafe{inner.stime});
        drop(inner);//write_data需要使用锁，提前释放
        task.write_data(vir_addr, &tms);
        return 0;
    }
    else{
        info!("[kernel]：illegal address!!!");
        return -1;
    }
    -1
}

pub fn sys_mmap(start: usize, len: usize, prot: u8, flags: usize, fd: usize, off: usize) -> isize{
    //文件映射，注意：由于可能多个进程共享文件，故需要在进程处对文件内存（文件描述符部分）施加保护
    //直接在块缓冲区写合适还是将文件转至用户缓冲区，修改后再填回块缓冲区（如果这样如何即时同步块缓冲区和用户缓冲区，不同进程如果修改文件该如何？）？
    let task = current_task().unwrap();//获取当前进程
    task.mmap(start, len, prot, off, flags, fd);
    0
    
}

pub fn sys_munmap(start: usize, len: usize) -> isize{
    let task = current_task().unwrap();//获取当前进程
    let res = task.inner_exclusive_access().memory_manager.drop_vma(start, len);
    if(res.is_err()){
        panic!("something wrong!!!");
        return -1;
    }
    0
}

pub fn sys_brk(brk_addr: usize) -> isize{
    let mut heap_ptr;
    if(brk_addr == 0){
        heap_ptr = current_task().unwrap().inner_exclusive_access().heap_ptr;
    }
    else{
        let grow_size = brk_addr - current_task().unwrap().inner_exclusive_access().heap_ptr;
        heap_ptr = current_task().unwrap().inner_exclusive_access().heap_grow(grow_size);
    }
    heap_ptr as isize
}


pub struct UTSNAME{
    sysname: [u8; 65],
    nodename:[u8; 65],
    release: [u8; 65],
    machine: [u8; 65],
    domainame: [u8; 65],
}

pub fn sys_uname(vaddr: usize) -> isize{
    let mut utsname = UTSNAME{
        sysname: [0u8; UNAME_LEN],
        nodename: [0u8; UNAME_LEN],
        release: [0u8; UNAME_LEN],
        machine: [0u8; UNAME_LEN],
        domainame: [0u8; UNAME_LEN],
    };
    utsname.machine[0..SYSNAME.len()].clone_from_slice(SYSNAME);
    utsname.nodename[0..NODENAME.len()].clone_from_slice(NODENAME);
    utsname.release[0..RELEASE.len()].clone_from_slice(RELEASE);
    utsname.machine[0..MACHINE.len()].clone_from_slice(MACHINE);
    utsname.domainame[0..DOMAINNAME.len()].clone_from_slice(DOMAINNAME);
    let task = current_task().unwrap();
    debug!("[kernel] 209");
    task.write_data(vaddr, &utsname);
    debug!("[kernel] 211");
    0
}