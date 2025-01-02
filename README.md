# toyos

#### 介绍
基于RISCV体系架构，可运行于k210开发板的操作系统内核``toyos``，并提供一些可供使用的系统调用。截至目前，toyos已实现操作系统的部分关键特性，支持多核进程管理、内存管理以及文件系统等基础功能，并相应实现了一些系统调用，在基于k210处理器的Sipeed M1开发板和qemu上能够成功运行。  

#### 软件架构

1、目录树：

```

|--bootloader
|    |--rustsbi-k210.bin用于k210开发板
|    |--rustsbi-qemu.bin用于qemu模拟器
|--toyos
|    |--src
|    |    |--trap trap管理实现
|    |    |--task 进程管理实现
|    |    |--syscall 系统调用部分实现
|    |    |--sync 可用于放互斥锁等实现文件，目前只有用聊
|    |    |--memory 内存管理实现
|    |    |--timer.rs 计数器包装、时钟中断等实现
|    |    |--sbi.rs 封装与Rustsbi的交互
|    |    |--loader加载用户程序数据
|    |    |--linker.ld链接文件
|    |    |--entry.asm 内核入口
|    |    |--console.rs 目前为控制台println等函数实现   
|    |--target
|    |--.cargo 
|    |--build.rs 
|    |--Makefile
|--user
|    |--src
|    |    |--bin 用户程序
|    |    |--console.rs println等函数实现
|    |    |--linker.ld 用户程序链接器
|    |    |--syscall.rs系统调用封装
|    |    |--lang_items.rs
|    |    |--lib.rs
|    |--target 
|    |--Makefile

```
#### 安装教程

1、Rust开发环境配置：

    1）首先安装 Rust 版本管理器 rustup 和 Rust 包管理器 cargo：
        curl https://sh.rustup.rs -sSf | sh
        若网速较慢，可修改 rustup 的镜像地址来加速：
            export RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static
            export RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup
            curl https://sh.rustup.rs -sSf | sh
    
    2）安装完成后，重新开启终端使环境变量生效
    
    3）输入 rustc --version 确认是否正确安装Rust工具链，注意：只能使用rustc的nightly版本
        若已安装rustc且非nightly版本，可使用如下命令安装nightly版本
            rustup install nightly
            rustup default nightly
    
        ** ！！！注意：由于rustc的nightly的某些版本无法使用llvm-asm宏，建议使用1.59.0版本！** 
        ** ！！！注意：由于rustc的nightly的某些版本无法使用llvm-asm宏，建议使用1.59.0版本！** 
        ** ！！！注意：由于rustc的nightly的某些版本无法使用llvm-asm宏，建议使用1.59.0版本！** 

    4）若网速较慢，最好将软件包管理器 cargo 所用的软件包镜像地址 crates.io 也换成中国科学技术大学的镜像服务器来加速三方库的下载
        打开或新建~/.cargo/config 文件，修改内容为：
            [source.crates-io]
            registry = "https://github.com/rust-lang/crates.io-index"
            replace-with = 'ustc'
            [source.ustc]
            registry = "git://mirrors.ustc.edu.cn/crates.io-index"
    
    5）安装rust相关软件包：
        rustup target add riscv64gc-unknown-none-elf
        cargo install cargo-binutils --vers =0.3.3
        rustup component add llvm-tools-preview
        rustup component add rust-src

2、安装qemu模拟器：

    1）安装所需依赖包：
        sudo apt install autoconf automake autotools-dev curl libmpc-dev libmpfr-dev libgmp-dev \
                      gawk build-essential bison flex texinfo gperf libtool patchutils bc \
                      zlib1g-dev libexpat-dev pkg-config  libglib2.0-dev libpixman-1-dev git tmux python3 python3-pip

    2）编译安装并配置RISC-V支持：
        cd qemu-5.0.0
        ./configure --target-list=riscv64-softmmu,riscv64-linux-user
        make -j$(nproc)

    注意，上面的依赖包可能并不完全，比如在 Ubuntu 18.04 上：
        出现 ERROR: pkg-config binary 'pkg-config' not found 时，可以安装 pkg-config 包；
        出现 ERROR: glib-2.48 gthread-2.0 is required to compile QEMU 时，可以安装 libglib2.0-dev 包；
        出现 ERROR: pixman >= 0.21.8 not present 时，可以安装 libpixman-1-dev 包

