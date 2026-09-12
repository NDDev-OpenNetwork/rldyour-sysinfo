fn main() {
    #[cfg(target_os = "macos")]
    {
        cc::Build::new()
            .file("src/source/macos/hid_temperature.c")
            .warnings(true)
            .compile("rldyour_hid_temperature");
        println!("cargo:rustc-link-lib=framework=IOKit");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rerun-if-changed=src/source/macos/hid_temperature.c");
    }
}
