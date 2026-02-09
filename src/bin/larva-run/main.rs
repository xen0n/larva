use larva::exec::{RvIsaState, elf, interp, mem::GuestAddr};
use std::env;

fn setup_stack(
    mmu: &mut larva::exec::mem::GuestMmu,
    sp: GuestAddr,
    argc: u64,
    argv: &[String],
) -> GuestAddr {
    // Calculate total stack frame size:
    // argc + argv pointers + NULL terminator + envp (just NULL) + strings
    let ptr_size = 8usize;
    let num_argv = argv.len();

    // Calculate string data size
    let strings_size: usize = argv.iter().map(|s| s.len() + 1).sum();

    // Layout:
    // [argc]
    // [argv[0]] ... [argv[n-1]] [NULL]
    // [envp[0]] [NULL]  (minimal - just NULL)
    // [auxv] (empty for now)
    // [string data]
    let data_offset = ptr_size * (1 + num_argv + 1 + 1); // argc + argv + NULL + envp NULL
    let total_size = data_offset + strings_size;

    // Round up to 16-byte alignment and add some padding
    let total_size = (total_size + 15) & !15;

    // Stack grows down, so subtract from sp
    let sp_bottom = sp.as_u64() - total_size as u64;
    let sp_new = GuestAddr(sp_bottom);

    let haddr = mmu.g2h(sp_new).expect("stack not mapped");
    let base_ptr = haddr.as_mut_ptr::<u8>();

    unsafe {
        // Write argc at bottom
        *(base_ptr as *mut u64) = argc;

        // Calculate where strings start
        let strings_start = sp_bottom + data_offset as u64;
        let mut string_offset = 0usize;

        // Write argv pointers
        for (i, arg) in argv.iter().enumerate() {
            let arg_addr = strings_start + string_offset as u64;
            *((base_ptr as *mut u64).add(1 + i)) = arg_addr;

            // Copy string
            let src = arg.as_bytes();
            let dst = base_ptr.add(data_offset + string_offset);
            std::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
            *dst.add(src.len()) = 0; // null terminator

            string_offset += src.len() + 1;
        }

        // Write argv NULL terminator
        *((base_ptr as *mut u64).add(1 + num_argv)) = 0;

        // Write envp NULL terminator (no envp for now)
        *((base_ptr as *mut u64).add(1 + num_argv + 1)) = 0;
    }

    sp_new
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <elf-file> [args...]", args[0]);
        std::process::exit(1);
    }

    let elf_path = &args[1];
    let argv: Vec<String> = args[1..].to_vec();
    let envp: Vec<String> = env::vars().map(|(k, v)| format!("{k}={v}")).collect();

    // Initialize MMU with 4KB pages
    let mut mmu = larva::exec::mem::GuestMmu::new(4096);

    // Load the ELF binary
    let (entry, sp) = match elf::load_and_setup(&mut mmu, elf_path, &argv, &envp, 1024 * 1024) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Failed to load ELF: {e}");
            std::process::exit(1);
        }
    };

    println!(
        "Loaded ELF: entry={:016x}, sp={:016x}",
        entry.as_u64(),
        sp.as_u64()
    );

    // Setup stack with argc/argv
    let sp = setup_stack(&mut mmu, sp, argv.len() as u64, &argv);

    // Initialize CPU state
    let mut state = RvIsaState::default();
    state.set_x(2, sp.as_u64()); // x2 = sp

    // Create interpreter
    let mut executor = interp::RvInterpreterExecutor::new(64, &mut state, &mut mmu);
    // executor.debug(true);

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
