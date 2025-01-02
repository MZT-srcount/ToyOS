use super::TaskContext;
use crate::config::{usize_va, PAGE_SIZE_BITS, SV39_VA, TRAP_CONTEXT,PAGE_SIZE, USER_HEAP_SIZE};
use crate::loader::get_app_data_by_name;
use crate::memory::{MemoryManager, PageBit, KERNEL_SPACE, reftime, PageTable, SectionBit, UserBuffer};
use crate::trap::{trap_handler, TrapContext};

use super::{pid_alloc, KernelStack, PidHandle};
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use alloc::vec;
use core::cell::RefMut;
use alloc::string::String;
use crate::timer::{get_time, tick_ms_translate, TimeSpec, TMS};

use spin::{Mutex, MutexGuard};

use crate::fs::{File, Stdin, Stdout, FileDescripter, FileClass};
/*
 * 任务控制块结构
 * 存储任务状态、上下文、trap上下文存储位置（trap存储在用户空间）、内存管理等
 *
 */
/// task control block structure
pub struct TaskControlBlockInner {
    pub status: TaskStatus,
    pub task_cx: TaskContext,
    pub trap_cx_ppn: usize,
    pub memory_manager: MemoryManager,
    pub base_addr: usize,
    pub hartid: isize,
    pub current_path: String,
    //维护父指针
    pub parent: Option<Weak<TaskControlBlock>>,
    pub children: Vec<Arc<TaskControlBlock>>,
    pub fd_table: Vec<Option<FileDescripter>>,
    pub heap_start: usize,
    pub heap_ptr: usize,
    pub last_time: usize,
    pub utime: usize,
    pub stime: usize,
    pub exit_code: i32,
}

