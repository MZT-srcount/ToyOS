all: 
	# rustup target add riscv64gc-unknown-none-elf
	cd user && make elf
	cd toyos && make all BOARD=k210
