mod heap_allocator;
mod page_table;
mod memory_manager;
mod phy_frame;
mod address;

pub use page_table::{PageTable, PTE, PageBit, translated_array_copy, get_data_buffer,translated_refmut,translated_str, UserBuffer, translated_byte_buffer};
pub use phy_frame::{Phy_PageFrame, frame_alloc, frame_dealloc, add_reftime, reftime, Frame_Manager};
pub use memory_manager::{MemoryManager, KERNEL_SPACE, SectionBit};
pub use address::*;

pub fn init() {
    heap_allocator::init_heap();
    phy_frame::init_frame_manager();
    KERNEL_SPACE.lock().activate();
    info!("[kernel]memory init succeed!");
}

pub fn kernel_token() -> usize{
  KERNEL_SPACE.lock().token()
}
pub fn othercore_init(){
    KERNEL_SPACE.lock().activate();
}
/*
 * 可用对外接口：
 *
 * ====================页表项结构：PTE===========================
 * pub struct PTE{
    flag: usize
}
 * PTE对外开放函数：
 *  /*获取物理页帧值*/
 *  pub fn ppn(&self) -> usize
    /*各种标志位判断*/
    pub fn is_valid(&self) -> bool//判断是否合法
    pub fn is_writable(&self) -> bool//判断是否可写
    pub fn is_executable(&self) -> bool//判断是否可执行
    pub fn is_empty(&self) -> bool//判断是否为空
    
    /*标志位设置*/
    pub fn set_flag(&mut self, flag: usize)//flag为页表项标志位，注意：flag采用直接赋值而非 与 策略，输入flag后原先flag清除
    pub fn set_valid(&mut self)//设置为合法，合法性是包括读写等一切可用性的前提
 *
 * ==============================================================
 *
 *
 * =====================页表结构：PageTable======================
 * pub struct PageTable{
    root_ppn: usize,
    frames: Vec<usize>,
}
 * PageTable对外开放函数：
 *
 * pub fn new() -> Self //新建页表
 *
 * pub fn from_token(satp: usize) -> Self//根据satp生成页表，也即通过给定地址在对应地址处建立页表,由于未对目标地址做如清除等任何处理，故可用于查询，如：知道某地址是页表，但没有页表结构，可通过from_token来获取页表结构从而使用相关函数
   输入：
   	satp:根目录地址，物理地址
 *
 * pub fn find_pte(&self, virt_pn: usize) -> Option<&PTE>//根据虚拟页号查找页表项，失败None
   输入：
   	virt_pn:虚拟页号
   输出:
   	Option<&PTE>:页表项，失败返回None
 *
 * pub fn map(&mut self, vpn: usize, ppn: usize, flags: usize)//虚拟页号到物理页号的映射
   输入：
   	vpn:虚拟页号
   	ppn：物理页号
   	flag:标志位，采用直接覆盖的方式
 *
 * pub fn unmap(&mut self, vpn: usize)//清空某个页表项，并取消映射某个虚拟页号-物理页号
   输入：
   	vpn:虚拟页号
 *
 * pub fn virt_to_phy(&self, vpn: usize) -> Option<PTE>//根据虚拟页号返回对应页表项的复制，失败返回None
   输入：
   	vpn:虚拟页号
   输出：
   	Option<PTE>:页表项，查询失败返回None
 *
 * pub fn token(&self) -> usize //satp值获取，即对应的根目录地址
   输出：
   	usize:satp值
 
 *
 * ==============================================================
 *
 *
 * ================页表项标志位flag===============================
     #[allow(dead_code)]
     pub enum PageBit{
         Valid = 1 << 0,
         Read  = 1 << 1,
         Write = 1 << 2,
         Execute = 1 << 3,
         User  = 1 << 4,
         Global = 1 << 5,
         Access = 1 << 6,
         Dirty = 1 << 7,
     }

 * 对外开放函数：pub fn val(self) -> usize//获取定义的PageBit的usize值
   输出：
   	usize:PageBit格式对应的usize,如：PageBit::Valid.val()则返回1
 * ===============================================================
 *
 * ====================其他对外函数接口===========================
 *
 * pub fn get_data_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]>
 * //获取缓冲区数据，并以字节数组形式输出
   输入：
   	token:相当于satp，用户的根目录地址
   	ptr  :缓冲区起始虚拟地址
   	len  :缓冲区长度，注意：非页数
   输出：
   	Vec<&'static mut [u8]>：字节数组
 *
 * pub fn init_frame_manager() //初始化页帧管理器，不过貌似对外用不上
 * 
 * pub fn frame_alloc() -> Option<Phy_PageFrame> //分配一个物理页帧，返回物理页帧号，失败返回None(无空闲页帧）
 *
 * pub fn frame_dealloc(ppn: usize) //回收一个物理页帧
   输入：
 	ppn:物理地址	
 *
 * ===============================================================
 *
 * ===================数据段结构：Map_Section=====================
 * pub struct Map_Section{
    vpn_start: usize,//起始虚拟页号
    vpn_end: usize,//末尾虚拟页号
    data_frames: BTreeMap<usize, Phy_PageFrame>,//已分配的页帧，虚拟页号-物理页号
    map_type: MapType,//映射类型，包括Identical恒等映射和Framed随即映射
    flag: usize,//访问方式，格式同PageBit,包括读、写、可执行、合法等
}
 *
 * 注：每个段均有一个Map_Section，如.text段
 *
 *可用对外接口：
 * pub fn new(start_va: usize,end_va: usize,map_type: MapType,flag: usize) -> Self//新建
   输入：
   	start_va:起始虚拟！地址 ！
   	end_va  :末尾虚拟！地址 ！
   	map_type:映射类型
   	flag    :区域类型
 *
 * pub fn map_one(&mut self, page_table: &mut PageTable, vpn: usize)//单页映射
   输入：
   	PageTable: 起始页表
   	vpn：需要映射的虚拟页号
 * 
 * pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: usize)//清除某个页表项
    输入：同map_one
 *
 * pub fn map(&mut self, pagetable: &mut PageTable)//在指定页表中映射map_section中的包括vpn_start到vpn_end的所有页面
    输入：
    	PageTable:同上
 *
 * pub fn unmap(&mut self, page_table: &mut PageTable)//清除map_section范围内在pagetable的所有页表项
    输入：
    	PageTable:同上
 *
 * pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8])//从vpn_start开始，通过page_table获取物理页号，然后将数据一页一页复制到内存，
    输入：
    	PageTable:对应的页表
    	data:想要保存的数据
 *
 * =================================================================
 *
 * ===================内存管理器：MemoryManager=====================
 *
 * pub struct MemoryManager{//一般而言，一个memorymanager用于一个程序
    page_table: PageTable,//页表
    sections: Vec<Map_Section>,
}
 * 注：每个应用均有一个内存管理器，保存每个段和该应用页表
 *
 * 可用对外接口：
 *
 * pub fn new() -> Self//新建
 *
 * pub fn map_trampoline(&mut self) //映射跳板
 *
 * pub fn insert_maparea(&mut self, start_vir: usize, end_vir: usize, flag: usize)//新增映射区域，主要是增加一个map_section
   输入：
   	start_vir:虚拟地址（注意虚拟地址与虚拟页号的区别！）
 *
 * pub fn push(&mut self, mut map_section: Map_Section, data: Option<&[u8]>)//将数据映射进对应段中（此处同mapsection的map操作），同时将map_section加入manager中
 *
 * pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) //从elf中读取数据
   输入：
   	elf_data:elf文件数据
   输出：
   	（自身（一个程序一个manager),用户栈栈顶，程序入口地址）//内核栈指针因为不是部署在用户空间，故不会在此处生成
 *
 * pub fn virt_find_pte(&self, vpn: usize) -> Option<PTE> //根据虚拟页号查找该应用空间中的页表项，失败返回None
 *
 * pub fn token(&self) -> usize //获取该应用程序的satp值（即根目录地址），因为内核中为恒等映射，故此地址无二义
 *
 *======================================================================
 *
 *已声明的内核的内存管理器：KERNEL_SPACE
 */

