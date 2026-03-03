fn main() {
    // Automatically enable the tokio_support feature on Linux (where tokio is always a dependency)
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-cfg=feature=\"tokio_support\"");
    }
}
