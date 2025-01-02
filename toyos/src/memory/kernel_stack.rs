use alloc::vec::Vec;
/*内核栈分配，由于引入线程后，线程号和进程号不相通，故使用栈分配，也可以通过线程进程共用一个标识符分配器来完成栈分配*/ 
pub struct KStackAllocator{
    current: usize,
    recycle: Vec<usize>,
}

impl KStackAllocator{
    pub fn new() -> Self{
        Self{
            current: 0,
            recycle: Vec::new(),
        }
    }
    pub fn alloc_kstack(&mut self) -> (usize, usize){
        let top: usize;
        let bottom: usize;
        let id: usize;
        let sid = self.recycle.pop();
        if sid == None {
            id = self.current;
            self.current += 1;
        }
        else{
            id = sid;
        }
        top = (TRAMPOLINE_PHY - id * (KERNEL_STACK_SIZE + PAGE_SIZE));
        bottom = top - KERNEL_STACK_SIZE;
        (id, bottom, top)
    }
    pub fn dealloc(&mut self, id: usize){
        self.recycle.push(id);
    }
}

pub struct KernelStack{
    pub bottom: usize,
    pub top: usize,
    pub ptr: usize,
    pub data_collect: Vec<usize>,//保存每次Push的值位置
}
impl KernelStack{
    #[allow(unused)]
    pub fn push<T>(&mut self, val: T)
    {
        let ptr_mut = (self.ptr - core::mem::size_of::<T>) as *mut T;
        unsafe{
            *ptr_mut = value;
        }
        self.data_collect.push(self.ptr);
        ptr -= core::mem::size_of::<T>;
    } 
    pub fn get_top(&self) -> usize{
        self.top
    }
    pub fn pop(&self){
        let his_ptr = self.data_collect.pop();
        let ptr_mut = (his_ptr - self.ptr) as *mut u64,
        unsafe{
            *ptr_mut = 0;
        }
        self.ptr = his_ptr;
    }
    pub fn new(kstack_bottom: usize, kstack_top: usize) -> Self{
        Self{
            bottom: kstack_bottom,
            top: kstack_top,
            ptr: kstack_top,
            data_collect: Vec::new(),
        }
    }
}
lazy_static!{
    static ref KSTACKALLOCATOR: UPSafeCell<KStackAllocator> = 
        unsafe{UPSafeCell::new(KStackAllocator::new())};
}




//usize转为virtual address：防止usize值过大
pub fn usize_va(virt: usize) -> usize {
	virt & ((1 << SV39_VA) - 1)
}