//! `optimize_rs <in.rs> <out.rs> [--without PASS]`: apply the production optimizer to a Rust file, optionally with one
//! pass switched off. Used by the parity and economics harnesses so a benchmark compares exactly what the compiler
//! hook emits (and, with `--without`, what one pass contributes).

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 && !(args.len() == 5 && args[3] == "--without") {
        eprintln!("usage: optimize_rs <in.rs> <out.rs> [--without PASS]");
        std::process::exit(2);
    }
    let src = std::fs::read_to_string(&args[1]).expect("read input");
    let mut cfg = windjammer::optimizer::OptimizationConfig::production();
    if args.len() == 5 {
        match args[4].as_str() {
            "bounded_counter" => cfg.bounded_counter = false,
            "with_capacity" => cfg.with_capacity = false,
            "string_append" => cfg.string_append = false,
            other => {
                eprintln!("optimize_rs: --without supports bounded_counter, with_capacity, string_append (got {other})");
                std::process::exit(2);
            }
        }
    }
    let out = windjammer::optimizer::run(&cfg, &src);
    std::fs::write(&args[2], out.code).expect("write output");
    for (pass, n) in out.applied {
        eprintln!("{pass}: {n}");
    }
}
