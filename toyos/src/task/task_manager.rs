//修改完成
//该模块只负责对任务进行添加和删除
use super::TaskControlBlock;
use super::processor::get_core_id;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
use spin::Mutex;
use crate::timer::get_time;


pub struct TaskManager{
    pub ready_queue:VecDeque<Arc<TaskControlBlock>>,
}

impl TaskManager{
    //用于初始化
    fn new()->Self{
        Self{ready_queue:VecDeque::new()}
    }
    //添加一个新的任务到队列末尾
    fn add(&mut self,task:Arc<TaskControlBlock>){
        self.ready_queue.push_back(task);
    }
    //从队列中拿出首个任务
    fn fetch(&mut self)->Option<Arc<TaskControlBlock>>{
        self.ready_queue.pop_front()
    }
}

lazy_static!{
    //为了避免后期频繁fork和exit可能导致的死锁问题以及同一cpu缓存无法重复利用的问题，使用三个管理器
    //一个cpu核对应一个管理器，主要存放同一个父子进程，剩余一个管理器存放与两个管理器所拥有进程无关的进程，第三个管理器提供公共访问，故需要加锁
    pub static ref TASK_MANAGER:[Mutex<TaskManager>; 3] = 
    unsafe{[Mutex::new(TaskManager::new()),
            Mutex::new(TaskManager::new()),
            Mutex::new(TaskManager::new())]};
}

pub fn add_task(task:Arc<TaskControlBlock>){
    //println!("begin add..");
    let hartid = task.inner_exclusive_access().hartid;
    if(hartid == -1){
        panic!("a task with no hartid");
    }
    //println!("TASK_MANAGER[2_add].is_locked():?{}", TASK_MANAGER[2].is_locked());
    let len_2 = TASK_MANAGER[2].lock().ready_queue.len();
    if(len_2 == 0 || TASK_MANAGER[hartid as usize].lock().ready_queue.len() > 3 * len_2){
        //保证备用管理器有进程等待，减少cpu空转时间
        task.inner_exclusive_access().refresh_hartid(2);
        TASK_MANAGER[2].lock().add(task);
    }
    else{
        TASK_MANAGER[hartid as usize].lock().add(task);//哪个核心就加入哪个进程
    }
    //println!("add succeed..");
}

//如果所有core都有进程并且core的子进程
pub fn fetch_task()->Option<Arc<TaskControlBlock>>{
    let current_hartid = get_core_id();
    if(get_time() % (TASK_MANAGER[current_hartid].lock().ready_queue.len() + 1) == 0){
        let mut option_task = TASK_MANAGER[2].lock().fetch();
        if(option_task.is_some()){//更新hartid值
            let task = option_task.unwrap();
            task.inner_exclusive_access().refresh_hartid(current_hartid as isize);
            return Some(task);
        }
    }
    let mut option_task = TASK_MANAGER[current_hartid].lock().fetch();
    if(option_task.is_none()){//如果当前核心没有可用进程，则从备用进程获取
        //所有core进程均在等待第三队列会出现问题，需要判断是否为等待状态，目前以随即抽取第三队列解决
        option_task = TASK_MANAGER[2].lock().fetch();
        if(option_task.is_some()){//更新hartid值
            let task = option_task.unwrap();
            task.inner_exclusive_access().refresh_hartid(current_hartid as isize);
            return Some(task);
        }
    }
    option_task
}
