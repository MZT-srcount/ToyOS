pub const USER_STACK_SIZE: usize = 4096 * 5;
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
pub const MEMORY_END: usize = 0x80800000;
pub const PAGE_SIZE_BITS: usize = 0xc;
pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;
pub const PAGE_SIZE: usize = 4096;
pub const SV39_VA : usize = 39;
pub const TRAMPOLINE: usize = (usize::MAX - PAGE_SIZE + 1) ;
pub const TRAP_CONTEXT: usize = (TRAMPOLINE_PHY - PAGE_SIZE) ;
pub const TRAMPOLINE_PHY: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT_PHY: usize = TRAMPOLINE_PHY - PAGE_SIZE;
pub const USER_HEAP_SIZE: usize = PAGE_SIZE * 64;
pub const EXITOFFSET: usize = 8;
pub const UNAME_LEN: usize = 65;

pub const SYSNAME: &[u8] = b"toyos \0";
pub const NODENAME: &[u8] = b"Network currently unsupported \0";
pub const RELEASE: &[u8] = b"0.0.1 \0";
pub const MACHINE: &[u8] = b"unknown \0";
pub const DOMAINNAME: &[u8] = b"unknown \0";

pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = (TRAMPOLINE_PHY - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE));
    let bottom = (top - KERNEL_STACK_SIZE);
    (bottom, top)
}



#[cfg(feature = "board_k210")]
pub const MMIO: &[(usize, usize)] = &[
    (0x0C00_0000, 0x3000),      /* PLIC      */
    (0x0C20_0000, 0x1000),      /* PLIC      */
    (0x3800_0000, 0x1000),      /* UARTHS    */
    (0x3800_1000, 0x1000),      /* GPIOHS    */
    (0x5020_0000, 0x1000),      /* GPIO      */
    (0x5024_0000, 0x1000),      /* SPI_SLAVE */
    (0x502B_0000, 0x1000),      /* FPIOA     */
    (0x502D_0000, 0x1000),      /* TIMER0    */
    (0x502E_0000, 0x1000),      /* TIMER1    */
    (0x502F_0000, 0x1000),      /* TIMER2    */
    (0x5044_0000, 0x1000),      /* SYSCTL    */
    (0x5200_0000, 0x1000),      /* SPI0      */
    (0x5300_0000, 0x1000),      /* SPI1      */
    (0x5400_0000, 0x1000),      /* SPI2      */
];

/// Device memory mapped IO for qemu
#[cfg(feature = "board_qemu")]
pub const MMIO: &[(usize, usize)] = &[
    (0x10000000, 0x10000),
];


//usize转为virtual address：防止usize值过大
pub fn usize_va(virt: usize) -> usize {
	virt & ((1 << SV39_VA) - 1)
}

#[cfg(feature = "board_k210")]
pub const CLOCK_FREQ: usize = 403000000 / 62;

#[cfg(feature = "board_qemu")]
pub const CLOCK_FREQ: usize = 12500000;



