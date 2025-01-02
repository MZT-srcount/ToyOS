#![no_std]
#![feature(llvm_asm)]
extern crate alloc;

mod blockdevice;
mod layout;
mod efs;
mod vfs;
mod block_cache;
#[macro_use]
mod console;
mod sbi;

pub const BLOCK_SZ:usize = 512;
pub use blockdevice::BlockDevice;
pub use vfs::VFile;
pub use layout::ShortDirEntry;
pub use efs::FAT32Manager;
pub use layout::*;
use block_cache::{get_block_cache,get_info_cache,write_to_dev,set_start_sec, CacheMode};

pub fn clone_into_array<A, T>(slice: &[T]) -> A where A: Default + AsMut<[T]>, T: Clone{
    let mut a = Default::default();
    <A as AsMut<[T]>>::as_mut(&mut a).clone_from_slice(slice);
    a
}


/*
可对外用接口：
=================================================================================
pub trait BlockDevice : Send + Sync + Any {
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    fn write_block(&self, block_id: usize, buf: &[u8]);
}
=================================================================================

================虚拟文件对象：vFile，为内核提供接口，屏蔽文件系统的内部细节=================
    pub struct VFile {
        name:String,
        short_sector: usize,
        short_offset: usize, //文件短目录项所在扇区和偏移
        long_pos_vec: Vec<(usize, usize)>, // 长目录项的位置<sector, offset>
        attribute:u8,  //文件属性，用于判断是目录还是文件
        fs: Arc<RwLock<FAT32Manager>>,
        block_device: Arc<dyn BlockDevice>,
    }
对外开放函数：
    pub fn new(//新建
        name:String,
        short_sector: usize,
        short_offset: usize,
        long_pos_vec:Vec<(usize, usize)>,
        attribute:u8,
        fs: Arc<RwLock<FAT32Manager>>,
        block_device: Arc<dyn BlockDevice>
    )->Self
    pub fn get_attribute(&self)->u8;
    pub fn get_size(&self)->u32;
    pub fn get_fs(&self) -> Arc<RwLock<FAT32Manager>>;
    pub fn is_dir(&self)->bool;    //判断是目录还是文件
    pub fn is_short(&self)->bool; //用于判断是短目录项或长目录项
    //读取短目录项
    pub fn read_short_dirent<V>(&self, f: impl FnOnce(&ShortDirEntry) -> V)->V;
    //修改短目录项
    fn modify_long_dirent<V>(&self, index:usize ,f: impl FnOnce(&mut LongDirEntry) -> V)->V;
    //修改短目录项
    pub fn modify_short_dirent<V>(&self, f: impl FnOnce(&mut ShortDirEntry) -> V)->V;
    pub fn get_pos(&self, offset:usize) -> (usize, usize);  //返回sector和offset
    fn find_long_name(&self,name: &str,dir_ent: &ShortDirEntry)->Option<VFile>;
    fn find_short_name(&self,name:&str,dir_ent: &ShortDirEntry) -> Option<VFile>;
    pub fn find_vfile_byname(&self,name: &str,) -> Option<VFile>; */
/* 根据名称搜索当前目录下的文件 *//*

     */
/* 根据路径递归搜索文件 *//*

    pub fn find_vfile_bypath(&self, path: Vec<&str>)-> Option<Arc<VFile>>;
     */
/* 在当前目录下创建文件 *//*

    pub fn create(& self, name: &str, attribute: u8) -> Option<Arc<VFile>>;
    pub fn first_cluster(&self)->u32;   //获取第一个簇
    pub fn set_first_cluster(&self, clu:u32);
     */
/* 获取当前目录下的所有文件名以及属性，以Vector形式返回 *//*

    pub fn ls_lite(&self)-> Option<Vec<(String, u8);
    // 获取目录中offset处目录项的信息, 返回<name, offset, firstcluster,attributes>
    pub fn dirent_info(&self, off:usize) -> Option<(String, u32, u32, u8)>;
    pub fn read_at(&self, offset: usize, buf: &mut [u8])->usize;
    pub fn write_at(&self, offset: usize, buf: &mut [u8])->usize;
    pub fn clear(&self);
     */
/* 查找可用目录项，返回offset，簇不够也会返回相应的offset，caller需要及时分配 *//*

    fn find_free_dirent(&self)->Option<usize>;
    pub fn creation_time(&self) -> (u32,u32,u32,u32,u32,u32,u64)// year-month-day-Hour-min-sec-long_sec
    pub fn accessed_time(&self) -> (u32,u32,u32,u32,u32,u32,u64);
    pub fn modification_time(&self) -> (u32,u32,u32,u32,u32,u32,u64);
==========================================================================

=====================短目录项：ShortDirEntry================================
pub struct ShortDirEntry{
    name: [u8;8],        // 删除时第0位为0xE5，未使用时为0x00. 有多余可以用0x20填充
    extension: [u8;3],
    attribute: u8,       //可以用于判断是目录还是文件
    winnt_reserved: u8,
    creation_tenths: u8, //精确到0.1s
    creation_time: u16,
    creation_date: u16,
    last_acc_date: u16,
    cluster_high: u16,
    modification_time: u16,
    modification_date: u16,
    cluster_low: u16,
    size: u32,
}
对外开放函数：
    //创建文件时调用，新建时不必分配块。写时检测初始簇是否为0，为0则需要分配。
    pub fn new(name_buffer: &[u8], exten: &[u8], attribute: u8) -> Self;
     */
