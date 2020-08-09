#![allow(unused_imports)]

use bindgen;

use std::env;
use std::path::{PathBuf};

fn main() {
    compile_metal();
}

#[cfg(not(feature="osx-metal"))]
fn compile_metal() {
}

#[cfg(feature="osx-metal")]
fn compile_metal() {
    // Generate .rs files from the binding headers
    println!("cargo:rerun-if-changed=bindings");

    let bindings = bindgen::Builder::default()
        .header("bindings/metal_vertex2d.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out = out.join("metal_vertex2d.rs");

    bindings.write_to_file(out).unwrap();
}