3、编辑~/.bashrc文件，在文件末尾加入几行：

    export PATH=$PATH:/home/shinbokuow/Downloads/built/qemu-5.0.0
    export PATH=$PATH:/home/shinbokuow/Downloads/built/qemu-5.0.0/riscv64-softmmu
    export PATH=$PATH:/home/shinbokuow/Downloads/built/qemu-5.0.0/riscv64-linux-user

    然后调用命令：source ~/.bashrc 更新系统路径

4、为了在k210真机运行内核，需要安装基于 Python 的串口通信库和简易的串口终端：

    pip3 install pyserial
    sudo apt install python3-serial



#### 使用说明

1. 运行toyos:

    在终端进入toyos目录，输入指令 make run 运行代码

2. 目前共包含四个部分:

    1）bootloader：引导程序，采用RustBin

    2）toyos：内核部分
        src内的为内核代码，详情见代码内注释

    3）user：用户测试程序，未实现文件系统前暂时用该文件来测试程序
        src内的为相关代码，包含链接文件和相关的测试程序，src/bin内为测试程序

    4）tools：包含文件系统烧录等工具

3. 对外开放的接口在内核各个文件夹中的mod.rs文件中声明或注释

# 系统设计

## 系统整体架构设计  

![xx](pic/framework.png)  

toyos遵循模块化设计的思想，将操作系统的构建根据RISC-V架构的特权级分为了三个层级：机器层, 操作系统层以及用户层。对应的程序执行权限也从高到低分布。
- 用户层运行于用户模式，位于虚拟地址空间，通过系统调用接口与内核进行交互；当用户需要进行系统调用时，需要通过ecall陷入内核。为了方便用户与内核的交互，在用户层往往还会对系统调用进行进一步的封装和部分预处理，形成用户标准库Lib。对于不同的语言，可以拥有不同的标准库。
- 操作系统层运行于监管者模式，同样位于虚拟地址空间，通过采用恒等映射的方式对物理内存进行管理。能够管理部分特殊寄存器。在toyos中采用了双核设计，对两个核心进行了封装。两个核心在同一时刻分别运行不同的进程，但与同一个内核进行交互，运行同样的内存管理、进程管理以及文件系统管理。
    - 内存管理主要负责管理用户的虚拟地址空间与物理内存空间的映射关系，包括物理页的分配与回收、虚实页面转换、堆栈的分配等。
    - 进程模块负责进程的资源管理，对进程进行调度等。
    - 文件系统采用FAT32结构，主要对磁盘进行读取和修改，并将读取的数据抽象化为文件，方便内核管理。 
    
    
  
  内核作为中间层，向上需要给用户层提供用户服务抽象，通过中断服务机制给上层提供系统调用接口；向下满足机器层的标准，通过SBI封装的接口与硬件交互。  
- SBI运行于机器模式，封装硬件功能，为操作系统层提供一个抽象化的接口，使得操作系统可以无需过多关心硬件的细节部分，具有更好的兼容性和逻辑性。

## 系统框架
- 进程模块
  - 进程号的分配与回收，进程号决定内核栈的位置
  - 处理器核心的上下文管理
  - 进程上下文管理
  - 进程资源管理
  - 进程调度管理
- 内存模块
  - 内核的动态内存分配
  - 内存管理器，负责管理进程的虚拟地址空间
  - 页表机制实现，包括虚实地址转换、页表项读取、映射以及权限控制等功能。 
  - 物理页帧管理，负责页帧的分配与回收
  - 用于多线程时的内核栈分配，目前暂未使用
- 文件系统模块
  - 磁盘块缓存实现
  - IO设备管理
  - FAT32磁盘数据结构组织、封装和操作
  - 文件系统管理
  - 内核抽象文件系统的实现与管理
  - 并发访问与控制

