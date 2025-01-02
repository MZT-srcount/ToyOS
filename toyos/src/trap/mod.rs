mod trap_context;
mod trap_handle;

pub use trap_context::*;
pub use trap_handle::*;

use riscv::register::{
    mtvec::TrapMode,
    stvec,
    scause::{
        self,
        Trap,
        Exception,
        Interrupt,
    },
    stval,
    sie,
};

//初始化，设置trap入口
pub fn init(){
unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}
#[no_mangle]
pub fn trap_from_kernel() -> ! {
    panic!("a trap {:?} from kernel! Stvec:{:x}, Stval:{:#x}", scause::read().cause(), stvec::read().bits(), stval::read());
}

pub fn enable_timer_interrupt() {
    unsafe { sie::set_stimer(); }
}

/* =======================trap上下文:TrapContext=======================
 * #[repr(C)]
    pub struct TrapContext {
        pub x: [usize; 32],//通用寄存器
        pub sstatus: Sstatus,//状态寄存器，反映cpu状态
        pub sepc: usize,//trap前执行的最后一条指令地址
        pub kernel_satp: usize,//内核页表
        pub kernel_sp: usize,//程序对应的内核栈指针
        pub trap_handler: usize,//中断处理函数地址
    }
 *
 * 可用对外接口：
 *
 * pub fn process_cx_init(entry: usize, sp: usize, kernel_satp: usize, trap_handler: usize,
 * kernel_sp: usize) -> Self//初始化程序的trap上下文
 * 输入：
 	entry:指向指令地址，刚开始读入程序时通过解析文件格式获取，如exe、elf文件的程序入口地址等，中断时保存sepc地址（此处通过汇编实现，可不用考虑）
 	sp:栈顶地址，初始时通过分配页面自主创建用户栈获取地址，中断时同样汇编完成用户栈指针保存
 	kernel_satp:内核的页表地址，KERNEL_SPACE地址（因为MemoryManager结构第一个变量为页表）
 	trap_handler:中断处理函数地址，由于做过c处理，直接使用函数地址即可
 	kernel_sp:内核栈地址，同sp处理
   输出：TrapContext结构变量
 * =====================================================================
 *
 * ========================其他接口=====================================
 * pub fn trap_handler() -> !//中断处理函数，可as usize
 *
 * pub fn trap_return() -> ! //中断返回函数, 可as usize
 * =====================================================================
 */