impl TaskControlBlockInner {
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        //物理页号转化为物理地址
        let trap_cx_phyAddr=(self.trap_cx_ppn << PAGE_SIZE_BITS);
        unsafe { ( trap_cx_phyAddr as *mut TrapContext).as_mut().unwrap() }
    }
    pub fn get_user_token(&self) -> usize {
        self.memory_manager.token()
    }
    fn get_status(&self) -> TaskStatus {
        self.status
    }
    pub fn get_work_path(&self) -> String{
        self.current_path.clone()
    }
    pub fn get_time(&self) -> TMS{
        let cutime: usize = 0;
        let cstime: usize = 0;
        TMS{
            tms_utime: tick_ms_translate(self.utime),
            tms_stime: tick_ms_translate(self.stime),
            tms_cutime: tick_ms_translate(cutime),
            tms_cstime: tick_ms_translate(cstime),
        }
    }
    pub fn utime_update(&mut self){
        self.utime += get_time() - self.last_time;
        //println!("self.utime refresh..utime: {}, time_now: {}, last_time: {}", self.utime, get_time(), self.last_time);
        self.last_time = get_time()
    }
    pub fn stime_update(&mut self){
        self.stime += get_time() - self.last_time;
        self.last_time = get_time();
    }
    fn get_parent(&self) -> &Option<Weak<TaskControlBlock>> {
        &self.parent
    }
    pub fn is_zombie(&self) -> bool {
        self.get_status() == TaskStatus::Zombie
    }
    pub fn memory_manager(&self) -> &MemoryManager{
        &self.memory_manager
    }
    pub fn get_children(&self) -> &Vec<Arc<TaskControlBlock>>{
        &self.children
    }
    pub fn heap_grow(&mut self, grow_size: usize) -> usize{
        if(self.heap_ptr + grow_size > self.heap_start + USER_HEAP_SIZE){
            panic!("task doesn't have enough memory to alloc, heap_start{:#x}, heap_ptr: {:#x}, grow_size: {}", self.heap_start, self.heap_ptr, grow_size);
        }
        else if(self.heap_ptr + grow_size < self.heap_start){
            panic!("task has gone to the top of heap, heap_start{:#x}, heap_ptr: {:#x}, grow_size: {}", self.heap_start, self.heap_ptr, grow_size);
        }
        self.heap_ptr += grow_size;
        self.heap_ptr
    }
    pub fn refresh_hartid(&mut self, hartid: isize){
        self.hartid = hartid;
    }
    
    pub fn fd_alloc(&mut self) -> usize{
        if let Some(fd) = (0..self.fd_table.len())
            .find(|fd| self.fd_table[*fd].is_none()) {
            fd
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
#[derive(Debug)]
/// task status: UnInit, Ready, Running, Exited
pub enum TaskStatus {
    Ready,
    Running,
    Exited,
    Zombie,
}

pub struct TaskControlBlock {
    pub pid: PidHandle,
    pub kernel_stack: KernelStack,
    inner: Mutex<TaskControlBlockInner>,
}

impl TaskControlBlock {
    pub fn inner_exclusive_access(&self) -> MutexGuard<'_, TaskControlBlockInner> {
        self.inner.lock()
    }
    pub fn getpid(&self) -> usize {
        self.pid.0
    }
    pub fn getppid(&self) -> usize {
        self.inner
            .lock()
            .get_parent()
            .as_ref()
            .unwrap()
            .upgrade() //获取父节点
            .unwrap()
            .pid
            .0
    }
    pub fn new(elf_data: &[u8]) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_manager, heap_top, user_sp, entry_point) = MemoryManager::from_elf(elf_data);
        let trap_cx_ppn = memory_manager
            .virt_find_pte(TRAP_CONTEXT /PAGE_SIZE)
            .unwrap()
            .ppn();
        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        // push a task context which goes to trap_return to the top of kernel stack

        //建立一个TaskControlBlock对象，其中的inner对象是还没有初始化的
        let task_control_block = Self {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                Mutex::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_addr: user_sp,
                    task_cx: TaskContext::register_init(kernel_stack_top),
                    status: TaskStatus::Ready,
                    
                    fd_table: vec![
                    // 0 -> stdin
                    Some( FileDescripter::new(
                        false,
                        FileClass::Abstr(Arc::new(Stdin)) 
                    )),
                    // 1 -> stdout
                    Some( FileDescripter::new(
                        false,
                        FileClass::Abstr(Arc::new(Stdout)) 
                    )),
                    // 2 -> stderr
                    Some( FileDescripter::new(
                        false,
                        FileClass::Abstr(Arc::new(Stdout)) 
                    )),
                    ],
                    memory_manager,
                    hartid: 2,//通过new创建的放在备用task_manager队列
                    parent: None,
                    current_path: String::from("/riscv64/"),
                    children: Vec::new(),
                    last_time: get_time(),
                    heap_start: heap_top,
                    heap_ptr: heap_top,
                    utime: 0,
                    stime: 0,
                    exit_code: 0,
                })
            },
        };
        //初始化inner的上下文
        // prepare TrapContext in user space
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        *trap_cx = TrapContext::process_cx_init(
            entry_point,
            user_sp,
            KERNEL_SPACE.lock().token(),
            trap_handler as usize,
            kernel_stack_top,
        );
        task_control_block
    }
    pub fn check_pagefault(& self, vaddr: usize, is_load: bool){
        let vpn: usize = vaddr / PAGE_SIZE;
        if is_load{
            let memory_manager = &mut self.inner_exclusive_access().memory_manager;
            let ret = memory_manager.sections_belong(vpn);
            if ret.is_none(){
                panic!("can not find the needed section..");
            }
            else{
                let (idx, section) = ret.unwrap();
                memory_manager.remap_one(vpn, idx);
            }
        }
        else{
            panic!("storage fault, can not find a way to deal with it.. location: task_control.check_pagefault");
        }

    }
    pub fn mmap(&self, vaddr: usize, len: usize, prot: u8, offset: usize, flag: usize, fd: usize) -> isize{

        let mut sectionbit: SectionBit = SectionBit::empty();
        if(prot & 0b1 == 1)//PROT_EXEC
        {
            sectionbit |= SectionBit::Execute;
        }
        if(prot & 0b10 == 1)
        {
            sectionbit |= SectionBit::Read;
        }
        if(prot & 0b100 == 1)
        {
            sectionbit |= SectionBit::Write;
        }
        let mut inner = self.inner_exclusive_access();
        if(fd >= inner.fd_table.len()){
            return -1;
        }
        let mut file = inner.fd_table[fd].clone();
        inner.memory_manager.map_vma(vaddr, len, offset, sectionbit);
        inner.memory_manager.mmap_file(vaddr, len, &mut file, offset)
        //此处将映射方式转换到页表项的保护方式中
        //当start为NULL时，代表让系统自动处理，需要注意！！
    }
    pub fn check_lazyalloc(&self, vaddr: usize){
        let vpn: usize = vaddr >> PAGE_SIZE_BITS;
        let memory_manager = &mut self.inner_exclusive_access().memory_manager;
        let ret = memory_manager.sections_belong(vpn);
        if ret.is_none(){
            panic!("can not find the needed section..")
        }
        else{
            panic!("find the needed section..");
        }
    }
    pub fn exec(&self, elf_data: &[u8]) {
        //用新的地址空间替换原有的地址空间和TrapContext上下文
        let (memory_manager, heap_top, user_sp, entry_point) = MemoryManager::from_elf(elf_data);
        let trap_cx_ppn = memory_manager
            .virt_find_pte(TRAP_CONTEXT/PAGE_SIZE)
            .unwrap()
            .ppn();
        let mut inner = self.inner.lock(); //实现所有权转移，致使生命周期结束
        inner.memory_manager = memory_manager;
        inner.trap_cx_ppn = trap_cx_ppn;
        //inner的初始化
        let trap_cx = inner.get_trap_cx();
        *trap_cx = TrapContext::process_cx_init(
            entry_point,
            user_sp,
            KERNEL_SPACE.lock().token(),
            trap_handler as usize,
            self.kernel_stack.get_top(), //使用自身的内核栈
        )
    }

    pub fn fork(self: &Arc<TaskControlBlock>) -> Arc<TaskControlBlock> {
        // ---- access parent PCB exclusively
        let mut parent_inner = self.inner_exclusive_access();
        // copy user space(include trap context)
        let memory_manager = MemoryManager::copy_on_write(&mut parent_inner.memory_manager);
        let trap_cx_ppn = memory_manager
            .virt_find_pte(TRAP_CONTEXT/PAGE_SIZE)
            .unwrap()
            .ppn();
        //unsafe{println!("trap_cx: {}", ((trap_cx_ppn << PAGE_SIZE_BITS) as *mut TrapContext).as_mut().unwrap().sepc)}
        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();

        //println!("kernelstack mapping.. pid_id: {}", pid_handle.0);
        let kernel_stack = KernelStack::new(&pid_handle);
        //println!("we can not get there,,, right ?");
        let kernel_stack_top = kernel_stack.get_top();
        
        
        let mut new_fd_table: Vec<Option<FileDescripter>> = Vec::new();
        
        for fd in parent_inner.fd_table.iter() {
            if let Some(file) = fd {
                new_fd_table.push(Some( file.clone() ));
            } else {
                new_fd_table.push(None);
            }
        }
        
        let mut task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                Mutex::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_addr: parent_inner.base_addr,
                    task_cx: TaskContext::register_init(kernel_stack_top),
                    status: TaskStatus::Ready,
                    memory_manager,
                    parent: Some(Arc::downgrade(self)),
                    //fd_table: Vec::new(),
                    hartid: parent_inner.hartid,
                    children: Vec::new(),
                    fd_table: new_fd_table,
                    current_path: parent_inner.current_path.clone(),
                    last_time: get_time(),
                    heap_start: parent_inner.heap_start,
                    heap_ptr: parent_inner.heap_ptr,
                    utime: 0,
                    stime: 0,
                    exit_code: 0,
                })
            },
        });
        // add child
        parent_inner.children.push(task_control_block.clone());
        // modify kernel_sp in trap_cx
        // **** access children PCB exclusively
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        trap_cx.kernel_sp = kernel_stack_top;


        // return
        task_control_block
        // ---- release parent PCB automatically
        // **** release children PCB automatically
    }

    pub fn write_data<T>(& self, vaddr: usize, data: &T){
        self.inner.lock().memory_manager.write_data(vaddr, data);
    }
    pub fn read_data<T: Clone>(&self, vaddr: usize) -> T{
        self.inner.lock().memory_manager.read_data(vaddr)
    }
}
