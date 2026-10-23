fn main() {
    println!("cargo:rerun-if-changed=src/macos/native.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        build_macos();
    }
    println!("cargo:rerun-if-changed=assets/ui/reeb.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/ui/reeb.ico")
            .set("ProductName", "Reeb")
            .set("FileDescription", "Reeb keyboard sounds")
            .compile()
            .expect("Could not compile the Reeb Windows icon resource");
    }
}

fn build_macos() {
    use std::{path::PathBuf, process::Command};
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "aarch64" => "arm64",
        "x86_64" => "x86_64",
        other => panic!("Unsupported macOS architecture: {other}"),
    };
    let object = out.join("reeb_macos.o");
    assert!(
        Command::new("xcrun")
            .args([
                "clang",
                "-c",
                "-fobjc-arc",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-mmacosx-version-min=11.0",
                "-arch",
                arch,
                "src/macos/native.m",
                "-o",
            ])
            .arg(&object)
            .status()
            .expect("Xcode Command Line Tools are required")
            .success()
    );
    assert!(
        Command::new("xcrun")
            .args(["ar", "crs"])
            .arg(out.join("libreeb_macos.a"))
            .arg(object)
            .status()
            .unwrap()
            .success()
    );
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=reeb_macos");
    for framework in ["AppKit", "CoreGraphics", "CoreFoundation"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}
