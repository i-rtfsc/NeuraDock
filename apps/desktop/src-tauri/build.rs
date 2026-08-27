fn main() {
    println!("cargo:rustc-check-cfg=cfg(neuradock_root_package)");
    println!("cargo:rustc-cfg=neuradock_root_package");
    tauri_build::build()
}
