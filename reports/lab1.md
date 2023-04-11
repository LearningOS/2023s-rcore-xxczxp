
## 设计思路

首先想到的自然是在TaskControlBlock中加入结构体来储存，但由于所需的内存量太大，如果直接嵌入在初始化的时候会导致爆栈（我试过增加栈的大小，但不知为何会出其他问题，猜测是由于储存内核代码的区域大小有限制，为0x80200000到0x80400000），所以用Box包裹储存在堆上。
之后简单粗暴地通过为TaskManager增加getInner方法来使其他模块可以获取TaskManager的Inner来修改taskInfo

## 简答作业

 1. 

产生如下输出
> [ERROR] [kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x80400414, core dumped.
> [ERROR] [kernel] IllegalInstruction in application, core dumped.
> [ERROR] [kernel] IllegalInstruction in application, core dumped.
>
这三者都触发异常从U态陷入到S态。跳转到__alltraps 函数，之后在完成保存寄存器状态并切换为内核栈后调用trap_handler函数，之后发现异常原因分别是Exception::StoreFault和Exception::IllegalInstruction，并进行log和退出当前用户程序

2. 深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:

    1. L40：刚进入 __restore 时，a0 代表了什么值。请指出 __restore 的两种使用情景。

    a0表示函数第一个参数或函数的返回值，一般来说，__restore的上一条指令为call trap_handler，所以a0代表了trap_handler的返回值也就是cx（储存在内核栈上的trapcontext）。
    所以__restore两种使用场景：
    - __alltraps陷入内核后调用完trap_handler用于返回用户态
    - 在程序初始化的时候将__restore设为起始地址（内核态），并将trapcontext压入内核栈。在内核态调用__switch函数后从__restore开始执行，而在这种情况下，a0代表了__switch函数的第一个参数依旧是上一个进程的trapcontext指针。

    2. L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。

    ld t0, 32*8(sp) 
    ld t1, 33*8(sp)
    ld t2, 2*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2

    从内核栈中恢复了了三个特殊寄存器sstatus，sepc，sscratch
    - sstatus 的SPP 字段给出 Trap 发生之前 CPU 处在哪个特权级（S/U）等信息
    - sepc 给出恢复用户态时所需的运行的指令地址
    - sscratch 则被用于手动储存用户栈
    sscratch在后续代码中与sp交换，进而在用户态存储内核栈指针，而sret时机器会自动帮我们按照sstatus的SPP字段切换特权级，并将PC指针设置为sepc

3. L50-L56：为何跳过了 x2 和 x4？

ld x1, 1*8(sp)
ld x3, 3*8(sp)
.set n, 5
.rept 27
    LOAD_GP %n
    .set n, n+1
.endr

x2 是sp栈指针，其保存值（用户栈地址）储存在sscratch中，不需要手动恢复
x4 按照实验指导书的说法，除非我们手动出于一些特殊用途使用它，否则一般也不会被用到。经Google得知x4是线程指针（指向线程局部变量区域），在lab3中连进程都还不完全，自然没线程什么事。

4. L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？

csrrw sp, sscratch, sp

该指令交换了sp和sscratch，该指令之后，sp指向用户栈，在一会恢复用户态的时候用到，而sscratch储存了当前进程的内核栈指针

5. __restore：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

在L61，sret指令
因为该指令会使 CPU 会将当前的特权级按照 sstatus 的 SPP 字段设置为 U 或者 S 并跳转到 sepc 寄存器指向的那条指令，然后继续执行。


6. L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？

csrrw sp, sscratch, sp

该指令之后，sp为内核栈指针，sscratch为用户栈指针

7. 从 U 态进入 S 态是哪一条指令发生的？

主动陷入是ecall指令

## 荣誉准则


1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

        《你交流的对象说明》

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

        实验指导书
        询问chatgpt （用于查询寄存器作用，结果发现这玩意回答错误）
        进行Google （用于查询寄存器作用）

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

