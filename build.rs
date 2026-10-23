fn main() {
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
