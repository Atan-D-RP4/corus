// build.rs
extern crate cc;

fn main() {
    let coroutines = "src/coroutines.c";
    let generators = "src/generators.c";
    cc::Build::new()
        .file(coroutines) // Path to your C file
        .opt_level(3)
        .static_flag(true)
        .warnings(false)
        .compile("coroutines");
    cc::Build::new()
        .file(generators) // Path to your C file
        .opt_level(3)
        .static_flag(true)
        .warnings(false)
        .compile("generators");

    println!("cargo:rerun-if-changed={}", coroutines);
    println!("cargo:rerun-if-changed={}", generators);
}
