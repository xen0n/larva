# LARVa - Bridging LoongArch to RISC-V

This project is a proof-of-concept RISC-V emulator aiming at near-native
execution performance on LoongArch.
Support may expand to other architectures in the future, if the techniques
employed here prove useful and reasonably arch-independent.

The project is named after a popular but extremely difficult chart with the
same name, in the rhythm game *maimai*. Binary translation is hard, running such
logic in privileged mode is even harder; while I cannot play the *maimai*
chart at all, I do hope to manage the difficulty *here* somehow!

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
