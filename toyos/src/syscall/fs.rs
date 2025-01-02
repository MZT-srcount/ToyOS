
use alloc::borrow::ToOwned;
 
use crate::task::{suspend_and_rnext,current_user_token,exit_and_rnext,current_task, TaskControlBlockInner};
use crate::memory::{get_data_buffer,translated_refmut, translated_str, UserBuffer, translated_byte_buffer};
use crate::fs::{make_pipe, OpenFlags, open, ch_dir, list_files, DiskInodeType,
    Dirent, FdSet, File, FileClass, FileDescripter, IoVec, IoVecs, Kstat, MNT_TABLE, NewStat};
use crate::timer::get_time_ms;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::mem::size_of;
use spin::MutexGuard;

/*
use crate::task::{current_user_token, current_task};
use crate::fs::{FileClass, Kstat, MNT_TABLE};
use crate::memory::{UserBuffer, translated_refmut, translated_str};
use crate::console::print;
*/
pub struct FStat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub __pad: u32,
    pub st_size: u32,
    pub st_blksize: u32,
    pub __pad2: i32,
    pub st_blocks: u64,
    pub st_atime_sec: u32,
    pub st_atime_nsec: u32,
    pub st_mtime_sec: u32,
    pub st_mtime_nsec: u32,
    pub st_ctime_sec: u32,
    pub st_ctime_nsec: u32,
    pub __unused: [u8; 2],
}

pub fn sys_fstat(fd: usize, vaddr: usize) -> isize{
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let mut fstat = Kstat::empty();
    let buf =  translated_byte_buffer(inner.memory_manager.token(), vaddr as *mut u8, size_of::<Kstat>());
    let mut userbuffer = UserBuffer::new(buf);
    if let Some(file) = inner.fd_table.get(fd){
        if let Some(file_clone) = file.clone(){
            match file_clone.fclass{
                FileClass::File(f) => {
                    f.get_fstat(&mut fstat);
                    //直接写入内存：
                    //inner.memory_manager.write_data(vaddr, Kstat);
                    userbuffer.write(fstat.as_bytes());
                }
                _ => {
                    userbuffer.write(Kstat::new_abstract().as_bytes());
                }
            }
            return 0;
        }
    }
    return -1;
}

//const char *special, const char *dir, const char *fstype, unsigned long flags, const void *data;
/*
pub fn sys_mount(special: usize, dir: usize, fstype: usize, flags: usize, data: usize) -> isize{
    /*
    let token = current_user_token();
    MNT_TABLE.unlock().mount(
        translated_str(token, special as *const u8), translated_str(token, dir as *const u8), translated_str(token, fstype as *const u8), flags as u32);
        */
}

pub fn sys_unmount(special: usize, flags: usize) -> isize{
    /*
    let token = current_user_token();
    MNT_TABLE.unlock().unmount(
        translated_str(token, special as *const u8), flags as u32);
        */
}
*/


const AT_FDCWD: isize = -100;
const FD_LIMIT: usize = 128;
const FD_StDOUT: usize = 1;
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let process = current_task().unwrap();
    let inner = process.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let f: Arc<dyn File + Send + Sync> = match &file.fclass {
            FileClass::Abstr(f)=> {f.clone()},
            FileClass::File(f)=>{f.clone()},
            _ => return -1,
        };
        if !f.writable() {
            return -1;
        }
        drop(inner);
        let size = f.write(
            UserBuffer::new(get_data_buffer(token, buf, len))
        );
        if fd == 2{
            let str = str::replace(translated_str(token, buf).as_str(), "\n", "\\n");
            ////gdb_println!(SYSCALL_ENABLE, "sys_write(fd: {}, buf: \"{}\", len: {}) = {}", fd, str, len, size);
        }
        else if fd > 2{
            ////gdb_println!(SYSCALL_ENABLE, "sys_write(fd: {}, buf: ?, len: {}) = {}", fd, len, size);
        }
        size as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();

    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file: Arc<dyn File + Send + Sync> = match &file.fclass {
            FileClass::Abstr(f)=> {f.clone()},
            FileClass::File(f)=>{f.clone()},
            _ => return -1,
        };
        if !file.readable() {
            return -1;
        }
        drop(inner);
        let size = file.read(
            UserBuffer::new(get_data_buffer(token, buf, len))
        );
        if fd > 2{
            //info!(SYSCALL_ENABLE, "sys_read(fd: {}, buf: ?, len: {}) = {}", fd, len, size);
        }
        size as isize
    } else {
        -1
    }
}


