mod rv;

fn main() {
    let mem = [
        0x97, 0xc4, 0x0f, 0x00, 0x9c, 0x67, 0x33, 0x03, 0xc3, 0x41, 0x03, 0xbe, 0x03, 0x4c, 0x13,
        0x53, 0x13, 0x00, 0x67, 0x00, 0x0e, 0x00,
    ];

    let d = rv::RvDecoder::new(64);
    let mut p = 0;
    loop {
        if let Some((insn, size)) = d.disas(&mem[p..mem.len()]) {
            println!("{:?}", insn);
            p += size;
        } else {
            break;
        }

        if p == mem.len() {
            break;
        }
    }
}
