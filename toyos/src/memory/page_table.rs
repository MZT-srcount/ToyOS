use super::{frame_alloc, Phy_PageFrame, phy_frame::FRAME_MANAGER};
use alloc::vec::Vec;
use alloc::vec;
use super::address::*;
use alloc::string::String;
use crate::config::{PAGE_SIZE, PAGE_SIZE_BITS,SV39_VA};
use crate::task::current_task;

/*页表项标志位flag*/
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

/*获取PageBit值*/
impl PageBit{
    pub fn val(self) -> usize{
        self as usize
    }
}


#[derive(Copy, Clone)]
#[repr(C)]
pub struct PTE{
    flag: usize
}

impl PTE{
    fn new(phypagenum: usize, flag: usize) -> Self{
        PTE{
            flag: ((phypagenum) << 10 | (flag & 0x3ff)) as usize,
        }
    }
    /*返回空标志位*/
    fn empty() -> Self{
        PTE{
            flag: 0,
        }
    }
    pub fn get_flag(&self) -> usize{
        (self.flag & 0xff) as usize
    }
    /*返回页表项的物理页号*/
    pub fn ppn(&self) -> usize{
        (self.flag >> 10 & ((1usize << 44) - 1)) as usize
    }
    
    /*各种标志位判断*/
    
    pub fn is_valid(&self) -> bool{
        self.flag & PageBit::Valid.val() != 0
    }
    pub fn is_readable(&self) -> bool{
        self.flag & PageBit::Read.val() != 0
    }
    pub fn is_writable(&self) -> bool{
        self.flag & PageBit::Write.val() != 0
    }
    pub fn is_executable(&self) -> bool{
        self.flag & PageBit::Execute.val() != 0
    }
    pub fn is_empty(&self) -> bool{
        self.flag != 0
    }
    
    /*标志位设置*/
    pub fn set_flag(&mut self, flag: usize){
        //
        self.flag &= !(0x3ff);
        self.flag |= (flag & 0x3ff);
    }
    pub fn set_valid(&mut self) {
        self.flag |= PageBit::Valid.val();
    }
}

/*页表结构，保存根页表和已分配物理页*/
pub struct PageTable{
    root_ppn: usize,
    frames: Vec<Phy_PageFrame>,//不能用usize,否则函数中创建的Phy_PageNum将变成局部变量，离开函数后被回收
}

