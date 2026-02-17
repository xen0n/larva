#!/bin/bash
# Build minimal RISC-V binary with correct file layout

set -e
OUTPUT="${1:-hello.elf}"

python3 << 'PYEOF'
import struct

# ELF header (64 bytes)
e_ident = b'\x7fELF'
e_ident += bytes([2, 1, 1, 0]) + bytes([0] * 8)

e_type = 2
e_machine = 0xf3
e_version = 1
e_entry = 0x10000
e_phoff = 64
e_shoff = 0
e_flags = 0
e_ehsize = 64
e_phentsize = 56
e_phnum = 1
e_shentsize = 0
e_shnum = 0
e_shstrndx = 0

elf_header = e_ident
elf_header += struct.pack('<HHIQQQIHHHHHH', 
    e_type, e_machine, e_version, e_entry, e_phoff, e_shoff,
    e_flags, e_ehsize, e_phentsize, e_phnum, e_shentsize, e_shnum, e_shstrndx)

# Program header - PT_LOAD at offset 0x1000 (4KB) in file, mapped at 0x10000
p_type = 1
p_flags = 5  # R | X
p_offset = 0x1000  # File offset where segment starts
p_vaddr = 0x10000  # Virtual address
p_paddr = 0x10000
p_filesz = 0x1000  # 4KB segment
p_memsz = 0x1000
p_align = 0x1000

phdr = struct.pack('<IIQQQQQQ',
    p_type, p_flags, p_offset, p_vaddr, p_paddr, p_filesz, p_memsz, p_align)

# RISC-V instructions
def addi(rd, rs1, imm):
    opcode = 0x13
    funct3 = 0
    insn = (imm & 0xfff) << 20 | (rs1 & 0x1f) << 15 | funct3 << 12 | (rd & 0x1f) << 7 | opcode
    return struct.pack('<I', insn)

def auipc(rd, imm):
    opcode = 0x17
    insn = (imm & 0xfffff) << 12 | (rd & 0x1f) << 7 | opcode
    return struct.pack('<I', insn)

def ecall():
    return struct.pack('<I', 0x00000073)

# Code at virtual address 0x10000 (file offset 0x1000)
# Message at virtual address 0x1002c (file offset 0x102c)
code = b''
code += addi(10, 0, 1)      # a0 = 1
code += auipc(11, 0)        # a1 = pc (0x10004)
code += addi(11, 11, 0x28)  # a1 = a1 + 40 = 0x1002c
code += addi(12, 0, 6)      # a2 = 6
code += addi(17, 0, 64)     # a7 = 64
code += ecall()
code += addi(10, 0, 0)      # a0 = 0
code += addi(17, 0, 93)     # a7 = 93
code += ecall()

# Pad to offset 0x2c (44 bytes) within segment
while len(code) < 0x2c:
    code += b'\x00'

# Message at 0x1002c
code += b'hello\n'

# Pad to 4KB
while len(code) < 0x1000:
    code += b'\x00'

# Build file: ELF header + PHDR + padding to 0x1000 + segment
file_data = elf_header + phdr
while len(file_data) < 0x1000:
    file_data += b'\x00'
file_data += code

with open('/tmp/hello.elf', 'wb') as f:
    f.write(file_data)

print("Created /tmp/hello.elf")
PYEOF

cp /tmp/hello.elf "$OUTPUT"
chmod +x "$OUTPUT"
file "$OUTPUT"
