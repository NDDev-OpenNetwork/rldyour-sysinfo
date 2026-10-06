fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        // iokit contains a Swift bridge. Its own rustc-link-arg is not
        // transitive, so the final executable must resolve Apple's system
        // Swift runtime itself, independently of any installed Xcode path.
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
}
