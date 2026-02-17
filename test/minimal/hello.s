# Minimal RISC-V "hello world" using raw Linux syscalls
# No libc, no musl - just syscalls

    .section .text
    .global _start
    .align 2

_start:
    # write(1, msg, len)
    li      a0, 1           # fd = stdout
    la      a1, msg         # buf = message
    li      a2, 6           # len = 6 ("hello\n")
    li      a7, 64          # syscall number for write
    ecall

    # exit_group(0)
    li      a0, 0           # exit code = 0
    li      a7, 93          # syscall number for exit_group
    ecall

    .section .rodata
    .align 2
msg:
    .string "hello\n"
