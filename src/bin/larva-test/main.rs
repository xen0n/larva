use larva::exec;

fn main() {
    let mem: [u8; 12] = [
        0x13, 0x05, 0xb0, 0x07, // 07b00513                li      a0,123
        0x93, 0x08, 0xd0, 0x05, // 05d00893                li      a7,93
        0x73, 0x00, 0x00, 0x00, // 00000073                ecall
    ];

    let mut state = exec::RvIsaState::default();
    let mut executor = exec::interp::RvInterpreterExecutor::new(&mut state, 0 as *mut u8);

    let entry_pc = mem.as_ptr() as u64;
    println!("entry pc = {:016x}", entry_pc);
    let exit_reason = executor.exec(entry_pc);
    println!("exit_reason = {:?}", exit_reason);
}
