use std::str;

///
/// Reads the README file for the crate
///
fn readme() -> &'static str {
    let readme_bytes    = include_bytes!("../README.md");
    let readme_str      = str::from_utf8(readme_bytes);

    readme_str.expect("Could not decode README.md")
}

#[test]
fn starts_with_version_number_toml() {
    let major_version = env!("CARGO_PKG_VERSION_MAJOR");
    let minor_version = env!("CARGO_PKG_VERSION_MINOR");

    let expected = format!("```toml
flo_draw = \"{}.{}\"
```", major_version, minor_version);

    println!("{}", expected);
    assert!(readme().starts_with(&expected));
}