pub fn sys_mkdir(dirfd:isize, path: *const u8, mode:u32)->isize{
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let path = translated_str(token, path);
    if dirfd == AT_FDCWD {
        let work_path = inner.current_path.clone();
        if let Some(inode) = open(
            inner.get_work_path().as_str(),
            path.as_str(),
            OpenFlags::CREATE,
            DiskInodeType::Directory
        ) {
            return 0
        } else {
            return -1
        }
    } else {
        let fd_usz = dirfd as usize;
        if fd_usz >= inner.fd_table.len() && fd_usz > FD_LIMIT {
            return -1
        }
        if let Some(file) = &inner.fd_table[fd_usz] {
            match &file.fclass {
                FileClass::File(f) => {
                    if let Some(new_dir) = f.create(path.as_str(), DiskInodeType::Directory){
                        return 0;
                    }else{
                        return -1;
                    }
                },
                _ => return -1,
            }
        } else {
            return -1
        }
    }
}


pub fn sys_openat(dirfd: isize, path: *const u8, flags: u32, mode: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    // 这里传入的地址为用户的虚地址，因此要使用用户的虚地址进行映射
    let path = translated_str(token, path);
    let mut inner = task.inner_exclusive_access();
    
    let oflags = OpenFlags::from_bits(flags).unwrap();
    if dirfd == AT_FDCWD {
        if let Some(inode) = open(
            inner.get_work_path().as_str(),
            path.as_str(),
            oflags,
            DiskInodeType::File
        ) {
            let fd = inner.fd_alloc();
            inner.fd_table[fd] = Some( FileDescripter::new(
                oflags.contains(OpenFlags::CLOEXEC),
                FileClass::File(inode)
            ));
            fd as isize
        } else {
            //panic!("open failed");
            -1
        }
    } else {    
        let fd_usz = dirfd as usize;
        if fd_usz >= inner.fd_table.len() && fd_usz > FD_LIMIT {
            return -1
        }
        if let Some(file) = &inner.fd_table[fd_usz] {
            match &file.fclass {
                FileClass::File(f) => {
                    if oflags.contains(OpenFlags::CREATE){ 
                        if let Some(tar_f) = f.create(path.as_str(), DiskInodeType::File){ 
                            let fd = inner.fd_alloc();
                            inner.fd_table[fd] = Some( FileDescripter::new(
                                oflags.contains(OpenFlags::CLOEXEC),
                                FileClass::File(tar_f)
                            ));
                            return fd as isize
                        }else{
                            return -1;
                        }
                    }
                    // 正常打开文件
                    if let Some(tar_f) = f.find(path.as_str(), oflags){
                        let fd = inner.fd_alloc();
                        inner.fd_table[fd] = Some( FileDescripter::new(
                            oflags.contains(OpenFlags::CLOEXEC),
                            FileClass::File(tar_f)
                        ));
                        fd as isize
                    }else{
                        return -1;
                    }
                },
                _ => return -1, 
            }
        } else {
            return -1
        }
    }
    
}


pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

pub fn sys_getcwd(buf: *mut u8, len: usize)->isize{
    let token = current_user_token();
    let task = current_task().unwrap();
    let buf_vec = get_data_buffer(token, buf, len);
    let inner = task.inner_exclusive_access();
    let mut userbuf = UserBuffer::new(buf_vec);
    let current_offset:usize = 0;
    if buf as usize == 0 {
        return 0
    } else {
        let cwd = inner.current_path.as_bytes();
        userbuf.write( cwd );
        return buf as isize
    }
}

