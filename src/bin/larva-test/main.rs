use larva::exec;

fn main() {
    /*
    00000000000100e8 <fib>:
       100e8:       4785                    li      a5,1
       100ea:       02a7d563                bge     a5,a0,10114 <fib+0x2c>
       100ee:       1101                    addi    sp,sp,-32
       100f0:       ec06                    sd      ra,24(sp)
       100f2:       e822                    sd      s0,16(sp)
       100f4:       e426                    sd      s1,8(sp)
       100f6:       842a                    mv      s0,a0
       100f8:       3579                    addiw   a0,a0,-2
       100fa:       fefff0ef                jal     ra,100e8 <fib>
       100fe:       84aa                    mv      s1,a0
       10100:       fff4051b                addiw   a0,s0,-1
       10104:       fe5ff0ef                jal     ra,100e8 <fib>
       10108:       9d25                    addw    a0,a0,s1
       1010a:       60e2                    ld      ra,24(sp)
       1010c:       6442                    ld      s0,16(sp)
       1010e:       64a2                    ld      s1,8(sp)
       10110:       6105                    addi    sp,sp,32
       10112:       8082                    ret
       10114:       4505                    li      a0,1
       10116:       8082                    ret

    0000000000010118 <_start>:
       10118:       1141                    addi    sp,sp,-16
       1011a:       e406                    sd      ra,8(sp)
       1011c:       452d                    li      a0,11
       1011e:       fcbff0ef                jal     ra,100e8 <fib>
       10122:       05d00893                li      a7,93
       10126:       00000073                ecall
             */
    let mem: [u8; 66] = [
        0x85, 0x47, 0x63, 0xd5, 0xa7, 0x02, 0x01, 0x11, 0x06, 0xec, 0x22, 0xe8, 0x26, 0xe4, 0x2a,
        0x84, 0x79, 0x35, 0xef, 0xf0, 0xff, 0xfe, 0xaa, 0x84, 0x1b, 0x05, 0xf4, 0xff, 0xef, 0xf0,
        0x5f, 0xfe, 0x25, 0x9d, 0xe2, 0x60, 0x42, 0x64, 0xa2, 0x64, 0x05, 0x61, 0x82, 0x80, 0x05,
        0x45, 0x82, 0x80, 0x41, 0x11, 0x06, 0xe4, 0x2d, 0x45, 0xef, 0xf0, 0xbf, 0xfc, 0x93, 0x08,
        0xd0, 0x05, 0x73, 0x00, 0x00, 0x00,
    ];

    let mut state = exec::RvIsaState::default();

    // init MMU, consume the code block
    let mut mmu = exec::mem::GuestMmu::new(4096); // RV uses 4K pages
    mmu.consume_host(mem.as_ptr(), mem.len()).unwrap();

    let mut executor = exec::interp::RvInterpreterExecutor::new(&mut state, &mut mmu);
    executor.stack(4096).unwrap();

    let block_addr = mem.as_ptr() as u64;
    let entry_pc = block_addr + 48;
    println!("code addr = {:016x}", block_addr);
    println!(" entry pc = {:016x}", entry_pc);
    let exit_reason = executor.exec(entry_pc);
    println!("exit_reason = {:?}", exit_reason);
}
