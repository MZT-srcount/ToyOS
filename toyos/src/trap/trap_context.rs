use riscv::register::sstatus::{Sstatus, self, SPP};

//trap上下文
#[repr(C)]
pub struct TrapContext {
    pub x: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
    pub kernel_satp: usize,
    pub kernel_sp: usize,
    pub trap_handler: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {self.x[2] = sp;}
    pub fn process_cx_init(entry: usize, sp: usize, kernel_satp: usize, trap_handler: usize, kernel_sp: usize) -> Self{
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self{
            x: [0;32],
            sstatus,
            sepc: entry,
            kernel_satp: kernel_satp,
            kernel_sp: kernel_sp,
            trap_handler: trap_handler,
        };
        cx.set_sp(sp);
        cx
    }
}
