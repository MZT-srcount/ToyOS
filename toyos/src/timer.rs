use riscv::register::time;
use crate::sbi::set_timer;
use crate::config::CLOCK_FREQ;
//riscv系列接口由RustSBI提供，可以看作是对机器层指令的封装和预处理

const MSEC_PER_SEC: usize = 1_000;
const TICKS_PER_SEC: usize = 100;//每秒时钟中断数
const USEC_PER_SEC: usize = 1_000_1000;//微秒
pub const NSEC_PER_SEC: usize = 1_000_000_000;

#[derive(Copy, Clone)]
pub struct TimeSpec{
    pub tv_sec: usize,
    pub tv_nsec: usize,
}

pub struct TMS
{ 
    pub tms_utime: usize,          /* User CPU time.  用户程序 CPU 时间*/ 
    pub tms_stime: usize,         /* System CPU time. 系统调用所耗费的 CPU 时间 */ 
    pub tms_cutime: usize,         /* User CPU time of dead children. 已死掉子进程的 CPU 时间*/ 
    pub tms_cstime: usize,    /* System CPU time of dead children.  已死掉子进程所耗费的系统调用 CPU 时间*/ 
}
//time::read()返回计数器值
pub fn get_time() -> usize {
    time::read()
}

//微秒级
pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC)
}

//秒级
pub fn get_time_s() -> usize{
    time::read() / CLOCK_FREQ
}
//纳秒级
pub fn get_time_ns() -> usize{
    time::read() / (CLOCK_FREQ / USEC_PER_SEC) * MSEC_PER_SEC
}

pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}

pub fn tick_ms_translate(time: usize) -> usize{
    time / (CLOCK_FREQ / MSEC_PER_SEC)
}
