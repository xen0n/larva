# Minimal RISC-V test program for LARVa

This directory contains a minimal RISC-V assembly program that uses Linux syscalls directly (no libc), useful for testing LARVa without musl's complex initialization.

## Files

- `build.sh` - Build script that creates the binary using Python (no cross-compiler needed)
- `hello.elf` - Pre-built test binary
- `README.md` - This file

## Building

No cross-compiler required! The build script uses Python to generate the ELF:

```bash
cd test/minimal
./build.sh
```

This creates a 64-bit RISC-V static executable that:
- Uses `write(1, "hello\n", 6)` syscall directly
- Uses `exit_group(0)` to exit
- Has no libc, no dynamic linking, no TLS setup

## Usage with LARVa

```bash
# From the project root
cargo run --bin larva-run -- ./test/minimal/hello.elf
```

## Expected Output

```
Loaded ELF: entry=0000000000010000, sp=..., phdr=...
hello
```

## How It Works

The build script creates a minimal ELF executable:

1. **ELF Header** (64 bytes) - Standard 64-bit RISC-V executable header
2. **Program Header** (56 bytes) - Single PT_LOAD segment for code+data
3. **Padding** to 4KB alignment
4. **Code** at virtual address 0x10000:
   ```asm
   _start:
       li a0, 1          # stdout
       auipc a1, 0       # get PC
       addi a1, a1, 40   # point to message
       li a2, 6          # length
       li a7, 64         # write syscall
       ecall
       li a0, 0          # exit code 0
       li a7, 93         # exit_group syscall
       ecall
   ```
5. **Message** "hello\n" at offset 0x2c within the segment

## License

This test code is dedicated to the public domain (CC0).
