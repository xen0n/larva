use larva::exec::{elf, interp, mem, RvIsaState};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <elf-file> [args...]", args[0]);
        std::process::exit(1);
    }

    let elf_path = &args[1];
    let argv: Vec<String> = args[1..].to_vec();
    let envp: Vec<String> = env::vars()
        .map(|(k, v)| format!("{k}={v}"))
        .collect();

    // Initialize MMU with 4KB pages
    let mut mmu = mem::GuestMmu::new(4096);

    // Load the ELF binary
    let (entry, sp) = match elf::load_and_setup(&mut mmu, elf_path, &argv, &envp, 1024 * 1024) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Failed to load ELF: {e}");
            std::process::exit(1);
        }
    };

    println!("Loaded ELF: entry={:016x}, sp={:016x}", entry.as_u64(), sp.as_u64());

    // Initialize CPU state
    let mut state = RvIsaState::default();
    state.set_x(2, sp.as_u64()); // x2 = sp

    // Create interpreter
    let mut executor = interp::RvInterpreterExecutor::new(64, &mut state, &mut mmu);
    
    // Run the program
    match executor.exec(entry.as_u64()) {
        Some(reason) => {
            eprintln!("Program stopped unexpectedly: {reason:?}");
            std::process::exit(1);
        }
        None => {
            // Normal exit via exit_group syscall
        }
    }
}
