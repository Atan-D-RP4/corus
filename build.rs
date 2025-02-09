// build.rs
extern crate cc;

fn main() {
    cc::Build::new()
        .file("coroutine.c")  // Path to your C file
        .opt_level(3)
        .static_flag(true)
        .compile("coroutine");

    println!("cargo:rerun-if-changed=coroutine.c");
}
