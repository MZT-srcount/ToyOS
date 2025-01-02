use crate::trap::trap_return;
/*
 * taskcontext结构以及相关函数，用于保存任务的上下文
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct TaskContext {
    /*
     * 控制寄存器的保存：
     *ra寄存器
     *sp寄存器: stack pointer
     * 
     * 通用寄存器: s0-s12
     */
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

impl TaskContext {
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }
    //初始化变量
    pub fn register_init(stack_pointer: usize) -> Self{
        Self {
            ra: trap_return as usize,
            sp: stack_pointer,
            s : [0;12],
        }
    }
}
