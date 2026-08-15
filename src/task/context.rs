//! ARM Cortex-M0+ Context Switcher in Assembly
//!
//! Handles stack frame creation and PendSV exception handler for
//! preemptive context switching between tasks.

use core::arch::global_asm;

/// Size of the saved register context (r4-r11 = 8 registers * 4 bytes = 32 bytes)
/// Plus hardware exception frame (r0-r3, r12, lr, pc, xpsr = 8 registers * 4 bytes = 32 bytes)
/// Total initial stack frame = 64 bytes (16 words).
pub const INITIAL_STACK_FRAME_WORDS: usize = 16;

/// Initializes a task stack so that when context-switched into via PendSV,
/// it will cleanly execute `entry(arg)` and exit to `task_exit()` if the function returns.
///
/// # Safety
/// `stack_top` must point to an 8-byte aligned valid memory location within the task stack.
pub unsafe fn init_task_stack(
    stack_top: *mut u32,
    entry: extern "C" fn(usize),
    arg: usize,
    exit_handler: extern "C" fn() -> !,
) -> *mut u32 {
    let mut sp = stack_top;

    // Align stack pointer to 8 bytes downward
    let sp_aligned = (sp as usize) & !0x7;
    sp = sp_aligned as *mut u32;

    // Hardware exception frame (auto-popped on exception return):
    // [sp - 1] = xPSR (Thumb mode flag bit 24 = 1 => 0x0100_0000)
    // [sp - 2] = PC (Entry point)
    // [sp - 3] = LR (Return address -> exit handler)
    // [sp - 4] = R12 (0)
    // [sp - 5] = R3 (0)
    // [sp - 6] = R2 (0)
    // [sp - 7] = R1 (0)
    // [sp - 8] = R0 (Argument to task entry function)
    sp = sp.offset(-1);
    *sp = 0x0100_0000; // xPSR with Thumb bit set

    sp = sp.offset(-1);
    *sp = entry as usize as u32; // PC

    sp = sp.offset(-1);
    *sp = exit_handler as usize as u32; // LR

    sp = sp.offset(-1);
    *sp = 0; // R12

    sp = sp.offset(-1);
    *sp = 0; // R3

    sp = sp.offset(-1);
    *sp = 0; // R2

    sp = sp.offset(-1);
    *sp = 0; // R1

    sp = sp.offset(-1);
    *sp = arg as u32; // R0

    // Software saved frame (r4-r11) popped by PendSV:
    // [sp - 9]  = R11
    // [sp - 10] = R10
    // [sp - 11] = R9
    // [sp - 12] = R8
    // [sp - 13] = R7
    // [sp - 14] = R6
    // [sp - 15] = R5
    // [sp - 16] = R4
    for _ in 0..8 {
        sp = sp.offset(-1);
        *sp = 0;
    }

    sp
}

// Global assembly implementation of PendSV_Handler for Cortex-M0+
// In Thumb-1 (ARMv6-M), stm/ldm and push/pop only work directly on low registers (r0-r7).
// High registers r8-r11 must be moved to r4-r7 to save/restore.
global_asm!(
    r#"
    .syntax unified
    .thumb

    .global PendSV_Handler
    .type PendSV_Handler, %function
    PendSV_Handler:
        cpsid i                  /* Disable interrupts during context switch */

        /* Check if CURRENT_TASK is null */
        ldr r3, =CURRENT_TASK_SP
        ldr r0, [r3]
        cmp r0, #0
        beq 1f                   /* If null, jump directly to loading next task */

        /* Save r4-r7 */
        mrs r0, psp
        subs r0, r0, #32
        stmia r0!, {{r4-r7}}

        /* Move r8-r11 to r4-r7 and save */
        mov r4, r8
        mov r5, r9
        mov r6, r10
        mov r7, r11
        stmia r0!, {{r4-r7}}

        /* Adjust r0 back to top of saved stack and update CURRENT_TASK_SP */
        subs r0, r0, #32
        ldr r3, =CURRENT_TASK_SP
        str r0, [r3]

    1:
        /* Call Rust scheduler to pick next task: NEXT_TASK_SP will be updated */
        push {{lr}}
        bl switch_context_rust
        pop {{r2}}
        mov lr, r2

        /* Load new task SP */
        ldr r3, =CURRENT_TASK_SP
        ldr r0, [r3]

        /* Restore r8-r11 */
        adds r0, r0, #16
        ldmia r0!, {{r4-r7}}
        mov r8, r4
        mov r9, r5
        mov r10, r6
        mov r11, r7

        /* Restore r4-r7 */
        subs r0, r0, #32
        ldmia r0!, {{r4-r7}}
        adds r0, r0, #16

        /* Set PSP to point to hardware frame and return */
        msr psp, r0

        cpsie i                  /* Re-enable interrupts */

        /* EXC_RETURN to Thread mode using PSP (0xFFFFFFFD) */
        ldr r0, =0xFFFFFFFD
        bx r0
    "#
);