pub fn sys_dup(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    let new_fd = inner.fd_alloc();
    inner.fd_table[new_fd] = Some(inner.fd_table[fd].as_ref().unwrap().clone());
    new_fd as isize
}

pub fn sys_dup3( old_fd: usize, new_fd: usize )->isize{
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    if  old_fd >= inner.fd_table.len() || new_fd > FD_LIMIT {
        return -1;
    }
    if inner.fd_table[old_fd].is_none() {
        return -1;
    }
    if new_fd >= inner.fd_table.len() {
        for i in inner.fd_table.len()..(new_fd + 1) {
            inner.fd_table.push(None);
        }
    }
    inner.fd_table[new_fd] = Some(inner.fd_table[old_fd].as_ref().unwrap().clone());
    new_fd as isize
}

pub fn sys_pipe2(pipe: *mut u32, flags: usize) -> isize {
    if flags != 0{
        println!("[sys_pipe2]: flags not support");
    }
    let task = current_task().unwrap();
    let token = current_user_token();
    let mut inner = task.inner_exclusive_access();
    let (pipe_read, pipe_write) = make_pipe();
    let read_fd = inner.fd_alloc();
    inner.fd_table[read_fd] = Some( FileDescripter::new(
        false,
        FileClass::Abstr(pipe_read)
    ));
    let write_fd = inner.fd_alloc();
    inner.fd_table[write_fd] = Some( FileDescripter::new(
        false,
        FileClass::Abstr(pipe_write)
    ));
    *translated_refmut(token, pipe) = read_fd as u32;
    *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd as u32;
    //gdb_println!(SYSCALL_ENABLE, "sys_pipe2() = [{}, {}]", read_fd, write_fd);
    0
}

pub fn sys_chdir(path: *const u8) -> isize{
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let path = translated_str(token, path);
    let mut work_path = inner.current_path.clone();
    let new_ino_id = ch_dir(work_path.as_str(), path.as_str()) as isize;
    if new_ino_id >= 0 {
        //inner.current_inode = new_ino_id as u32;
        if path.chars().nth(0).unwrap() == '/' {
            inner.current_path = path.clone();
        } else {
            work_path.push('/');
            work_path.push_str(path.as_str());
            let mut path_vec: Vec<&str> = work_path.as_str().split('/').collect();
            let mut new_pathv: Vec<&str> = Vec::new();
            for i in 0..path_vec.len(){
                if path_vec[i] == "" || path_vec[i] == "." {
                    continue;
                }
                if path_vec[i] == ".." {
                    new_pathv.pop();
                    continue;
                }
                new_pathv.push(path_vec[i]);
            }
            let mut new_wpath = String::new();
            for i in 0..new_pathv.len(){
                new_wpath.push('/');
                new_wpath.push_str(new_pathv[i]);
            }
            if new_pathv.len() == 0 {
                new_wpath.push('/');
            }
            inner.current_path = new_wpath.clone();
        }
        new_ino_id
    }else{
        new_ino_id
    }
}

