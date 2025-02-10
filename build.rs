// build.rs
extern crate cc;

fn main() {
    cc::Build::new()
        .file("src/nakeds.c")  // Path to your C file
        .opt_level(3)
        .static_flag(true)
        .warnings(false)
        .compile("coroutines");

    println!("cargo:rerun-if-changed=nakeds.c");
}
