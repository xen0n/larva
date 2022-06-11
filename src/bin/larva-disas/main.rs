use larva::rv::RvDecoder;

fn main() {
        let argv = std::env::args();
        for input_path in argv.skip(1) {
            process(&input_path);
        }
}

fn process(input_path: &str) {
    let mem = std::fs::read(input_path).unwrap();

    let d = RvDecoder::new(64);
    let mut p = 0;
    loop {
        if let Some((insn, size)) = d.disas(&mem[p..mem.len()]) {
            println!("{:16x}: {:?}", p, insn);
            p += size;
        } else {
            break;
        }

        if p == mem.len() {
            break;
        }
    }
}
