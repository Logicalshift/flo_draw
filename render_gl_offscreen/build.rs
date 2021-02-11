#[cfg(not(target_os = "linux"))]
fn main() {
    // No build steps to take for non-linux OSes
}

#[cfg(target_os = "linux")]
fn main() {
    use std::env;
    use std::path::{PathBuf};

    // Linux build: generate bindings for gbm
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out = out.join("gbm.rs");

    bindgen::Builder::default()
        .header("linux_gbm.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Failed to generate bindings for gbm")
        .write_to_file(out)
        .expect("Could not write gbm.rs");
}