# 子模块设计
## 进程管理

​		在进程管理部分，需要考虑进程的创建与初始化，维护进程的父子关系，进程的切换，进程资源的分配与回收等问题。一个进程急需要考虑在用户空间上的执行，也需要考虑在内核空间的执行，这也增加了其实现的复杂性。在我们的设计过程中，所有进程之间可以形成一个树结构，树的根节点为一个初始进程,其余进程均在其基础上进行创建。对于进程的管理，其实质上是如何有效维护该进程树。为了实现一个良好的抽象和对进程树的管理，我们将其分成了三个模块，分别是进程控制模块，进程调度模块，进程切换模块。


### 调用关系

​		一个进程的执行过程可以分为两个部分，一部分是在用户空间中执行，另一部分时在内核空间中执行，后者对于前者是透明的，分时多任务系统中，可以让每一个进行感觉自己在连续执行，占用所有的系统资源和时间。在我们的实现中，进程执行的调用关系如下图：

![](pic/P_CallRelation.png)

​		每一个执行的应用程序，在内核空间会被分成不同的任务块，当一个应用的所有任务块被执行完毕，该应用程序方算执行完毕，同时这些任务块会在PCB控制块中存储自身的信息。不同的应用程序中的任务块由统一的进程管理器进行管理，由其确定在不同时刻应该由哪一个任务块进行执行，当一个任务块执行完毕或者由于异常退出时，也会分配给下一个任务块进行执行。

### 模块说明

- **进程控制模块**  

主要功能是维护该进程的信息以便对进程进行控制，是用于进程管理的核心模块。这些信息包括进程标识，进程状态，地址空间信息，以及父子进程等。这些信息代表着一个进程中的核心属性，当进程的状态发生改变，相应的便是维护对应进程的控制属性内容，因此要保证不同的进程之间在满足区分的条件之下，又能通过以上的属性进行功能的实现。

- **进程调度模块**  

​		就是以某一种调度策略对当前在进程队列中的全部进程进行管理。用于管理的工具可称之为调度器，其主要的提供的功能便是从队列中取出一个任务以及添加一个新任务。其中可以维护内存中的一组进程队列。当有新的进程被创建，则应该往队列中添加新进程的属性信息，如果进程结束，或者异常退出，则应该会导致该进程中的内容，从队列中删除。但当其他涉及任务调度时，则任务应该始终在队列中。

- **进程切换模块**  

​	用于保存当前执行进程的控制块信息，以及控制流的任务上下文。其主要功能是可以获取当前执行进程的信息以及根据调度结果切换不同的进程。其核心作用可以用于获取当前正在被处理器执行的进程，而当任务被切换，也可通过当前进程控制流的上下文进行进程的切换。因此会与进程调度模块密切相关，当有进程被调度时，代表着应该进行进程的切换，因此需要进程控制器的分配和不同上下文的切换。

## 内存管理
内存管理主要包括内核空间管理、用户空间管理以及页表地址转换三部分，内存空间的管理在内核虚拟地址空间完成。内核空间通过以页为单位恒等映射物理地址空间，对物理地址空间进行管理，用户空间和内核空间在物理内存的分配以及页表地址的转换均在内核空间完成。同时在内核空间还负责管理用户和内核在物理内存的页帧分配与回收、虚拟地址到物理地址的页表映射、用户栈的分配、文件和设备映射以及程序运行中的动态内存分配等。
当前内核地址空间和用户地址空间的布局情况以及它们与物理地址的映射关系如下图所示：  

![](pic/space%20layout1.png)  

内核地址空间采用恒等映射和随机映射相结合的方式，在`0x8000000~0x80800000`地址段采用恒等映射，使内核常驻内存，同时方便对物理内存的管理，`MMIO`部分同样采用恒等映射。在`0x80800000`以上地址采用随机映射，主要用于跳板页面以及每个进程的内核栈的分配。内核的堆空间作为未初始化的全局变量划分在.bss段中。用户地址空间采用随机映射的方式。
  
### 模块概述：
- **内存管理器模块**  
   
