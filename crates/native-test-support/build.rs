fn main() {
    println!("cargo:rerun-if-changed=fixtures.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("fixtures.m")
            .flag("-fobjc-arc")
            .compile("radiant_native_test_fixtures");
    }
}
