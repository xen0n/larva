# Musl-based Test Programs for LARVa

This directory contains tools to build musl libc from source and create debuggable test programs for LARVa debugging.

## Why Build Musl?

The busybox binary uses musl, but it's stripped and hard to debug. By building musl ourselves with debug symbols (`-g -O0`), we can:

1. See exact line numbers when crashes occur
2. Add print statements to musl for debugging
3. Understand the exact initialization sequence
4. Create minimal test cases

## Quick Start

```bash
cd test

# Step 1: Build musl (one-time, takes ~5-10 minutes)
./build-musl.sh

# Step 2: Build test programs
make

# Step 3: Test with LARVa
make test
```

## Files

- `build-musl.sh` - Downloads and builds musl from source with debug symbols
- `hello-argv.c` - Simple test program using argc/argv
- `Makefile` - Builds test programs with the custom musl
- `README.md` - This file

## Build Details

### Musl Configuration

The build script configures musl with:
- `--enable-debug` - Debug assertions enabled
- `-g` - Full debug symbols
- `-O0` - No optimization (easier debugging)
- `-fno-omit-frame-pointer` - Keep frame pointers for stack traces

### Test Programs

Current test programs:

- **hello-argv.elf** - Uses printf() with argc/argv, exercises:
  - musl's `_start` entry point
  - Stack setup (argc/argv)
  - `__libc_start_main` initialization
  - printf() formatting
  - stdout buffering

## Debugging with GDB

If you have a RISC-V GDB:

```bash
# On host with QEMU
cd test
riscv64-linux-gnu-gdb hello-argv.elf
(gdb) target remote :1234
(gdb) break _start
(gdb) continue
```

## Troubleshooting

### No cross-compiler

Install a RISC-V cross-compiler. On Gentoo:
```bash
crossdev -t riscv64-linux-gnu
```

On Ubuntu/Debian:
```bash
apt-get install gcc-riscv64-linux-gnu
```

### Build fails

Check that you have the required tools:
```bash
# Required
riscv64-linux-gnu-gcc --version
riscv64-linux-gnu-ar --version
make --version
wget --version
```

### LARVa crashes

This is expected! The whole point is to debug why musl crashes in LARVa. Use:

```bash
# With debug output
cd .. && LARVA_DEBUG=1 cargo run --bin larva-run -- ./test/hello-argv.elf

# With syscall tracing
cd .. && LARVA_SYSCALL_LOG=/tmp/syscalls.txt cargo run --bin larva-run -- ./test/hello-argv.elf
cat /tmp/syscalls.txt
```

## License

The build scripts and test programs are dedicated to the public domain (CC0).
Musl itself is MIT licensed.
