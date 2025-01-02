use core::cell::{RefCell, RefMut};
use core::llvm_asm;
use super::up::RefFMutex;
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
#[allow(dead_code)]
enum STATE{
    unlock = 0,
    lock = 1,
}
/*自旋锁*/
pub struct SpinMutex {
    state : STATE,
}


impl SpinMutex{
    #[allow(dead_code)]
    /*新建自选锁*/
    pub fn new() -> Self{
        SpinMutex {
            state : STATE::unlock,
        }
    }
    /*加锁，忙等待策略*/
    pub fn lock(&mut self){
        let mut state : STATE = STATE::lock;
    	//info!("[kernel]lock try...[{:#x}, {:#x})\n", self.state as usize, 1);
        while(state == STATE::lock)
        {
            unsafe{
            	llvm_asm!("amoswap.w.aq $0, $1, ($2)\n" : "=r"(state) : "r"(1), "r"(&self.state) :: "volatile");
            }
            //info!("[kernel]recycle...[{:#x})\n", state as usize);
        }
    	//info!("[kernel]lock succeed...");
    }
    /*解锁*/
    pub fn unlock(&mut self){
        let state = &mut self.state;
        unsafe{
            llvm_asm!("amoswap.w.rl zero, zero, ($0)\n"::"r"(state)::"volatile");
        }
    	//info!("[kernel]unlock succeed...");
    }
}

/*自旋锁，可用于给自定义的变量加锁*/
pub struct SpinLock<T: ?Sized>{
    state : STATE,
    data  : RefFMutex<T>,
}

unsafe impl<T: ?Sized + Send> Sync for SpinLock<T>{}
unsafe impl<T: ?Sized + Send> Send for SpinLock<T>{}

impl<T> SpinLock<T>{
    /*新建自旋锁，data:变量*/
    pub fn new(data: T) -> SpinLock<T>{
    	SpinLock{
    	    state : STATE::unlock,
    	    data  : unsafe{RefFMutex::new(data)},
    	}
    }
    /*加锁，返回变量T*/
    pub fn lock(&mut self) -> RefMut<'_, T>{
    	let mut state = STATE::lock;
    	while(state == STATE::lock)
    	{
    	     unsafe{
    	     	   llvm_asm!("amoswap.w.aq $0, $1, ($2)\n" : "=r"(state) : "r"(1), "r"(&self.state)::"volatile");
    	     }
    	}
    	self.data.exclusive_access()
    }
    /*解锁，无返回值*/
    pub fn unlock(&mut self){
    	unsafe{
    	   llvm_asm!("amoswap.w.rl zero, zero, ($0)\n" ::"r"(&mut self.state)::"volatile");
    	}
    }
}
