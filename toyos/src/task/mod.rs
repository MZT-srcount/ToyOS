mod task_context;
mod task_control;
mod task_manager;
mod pid;
mod processor;

use crate::loader::{get_app_data_by_name};
use task_context::TaskContext;
use task_control::{TaskControlBlock,TaskStatus};
pub use task_control::{TaskControlBlockInner};
pub use task_manager::{add_task,fetch_task,};
use core::arch::global_asm;
pub use pid::{pid_alloc,KernelStack,PidHandle};
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
};
use crate::config::*;
use crate::timer::{TimeSpec};
use lazy_static::*;
use alloc::sync::Arc;

//用于切换不同的任务流
global_asm!(include_str!("switch.S"));

extern "C" {
    pub fn __switch(
        current_task_cx_ptr: *mut TaskContext,
        next_task_cx_ptr: *const TaskContext
    );
}

// /*公共接口定义*/
// pub fn run_first_task() {//运行第一个应用程序
//     TASK_MANAGER.run_first_task();
// }

// pub fn run_next_task() {//运行下一个应用程序
//     TASK_MANAGER.run_next_task();
// }

// pub fn suspend_and_rnext() {//暂停当前应用程序并运行下一个可执行应用程序
//     TASK_MANAGER.suspended();
//     run_next_task();
// }

// pub fn exit_and_rnext() {//关闭当前应用程序并运行下一个可执行应用程序
//     TASK_MANAGER.exited();
//     run_next_task();
// }

// pub fn current_user_token() -> usize {//当前应用程序的satp
//     TASK_MANAGER.current_token()
// }

// pub fn current_trap_cx() -> &'static mut TrapContext {//当前应用程序的trap上下文指针
//     TASK_MANAGER.current_trap_cx()
// }

lazy_static! {
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(
        TaskControlBlock::new(get_app_data_by_name("initproc").unwrap())
    );
}
//用于添加第一个任务进程
pub fn add_initproc(){
    add_task(INITPROC.clone());
}

//暂停当前的任务，并切换到下一个任务
pub fn suspend_and_rnext() -> isize {
    //println!("task num: {}", TASK_MANAGER[0].exclusive_access().ready_queue.len());
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.status = TaskStatus::Ready;
    //由于在释放task_inner(当前任务)，task(要被切换进来的任务)前，需要进行任务流上下文的转换shedule
    //因此要先释放锁
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
    0
}
//注意要标记为僵尸进程，还要考虑父子关系
//退出当前的进程，并运行下一个进程
pub fn exit_and_rnext(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();
    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    //将不用的进程放置到初始用户进程下，以便回收
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB

    inner.children.clear();
    // deallocate user space
    inner.memory_manager.recycle_data_pages();
    drop(inner);
    // **** release current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}