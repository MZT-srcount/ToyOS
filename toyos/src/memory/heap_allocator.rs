use buddy_system_allocator::LockedHeap;
use crate::config::KERNEL_HEAP_SIZE;

/*
 *堆分配器实现
 *此处借用已实现的堆分配器
 */

/*全局化堆分配器*/
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

/*分配出错时的处理*/
#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

/*堆空间大小设置*/
static mut HEAP_SPACE :[u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

/*堆初始化*/
pub fn init_heap(){
    unsafe{
        HEAP_ALLOCATOR.lock().init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}

