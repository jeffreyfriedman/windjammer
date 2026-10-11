//! `optimize_rs <in.rs> <out.rs>`: apply the production optimizer to a Rust file.
//! Used by `run.py` so the benchmark compares exactly what the compiler hook emits.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: optimize_rs <in.rs> <out.rs>");
        std::process::exit(2);
    }
    let src = std::fs::read_to_string(&args[1]).expect("read input");
    let out = windjammer::optimizer::run(&windjammer::optimizer::OptimizationConfig::production(), &src);
    std::fs::write(&args[2], out.code).expect("write output");
    for (pass, n) in out.applied {
        eprintln!("{pass}: {n}");
    }
}