impl PageTable{
    pub fn new() -> Self{
        let frame = frame_alloc().unwrap();
        PageTable{
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
    pub fn from_token(satp: usize) -> Self{
        Self{
            root_ppn: (satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }
    /*查找或创建一个页表项*/
    fn get_pte(&mut self, virtual_addr: usize) -> Option<&mut PTE> {
        let vpn = [
            (virtual_addr >> 18) & 0x1ff,
            (virtual_addr >> 9) & 0x1ff,
            (virtual_addr >> 0) & 0x1ff
            ];
        let mut ppn = unsafe{
            core::slice::from_raw_parts_mut((self.root_ppn << 12) as *mut PTE, 512)
        };
        let mut ret: Option<&mut PTE> = None;
        for i in 0..3{
            let pte = &mut ppn[vpn[i]];
            if i == 2{
                return Some(pte);
            }
            else if !pte.is_valid() {

                //println!("pte map begin..");
                let frame = frame_alloc().unwrap();

                //println!("pte map end..");
                *pte = PTE::new(frame.ppn, PageBit::Valid.val());
                self.frames.push(frame);
            }
            ppn = unsafe{
               core::slice::from_raw_parts_mut((pte.ppn() << PAGE_SIZE_BITS) as *mut PTE, 512)
            };
        }
        ret
    }
    /*查找页表项*/
    pub fn find_pte(&self, virt_pn: usize) -> Option<&PTE>{
    	//三级页表，三段虚拟页号
        let vpn = [
            (virt_pn >> 18) & 0x1ff,
            (virt_pn >> 9) & 0x1ff,
            (virt_pn >> 0) & 0x1ff
            ];
        let mut pagetable = unsafe{
            core::slice::from_raw_parts_mut((self.root_ppn << PAGE_SIZE_BITS) as *mut PTE, 512)
        };
        let mut ret: Option<&PTE> = None;
        for i in 0..3{
            let pte = &pagetable[vpn[i]];
            if i == 2{
                return Some(pte);
            }
            else if !pte.is_valid(){
                break;
            } 
            pagetable = unsafe{
                core::slice::from_raw_parts_mut((pte.ppn() << PAGE_SIZE_BITS) as *mut PTE, 512)
            }
        }
        ret
    }
    pub fn find_pte_mut(&self, virt_pn: usize) -> Option<&mut PTE>{
        let vpn = [
            (virt_pn >> 18) & 0x1ff,
            (virt_pn >> 9) & 0x1ff,
            (virt_pn >> 0) & 0x1ff
            ];
        let mut pagetable = unsafe{
            core::slice::from_raw_parts_mut((self.root_ppn << PAGE_SIZE_BITS) as *mut PTE, 512)
        };
        let mut ret: Option<&mut PTE> = None;
        for i in 0..3{
            let pte = &mut pagetable[vpn[i]];
            if i == 2{
                return Some(pte);
            }
            else if !pte.is_valid(){
                break;
            } 
            pagetable = unsafe{
                core::slice::from_raw_parts_mut((pte.ppn() << PAGE_SIZE_BITS) as *mut PTE, 512)
            };
        }
        ret
    }
    //虚拟地址到物理地址的映射
    #[allow(unused)]
    pub fn map(&mut self, vpn: usize, ppn: usize, flags: usize) {
        let pte = self.get_pte(vpn).unwrap();
        //println!("vpn is: {:?}", vpn);
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping, {}", vpn, pte.is_valid());
        *pte = PTE::new(ppn, flags | PageBit::Valid.val());
    }
    //清空某个页表项
    #[allow(unused)]
    pub fn unmap(&mut self, vpn: usize) {
        let pte = self.get_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PTE::empty();
    }
    //根据虚拟页号返回对应页表项的复制
    pub fn virt_to_phy(&self, vpn: usize) -> Option<PTE>{
        self.find_pte(vpn).map(|pte|{pte.clone()})
    }
    pub fn virtAddr_to_phyAddr(&self, va: usize) -> Option<usize> {
        //虚拟地址-》页表项-》物理页号-》物理页号所在地址
        //虚拟地址-》页内偏移(末12位)
        //两者拼接得到物理地址
        self.find_pte(va/PAGE_SIZE).map(|pte| {
            //println!("translate_va:va = {:?}", va);
            let aligned_pa: usize = pte.ppn()<< PAGE_SIZE_BITS;
            //println!("translate_va:pa_align = {:?}", aligned_pa);
            let offset = va &(PAGE_SIZE - 1);
            (aligned_pa + offset).into()
        })
    }
    //satp值获取
    pub fn token(&self) -> usize{
        8usize << 60 | self.root_ppn
    }
    pub fn set_flag(&mut self, vpn: usize, flag: usize){
        let pte = self.get_pte(vpn).unwrap();
        pte.set_flag(flag);
    }

}

/*userbuffer*/
pub struct UserBuffer{
    pub buffer: Vec<&'static mut [u8]>,
}

impl UserBuffer{
    pub fn new(buf: Vec<&'static mut [u8]>)->Self{
        Self{
            buffer: buf,
        }
    }
    pub fn len(&self) -> usize{
        let mut total : usize = 0;
        for idx in 0..self.buffer.len() {
            total += self.buffer[idx].len();
        }
        total
    }
    //将一个Buffer的数据写入UserBuffer，返回写入长度
    pub fn write(&mut self, buf: &[u8])->usize{
        let len = self.len().min(buf.len());
        let mut current = 0;
        for sub_buffer in self.buffer.iter_mut() {
            let sblen = (*sub_buffer).len();
            for j in 0..sblen {
                (*sub_buffer)[j] = buf[current];
                current += 1;
                if current == len {
                    return len;
                }
            }
        }
        return len;
    }
    pub fn clear( &mut self ){
        for sub_buffer in self.buffer.iter_mut() {
            let sblen = (*sub_buffer).len();
            for j in 0..sblen {
                (*sub_buffer)[j] = 0;
            }
        }
    }
    pub fn write_at(&mut self, offset:usize, buf: &[u8])->isize{
        let len = buf.len();
        if offset + len > self.len() {
            return -1
        }
        let mut head = 0;
        let mut current = 0;
        for sub_buffer in self.buffer.iter_mut() {
            let sblen = (*sub_buffer).len();
            if head + sblen < offset {
                continue;
            } else if head < offset {
                for j in (offset - head)..sblen {
                    (*sub_buffer)[j] = buf[current];
                    current += 1;
                    if current == len {
                        return len as isize;
                    }
                }
            } else {
                for j in 0..sblen {
                    (*sub_buffer)[j] = buf[current];
                    current += 1;
                    if current == len {
                        return len as isize;
                    }
                }
            }
            head += sblen;
        }
        0
    }
    pub fn read(&self, buf:&mut [u8])->usize{
        let len = self.len().min(buf.len());
        let mut current = 0;
        for sub_buffer in self.buffer.iter() {
            let sblen = (*sub_buffer).len();
            for j in 0..sblen {
                buf[current] = (*sub_buffer)[j];
                current += 1;
                if current == len {
                    return len;
                }
            }
        }
        return len;
    }
    pub fn read_as_vec(&self, vec: &mut Vec<u8>, vlen:usize)->usize{
        let len = self.len();
        let mut current = 0;
        for sub_buffer in self.buffer.iter() {
            let sblen = (*sub_buffer).len();
            for j in 0..sblen {
                vec.push( (*sub_buffer)[j] );
                current += 1;
                if current == len {
                    return len;
                }
            }
        }
        return len;
    }
}
impl IntoIterator for UserBuffer {
    type Item = *mut u8;
    type IntoIter = UserBufferIterator;
    fn into_iter(self) -> Self::IntoIter {
        UserBufferIterator {
            buffers: self.buffer,
            current_buffer: 0,
            current_idx: 0,
        }
    }
}

pub struct UserBufferIterator {
    buffers: Vec<&'static mut [u8]>,
    current_buffer: usize,
    current_idx: usize,
}

impl Iterator for UserBufferIterator {
    type Item = *mut u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_buffer >= self.buffers.len() {
            None
        } else {
            let r = &mut self.buffers[self.current_buffer][self.current_idx] as *mut _;
            if self.current_idx + 1 == self.buffers[self.current_buffer].len() {
                self.current_idx = 0;
                self.current_buffer += 1;
            } else {
                self.current_idx += 1;
            }
            Some(r)
        }
    }
}

/*获取缓冲区数据，并以字节数组形式输出*/
pub fn get_data_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let mut vpn = start >> PAGE_SIZE_BITS;
        let ppn = page_table
            .virt_to_phy(vpn)
            .unwrap()
            .ppn();
        vpn += 1;
        let mut end_va = vpn << PAGE_SIZE_BITS;
        if end < end_va {
             end_va = end;
        }
        let data = unsafe{core::slice::from_raw_parts_mut((ppn << PAGE_SIZE_BITS) as *mut u8, 4096) as &mut [u8]};
        if end_va & (PAGE_SIZE - 1) == 0 {
            v.push(&mut data[(start & (PAGE_SIZE - 1)) as usize..]);
        } else {
            v.push(&mut data[(start & (PAGE_SIZE - 1)) as usize..(end_va & (PAGE_SIZE - 1)) as usize]);
        }
        start = end_va;
    }
    v
}

//获取虚拟地址的物理地址(可修改)，用于写入内容
//这个位置的类型不知道，因此采用泛型
pub fn translated_refmut<T>(token: usize, ptr: *mut T) -> &'static mut T {
    let page_table = PageTable::from_token(token);
    let va_orig = ptr as usize;
    let mut va=va_orig & ((1 << SV39_VA) - 1);
    if(page_table.virtAddr_to_phyAddr(va).is_none()){
        let task = current_task().unwrap();
        task.check_lazyalloc(va);
    }
    let ptrT = unsafe { page_table
        .virtAddr_to_phyAddr(va)
        .unwrap() as *mut T };
       unsafe{&mut *ptrT}
}

