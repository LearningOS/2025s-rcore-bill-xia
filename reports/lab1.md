# Lab1 Report

## 简答题

### 1. bad 测例出错行为描述

SBI 版本：

```
[rustsbi] RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0
.______       __    __      _______.___________.  _______..______   __
|   _  \     |  |  |  |    /       |           | /       ||   _  \ |  |
|  |_)  |    |  |  |  |   |   (----`---|  |----`|   (----`|  |_)  ||  |
|      /     |  |  |  |    \   \       |  |      \   \    |   _  < |  |
|  |\  \----.|  `--'  |.----)   |      |  |  .----)   |   |  |_)  ||  |
| _| `._____| \______/ |_______/       |__|  |_______/    |______/ |__|
[rustsbi] Implementation     : RustSBI-QEMU Version 0.2.0-alpha.2
```

程序输出如下：

```
[kernel] Store PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
```

分别对应 ch2b_bas_address.rs, ch2b_bad_instructions.rs, ch2b_bas_register.rs。这些异常被 `trap_handler` 中的 match 语句捕获，然后内核停止程序并继续调度其他程序（`exit_current_and_run_next`）。

### 2. 理解 `__alltraps` 和 `__restore`

`__alltraps` 是所有 trap 的处理入口，处理外部中断和cpu内部异常（特别地，syscall）。使用汇编代码保存返回用户态所需的上下文后尽快进入 Rust 代码开始真正处理 trap。处理有两种结果：进程被销毁，或者经过若干次调度后最终回到 `call trap_handler` 的下一行开始执行 `__restore` 回到用户态。

> 这里的上下文是TrapContext，所需状态比TaskContext要多，因为TaskContext切换前后都是内核态从而不涉及切换特权态所需的状态，并且假设TaskContext符合函数调用ABI从而不用存Caller Saved Register。

`__restore` 假设栈顶是一个 TrapContext，恢复各种寄存器之后最后 sret 到用户态。如果他从 `__alltraps` fallthrough下来自然没有问题，但并不要求严格与 trap 配对，例如新任务第一次进用户态用的就是手工构造的 TrapContext（`goto_restore`）。

1. 刚进入 `__restore` 时 sp 是内核栈顶，指向一个 TrapContext。两个场景就是第一次启动进入用户态和从 Trap 返回用户态。
2. 处理了 `sscratch`, `sstatus`, `sepc` 三个寄存器
    - `sscratch` 在用户态保存内核栈，应该是 sret 时设置的，进入 S 态后没用了
    - `sstatus` 包含 sret 之后应处的特权级
    - `sepc` 包含用户态返回地址
3. x2 是 sp，TrapContext 中应该存用户态栈顶而现在 sp 是内核态栈顶，后面在 sscratch 中存进去了（感觉这顺序其实无所谓，这样搞还打乱了访问 cache 的顺序？）x4 是 tp，目前不支持 thread_local 所以先不管。
4. sp 已经指向用户态栈顶，sscratch 指向内核态栈顶
5. sret
6. sp 指向内核态栈顶，sscratch 指向用户态栈顶
7. ecall，或者外部中断或者其他异常

## Honor Code

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

    None

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

    查看 `https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter2/4trap-handling.html#term-s-mod-csr` 讲解 CSR 功能

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

## Feedback

我踩了个坑，就是我开了个 512 的数组试图统计所有 syscall 的调用次数，结果这些 TCB 有 64K 大，似乎把堆还是什么踩爆了，总之会出现非常奇怪的 bug，调了好久。归根结底是内存空间分配没用 MMU 和页表导致太 arbitrary 了，如果有办法离线检查下各种内存地址空间有没有爆体验可能会好一些。