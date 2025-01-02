//该模块涉及到对任务流的切换，并存储有关当前任务的信息
use super::{TaskContext, TaskControlBlock,TaskStatus,fetch_task,__switch};
use crate::trap::{TrapContext};
use lazy_static::*;
use alloc::sync::Arc;
use spin::Mutex;
pub struct Processor{
    current:Option<Arc<TaskControlBlock>>,
    idle_task_cx:TaskContext,//表示当前处理器上的 idle 控制流的任务上下文，保存run_tasks中的状态，每次schedule利用该上下文回到run_tasks进行再次调度
}

pub fn get_core_id() -> usize{
    
    let tp:usize;
    unsafe {
        llvm_asm!("mv $0, tp" : "=r"(tp));
    }
    tp
    //未完成，容易出现bug,暂时用0代替
}

impl Processor {
    pub fn new()->Self {
        Processor{
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }

    //可以取出当前正在执行的任务。
    pub fn take_current(&mut self)->Option<Arc<TaskControlBlock>>{
        self.current.take()//Option<T>中包含take方法
    }

    //可以取出当前任务的一份拷贝
    pub fn current(&self)->Option<Arc<TaskControlBlock>>{
        self.current.as_ref().map(|task| Arc::clone(task))
    }
    //获取当前任务流上下文地址，在当任务流在内核进行切换时需要使用到
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
}

lazy_static!{
    static ref PROCESSOR: [Mutex<Processor>; 2] =
    unsafe {[Mutex::new(Processor::new()), Mutex::new(Processor::new())]};
}

//接口封装
//两个封装有什么区别？
//取出当前任务
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
        let core_id = get_core_id();
        PROCESSOR[core_id].lock().take_current()
    }
//取出当前任务的拷贝
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    let core_id = get_core_id();
    PROCESSOR[core_id].lock().current()
}

//获取当前任务的token
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    let token = task.inner_exclusive_access().get_user_token();
    token
}
//获取当前任务的trap上下文
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task().unwrap().inner_exclusive_access().get_trap_cx()
}

pub fn run_tasks() {
    let core_id = get_core_id();
    loop {
        if let Some(task) = fetch_task() {
            let mut processor = PROCESSOR[core_id].lock();
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            // access coming task TCB exclusivelyu
            let mut task_inner = task.inner_exclusive_access();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.status = TaskStatus::Running;
            drop(task_inner);
            // release coming task TCB manually
            processor.current = Some(task);
            // release processor manually
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }//每次schedule后回到此处
        }
    }
}

//发生yield或者时间片用完调用schedule进行任务切换
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let core_id = get_core_id();
    let mut processor = PROCESSOR[core_id].lock();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);//因为在函数返回前需要调用processor中的内容，因此需要先释放
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}
