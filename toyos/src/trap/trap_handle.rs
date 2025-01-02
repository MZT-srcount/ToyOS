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
use crate::{syscall::syscall, task::current_task};
use super::trap_from_kernel;
use core::arch::{global_asm, asm};
use crate::timer::set_next_trigger;
use crate::config::{TRAP_CONTEXT_PHY, TRAMPOLINE_PHY};
use crate::task::{
    exit_and_rnext,
    suspend_and_rnext,
    current_user_token,
    current_trap_cx,
};

global_asm!(include_str!("trap.S"));



#[no_mangle]
pub fn trap_handler() -> !{
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
    current_task().unwrap().inner_exclusive_access().utime_update();
    //读取中断号
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
    //系统调用类trap
        Trap::Exception(Exception::UserEnvCall) => {
            let mut cx = current_trap_cx();
            cx.sepc += 4;
            //println!("cx.sepc: {:#x}", cx.sepc);
            let result = syscall(cx.x[17], [cx.x[10],cx.x[11],cx.x[12], 0, 0, 0, 0]) as usize;
            current_task().unwrap().inner_exclusive_access().stime_update();
            cx = current_trap_cx();
            cx.x[10] = result as usize;
        }
    //出现访存错误trap
        Trap::Exception(Exception::StoreFault) |
        Trap::Exception(Exception::StorePageFault) |
        Trap::Exception(Exception::LoadFault) |
        Trap::Exception(Exception::LoadPageFault)=> {
            info!("[kernel] PageFault..{:?}", scause.cause());
            let mut is_load = false;
            if(true){//scause.cause() == Trap::Exception(Exception::LoadFault) || scause.cause() == Trap::Exception(Exception::LoadPageFault)){
                    is_load = true;
                let task = current_task().unwrap();
                task.check_pagefault(stval as usize, is_load);
            }
            else{
                let cx = current_trap_cx();
                //println!("[kernel] PageFault in application, kernel killed it. satp:{}, sepc:{}, kernel_satp:{}", current_user_token(), cx.sepc, cx.kernel_satp);
                exit_and_rnext(-2);
            }
        }
        Trap::Exception(Exception::InstructionPageFault) => {
            //println!("Posi, vaddr: {}", stval);
            if let pte = current_task().unwrap().inner_exclusive_access().memory_manager.virt_find_pte(stval){
                //println!("the flag is: {}", pte.unwrap().get_flag());
            }
            let cx = current_trap_cx();
            panic!("Unsupported trap {:?}, stval = {:#x}!, sepc = {:#x}, pid = {}", scause.cause(), stval, cx.sepc, current_task().unwrap().pid.0);
        }
    //出现非法指令错误trap
        Trap::Exception(Exception::IllegalInstruction) => {
            let cx = current_trap_cx();
            //println!("[kernel] IllegalInstruction, kernel killed it. satp:{:#x}, sepc:{:#x}, kernel_satp:{:#x}", current_user_token(), cx.sepc, cx.kernel_satp);
            exit_and_rnext(-3);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            //println!("interrupt..");
            set_next_trigger();
            
            current_task().unwrap().inner_exclusive_access().stime_update();
            suspend_and_rnext();
        }
    //其他情形trap
        _ => {
            panic!("Unsupported trap {:?}, stval = {:#x}!", scause.cause(), stval);
        }
    }
    trap_return();
}

#[no_mangle]
pub fn trap_return() -> ! {
    
    
    let trap_cx_ptr = TRAP_CONTEXT_PHY;
    let user_satp = current_user_token();
    
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    //println!("trap_return,{:#x}", user_satp);
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE_PHY;
    //println!("can we get there?101");
    
    unsafe {
        stvec::write(TRAMPOLINE_PHY, TrapMode::Direct);
    }
    //unsafe{println!("trap_cx in trap_return: {}", ((trap_cx_ptr) as *mut TrapContext).as_mut().unwrap().sepc)}
    //println!("trap_return : cx.sepc: {:#x}, satp: {:#x}", current_trap_cx().sepc, user_satp);
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,
            in("a1") user_satp,
            options(noreturn)
        );
    }
}



pub use crate::trap::TrapContext;
