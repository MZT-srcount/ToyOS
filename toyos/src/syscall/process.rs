use core::time;

use crate::loader::{get_app_data_by_name};
use crate::memory::{translated_refmut,translated_str};
use crate::task::{
    // suspend_and_rnext,
    // exit_and_rnext,
    add_task, current_task, current_user_token,exit_and_rnext,
    suspend_and_rnext, TaskControlBlockInner,
};
use core::cmp::min;

use crate::timer::{get_time_ms, TimeSpec, get_time_ns, get_time_s, NSEC_PER_SEC};

use alloc::sync::Arc;
use alloc::task;

use crate::config::{PAGE_SIZE_BITS, EXITOFFSET};
use crate::fs::{open, OpenFlags, DiskInodeType, FileDescripter, FileClass, File};

/*
 * *****系统调用******
 */

 pub fn sys_exit(exit_code: i32) -> ! {
    info!("[kernel] Application exited with code {}", exit_code);
    exit_and_rnext(exit_code);
    panic!("Unreachable in sys_exit!");
}


pub fn sys_getpid() -> isize {
    current_task().unwrap().pid.0 as isize
}

pub fn sys_getppid()->isize{
    current_task().unwrap().getppid() as isize
}

pub fn sys_fork(flags: usize, stack_ptr: usize, ptid: usize, ctid: usize, newtls: usize)->isize{
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    if(stack_ptr != 0){
        new_task.inner_exclusive_access().get_trap_cx().set_sp(stack_ptr);
    }
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    //返回创建的子进程的TrapContext,还需要修改返回值，这也是区分父子进程的唯一标识
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    //父进程返回子进程id
    new_pid as isize
}

//从应用程序空间传过来的地址均为虚拟地址，我们需要按照该虚拟地址获取对应的应用程序名
pub fn sys_exec(ptr:*const u8)->isize{
    let token = current_user_token();
    let path=translated_str(token,ptr);

    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    //
    //info!("can we get there? sys_exec, current_path: {}, name: {}", task_inner.current_path.as_str(), path.as_str());
    if let Some(app_inode) = open(task_inner.current_path.as_str(), path.as_str(), OpenFlags::RDONLY, DiskInodeType::File){
        let fd = task_inner.fd_alloc();
        let elf_data = app_inode.read_all();
        task_inner.fd_table[fd] = Some(FileDescripter::new(false, FileClass::File(app_inode)));

        /*
        此处后期应用read(buf)代替，没有预先设定容许大小，有一定的风险
        */

        drop(task_inner);
        task.exec(&elf_data);
        0
    }
    else {
        -1
    }
}

pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32,option:usize)->isize{
    //println!("sys_waitpid..");
    //判断三种情况
    //如果是WNOHANG，WUNTRACED，WCONTINUED
    //以下是通过非阻塞方式进行解决的WNOHANG
    let task = current_task().unwrap();
    // find a child process

    // ---- access current TCB exclusively
    //不存在该子进程
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        //println!("not exist.., childnum: {}", inner.children.len());
        if(task.pid.0 == 0){
            panic!("finished..");
        }
        return -1;
        // ---- release current PCB
    }

    //找到该子进程的索引位置
    while(true){
        let mut find_pid: bool = false;
        let pair= inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB lock exclusively
        if(pid == -1 || pid as usize == p.getpid()){
            find_pid = true;
        }
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
        if(pair.is_none() && find_pid){
            drop(inner);
            suspend_and_rnext();
            inner = task.inner_exclusive_access();
        }
        else if let Some((idx, _)) = pair {
            let child = inner.children.remove(idx);
            // confirm that child will be deallocated after removing from children list
            assert_eq!(Arc::strong_count(&child), 1);
            let found_pid = child.getpid();
            // ++++ temporarily access child TCB exclusively
            let exit_code = child.inner_exclusive_access().exit_code;
            //exit_code_ptr需要传递到应用地址空间的位置
            // ++++ release child PCB
            if(exit_code_ptr as usize != 0){//输入值为null时不可写
                *translated_refmut(inner.memory_manager.token(), exit_code_ptr) = exit_code << EXITOFFSET;
            }
            //println!("succeed find..");
            return found_pid as isize
        } else {

            break;
        }
    }
    -2
    // ---- release current PCB lock automatically
}
pub fn sys_nanosleep(vaddr: usize, ret_vaddr: usize) -> isize{
    /*由于沉睡指定时间后并非立即唤醒，故只需要让线程睡眠时间大于指定时间即可*/
    /*第二个结构返回剩余睡眠时间，因为不会被打断，此处暂不处理*/
    let init_time_ns = get_time_ns();
    let task = current_task().unwrap();
    let timespec: TimeSpec = task.read_data(vaddr);
    
    while(timespec.tv_nsec > get_time_ns() - init_time_ns || timespec.tv_sec > get_time_s() - init_time_ns / NSEC_PER_SEC){
        suspend_and_rnext();
    }
    0
}