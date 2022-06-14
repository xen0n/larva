use larva::exec;

fn main() {
    /*
    00000000000100b0 <_start>:
           100b0:       4505                    li      a0,1
           100b2:       00000597                auipc   a1,0x0
           100b6:       01e58593                addi    a1,a1,30 # 100d0 <hello>
           100ba:       4631                    li      a2,12
           100bc:       04000893                li      a7,64
           100c0:       00000073                ecall
           100c4:       00000513                li      a0,0
           100c8:       05d00893                li      a7,93
           100cc:       00000073                ecall

    00000000000100d0 <hello>:
           100d0:       6c6c6568                .word   0x6c6c6568
           100d4:       6f77206f                .word   0x6f77206f
           100d8:       0a646c72                .word   0x0a646c72
         */
    let mem: [u8; 44] = [
        0x05, 0x45, 0x97, 0x05, 0x00, 0x00, 0x93, 0x85, 0xe5, 0x01, 0x31, 0x46, 0x93, 0x08, 0x00,
        0x04, 0x73, 0x00, 0x00, 0x00, 0x13, 0x05, 0x00, 0x00, 0x93, 0x08, 0xd0, 0x05, 0x73, 0x00,
        0x00, 0x00, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x0a,
    ];

    let mut state = exec::RvIsaState::default();

    // init MMU, consume the code block
    let mut mmu = exec::mem::GuestMmu::new(4096); // RV uses 4K pages
    mmu.consume_host(mem.as_ptr(), mem.len()).unwrap();

    let mut executor = exec::interp::RvInterpreterExecutor::new(64, &mut state, &mut mmu);
    executor.stack(4096).unwrap();

    let block_addr = mem.as_ptr() as u64;
    let entry_pc = block_addr + 0;
    println!("code addr = {:016x}", block_addr);
    println!(" entry pc = {:016x}", entry_pc);
    let exit_reason = executor.exec(entry_pc);
    println!("exit_reason = {:?}", exit_reason);
}
