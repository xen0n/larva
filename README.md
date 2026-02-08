# LARVa - Bridging LoongArch to RISC-V

This project is a proof-of-concept RISC-V emulator aiming at near-native
execution performance on LoongArch.
Support may expand to other architectures in the future, if the techniques
employed here prove useful and reasonably arch-independent.

The project is named after a popular but extremely difficult chart with the
same name, in the rhythm game *maimai*. Binary translation is hard, running such
logic in privileged mode is even harder; while I cannot play the *maimai*
chart at all, I do hope to manage the difficulty *here* somehow!

## Requirements

- **Host architecture**: Currently developed and tested on x86_64; designed to be
  architecture-agnostic for the interpreter
- **Floating-point**: IEEE 754-2008 compliant implementation with nan2008 NaN
  signaling/quiet semantics required (standard on modern systems including
  Loongson 3A4000+, but legacy MIPS may differ)

## License

[GPL-3.0-or-later](https://spdx.org/licenses/GPL-3.0-or-later.html)

## Roadmap

* [x] RV64GC disassembly
* [ ] verification interpreter -- WIP
* [ ] emulation machinery
    * [x] guest MMU -- barebones
* [ ] linux-user emulation
    * [x] stack -- works okay
    * [ ] thread-local storage
    * [ ] syscalls -- WIP, only `exit_group` so far
* [ ] LoongArch assembly
* [ ] translation passes
* [ ] system level PoC
    - TODO