地址空间是一系列有关联的不一定连续的逻辑段，这种关联一般是指这些逻辑段组成的虚拟内存空间与一个运行的程序绑定，即这个运行的程序对代码和数据的直接访问范围限制在它关联的虚拟地址空间之内，每个进程都有自己的内存空间。内存管理器主要便负责管理每个进程的整个虚拟地址空间。包括对地址空间的段的管理、页表管理等。  
  
- **多级页表管理模块**  
  
SV39 多级页表以节点为单位进行管理。每个节点恰好存储在一个物理页帧中，它的位置可以用一个物理页号来表示。每个应用都对应一个不同的多级页表。页表管理器主要负责管理进程多级页表的映射、页表项的管理以及页面的查找等。  
  
- **逻辑段管理模块**  
  
我们用逻辑段为单位描述一段连续地址的虚拟内存。一个内存管理器中包含数个段空间管理器。段空间管理器主要负责段内页面的映射、段内数据管理、段的映射方式以及段的虚拟地址范围确认和修改等。  
  
- **物理页帧管理模块**  
  
当bootloader把内核加载到物理内存中后，物理内存上已经有一部分用于放置内核的代码和数据。我们需要将剩下的空闲内存以单个物理页帧为单位管理起来，当需要存放应用数据或扩展应用的多级页表时分配空闲的物理页帧，并在应用出错或退出的时候回收应用占有的所有物理页帧。  
    
- **内存动态分配模块**  
  
我们在内核的.bss段划分了一段空间用于内核的动态内存分配，通过堆大小的设置、堆分配出错时的处理等对堆进行初始化后，便可利用alloc库对堆空间进行管理  
   
   
K210平台中物理内存最大为8MB，这意味着在高负荷的情况下很容易出现内存不足的情况。针对这点，我们设想了几种方式预防物理内存不足的问题：

- 采用`Copy On Write`写时复制机制；一般情况下，通过`fork`形成的子进程在后续的程序运行中大部分页面并不需要修改，采用页帧共享，延时机制能很好的节省本就紧缺的内存资源，同时也减少了内存访问所带来的CPU周期的消耗以及对`CPU cache`的破坏。
  
- 采用页面换入换出机制；`Copy On Write`对于某些情况下如一个程序多个进程的模式等有很好的效果，但如果同时运行的程序过多，程序过大，写时复制并不能很好的发挥效果。我们可以在外存中设置一个缓冲文件，专门用于存储程序的换出页面并保存每个换出页面的虚拟地址等信息，需要时再调入内存。
  
- 采用`Lazy Alloc`机制；每一个进程所需的堆栈空间并不相同，提前设定空间大小容易造成空间浪费或空间不足的情况，而完全让用户程序来决定分配多少堆栈空间又有内存虚占甚至是恶意攻击的风险。通过用户预设大小，内核实时分配的机制能够很好的解决这些问题。  

针对后期可能的内存占用过大的问题，我们设想了几种解决方案：  

- 程序部分加载机制；在内存占用过大时，物理页帧过少，可能造成无法加载新程序到内存中的问题，为了让程序能够运行，可以只预先加载部分页面到内存中，通过缺页中断机制调用外存页面完成剩余页面的补全。这种机制可以在内存中同时运行更多程序，但容易造成卡顿等问题。  
  
- 程序换出机制；在内存压力过大时，为了及时加载新的程序，可以考虑标记某些内存中的进程并将其换出内存，放入外部缓存中，在内存压力减小时再换回内存中。

由于时间有限，目前我们只实现了`Copy on Write`和堆空间的`Lazy Alloc`机制。

