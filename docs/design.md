# LARVa Design Sketches

For performance, the translation should be mostly one-pass, except for some
beneficial "macro-op fusion" peephole optimizations. Regalloc is necessary
though, and some kind of address space manipulation is also needed for
transparency of emulation.

## User-mode insn correspondence

These are nearly 1:1, which is extremely convenient.

We model an RV64GC core, so XLEN=64 is assumed for native-width insns below.

|RV Privileged|LA64|
|:------------|:---|
|`ecall`|`syscall`|

|RV32I|LA64|
|:----|:---|
|`lui`|`lu12i.w`|
|`auipc`|`pcaddu12i`|
|`jal`|`jirl`?|
|`jalr`|`jirl`|
|`beq`|`beq`|
|`bne`|`bne`|
|`blt`|`bgt`|
|`bge`|`ble`|
|`bltu`|`bgtu`|
|`bgeu`|`bleu`|
|`lb`|`ld.b`|
|`lh`|`ld.h`|
|`lw`|`ld.w`|
|`lbu`|`ld.bu`|
|`lhu`|`ld.hu`|
|`sb`|`st.b`|
|`sh`|`st.h`|
|`sw`|`st.w`|
|`addi`|`addi.d`|
|`slti`|`slti`|
|`sltiu`|`sltui`|
|`xori`|`xori`|
|`ori`|`ori`|
|`andi`|`andi`|
|`slli`|`slli.d`|
|`srli`|`srli.d`|
|`srai`|`srai.d`|
|`add`|`add.d`|
|`sub`|`sub.d`|
|`sll`|`sll.d`|
|`slt`|`slt`|
|`sltu`|`sltu`|
|`xor`|`xor`|
|`srl`|`srl.d`|
|`sra`|`sra.d`|
|`or`|`or`|
|`and`|`and`|
|`fence`|`dbar`|
|`fence_i`|`ibar`|
|`csrrw`|TODO|
|`csrrs`|TODO|
|`csrrc`|TODO|
|`csrrwi`|TODO|
|`csrrsi`|TODO|
|`csrrci`|TODO|

Only `dbar 0` is available on LA64 v1.00, but finer-grained barriers should
appear in the next revision (and Loongson 3A6000).

|RV64I|LA64|
|:----|:---|
|`lwu`|`ld.wu`|
|`ld`|`ld.d`|
|`sd`|`st.d`|
|`addiw`|`add.w`|
|`slliw`|`slli.w`|
|`srliw`|`srli.w`|
|`sraiw`|`srai.w`|
|`addw`|`add.w`|
|`subw`|`sub.w`|
|`sllw`|`sll.w`|
|`srlw`|`srl.w`|
|`sraw`|`sra.w`|

|RV32M|LA64|
|:----|:---|
|`mul`|`mul.d`|
|`mulh`|`mulh.d`|
|`mulhsu`|X|
|`mulhu`|`mulh.du`|
|`div`|`div.d`|
|`divu`|`div.du`|
|`rem`|`mod.d`|
|`remu`|`mod.du`|

`mulhsu` multiplies a signed value by an unsigned value, which has no direct
LA64 correspondence.

|RV64M|LA64|
|:----|:---|
|`mulw`|`mul.w`|
|`divw`|`div.w`|
|`divuw`|`div.wu`|
|`remw`|`mod.w`|
|`remuw`|`mod.wu`|

|RV32A|LA64|
|:----|:---|
|`lr_w`|`ll.w`|
|`sc_w`|`sc.w`|
|`amoswap_w`|`amswap.w`|
|`amoadd_w`|`amadd.w`|
|`amoxor_w`|`amxor.w`|
|`amoand_w`|`amand.w`|
|`amoor_w`|`amor.w`|
|`amomin_w`|`ammin.w`|
|`amomax_w`|`ammax.w`|
|`amominu_w`|`ammin.wu`|
|`amomaxu_w`|`ammax.wu`|

|RV64A|LA64|
|:----|:---|
|`lr_d`|`ll.d`|
|`sc_d`|`sc.d`|
|`amoswap_d`|`amswap.d`|
|`amoadd_d`|`amadd.d`|
|`amoxor_d`|`amxor.d`|
|`amoand_d`|`amand.d`|
|`amoor_d`|`amor.d`|
|`amomin_d`|`ammin.d`|
|`amomax_d`|`ammax.d`|
|`amominu_d`|`ammin.du`|
|`amomaxu_d`|`ammax.du`|

RVF and RVD correspondences: TODO
