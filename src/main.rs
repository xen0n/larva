mod rv;

fn main() {
    macro_rules! d {
        ($x: expr) => {
            println!("{:?}", rv::disas_riscv_insn_32bit($x));
        };
    }

    d!(0x000fc397);
    d!(0x41c30333);
    d!(0x4c03be03);
    d!(0x00135313);
    d!(0x000e0067);
}