/* 获取用户数组的一份拷贝 */
pub fn translated_array_copy<T>(token: usize, ptr: *mut T, len: usize) -> Vec< T>
    where T:Copy {
    let page_table = PageTable::from_token(token);
    let mut ref_array:Vec<T> = Vec::new();
    let mut va = ptr as usize;
    let step = core::mem::size_of::<T>();
    //println!("step = {}, len = {}", step, len);
    for _i in 0..len {
        let u_buf = UserBuffer::new( translated_byte_buffer(token, va as *const u8, step) );
        let mut bytes_vec:Vec<u8> = Vec::new();
        u_buf.read_as_vec(&mut bytes_vec, step);
        //println!("loop, va = 0x{:X}, vec = {:?}", va, bytes_vec);
        unsafe{
            ref_array.push(  *(bytes_vec.as_slice() as *const [u8] as *const u8 as usize as *const T) );
        }
        va += step;
    }
    ref_array
}

pub fn translated_str(token: usize, ptr: *const u8) -> String {
    let page_table = PageTable::from_token(token);
    let mut string = String::new();
    let va_orig = ptr as usize;
    let mut va=va_orig & ((1 << SV39_VA) - 1);
    loop {
        // let ch: u8 = *(page_table.translate_va(VirtAddr::from(va)).unwrap().get_mut());
        //println!("phyaddr: {:#x}", (page_table.virtAddr_to_phyAddr(va).unwrap()));
        let ch:u8=* unsafe { (page_table
            .virtAddr_to_phyAddr(va)
            .unwrap() as *mut u8).as_mut().unwrap() };
        if ch == 0 {
           break;
       } else {
           string.push(ch as char);
           va += 1;
       }
   }
   string
}
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        //println!("tbb vpn = 0x{:X}", vpn.0);
        // let ppn: PhysPageNum;
        if page_table.virt_to_phy(vpn.0).is_none() {
            panic!("can not find the virtual address, lazy_alloc not support yet");
            // println!{"preparing into checking lazy..."}
            //println!("check_lazy 3");
            //current_task().unwrap().check_lazy(start_va, true);
            unsafe {
                llvm_asm!("sfence.vma" :::: "volatile");
                llvm_asm!("fence.i" :::: "volatile");
            }
            //println!{"preparing into checking lazy..."}
        }
        let ppn = page_table
            .virt_to_phy(vpn.0)
            .unwrap()
            .ppn();
        //println!("vpn = {} ppn = {}", vpn.0, ppn.0);
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut PhysPageNum(ppn).get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut PhysPageNum(ppn).get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}