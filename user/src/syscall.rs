#![feature(asm)]

use core::arch::asm;


pub const SYSCALL_GETCWD: usize = 17; //获取当前工作目录
pub const SYSCALL_PIPE2: usize = 59; //创建管道
pub const SYSCALL_DUP: usize = 23; //复制文件描述符
pub const SYSCALL_CHDIR: usize = 49; //切换工作目录
pub const SYSCALL_DUP3: usize = 24; //复制文件描述符，并指定新的文件描述符
pub const SYSCALL_chdir: usize = 49; //切换工作目录
pub const SYSCALL_OPENAT: usize = 56; //打开或创建一个文件
pub const SYSCALL_CLOSE: usize = 57; //关闭一个文件描述符
pub const SYSCALL_GETDENTS64: usize = 61; //获取目录条目
pub const SYSCALL_READ: usize = 63; //从一个文件描述符中读取
pub const SYSCALL_WRITE: usize = 64; //从一个文件描述符写入
pub const SYSCALL_LINKAT: usize = 37; //创建文件的链接
pub const SYSCALL_UNLINKAT: usize = 35; //移除制定文件的链接（可用于删除文件）
pub const SYSCALL_MKDIRAT: usize = 34; //创建目录
pub const SYSCALL_UMOUNT2: usize = 39; //卸载文件系统
pub const SYSCALL_MOUNT: usize = 40; //挂载文件系统
pub const SYSCALL_FSTAT: usize = 80; //获取文件状态
pub const SYSCALL_FORK: usize = 220; //创建一个子进程
pub const SYSCALL_EXECVE: usize = 221; //执行一个指定的程序
pub const SYSCALL_WAIT4: usize = 260; //等待进程改变状态
pub const SYSCALL_EXIT: usize = 93;  //触发进程终止，无返回值
pub const SYSCALL_GETPID : usize = 172;
pub const SYSCALL_GETPPID : usize = 173; //获取父进程ID
pub const SYSCALL_BRK: usize = 214; //修改数据段的大小
pub const SYSCALL_MUNMAP: usize = 215; //将文件或设备取消映射到内存中
pub const SYSCALL_MMAP: usize = 222; //将文件或设备映射到内存中
pub const SYSCALL_TIMES: usize = 153; //获取进程时间
pub const SYSCALL_UNAME: usize = 160; //打印系统信息
pub const SYSCALL_YIELD: usize = 124; //让出调度器
pub const SYSCALL_GET_TIMEOFDAY:usize = 169; //获取时间
pub const SYSCALL_NANOSLEEP: usize = 101; //执行线程睡眠，sleep()库函数都基于此系统调用 




fn syscall(id: usize, args: [usize; 7]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x13") args[3],
            in("x14") args[4],
            in("x15") args[5],
            in("x16") args[6],
            in("x17") id
        );
    }
    ret
}

pub fn sys_read(fd:usize,buffer:&mut[u8])->isize{
    syscall(SYSCALL_READ, [fd, buffer.as_mut_ptr() as usize, buffer.len(), 0, 0, 0, 0])
}
pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len(), 0, 0, 0, 0])
}

pub fn sys_exit(exit_code: i32) -> isize {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0, 0, 0, 0, 0])
}

pub fn sys_yield() -> isize {
    syscall(SYSCALL_YIELD, [0, 0, 0, 0, 0, 0, 0])
}


pub fn sys_fork()->isize{
    syscall(SYSCALL_FORK,[0, 0, 0, 0, 0, 0, 0])
}

pub fn sys_waitpid(pid:isize,exit_code:*mut i32)->isize{
    syscall(SYSCALL_WAIT4, [pid as usize, exit_code as usize,0, 0, 0, 0, 0])
}

pub fn sys_exec(path:&str)->isize{
    // println!("app name in user space is {}",path);
    syscall(SYSCALL_EXECVE, [path.as_ptr() as usize,0,0, 0, 0, 0, 0])
}

pub fn sys_getpid() -> isize {
    syscall(SYSCALL_GETPID, [0, 0, 0, 0, 0, 0, 0])
}
pub fn sys_getppid() -> isize {
    syscall(SYSCALL_GETPPID, [0, 0, 0, 0, 0, 0, 0])
}


pub fn sys_gettimeofday(timespec: usize) -> isize{
    syscall(SYSCALL_GET_TIMEOFDAY, [timespec as usize, 0, 0, 0, 0, 0, 0])
}


pub fn sys_times(tms: usize) -> isize{
    syscall(SYSCALL_TIMES, [tms as usize, 0, 0, 0, 0, 0, 0])
}

pub fn sys_mmap(start: usize, len: usize, prot: u8, flags: usize, fd: usize, off: usize) -> isize{
    syscall(SYSCALL_MMAP, [start, len, prot as usize, flags, fd, off, 0])
}

pub fn sys_munmap(start: usize, len: usize) -> isize{
    syscall(SYSCALL_MUNMAP, [start, len, 0, 0, 0, 0, 0])
}

pub fn sys_uname(utsname: usize) -> isize{
    syscall(SYSCALL_UNAME, [utsname as usize, 0, 0, 0, 0, 0, 0])
}

pub fn sys_nanosleep(request: usize, remain: usize) -> isize{
    syscall(SYSCALL_NANOSLEEP, [request as usize, remain as usize, 0, 0, 0, 0, 0])
}