/* 返回目前使用的簇的数量 *//*

    pub fn data_clusters(&self, bytes_per_cluster: u32)->u32;
    pub fn is_dir(&self)->bool;
    pub fn is_valid(&self)->bool;
    pub fn is_deleted(&self)->bool;
    pub fn is_empty(&self)->bool;
    pub fn is_file(&self)->bool;
    pub fn is_long(&self)->bool;
    pub fn attribute(&self)->u8;
    pub fn get_name_uppercase(&self)-> String;//获取短文件名
    pub fn get_name_lowercase(&self) -> String;
    pub fn checksum(&self)->u8;// 计算校验和
    //获取文件偏移量所在的簇、扇区和偏移
    pub fn get_pos(&self,offset:usize,manager: &Arc<RwLock<FAT32Manager>>,fat: &Arc<RwLock<FAT>>,block_device:&Arc<dyn BlockDevice>)
    ->(u32, usize, usize);
    // 以偏移量读取文件
    pub fn read_at(&self,offset: usize,buf: &mut [u8],manager: &Arc<RwLock<FAT32Manager>>,fat: &Arc<RwLock<FAT>>,
        block_device: &Arc<dyn BlockDevice>) -> usize
    //以偏移量写文件，这里会对fat和manager加读锁
    pub fn write_at(&self,offset: usize,buf: & [u8],manager: &Arc<RwLock<FAT32Manager>>, fat: &Arc<RwLock<FAT>>,
        block_device: &Arc<dyn BlockDevice>) -> usize {
==============================================================

===================:FAT32Manager
    pub struct FAT32Manager {
        block_device: Arc<dyn BlockDevice>,
        fsinfo: Arc<FSInfo>,
        sectors_per_cluster: u32,
        bytes_per_sector: u32,
        bytes_per_cluster: u32,
        fat: Arc<RwLock<FAT>>,
        root_sec: u32,
        #[allow(unused)]
        total_sectors: u32,
        vroot_dirent:Arc<RwLock<ShortDirEntry>>,
    }
    // 某个簇的第一个扇区
    pub fn first_sector_of_cluster(&self, cluster: u32) -> usize;
    // 第一个数据簇（ROOT）的扇区
    pub fn first_data_sector(&self)->u32;
    // 打开现有的FAT32/
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<RwLock<Self>>;
    // 分配簇，填写FAT，成功返回第一个簇号，失败返回None
    pub fn alloc_cluster(&self, num: u32)->Option<u32>;
    // 计算扩大至new_size(B)需要多少个簇
    pub fn cluster_num_needed(&self, old_size:u32, new_size:u32, is_dir: bool, first_cluster: u32)->u32;
    // 计算当前偏移量在第几个簇
    pub fn cluster_of_offset(&self, offset: usize)->u32;
    // 将长文件名拆分，并且补全0
    pub fn long_name_split(&self, name: &str)->Vec<String>;
    // 拆分文件名和后缀
    pub fn split_name_ext<'a>(&self, name: &'a str)->(&'a str, &'a str);
    // 将短文件名格式化为目录项存储的内容
    pub fn short_name_format(&self, name: &str)->([u8;8],[u8;3]);
    // 由长文件名生成短文件名
    pub fn generate_short_name(&self, long_name:&str)->String;
==============================================================

===================layout::*==================================
pub const FREE_CLUSTER:u32 = 0x00000000;
pub const END_CLUSTER:u32  = 0x0FFFFFF8;
pub const BAD_CLUSTER:u32  = 0x0FFFFFF7;
const FATENTRY_PER_SEC:u32 = BLOCK_SZ as u32/4;
#[allow(unused)]
pub const ATTRIBUTE_READ_ONLY:u8 = 0x01;
#[allow(unused)]
pub const ATTRIBUTE_HIDDEN   :u8 = 0x02;
#[allow(unused)]
pub const ATTRIBUTE_SYSTEM   :u8 = 0x04;
#[allow(unused)]
pub const ATTRIBUTE_VOLUME_ID:u8 = 0x08;
#[allow(unused)]
pub const ATTRIBUTE_DIRECTORY:u8 = 0x10;
#[allow(unused)]
pub const ATTRIBUTE_ARCHIVE  :u8 = 0x20;
#[allow(unused)]
pub const ATTRIBUTE_LFN      :u8 = 0x0F;
pub const DIRENT_SZ:usize = 32;
#[allow(unused)]
pub const SHORT_NAME_LEN:u32 = 8;
#[allow(unused)]
pub const SHORT_EXT_LEN:u32 = 3;
pub const LONG_NAME_LEN:u32 = 13;
pub const ALL_UPPER_CASE:u8 = 0x00;
pub const ALL_LOWER_CASE:u8 = 0x08;
pub fn next_free_cluster(&self, current_cluster:u32, block_device: Arc<dyn BlockDevice>)->u32;//获取下一个可用簇的簇号
pub fn get_next_cluster(&self, cluster: u32, block_device: Arc<dyn BlockDevice>) -> u32;// 获取当前簇的下一簇
pub fn set_next_cluster(&self, cluster:u32, next_cluster:u32, block_device: Arc<dyn BlockDevice>);// 设置当前簇的下一簇
pub fn get_cluster_at(&self, start_cluster:u32, index: u32, block_device: Arc<dyn BlockDevice>) -> u32;//获取某个簇链的第i个簇(i为参数)
pub fn final_cluster(&self, start_cluster:u32, block_device: Arc<dyn BlockDevice>)->u32;// 获取某个簇链的最后一个簇
pub fn get_all_cluster_of(&self, start_cluster:u32, block_device: Arc<dyn BlockDevice>)->Vec<u32>;// 获得某个簇链从指定簇开始的所有簇
=============================================================
 */