pub fn sys_getdents64(fd:isize, buf: *mut u8, len:usize)->isize{
    let token = current_user_token();
    let task = current_task().unwrap();
    let buf_vec = get_data_buffer(token, buf, len);
    let inner = task.inner_exclusive_access();
    let dent_len = size_of::<Dirent>();
    let mut total_len:usize = 0;
    let mut userbuf = UserBuffer::new(buf_vec);
    let mut dirent = Dirent::empty();
    if fd == AT_FDCWD {
        let work_path = inner.current_path.clone();
        if let Some(file) = open(
            "/",
            work_path.as_str(),
            OpenFlags::RDONLY,
            DiskInodeType::Directory
        ) {
            loop {
                if total_len + dent_len > len {break;}
                if file.getdirent(&mut dirent) > 0 {
                    userbuf.write_at( total_len, dirent.as_bytes());
                    total_len += dent_len;
                } else {
                    break;
                }
            }
            //gdb_println!(SYSCALL_ENABLE,"sys_getdents64(fd:{}, len:{}) = {}", fd, len, total_len );
            return total_len as isize;
        } else {
            //gdb_println!(SYSCALL_ENABLE,"sys_getdents64(fd:{}, len:{}) = {}", fd, len, -1 );
            return -1
        }
    } else {
        let fd_usz = fd as usize;
        if fd_usz >= inner.fd_table.len() && fd_usz > FD_LIMIT {
            return -1
        }
        if let Some(file) = &inner.fd_table[fd_usz] {
            match &file.fclass {
                FileClass::File(f) => {
                    loop {
                        if total_len + dent_len > len {break;}
                        if f.getdirent(&mut dirent) > 0 {
                            userbuf.write_at( total_len, dirent.as_bytes());
                            total_len += dent_len;
                        } else {
                            break;
                        }
                    }
                    //gdb_println!(SYSCALL_ENABLE,"sys_getdents64(fd:{}, len:{}) = {}", fd, len, total_len );
                    return total_len as isize; //warning
                },
                _ => {
                    //gdb_println!(SYSCALL_ENABLE,"sys_getdents64(fd:{}) = {}", fd, -1 );
                    return -1;
                }
            }
        } else {
            return -1
        }
    }
}

pub fn sys_mount( p_special:*const u8, p_dir:*const u8, p_fstype: *const u8, flags:usize, data: *const u8 )->isize{
    let token = current_user_token();
    let special = translated_str(token, p_special);
    let dir = translated_str(token, p_dir);
    let fstype = translated_str(token, p_fstype);
    MNT_TABLE.lock().mount(special, dir, fstype, flags as u32)
}

pub fn sys_umount( p_special:*const u8, flags:usize )->isize{
    let token = current_user_token();
    let special = translated_str(token, p_special);
    MNT_TABLE.lock().umount(special, flags as u32)
}

pub fn sys_unlink(fd:i32, path:*const u8, flags:u32)->isize{
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    let mut inner = task.inner_exclusive_access();
    if let Some(file) = get_file_discpt(fd as isize, & path, &inner, OpenFlags::from_bits(flags).unwrap()){
        match file {
            FileClass::File(f)=>{
                f.delete();
                return 0
            }
            _=> return -1
        }
    } else{
        return -1
    }
    
    -1
}


fn get_file_discpt(fd: isize, path:&String, inner: &MutexGuard<TaskControlBlockInner>, oflags: OpenFlags) -> Option<FileClass>{
    let type_ = {
        if oflags.contains(OpenFlags::DIRECTROY) {
            DiskInodeType::Directory
        } else {
            DiskInodeType::File
        }
    };
    if fd == AT_FDCWD {
        if let Some(inode) = open(
            inner.get_work_path().as_str(),
            path.as_str(),
            oflags,
            type_
        ) {
            //println!("find old");
            return Some(FileClass::File(inode))
        } else {
            return None
        }
    } else {    
        let fd_usz = fd as usize;
        if fd_usz >= inner.fd_table.len() && fd_usz > FD_LIMIT {
            return None
        }
        if let Some(file) = &inner.fd_table[fd_usz] {
            match &file.fclass {
                FileClass::File(f) => {
                    if oflags.contains(OpenFlags::CREATE){
                        if let Some(tar_f) = f.create(path.as_str(), type_){
                            return Some(FileClass::File(tar_f))
                        }else{
                            return None
                        }
                    }else{
                        if let Some(tar_f) = f.find(path.as_str(), oflags){
                            return Some(FileClass::File(tar_f))
                        }else{
                            return None
                        }
                    }
                },
                _ => return None, // 如果是抽象类文件，不能open
            }
        } else {
            return None
        }
    }
}

