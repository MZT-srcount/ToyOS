use crate::config::{
    MEMORY_END,
    PAGE_SIZE,
    TRAMPOLINE,
    TRAP_CONTEXT,
    USER_STACK_SIZE,
    PAGE_SIZE_BITS, self
};
use core::iter::Map;
use core::mem::size_of;
use core::arch::asm;
use super::{PageTable, PTE, PageBit, translated_byte_buffer, UserBuffer};
use super::{frame_alloc, Phy_PageFrame,add_reftime, reftime};
use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use riscv::register::satp;
use alloc::sync::Arc;
use lazy_static::*;
use core::cmp::{min, max};
use spin::Mutex;
use crate::fs::{FileDescripter, FileClass, File};
use crate::config::*;


/*暂时使用两种映射方式*/
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType{
    Identical,
    Framed,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Content{
    UserStack,
    KernelStack,
    TrapContext,
    Other,
    MMIO,
}
bitflags!{
    pub struct SectionBit: u8{
        const Valid = 1 << 0;
        const Read  = 1 << 1;
        const Write = 1 << 2;
        const Execute = 1 << 3;
        const User = 1 << 4;//用于copyonwrite
    }
}

/*对应用数据段的管理*/
pub struct Map_Section{
    content: Content,//利用Content区分段，仅需修改少量代码，之前使用的代码几乎无需修改
    vpn_start: usize,
    vpn_end: usize,
    data_frames: BTreeMap<usize, Phy_PageFrame>,//每个已分配页面
    map_type: MapType,
    flag: SectionBit,
    writable: bool,
}

impl Map_Section{
    pub fn new(
        start_va: usize,
        end_va  : usize,
        map_type: MapType,
        content: Content,
        flag    : SectionBit) -> Self{
        Self{
            vpn_start: start_va / PAGE_SIZE,
            vpn_end: (end_va + PAGE_SIZE - 1)/ PAGE_SIZE,
            content: content,
            data_frames: BTreeMap::new(), 		
            map_type,
            flag,
            writable: flag.bits() & SectionBit::Write.bits() > 0,
        }
    }


    //单页映射，虚拟地址到物理地址的映射，分恒等映射和随机映射，恒等映射主要用于内核
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: usize){
        let ppn: usize;
        match self.map_type{
            MapType::Identical=>{
                ppn = vpn;
            }
            MapType::Framed=>{
                //println!("map begin..");
                let frame = frame_alloc().unwrap();
                //println!("map end..");
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
        }
        let flags = self.flag;
        //println!("vpn: {:#x}, ppn: {:#x	}", vpn, ppn);
        page_table.map(vpn, ppn, flags.bits().into());
    }
    
    #[allow(unused)]
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: usize){
        match self.map_type{
            MapType::Framed => {
                self.data_frames.remove(&vpn);
            }
            _ => {}
        }
        page_table.unmap(vpn);
    }
    
    /*映射区域内所有地址*/
    pub fn map(&mut self, pagetable: &mut PageTable){
        let mut l: usize = self.vpn_start;
        let r: usize = self.vpn_end;
        while l < r{
            self.map_one(pagetable, l);
            l += 1;
        }
    }
    pub fn unmap(&mut self, page_table: &mut PageTable){
        let mut l: usize = self.vpn_start;
        let r: usize = self.vpn_end;
        while l < r{
            self.unmap_one(page_table, l);
            l += 1;
        }
    }
    pub fn unmap_part(&mut self, vpn_start: usize, vpn_end: usize, page_table: &mut PageTable) -> Result<(), &'static str>{
        let mut l = vpn_start;
        let r = vpn_end;
        if(l < self.vpn_start || l > self.vpn_end || vpn_end < self.vpn_start || vpn_end > self.vpn_end){
            panic!("l:{}, self.vpn_start: {}; r:{}, self.vpn_end: {}",l, self.vpn_start, r, self.vpn_end);
            return Err("virtual address out of range, unmap failure");
        }
        while l < r{
            self.unmap_one(page_table, l);
            l += 1;
        }
        Ok(())
    }
    /*将区域内数据复制到内存*/
    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]){//问题：未使用data_framed
        assert_eq!(self.map_type, MapType::Framed);
        let mut start :usize = 0;
        let mut current_vpn = self.vpn_start;
        let len = data.len();
        loop{
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let array = 
            unsafe{core::slice::from_raw_parts_mut(
                    ((page_table.virt_to_phy(current_vpn).unwrap().ppn() * PAGE_SIZE) as *mut u8), 4096)};
            let dst = &mut array[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn += 1;
        }
    }
    pub fn contains_vpn(&self, vpn: usize) -> bool{
        if(self.data_frames.contains_key(&vpn)){
            return true;
        }
        false
    }
    //获取已映射的物理页号
    pub fn get_ppn(&self, vpn: usize) -> Option<usize>{
        if let Some(ppn) = self.data_frames.get(&vpn){
            return Some(ppn.ppn);
        }
        None
    }
    pub fn map_shared(section: &Map_Section, pt_self: &mut PageTable, pt_another: &mut PageTable) -> Self{
        //对已经映射的每一页共享，假设可能存在未全部映射
        let mut frames = BTreeMap::new();
        if(section.content == Content::Other)
        {
            /*
            for vpn in section.vpn_start..section.vpn_end{
                //println!("vpn: {:#x}", vpn);
                /*
                if(!section.contains_vpn(vpn)){
                    println!("can not obtain all vpn..");
                }
                */
                let new_frame = frame_alloc().unwrap();
                let new_ppn = new_frame.ppn;
                frames.insert(vpn, new_frame);
                let pte = pt_another.find_pte(vpn).unwrap();
                let flag = pte.get_flag();
                pt_self.map(vpn, new_ppn, flag);
                let src_addr = pte.ppn() << PAGE_SIZE_BITS;
                let dst_addr = new_ppn << PAGE_SIZE_BITS;
                unsafe{core::slice::from_raw_parts_mut(dst_addr as *mut u8, PAGE_SIZE)
                    .copy_from_slice(core::slice::from_raw_parts(src_addr as *mut u8, PAGE_SIZE))}
            }
            */
            for vpn in section.vpn_start..section.vpn_end{
                if(!section.data_frames.contains_key(&vpn)){
                    panic!("something wrong..");
                }
                let ppn = section.data_frames.get(&vpn).unwrap().ppn;
                frames.insert(vpn, Phy_PageFrame::new_for_share(ppn));
                let pte = pt_another.find_pte(vpn).unwrap();
                let flag = (pte.get_flag() as u8 & !SectionBit::Write.bits()) as usize;
                //println!("flag: {:#b}, old_flag: {:#b}, pte: {:#x}", flag, pte.get_flag(), pte.ppn());
                add_reftime(pte.ppn());
                pt_self.map(vpn, pte.ppn(), flag);//子进程不可写
                pt_another.set_flag(vpn, flag);//父进程也不可写
            }
            
            
        }
        else{
            
            //println!("vpn start: {:#x}, vpn end: {:#x}", section.vpn_start, section.vpn_end);
            /*
            for frame in &section.data_frames{
                let vpn = *frame.0;
                //println!("vpn: {:#x}", vpn);
                let new_frame = frame_alloc().unwrap();
                let new_ppn = new_frame.ppn;
                frames.insert(vpn, new_frame);
                let pte = pt_another.find_pte(vpn).unwrap();
                let flag = pte.get_flag();
                pt_self.map(vpn, new_ppn, flag);
                let src_addr = pte.ppn() << PAGE_SIZE_BITS;
                let dst_addr = new_ppn << PAGE_SIZE_BITS;
                println!("Pte: {:#x}", new_ppn);
                unsafe{core::slice::from_raw_parts_mut(dst_addr as *mut u8, PAGE_SIZE)
                    .copy_from_slice(core::slice::from_raw_parts(src_addr as *mut u8, PAGE_SIZE))}
            }
            */
            
            //println!("vpn start: {:#x}, vpn end: {:#x}", section.vpn_start, section.vpn_end);
            for vpn in section.vpn_start..section.vpn_end{
                //println!("vpn: {:#x}", vpn);
                if(!section.contains_vpn(vpn)){
                    panic!("can not obtain all vpn..");
                }
                let new_frame = frame_alloc().unwrap();
                let new_ppn = new_frame.ppn;
                frames.insert(vpn, new_frame);
                let pte = pt_another.find_pte(vpn).unwrap();
                let flag = pte.get_flag();
                pt_self.map(vpn, new_ppn, flag);
                let src_addr = pte.ppn() << PAGE_SIZE_BITS;
                let dst_addr = new_ppn << PAGE_SIZE_BITS;
                unsafe{core::slice::from_raw_parts_mut(dst_addr as *mut u8, PAGE_SIZE)
                    .copy_from_slice(core::slice::from_raw_parts(src_addr as *mut u8, PAGE_SIZE))}
            }
            
        }
        let sflag = section.flag;
        Self {
        content: section.content,
        vpn_start: section.vpn_start,
        vpn_end: section.vpn_end,
        data_frames: frames,
        map_type: section.map_type,
        flag: sflag,
        writable: section.writable,
        }
    }
    pub fn from_another(another: &Map_Section) -> Self {
        Self {
            content: another.content,
            vpn_start: another.vpn_start,
            vpn_end: another.vpn_end,
           data_frames: BTreeMap::new(),
           map_type: another.map_type,
           flag:another.flag,
           writable: another.writable,
       }
   }
   pub fn writable(&self) -> bool{
       self.writable
   }
   pub fn remap_one(&mut self, vpn: usize, pt: &mut PageTable){
        if(!self.data_frames.contains_key(&vpn)){
            panic!("can not find the page mapped before");
        }
        else{
            let mut ppn: usize;
            let old_frame = self.data_frames.get(&vpn).unwrap();
            let mut frame: Phy_PageFrame;
            match self.map_type{
                MapType::Identical=>{
                    ppn = vpn;
                }
                MapType::Framed=>{
                    frame = frame_alloc().unwrap();
                    ppn = frame.ppn;
                    let old_phyaddr = old_frame.ppn * PAGE_SIZE;
                    unsafe{
                        core::slice::from_raw_parts_mut((ppn << PAGE_SIZE_BITS) as *mut u8, PAGE_SIZE)
                        .copy_from_slice(core::slice::from_raw_parts_mut(old_phyaddr as *mut u8, PAGE_SIZE))
                    }
                    let flag = pt.find_pte(vpn).unwrap().get_flag();
                    pt.unmap(vpn);
                    pt.map(vpn, ppn, flag | PageBit::Write.val());
                    self.data_frames.remove(&vpn);
                    self.data_frames.insert(vpn, frame);
                }
            }
        }
   }
}
/*
 * 内存管理器
 * 每个应用的内存管理
 * page_table存储应用的页表、sections存储应用的各个部分的信息
 */
