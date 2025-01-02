.altmacro

#类似define
.macro SAVE_GP n
	sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
	ld x\n, \n*8(sp)
.endm
	
#位于.text段
	.section .text.trampoline
	.globl __alltraps
	.globl __restore
	.align 2

__alltraps:
#从用户栈到用户空间中trapcontext位置，sscratch保存的是trapcontext地址,csrrw可理解为交换寄存器值,sp指向trapcontext
	csrrw sp, sscratch, sp

	sd x1, 1*8(sp)
	sd x3, 3*8(sp)
#循环函数，保存寄存器

	
	.set n, 5
	.rept 27
		SAVE_GP %n
		.set n, n+1
	.endr

	csrr t0, sstatus
	csrr t1, sepc
	sd t0, 32*8(sp)
	sd t1, 33*8(sp)
#保存用户栈栈顶地址
	csrr t2, sscratch
	sd t2, 2*8(sp)
#传参内核satp、trap_handler地址等
	
	ld t0, 34*8(sp)
	ld t1, 36*8(sp)
	ld sp, 35*8(sp)
#进入内核虚拟空间，跳转进入trap_handler
	csrw satp, t0
	sfence.vma
	jr t1

__restore:
	csrw satp, a1
	sfence.vma
	csrw sscratch, a0
	mv sp, a0
	ld t0, 32*8(sp)
	ld t1, 33*8(sp)
	
	csrw sstatus, t0
	csrw sepc, t1

	ld x1, 1*8(sp)
	ld x3, 3*8(sp)
	.set n, 5
	.rept 27
		LOAD_GP %n
		.set n, n+1
	.endr

    	ld sp, 2*8(sp)
    	
	sret
