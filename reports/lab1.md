
# 简答作业

## 1. 
产生如下输出
> [ERROR] [kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x80400414, core dumped.
> [ERROR] [kernel] IllegalInstruction in application, core dumped.
> [ERROR] [kernel] IllegalInstruction in application, core dumped.
>
这三者都触发异常从U态陷入到S态。跳转到__alltraps 函数，之后在完成保存寄存器状态并切换为内核栈后调用trap_handler函数，之后发现异常原因分别是Exception::StoreFault和Exception::IllegalInstruction，并进行log和退出当前用户程序

2. 深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:

    L40：刚进入 __restore 时，a0 代表了什么值。请指出 __restore 的两种使用情景。

    L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。

    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    ld t2, 2*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2

3. L50-L56：为何跳过了 x2 和 x4？

ld x1, 1*8(sp)
ld x3, 3*8(sp)
.set n, 5
.rept 27
    LOAD_GP %n
    .set n, n+1
.endr

4. L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？

csrrw sp, sscratch, sp

5. __restore：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

6. L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？

csrrw sp, sscratch, sp

7. 从 U 态进入 S 态是哪一条指令发生的？

