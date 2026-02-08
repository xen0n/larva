# AGENTS.md

This document guides AI agents working on LARVa — a proof-of-concept RISC-V to LoongArch binary translator. Use this as the maintainer-preferred guide for automated changes.

## Key expectations

- Keep changes minimal and scoped.
- One logical change per commit (no unrelated edits in the same commit).
- Prefer safe, idiomatic Rust; avoid `unsafe` unless absolutely necessary.
- Avoid reformatting unrelated files.
- Review diffs for unrelated changes before finalizing.
- Update this AGENTS.md when architectural changes occur.

## Project overview

- **Language**: Rust (Edition 2021)
- **Goal**: Near-native RISC-V (RV64GC) emulation on LoongArch via binary translation
- **License**: GPL-3.0-or-later

High-level layout:

- `src/rv/`: RISC-V instruction definitions, decoding, disassembly
- `src/exec/`: Execution engine (interpreter, memory management)
- `src/bin/`: Executable tools (`larva-disas`, `larva-test`)
- `docs/`: Design documentation and instruction correspondence tables

## Common commands

```bash
# Build
cargo build

# Build release
cargo build --release

# Run tests
cargo test

# Run the hello-world demo
cargo run --bin larva-test

# Disassemble a RISC-V binary
cargo run --bin larva-disas -- <file>

# Check formatting
cargo fmt -- --check

# Run clippy
cargo clippy -- -D warnings
```

Run the minimal set needed for the touched area.

## Architecture notes

### Instruction handling

- `src/rv/insn.rs`: Central `RvInsn` enum with all instruction variants
- `src/rv/args.rs`: Instruction argument types (R-type, I-type, etc.)
- `src/rv/disas_helper.rs`: Disassembly formatting helpers
- Adding new instructions: update enum, decoder, and interpreter

### Execution flow

1. **Decode**: `RvDecoder` converts bytes to `RvInsn`
2. **Interpret**: `RvInterpreterExecutor` executes one instruction at a time
3. **Memory**: `GuestMmu` handles guest→host address translation
4. **Syscalls**: Minimal linux-user emulation in `src/exec/interp/syscall.rs`

### StopReason handling

The interpreter returns `StopReason` after each instruction:
- `Next`: Continue to next PC
- `ContinueAt(addr)`: Jump to addr
- `Break`, `Segv`, `ReservedInsn`: Stop execution

## Code style and conventions

- Follow `rustfmt` defaults; run `cargo fmt` before committing
- Clippy-clean code; no warnings in CI
- Prefer explicit error handling over `unwrap()`/ `expect()`
- Use `todo!()` for unimplemented cases (mark with issue reference if known)
- Keep `unsafe` blocks minimal and documented with safety comments

## Commit message style

Follow Conventional Commits:

```
<type>(<scope>): <summary>
```

Types:
- `feat`: New feature or instruction implementation
- `fix`: Bug fix
- `refactor`: Code restructuring without behavior change
- `docs`: Documentation only
- `test`: Adding or fixing tests
- `build`: Build system or dependencies

Scopes:
- `rv`: RISC-V decoding/instructions
- `exec`: Execution engine
- `interp`: Interpreter specifically
- `mmu`: Memory management
- `syscall`: System call handling
- `disas`: Disassembler
- `test`: Test utilities

Guidelines:
- Imperative, present-tense summary (no trailing period)
- ~50-72 characters for summary
- One logical change per commit
- Include body explaining motivation for non-trivial changes
- For LLM-generated commits:
  - Add `Original prompt:` in body with blockquote
  - Add `Co-authored-by: <agent model name>` trailer

Examples:
```
feat(rv): implement remaining RV64A atomic operations

Implement lr.d, sc.d, and all AMO instructions for RV64A.
Only single-threaded execution is supported; atomics
behave sequentially consistent.

Original prompt:
> Implement the remaining atomic operations in the interpreter.

Co-authored-by: Kimi k2.5
```

## Testing strategy

- `cargo test` for unit tests (currently minimal)
- `cargo run --bin larva-test` for integration (must print "hello world")
- Test against real RISC-V binaries compiled with `riscv64-linux-gnu-gcc`

## Roadmap priorities

See README.md for full roadmap. Current focus areas:

1. **Syscalls**: Expand beyond `write`/`exit_group` to support real programs
2. **Floating point**: Implement RVF/RVD (currently mostly `todo!()`)
3. **Atomics**: Implement RV32A/RV64A for multi-threaded guests
4. **Translation**: Begin LoongArch code generation (the core BT engine)

## Validation checklist

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` is clean
- [ ] `cargo fmt -- --check` passes
- [ ] `cargo run --bin larva-test` outputs "hello world"
- [ ] New instructions tested with hand-crafted or compiled RISC-V code
