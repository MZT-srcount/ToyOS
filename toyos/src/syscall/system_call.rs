/*
 * 系统调用分类处理
 */
use super::config::*;
use super::interface::*;
use super::process::*;
use super::fs::*;

pub fn syscall(syscall_id: usize, args: [usize; 7]) -> isize {
    match syscall_id{
        SYS_WRITE => {sys_write(args[0], args[1] as *const u8, args[2])},
        SYS_EXIT => {sys_exit(args[0] as i32)},
        SYS_SCHED_YIELD => {sys_yield()},
        SYS_GET_TIMEOFDAY => {sys_gettimeofday(args[0] as usize)},
        SYS_UNLINKAT=> {
            sys_unlink(args[0] as i32, args[1] as *const u8, args[2] as u32)
        },
        SYS_GETCWD => {
            sys_getcwd(args[0] as *mut u8, args[1] as usize)
        },
        SYS_PIPE2 => {
            sys_pipe2(args[0] as *mut u32, args[1] as usize)
        },
        SYS_DUP => {
            sys_dup(args[0])
        },
        SYS_MKDIRAT =>{
            sys_mkdir(args[0] as isize, args[1] as *const u8, args[2] as u32)
        }
        SYS_DUP3 => {
            sys_dup3(args[0] as usize, args[1] as usize)
        },
        SYS_CHDIR => {
            sys_chdir(args[0] as *const u8)
        },
        SYS_OPENAT => {
            sys_openat(args[0] as isize, args[1] as *const u8, args[2] as u32, args[3] as u32)
        },
        SYS_CLOSE => {
            sys_close(args[0])
        },
        SYS_GETDENTS64 => {
            sys_getdents64(args[0] as isize, args[1] as *mut u8, args[2] as usize)
        },
        SYS_READ => {
            sys_read(args[0], args[1] as *const u8, args[2])
        },
        SYS_UMOUNT2 => {
            sys_umount(args[0] as *const u8, args[1] as usize)
        },
        SYS_MOUNT => {
            sys_mount(args[0] as *const u8, args[1] as *const u8, args[2] as *const u8, args[3] as usize, args[4] as *const u8)
        },
        SYS_FSTAT => {
            sys_fstat(args[0] as usize, args[1] as usize)
        },
        SYS_CLONE => {
            sys_fork(args[0] as usize, args[1] as  usize, args[2] as  usize, args[3] as  usize, args[4] as usize)
        },
        SYS_EXECVE => {
            sys_exec(args[0] as *const u8)
        },
        SYS_WAIT4 => {
            sys_waitpid(args[0] as isize,args[1] as * mut i32,args[2])
        },
        SYS_EXIT => {
            sys_exit(args[0] as i32)
        },
        SYS_GETPPID => {
            sys_getppid()
        },
        SYS_GETPID => {
            sys_getpid()
        },
        SYS_BRK => {
            sys_brk(args[0] as usize)
        },
        SYS_MUNMAP => {
            sys_munmap(args[0] as usize, args[1] as usize)
        },
        SYS_MMAP => {
            sys_mmap(args[0] as usize, args[1] as usize, args[2] as u8, args[3] as usize, args[4] as usize, args[5] as usize)
        },
        SYS_TIMES => {
            sys_times(args[0] as usize)
        },
        SYS_UNAME => {
            sys_uname(args[0] as usize)
        },
        SYS_NANOSLEEP => {
            sys_nanosleep(args[0] as usize, args[1] as usize)
        },
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
