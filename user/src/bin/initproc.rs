#![no_std]//移除标准库
#![no_main]//移除默认主函数

#[macro_use]//宏定义在其它的 crate 中。或者其它的 crate 子模块中,需要引入宏定义
extern crate user_lib;

//使用user_lib中的一些函数，可以用use来简化路径
use user_lib::{
    yield_,
    wait,
    fork,
    exec,
    waitpid,
};

static TESTS: &[&str] = &[
"clone\0",
"exit\0",
"fork\0",
"brk\0",
"execve\0",
"close\0",
"dup2\0",
"dup\0",
"fstat\0",
"getcwd\0",
"getdents\0",
"gettimeofday\0",
"mmap\0",
"mount\0",
"munmap\0",
"open\0",
"read\0",
"times\0",
"umount\0",
"uname\0",
"wait\0",
"write\0",
"yield\0",
"pipe\0",
"waitpid\0",
"chdir\0",
"openat\0",
"mkdir_\0",
"unlink\0",
];

#[no_mangle]
fn main() -> i32 {
    println!("I am in initproc before fork");
    /*
    //子进程就是交互界面输入
    let ret = fork();
    println!("ret: {}", ret);
    if(ret == 0)
    {
        println!("I am in initproc");
        exec("user_shell\0");
    }
    else {//当前初始进程
        loop{
            let mut exit_code=0;
       	    println!("get there?0");
            let pid=wait(&mut exit_code);//等待任何一个子进程返回
       	    println!("get there?1");
            if pid==-1//说明该进程不存在
            {
                //println!("get there?3");
                    yield_();
                //println!("get there?2");
                    continue;
            }
            //打印需要进行处理的僵尸进程
            println!( "[initproc] Released a zombie process, pid={}, exit_code={}",pid,exit_code);
        }
    }
    */
    for test in TESTS{
    	println!("Usertests: Running {}", test);
    	let pid = fork();
        if pid == 0 {
            println!("test name: {}", test);
            exec(*test);
            panic!("unreachable!");
        } else {
            let mut exit_code: i32 = Default::default();
            let wait_pid = waitpid(pid as usize, &mut exit_code);
            assert_eq!(pid, wait_pid);
            println!(
                "\x1b[32mUsertests: Test {} in Process {} exited with code {}\x1b[0m",
                test, pid, exit_code
            );
        }
    }
    println!("Usertests passed!");
    0
}
