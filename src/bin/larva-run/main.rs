use larva::exec::{RvIsaState, elf, interp, mem::GuestAddr, mem::MemPerms};
use std::env;

/// Auxiliary vector entry types (AT_* constants)
const AT_NULL: u64 = 0;
const AT_PHDR: u64 = 3;
const AT_PHENT: u64 = 4;
const AT_PHNUM: u64 = 5;
const AT_PAGESZ: u64 = 6;
const AT_ENTRY: u64 = 9;
const AT_RANDOM: u64 = 25;

fn setup_stack(
    mmu: &mut larva::exec::mem::GuestMmu,
    sp: GuestAddr,
    argc: u64,
    argv: &[String],
    envp: &[String],
    elf_info: &elf::LoadedElf,
) -> GuestAddr {
    let ptr_size = 8usize;
    let num_argv = argv.len();
    let num_envp = envp.len();

    // Calculate string data size
    let strings_size: usize = argv.iter().map(|s| s.len() + 1).sum::<usize>()
        + envp.iter().map(|s| s.len() + 1).sum::<usize>();

    // Layout (growing upward from bottom of stack frame):
    // [argc]
    // [argv[0]] ... [argv[n-1]] [NULL]
    // [envp[0]] ... [envp[m-1]] [NULL]
    // [auxv[0]] ... [auxv[k]] [(AT_NULL, 0)]
    // [padding to 16-byte]
    // [AT_RANDOM bytes (16 bytes)]
    // [string data]

    // Number of auxv entries (including terminating AT_NULL)
    let num_auxv = 7; // AT_PHDR, AT_PHENT, AT_PHNUM, AT_PAGESZ, AT_ENTRY, AT_RANDOM, AT_NULL

    let auxv_offset = ptr_size * (1 + num_argv + 1 + num_envp + 1);
    let strings_start_offset = auxv_offset + ptr_size * 2 * num_auxv;

    // Reserve space for AT_RANDOM (16 bytes, 16-byte aligned)
    let random_offset = (strings_start_offset + strings_size + 15) & !15;
    let total_size = random_offset + 16;

    // Stack grows down, so subtract from sp
    let sp_bottom = sp.as_u64() - total_size as u64;
    let sp_new = GuestAddr(sp_bottom);

    let haddr = mmu.g2h(sp_new).expect("stack not mapped");
    let base_ptr = haddr.as_mut_ptr::<u8>();

    // Calculate string addresses
    let strings_start = sp_bottom + strings_start_offset as u64;
    let mut string_offset = 0usize;

    unsafe {
        // Write argc at bottom
        *(base_ptr as *mut u64) = argc;

        // Write argv pointers
        for (i, arg) in argv.iter().enumerate() {
            let arg_addr = strings_start + string_offset as u64;
            *((base_ptr as *mut u64).add(1 + i)) = arg_addr;

            // Copy string
            let src = arg.as_bytes();
            let dst = base_ptr.add(strings_start_offset + string_offset);
            std::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
            *dst.add(src.len()) = 0;
            string_offset += src.len() + 1;
        }
        // Write argv NULL terminator
        *((base_ptr as *mut u64).add(1 + num_argv)) = 0;

        // Write envp pointers
        for (i, env) in envp.iter().enumerate() {
            let env_addr = strings_start + string_offset as u64;
            *((base_ptr as *mut u64).add(1 + num_argv + 1 + i)) = env_addr;

            // Copy string
            let src = env.as_bytes();
            let dst = base_ptr.add(strings_start_offset + string_offset);
            std::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
            *dst.add(src.len()) = 0;
            string_offset += src.len() + 1;
        }
        // Write envp NULL terminator
        *((base_ptr as *mut u64).add(1 + num_argv + 1 + num_envp)) = 0;

        // Write auxv entries (key, value pairs)
        let auxv_base = base_ptr.add(auxv_offset) as *mut u64;
        let mut idx = 0;

        // AT_PHDR - program header address
        *auxv_base.add(idx) = AT_PHDR;
        *auxv_base.add(idx + 1) = elf_info.phdr_addr.as_u64();
        idx += 2;

        // AT_PHENT - program header entry size
        *auxv_base.add(idx) = AT_PHENT;
        *auxv_base.add(idx + 1) = elf_info.phent as u64;
        idx += 2;

        // AT_PHNUM - number of program headers
        *auxv_base.add(idx) = AT_PHNUM;
        *auxv_base.add(idx + 1) = elf_info.phnum as u64;
        idx += 2;

        // AT_PAGESZ - page size
        *auxv_base.add(idx) = AT_PAGESZ;
        *auxv_base.add(idx + 1) = mmu.page_size() as u64;
        idx += 2;

        // AT_ENTRY - entry point
        *auxv_base.add(idx) = AT_ENTRY;
        *auxv_base.add(idx + 1) = elf_info.entry.as_u64();
        idx += 2;

        // AT_RANDOM - pointer to 16 random bytes
        let random_addr = sp_bottom + random_offset as u64;
        *auxv_base.add(idx) = AT_RANDOM;
        *auxv_base.add(idx + 1) = random_addr;
        idx += 2;

        // AT_NULL - terminator
        *auxv_base.add(idx) = AT_NULL;
        *auxv_base.add(idx + 1) = 0;

        // Fill AT_RANDOM bytes with pseudo-random data (zeros for now)
        let random_ptr = base_ptr.add(random_offset);
        std::ptr::write_bytes(random_ptr, 0, 16);
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
    let (elf_info, sp) = match elf::load_and_setup(&mut mmu, elf_path, &argv, &envp, 1024 * 1024) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Failed to load ELF: {e}");
            std::process::exit(1);
        }
    };

    println!(
        "Loaded ELF: entry={:016x}, sp={:016x}, phdr={:016x}",
        elf_info.entry.as_u64(),
        sp.as_u64(),
        elf_info.phdr_addr.as_u64()
    );

    // Setup stack with argc/argv/envp/auxv
    let sp = setup_stack(&mut mmu, sp, argv.len() as u64, &argv, &envp, &elf_info);

    // Initialize CPU state
    let mut state = RvIsaState::default();
    state.set_x(2, sp.as_u64()); // x2 = sp
    state.set_x(8, sp.as_u64()); // x8 = s0/fp - initialize frame pointer to stack

    // Allocate and set up thread-local storage (required by musl)
    // musl on RISC-V uses TLS_ABOVE_TP: pthread struct is BELOW the TP register
    // TP points to end of pthread struct
    let tls_size = 0x2000; // 8KB for TLS area
    let tls_base = mmu.mmap(tls_size, MemPerms::rw(), false).expect("failed to allocate TLS");
    
    // Set up minimal pthread structure for musl
    // pthread struct is approximately 200-300 bytes, place it at end of TLS area
    let pthread_size = 0x200; // 512 bytes for pthread struct
    let pthread_addr = tls_base.as_u64() + tls_size as u64 - pthread_size;
    
    // Initialize pthread struct fields in guest memory
    if let Some(haddr) = mmu.g2h(GuestAddr(pthread_addr)) {
        let pthread_ptr = haddr.as_mut_ptr::<u64>();
        unsafe {
            // Field 0: self = pointer to itself
            pthread_ptr.write(pthread_addr);
            // Field 1: prev = self (circular list)
            pthread_ptr.add(1).write(pthread_addr);
            // Field 2: next = self (circular list)
            pthread_ptr.add(2).write(pthread_addr);
            // Field 3: sysinfo = 0
            pthread_ptr.add(3).write(0);
            // Field 4: tid = 1 (main thread)
            pthread_ptr.add(4).write(1);
            // More fields zeroed by default from mmap
        }
    }
    
    // TP points to end of pthread struct (musl convention with TLS_ABOVE_TP)
    let tp = pthread_addr + pthread_size;
    state.set_x(4, tp); // x4 = tp (thread pointer)
    
    // Also allocate and set up a fake __libc structure that musl accesses via x8-112
    // The crash happens because musl tries to load a pointer from x8-112,
    // then access fields within __libc. We need to pre-initialize this.
    let libc_size = 0x100; // 256 bytes for __libc structure
    let libc_area = mmu.mmap(libc_size, MemPerms::rw(), false).expect("failed to allocate libc area");
    
    // Initialize __libc structure fields
    if let Some(haddr) = mmu.g2h(libc_area) {
        let libc_ptr = haddr.as_mut_ptr::<u64>();
        unsafe {
            // __libc.can_do_threads = 1 (offset 0)
            libc_ptr.write(1);
            // __libc.threaded = 0 (offset 1)
            libc_ptr.add(1).write(0);
            // __libc.secure = 0 (offset 2)  
            libc_ptr.add(2).write(0);
            // __libc.need_locks = 0 (offset 3)
            libc_ptr.add(3).write(0);
            // __libc.threads_minus_1 = 0 (offset 4)
            libc_ptr.add(4).write(0);
            // __libc.page_size = 4096 (offset ~56)
            (libc_ptr.add(7) as *mut u64).write(4096);
        }
    }
    
    // Store pointer to __libc at x8-112 location (we'll set x8 to sp + 112)
    // Actually, musl expects x8 to point to a location where x8-112 contains the __libc pointer
    // Let's set up a structure on the stack for this
    let stack_haddr = mmu.g2h(sp).expect("stack not mapped");
    unsafe {
        // At sp+112, store pointer to __libc area
        let libc_ptr_location = (stack_haddr.as_mut_ptr::<u64>()).add(14); // sp + 14*8 = sp + 112
        libc_ptr_location.write(libc_area.as_u64());
    }

    // Create interpreter
    let mut executor = interp::RvInterpreterExecutor::new(64, &mut state, &mut mmu);
    
    // Enable debug mode if LARVA_DEBUG is set
    if std::env::var("LARVA_DEBUG").is_ok() {
        executor.debug(true);
    }

    // Run the program
    match executor.exec(elf_info.entry.as_u64()) {
        Some(reason) => {
            eprintln!("Program stopped unexpectedly: {reason:?}");
            std::process::exit(1);
        }
        None => {
            // Normal exit via exit_group syscall
        }
    }
}
