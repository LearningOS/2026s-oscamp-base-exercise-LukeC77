//! # Stackful Coroutine and Context Switch (riscv64)
//!
//! In this exercise, you implement the minimal context switch using inline assembly,
//! which is the core mechanism of OS thread scheduling. This crate is **riscv64 only**;
//! run `cargo test` on riscv64 Linux, or use the repo's normal flow (`./check.sh` / `oscamp`) on x86 with QEMU.
//!
//! ## Key Concepts
//! - **Callee-saved registers**: Save and restore them on switch so the switched-away task can resume correctly later.
//! - **Stack pointer `sp`** and **return address `ra`**: Restore them in the new context; the first time we switch to a task, `ret` jumps to `ra` (the entry point).
//! - Inline assembly: `core::arch::asm!`
//!
//! ## riscv64 ABI (for this exercise)
//! - Callee-saved: `sp`, `ra`, `s0`–`s11`. The `ret` instruction is `jalr zero, 0(ra)`.
//! - First and second arguments: `a0` (old context), `a1` (new context).

#![cfg(target_arch = "riscv64")]

/// Saved register state for one task (riscv64). Layout must match the offsets used in the asm below: for one task (riscv64). Layout must match the offsets used in the asm below:
/// `sp` at 0, `ra` at 8, then `s0`–`s11` at 16, 24, … 104.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
/// 一个按 riscv64 ABI 精确布局的寄存器快照结构体，
/// 用于在 context switch 时保存/恢复 sp、ra、s0–s11，
/// 从而让任务可以被暂停并在未来恢复执行。
pub struct TaskContext {
    pub sp: u64,
    pub ra: u64,
    pub s0: u64,
    pub s1: u64,
    pub s2: u64,
    pub s3: u64,
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub s8: u64,
    pub s9: u64,
    pub s10: u64,
    pub s11: u64,
}

impl TaskContext {
    pub const fn empty() -> Self {
        Self {
            sp: 0,
            ra: 0,
            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
        }
    }

    /// Initialize this context so that when we switch to it, execution starts at `entry`.
    ///
    /// - Set `ra = entry` so that the first `ret` in the new context jumps to `entry`.
    /// - Set `sp = stack_top` with 16-byte alignment (RISC-V ABI requires 16-byte aligned stack at function entry).
    /// - Leave `s0`–`s11` zero; they will be loaded on switch.
    /// 
    /// ra = entry 表示把返回地址寄存器设置为 entry；当执行 ret 时，CPU 会把 program counter 设置为 ra，因此程序会从 entry 地址开始执行。
    /// jal ra, func命令表示ra = return address，跳到 func
    /// ret命令表示PC = ra（跳回去
    /// entry是这个任务第一次开始运行时，要从哪个函数开始执行，通常是线程入口函数、协程入口函数、scheduler 给这个任务指定的启动函数，不是“整个进程”，而是“这个执行流第一次启动的入口点”。
    /// 一个新任务还没有运行过，所以它的寄存器状态是未知的；我们通过 init 函数把它初始化为一个合理的状态，使得当我们第一次 switch 到这个任务时，它能正确地从 entry 开始执行。
    /// ret表示PC = ra，跳到 ra 指向的地址
    /// 即使是新任务，也必须有 TaskContext，因为调度器只会执行“恢复一个上下文”；新任务需要通过人为构造 sp 和 ra，伪装成一个可以被恢复的执行状态。
    pub fn init(&mut self, stack_top: usize, entry: usize) {
        //todo!("set ra = entry, sp = stack_top (16-byte aligned)")
        self.ra = entry as u64; //第一次切换到这个 context 后，ret 跳到 entry
        self.sp = (stack_top as u64) & !0xf; //RISC-V ABI（调用约定）硬性规定:在函数入口时，sp 必须是 16 字节对齐
    }
}

