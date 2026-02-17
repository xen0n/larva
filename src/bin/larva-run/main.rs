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

    // Allocate and set up thread-local storage (required by musl)
    let tls_size = 0x1000;
    let tls_addr = mmu.mmap(tls_size, MemPerms::rw(), false).expect("failed to allocate TLS");
    state.set_x(4, tls_addr.as_u64()); // x4 = tp (thread pointer)

    // Create interpreter
    let mut executor = interp::RvInterpreterExecutor::new(64, &mut state, &mut mmu);
    // executor.debug(true);

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
