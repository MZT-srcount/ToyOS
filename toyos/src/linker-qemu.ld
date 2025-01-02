/*设置目标平台为riscv*/
OUTPUT_ARCH(riscv)
/*设置入口位置*/
ENTRY(_start)
/*声明一个常量，作为初始化代码被放置的地址*/
BASE_ADDRESS = 0x80200000;

SECTIONS
{
	/*设置起始地址*/
	. = BASE_ADDRESS;

	/*顺序放置skernel和stext*/
	skernel = .;
	stext = .;

	/*
	 * .text段放置内容
	 * 格式：<ObjectFile>(SectionName)
	 */
	.text : {
		*(.text.entry)
		
		. = ALIGN(4k);
		
		strampoline = .;
		*(.text.trampoline);
		
		. = ALIGN(4k);
		
		*(.text .text.*)
	}

	/*
	 * 按4k大小对齐
	 */
	. = ALIGN(4k);
	
	etext = .;
	srodata = .;
	.rodata : {
		*(.rodata .rodata.*)
		*(.srodata .srodata.*)
	}

	. = ALIGN(4k);

	erodata = .;
	sdata = .;
	.data : {
		*(.data .data.*)
		*(.sdata .sdata.*)
	}

	. = ALIGN(4k);

	edata = .;
	sbss_with_stack = .;
	.bss : {
		*(.bss.stack)
		sbss = .;
		*(.bss .bss.*)
		*(.sbss .sbss.*)
	}

	. = ALIGN(4k);
	ebss = .;
	ekernel = .;

	/DISCARD/ : {
		*(.eh_frame)
	}
}
