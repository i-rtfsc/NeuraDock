use std::path::Path;

fn main() {
    track_dir(Path::new("migrations"));
}

fn track_dir(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                track_dir(&entry_path);
            } else {
                println!("cargo:rerun-if-changed={}", entry_path.display());
            }
        }
    }
}
