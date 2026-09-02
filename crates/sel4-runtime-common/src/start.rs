//
// Copyright 2025, Colias Group, LLC
//
// SPDX-License-Identifier: BSD-2-Clause
//

use core::arch::global_asm;

#[macro_export]
macro_rules! declare_entrypoint {
    () => {
        $crate::_private::global_asm! {
            r#"
                .extern __sel4_runtime_common__call_rust_entrypoint

                .section .text

                .global _start
                _start:
            "#,
            #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
            r#"
                    b __sel4_runtime_common__call_rust_entrypoint
            "#,
            #[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
            r#"
                    j __sel4_runtime_common__call_rust_entrypoint
            "#,
            #[cfg(target_arch = "x86_64")]
            r#"
                    jmp __sel4_runtime_common__call_rust_entrypoint
            "#,
        }
    };
}

#[macro_export]
macro_rules! declare_entrypoint_with_stack_init {
    () => {
        $crate::_private::global_asm! {
            r#"
                .extern __sel4_runtime_common__stack_init

                .section .text

                .global _start
                _start:
            "#,
            #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
            r#"
                    b __sel4_runtime_common__stack_init
            "#,
            #[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
            r#"
                    j __sel4_runtime_common__stack_init
            "#,
            #[cfg(target_arch = "x86_64")]
            r#"
                    jmp __sel4_runtime_common__stack_init
            "#,
        }
    };
}

#[macro_export]
macro_rules! declare_stack {
    ($size:expr) => {
        const _: () = {
            #[unsafe(no_mangle)]
            static __sel4_runtime_common__stack_bottom: $crate::_private::StackBottom = {
                static STACK: $crate::_private::Stack<{ $size }> = $crate::_private::Stack::new();
                STACK.bottom()
            };
        };
    };
}

global_asm! {
    r#"
        .extern __sel4_runtime_common__rust_entrypoint

        .section .text.__sel4_runtime_common__call_rust_entrypoint, "ax", %progbits

        .global __sel4_runtime_common__call_rust_entrypoint
        __sel4_runtime_common__call_rust_entrypoint:
    "#,
    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    r#"
            bl __sel4_runtime_common__rust_entrypoint
        1:  b 1b
    "#,
    #[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
    r#"
        .option push
        .option norelax
        1:  auipc gp, %pcrel_hi(__global_pointer$)
            addi gp, gp, %pcrel_lo(1b)
        .option pop
            jal __sel4_runtime_common__rust_entrypoint
        1:  j 1b
    "#,
    #[cfg(target_arch = "x86_64")]
    r#"
            call __sel4_runtime_common__rust_entrypoint
        1:  jmp 1b
    "#,
}

global_asm! {
    r#"
        .extern __sel4_runtime_common__stack_bottom
        .extern __sel4_runtime_common__call_rust_entrypoint

        .section .text.__sel4_runtime_common__stack_init, "ax", %progbits

        .global __sel4_runtime_common__stack_init
        __sel4_runtime_common__stack_init:
    "#,
    #[cfg(target_arch = "aarch64")]
    r#"
            ldr x9, =__sel4_runtime_common__stack_bottom
            ldr x9, [x9]
            mov sp, x9
            b __sel4_runtime_common__call_rust_entrypoint
    "#,
    #[cfg(target_arch = "arm")]
    r#"
            ldr r8, =__sel4_runtime_common__stack_bottom
            ldr r8, [r8]
            mov sp, r8
            b __sel4_runtime_common__call_rust_entrypoint
    "#,
    #[cfg(target_arch = "riscv64")]
    r#"
            la sp, __sel4_runtime_common__stack_bottom
            ld sp, (sp)
            j __sel4_runtime_common__call_rust_entrypoint
    "#,
    #[cfg(target_arch = "riscv32")]
    r#"
            la sp, __sel4_runtime_common__stack_bottom
            lw sp, (sp)
            j __sel4_runtime_common__call_rust_entrypoint
    "#,
    #[cfg(target_arch = "x86_64")]
    r#"
            mov rsp, __sel4_runtime_common__stack_bottom
            mov rbp, rsp
            // The stack must be 16-byte aligned at each `call`, so that the
            // callee sees `rsp % 16 == 8` once the return address is pushed.
            // `__sel4_runtime_common__stack_bottom` is 16-byte aligned and two
            // `call`s separate this point from Rust code: the one below, and
            // the one in `__sel4_runtime_common__call_rust_entrypoint`. One
            // 8-byte adjustment makes both land correctly; adjusting twice
            // leaves Rust entered with `rsp % 16 == 0`, and the first SSE spill
            // in the entrypoint's callees (`movaps`) then raises #GP.
            sub rsp, 0x8
            call __sel4_runtime_common__call_rust_entrypoint
        1:  jmp 1b
    "#,
}
