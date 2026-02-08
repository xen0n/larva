# TODO

This file tracks tasks for AI agent collaboration on LARVa. Pick a task, create a branch, and submit a PR.

## How to use this file

- **Claim a task**: Open a PR that adds your name/agent ID next to the task
- **Mark done**: Check the box when PR is merged
- **Add tasks**: File an issue or PR to add new tasks
- **Check mutex first**: Look at the **Mutex** section before starting work

## Mutex (Work in Progress)

These tasks are currently being worked on. **Do not start these concurrently** — coordinate with the assigned agent first.

| Task | Assigned To | PR/Branch | Started |
|------|-------------|-----------|---------|
| *(none)* | — | — | — |

## Current Tasks

### High Priority

- [ ] **Add CI workflow** — GitHub Actions for build, test, clippy

- [ ] **Complete RV64A atomic operations** — implement all 22 `todo!()` atomics in `src/exec/interp/mod.rs`
  - `LrW`, `ScW`, all `Amo*W` variants
  - `LrD`, `ScD`, all `Amo*D` variants
  - See `docs/design.md` for LoongArch correspondence

- [ ] **Complete RVF (float) operations** — implement ~25 `todo!()` float ops
  - Arithmetic: `FaddS`, `FsubS`, `FmulS`, `FdivS`, `FsqrtS`
  - Comparisons: `FeqS`, `FltS`, `FleS`, `FclassS`
  - Conversions: `Fcvt*S` variants
  - See `docs/design.md` for missing correspondences

- [ ] **Complete RVD (double) operations** — implement ~25 `todo!()` double ops
  - Same structure as RVF
  - `FmaddD`, `FmsubD`, etc.

### Medium Priority

- [ ] **Expand syscall coverage** — add syscalls for running real programs
  - [ ] `read` — needed for stdin
  - [ ] `openat` / `open` — file opening
  - [ ] `mmap` / `munmap` — memory mapping
  - [ ] `close` — file closing
  - [ ] `brk` — heap management
  - Target: run a simple static binary (e.g., `busybox`)

- [ ] **Implement TLS (Thread-Local Storage)** — needed for multi-threaded programs
  - Add `tp` register to `RvIsaState`
  - Handle `fs`/`gs` segment references in syscalls

- [ ] **Add proper test runner** — replace hardcoded hello-world with actual tests
  - Compile RISC-V test programs with `riscv64-linux-gnu-gcc -static`
  - Run and compare output against QEMU
  - Add to CI

### Low Priority

- [ ] **Implement FenceI** — instruction fence for self-modifying code
- [ ] **Improve error handling** — replace more `unwrap()` with proper errors
- [ ] **Documentation** — add module-level docs to `src/exec/`

## Future / Research

- [ ] **Binary translation prototype** — start LoongArch code generation
  - Research: CRanelift, LLVM, or handwritten assembly?
  - Start with simple block translation
  - Target: translate `addi` → `addi.d`

- [ ] **System-level emulation** — run a minimal kernel
  - Requires implementing privileged mode
  - Guest MMU for virtual memory

## Done

- [x] AGENTS.md — guide for AI agent collaboration
- [x] Update deps and Rust 2024 edition
- [x] Fix all clippy warnings
