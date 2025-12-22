fn main() {
    // tauri-build reads configuration from the build script's current directory.
    // Keep a single source of truth by always pointing codegen at src-tauri/
    // (where the canonical tauri.conf.json lives).
    let tauri_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    println!("cargo:rerun-if-changed={}", tauri_dir.join("tauri.conf.json").display());
    if let Err(err) = std::env::set_current_dir(&tauri_dir) {
        panic!("failed to set current dir to {}: {err}", tauri_dir.display());
    }

    tauri_build::build()
}
