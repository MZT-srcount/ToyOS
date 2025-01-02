use core::f32::consts::FRAC_1_PI;

use alloc::{vec::Vec, collections::BTreeMap};
use crate::sync::{SpinMutex};
use lazy_static::*;
use crate::config::{PAGE_SIZE, MEMORY_END};

use spin::Mutex;

/*物理页帧*/
pub struct Phy_PageFrame{
   pub ppn: usize,
}

impl Phy_PageFrame{
    pub fn new_for_share(phypagenum: usize) -> Self{
        Self{
            ppn: phypagenum as usize
        }
    }
   pub fn zero_new(phypagenum: usize) -> Self{
       //println!("phypagenum: {:#x}", phypagenum);
       unsafe{
           core::slice::from_raw_parts_mut(
               (phypagenum * PAGE_SIZE) as *mut u8,
               PAGE_SIZE,
               ).fill(0);
       }
       Self{
       	    ppn: phypagenum as usize
       	}
   }
}

impl Drop for Phy_PageFrame{
    fn drop(&mut self) {
        //println!("drop Phy_PageFrame..");
        let mut mutex = SpinMutex::new();
        frame_dealloc(self.ppn);
        //print!("drop succeed..");
    }
}
/*页帧分配*/
trait FrameAlloc{
    fn new() -> Self;
    fn alloc(&mut self) -> Option<usize>;
    fn dealloc(&mut self, ppn: usize);
    fn add_reftime(&mut self, ppn: usize);
}

/*
 * =========页帧管理==========
 * left、right指明当前未被分配的页帧范围
 * recycle为回收的页帧
 */
pub struct Frame_Manager{
    left: usize,
    right: usize,
    recycle: Vec<usize>,
    reftime: BTreeMap<usize, u8>,
}

impl Frame_Manager{
    pub fn init(&mut self, l: usize ,r:usize) {
        self.left = l;
        self.right = r;
    }
}
impl FrameAlloc for Frame_Manager{
    fn new() -> Self{
        Self{
            left : 0,
            right: 0,
            recycle: Vec::new(),
            reftime: BTreeMap::new(),
        }
    }
    fn alloc(&mut self) -> Option<usize>{//不能返回PhyPageFrame, usize->PhyPageFrame会有局部变量，导致返回时PhyPageFrame的drop,当FRAME_MANAGER调用时会出现调用自身的情况
    	//println!("recycle_size:{}, left:{}, right:{}", self.recycle.len(), self.left, self.right);
        //如果有回收的页帧，优先分配回收页帧，否则分配未使用页帧，如无可分配页帧，返回None
        //后期无可分配页帧可采取主动回收
        //assert_eq!(!(self.recycle.len() == 0 && self.left == self.right) as usize, 1);
        let mut phypagenum: Option<usize> = None;
        if let Some(ppn) = self.recycle.pop() {
             phypagenum = Some(ppn);
        }
        else{
            if self.left != self.right{
                let ppn = self.left;
                self.left += 1;
                phypagenum = Some(
                   ppn
                );
            }
            else{
                //无可用页面
                panic!("can not find redundant pageframe");
            }
        }
        if self.reftime.contains_key(& phypagenum.unwrap()){
            panic!("alloc page which has been alloc..location:Framealloc.alloc()");
        }
        else{
            self.reftime.insert(phypagenum.unwrap(), 1);
        }
        phypagenum
    }
    fn dealloc(&mut self, ppn: usize){
        //页帧的回收
        //println!("phyframe dealloc..");
        if self.left <= ppn || self.recycle.iter().find(|&v| {(*v) == ppn})
            .is_some(){
                panic!("memory/phyframe.rs: PageFrame {} has not been allocated!", ppn);
            }
        if self.reftime.contains_key(&ppn){
            *self.reftime.get_mut(&ppn).unwrap() -= 1;
            if(*self.reftime.get(&ppn).unwrap() == 0){
                self.reftime.remove(&ppn);
                self.recycle.push(ppn);
            }
        }
    }
    fn add_reftime(&mut self, ppn: usize){
        if self.reftime.contains_key(&ppn){
            *self.reftime.get_mut(&ppn).unwrap() += 1;
        }
        else{
            panic!("can not find the ppn..location:FrameAlloc.alloc()");
        }
    }
}

/*页帧管理的全局声明*/
lazy_static!{//必须上锁
    pub static ref FRAME_MANAGER : Mutex<Frame_Manager> = unsafe{
        Mutex::new(Frame_Manager::new())
    };
}

/*对外支持的一些页帧管理接口*/
pub fn init_frame_manager() {
    extern "C"{
    	fn ekernel();
    }
    FRAME_MANAGER.lock()
    .init(ekernel as usize / PAGE_SIZE, MEMORY_END / PAGE_SIZE);
}

pub fn frame_alloc() -> Option<Phy_PageFrame> {
    FRAME_MANAGER
    .lock()
    .alloc()
    .map(|ppn| Phy_PageFrame::zero_new(ppn))
}

pub fn frame_dealloc(ppn: usize) {
    FRAME_MANAGER
    	.lock()
    	.dealloc(ppn);
}

pub fn add_reftime(ppn: usize){
    FRAME_MANAGER.lock().add_reftime(ppn);
}
pub fn reftime(ppn: usize) -> usize{
    FRAME_MANAGER.lock().reftime[&ppn].into()
}