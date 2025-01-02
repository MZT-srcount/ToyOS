//与内存管理有关
use crate::config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE};
use crate::memory::{SectionBit,KERNEL_SPACE};

//进程号管理
//pid用于分配全局唯一的进程号(进程号今后作为进程的唯一标识)
use alloc::vec::Vec;
use lazy_static::*;

use spin::Mutex;


//current用于保存已分配出去的最大进程号+1
//recycled用于保存回收的进程号
struct PidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl PidAllocator {
    pub fn new() -> Self {
        PidAllocator {
            current: 0,
            recycled: Vec::new(),
        }
    }
    pub fn alloc(&mut self) -> PidHandle {
        if let Some(pid) = self.recycled.pop() {
            PidHandle(pid)
        } else {
            self.current += 1;
            PidHandle(self.current - 1)
        }
    }
    pub fn dealloc(&mut self, pid: usize) {
        assert!(pid < self.current);
        assert!(
            self.recycled.iter().find(|ppid| **ppid == pid).is_none(),
            "pid {} has been deallocated!", pid
        );
        self.recycled.push(pid);
    }
}

lazy_static! {
    static ref PID_ALLOCATOR : Mutex<PidAllocator> = unsafe {
        Mutex::new(PidAllocator::new())
    };
}

pub struct PidHandle(pub usize);
impl Drop for PidHandle {
    fn drop(&mut self) {
        //println!("drop pid {}", self.0);
        PID_ALLOCATOR.lock().dealloc(self.0);
    }
}


//封装称一个接口用于专门分配进程号
pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.lock().alloc()
}


//存储应用对应的内核栈中的信息，因此每个应该都有一个内核栈
pub struct KernelStack {
    pid: usize,
}
/// Return (bottom, top) of a kernel stack in kernel space.
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        //println!("drop kernel_stack, id: {}", self.pid);
        let (kernel_stack_bottom, _) = kernel_stack_position(self.pid);
        KERNEL_SPACE
            .lock()
            .remove_area_with_start_vpn(kernel_stack_bottom);
    }
}

impl KernelStack {
    pub fn new(pid_handle: &PidHandle) -> Self {
        let pid = pid_handle.0;
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid);
        KERNEL_SPACE.lock().insert_maparea(
            kernel_stack_bottom,
            kernel_stack_top,
            SectionBit::Read | SectionBit::Write,
        );
        KernelStack { pid: pid_handle.0 }
    }
    #[allow(unused)]
    pub fn push_on_top<T>(&self, value: T) -> *mut T
    where
        T: Sized,
    {
        let kernel_stack_top = self.get_top();
        let ptr_mut = (kernel_stack_top - core::mem::size_of::<T>()) as *mut T;
        unsafe {
            *ptr_mut = value;
        }
        ptr_mut
    }
    pub fn get_top(&self) -> usize {
        let (_, kernel_stack_top) = kernel_stack_position(self.pid);
        kernel_stack_top
    }
}
