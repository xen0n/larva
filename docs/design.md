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

|RV32F|Implementation|
|:----|:---|
|`flw`|`ld.w` + bitcast|
|`fsw`|`st.w` + bitcast|
|`fmadd.s`|`fmadd.d` (promote to double, compute, convert back)|
|`fmsub.s`|`fmsub.d`|
|`fnmsub.s`|`fnmsub.d`|
|`fnmadd.s`|`fnmadd.d`|
|`fadd.s`|`fadd.d`|
|`fsub.s`|`fsub.d`|
|`fmul.s`|`fmul.d`|
|`fdiv.s`|`fdiv.d`|
|`fsqrt.s`|`fsqrt.d`|
|`fsgnj.s`|`fsgnj.d`|
|`fsgnjn.s`|`fsgnjn.d`|
|`fsgnjx.s`|`fsgnjx.d`|
|`fmin.s`|`fmin.d`|
|`fmax.s`|`fmax.d`|
|`fcvt.w.s`|`ftintrz.w.d` + `movgr2fr.w`|
|`fcvt.wu.s`|`ftintrz.wu.d`|
|`fmv.x.w`|`movfr2gr.s`|
|`feq.s`|`fcmp.ceq.d`|
|`flt.s`|`fcmp.clt.d`|
|`fle.s`|`fcmp.cle.d`|
|`fclass.s`|`fclass.d`|
|`fcvt.s.w`|`movgr2fr.w` + `ffint.s.d`|
|`fcvt.s.wu`|`ffint.su.d`|
|`fmv.w.x`|`movgr2fr.s`|

|RV64F|Implementation|
|:----|:---|
|`fcvt.l.s`|`ftintrz.l.d`|
|`fcvt.lu.s`|`ftintrz.lu.d`|
|`fcvt.s.l`|`ffint.l.d`|
|`fcvt.s.lu`|`ffint.lu.d`|

|RV32D|Implementation|
|:----|:---|
|`fld`|`ld.d`|
|`fsd`|`st.d`|
|`fmadd.d`|`fmadd.d` (native)|
|`fmsub.d`|`fmsub.d`|
|`fnmsub.d`|`fnmsub.d`|
|`fnmadd.d`|`fnmadd.d`|
|`fadd.d`|`fadd.d`|
|`fsub.d`|`fsub.d`|
|`fmul.d`|`fmul.d`|
|`fdiv.d`|`fdiv.d`|
|`fsqrt.d`|`fsqrt.d`|
|`fsgnj.d`|`fsgnj.d`|
|`fsgnjn.d`|`fsgnjn.d`|
|`fsgnjx.d`|`fsgnjx.d`|
|`fmin.d`|`fmin.d`|
|`fmax.d`|`fmax.d`|
|`fcvt.s.d`|`fcvt.s.d`|
|`fcvt.d.s`|`fcvt.d.s`|
|`feq.d`|`fcmp.ceq.d`|
|`flt.d`|`fcmp.clt.d`|
|`fle.d`|`fcmp.cle.d`|
|`fclass.d`|`fclass.d`|
|`fcvt.w.d`|`ftintrz.w.d`|
|`fcvt.wu.d`|`ftintrz.wu.d`|
|`fcvt.d.w`|`ffint.d.w`|
|`fcvt.d.wu`|`ffint.d.wu`|

|RV64D|Implementation|
|:----|:---|
|`fcvt.l.d`|`ftintrz.l.d`|
|`fcvt.lu.d`|`ftintrz.lu.d`|
|`fmv.x.d`|`movfr2gr.d`|
|`fcvt.d.l`|`ffint.d.l`|
|`fcvt.d.lu`|`ffint.d.lu`|
|`fmv.d.x`|`movgr2fr.d`|

## Implementation notes

### Floating-point

The interpreter assumes the host provides IEEE 754-2008 compliant floating-point
with nan2008 NaN signaling/quiet semantics. This is standard on modern systems
(x86_64, ARM, Loongson 3A4000+) but legacy MIPS implementations may differ.

RISC-V (and LoongArch) use nan2008 semantics where:
- Quiet NaN has the most significant bit of the mantissa set (0x7FC00000)
- Signaling NaN has the MSB clear (0x7F800001)

Legacy MIPS used the opposite convention.
