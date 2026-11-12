// Tanam ikon (praydf.rc -> praydf.ico) ke exe memakai windres dari toolchain MinGW.
fn main() {
    println!("cargo:rerun-if-changed=praydf.rc");
    println!("cargo:rerun-if-changed=praydf.ico");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("praydf-res.o");
    let ok = std::process::Command::new("windres")
        .args(["praydf.rc", "-O", "coff", "-o"])
        .arg(&out)
        .status()
        .is_ok_and(|s| s.success());
    if ok {
        println!("cargo:rustc-link-arg-bins={}", out.display());
    } else {
        println!("cargo:warning=windres tidak ditemukan; exe dibuat tanpa ikon");
    }
}
