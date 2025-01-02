mod pipe;
pub mod stdio;
mod inode;
mod mount;
pub mod finfo;
mod iovec;
use crate::memory::UserBuffer;
use alloc::sync::Arc; 

#[derive(Clone)]
pub struct FileDescripter{
    cloexec: bool,
    pub fclass: FileClass,
}

impl FileDescripter {
    pub fn new(cloexec:bool, fclass:FileClass)->Self{
        Self{
            cloexec,
            fclass
        }
    }
    pub fn set_cloexec(&mut self, flag: bool){
        self.cloexec = flag;
    }
    pub fn get_cloexec(& self) -> bool{
        self.cloexec
    }
}

#[derive(Clone)]
pub enum FileClass {
    File (Arc<OSInode>),
    Abstr (Arc<dyn File + Send + Sync>),
}

pub trait File : Send + Sync {
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn read(&self, buf: UserBuffer) -> usize;
    fn write(&self, buf: UserBuffer) -> usize;
    fn ioctl(&self, cmd: u32, arg: usize)-> isize {0}
    fn r_ready(&self)->bool{true}
    fn w_ready(&self)->bool{true}

}

pub use mount::MNT_TABLE;
pub use finfo::{Dirent, Kstat, NewStat, FdSet,  DT_DIR, DT_REG, DT_UNKNOWN, *};
pub use iovec::{IoVec, IoVecs};
pub use pipe::{Pipe, make_pipe};
pub use stdio::{Stdin, Stdout, _print};
pub use inode::{OSInode, open, clear_cache, init_rootfs, OpenFlags,ch_dir, list_files,DiskInodeType};

/* ============================MNT_TABLE========================================
pub static ref MNT_TABLE: Arc<Mutex<MountTable>> = {
        let mnt_table = MountTable {
            mnt_list: Vec::new(),
        };
        Arc::new(Mutex::new( mnt_table ))
};
============================Pipe========================================
pub struct Pipe {
    readable: bool,
    writable: bool,
    buffer: Arc<Mutex<PipeRingBuffer>>,
}
pub fn read_end_with_buffer(buffer: Arc<Mutex<PipeRingBuffer>>) -> Self;
pub fn write_end_with_buffer(buffer: Arc<Mutex<PipeRingBuffer>>) -> Self;
========================================================================
pub fn make_pipe() -> (Arc<Pipe>, Arc<Pipe>);
============================iovec=======================================
pub struct IoVec {
    base: *mut u8,
    len: usize,
}
pub struct IoVecs(pub Vec<&'static mut [u8]>);
pub unsafe fn new(iov_ptr: *mut IoVec,iov_num: usize,token: usize,)-> Self;
============================inode:OSInode=======================================
pub struct OSInode {
    readable: bool,
    writable: bool,
    inner: Mutex<OSInodeInner>,
}
pub fn new( readable: bool,writable: bool,inode: Arc<VFile>,) -> Self;
pub fn read_vec(&self, offset:isize, len:usize)->Vec<u8>;
pub fn read_all(&self) -> Vec<u8>;
pub fn write_all(&self, str_vec:&Vec<u8>)->usize;
pub fn getdirent(&self, dirent: &mut Dirent )->isize;
pub fn get_fstat(&self, kstat:&mut Kstat);
pub fn get_newstat(&self, stat:&mut NewStat);
pub fn create(&self, path:&str, type_: DiskInodeType)->Option<Arc<OSInode>>;
pub fn lseek(&self, offset: isize, whence: i32)->isize;
============================inode:=================================================
pub fn open(work_path: &str, path: &str, flags: OpenFlags, type_: DiskInodeType) -> Option<Arc<OSInode>>;
pub fn ch_dir(work_path: &str, path: &str) -> isize;
pub fn list_files(work_path: &str, path: &str);
============================inode:OpenFlags=====================================
    pub struct OpenFlags: u32 {
        const RDONLY = 0;
        const WRONLY = 1 << 0;
        const RDWR = 1 << 1;
        const CREATE = 1 << 6;
        const TRUNC = 1 << 10;
        const DIRECTROY = 0200000;
        const LARGEFILE  = 0100000;
        const CLOEXEC = 02000000;
    }
    pub fn read_write(&self) -> (bool, bool);
============================inode:DiskInodeType=====================================
pub enum DiskInodeType {
    File,
    Directory,
}
===================================================================================== */