pub struct MemoryManager{
    page_table: PageTable,
    sections: Vec<Map_Section>,
    //heap: Vec<>
    vma: Vec<Map_Section>,//fd, map_section
}

impl MemoryManager{
    pub fn new() -> Self{
        Self{
            page_table: PageTable::new(),
            sections: Vec::new(),
            vma: Vec::new(),
        }
    }
    /*跳板，内核和应用的虚拟空间中跳板都一样*/
    pub fn map_trampoline(&mut self){
        //println!("map_trampoline..");
        self.page_table.map(
            TRAMPOLINE / PAGE_SIZE,
            strampoline as usize / PAGE_SIZE,
            (SectionBit::Read | SectionBit::Execute).bits().into(),
            );
    }
    
    /*插入映射区域,默认content类型为Other*/
    pub fn insert_maparea(&mut self, start_vir: usize, end_vir: usize, flag: SectionBit){
        self.push(
            Map_Section::new(
                start_vir,
                end_vir,
                MapType::Framed,
                Content::Other,
                flag,
            ), None);
    }
    
    /*内存映射*/
    pub fn push(&mut self, mut map_section: Map_Section, data: Option<&[u8]>){
        map_section.map(&mut self.page_table);
        if let Some(data) = data{
            map_section.copy_data(&mut self.page_table, data);
        }
        self.sections.push(map_section);
    }
    /*vma映射*/
    pub fn push_vma(&mut self, mut map_section: Map_Section){
        map_section.map(&mut self.page_table);
        self.vma.push(map_section);
    }
    /*压入共享段*/
    pub fn push_shared(&mut self, mut map_section: Map_Section){
        self.sections.push(map_section);
    }
    /*建立并映射内核虚拟空间
     *目前内核采用恒等映射
     *映射跳板、.text、.data、.rodata、.bss、其他区域
     */
    fn new_kernel() -> Self{
        let mut memory_manager = Self::new();
    	//println!("kernel_root_ppn: {}", memory_manager.page_table.root_ppn);
        println!("mapping .text section, stext{}, etext{}", stext as usize / PAGE_SIZE, etext as usize / PAGE_SIZE);
        memory_manager.map_trampoline();
        memory_manager.push(Map_Section::new(
            (stext as usize),
            (etext as usize),
            MapType::Identical,
            Content::Other,
            SectionBit::Read | SectionBit::Execute,
        ), None);
        println!("mapping .rodata section, srodata{}, erodata{}", srodata as usize / PAGE_SIZE, erodata as usize / PAGE_SIZE);
        memory_manager.push(Map_Section::new(
            (srodata as usize),
            (erodata as usize),
            MapType::Identical,
            Content::Other,
            SectionBit::Read,
        ), None);
        println!("mapping .data section");
        memory_manager.push(Map_Section::new(
            (sdata as usize),
            (edata as usize),
            MapType::Identical,
            Content::Other,
            SectionBit::Read | SectionBit::Write,
        ), None);
        println!("mapping .bss section");
        memory_manager.push(Map_Section::new(
            (sbss_with_stack as usize),
            (ebss as usize),
            MapType::Identical,
            Content::Other,
            SectionBit::Read | SectionBit::Write,
        ), None);
        println!("mapping physical memory");
        memory_manager.push(Map_Section::new(
            (ekernel as usize),
            MEMORY_END,
            MapType::Identical,
            Content::Other,
            SectionBit::Read | SectionBit::Write,
        ), None);

        
        println!("map MMIO");
        for addr in MMIO{
        memory_manager.push(Map_Section::new(
            (*addr).0,
            (*addr).0 + (*addr).1,
            MapType::Identical,
            Content::MMIO,
            SectionBit::Read | SectionBit::Write,
        ), None);
        }
        
        println!("map succeed");

        memory_manager
    }
    /*从elf中读取数据*/
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize, usize) {
        //println!("new task from elf..");
        let mut memory_set = Self::new();
        
        memory_set.map_trampoline();
        
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = 0;
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: usize = ph.virtual_addr() as usize;
                let end_va: usize = (ph.virtual_addr() + ph.mem_size()) as usize;
                let mut map_perm = SectionBit::User;
                let ph_flags = ph.flags();
                if ph_flags.is_read() { map_perm |= SectionBit::Read; }
                if ph_flags.is_write() { map_perm |= SectionBit::Write; }
                if ph_flags.is_execute() { map_perm |= SectionBit::Execute; }
                let map_section = Map_Section::new(
                    start_va,
                    end_va,
                    MapType::Framed,
                    Content::Other,
                    map_perm,
                );
                max_end_vpn = map_section.vpn_end;
                memory_set.push(
                    map_section,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize])
                );
            }
        }
        
        let max_end_va: usize = max_end_vpn * PAGE_SIZE;
        let mut user_heap_bottom: usize = max_end_va;
        user_heap_bottom += PAGE_SIZE;
        
        //println!("Trap_context: {:#x}", TRAP_CONTEXT);
        memory_set.push(Map_Section::new(
            TRAP_CONTEXT as usize,
            TRAMPOLINE as usize,
            MapType::Framed,
            Content::TrapContext,
            SectionBit::Read | SectionBit::Write,
        ), None);
        
        let safe_guard_high = TRAP_CONTEXT;//保护页
        let safe_guard_low = TRAP_CONTEXT - PAGE_SIZE;
        memory_set.push(Map_Section::new(
            safe_guard_low,
            safe_guard_high,
            MapType::Framed,
            Content::Other,
            SectionBit::Read | SectionBit::Write | SectionBit::User,), None
        );

        let mut user_stack_top = TRAP_CONTEXT - PAGE_SIZE;
        let mut user_stack_bottom = user_stack_top - USER_STACK_SIZE;//有栈溢出的可能
        memory_set.push(Map_Section::new(
            user_stack_bottom,
            user_stack_top,
            MapType::Framed,
            Content::UserStack,
            SectionBit::Read | SectionBit::Write | SectionBit::User,
        ), None);
        (memory_set, user_heap_bottom, user_stack_top, elf.header.pt2.entry_point() as usize)
    }
    /*多级页表机制使能*/
    pub fn activate(&self){
        let satp = self.page_table.token();
        //println!("value:{:#x}", satp);
        unsafe{
            satp::write(satp);
            asm!("sfence.vma");
        }
        //println!(".text posi:{:#x}",self.page_table.find_pte(0x8020b).unwrap().ppn());
    }

    pub fn mmap_file(&mut self, start_vir: usize, len: usize, file_option: &mut Option<FileDescripter>, offset: usize) -> isize{
        let vpn = start_vir >> PAGE_SIZE_BITS;
        let mut idx: usize = self.vma.len();
            for i in 0..self.vma.len(){
                if(vpn >= self.vma[i].vpn_start && (vpn < self.vma[i].vpn_end || self.vma[i].vpn_end == self.vma[i].vpn_start)){
                    idx = i;
                    break;
                }
            }
            if(idx == self.vma.len()){
                return -1;
            }
            if let Some(file) = file_option{
                match &file.fclass {
                    FileClass::File(f)=>{
                        f.set_offset(offset);
                        if !f.readable(){
                            return -1;
                        }
                        let mut remain_len = len;
                        for vpn in self.vma[idx].vpn_start.. self.vma[idx].vpn_end{//对每一页进行映射，直到到达len长度为止
                            f.read(UserBuffer::new(translated_byte_buffer(self.token(), (vpn << PAGE_SIZE_BITS) as *const u8, 
                        if(remain_len > PAGE_SIZE){
                            remain_len -= PAGE_SIZE;
                            PAGE_SIZE
                        }
                        else{
                            let final_len = remain_len;
                            remain_len = 0;
                            final_len
                        }
                        )));
                        if(remain_len == 0){
                            break;
                        }
                        }
                    },
                    _ => {return -1},
                }
            }
            1
    }
    /*映射文件或设备*/
    pub fn map_vma(&mut self, start_vir: usize, len: usize, offset: usize, flag: SectionBit){
        self.push_vma(
            Map_Section::new(
                start_vir,
                start_vir + len,
                MapType::Framed,
                Content::Other,
                flag,
            ));
    }
    /*虚拟页号查找页表项*/
    pub fn virt_find_pte(&self, vpn: usize) -> Option<PTE>{
        self.page_table.virt_to_phy(vpn)
    }
    pub fn remap_one(&mut self, vpn: usize, idx: usize){//重新映射页面，如果reftime == 1且该段可写则直接将页表项改为可写
        if(self.sections[idx].writable())
        {
            let ppn = self.sections[idx].get_ppn(vpn);
            if ppn.is_none(){
                panic!("can not find ppn in section");
            }
            if reftime(ppn.unwrap()) == 1{
                let flag = self.page_table.find_pte(vpn).unwrap().get_flag();
                self.page_table.set_flag(vpn, flag | PageBit::Write.val());
            }
            else{
                self.sections[idx].remap_one(vpn, &mut self.page_table);
            }
        }
        else{
            
            panic!("this section not allow to be write");
        }
        
    }
    /*satp值*/
    pub fn token(&self) -> usize {
        self.page_table.token()
    }
    //复制一个进程空间,用在fork中
    pub fn from_existed_user(user_space: &MemoryManager) -> MemoryManager {
        //println!("new task from existed user..");
        let mut memory_manager = Self::new();
        // map trampoline
        memory_manager.map_trampoline();
        // copy data sections/trap_context/user_stack,逻辑段的复制，会分配物理空间
        for area in user_space.sections.iter() {
            //此时还没有真正映射到物理页帧上
            let new_area = Map_Section::from_another(area);
            memory_manager.push(new_area, None);
            // copy data from another space，页面的映射
            for vpn in area.vpn_start..area.vpn_end {
                let src_ppn = user_space.virt_find_pte(vpn).unwrap().ppn();
                let dst_ppn = memory_manager.virt_find_pte(vpn).unwrap().ppn();
                //找到物理页号对应物理地址
                let src_phyAddr=src_ppn << PAGE_SIZE_BITS;
                let dst_phyAddr=dst_ppn << PAGE_SIZE_BITS;
                //将源物理地址页内容复制到当前
                unsafe{core::slice::from_raw_parts_mut(dst_phyAddr as *mut u8,PAGE_SIZE).copy_from_slice(
                    core::slice::from_raw_parts_mut(src_phyAddr as * mut u8,PAGE_SIZE));}
            }
        }
        //println!("succeed fork a new task from existed user..");
        memory_manager
    }
    pub fn copy_on_write(user_space: &mut MemoryManager) -> MemoryManager{
        
        //当前已在内存中的页面进行共享，后期加载的页面则各不相关
        let mut memory_manager = Self::new();
        memory_manager.map_trampoline();
        for area in user_space.sections.iter_mut(){
            if((area.flag & SectionBit::Write).bits() > 0)
            {
                area.writable = true;
            }
            //与existed_user不同，按已分配页面进行映射，假定非所有页面均在内存中(目前还是假设所有页面均在内存中)
            let new_area = Map_Section::map_shared(&area, &mut memory_manager.page_table, &mut user_space.page_table);
            
            memory_manager.push_shared(new_area);
        }
        memory_manager
        
    }
    pub fn read_data<T: Clone>(&mut self, init_vir: usize) -> T{
        let len = size_of::<T>();
        let mut data_user = Vec::new();
        let mut start_vir = init_vir;
        let end_vir = start_vir + len;
        while start_vir < end_vir{
            let offset = start_vir % PAGE_SIZE;
            let mut pte: &PTE;
            let vpn = start_vir >> PAGE_SIZE_BITS;
            if let option = self.page_table.find_pte(vpn){
                pte = option.unwrap();
            }
            else{//页面被换回外存或地址有误，报错
                panic!("invalid address");
            }
            let pdata = unsafe{
                core::slice::from_raw_parts((pte.ppn() << PAGE_SIZE_BITS) as *mut u8, PAGE_SIZE)//既然可以获得数组，可否直接赋值？
            };
            data_user.push(&pdata[offset..(min(start_vir - offset + PAGE_SIZE - 1, end_vir) % PAGE_SIZE)]);
            if(start_vir % PAGE_SIZE != 0){
                start_vir = start_vir - start_vir % PAGE_SIZE;
            }
            start_vir += PAGE_SIZE;
        }
        let mut res: Vec<u8> = Vec::new();
        for elem in data_user{
            for offset in 0..elem.len(){
                res.push(elem[offset]);
            }
        }
        let data_T: &[u8] = &res; 
        unsafe{(data_T.as_ptr() as usize as *mut T).as_mut()}.unwrap().clone()
    }
    pub fn write_data<T>(&mut self, init_vir: usize, data: &T){//物理页并不连续，故需要进行拼接
        let len = size_of::<T>();
        let mut data_user = Vec::new();
        let mut start_vir = init_vir;
        let end_vir = start_vir + len;
        while start_vir < end_vir{
            let offset = start_vir % PAGE_SIZE;
            let mut pte: &PTE;
            let vpn = start_vir >> PAGE_SIZE_BITS;
            if let option = self.page_table.find_pte(vpn){
                pte = option.unwrap();
            }
            else{//因为有页面被置换回外存的可能，故假设给定地址正确（需要在上一级进行检查地址），如果没有该页面，分配一个页面。
                //如果和内核页表合并，则内核有被恶意攻击的风险，此处因为未合并，最差自身程序被破坏
                if let sec_option = self.sections_belong(vpn) {
                    let sec_pair = sec_option.unwrap();
                    let idx = sec_pair.0;
                    if (!self.sections[idx].contains_vpn(vpn)){
                        panic!("memory mistake");
                    }
                    let frame = frame_alloc().unwrap();
                    self.page_table.map(vpn, frame.ppn, PageBit::Read.val() | PageBit::Write.val());
                    self.sections[idx].data_frames.insert(vpn, frame);
                    pte = self.page_table.find_pte(vpn).unwrap();
                }
                else{
                    panic!("invalid address");
                }
            }
            let pdata = unsafe{
                core::slice::from_raw_parts_mut((pte.ppn() << PAGE_SIZE_BITS) as *mut u8, PAGE_SIZE)//既然可以获得数组，可否直接赋值？
            };
            data_user.push(&mut pdata[offset..(min(start_vir - offset + PAGE_SIZE - 1, end_vir) % PAGE_SIZE)]);
            if(start_vir % PAGE_SIZE != 0){
                start_vir = start_vir - start_vir % PAGE_SIZE;
            }
            start_vir += PAGE_SIZE;
        }
        let iter = data as *const T as usize as *const u8;
        for elem in data_user.iter_mut(){
            unsafe{(*elem).copy_from_slice(core::slice::from_raw_parts(iter, core::mem::size_of_val(*elem)))};
            unsafe{iter.add(core::mem::size_of_val(*elem))};
        }
    //还需要考虑什么特殊情况？
    }
    pub fn recycle_data_pages(&mut self) {
        //println!("self.sections.clear()");
        self.sections.clear();
    }
    //用于回收逻辑段的内容
    pub fn remove_area_with_start_vpn(&mut self, start_vaddr: usize) {
        //println!("..unmap..");
        let start_vpn = start_vaddr / PAGE_SIZE;
        if let Some((idx, area)) = self
            .sections
            .iter_mut()
            .enumerate()
            .find(|(_, area)| area.vpn_start == start_vpn)
        {
            //println!("area.unmap..");
            area.unmap(&mut self.page_table);
            self.sections.remove(idx);
        }
    }
    //查询虚拟页所属段，查找成功则返回(段下标，段)，否则返回None
    pub fn sections_belong(& self, vpn: usize) -> Option<(usize, &Map_Section)>{//------------可以使用段落树进行优化
        for i in 0..self.sections.len(){
            if(vpn >= self.sections[i].vpn_start && (vpn < self.sections[i].vpn_end || self.sections[i].vpn_end == self.sections[i].vpn_start)){
                return Some((i, &self.sections[i]));
            }
        }
        None
    }

    pub fn drop_vma(&mut self, start_vir: usize, len: usize) -> Result<(), &'static str>{
        let start_vpn = start_vir >> PAGE_SIZE_BITS;
        let end_vpn = (start_vir + len) >> PAGE_SIZE_BITS;
        for idx in 0..self.vma.len(){
            if(start_vpn >= self.vma[idx].vpn_start && end_vpn <= self.vma[idx].vpn_end){
                if(start_vpn == self.vma[idx].vpn_start || end_vpn == self.vma[idx].vpn_end){
                    let res = self.vma[idx].unmap_part(start_vpn, end_vpn, &mut self.page_table);
                    if(res.is_err()){
                        return res;
                    }
                    if(start_vpn == self.vma[idx].vpn_start && end_vpn == self.vma[idx].vpn_end){
                        break;
                    }
                    self.vma[idx].vpn_start = start_vpn;
                    self.vma[idx].vpn_end = end_vpn;
                }
                else{//重新映射剩余部分
                    /*
                    self.vma[idx].unmap(&mut self.page_table);
                    let flag = self.vma[idx].flag.bits;
                    self.vma.remove(idx);
                    if(start_vpn != self.vma[idx].vpn_start){
                        self.map_vma(start_vpn, start_vpn - self.vma[idx].vpn_start, flag.into());
                    }
                    if(end_vpn != self.vma[idx].vpn_end){
                        self.map_vma(end_vpn, self.vma[idx].vpn_end - (end_vpn), flag.into());
                    }
                    */
                    
                    let res = self.vma[idx].unmap_part(start_vpn, end_vpn, &mut self.page_table);
                }
                    break;
            }
            else if start_vpn >= self.vma[idx].vpn_start || end_vpn <= self.vma[idx].vpn_end{
                return Err("section overlap!!");
            }
        }
        Ok(())
    }
    /*根据起始地址寻找对应段并删除*/
    pub fn drop_section(& self, start_vir: usize, len: usize) -> isize{
        /*
        if let Some((idx, section)) = self.sections.iter().enumerate().find((|_,section|), section.get_range() == (start_vir, start_vir + len)){//只有完全符合要求的才能drop
            section.unmap(&mut self.page_table);
            self.sections.remove(idx);
            true
        }
        info!("未找到符合条件的段");
        false
        */
        -1
    }
    
}

/*内存管理结构的全局声明*/
lazy_static! {
    pub static ref KERNEL_SPACE: Arc<Mutex<MemoryManager>> = Arc::new(unsafe {
        Mutex::new(MemoryManager::new_kernel()
    )});
}


extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();
    fn ekernel();
    fn strampoline();
}