/// Switch from `old` to `new` context: save current callee-saved regs into `old`, load from `new`, then `ret` (jumps to `new.ra`).
///
/// In asm: store `sp`, `ra`, `s0`–`s11` to `[a0]` (old), load from `[a1]` (new), zero `a0`/`a1` so we do not leak pointers into the new context, then `ret`.
///
/// Must be `#[unsafe(naked)]` to prevent the compiler from generating a prologue/epilogue.
/// 
/// naked_asm!需要使用纯汇编语言写
#[unsafe(naked)]
pub unsafe extern "C" fn switch_context(old: &mut TaskContext, new: &TaskContext) {
    //todo!("save callee-saved regs to old, load from new, then ret; use #[unsafe(naked)] + naked_asm!, see module doc for riscv64 ABI and layout")
    std::arch::naked_asm!(
        "sd sp, 0(a0)", // save sp to old.sp
        "sd ra, 8(a0)", // save ra to old.ra
        "sd s0, 16(a0)", // save s0 to old.s0
        "sd s1, 24(a0)", // save s1 to old.s1
        "sd s2, 32(a0)", // save s2 to old.s2
        "sd s3, 40(a0)", // save s3 to old.s3
        "sd s4, 48(a0)", // save s4 to old.s4
        "sd s5, 56(a0)", // save s5 to old.s5
        "sd s6, 64(a0)", // save s6 to old.s6
        "sd s7, 72(a0)", // save s7 to old.s7
        "sd s8, 80(a0)", // save s8 to old.s8
        "sd s9, 88(a0)", // save s9 to old.s9
        "sd s10, 96(a0)", // save s10 to old.s10
        "sd s11, 104(a0)", // save s11 to old.s11

        "ld sp, 0(a1)", // load new.sp to sp
        "ld ra, 8(a1)", // load new.ra to ra
        "ld s0, 16(a1)", // load new.s0 to s0
        "ld s1, 24(a1)", // load new.s1 to s1
        "ld s2, 32(a1)", // load new.s2 to s2
        "ld s3, 40(a1)", // load new.s3 to s3
        "ld s4, 48(a1)", // load new.s4 to s4
        "ld s5, 56(a1)", // load new.s5 to s5
        "ld s6, 64(a1)", // load new.s6 to s6
        "ld s7, 72(a1)", // load new.s7 to s7
        "ld s8, 80(a1)", // load new.s8 to s8
        "ld s9, 88(a1)", // load new.s9 to s9
        "ld s10, 96(a1)", // load new.s10 to s10
        "ld s11, 104(a1)", // load new.s11 to s11

        "li a0, 0", // zero a0 to avoid leaking old context pointer
        "li a1, 0", // zero a1 to avoid leaking new context pointer
        "ret", // jump to new.ra
    )
}

const STACK_SIZE: usize = 1024 * 64;

/// Allocate a stack for a coroutine. Returns `(buffer, stack_top)` where `stack_top` is the high address
/// (stack grows down). The buffer must be kept alive for the lifetime of the context using this stack.
pub fn alloc_stack() -> (Vec<u8>, usize) {
    //todo!("allocate stack buffer, return (buffer, stack_top) with stack_top 16-byte aligned") stack_top地址必须是 16, 32, 48, 64 ... 只有特定地址满足
    let buffer = vec![0u8; STACK_SIZE];
    let stack_top = buffer.as_ptr() as usize + STACK_SIZE;
    (buffer, stack_top & !0xf) // ensure stack_top is 16-byte
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    extern "C" fn task_entry() {
        COUNTER.store(42, Ordering::SeqCst);
        loop {
            std::hint::spin_loop();
        }
    }

    #[test]
    fn test_alloc_stack() {
        let (buf, top) = alloc_stack();
        assert_eq!(top, buf.as_ptr() as usize + STACK_SIZE);
        assert!(top % 16 == 0);
    }

    #[test]
    fn test_context_init() {
        let (buf, top) = alloc_stack();
        let _ = buf;
        let mut ctx = TaskContext::empty();
        let entry = task_entry as *const () as usize;
        ctx.init(top, entry);
        assert_eq!(ctx.ra, entry as u64);
        assert!(ctx.sp != 0);
    }

    #[test]
    fn test_switch_to_task() {
        COUNTER.store(0, Ordering::SeqCst);

        static mut MAIN_CTX_PTR: *mut TaskContext = std::ptr::null_mut();
        static mut TASK_CTX_PTR: *mut TaskContext = std::ptr::null_mut();

        extern "C" fn cooperative_task() {
            COUNTER.store(99, Ordering::SeqCst);
            unsafe {
                switch_context(&mut *TASK_CTX_PTR, &*MAIN_CTX_PTR);
            }
        }

        let (_stack_buf, stack_top) = alloc_stack();
        let mut main_ctx = TaskContext::empty();
        let mut task_ctx = TaskContext::empty();
        task_ctx.init(stack_top, cooperative_task as *const () as usize);

        unsafe {
            MAIN_CTX_PTR = &mut main_ctx;
            TASK_CTX_PTR = &mut task_ctx;
            switch_context(&mut main_ctx, &task_ctx);
        }

        assert_eq!(COUNTER.load(Ordering::SeqCst), 99);
    }
}