## 文件系统
基于 rCore-Tutorial-Book-v3 教程的松耦合模块化设计思路，我们实现了一个FAT32文件系统，这样的开发过程更易于理解，且具有更好的可拓展性。一方面我们采用抽象接口BlockDevice与底层设备驱动进行连接，避免了与设备驱动的绑定，另一方面通过Rust提供的alloccrate对操作系统内核的内存管理进行了隔离，避免了直接调用内存管理的内核函数。为了避免访问外设中断的相关内核函数，在底层驱动上又采用了轮询的方式来访问虚拟磁盘设备。因此，我们的磁盘文件系统与内核虚拟文件系统分隔开的。除此之外，在我们的文件系统中，任何具备读写功能的系统对象都被视为抽象的文件，对并发访问我们也做了相应的设计从而满足双核系统的需求。
### 文件系统整体结构 
文件系统采用了层次化和模块化的结构设计，磁盘文件系统从下到上主要分为磁盘块设备接口层、块缓存层、磁盘数据结构层、文件系统管理层、虚拟文件系统层。
|层级	| 描述 |
| ---- | ---- |
| 磁盘块设备接口层 |	声明了一个块设备的抽象接口 BlockDevice，实现两个抽象方法read_block和write_block，这两个方法由文件系统的实际使用者提供。|
| 块缓存层 | 提供一个get_block_cache接口来访问块，会根据需求自动读取、写回或替换块。|
| 磁盘数据结构层 |	实现了引导扇区，扩展引导扇区，文件系统信息扇区和长短目录项等核心数据结构，以及抽象的FAT。 |
| 文件系统管理层 |	对磁盘布局的一种抽象。可以打开已有的FAT32文件系统，控制簇的分配与回收。 |
| 虚拟文件系统层 |	为内核提供了文件操作的接口，比如文件的创建、读写、清空来向上支持相关的系统调用。 |

### 文件系统各层级介绍
#### 磁盘块设备接口层
为了在虚拟机和开发板上运行，文件系统必须支持不同的块设备。块设备接口层即用于与不同的块设备对接，同时为文件系统屏蔽不同块设备的差异性，定义了一个以块大小为单位对磁盘块设备进行读写的trait接口。
#### 块缓存层 
I/O设备的读写是影响文件系统性能的关键。为了提升性能，需要利用局部性原理设计缓存以减小I/O设备读写次数。此外，为了避免不同类型的块数据覆盖而造成效率下降，我们设计了双路缓存，分别存储文件数据和检索信息。使用磁盘缓存的另一个好处是可以屏蔽具体的块读写细节，以此提升效率。在我们的设计中，上层模块可以直接向缓存索取需要的块，具体的读写、替换过程交由缓存完成。
#### 磁盘数据结构层 
本层真正开始对文件系统进行组织。FAT32有许多重要的磁盘数据结构，例如引导扇区、扩展引导扇区、文件系统信息扇区、FAT目录项等。他们由不同的字段构成，存储文件系统的信息，部分字段也存在特定的取值。磁盘布局层的工作就是组织这些数据结构，并为上层提供便捷的接口以获取或修改信息。
#### 文件系统管理层 
文件系统管理器层是整个文件系统的核心，其负责文件系统的启动、整体结构的组织、重要信息的维护、簇的分配与回收，以及一些的实用的计算工具。该层为其他模块提供了FAT32相关的实用接口，其他模块如有任何相关的计算或者处理工作。
#### 磁盘虚拟文件系统层 
虚拟文件系统层主要负责为内核提供接口，屏蔽文件系统的内部细节，首要任务就是实现复杂的功能。在该层中，我们定义了虚拟文件结构体以对文件进行描述，其与短目录项成对应关系，共同作为访问文件的入口。该层实现了文件系统常见的功能，例如创建、读写、查找、删除等。
#### 内核虚拟文件系统 
统筹了所有类型的文件，把不同可读写对象抽象出了统一的接口，主要面向系统调用。通过这些接口，相关系统调用可以按一致的编程模式实现，既能提高代码复用率，又具备很强的可扩展性。 
#### 设备管理
在操作系统中，I/O设备管理无处不在，由于各种I/O设备的存在才使得计算机的强大功能。设备管理是内核与设备驱动之间的桥梁，各种I/O设备的高效管理是对一个优秀计算机系统操作系统的考验。对于内核，其需要提供接口使得驱动获取来自用户的控制信息；对于驱动，其需要提供接口以便内核控制和调度。目前来讲，我们的系统仅支持SD Card块设备。
